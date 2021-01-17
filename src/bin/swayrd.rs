extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use swayr::demon;
use swayr::ipc;

fn main() {
    let win_props: Arc<RwLock<HashMap<ipc::Id, ipc::WindowProps>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let win_props_for_ev_handler = win_props.clone();

    let subscriber_handle =
        thread::spawn(move || demon::monitor_window_events(win_props_for_ev_handler));

    let subscriber_result = subscriber_handle.join();
    match subscriber_result {
        Ok(()) => println!("Subscriber thread shut down cleanly."),
        Err(err) => panic!(err),
    }
}
