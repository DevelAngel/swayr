use crate::con;
use crate::util;

pub fn switch_window() {
    let root = con::get_tree();
    let mut windows = con::get_windows(&root);
    windows.sort();

    if let Some(window) = con::select_window("Switch to window", &windows) {
        util::swaymsg(vec![
            format!("[con_id={}]", window.get_id()).as_str(),
            "focus",
        ]);
    }
}

pub fn switch_workspace() {
    let root = con::get_tree();
    let mut workspaces = con::get_workspaces(&root, false);
    workspaces.sort();

    if let Some(workspace) =
        con::select_workspace("Switch to workspace", &workspaces)
    {
        util::swaymsg(vec!["workspace", "number", workspace.get_name()]);
    }
}

pub fn quit_window() {
    let root = con::get_tree();
    let mut windows = con::get_windows(&root);
    windows.sort_by(|a, b| a.cmp(b).reverse());

    if let Some(window) = con::select_window("Quit window", &windows) {
        util::swaymsg(vec![
            format!("[con_id={}]", window.get_id()).as_str(),
            "kill",
        ]);
    }
}
