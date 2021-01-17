use crate::ipc;
use serde_json::Deserializer;
use std::collections::HashMap;
use std::process as proc;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn monitor_window_events(win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>>) {
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
    let stream = Deserializer::from_reader(stdout).into_iter::<ipc::WindowEvent>();
    for res in stream {
        match res {
            Ok(win_ev) => handle_window_event(win_ev, win_props.clone()),
            Err(err) => panic!("{:?}", err),
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
