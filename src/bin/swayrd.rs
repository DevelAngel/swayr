//! The `swayrd` binary.

extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use swayr::demon;
use swayr::ipc;

fn main() {
    let con_props: Arc<RwLock<HashMap<i64, ipc::ExtraProps>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let con_props_for_ev_handler = con_props.clone();

    thread::spawn(move || {
        demon::monitor_sway_events(con_props_for_ev_handler);
    });

    demon::serve_client_requests(con_props);
}
