//! Functions and data structures of the swayrd demon.

use crate::cmds;
use crate::ipc;
use crate::util;

use std::collections::HashMap;
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use swayipc as s;
use swayipc::reply as r;

pub fn monitor_sway_events(
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
) {
    'reset: loop {
        println!("Connecting to sway for subscribing to events...");
        let iter = s::Connection::new()
            .expect("Could not connect!")
            .subscribe(&[s::EventType::Window, s::EventType::Workspace])
            .expect("Could not subscribe to window and workspace events.");

        for ev_result in iter {
            let handled;
            match ev_result {
                Ok(ev) => match ev {
                    r::Event::Window(win_ev) => {
                        let extra_props_clone = extra_props.clone();
                        handled =
                            handle_window_event(win_ev, extra_props_clone);
                    }
                    r::Event::Workspace(ws_ev) => {
                        let extra_props_clone = extra_props.clone();
                        handled =
                            handle_workspace_event(ws_ev, extra_props_clone);
                    }
                    _ => handled = false,
                },
                Err(e) => {
                    eprintln!("Error while receiving events: {}", e);
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

fn handle_window_event(
    ev: Box<r::WindowEvent>,
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
) -> bool {
    let r::WindowEvent { change, container } = *ev;
    match change {
        r::WindowChange::New | r::WindowChange::Focus => {
            update_last_focus_time(container.id, extra_props);
            true
        }
        r::WindowChange::Close => {
            remove_extra_props(container.id, extra_props);
            true
        }
        _ => false,
    }
}

fn handle_workspace_event(
    ev: Box<r::WorkspaceEvent>,
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
) -> bool {
    let r::WorkspaceEvent {
        change,
        current,
        old: _,
    } = *ev;
    match change {
        r::WorkspaceChange::Init | r::WorkspaceChange::Focus => {
            update_last_focus_time(
                current
                    .expect("No current in Init or Focus workspace event")
                    .id,
                extra_props,
            );
            true
        }
        r::WorkspaceChange::Empty => {
            remove_extra_props(
                current.expect("No current in Empty workspace event").id,
                extra_props,
            );
            false
        }
        _ => false,
    }
}

fn update_last_focus_time(
    id: i64,
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
) {
    let mut write_lock = extra_props.write().unwrap();
    if let Some(wp) = write_lock.get_mut(&id) {
        wp.last_focus_time = get_epoch_time_as_millis();
    } else {
        write_lock.insert(
            id,
            ipc::ExtraProps {
                last_focus_time: get_epoch_time_as_millis(),
            },
        );
    }
}

fn remove_extra_props(
    id: i64,
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
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
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
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
    extra_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>>,
) {
    let mut cmd_str = String::new();
    if stream.read_to_string(&mut cmd_str).is_ok() {
        if let Ok(cmd) = serde_json::from_str::<ipc::SwayrCommand>(&cmd_str) {
            cmds::exec_swayr_cmd(&cmd, extra_props);
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
