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
use crate::criteria;
use crate::focus::FocusData;
use crate::focus::FocusMessage;
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
use std::sync::Mutex;
use std::sync::MutexGuard;
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

#[derive(clap::Parser, PartialEq, Debug, Clone, Deserialize, Serialize)]
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
    /// Switch to the (first) window matching the given criteria (see section
    /// `CRITERIA` in `sway(5)`) if it exists and is not already focused.
    /// Otherwise, switch to the next urgent window (if any) or to the last
    /// recently used window.
    SwitchToMatchingOrUrgentOrLRUWindow { criteria: String },
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
    pub focus_data: &'a FocusData,
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

static LAST_COMMAND: Lazy<Mutex<SwayrCommand>> =
    Lazy::new(|| Mutex::new(SwayrCommand::Nop));

#[derive(Debug)]
pub struct SwitchToMatchingData {
    visited: Vec<i64>,
    lru: Option<i64>,
    fallback: Option<i64>,
}

static SWITCH_TO_MATCHING_DATA: Lazy<Mutex<SwitchToMatchingData>> =
    Lazy::new(|| {
        Mutex::new(SwitchToMatchingData {
            visited: vec![],
            lru: None,
            fallback: None,
        })
    });

pub fn exec_swayr_cmd(args: ExecSwayrCmdArgs) {
    log::info!("Running SwayrCommand {:?}", args.cmd);
    let fdata = args.focus_data;

    let mut last_command = LAST_COMMAND.lock().expect("Could not lock mutex");
    let mut switch_to_matching_data = SWITCH_TO_MATCHING_DATA
        .lock()
        .expect("Could not lock mutex");

    // If this command is not equal to the last command, nuke the
    // switch_to_matching_data so that we start a new sequence.
    if *args.cmd != *last_command {
        switch_to_matching_data.visited.clear();
        switch_to_matching_data.lru = None;
        switch_to_matching_data.fallback = None;
    }

    if args.cmd.is_prev_next_window_variant() {
        fdata.send(FocusMessage::TickUpdateInhibit);
    } else {
        fdata.send(FocusMessage::TickUpdateActivate);
    }

    match args.cmd {
        SwayrCommand::Nop => {}
        SwayrCommand::SwitchToUrgentOrLRUWindow => {
            switch_to_urgent_or_lru_window(fdata)
        }
        SwayrCommand::SwitchToAppOrUrgentOrLRUWindow { name } => {
            switch_to_app_or_urgent_or_lru_window(
                name,
                &mut switch_to_matching_data,
                fdata,
            )
        }
        SwayrCommand::SwitchToMarkOrUrgentOrLRUWindow { con_mark } => {
            switch_to_mark_or_urgent_or_lru_window(
                con_mark,
                &mut switch_to_matching_data,
                fdata,
            )
        }
        SwayrCommand::SwitchToMatchingOrUrgentOrLRUWindow { criteria } => {
            switch_to_matching_or_urgent_or_lru_window(
                criteria,
                &mut switch_to_matching_data,
                fdata,
            )
        }
        SwayrCommand::SwitchWindow => switch_window(fdata),
        SwayrCommand::SwitchWorkspace => switch_workspace(fdata),
        SwayrCommand::SwitchOutput => switch_output(),
        SwayrCommand::SwitchWorkspaceOrWindow => {
            switch_workspace_or_window(fdata)
        }
        SwayrCommand::SwitchWorkspaceContainerOrWindow => {
            switch_workspace_container_or_window(fdata)
        }
        SwayrCommand::SwitchTo => switch_to(fdata),
        SwayrCommand::QuitWindow { kill } => quit_window(fdata, *kill),
        SwayrCommand::QuitWorkspaceOrWindow => quit_workspace_or_window(fdata),
        SwayrCommand::QuitWorkspaceContainerOrWindow => {
            quit_workspace_container_or_window(fdata)
        }
        SwayrCommand::MoveFocusedToWorkspace => {
            move_focused_to_workspace(fdata)
        }
        SwayrCommand::MoveFocusedTo => move_focused_to(fdata),
        SwayrCommand::SwapFocusedWith => swap_focused_with(fdata),
        SwayrCommand::NextWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            fdata,
            Box::new(always_true),
        ),
        SwayrCommand::PrevWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            fdata,
            Box::new(always_true),
        ),
        SwayrCommand::NextTiledWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            fdata,
            Box::new(|dn: &t::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            }),
        ),
        SwayrCommand::PrevTiledWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            fdata,
            Box::new(|dn: &t::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            }),
        ),
        SwayrCommand::NextTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                fdata,
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
                fdata,
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
                fdata,
                Box::new(|dn: &t::DisplayNode| dn.node.is_floating()),
            )
        }
        SwayrCommand::PrevFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                fdata,
                Box::new(|dn: &t::DisplayNode| dn.node.is_floating()),
            )
        }
        SwayrCommand::NextWindowOfSameLayout { windows } => {
            focus_window_of_same_layout_in_direction(
                Direction::Forward,
                windows,
                fdata,
            )
        }
        SwayrCommand::PrevWindowOfSameLayout { windows } => {
            focus_window_of_same_layout_in_direction(
                Direction::Backward,
                windows,
                fdata,
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
                    focus_data: args.focus_data,
                });
            }
        }
    }

    *last_command = args.cmd.clone();
}

fn focus_window_by_id(id: i64) -> i64 {
    run_sway_command(&[format!("[con_id={}]", id).as_str(), "focus"]);
    id
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

pub fn switch_to_urgent_or_lru_window(fdata: &FocusData) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let mutex = Mutex::new(SwitchToMatchingData {
        visited: vec![],
        lru: None,
        fallback: None,
    });
    let mut guard = mutex.lock().expect("Could not lock mutex");
    focus_matching_or_lru_window(&wins, fdata, &mut guard, |_| false);
}

