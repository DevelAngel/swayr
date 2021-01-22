use crate::ipc;
use crate::util;
use crate::window;

pub fn switch_window() {
    let root_node = get_tree();
    let mut windows = window::get_windows(&root_node);
    windows.sort();

    if let Some(window) = util::select_window("Switch to window", &windows) {
        util::swaymsg(vec![
            format!("[con_id={}]", window.get_id()).as_str(),
            "focus",
        ]);
    }
}

pub fn quit_window() {
    let root_node = get_tree();
    let mut windows = window::get_windows(&root_node);
    windows.sort_by(|a, b| a.cmp(b).reverse());

    if let Some(window) = util::select_window("Quit window", &windows) {
        util::swaymsg(vec![
            format!("[con_id={}]", window.get_id()).as_str(),
            "kill",
        ]);
    }
}

fn get_tree() -> ipc::Node {
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
