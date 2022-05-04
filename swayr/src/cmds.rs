// Copyright (C) 2021-2022  Tassilo Horn <tsdh@gnu.org>
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

use crate::config as cfg;
use crate::layout;
use crate::shared::ipc;
use crate::shared::ipc::NodeMethods;
use crate::tree as t;
use crate::util;
use crate::util::DisplayFormat;
use once_cell::sync::Lazy;
use rand::prelude::SliceRandom;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic;
use std::sync::Arc;
use std::sync::RwLock;
use swayipc as s;

pub fn run_sway_command_1(cmd: &str) {
    log::debug!("Running sway command: {}", cmd);
    match s::Connection::new() {
        Ok(mut con) => {
            if let Err(err) = con.run_command(cmd) {
                log::error!("Could not run sway command: {}", err)
            }
        }
        Err(err) => panic!("{}", err),
    }
}

pub fn run_sway_command(args: &[&str]) {
    let cmd = args.join(" ");
    run_sway_command_1(&cmd);
}

#[derive(clap::Parser, Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ConsiderFloating {
    /// Include floating windows.
    IncludeFloating,
    /// Exclude floating windows.
    ExcludeFloating,
}

#[derive(clap::Parser, Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum ConsiderWindows {
    /// Consider windows of all workspaces.
    AllWorkspaces,
    /// Consider windows of only the current workspaces.
    CurrentWorkspace,
}

#[derive(clap::Parser, Debug, Deserialize, Serialize)]
pub enum SwayrCommand {
    /// No-operation. Interrupts any in-progress prev/next sequence but has
    /// no other effect
    Nop,
    /// Switch to next urgent window (if any) or to last recently used window.
    SwitchToUrgentOrLRUWindow,
    /// Switch to the given app (given by app_id or window class) if that's not
    /// focused already.  If it is, switch to the next urgent window (if any)
    /// or to last recently used window.
    ///
    /// For example, you can provide "firefox" as argument to this command to
    /// have a convenient firefox <-> last-recently-used window toggle.
    SwitchToAppOrUrgentOrLRUWindow { name: String },
    /// Switch to the window with the given mark if that's not focused already.
    /// If it is, switch to the next urgent window (if any) or to last recently
    /// used window.
    ///
    /// For example, you can assign a "browser" mark to your browser window
    /// (using a standard sway `for_window` rule).  Then you can provide
    /// "browser" as argument to this command to have a convenient browser <->
    /// last-recently-used window toggle.
    SwitchToMarkOrUrgentOrLRUWindow { con_mark: String },
    /// Focus the selected window.
    SwitchWindow,
    /// Switch to the selected workspace.
    SwitchWorkspace,
    /// Switch to the selected workspace.
    SwitchOutput,
    /// Switch to the selected workspace or focus the selected window.
    SwitchWorkspaceOrWindow,
    /// Switch to the selected workspace or focus the selected container, or
    /// window.
    SwitchWorkspaceContainerOrWindow,
    /// Switch to the selected output or workspace or focus the selected
    /// container, or window.
    SwitchTo,
    /// Quit the selected window.
    QuitWindow {
        #[clap(
            short,
            long,
            help = "Kill the window's process rather than just quitting it"
        )]
        kill: bool,
    },
    /// Quit all windows of selected workspace or the selected window.
    QuitWorkspaceOrWindow,
    /// Quit all windows of selected workspace, or container or the selected
    /// window.
    QuitWorkspaceContainerOrWindow,
    /// Focus the next window in LRU order.
    NextWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the previous window in LRU order.
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
    /// Focus the next floating window.
    NextFloatingWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the previous floating window.
    PrevFloatingWindow {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the next window having the same layout as the current one.
    NextWindowOfSameLayout {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Focus the previous window having the same layout as the current one.
    PrevWindowOfSameLayout {
        #[clap(subcommand)]
        windows: ConsiderWindows,
    },
    /// Move the currently focused window or container to the selected
    /// workspace.
    MoveFocusedToWorkspace,
    /// Move the currently focused window or container to the selected output,
    /// workspace, container or window.
    MoveFocusedTo,
    /// Swap the currently focused window or container with the selected
    /// container or window.
    SwapFocusedWith,
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
    /// Configure outputs.
    ConfigureOutputs,
}

impl SwayrCommand {
    fn is_prev_next_window_variant(&self) -> bool {
        matches!(
            self,
            SwayrCommand::NextWindow { .. }
                | SwayrCommand::PrevWindow { .. }
                | SwayrCommand::NextTiledWindow { .. }
                | SwayrCommand::PrevTiledWindow { .. }
                | SwayrCommand::NextTabbedOrStackedWindow { .. }
                | SwayrCommand::PrevTabbedOrStackedWindow { .. }
                | SwayrCommand::NextFloatingWindow { .. }
                | SwayrCommand::PrevFloatingWindow { .. }
                | SwayrCommand::NextWindowOfSameLayout { .. }
                | SwayrCommand::PrevWindowOfSameLayout { .. }
        )
    }
}

pub struct ExecSwayrCmdArgs<'a> {
    pub cmd: &'a SwayrCommand,
    pub extra_props: Arc<RwLock<HashMap<i64, t::ExtraProps>>>,
}

