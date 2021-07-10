// Copyright (C) 2021  Tassilo Horn <tsdh@gnu.org>
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

//! Functions and data structures of the swayrd demon.

use crate::cmds;
use crate::con;
use crate::config;
use crate::layout;
use crate::util;
use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use swayipc as s;

pub fn run_demon() {
    let extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let extra_props_for_ev_handler = extra_props.clone();

    thread::spawn(move || {
        monitor_sway_events(extra_props_for_ev_handler);
    });

    serve_client_requests(extra_props);
}

fn connect_and_subscribe() -> s::Fallible<s::EventStream> {
    s::Connection::new()?
        .subscribe(&[s::EventType::Window, s::EventType::Workspace])
}

pub fn monitor_sway_events(
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
) {
    let config = config::load_config();

    'reset: loop {
        println!("Connecting to sway for subscribing to events...");
        match connect_and_subscribe() {
            Err(err) => {
                eprintln!("Could not connect and subscribe: {}", err);
                std::thread::sleep(std::time::Duration::from_secs(3));
                break 'reset;
            }
            Ok(iter) => {
                for ev_result in iter {
                    let handled;
                    match ev_result {
                        Ok(ev) => match ev {
                            s::Event::Window(win_ev) => {
                                let extra_props_clone = extra_props.clone();
                                handled = handle_window_event(
                                    win_ev,
                                    extra_props_clone,
                                    &config,
                                );
                            }
                            s::Event::Workspace(ws_ev) => {
                                let extra_props_clone = extra_props.clone();
                                handled = handle_workspace_event(
                                    ws_ev,
                                    extra_props_clone,
                                );
                            }
                            _ => handled = false,
                        },
                        Err(e) => {
                            eprintln!("Error while receiving events: {}", e);
                            std::thread::sleep(std::time::Duration::from_secs(
                                3,
                            ));
                            eprintln!("Resetting!");
                            break 'reset;
                        }
                    }
                    if handled {
                        println!(
                            "New extra_props state:\n{:#?}",
                            *extra_props.read().unwrap()
                        );
                    }
                }
            }
        }
    }
}

fn handle_window_event(
    ev: Box<s::WindowEvent>,
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
    config: &config::Config,
) -> bool {
    let s::WindowEvent {
        change, container, ..
    } = *ev;
    match change {
        s::WindowChange::Focus => {
            layout::maybe_auto_tile(config);
            update_last_focus_time(container.id, extra_props);
            println!("Handled window event type {:?}", change);
            true
        }
        s::WindowChange::New => {
            layout::maybe_auto_tile(config);
            update_last_focus_time(container.id, extra_props);
            println!("Handled window event type {:?}", change);
            true
        }
        s::WindowChange::Close => {
            remove_extra_props(container.id, extra_props);
            layout::maybe_auto_tile(config);
            println!("Handled window event type {:?}", change);
            true
        }
        s::WindowChange::Move | s::WindowChange::Floating => {
            layout::maybe_auto_tile(config);
            println!("Handled window event type {:?}", change);
            false // We don't affect the extra_props state here.
        }
        _ => {
            println!("Unhandled window event type {:?}", change);
            false
        }
    }
}

fn handle_workspace_event(
    ev: Box<s::WorkspaceEvent>,
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
) -> bool {
    let s::WorkspaceEvent {
        change,
        current,
        old: _,
        ..
    } = *ev;
    match change {
        s::WorkspaceChange::Init | s::WorkspaceChange::Focus => {
            update_last_focus_time(
                current
                    .expect("No current in Init or Focus workspace event")
                    .id,
                extra_props,
            );
            println!("Handled workspace event type {:?}", change);
            true
        }
        s::WorkspaceChange::Empty => {
            remove_extra_props(
                current.expect("No current in Empty workspace event").id,
                extra_props,
            );
            println!("Handled workspace event type {:?}", change);
            true
        }
        _ => false,
    }
}

fn update_last_focus_time(
    id: i64,
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
) {
    let mut write_lock = extra_props.write().unwrap();
    if let Some(wp) = write_lock.get_mut(&id) {
        wp.last_focus_time = get_epoch_time_as_millis();
    } else {
        write_lock.insert(
            id,
            con::ExtraProps {
                last_focus_time: get_epoch_time_as_millis(),
            },
        );
    }
}

fn remove_extra_props(
    id: i64,
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
) {
    extra_props.write().unwrap().remove(&id);
}

fn get_epoch_time_as_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Couldn't get epoch time!")
        .as_millis()
}

pub fn serve_client_requests(
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
) {
    match std::fs::remove_file(util::get_swayr_socket_path()) {
        Ok(()) => println!("Deleted stale socket from previous run."),
        Err(e) => eprintln!("Could not delete socket:\n{:?}", e),
    }

    match UnixListener::bind(util::get_swayr_socket_path()) {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        handle_client_request(stream, extra_props.clone());
                    }
                    Err(err) => {
                        eprintln!("Error handling client request: {}", err);
                        break;
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Could not bind socket: {}", err)
        }
    }
}

fn handle_client_request(
    mut stream: UnixStream,
    extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
) {
    let mut cmd_str = String::new();
    if stream.read_to_string(&mut cmd_str).is_ok() {
        if let Ok(cmd) = serde_json::from_str::<cmds::SwayrCommand>(&cmd_str) {
            cmds::exec_swayr_cmd(cmds::ExecSwayrCmdArgs {
                cmd: &cmd,
                extra_props,
            });
        } else {
            eprintln!(
                "Could not serialize following string to SwayrCommand.\n{}",
                cmd_str
            );
        }
    } else {
        eprintln!("Could not read command from client.");
    }
}
