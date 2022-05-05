// Copyright (C) 2021-2022  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

//! Functions and data structures of the swayrd daemon.

use crate::cmds;
use crate::config::{self, Config};
use crate::focus::FocusData;
use crate::focus::FocusEvent;
use crate::focus::FocusMessage;
use crate::layout;
use crate::util;
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;
use swayipc as s;

pub fn run_daemon() {
    let (focus_tx, focus_rx) = mpsc::channel();
    let fdata = FocusData {
        focus_tick_by_id: Arc::new(RwLock::new(HashMap::new())),
        focus_chan: focus_tx,
    };

    let config = config::load_config();
    let lockin_delay = config.get_focus_lockin_delay();

    {
        let fdata = fdata.clone();
        thread::spawn(move || {
            monitor_sway_events(fdata, &config);
        });
    }

    {
        let fdata = fdata.clone();
        thread::spawn(move || {
            focus_lock_in_handler(focus_rx, fdata, lockin_delay);
        });
    }

    serve_client_requests(fdata);
}

fn connect_and_subscribe() -> s::Fallible<s::EventStream> {
    s::Connection::new()?.subscribe(&[
        s::EventType::Window,
        s::EventType::Workspace,
        s::EventType::Shutdown,
    ])
}

pub fn monitor_sway_events(fdata: FocusData, config: &Config) {
    let mut focus_counter = 0;
    let mut resets = 0;
    let max_resets = 10;

    'reset: loop {
        if resets >= max_resets {
            break;
        }
        resets += 1;

        log::debug!("Connecting to sway for subscribing to events...");
        match connect_and_subscribe() {
            Err(err) => {
                log::warn!("Could not connect and subscribe: {}", err);
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            Ok(iter) => {
                for ev_result in iter {
                    let show_extra_props_state;
                    resets = 0;
                    match ev_result {
                        Ok(ev) => match ev {
                            s::Event::Window(win_ev) => {
                                focus_counter += 1;
                                show_extra_props_state = handle_window_event(
                                    win_ev,
                                    &fdata,
                                    config,
                                    focus_counter,
                                );
                            }
                            s::Event::Workspace(ws_ev) => {
                                focus_counter += 1;
                                show_extra_props_state = handle_workspace_event(
                                    ws_ev,
                                    &fdata,
                                    focus_counter,
                                );
                            }
                            s::Event::Shutdown(sd_ev) => {
                                log::debug!(
                                    "Sway shuts down with reason '{:?}'.",
                                    sd_ev.change
                                );
                                break 'reset;
                            }
                            _ => show_extra_props_state = false,
                        },
                        Err(e) => {
                            log::warn!("Error while receiving events: {}", e);
                            std::thread::sleep(std::time::Duration::from_secs(
                                3,
                            ));
                            show_extra_props_state = false;
                            log::warn!("Resetting!");
                        }
                    }
                    if show_extra_props_state {
                        log::debug!(
                            "New extra_props state:\n{:#?}",
                            *fdata.focus_tick_by_id.read().unwrap()
                        );
                    }
                }
            }
        }
    }
    log::debug!("Swayr daemon shutting down.")
}

fn handle_window_event(
    ev: Box<s::WindowEvent>,
    fdata: &FocusData,
    config: &config::Config,
    focus_val: u64,
) -> bool {
    let s::WindowEvent {
        change, container, ..
    } = *ev;
    match change {
        s::WindowChange::Focus => {
            layout::maybe_auto_tile(config);
            fdata.send(FocusMessage::FocusEvent(FocusEvent {
                node_id: container.id,
                ev_focus_ctr: focus_val,
            }));
            log::debug!("Handled window event type {:?}", change);
            true
        }
        s::WindowChange::New => {
            layout::maybe_auto_tile(config);
            fdata.ensure_id(container.id);
            log::debug!("Handled window event type {:?}", change);
            true
        }
        s::WindowChange::Close => {
            fdata.remove_focus_data(container.id);
            layout::maybe_auto_tile(config);
            log::debug!("Handled window event type {:?}", change);
            true
        }
        s::WindowChange::Move | s::WindowChange::Floating => {
            layout::maybe_auto_tile(config);
            log::debug!("Handled window event type {:?}", change);
            false // We don't affect the extra_props state here.
        }
        _ => {
            log::debug!("Unhandled window event type {:?}", change);
            false
        }
    }
}