impl DisplayFormat for SwayrCommand {
    fn format_for_display(&self, _: &cfg::Config) -> std::string::String {
        // TODO: It would be very nice if the display format was exactly like
        // the swayr invocation in the shell.  Can that somehow be retrieved
        // from clap?
        format!("{:?}", self)
    }

    fn get_indent_level(&self) -> usize {
        0
    }
}

fn always_true(_x: &t::DisplayNode) -> bool {
    true
}

static IN_NEXT_PREV_WINDOW_SEQ: atomic::AtomicBool =
    atomic::AtomicBool::new(false);

pub fn exec_swayr_cmd(args: ExecSwayrCmdArgs) {
    let props = args.extra_props;

    if args.cmd.is_prev_next_window_variant() {
        let before =
            IN_NEXT_PREV_WINDOW_SEQ.swap(true, atomic::Ordering::SeqCst);
        if !before {
            let mut map = props.write().unwrap();
            for val in map.values_mut() {
                val.last_focus_tick_for_next_prev_seq = val.last_focus_tick;
            }
        }
    } else {
        IN_NEXT_PREV_WINDOW_SEQ.store(false, atomic::Ordering::SeqCst);
    }

    match args.cmd {
        SwayrCommand::Nop => {}
        SwayrCommand::SwitchToUrgentOrLRUWindow => {
            switch_to_urgent_or_lru_window(&*props.read().unwrap())
        }
        SwayrCommand::SwitchToAppOrUrgentOrLRUWindow { name } => {
            switch_to_app_or_urgent_or_lru_window(
                Some(name),
                &*props.read().unwrap(),
            )
        }
        SwayrCommand::SwitchToMarkOrUrgentOrLRUWindow { con_mark } => {
            switch_to_mark_or_urgent_or_lru_window(
                Some(con_mark),
                &*props.read().unwrap(),
            )
        }
        SwayrCommand::SwitchWindow => switch_window(&*props.read().unwrap()),
        SwayrCommand::SwitchWorkspace => {
            switch_workspace(&*props.read().unwrap())
        }
        SwayrCommand::SwitchOutput => switch_output(&*props.read().unwrap()),
        SwayrCommand::SwitchWorkspaceOrWindow => {
            switch_workspace_or_window(&*props.read().unwrap())
        }
        SwayrCommand::SwitchWorkspaceContainerOrWindow => {
            switch_workspace_container_or_window(&*props.read().unwrap())
        }
        SwayrCommand::SwitchTo => switch_to(&*props.read().unwrap()),
        SwayrCommand::QuitWindow { kill } => {
            quit_window(&*props.read().unwrap(), *kill)
        }
        SwayrCommand::QuitWorkspaceOrWindow => {
            quit_workspace_or_window(&*props.read().unwrap())
        }
        SwayrCommand::QuitWorkspaceContainerOrWindow => {
            quit_workspace_container_or_window(&*props.read().unwrap())
        }
        SwayrCommand::MoveFocusedToWorkspace => {
            move_focused_to_workspace(&*props.read().unwrap())
        }
        SwayrCommand::MoveFocusedTo => move_focused_to(&*props.read().unwrap()),
        SwayrCommand::SwapFocusedWith => {
            swap_focused_with(&*props.read().unwrap())
        }
        SwayrCommand::NextWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            &*props.read().unwrap(),
            Box::new(always_true),
        ),
        SwayrCommand::PrevWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            &*props.read().unwrap(),
            Box::new(always_true),
        ),
        SwayrCommand::NextTiledWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            &*props.read().unwrap(),
            Box::new(|dn: &t::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            }),
        ),
        SwayrCommand::PrevTiledWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            &*props.read().unwrap(),
            Box::new(|dn: &t::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            }),
        ),
        SwayrCommand::NextTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                &*props.read().unwrap(),
                Box::new(|dn: &t::DisplayNode| {
                    !dn.node.is_floating()
                        && dn
                            .tree
                            .is_child_of_tabbed_or_stacked_container(dn.node.id)
                }),
            )
        }
        SwayrCommand::PrevTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                &*props.read().unwrap(),
                Box::new(|dn: &t::DisplayNode| {
                    !dn.node.is_floating()
                        && dn
                            .tree
                            .is_child_of_tabbed_or_stacked_container(dn.node.id)
                }),
            )
        }
        SwayrCommand::NextFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                &*props.read().unwrap(),
                Box::new(|dn: &t::DisplayNode| dn.node.is_floating()),
            )
        }
        SwayrCommand::PrevFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                &*props.read().unwrap(),
                Box::new(|dn: &t::DisplayNode| dn.node.is_floating()),
            )
        }
        SwayrCommand::NextWindowOfSameLayout { windows } => {
            focus_window_of_same_layout_in_direction(
                Direction::Forward,
                windows,
                &*props.read().unwrap(),
            )
        }
        SwayrCommand::PrevWindowOfSameLayout { windows } => {
            focus_window_of_same_layout_in_direction(
                Direction::Backward,
                windows,
                &*props.read().unwrap(),
            )
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
        SwayrCommand::ConfigureOutputs => configure_outputs(),
        SwayrCommand::ExecuteSwaymsgCommand => exec_swaymsg_command(),
        SwayrCommand::ExecuteSwayrCommand => {
            let mut cmds = vec![
                SwayrCommand::MoveFocusedToWorkspace,
                SwayrCommand::MoveFocusedTo,
                SwayrCommand::SwapFocusedWith,
                SwayrCommand::QuitWorkspaceOrWindow,
                SwayrCommand::SwitchWindow,
                SwayrCommand::SwitchWorkspace,
                SwayrCommand::SwitchOutput,
                SwayrCommand::SwitchWorkspaceOrWindow,
                SwayrCommand::SwitchToUrgentOrLRUWindow,
                SwayrCommand::ConfigureOutputs,
                SwayrCommand::ExecuteSwaymsgCommand,
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

            for kill in [false, true] {
                cmds.push(SwayrCommand::QuitWindow { kill });
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

            if let Ok(c) = util::select_from_menu("Select swayr command", &cmds)
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

pub fn get_outputs() -> Vec<s::Output> {
    match s::Connection::new() {
        Ok(mut con) => con.get_outputs().expect("Got no outputs"),
        Err(err) => panic!("{}", err),
    }
}

pub fn switch_to_urgent_or_lru_window(
    extra_props: &HashMap<i64, t::ExtraProps>,
) {
    switch_to_app_or_urgent_or_lru_window(None, extra_props)
}

pub fn switch_to_app_or_urgent_or_lru_window(
    name: Option<&str>,
    extra_props: &HashMap<i64, t::ExtraProps>,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root, extra_props);
    let wins = tree.get_windows();
    let app_win =
        name.and_then(|n| wins.iter().find(|w| w.node.get_app_name() == n));
    focus_win_if_not_focused(app_win, wins.get(0))
}

pub fn switch_to_mark_or_urgent_or_lru_window(
    con_mark: Option<&str>,
    extra_props: &HashMap<i64, t::ExtraProps>,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root, extra_props);
    let wins = tree.get_windows();
    let marked_win = con_mark.and_then(|mark| {
        wins.iter()
            .find(|w| w.node.marks.contains(&mark.to_owned()))
    });
    focus_win_if_not_focused(marked_win, wins.get(0))
}

pub fn focus_win_if_not_focused(
    win: Option<&t::DisplayNode>,
    other: Option<&t::DisplayNode>,
) {
    match win {
        Some(win) if !win.node.is_current() => focus_window_by_id(win.node.id),
        _ => {
            if let Some(win) = other {
                focus_window_by_id(win.node.id)
            } else {
                log::debug!("No window to switch to.")
            }
        }
    }
}

static DIGIT_AND_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d):(.*)").unwrap());

fn create_workspace(ws_name: &str) {
    if DIGIT_AND_NAME.is_match(ws_name) {
        run_sway_command(&["workspace", "number", ws_name]);
    } else {
        run_sway_command(&["workspace", ws_name]);
    }
}

static SPECIAL_WORKSPACE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#*w:(.*)").unwrap());
static SPECIAL_SWAY: Lazy<regex::Regex> =
    Lazy::new(|| Regex::new(r"^#*s:(.*)").unwrap());

fn chop_workspace_shortcut(input: &str) -> &str {
    match SPECIAL_WORKSPACE.captures(input) {
        Some(c) => c.get(1).unwrap().as_str(),
        None => input,
    }
}

fn chop_sway_shortcut(input: &str) -> &str {
    match SPECIAL_SWAY.captures(input) {
        Some(c) => c.get(1).unwrap().as_str(),
        None => input,
    }
}

fn handle_non_matching_input(input: &str) {
    if input.is_empty() {
        return;
    }

    if let Some(c) = SPECIAL_SWAY.captures(input) {
        run_sway_command(&c[1].split_ascii_whitespace().collect::<Vec<&str>>());
    } else {
        let ws = chop_workspace_shortcut(input);
        create_workspace(ws);
    }
}

fn select_and_focus(prompt: &str, choices: &[t::DisplayNode]) {
    match util::select_from_menu(prompt, choices) {
        Ok(tn) => match tn.node.get_type() {
            ipc::Type::Output => {
                if !tn.node.is_scratchpad() {
                    run_sway_command(&["focus output", tn.node.get_name()]);
                }
            }
            ipc::Type::Workspace => {
                if !tn.node.is_scratchpad() {
                    run_sway_command(&["workspace", tn.node.get_name()]);
                }
            }
            ipc::Type::Window | ipc::Type::Container => {
                focus_window_by_id(tn.node.id)
            }
            t => {
                log::error!("Cannot handle {:?} in select_and_focus", t)
            }
        },
        Err(non_matching_input) => {
            handle_non_matching_input(&non_matching_input)
        }
    }
}

pub fn switch_window(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_focus("Select window", &tree.get_windows());
}

pub fn switch_workspace(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root, extra_props);
    select_and_focus("Select workspace", &tree.get_workspaces());
}

