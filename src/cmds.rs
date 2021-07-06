// Copyright (C) 2021  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

//! Functions and data structures of the swayr client.

use crate::con;
use crate::config as cfg;
use crate::util;
use crate::util::DisplayFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use swayipc as s;

use clap::Clap;

#[derive(Clap, Debug, Deserialize, Serialize)]
pub enum SwayrCommand {
    /// Switch to next urgent window (if any) or to last recently used window.
    SwitchToUrgentOrLRUWindow,
    /// Focus the selected window
    SwitchWindow,
    /// Focus the next window.
    NextWindow,
    /// Focus the previous window.
    PrevWindow,
    /// Quit the selected window
    QuitWindow,
    /// Switch to the selected workspace
    SwitchWorkspace,
    /// Switch to the selected workspace or focus the selected window
    SwitchWorkspaceOrWindow,
    /// Quit all windows of selected workspace or the selected window
    QuitWorkspaceOrWindow,
    /// Select and execute a swaymsg command
    ExecuteSwaymsgCommand,
    /// Select and execute a swayr command
    ExecuteSwayrCommand,
}

pub struct ExecSwayrCmdArgs<'a> {
    pub cmd: &'a SwayrCommand,
    pub extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
}

impl DisplayFormat for SwayrCommand {
    fn format_for_display(&self, _: &cfg::Config) -> std::string::String {
        // TODO: Add a format to Config
        format!("{:?}", self)
    }
}

pub fn exec_swayr_cmd(args: ExecSwayrCmdArgs) {
    let props = args.extra_props;
    match args.cmd {
        SwayrCommand::SwitchToUrgentOrLRUWindow => {
            switch_to_urgent_or_lru_window(Some(&*props.read().unwrap()))
        }
        SwayrCommand::SwitchWindow => {
            switch_window(Some(&*props.read().unwrap()))
        }
        SwayrCommand::NextWindow => focus_next_window_in_direction(
            Direction::Forward,
            Some(&*props.read().unwrap()),
        ),
        SwayrCommand::PrevWindow => focus_next_window_in_direction(
            Direction::Backward,
            Some(&*props.read().unwrap()),
        ),
        SwayrCommand::QuitWindow => quit_window(Some(&*props.read().unwrap())),
        SwayrCommand::SwitchWorkspace => {
            switch_workspace(Some(&*props.read().unwrap()))
        }
        SwayrCommand::SwitchWorkspaceOrWindow => {
            switch_workspace_or_window(Some(&*props.read().unwrap()))
        }
        SwayrCommand::QuitWorkspaceOrWindow => {
            quit_workspace_or_window(Some(&*props.read().unwrap()))
        }
        SwayrCommand::ExecuteSwaymsgCommand => exec_swaymsg_command(),
        SwayrCommand::ExecuteSwayrCommand => {
            if let Some(c) = util::select_from_menu(
                "Select swayr command",
                &[
                    SwayrCommand::ExecuteSwaymsgCommand,
                    SwayrCommand::QuitWindow,
                    SwayrCommand::QuitWorkspaceOrWindow,
                    SwayrCommand::SwitchWindow,
                    SwayrCommand::SwitchWorkspace,
                    SwayrCommand::SwitchWorkspaceOrWindow,
                    SwayrCommand::SwitchToUrgentOrLRUWindow,
                    SwayrCommand::NextWindow,
                    SwayrCommand::PrevWindow,
                ],
            ) {
                exec_swayr_cmd(ExecSwayrCmdArgs {
                    cmd: c,
                    extra_props: props,
                });
            }
        }
    }
}

fn focus_window_by_id(id: i64) {
    run_sway_command(&[format!("[con_id={}]", id).as_str(), "focus"]);
}

fn quit_window_by_id(id: i64) {
    run_sway_command(&[format!("[con_id={}]", id).as_str(), "kill"]);
}

pub fn get_tree() -> s::Node {
    match s::Connection::new() {
        Ok(mut con) => con.get_tree().expect("Got no root node"),
        Err(err) => panic!("{}", err),
    }
}

pub fn switch_to_urgent_or_lru_window(
    extra_props: Option<&HashMap<i64, con::ExtraProps>>,
) {
    let root = get_tree();
    let windows = con::get_windows(&root, false, extra_props);
    if let Some(win) = windows
        .iter()
        .find(|w| w.is_urgent())
        .or_else(|| windows.get(0))
    {
        println!("Switching to {}, id: {}", win.get_app_name(), win.get_id());
        focus_window_by_id(win.get_id())
    } else {
        println!("No window to switch to.")
    }
}

