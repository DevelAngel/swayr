use crate::ipc;
use crate::util;
use crate::window;
use std::collections::HashMap;
use std::os::unix::net::UnixStream;

fn get_window_props() -> Result<HashMap<ipc::Id, ipc::WindowProps>, serde_json::Error> {
    if let Ok(sock) = UnixStream::connect(util::get_swayr_socket_path()) {
        serde_json::from_reader(sock)
    } else {
        panic!("Could not connect to socket!")
    }
}

pub fn switch_window() {
    let root_node = ipc::get_tree();
    let mut windows = window::get_windows(&root_node);
    if let Ok(win_props) = get_window_props() {
        windows.sort_unstable_by(|a, b| {
            if a.node.focused {
                std::cmp::Ordering::Greater
            } else if b.node.focused {
                std::cmp::Ordering::Less
            } else {
                let lru_a = win_props
                    .get(&a.node.id)
                    .map(|p| p.last_focus_time)
                    .unwrap_or(0);
                let lru_b = win_props
                    .get(&b.node.id)
                    .map(|p| p.last_focus_time)
                    .unwrap_or(0);
                lru_a.cmp(&lru_b).reverse()
            }
        });
    }

    if let Some(window) = util::select_window(&windows) {
        util::swaymsg(vec![
            format!("[con_id={}]", window.node.id).as_str(),
            "focus",
        ]);
    }
}