fn handle_workspace_event(
    ev: Box<s::WorkspaceEvent>,
    fdata: &FocusData,
    focus_val: u64,
) -> bool {
    let s::WorkspaceEvent {
        change,
        current,
        old: _,
        ..
    } = *ev;
    match change {
        s::WorkspaceChange::Init | s::WorkspaceChange::Focus => {
            let id = current
                .expect("No current in Init or Focus workspace event")
                .id;
            fdata.send(FocusMessage::FocusEvent(FocusEvent {
                node_id: id,
                ev_focus_ctr: focus_val,
            }));
            log::debug!("Handled workspace event type {:?}", change);
            true
        }
        s::WorkspaceChange::Empty => {
            fdata.remove_focus_data(
                current.expect("No current in Empty workspace event").id,
            );
            log::debug!("Handled workspace event type {:?}", change);
            true
        }
        _ => false,
    }
}

pub fn serve_client_requests(fdata: FocusData) {
    match std::fs::remove_file(util::get_swayr_socket_path()) {
        Ok(()) => log::debug!("Deleted stale socket from previous run."),
        Err(e) => log::error!("Could not delete socket:\n{:?}", e),
    }

    match UnixListener::bind(util::get_swayr_socket_path()) {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        handle_client_request(stream, &fdata);
                    }
                    Err(err) => {
                        log::error!("Error handling client request: {}", err);
                        break;
                    }
                }
            }
        }
        Err(err) => {
            log::error!("Could not bind socket: {}", err)
        }
    }
}

fn handle_client_request(mut stream: UnixStream, fdata: &FocusData) {
    let mut cmd_str = String::new();
    if stream.read_to_string(&mut cmd_str).is_ok() {
        if let Ok(cmd) = serde_json::from_str::<cmds::SwayrCommand>(&cmd_str) {
            cmds::exec_swayr_cmd(cmds::ExecSwayrCmdArgs {
                cmd: &cmd,
                focus_data: fdata,
            });
        } else {
            log::error!(
                "Could not serialize following string to SwayrCommand.\n{}",
                cmd_str
            );
        }
    } else {
        log::error!("Could not read command from client.");
    }
}

#[derive(Debug)]
enum InhibitState {
    FocusInhibit,
    FocusActive,
}

impl InhibitState {
    pub fn set(&mut self) {
        if let InhibitState::FocusActive = self {
            log::debug!("Inhibiting tick focus updates");
            *self = InhibitState::FocusInhibit;
        }
    }

    pub fn clear(&mut self) {
        if let InhibitState::FocusInhibit = self {
            log::debug!("Activating tick focus updates");
            *self = InhibitState::FocusActive;
        }
    }
}

fn focus_lock_in_handler(
    focus_chan: mpsc::Receiver<FocusMessage>,
    fdata: FocusData,
    lockin_delay: Duration,
) {
    // Focus event that has not yet been locked-in to the LRU order
    let mut pending_fev: Option<FocusEvent> = None;

    // Toggle to inhibit LRU focus updates
    let mut inhibit = InhibitState::FocusActive;

    let update_focus = |fev: Option<FocusEvent>| {
        if let Some(fev) = fev {
            log::debug!("Locking-in focus on {}", fev.node_id);
            fdata.update_last_focus_tick(
                fev.node_id,
                fev.ev_focus_ctr,
            )
        }
    };

    // outer loop, waiting for focus events
    loop {
        let fmsg = match focus_chan.recv() {
            Ok(fmsg) => fmsg,
            Err(mpsc::RecvError) => return,
        };

        let mut fev = match fmsg {
            FocusMessage::TickUpdateInhibit => {
                inhibit.set();
                continue;
            }
            FocusMessage::TickUpdateActivate => {
                inhibit.clear();
                update_focus(pending_fev.take());
                continue
            }
            FocusMessage::FocusEvent(fev) => {
                if let InhibitState::FocusInhibit = inhibit {
                    // update the pending event but take no further action
                    pending_fev = Some(fev);
                    continue;
                }
                fev
            }
        };

        // Inner loop, waiting for the lock-in delay to expire
        loop {
            let fmsg = match focus_chan.recv_timeout(lockin_delay) {
                Ok(fmsg) => fmsg,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    update_focus(Some(fev));
                    break; // return to outer loop
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            };

            match fmsg {
                FocusMessage::TickUpdateInhibit => {
                    // inhibit requested before currently focused container
                    // was locked-in, set it as pending in case no other
                    // focus changes are made while updates remain inhibited
                    inhibit.set();
                    pending_fev = Some(fev);
                    break; // return to outer loop with a preset pending_fev
                }
                FocusMessage::TickUpdateActivate => {
                    // updates reactivated while we were waiting to lockin
                    // Immediately lockin fev
                    inhibit.clear();
                    update_focus(Some(fev));
                    break;
                }
                FocusMessage::FocusEvent(new_fev) => {
                    // start a new wait (inner) loop with the most recent
                    // focus event
                    fev = new_fev;
                }
            }
        }
    }
}