pub fn switch_window(extra_props: Option<&HashMap<i64, con::ExtraProps>>) {
    let root = get_tree();
    let windows = con::get_windows(&root, true, extra_props);

    if let Some(window) = con::select_window("Switch to window", &windows) {
        focus_window_by_id(window.get_id())
    }
}

pub enum Direction {
    Backward,
    Forward,
}

pub fn focus_next_window_in_direction(
    dir: Direction,
    extra_props: Option<&HashMap<i64, con::ExtraProps>>,
) {
    let root = get_tree();
    let windows = con::get_windows(&root, false, None);

    if windows.len() < 2 {
        return;
    }

    let pred: Box<dyn Fn(&con::Window) -> bool> =
        if !windows.iter().any(|w| w.is_focused()) {
            let last_focused_win_id =
                con::get_windows(&root, false, extra_props)
                    .get(0)
                    .unwrap()
                    .get_id();
            Box::new(move |w| w.get_id() == last_focused_win_id)
        } else {
            Box::new(|w: &con::Window| w.is_focused())
        };

    let mut iter: Box<dyn Iterator<Item = &con::Window>> = match dir {
        Direction::Forward => Box::new(windows.iter().rev().cycle()),
        Direction::Backward => Box::new(windows.iter().cycle()),
    };

    loop {
        let win = iter.next().unwrap();
        if pred(win) {
            let win = iter.next().unwrap();
            focus_window_by_id(win.get_id());
            return;
        }
    }
}

pub fn switch_workspace(extra_props: Option<&HashMap<i64, con::ExtraProps>>) {
    let root = get_tree();
    let workspaces = con::get_workspaces(&root, false, extra_props);

    if let Some(workspace) =
        con::select_workspace("Switch to workspace", &workspaces)
    {
        run_sway_command(&["workspace", "number", workspace.get_name()]);
    }
}

pub fn switch_workspace_or_window(
    extra_props: Option<&HashMap<i64, con::ExtraProps>>,
) {
    let root = get_tree();
    let workspaces = con::get_workspaces(&root, true, extra_props);
    let ws_or_wins = con::WsOrWin::from_workspaces(&workspaces);
    if let Some(ws_or_win) = con::select_workspace_or_window(
        "Select workspace or window",
        &ws_or_wins,
    ) {
        match ws_or_win {
            con::WsOrWin::Ws { ws } => {
                run_sway_command(&["workspace", "number", ws.get_name()]);
            }
            con::WsOrWin::Win { win } => focus_window_by_id(win.get_id()),
        }
    }
}

pub fn quit_window(extra_props: Option<&HashMap<i64, con::ExtraProps>>) {
    let root = get_tree();
    let windows = con::get_windows(&root, true, extra_props);

    if let Some(window) = con::select_window("Quit window", &windows) {
        quit_window_by_id(window.get_id())
    }
}

pub fn quit_workspace_or_window(
    extra_props: Option<&HashMap<i64, con::ExtraProps>>,
) {
    let root = get_tree();
    let workspaces = con::get_workspaces(&root, true, extra_props);
    let ws_or_wins = con::WsOrWin::from_workspaces(&workspaces);
    if let Some(ws_or_win) =
        con::select_workspace_or_window("Quit workspace or window", &ws_or_wins)
    {
        match ws_or_win {
            con::WsOrWin::Ws { ws } => {
                for win in &ws.windows {
                    quit_window_by_id(win.get_id())
                }
            }
            con::WsOrWin::Win { win } => quit_window_by_id(win.get_id()),
        }
    }
}

fn get_swaymsg_commands<'a>() -> Vec<SwaymsgCmd<'a>> {
    let mut cmds = vec![];

    for b in &["none", "normal", "csd", "pixel"] {
        cmds.push(vec!["border", b]);
    }

    cmds.push(vec!["exit"]);
    cmds.push(vec!["floating", "toggle"]);
    cmds.push(vec!["focus", "child"]);
    cmds.push(vec!["focus", "parent"]);
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
        cmds.push(vec!["shortcuts_inhibitor", e])
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

impl DisplayFormat for SwaymsgCmd<'_> {
    fn format_for_display(&self, _: &cfg::Config) -> std::string::String {
        self.cmd.join(" ")
    }
}

pub fn exec_swaymsg_command() {
    let cmds = get_swaymsg_commands();
    let cmd = util::select_from_menu("Execute swaymsg command", &cmds);
    if let Some(cmd) = cmd {
        run_sway_command(&cmd.cmd);
    }
}

pub fn run_sway_command(args: &[&str]) {
    let cmd = args.join(" ");
    println!("Running sway command: {}", cmd);
    match s::Connection::new() {
        Ok(mut con) => {
            if let Err(err) = con.run_command(cmd) {
                eprintln!("Could not run sway command: {}", err)
            }
        }
        Err(err) => panic!("{}", err),
    }
}