pub fn switch_output(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root, extra_props);
    select_and_focus("Select output", &tree.get_outputs());
}

pub fn switch_workspace_or_window(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_focus(
        "Select workspace or window",
        &tree.get_workspaces_and_windows(),
    );
}

pub fn switch_workspace_container_or_window(
    extra_props: &HashMap<i64, t::ExtraProps>,
) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_focus(
        "Select workspace, container or window",
        &tree.get_workspaces_containers_and_windows(),
    );
}

pub fn switch_to(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_focus(
        "Select output, workspace, container or window",
        &tree.get_outputs_workspaces_containers_and_windows(),
    );
}

fn kill_process_by_pid(pid: Option<i32>) {
    if let Some(pid) = pid {
        if let Err(err) = std::process::Command::new("kill")
            .arg("-9")
            .arg(format!("{}", pid))
            .output()
        {
            log::error!("Error killing process {}: {}", pid, err)
        }
    } else {
        log::error!("Cannot kill window with no pid.");
    }
}

fn select_and_quit(prompt: &str, choices: &[t::DisplayNode], kill: bool) {
    if let Ok(tn) = util::select_from_menu(prompt, choices) {
        match tn.node.get_type() {
            ipc::Type::Workspace | ipc::Type::Container => {
                for win in
                    tn.node.iter().filter(|n| n.get_type() == ipc::Type::Window)
                {
                    quit_window_by_id(win.id)
                }
            }
            ipc::Type::Window => {
                if kill {
                    kill_process_by_pid(tn.node.pid)
                } else {
                    quit_window_by_id(tn.node.id)
                }
            }
            t => {
                log::error!("Cannot handle {:?} in quit_workspace_or_window", t)
            }
        }
    }
}