pub fn focus_matching_or_lru_window<P>(
    wins: &[t::DisplayNode],
    fdata: &FocusData,
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
    pred: P,
) where
    P: Fn(&t::DisplayNode) -> bool,
{
    // Initialize the fallback on first invocation.
    if switch_to_matching_data.visited.is_empty() {
        // The currently focused window is already visited, obviously.
        let focused = wins.iter().find(|w| w.node.focused);
        if let Some(f) = focused {
            switch_to_matching_data.visited.push(f.node.id);
            // The focused window is the fallback we want to return to.
            switch_to_matching_data.fallback = Some(f.node.id);
        }

        switch_to_matching_data.lru = wins
            .iter()
            .filter(|w| !w.node.focused)
            .max_by(|a, b| {
                fdata
                    .last_focus_tick(a.node.id)
                    .cmp(&fdata.last_focus_tick(b.node.id))
            })
            .map(|w| w.node.id);

        log::debug!(
            "Initialized SwitchToMatchingData: {:?}",
            switch_to_matching_data
        );
    }

    let visited = &switch_to_matching_data.visited;
    if let Some(win) = wins.iter().find(|w| {
        switch_to_matching_data.lru != Some(w.node.id)
            && switch_to_matching_data.fallback != Some(w.node.id)
            && !visited.contains(&w.node.id)
            && (w.node.urgent || pred(w))
    }) {
        log::debug!("Switching to by urgency or matching predicate");
        focus_window_by_id(win.node.id);
        switch_to_matching_data.visited.push(win.node.id);
    } else if switch_to_matching_data.lru.is_some()
        && !visited.contains(&switch_to_matching_data.lru.unwrap())
    {
        log::debug!("Switching to LRU");
        let id = switch_to_matching_data.lru.unwrap();
        switch_to_matching_data.visited.push(id);
        focus_window_by_id(id);
    } else {
        log::debug!("Switching to Fallback");
        if let Some(id) = switch_to_matching_data.fallback {
            focus_window_by_id(id);
        } else {
            log::debug!("No fallback window.")
        }
        switch_to_matching_data.visited = vec![];
        switch_to_matching_data.lru = None;
        switch_to_matching_data.fallback = None;
    }
}

pub fn switch_to_app_or_urgent_or_lru_window(
    name: &str,
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let pred = |w: &t::DisplayNode| w.node.get_app_name() == name;

    focus_matching_or_lru_window(&wins, fdata, switch_to_matching_data, pred);
}

pub fn switch_to_mark_or_urgent_or_lru_window(
    con_mark: &str,
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let con_mark = &con_mark.to_owned();
    let pred = |w: &t::DisplayNode| w.node.marks.contains(con_mark);

    focus_matching_or_lru_window(&wins, fdata, switch_to_matching_data, pred);
}

fn switch_to_matching_or_urgent_or_lru_window(
    criteria: &str,
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);

    if let Some(crit) = criteria::parse_criteria(criteria) {
        let pred = criteria::criteria_to_predicate(crit, &wins);
        focus_matching_or_lru_window(
            &wins,
            fdata,
            switch_to_matching_data,
            pred,
        )
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
                focus_window_by_id(tn.node.id);
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

pub fn switch_window(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus("Select window", &tree.get_windows(fdata));
}

pub fn switch_workspace(fdata: &FocusData) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    select_and_focus("Select workspace", &tree.get_workspaces(fdata));
}

pub fn switch_output() {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    select_and_focus("Select output", &tree.get_outputs());
}

pub fn switch_workspace_or_window(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus(
        "Select workspace or window",
        &tree.get_workspaces_and_windows(fdata),
    );
}

pub fn switch_workspace_container_or_window(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus(
        "Select workspace, container or window",
        &tree.get_workspaces_containers_and_windows(fdata),
    );
}

pub fn switch_to(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus(
        "Select output, workspace, container or window",
        &tree.get_outputs_workspaces_containers_and_windows(fdata),
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

pub fn quit_window(fdata: &FocusData, kill: bool) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_quit("Quit window", &tree.get_windows(fdata), kill);
}

pub fn quit_workspace_or_window(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_quit(
        "Quit workspace or window",
        &tree.get_workspaces_and_windows(fdata),
        false,
    );
}

pub fn quit_workspace_container_or_window(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_quit(
        "Quit workspace, container or window",
        &tree.get_workspaces_containers_and_windows(fdata),
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

pub fn move_focused_to_workspace(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_move_focused_to(
        "Move focused container to workspace",
        &tree.get_workspaces(fdata),
    );
}

pub fn move_focused_to(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_move_focused_to(
        "Move focused container to workspace or container",
        &tree.get_outputs_workspaces_containers_and_windows(fdata),
    );
}

pub fn swap_focused_with(fdata: &FocusData) {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    match util::select_from_menu(
        "Swap focused with",
        &tree.get_workspaces_containers_and_windows(fdata),
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
    fdata: &FocusData,
    pred: Box<dyn Fn(&t::DisplayNode) -> bool>,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let mut wins = tree.get_windows(fdata);

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
        let lru_a = fdata.last_focus_tick(a.node.id);
        let lru_b = fdata.last_focus_tick(b.node.id);
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
    fdata: &FocusData,
) {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let cur_win = wins.iter().find(|w| w.node.focused);

    if let Some(cur_win) = cur_win {
        focus_window_in_direction(
            dir,
            consider_wins,
            fdata,
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
