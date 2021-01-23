use crate::con;
use crate::util;
use std::fmt;

pub fn switch_window() {
    let root = con::get_tree();
    let mut windows = con::get_windows(&root);
    windows.sort();

    if let Some(window) = con::select_window("Switch to window", &windows) {
        util::swaymsg(&[
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
        util::swaymsg(&["workspace", "number", workspace.get_name()]);
    }
}

pub fn quit_window() {
    let root = con::get_tree();
    let mut windows = con::get_windows(&root);
    windows.sort_by(|a, b| a.cmp(b).reverse());

    if let Some(window) = con::select_window("Quit window", &windows) {
        util::swaymsg(&[
            format!("[con_id={}]", window.get_id()).as_str(),
            "kill",
        ]);
    }
}

fn get_swaymsg_commands<'a>() -> Vec<SwaymsgCmd<'a>> {
    let mut cmds = vec![];
    cmds.push(vec!["exit"]);
    cmds.push(vec!["floating", "toggle"]);
    cmds.push(vec!["focus", "child"]);
    cmds.push(vec!["focus", "parent"]);

    for b in &["none", "normal", "csd", "pixel"] {
        cmds.push(vec!["border", b]);
    }

    cmds.push(vec!["focus", "tiling"]);
    cmds.push(vec!["focus", "floating"]);
    cmds.push(vec!["focus", "mode_toggle"]);

    cmds.push(vec!["fullscreen", "toggle"]);

    for x in &["focus", "fullscreen", "open", "none", "visible"] {
        cmds.push(vec!["inhibit_idle", x])
    }

    for l in &["default", "splith", "splitv", "stacking", "tiling"] {
        cmds.push(vec!["layout", l])
    }

    cmds.push(vec!["reload"]);

    for e in &["enable", "disable"] {
        cmds.push(vec!["shortcuts", "inhibitor", e])
    }

    cmds.push(vec!["sticky", "toggle"]);

    for x in &["yes", "no", "always"] {
        cmds.push(vec!["focus_follows_mouse", x])
    }

    for x in &["smart", "urgent", "focus", "none"] {
        cmds.push(vec!["focus_on_window_activation", x])
    }

    for x in &["yes", "no", "force", "workspace"] {
        cmds.push(vec!["focus_wrapping", x])
    }

    for x in &[
        "none",
        "vertical",
        "horizontal",
        "both",
        "smart",
        "smart_no_gaps",
    ] {
        cmds.push(vec!["hide_edge_borders", x])
    }

    cmds.push(vec!["kill"]);

    for x in &["on", "no_gaps", "off"] {
        cmds.push(vec!["smart_borders", x])
    }

    for x in &["on", "off"] {
        cmds.push(vec!["smart_gaps", x])
    }

    for x in &["output", "container", "none"] {
        cmds.push(vec!["mouse_warping", x])
    }

    for x in &["smart", "ignore", "leave_fullscreen"] {
        cmds.push(vec!["popup_during_fullscreen", x])
    }

    for x in &["yes", "no"] {
        cmds.push(vec!["show_marks", x]);
        cmds.push(vec!["workspace_auto_back_and_forth", x]);
    }

    cmds.push(vec!["tiling_drag", "toggle"]);

    for x in &["left", "center", "right"] {
        cmds.push(vec!["title_align", x]);
    }

    for x in &["enable", "disable", "allow", "deny"] {
        cmds.push(vec!["urgent", x])
    }

    cmds.sort();

    cmds.iter()
        .map(|v| SwaymsgCmd { cmd: v.to_vec() })
        .collect()
}

struct SwaymsgCmd<'a> {
    cmd: Vec<&'a str>,
}

impl<'a> fmt::Display for SwaymsgCmd<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.cmd.join(" "))
    }
}

pub fn exec_swaymsg_command() {
    let cmds = get_swaymsg_commands();
    let cmd = util::wofi_select("Execute swaymsg command", &cmds);
    if let Some(cmd) = cmd {
        util::swaymsg(&cmd.cmd);
    }
}