pub fn quit_window(extra_props: &HashMap<i64, t::ExtraProps>, kill: bool) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_quit("Quit window", &tree.get_windows(), kill);
}

pub fn quit_workspace_or_window(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_quit(
        "Quit workspace or window",
        &tree.get_workspaces_and_windows(),
        false,
    );
}

pub fn quit_workspace_container_or_window(
    extra_props: &HashMap<i64, t::ExtraProps>,
) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_quit(
        "Quit workspace, container or window",
        &tree.get_workspaces_containers_and_windows(),
        false,
    );
}

fn move_focused_to_workspace_1(ws_name: &str) {
    if DIGIT_AND_NAME.is_match(ws_name) {
        run_sway_command(&[
            "move",
            "container",
            "to",
            "workspace",
            "number",
            ws_name,
        ]);
    } else {
        run_sway_command(&["move", "container", "to", "workspace", ws_name]);
    }
}

fn move_focused_to_container_or_window(id: i64) {
    run_sway_command(&[
        &format!("[con_id=\"{}\"]", id),
        "mark",
        "--add",
        "__SWAYR_MOVE_TARGET__",
    ]);
    run_sway_command(&["move", "to", "mark", "__SWAYR_MOVE_TARGET__"]);
    run_sway_command(&["unmark", "__SWAYR_MOVE_TARGET__"]);
}

