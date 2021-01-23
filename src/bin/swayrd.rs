extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use swayr::demon;
use swayr::ipc;

fn main() {
    let con_props: Arc<RwLock<HashMap<ipc::Id, ipc::ConProps>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let con_props_for_ev_handler = con_props.clone();

    let subscriber_handle = thread::spawn(move || {
        demon::monitor_window_events(con_props_for_ev_handler)
    });

    match demon::serve_client_requests(con_props) {
        Ok(()) => {
            let subscriber_result = subscriber_handle.join();
            match subscriber_result {
                Ok(()) => println!("Subscriber thread shut down cleanly."),
                Err(err) => panic!(err),
            }
        }
        Err(err) => panic!(err),
    }
}
