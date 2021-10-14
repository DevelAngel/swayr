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
use crate::con::NodeMethods;
use crate::config as cfg;
use crate::layout;
use crate::util;
use crate::util::DisplayFormat;
use clap::Clap;
use rand;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use swayipc as s;

#[derive(Clap, Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ConsiderFloating {
    /// Include floating windows.
    IncludeFloating,
    /// Exclude floating windows.
    ExcludeFloating,
}

#[derive(Clap, Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ConsiderWindows {
    /// Consider windows of all workspaces.
    AllWorkspaces,
    /// Consider windows of only the current workspaces.
    CurrentWorkspace,
}

#[derive(Clap, Debug, Deserialize, Serialize)]
pub enum SwayrCommand {
    /// Switch to next urgent window (if any) or to last recently used window.
    SwitchToUrgentOrLRUWindow,
    /// Focus the selected window.
    SwitchWindow,
    /// Focus the next window.
    NextWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the previous window.
    PrevWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the next window of a tiled container.
    NextTiledWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the previous window of a tiled container.
    PrevTiledWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the next window of a tabbed or stacked container.
    NextTabbedOrStackedWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the previous window of a tabbed or stacked container.
    PrevTabbedOrStackedWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    NextFloatingWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    PrevFloatingWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    NextSimilarWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    PrevSimilarWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Quit the selected window.
    QuitWindow,
    /// Switch to the selected workspace.
    SwitchWorkspace,
    /// Switch to the selected workspace or focus the selected window.
    SwitchWorkspaceOrWindow,
    /// Quit all windows of selected workspace or the selected window.
    QuitWorkspaceOrWindow,
    /// Tab or shuffle-and-tile the windows on the current workspace, including
    /// or excluding floating windows.
    ToggleTabShuffleTileWorkspace {
        #[clap(subcommand)]
        floating: ConsiderFloating,
    },
    /// Tiles the windows on the current workspace, including or excluding
    /// floating windows.
    TileWorkspace {
        #[clap(subcommand)]
        floating: ConsiderFloating,
    },
    /// Tabs the windows on the current workspace, including or excluding
    /// floating windows.
    TabWorkspace {
        #[clap(subcommand)]
        floating: ConsiderFloating,
    },
    /// Shuffles and tiles the windows on the current workspace, including or
    /// excluding floating windows.
    ShuffleTileWorkspace {
        #[clap(subcommand)]
        floating: ConsiderFloating,
    },
    /// Select and execute a swaymsg command.
    ExecuteSwaymsgCommand,
    /// Select and execute a swayr command.
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

fn always_true(_x: &con::Window) -> bool {
    true
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
        SwayrCommand::NextWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            Some(&*props.read().unwrap()),
            Box::new(always_true),
        ),
        SwayrCommand::PrevWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            Some(&*props.read().unwrap()),
            Box::new(always_true),
        ),
        SwayrCommand::NextTiledWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            Some(&*props.read().unwrap()),
            Box::new(|w: &con::Window| w.is_child_of_tiled_container()),
        ),
        SwayrCommand::PrevTiledWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            Some(&*props.read().unwrap()),
            Box::new(|w: &con::Window| w.is_child_of_tiled_container()),
        ),
        SwayrCommand::NextTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                Some(&*props.read().unwrap()),
                Box::new(|w: &con::Window| {
                    w.is_child_of_tabbed_or_stacked_container()
                }),
            )
        }
        SwayrCommand::PrevTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                Some(&*props.read().unwrap()),
                Box::new(|w: &con::Window| {
                    w.is_child_of_tabbed_or_stacked_container()
                }),
            )
        }
        SwayrCommand::NextFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                Some(&*props.read().unwrap()),
                Box::new(|w: &con::Window| w.is_floating()),
            )
        }
        SwayrCommand::PrevFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                Some(&*props.read().unwrap()),
                Box::new(|w: &con::Window| w.is_floating()),
            )
        }
        SwayrCommand::NextSimilarWindow { windows } => {
            focus_similar_window_in_direction(
                Direction::Forward,
                windows,
                Some(&*props.read().unwrap()),
            )
        }
        SwayrCommand::PrevSimilarWindow { windows } => {
            focus_similar_window_in_direction(
                Direction::Backward,
                windows,
                Some(&*props.read().unwrap()),
            )
        }
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
        SwayrCommand::TileWorkspace { floating } => {
            tile_current_workspace(floating, false)
        }
        SwayrCommand::TabWorkspace { floating } => {
            tab_current_workspace(floating)
        }
        SwayrCommand::ShuffleTileWorkspace { floating } => {
            tile_current_workspace(floating, true)
        }
        SwayrCommand::ToggleTabShuffleTileWorkspace { floating } => {
            toggle_tab_tile_current_workspace(floating)
        }
        SwayrCommand::ExecuteSwaymsgCommand => exec_swaymsg_command(),
        SwayrCommand::ExecuteSwayrCommand => {
            let mut cmds = vec![
                SwayrCommand::ExecuteSwaymsgCommand,
                SwayrCommand::QuitWindow,
                SwayrCommand::QuitWorkspaceOrWindow,
                SwayrCommand::SwitchWindow,
                SwayrCommand::SwitchWorkspace,
                SwayrCommand::SwitchWorkspaceOrWindow,
                SwayrCommand::SwitchToUrgentOrLRUWindow,
            ];
            for f in [
                ConsiderFloating::ExcludeFloating,
                ConsiderFloating::IncludeFloating,
            ] {
                cmds.push(SwayrCommand::ToggleTabShuffleTileWorkspace {
                    floating: f.clone(),
                });
                cmds.push(SwayrCommand::TileWorkspace {
                    floating: f.clone(),
                });
                cmds.push(SwayrCommand::TabWorkspace {
                    floating: f.clone(),
                });
                cmds.push(SwayrCommand::ShuffleTileWorkspace {
                    floating: f.clone(),
                });
            }
            for w in [
                ConsiderWindows::AllWorkspaces,
                ConsiderWindows::CurrentWorkspace,
            ] {
                cmds.push(SwayrCommand::NextWindow { windows: w.clone() });
                cmds.push(SwayrCommand::PrevWindow { windows: w.clone() });
                cmds.push(SwayrCommand::NextTiledWindow { windows: w.clone() });
                cmds.push(SwayrCommand::PrevTiledWindow { windows: w.clone() });
                cmds.push(SwayrCommand::NextTabbedOrStackedWindow {
                    windows: w.clone(),
                });
                cmds.push(SwayrCommand::PrevTabbedOrStackedWindow {
                    windows: w.clone(),
                });
                cmds.push(SwayrCommand::NextFloatingWindow {
                    windows: w.clone(),
                });
                cmds.push(SwayrCommand::PrevFloatingWindow {
                    windows: w.clone(),
                })
            }

            if let Some(c) =
                util::select_from_menu("Select swayr command", &cmds)
            {
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

pub fn focus_window_in_direction(
    dir: Direction,
    consider_wins: &ConsiderWindows,
    extra_props: Option<&HashMap<i64, con::ExtraProps>>,
    pred: Box<dyn Fn(&con::Window) -> bool>,
) {
    let root = get_tree();
    let root = match consider_wins {
        ConsiderWindows::AllWorkspaces => &root,
        ConsiderWindows::CurrentWorkspace => {
            con::get_workspaces(&root, false, extra_props)
                .iter()
                .find(|w| w.is_current())
                .expect("No current workspace!")
                .node
        }
    };

    let windows = con::get_windows(root, false, extra_props);

    if windows.len() < 2 {
        return;
    }

    let is_focused_window: Box<dyn Fn(&con::Window) -> bool> =
        if !windows.iter().any(|w| w.is_focused()) {
            let last_focused_win_id = windows.get(0).unwrap().get_id();
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
        if is_focused_window(win) {
            let win = iter.find(|w| pred(w)).unwrap();
            focus_window_by_id(win.get_id());
            return;
        }
    }
}

pub fn focus_similar_window_in_direction(
    dir: Direction,
    consider_wins: &ConsiderWindows,
    extra_props: Option<&HashMap<i64, con::ExtraProps>>,
) {
    let root = get_tree();
    let windows = con::get_windows(&root, false, extra_props);
    let current_window = windows
        .iter()
        .find(|w| w.is_focused())
        .or_else(|| windows.get(0));

    if let Some(current_window) = current_window {
        focus_window_in_direction(
            dir,
            consider_wins,
            extra_props,
            if current_window.is_floating() {
                Box::new(|w| w.is_floating())
            } else if current_window.is_child_of_tabbed_or_stacked_container() {
                Box::new(|w| w.is_child_of_tabbed_or_stacked_container())
            } else if current_window.is_child_of_tiled_container() {
                Box::new(|w| w.is_child_of_tiled_container())
            } else {
                Box::new(always_true)
            },
        )
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

fn tile_current_workspace(floating: &ConsiderFloating, shuffle: bool) {
    match layout::relayout_current_workspace(
        floating == &ConsiderFloating::IncludeFloating,
        Box::new(move |wins, con: &mut s::Connection| {
            con.run_command("focus parent".to_string())?;
            con.run_command("layout splith".to_string())?;

            let mut placed_wins = vec![];
            let mut rng = rand::thread_rng();
            if shuffle {
                wins.shuffle(&mut rng);
            } else {
                wins.reverse()
            }
            for win in wins {
                if win.is_floating() {
                    con.run_command(format!(
                        "[con_id={}] floating disable",
                        win.get_id()
                    ))?;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
                con.run_command(format!(
                    "[con_id={}] move to workspace current",
                    win.get_id()
                ))?;
                placed_wins.push(win);
                if shuffle {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                    if let Some(win) = placed_wins.choose(&mut rng) {
                        con.run_command(format!(
                            "[con_id={}] focus",
                            win.get_id()
                        ))?;
                    }
                }
            }
            Ok(())
        }),
    ) {
        Ok(_) => (),
        Err(err) => eprintln!("Error retiling workspace: {:?}", err),
    }
}

fn tab_current_workspace(floating: &ConsiderFloating) {
    match layout::relayout_current_workspace(
        floating == &ConsiderFloating::IncludeFloating,
        Box::new(move |wins, con: &mut s::Connection| {
            con.run_command("focus parent".to_string())?;
            con.run_command("layout tabbed".to_string())?;

            let mut placed_wins = vec![];
            wins.reverse();
            for win in wins {
                if win.is_floating() {
                    con.run_command(format!(
                        "[con_id={}] floating disable",
                        win.get_id()
                    ))?;
                }

                std::thread::sleep(std::time::Duration::from_millis(25));
                con.run_command(format!(
                    "[con_id={}] move to workspace current",
                    win.get_id()
                ))?;
                placed_wins.push(win);
            }
            Ok(())
        }),
    ) {
        Ok(_) => (),
        Err(err) => eprintln!("Error retiling workspace: {:?}", err),
    }
}

fn toggle_tab_tile_current_workspace(floating: &ConsiderFloating) {
    let tree = get_tree();
    let workspaces = tree.workspaces();
    let cur_ws = workspaces
        .iter()
        .find(|w| con::is_current_container(w))
        .unwrap();
    if cur_ws.layout == s::NodeLayout::Tabbed {
        tile_current_workspace(floating, true);
    } else {
        tab_current_workspace(floating);
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