fn select_and_move_focused_to(prompt: &str, choices: &[t::DisplayNode]) {
    match util::select_from_menu(prompt, choices) {
        Ok(tn) => match tn.node.get_type() {
            ipc::Type::Output => {
                if tn.node.is_scratchpad() {
                    run_sway_command_1("move container to scratchpad")
                } else {
                    run_sway_command(&[
                        "move container to output",
                        tn.node.get_name(),
                    ])
                }
            }
            ipc::Type::Workspace => {
                if tn.node.is_scratchpad() {
                    run_sway_command_1("move container to scratchpad")
                } else {
                    move_focused_to_workspace_1(tn.node.get_name())
                }
            }
            ipc::Type::Container | ipc::Type::Window => {
                move_focused_to_container_or_window(tn.node.id)
            }
            t => log::error!("Cannot move focused to {:?}", t),
        },
        Err(input) => {
            let ws_name = chop_workspace_shortcut(&input);
            move_focused_to_workspace_1(ws_name);
        }
    }
}

pub fn move_focused_to_workspace(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_move_focused_to(
        "Move focused container to workspace",
        &tree.get_workspaces(),
    );
}

pub fn move_focused_to(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    select_and_move_focused_to(
        "Move focused container to workspace or container",
        &tree.get_outputs_workspaces_containers_and_windows(),
    );
}

pub fn swap_focused_with(extra_props: &HashMap<i64, t::ExtraProps>) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root, extra_props);
    match util::select_from_menu(
        "Swap focused with",
        &tree.get_workspaces_containers_and_windows(),
    ) {
        Ok(tn) => match tn.node.get_type() {
            ipc::Type::Workspace | ipc::Type::Container | ipc::Type::Window => {
                run_sway_command(&[
                    "swap",
                    "container",
                    "with",
                    "con_id",
                    &format!("{}", tn.node.id),
                ])
            }
            t => log::error!("Cannot move focused to {:?}", t),
        },
        Err(input) => {
            let ws_name = chop_workspace_shortcut(&input);
            move_focused_to_workspace_1(ws_name);
        }
    }
}

pub enum Direction {
    Backward,
    Forward,
}

pub fn focus_window_in_direction(
    dir: Direction,
    consider_wins: &ConsiderWindows,
    extra_props: &HashMap<i64, t::ExtraProps>,
    pred: Box<dyn Fn(&t::DisplayNode) -> bool>,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root, extra_props);
    let mut wins = tree.get_windows();

    if consider_wins == &ConsiderWindows::CurrentWorkspace {
        let cur_ws = tree.get_current_workspace();
        wins.retain(|w| {
            tree.get_parent_node_of_type(w.node.id, ipc::Type::Workspace)
                .unwrap()
                .id
                == cur_ws.id
        });
    }

    wins.retain(pred);

    if wins.len() < 2 {
        return;
    }

    wins.sort_by(|a, b| {
        let lru_a = tree.last_focus_tick_for_next_prev_seq(a.node.id);
        let lru_b = tree.last_focus_tick_for_next_prev_seq(b.node.id);
        lru_a.cmp(&lru_b).reverse()
    });

    let is_focused_window: Box<dyn Fn(&t::DisplayNode) -> bool> =
        if !wins.iter().any(|w| w.node.focused) {
            let last_focused_win_id = wins.get(0).unwrap().node.id;
            Box::new(move |dn| dn.node.id == last_focused_win_id)
        } else {
            Box::new(|dn| dn.node.focused)
        };

    let mut iter: Box<dyn Iterator<Item = &t::DisplayNode>> = match dir {
        Direction::Forward => Box::new(wins.iter().rev().cycle()),
        Direction::Backward => Box::new(wins.iter().cycle()),
    };

    loop {
        let win = iter.next().unwrap();
        if is_focused_window(win) {
            let win = iter.next().unwrap();
            focus_window_by_id(win.node.id);
            return;
        }
    }
}

