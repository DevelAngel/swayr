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
    let root_node = get_tree();
    let mut windows = window::get_windows(&root_node);
    match get_window_props() {
        Ok(win_props) => window::sort_windows(&mut windows, win_props),
        Err(e) => eprintln!("Got no win_props: {:?}", e),
    }

    if let Some(window) = util::select_window(&windows) {
        util::swaymsg(vec![
            format!("[con_id={}]", window.node.id).as_str(),
            "focus",
        ]);
    }
}

pub fn get_tree() -> ipc::Node {
    let output = util::swaymsg(vec!["-t", "get_tree"]);
    let result = serde_json::from_str(output.as_str());

    match result {
        Ok(node) => node,
        Err(e) => {
            eprintln!("Error: {}", e);
            panic!()
        }
    }
}

#[test]
fn test_get_tree() {
    let tree = get_tree();

    println!("Those IDs are in get_tree():");
    for n in tree.iter() {
        println!("  id: {}, type: {:?}", n.id, n.r#type);
    }
}

#[test]
fn test_get_windows() {
    let tree = get_tree();
    let cons = window::get_windows(&tree);

    println!("There are {} cons.", cons.len());

    for c in cons {
        println!("  {}", c);
    }
}
