use crate::ipc;
use crate::util;
use serde_json::Deserializer;
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process as proc;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn monitor_window_events(
    con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>>,
) {
    let child = proc::Command::new("swaymsg")
        .arg("--monitor")
        .arg("--raw")
        .arg("-t")
        .arg("subscribe")
        .arg("[\"window\", \"workspace\"]")
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect("Failed to subscribe to window events");
    let stdout: std::process::ChildStdout = child.stdout.unwrap();
    let stream = Deserializer::from_reader(stdout).into_iter::<ipc::ConEvent>();
    for res in stream {
        match res {
            Ok(win_ev) => handle_con_event(win_ev, con_props.clone()),
            Err(err) => eprintln!("Error handling window event:\n{:?}", err),
        }
    }
}

fn update_last_focus_time(
    id: ipc::Id,
    con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>>,
) {
    let mut write_lock = con_props.write().unwrap();
    if let Some(mut wp) = write_lock.get_mut(&id) {
        wp.last_focus_time = get_epoch_time_as_millis();
    } else {
        write_lock.insert(
            id,
            ipc::ConProps {
                last_focus_time: get_epoch_time_as_millis(),
            },
        );
    }
}

fn remove_winprops(
    id: &ipc::Id,
    con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>>,
) {
    con_props.write().unwrap().remove(id);
}

fn handle_con_event(
    ev: ipc::ConEvent,
    con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>>,
) {
    let mut handled = true;
    let con_props2 = con_props.clone();

    match ev {
        ipc::ConEvent::WindowEvent { change, container } => match change {
            ipc::WindowEventType::New | ipc::WindowEventType::Focus => {
                update_last_focus_time(container.id, con_props)
            }
            ipc::WindowEventType::Close => {
                remove_winprops(&container.id, con_props)
            }
            _ => handled = false,
        },
        ipc::ConEvent::WorkspaceEvent {
            change,
            current,
            old: _,
        } => match change {
            ipc::WorkspaceEventType::Init | ipc::WorkspaceEventType::Focus => {
                println!("WsEv");
                update_last_focus_time(current.id, con_props)
            }
            ipc::WorkspaceEventType::Empty => {
                remove_winprops(&current.id, con_props)
            }
            _ => handled = false,
        },
    }

    if handled {
        println!("New con_props state:\n{:#?}", *con_props2.read().unwrap());
    }
}

fn get_epoch_time_as_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Couldn't get epoch time!")
        .as_millis()
}

pub fn serve_client_requests(
    con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>>,
) -> std::io::Result<()> {
    match std::fs::remove_file(util::get_swayr_socket_path()) {
        Ok(()) => println!("Deleted stale socket from previous run."),
        Err(e) => eprintln!("Could not delete socket:\n{:?}", e),
    }

    let listener = UnixListener::bind(util::get_swayr_socket_path())?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let wp_clone = con_props.clone();
                thread::spawn(move || handle_client_request(stream, wp_clone));
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn handle_client_request(
    mut stream: UnixStream,
    con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>>,
) {
    let json = serde_json::to_string(&*con_props.read().unwrap()).unwrap();
    if let Err(err) = stream.write_all(json.as_bytes()) {
        eprintln!("Error writing to client: {:?}", err);
    }
}