pub fn focus_window_of_same_layout_in_direction(
    dir: Direction,
    consider_wins: &ConsiderWindows,
    extra_props: &HashMap<i64, t::ExtraProps>,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root, extra_props);
    let wins = tree.get_windows();
    let cur_win = wins.iter().find(|w| w.node.focused);

    if let Some(cur_win) = cur_win {
        focus_window_in_direction(
            dir,
            consider_wins,
            extra_props,
            if cur_win.node.is_floating() {
                Box::new(|dn| dn.node.is_floating())
            } else if !cur_win.node.is_floating()
                && cur_win
                    .tree
                    .is_child_of_tabbed_or_stacked_container(cur_win.node.id)
            {
                Box::new(|dn| {
                    !dn.node.is_floating()
                        && dn
                            .tree
                            .is_child_of_tabbed_or_stacked_container(dn.node.id)
                })
            } else if !cur_win.node.is_floating()
                && cur_win.tree.is_child_of_tiled_container(cur_win.node.id)
            {
                Box::new(|dn| {
                    !dn.node.is_floating()
                        && dn.tree.is_child_of_tiled_container(dn.node.id)
                })
            } else {
                Box::new(always_true)
            },
        )
    }
}

fn tile_current_workspace(floating: &ConsiderFloating, shuffle: bool) {
    match layout::relayout_current_workspace(
        floating == &ConsiderFloating::IncludeFloating,
        Box::new(move |wins, con: &mut s::Connection| {
            con.run_command("focus parent")?;
            con.run_command("layout splith")?;

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
                        win.id
                    ))?;
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
                con.run_command(format!(
                    "[con_id={}] move to workspace current",
                    win.id
                ))?;
                placed_wins.push(win);
                if shuffle {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                    if let Some(win) = placed_wins.choose(&mut rng) {
                        con.run_command(format!("[con_id={}] focus", win.id))?;
                    }
                }
            }
            Ok(())
        }),
    ) {
        Ok(_) => (),
        Err(err) => log::error!("Error retiling workspace: {:?}", err),
    }
}

fn tab_current_workspace(floating: &ConsiderFloating) {
    match layout::relayout_current_workspace(
        floating == &ConsiderFloating::IncludeFloating,
        Box::new(move |wins, con: &mut s::Connection| {
            con.run_command("focus parent")?;
            con.run_command("layout tabbed")?;

            let mut placed_wins = vec![];
            wins.reverse();
            for win in wins {
                if win.is_floating() {
                    con.run_command(format!(
                        "[con_id={}] floating disable",
                        win.id
                    ))?;
                }

                std::thread::sleep(std::time::Duration::from_millis(25));
                con.run_command(format!(
                    "[con_id={}] move to workspace current",
                    win.id
                ))?;
                placed_wins.push(win);
            }
            Ok(())
        }),
    ) {
        Ok(_) => (),
        Err(err) => log::error!("Error retiling workspace: {:?}", err),
    }
}

fn toggle_tab_tile_current_workspace(floating: &ConsiderFloating) {
    let tree = ipc::get_root_node(false);
    let workspaces = tree.nodes_of_type(ipc::Type::Workspace);
    let cur_ws = workspaces.iter().find(|w| w.is_current()).unwrap();
    if cur_ws.layout == s::NodeLayout::Tabbed {
        tile_current_workspace(floating, true);
    } else {
        tab_current_workspace(floating);
    }
}

