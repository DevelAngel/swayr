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
    win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>>,
) {
    let child = proc::Command::new("swaymsg")
        .arg("--monitor")
        .arg("--raw")
        .arg("-t")
        .arg("subscribe")
        .arg("[\"window\"]")
        .stdout(proc::Stdio::piped())
        .spawn()
        .expect("Failed to subscribe to window events");
    let stdout: std::process::ChildStdout = child.stdout.unwrap();
    let stream =
        Deserializer::from_reader(stdout).into_iter::<ipc::WindowEvent>();
    for res in stream {
        match res {
            Ok(win_ev) => handle_window_event(win_ev, win_props.clone()),
            Err(err) => eprintln!("Error handling window event:\n{:?}", err),
        }
    }
}

fn handle_window_event(
    ev: ipc::WindowEvent,
    win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>>,
) {
    match ev.change {
        ipc::WindowEventType::Focus => {
            let mut write_lock = win_props.write().unwrap();
            if let Some(mut wp) = write_lock.get_mut(&ev.container.id) {
                wp.last_focus_time = get_epoch_time_as_millis();
            } else {
                write_lock.insert(
                    ev.container.id,
                    ipc::WindowProps {
                        last_focus_time: get_epoch_time_as_millis(),
                    },
                );
            }
        }
        ipc::WindowEventType::Close => {
            win_props.write().unwrap().remove(&ev.container.id);
        }
        _ => (),
    }
}

fn get_epoch_time_as_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Couldn't get epoch time!")
        .as_millis()
}

pub fn serve_client_requests(
    win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>>,
) -> std::io::Result<()> {
    match std::fs::remove_file(util::get_swayr_socket_path()) {
        Ok(()) => println!("Deleted stale socket from previous run."),
        Err(e) => eprintln!("Could not delete socket:\n{:?}", e),
    }

    let listener = UnixListener::bind(util::get_swayr_socket_path())?;
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let wp_clone = win_props.clone();
                thread::spawn(move || handle_client_request(stream, wp_clone));
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn handle_client_request(
    mut stream: UnixStream,
    win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>>,
) {
    let json = serde_json::to_string(&*win_props.read().unwrap()).unwrap();
    if let Err(err) = stream.write_all(json.as_bytes()) {
        eprintln!("Error writing to client: {:?}", err);
    }
}