fn get_swaymsg_commands() -> Vec<SwaymsgCmd> {
    let mut cmds: Vec<String> = vec![];

    for b in &["none", "normal", "csd", "pixel"] {
        cmds.push(format!["border {}", b]);
    }

    cmds.push("exit".to_string());
    cmds.push("floating toggle".to_string());
    cmds.push("focus child".to_string());
    cmds.push("focus parent".to_string());
    cmds.push("focus tiling".to_string());
    cmds.push("focus floating".to_string());
    cmds.push("focus mode_toggle".to_string());
    cmds.push("fullscreen toggle".to_string());
    cmds.push("reload".to_string());
    cmds.push("sticky toggle".to_string());
    cmds.push("kill".to_string());
    cmds.push("tiling_drag toggle".to_string());

    for x in &["focus", "fullscreen", "open", "none", "visible"] {
        cmds.push(format!["inhibit_idle {}", x])
    }

    for l in &["default", "splith", "splitv", "stacking", "tiling"] {
        cmds.push(format!["layout {}", l])
    }

    for e in &["enable", "disable"] {
        cmds.push(format!["shortcuts_inhibitor {}", e])
    }

    for x in &["yes", "no", "always"] {
        cmds.push(format!["focus_follows_mouse {}", x])
    }

    for x in &["smart", "urgent", "focus", "none"] {
        cmds.push(format!["focus_on_window_activation {}", x])
    }

    for x in &["yes", "no", "force", "workspace"] {
        cmds.push(format!["focus_wrapping {}", x])
    }

    for x in &[
        "none",
        "vertical",
        "horizontal",
        "both",
        "smart",
        "smart_no_gaps",
    ] {
        cmds.push(format!["hide_edge_borders {}", x])
    }

    for x in &["on", "no_gaps", "off"] {
        cmds.push(format!["smart_borders {}", x])
    }

    for x in &["on", "off"] {
        cmds.push(format!["smart_gaps {}", x])
    }

    for x in &["output", "container", "none"] {
        cmds.push(format!["mouse_warping {}", x])
    }

    for x in &["smart", "ignore", "leave_fullscreen"] {
        cmds.push(format!["popup_during_fullscreen {}", x])
    }

    for x in &["yes", "no"] {
        cmds.push(format!["show_marks {}", x]);
        cmds.push(format!["workspace_auto_back_and_forth {}", x]);
    }

    for x in &["left", "center", "right"] {
        cmds.push(format!["title_align {}", x]);
    }

    for x in &["enable", "disable", "allow", "deny"] {
        cmds.push(format!["urgent {}", x])
    }

    cmds.sort();

    cmds.into_iter().map(|c| SwaymsgCmd { cmd: c }).collect()
}

struct SwaymsgCmd {
    cmd: String,
}

impl DisplayFormat for SwaymsgCmd {
    fn format_for_display(&self, _: &cfg::Config) -> std::string::String {
        self.cmd.clone()
    }

    fn get_indent_level(&self) -> usize {
        0
    }
}

pub fn exec_swaymsg_command() {
    let cmds = get_swaymsg_commands();
    let cmd = util::select_from_menu("Execute swaymsg command", &cmds);
    match cmd {
        Ok(cmd) => run_sway_command_1(&cmd.cmd),
        Err(cmd) if !cmd.is_empty() => {
            let cmd = chop_sway_shortcut(&cmd);
            run_sway_command_1(cmd);
        }
        Err(_) => (),
    }
}

pub fn configure_outputs() {
    let outputs = get_outputs();

    let mut cmds = vec![];
    for o in outputs {
        cmds.push(format!("output {} toggle", o.name));

        for mode in o.modes {
            cmds.push(format!(
                "output {} mode {}x{}",
                o.name, mode.width, mode.height
            ));
        }

        for on_off in ["on", "off"] {
            cmds.push(format!("output {} dpms {}", o.name, on_off));
            cmds.push(format!("output {} adaptive_sync {}", o.name, on_off));
        }

        for transform in [
            "normal",
            "90",
            "180",
            "270",
            "360",
            "flipped",
            "flipped-90",
            "flipped-180",
            "flipped-270",
        ] {
            for dir in ["clockwise", "anticlockwise"] {
                cmds.push(format!(
                    "output {} transform {} {}",
                    o.name, transform, dir
                ));
            }
        }

        for pix in ["rgb", "bgr", "vrbg", "vbgr", "none"] {
            cmds.push(format!("output {} subpixel {}", o.name, pix));
        }

        for pix in ["linear", "nearest", "smart"] {
            cmds.push(format!("output {} scale_filter {}", o.name, pix));
        }
    }
    cmds.sort();
    let cmds: Vec<SwaymsgCmd> =
        cmds.into_iter().map(|c| SwaymsgCmd { cmd: c }).collect();

    while let Ok(cmd) = util::select_from_menu("Output command", &cmds) {
        run_sway_command_1(&cmd.cmd);
    }
}
