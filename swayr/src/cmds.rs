// Copyright (C) 2021-2023  Tassilo Horn <tsdh@gnu.org>
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
use crate::daemon::CONFIG;
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
use std::io::Read;
use std::sync::mpsc::channel;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use swayipc as s;

pub fn run_sway_command_1(cmd: &str) -> Result<String, String> {
    log::debug!("Running sway command: {cmd}");
    match s::Connection::new() {
        Ok(mut con) => match con.run_command(cmd) {
            Err(err) => {
                log::error!("Could not run sway command: {err}");
                Err(err.to_string())
            }
            _ => Ok(format!("Executed sway command '{cmd}'")),
        },
        Err(err) => {
            log::error!("Couldn't create sway ipc connection: {err}");
            Err(err.to_string())
        }
    }
}

pub fn run_sway_command(args: &[&str]) -> Result<String, String> {
    let cmd = args.join(" ");
    run_sway_command_1(&cmd)
}

#[derive(clap::Parser, Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ConsiderFloating {
    /// Include floating windows.
    IncludeFloating,
    /// Exclude floating windows.
    ExcludeFloating,
}

#[derive(clap::Parser, Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub enum ConsiderWindows {
    /// Consider windows of all workspaces.
    AllWorkspaces,
    /// Consider windows of only the current workspaces.
    CurrentWorkspace,
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub struct SkipFlags {
    #[clap(short = 'u', long, help = "Skip urgent windows")]
    skip_urgent: bool,
    #[clap(
        short = 'l',
        long,
        conflicts_with("skip_lru_if_current_doesnt_match"),
        help = "Skip the last recently used window"
    )]
    skip_lru: bool,
    #[clap(
        short = 'L',
        long,
        conflicts_with("skip_lru"),
        help = "Skip the last recently used window iff the current doesn't match"
    )]
    skip_lru_if_current_doesnt_match: bool,
    #[clap(short = 'o', long, help = "Don't switch back to the origin window")]
    skip_origin: bool,
}

#[derive(clap::Parser, PartialEq, Eq, Debug, Clone, Deserialize, Serialize)]
pub enum SwayrCommand {
    /// No-operation. Interrupts any in-progress prev/next sequence but has
    /// no other effect
    Nop,
    /// Switch to next urgent window (if any) or to last recently used window.
    SwitchToUrgentOrLRUWindow {
        #[clap(flatten)]
        skip_flags: SkipFlags,
    },
    /// Switch to the given app (given by app_id or window class) if that's not
    /// focused already.  If it is, switch to the next urgent window (if any)
    /// or to last recently used window.
    ///
    /// For example, you can provide "firefox" as argument to this command to
    /// have a convenient firefox <-> last-recently-used window toggle.
    SwitchToAppOrUrgentOrLRUWindow {
        /// The app_id or window class of the windows to switch to.  Compared
        /// literally, i.e., not a regex.
        name: String,

        #[clap(flatten)]
        skip_flags: SkipFlags,
    },
    /// Switch to the window with the given mark if that's not focused already.
    /// If it is, switch to the next urgent window (if any) or to last recently
    /// used window.
    ///
    /// For example, you can assign a "browser" mark to your browser window
    /// (using a standard sway `for_window` rule).  Then you can provide
    /// "browser" as argument to this command to have a convenient browser <->
    /// last-recently-used window toggle.
    SwitchToMarkOrUrgentOrLRUWindow {
        /// The con_mark to switch to.
        con_mark: String,

        #[clap(flatten)]
        skip_flags: SkipFlags,
    },
    /// Switch to the (first) window matching the given criteria (see section
    /// `CRITERIA` in `sway(5)`) if it exists and is not already focused.
    /// Otherwise, switch to the next urgent window (if any) or to the last
    /// recently used window.
    SwitchToMatchingOrUrgentOrLRUWindow {
        /// The criteria query defining which windows to switch to.
        criteria: String,

        #[clap(flatten)]
        skip_flags: SkipFlags,
    },
    /// Focus the selected window.
    SwitchWindow,
    /// Steal the selected window from another workspace into the current
    /// workspace.
    StealWindow,
    /// Steal the selected window or container from another workspace into the
    /// current workspace.
    StealWindowOrContainer,
    /// Switch to the selected workspace.
    SwitchWorkspace,
    /// Switch to the selected output.
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
    /// Focus the next window matching the given criteria query.
    NextMatchingWindow {
        /// The criteria query defining which windows to switch to.
        criteria: String,
    },
    /// Focus the previous window matching the given criteria query.
    PrevMatchingWindow {
        /// The criteria query defining which windows to switch to.
        criteria: String,
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
    /// Returns a JSON array of all sway nodes being actual application windows
    /// with some extra properties not present in sway IPC (`swayr_icon`,
    /// `swayr_type`).
    GetWindowsAsJson {
        #[clap(
            short,
            long,
            help = "Determines if windows on the scratchpad are to be included."
        )]
        include_scratchpad: bool,
        #[clap(
            short = 'm',
            long = "matching",
            help = "A criteria query defining which windows to return."
        )]
        criteria: Option<String>,
        #[clap(
            short,
            long,
            help = "Return non-zero if no (matching) windows are found instead of returning an empty JSON array."
        )]
        error_if_no_match: bool,
    },
    /// Executes a shell command for each matching window.
    ForEachWindow {
        #[clap(
            short,
            long,
            help = "Determines if windows on the scratchpad are to be included."
        )]
        include_scratchpad: bool,
        #[clap(
            short,
            long,
            help = "Return non-zero if no (matching) windows are found instead of just doing nothing."
        )]
        error_if_no_match: bool,
        criteria: String,
        shell_command: Vec<String>,
    },
    /// Print the current effective swayr configuration (without default
    /// values).
    PrintConfig,
    /// Prints the default swayr configuration.
    PrintDefaultConfig,
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
                | SwayrCommand::NextMatchingWindow { .. }
                | SwayrCommand::PrevMatchingWindow { .. }
        )
    }

    pub(crate) fn is_scripting_command(&self) -> bool {
        matches!(
            self,
            SwayrCommand::GetWindowsAsJson { .. }
                | SwayrCommand::ForEachWindow { .. }
        )
    }
}

pub struct ExecSwayrCmdArgs<'a> {
    pub cmd: &'a SwayrCommand,
    pub focus_data: &'a FocusData,
}

impl DisplayFormat for SwayrCommand {
    fn format_for_display(&self) -> std::string::String {
        // TODO: It would be very nice if the display format was exactly like
        // the swayr invocation in the shell.  Can that somehow be retrieved
        // from clap?
        format!("{self:?}")
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
    origin: Option<i64>,
    skip_urgent: bool,
    skip_lru: bool,
    skip_lru_if_current_doesnt_match: bool,
    skip_origin: bool,
}

impl SwitchToMatchingData {
    pub fn reset(&mut self, reset_skip_flags: bool) {
        self.visited.clear();
        self.lru = None;
        self.origin = None;
        if reset_skip_flags {
            self.skip_urgent = false;
            self.skip_lru = false;
            self.skip_lru_if_current_doesnt_match = false;
            self.skip_origin = false;
        }
    }

    fn new() -> SwitchToMatchingData {
        SwitchToMatchingData {
            visited: vec![],
            lru: None,
            origin: None,
            skip_urgent: false,
            skip_lru: false,
            skip_lru_if_current_doesnt_match: false,
            skip_origin: false,
        }
    }
}

static SWITCH_TO_MATCHING_DATA: Lazy<Mutex<SwitchToMatchingData>> =
    Lazy::new(|| Mutex::new(SwitchToMatchingData::new()));

pub fn exec_swayr_cmd(args: ExecSwayrCmdArgs) -> Result<String, String> {
    log::info!("Running SwayrCommand {:?}", args.cmd);

    let mut last_command = LAST_COMMAND.lock().expect("Could not lock mutex");
    let mut switch_to_matching_data = SWITCH_TO_MATCHING_DATA
        .lock()
        .expect("Could not lock mutex");

    // Scripting commands are commands not intended for interactive use like
    // get-windows-as-json.  They should not mess with switch_to_matching_data
    // or focus ticks.
    if !args.cmd.is_scripting_command() {
        // If this command is not equal to the last command, nuke the
        // switch_to_matching_data so that we start a new sequence.
        if *args.cmd != *last_command {
            switch_to_matching_data.reset(true);
        }
        *last_command = args.cmd.clone();

        let fdata = args.focus_data;
        if args.cmd.is_prev_next_window_variant() {
            fdata.send(FocusMessage::TickUpdateInhibit);
        } else {
            fdata.send(FocusMessage::TickUpdateActivate);
        }
    }

    exec_swayr_cmd_1(args, &mut switch_to_matching_data)
}

fn exec_swayr_cmd_1(
    args: ExecSwayrCmdArgs,
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
) -> Result<String, String> {
    let fdata = args.focus_data;

    match args.cmd {
        SwayrCommand::Nop => Ok("done".to_owned()),
        SwayrCommand::SwitchToUrgentOrLRUWindow { skip_flags } => {
            init_switch_to_matching_data(switch_to_matching_data, skip_flags);
            switch_to_urgent_or_lru_window(switch_to_matching_data, fdata)
        }
        SwayrCommand::SwitchToAppOrUrgentOrLRUWindow { name, skip_flags } => {
            init_switch_to_matching_data(switch_to_matching_data, skip_flags);
            switch_to_app_or_urgent_or_lru_window(
                name,
                switch_to_matching_data,
                fdata,
            )
        }
        SwayrCommand::SwitchToMarkOrUrgentOrLRUWindow {
            con_mark,
            skip_flags,
        } => {
            init_switch_to_matching_data(switch_to_matching_data, skip_flags);
            switch_to_mark_or_urgent_or_lru_window(
                con_mark,
                switch_to_matching_data,
                fdata,
            )
        }
        SwayrCommand::SwitchToMatchingOrUrgentOrLRUWindow {
            criteria,
            skip_flags,
        } => {
            init_switch_to_matching_data(switch_to_matching_data, skip_flags);
            switch_to_matching_or_urgent_or_lru_window(
                criteria,
                switch_to_matching_data,
                fdata,
            )
        }
        SwayrCommand::SwitchWindow => switch_window(fdata),
        SwayrCommand::StealWindow => steal_window(fdata),
        SwayrCommand::StealWindowOrContainer => {
            steal_window_or_container(fdata)
        }
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
            always_true,
        ),
        SwayrCommand::PrevWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            fdata,
            always_true,
        ),
        SwayrCommand::NextTiledWindow { windows } => focus_window_in_direction(
            Direction::Forward,
            windows,
            fdata,
            |dn: &t::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            },
        ),
        SwayrCommand::PrevTiledWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            fdata,
            |dn: &t::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            },
        ),
        SwayrCommand::NextTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                fdata,
                |dn: &t::DisplayNode| {
                    !dn.node.is_floating()
                        && dn
                            .tree
                            .is_child_of_tabbed_or_stacked_container(dn.node.id)
                },
            )
        }
        SwayrCommand::PrevTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                fdata,
                |dn: &t::DisplayNode| {
                    !dn.node.is_floating()
                        && dn
                            .tree
                            .is_child_of_tabbed_or_stacked_container(dn.node.id)
                },
            )
        }
        SwayrCommand::NextFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                fdata,
                |dn: &t::DisplayNode| dn.node.is_floating(),
            )
        }
        SwayrCommand::PrevFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                fdata,
                |dn: &t::DisplayNode| dn.node.is_floating(),
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
        SwayrCommand::NextMatchingWindow { criteria } => {
            focus_matching_window_in_direction(
                Direction::Forward,
                criteria,
                fdata,
            )
        }
        SwayrCommand::PrevMatchingWindow { criteria } => {
            focus_matching_window_in_direction(
                Direction::Backward,
                criteria,
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
        SwayrCommand::GetWindowsAsJson {
            include_scratchpad,
            criteria,
            error_if_no_match,
        } => get_windows_as_json(
            fdata,
            *include_scratchpad,
            criteria,
            *error_if_no_match,
        ),
        SwayrCommand::ForEachWindow {
            include_scratchpad,
            error_if_no_match,
            criteria,
            shell_command,
        } => for_each_window(
            fdata,
            *include_scratchpad,
            *error_if_no_match,
            criteria,
            shell_command,
        ),
        SwayrCommand::ExecuteSwaymsgCommand => exec_swaymsg_command(),
        SwayrCommand::ExecuteSwayrCommand => {
            let mut cmds = vec![
                SwayrCommand::MoveFocusedToWorkspace,
                SwayrCommand::MoveFocusedTo,
                SwayrCommand::SwapFocusedWith,
                SwayrCommand::QuitWorkspaceOrWindow,
                SwayrCommand::SwitchWindow,
                SwayrCommand::StealWindow,
                SwayrCommand::StealWindowOrContainer,
                SwayrCommand::SwitchWorkspace,
                SwayrCommand::SwitchOutput,
                SwayrCommand::SwitchWorkspaceOrWindow,
                SwayrCommand::SwitchToUrgentOrLRUWindow {
                    skip_flags: SkipFlags {
                        skip_urgent: false,
                        skip_lru: false,
                        skip_lru_if_current_doesnt_match: false,
                        skip_origin: false,
                    },
                },
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

            match util::select_from_menu("Select swayr command", &cmds) {
                Ok(c) => exec_swayr_cmd_1(
                    ExecSwayrCmdArgs {
                        cmd: c,
                        focus_data: args.focus_data,
                    },
                    switch_to_matching_data,
                ),
                _ => Err("No swayr command selected".to_owned()),
            }
        }
        SwayrCommand::PrintConfig => print_config(false),
        SwayrCommand::PrintDefaultConfig => print_config(true),
    }
}

fn print_config(default_config: bool) -> Result<String, String> {
    let dc = cfg::Config::default();
    let cfg = if default_config {
        Some(&dc)
    } else {
        once_cell::sync::Lazy::get(&CONFIG)
    };

    if let Some(cfg) = cfg {
        match toml::to_string_pretty(cfg) {
            Ok(json) => Ok(json),
            Err(err) => Err(err.to_string()),
        }
    } else {
        Err("Config not yet initialized.".to_owned())
    }
}

fn init_switch_to_matching_data(
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
    skip_flags: &SkipFlags,
) {
    switch_to_matching_data.skip_urgent = skip_flags.skip_urgent;
    switch_to_matching_data.skip_lru = skip_flags.skip_lru;
    switch_to_matching_data.skip_lru_if_current_doesnt_match =
        skip_flags.skip_lru_if_current_doesnt_match;
    switch_to_matching_data.skip_origin = skip_flags.skip_origin;
}

fn get_matching_windows<'a>(
    criteria: Option<&String>,
    wins: &'a [t::DisplayNode<'a>],
) -> Result<Vec<&'a t::DisplayNode<'a>>, String> {
    if let Some(criteria) = criteria {
        let c = criteria::parse_criteria(criteria)?;
        let pred = criteria::criterion_to_predicate(&c, wins);
        Ok(wins.iter().filter(|w| pred(w)).collect())
    } else {
        Ok(wins.iter().collect())
    }
}

fn get_windows_as_json(
    fdata: &FocusData,
    include_scratchpad: bool,
    criteria: &Option<String>,
    error_if_no_match: bool,
) -> Result<String, String> {
    let root = ipc::get_root_node(include_scratchpad);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let wins = get_matching_windows(criteria.as_ref(), &wins)?;
    if error_if_no_match && wins.is_empty() {
        Err(String::from(if criteria.is_some() {
            "No matching windows"
        } else {
            "No windows"
        }))
    } else {
        serde_json::to_string_pretty(&wins)
            .map_or_else(|e| Err(e.to_string()), Ok)
    }
}

#[derive(Serialize, Deserialize)]
struct ShellCommandResult {
    exit_code: i32,
    std_out: String,
    std_err: String,
    error: Option<String>,
}

fn read_from_child(
    child: &mut std::process::Child,
    out: &mut String,
    err: &mut String,
) {
    child.stdout.take().map(|mut co| co.read_to_string(out));
    child.stderr.take().map(|mut ce| ce.read_to_string(err));
}

fn run_shell_command_on_window(
    win: &t::DisplayNode,
    shell_command: &[String],
) -> ShellCommandResult {
    let cmd: Vec<String> = shell_command
        .iter()
        .map(|arg| win.subst_node_placeholders(arg, false))
        .collect();
    log::debug!("Running shell command on {}", win.node.id);
    match std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            // Drop stdin, we don't use it anyway.
            if let Some(i) = child.stdin.take() {
                drop(i)
            }

            let mut out = String::new();
            let mut err = String::new();
            let mut sleep_time: u16 = 4;
            let mut slept_time: u16 = 0;
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        read_from_child(&mut child, &mut out, &mut err);
                        return ShellCommandResult {
                            exit_code: status.code().unwrap(),
                            std_out: out,
                            std_err: err,
                            error: None,
                        };
                    }
                    Ok(None) => {
                        if slept_time >= 2000 {
                            let k = child.kill();
                            read_from_child(&mut child, &mut out, &mut err);
                            return ShellCommandResult {
                                exit_code: 997,
                                std_out: out,
                                std_err: err,
                                error: Some(format!(
                                    "Didn't finish, I killed it.{}",
                                    match k {
                                        Ok(_) => String::new(),
                                        Err(err) => format!(" And even killing failed with: {err}"),
                                    }
                                )),
                            };
                        } else {
                            std::thread::sleep(
                                std::time::Duration::from_millis(
                                    sleep_time as u64,
                                ),
                            );
                            slept_time += sleep_time;
                            if sleep_time < 100 {
                                sleep_time *= 2;
                            }
                        }
                    }
                    Err(err) => {
                        return ShellCommandResult {
                            exit_code: err.raw_os_error().unwrap_or(998),
                            std_out: String::new(),
                            std_err: String::new(),
                            error: Some(err.to_string()),
                        }
                    }
                }
            }
        }
        Err(err) => ShellCommandResult {
            exit_code: err.raw_os_error().unwrap_or(999),
            std_out: String::new(),
            std_err: String::new(),
            error: Some(err.to_string()),
        },
    }
}

fn for_each_window(
    fdata: &FocusData,
    include_scratchpad: bool,
    error_if_no_match: bool,
    criteria: &String,
    shell_command: &Vec<String>,
) -> Result<String, String> {
    if shell_command.is_empty() {
        return Err("No shell_command given".to_owned());
    }
    let root = ipc::get_root_node(include_scratchpad);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let wins = get_matching_windows(Some(criteria), &wins)?;

    if error_if_no_match && wins.is_empty() {
        return Err(String::from("No matching windows"));
    }

    let (sender, receiver) = channel::<ShellCommandResult>();

    thread::scope(|scope| {
        for w in wins {
            let s = sender.clone();
            scope.spawn(move || {
                s.send(run_shell_command_on_window(w, shell_command))
                    .expect("Error on send!");
            });
        }
    });

    // Drop the last sender explicity, otherwise receiver.iter().collect()
    // blocks indefinitely.
    drop(sender);

    let results: Vec<ShellCommandResult> = receiver.iter().collect();
    let json =
        serde_json::to_string_pretty(&results).expect("Error generating JSON");
    if results.iter().all(|r| r.exit_code == 0) {
        Ok(json)
    } else {
        Err(json)
    }
}

fn steal_window_by_id(id: i64) -> Result<String, String> {
    run_sway_command(&[
        format!("[con_id={id}]").as_str(),
        "move to workspace current",
    ])
}

fn focus_window_by_id(id: i64) -> Result<String, String> {
    run_sway_command(&[format!("[con_id={id}]").as_str(), "focus"])
}

fn quit_window_by_id(id: i64) -> Result<String, String> {
    run_sway_command(&[format!("[con_id={id}]").as_str(), "kill"])
}

pub fn get_outputs() -> Vec<s::Output> {
    match s::Connection::new() {
        Ok(mut con) => con.get_outputs().expect("Got no outputs"),
        Err(err) => panic!("{}", err),
    }
}

pub fn switch_to_urgent_or_lru_window(
    stm_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    focus_urgent_or_matching_or_lru_window(
        &wins,
        fdata,
        stm_data,
        |_| false,
        true,
    )
}

pub fn focus_urgent_or_matching_or_lru_window<P>(
    wins: &[t::DisplayNode],
    fdata: &FocusData,
    stm_data: &mut MutexGuard<SwitchToMatchingData>,
    pred: P,
    ignore_pred: bool,
) -> Result<String, String>
where
    P: Fn(&t::DisplayNode) -> bool,
{
    let focused = wins.iter().find(|w| w.node.focused);
    let focused_id = focused.map(|f| f.node.id).unwrap_or(-1);

    // Initialize the fallback on first invocation.
    let initialized_now = if stm_data.visited.is_empty() {
        // If we should not ignore the predicate is given, then we want at
        // least one matching window.
        if !ignore_pred && !wins.iter().any(&pred) {
            return Err("No window matches.".to_owned());
        }

        // The currently focused window is already visited, obviously.
        if let Some(f) = focused {
            // The focused window is the fallback we want to return to.
            stm_data.origin = Some(f.node.id);
        }

        if !ignore_pred
            && stm_data.skip_lru_if_current_doesnt_match
            && (focused.is_none()
                || focused.is_some() && !pred(focused.unwrap()))
        {
            stm_data.skip_lru = true;
        }

        if !stm_data.skip_lru {
            stm_data.lru = wins
                .iter()
                .filter(|w| !w.node.focused)
                .max_by(|a, b| {
                    fdata
                        .last_focus_tick(a.node.id)
                        .cmp(&fdata.last_focus_tick(b.node.id))
                })
                .map(|w| w.node.id);
        }

        log::debug!("Initialized SwitchToMatchingData: {stm_data:?}");
        true
    } else {
        false
    };

    // We might have changed focus through other means (normal sway commands)
    // so just add the current window to visited unconditionally.
    stm_data.visited.push(focused_id);

    if let Some(win) = wins.iter().find(|w| {
        w.node.id != focused_id
            && !stm_data.skip_urgent
            && w.node.urgent
            && !stm_data.visited.contains(&w.node.id)
    }) {
        log::debug!("Switching to by urgency");
        stm_data.visited.push(win.node.id);
        focus_window_by_id(win.node.id)
            .map(|msg| msg + " (It's a window with urgency hint.)")
    } else if let Some(win) = wins.iter().find(|w| {
        w.node.id != focused_id
            && (stm_data.skip_origin || stm_data.origin != Some(w.node.id))
            && !stm_data.visited.contains(&w.node.id)
            && pred(w)
    }) {
        log::debug!("Switching to by matching predicate");
        stm_data.visited.push(win.node.id);
        focus_window_by_id(win.node.id)
            .map(|msg| msg + " (It's a matching window.)")
    } else if !stm_data.skip_lru
        && stm_data.lru.is_some()
        && stm_data.lru != Some(focused_id)
        && !stm_data.visited.contains(&stm_data.lru.unwrap())
        && wins.iter().any(|w| w.node.id == stm_data.lru.unwrap())
    {
        log::debug!("Switching to LRU");
        let id = stm_data.lru.unwrap();
        stm_data.visited.push(id);
        focus_window_by_id(id).map(|msg| msg + " (It's the LRU window.)")
    } else if !stm_data.skip_origin {
        log::debug!("Switching back to origin");
        if let Some(id) = stm_data.origin {
            if id == focused_id {
                log::debug!("Origin is already focused; resetting.");
                stm_data.reset(false);
                if initialized_now {
                    Ok("Origin is already focused.".to_owned())
                } else {
                    focus_urgent_or_matching_or_lru_window(
                        wins,
                        fdata,
                        stm_data,
                        pred,
                        ignore_pred,
                    )
                }
            } else if id != focused_id && wins.iter().any(|w| w.node.id == id) {
                stm_data.reset(false);
                focus_window_by_id(id)
                    .map(|msg| msg + " (It's the origin window.)")
            } else {
                log::debug!("Origin is gone; resetting.");
                stm_data.reset(false);
                if initialized_now {
                    Err("Nothing to be switched to.".to_owned())
                } else {
                    focus_urgent_or_matching_or_lru_window(
                        wins,
                        fdata,
                        stm_data,
                        pred,
                        ignore_pred,
                    )
                }
            }
        } else {
            log::debug!("No origin window; resetting.");
            stm_data.reset(false);
            if !initialized_now {
                focus_urgent_or_matching_or_lru_window(
                    wins,
                    fdata,
                    stm_data,
                    pred,
                    ignore_pred,
                )
            } else {
                Err("Nothing to be switched to.".to_owned())
            }
        }
    } else {
        log::debug!("Cycle exhausted; resetting.");
        stm_data.reset(false);
        if !initialized_now {
            focus_urgent_or_matching_or_lru_window(
                wins,
                fdata,
                stm_data,
                pred,
                ignore_pred,
            )
        } else {
            match focused {
                Some(win) if pred(win) => Ok(format!(
                    "The single matching window {focused_id} is already focused."
                )),
                _ => Err("Nothing to be switched to.".to_owned()),
            }
        }
    }
}

pub fn switch_to_app_or_urgent_or_lru_window(
    name: &str,
    stm_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let pred = |w: &t::DisplayNode| w.node.get_app_name() == name;

    focus_urgent_or_matching_or_lru_window(&wins, fdata, stm_data, pred, false)
}

pub fn switch_to_mark_or_urgent_or_lru_window(
    con_mark: &str,
    stm_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let con_mark = &con_mark.to_owned();
    let pred = |w: &t::DisplayNode| w.node.marks.contains(con_mark);

    focus_urgent_or_matching_or_lru_window(&wins, fdata, stm_data, pred, false)
}

fn switch_to_matching_or_urgent_or_lru_window(
    criteria: &str,
    switch_to_matching_data: &mut MutexGuard<SwitchToMatchingData>,
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);

    let crit = criteria::parse_criteria(criteria)?;
    let pred = criteria::criterion_to_predicate(&crit, &wins);
    focus_urgent_or_matching_or_lru_window(
        &wins,
        fdata,
        switch_to_matching_data,
        pred,
        false,
    )
}

static DIGIT_AND_NAME: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(\d):(.*)").unwrap());

fn create_workspace(ws_name: &str) -> Result<String, String> {
    if DIGIT_AND_NAME.is_match(ws_name) {
        run_sway_command(&["workspace", "number", ws_name])
    } else {
        run_sway_command(&["workspace", ws_name])
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

fn handle_non_matching_input(input: &str) -> Result<String, String> {
    if input.is_empty() {
        Err("Cannot handle empty string as non-matching input.".to_owned())
    } else if let Some(c) = SPECIAL_SWAY.captures(input) {
        let cmd = c[1].split_ascii_whitespace().collect::<Vec<&str>>();
        run_sway_command(&cmd).map(|msg| msg + " (for non-matching input)")
    } else {
        let ws = chop_workspace_shortcut(input);
        create_workspace(ws).map(|msg| msg + " (for non-matching input)")
    }
}

fn select_and_focus(
    prompt: &str,
    choices: &[t::DisplayNode],
) -> Result<String, String> {
    match util::select_from_menu(prompt, choices) {
        Ok(tn) => match tn.node.get_type() {
            ipc::Type::Output => {
                if tn.node.is_scratchpad() {
                    Err("Cannot switch to the scratchpad output.".to_owned())
                } else {
                    run_sway_command(&["focus output", tn.node.get_name()])
                }
            }
            ipc::Type::Workspace => {
                if tn.node.is_scratchpad() {
                    Err("Cannot switch to the scratchpad workspace.".to_owned())
                } else {
                    run_sway_command(&["workspace", tn.node.get_name()])
                }
            }
            ipc::Type::Window | ipc::Type::Container => {
                focus_window_by_id(tn.node.id)
            }
            t => {
                log::error!("Cannot handle {t:?} in select_and_focus");
                Err(format!("Cannot handle node type {t:?}."))
            }
        },
        Err(non_matching_input) => {
            handle_non_matching_input(&non_matching_input)
        }
    }
}

fn select_and_steal(
    prompt: &str,
    choices: &[t::DisplayNode],
) -> Result<String, String> {
    match util::select_from_menu(prompt, choices) {
        Ok(tn) => match tn.node.get_type() {
            ipc::Type::Window | ipc::Type::Container => {
                steal_window_by_id(tn.node.id)
            }
            ipc::Type::Workspace => {
                log::info!("Can't steal whole workspace");
                Err("Can't steal whole workspace".to_owned())
            }
            t => {
                log::error!("Cannot handle {t:?} in select_and_steal");
                Err(format!("Cannot handle {t:?}."))
            }
        },
        Err(non_matching_input) => {
            log::warn!(
                "Cannot handle non-matching input {non_matching_input:?} in select and steal"
            );
            Err("Cannot handle non-matching input.".to_owned())
        }
    }
}

pub fn switch_window(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus("Select window", &tree.get_windows(fdata))
}

fn retain_nodes_of_non_current_workspaces(
    tree: &t::Tree,
    nodes: &mut Vec<t::DisplayNode>,
) {
    if let Some(current) = tree.get_current_workspace() {
        nodes.retain(|w| {
            match tree.get_parent_node_of_type(w.node.id, ipc::Type::Workspace)
            {
                Some(ws) => &current != ws,
                None => true,
            }
        })
    };
}

pub fn steal_window(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    let wins = &mut tree.get_windows(fdata);
    retain_nodes_of_non_current_workspaces(&tree, wins);
    select_and_steal("Select window", wins)
}

pub fn steal_window_or_container(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    let wins_and_ws = &mut tree.get_workspaces_containers_and_windows(fdata);
    retain_nodes_of_non_current_workspaces(&tree, wins_and_ws);
    select_and_steal("Select window or container", wins_and_ws)
}

pub fn switch_workspace(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    select_and_focus("Select workspace", &tree.get_workspaces(fdata))
}

pub fn switch_output() -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    select_and_focus("Select output", &tree.get_outputs())
}

pub fn switch_workspace_or_window(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus(
        "Select workspace or window",
        &tree.get_workspaces_and_windows(fdata),
    )
}

pub fn switch_workspace_container_or_window(
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus(
        "Select workspace, container or window",
        &tree.get_workspaces_containers_and_windows(fdata),
    )
}

pub fn switch_to(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_focus(
        "Select output, workspace, container or window",
        &tree.get_outputs_workspaces_containers_and_windows(fdata),
    )
}

fn kill_process_by_pid(pid: Option<i32>) -> Result<String, String> {
    if let Some(pid) = pid {
        match std::process::Command::new("kill")
            .arg("-9")
            .arg(format!("{pid}"))
            .output()
        {
            Err(err) => {
                log::error!("Error killing process {pid}: {err}");
                Err(err.to_string())
            }
            _ => Ok(format!("Killed process with pid {pid}.")),
        }
    } else {
        log::error!("Cannot kill window with no pid.");
        Err("No pid to kill given.".to_owned())
    }
}

fn select_and_quit(
    prompt: &str,
    choices: &[t::DisplayNode],
    kill: bool,
) -> Result<String, String> {
    match util::select_from_menu(prompt, choices) {
        Ok(tn) => match tn.node.get_type() {
            ipc::Type::Workspace | ipc::Type::Container => {
                for win in
                    tn.node.iter().filter(|n| n.get_type() == ipc::Type::Window)
                {
                    match quit_window_by_id(win.id) {
                        Ok(_) => (),
                        e @ Err(_) => return e,
                    }
                }
                Ok(format!(
                    "Quit all windows on {:?} {}.",
                    tn.swayr_type,
                    tn.node.get_name()
                ))
            }
            ipc::Type::Window => {
                if kill {
                    kill_process_by_pid(tn.node.pid)
                } else {
                    quit_window_by_id(tn.node.id)
                }
            }
            t => {
                log::error!("Cannot handle {t:?} in select_and_quit");
                Err(format!("Cannot handle container of type {t:?}."))
            }
        },
        Err(err) => Err(err),
    }
}

pub fn quit_window(fdata: &FocusData, kill: bool) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_quit("Quit window", &tree.get_windows(fdata), kill)
}

pub fn quit_workspace_or_window(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_quit(
        "Quit workspace or window",
        &tree.get_workspaces_and_windows(fdata),
        false,
    )
}

pub fn quit_workspace_container_or_window(
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_quit(
        "Quit workspace, container or window",
        &tree.get_workspaces_containers_and_windows(fdata),
        false,
    )
}

fn move_focused_to_workspace_1(ws_name: &str) -> Result<String, String> {
    if DIGIT_AND_NAME.is_match(ws_name) {
        run_sway_command(&[
            "move",
            "container",
            "to",
            "workspace",
            "number",
            ws_name,
        ])
    } else {
        run_sway_command(&["move", "container", "to", "workspace", ws_name])
    }
}

fn move_focused_to_container_or_window(id: i64) -> Result<String, String> {
    run_sway_command(&[
        &format!("[con_id={id}"),
        "mark",
        "--add",
        "__SWAYR_MOVE_TARGET__",
    ])?;
    run_sway_command(&["move", "to", "mark", "__SWAYR_MOVE_TARGET__"])?;
    run_sway_command(&["unmark", "__SWAYR_MOVE_TARGET__"])
}

fn select_and_move_focused_to(
    prompt: &str,
    choices: &[t::DisplayNode],
) -> Result<String, String> {
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
            t => {
                log::error!("Cannot move focused to {t:?}");
                Err(format!("Cannot move focused to node of type {t:?}."))
            }
        },
        Err(input) => {
            let ws_name = chop_workspace_shortcut(&input);
            move_focused_to_workspace_1(ws_name)
        }
    }
}

pub fn move_focused_to_workspace(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_move_focused_to(
        "Move focused container to workspace",
        &tree.get_workspaces(fdata),
    )
}

pub fn move_focused_to(fdata: &FocusData) -> Result<String, String> {
    let root = ipc::get_root_node(true);
    let tree = t::get_tree(&root);
    select_and_move_focused_to(
        "Move focused container to workspace or container",
        &tree.get_outputs_workspaces_containers_and_windows(fdata),
    )
}

pub fn swap_focused_with(fdata: &FocusData) -> Result<String, String> {
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
            t => {
                let msg = format!("Cannot swap with container of type {t:?}.");
                log::error!("{msg}");
                Err(msg)
            }
        },
        Err(_) => Err("No swap target selected from menu.".to_owned()),
    }
}

pub enum Direction {
    Backward,
    Forward,
}

fn focus_window_in_direction_1(
    wins: &[t::DisplayNode],
    dir: Direction,
    fdata: &FocusData,
    pred: impl Fn(&t::DisplayNode) -> bool,
) -> Result<String, String> {
    let mut wins: Vec<&t::DisplayNode> =
        wins.iter().filter(|w| pred(w)).collect();

    if wins.is_empty() {
        return Err("No matching windows.".to_owned());
    }

    wins.sort_by(|a, b| {
        let lru_a = fdata.last_focus_tick(a.node.id);
        let lru_b = fdata.last_focus_tick(b.node.id);
        lru_a.cmp(&lru_b).reverse()
    });

    let is_focused_window: Box<dyn Fn(&t::DisplayNode) -> bool> =
        if !wins.iter().any(|w| w.node.focused) {
            let last_focused_win_id = wins.first().unwrap().node.id;
            Box::new(move |dn| dn.node.id == last_focused_win_id)
        } else {
            Box::new(|dn| dn.node.focused)
        };

    let mut iter: Box<dyn Iterator<Item = &&t::DisplayNode>> = match dir {
        Direction::Forward => Box::new(wins.iter().rev().cycle()),
        Direction::Backward => Box::new(wins.iter().cycle()),
    };

    loop {
        let win = iter.next().unwrap();
        if is_focused_window(win) {
            let win = iter.next().unwrap();
            return focus_window_by_id(win.node.id);
        }
    }
}

fn focus_matching_window_in_direction(
    dir: Direction,
    criteria: &str,
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);

    let crits = criteria::parse_criteria(criteria)?;
    let pred = criteria::criterion_to_predicate(&crits, &wins);
    focus_window_in_direction_1(&wins, dir, fdata, pred)
}

pub fn focus_window_in_direction(
    dir: Direction,
    consider_wins: &ConsiderWindows,
    fdata: &FocusData,
    pred: impl Fn(&t::DisplayNode) -> bool,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let mut wins = tree.get_windows(fdata);

    if consider_wins == &ConsiderWindows::CurrentWorkspace {
        if let Some(cur_ws) = tree.get_current_workspace() {
            wins.retain(|w| {
                tree.get_parent_node_of_type(w.node.id, ipc::Type::Workspace)
                    .unwrap()
                    .id
                    == cur_ws.id
            });
        } else {
            return Err("No current workspace!".to_owned());
        };
    }

    focus_window_in_direction_1(&wins, dir, fdata, pred)
}

pub fn focus_window_of_same_layout_in_direction(
    dir: Direction,
    consider_wins: &ConsiderWindows,
    fdata: &FocusData,
) -> Result<String, String> {
    let root = ipc::get_root_node(false);
    let tree = t::get_tree(&root);
    let wins = tree.get_windows(fdata);
    let cur_win = wins.iter().find(|w| w.node.focused);

    match cur_win {
        Some(cur_win) => focus_window_in_direction(
            dir,
            consider_wins,
            fdata,
            if cur_win.node.is_floating() {
                |dn: &t::DisplayNode| dn.node.is_floating()
            } else if !cur_win.node.is_floating()
                && cur_win
                    .tree
                    .is_child_of_tabbed_or_stacked_container(cur_win.node.id)
            {
                |dn: &t::DisplayNode| {
                    !dn.node.is_floating()
                        && dn
                            .tree
                            .is_child_of_tabbed_or_stacked_container(dn.node.id)
                }
            } else if !cur_win.node.is_floating()
                && cur_win.tree.is_child_of_tiled_container(cur_win.node.id)
            {
                |dn: &t::DisplayNode| {
                    !dn.node.is_floating()
                        && dn.tree.is_child_of_tiled_container(dn.node.id)
                }
            } else {
                always_true
            },
        ),
        None => Err("There's no focused window.".to_owned()),
    }
}

fn tile_current_workspace(
    floating: &ConsiderFloating,
    shuffle: bool,
) -> Result<String, String> {
    layout::relayout_current_workspace(
        floating == &ConsiderFloating::IncludeFloating,
        move |wins, con: &mut s::Connection| {
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
        },
    )
}

fn tab_current_workspace(
    floating: &ConsiderFloating,
) -> Result<String, String> {
    layout::relayout_current_workspace(
        floating == &ConsiderFloating::IncludeFloating,
        move |wins, con: &mut s::Connection| {
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
        },
    )
}

fn toggle_tab_tile_current_workspace(
    floating: &ConsiderFloating,
) -> Result<String, String> {
    let tree = ipc::get_root_node(false);
    let workspaces = tree.nodes_of_type(ipc::Type::Workspace);
    if let Some(cur_ws) = workspaces.iter().find(|w| w.is_current()) {
        if cur_ws.layout == s::NodeLayout::Tabbed {
            tile_current_workspace(floating, true)
        } else {
            tab_current_workspace(floating)
        }
    } else {
        Err("No current workspace!".to_owned())
    }
}

fn get_swaymsg_commands() -> Vec<SwaymsgCmd> {
    let mut sm_cmds: Vec<SwaymsgCmd> = vec![];

    if let Some(custom_commands) = CONFIG.get_swaymsg_commands_commands() {
        for (label, cmd) in custom_commands {
            sm_cmds.push(SwaymsgCmd {
                label: Some(label),
                cmd,
            })
        }
    }

    if CONFIG.get_swaymsg_commands_include_predefined() {
        let mut cmds: Vec<String> = vec![];
        for b in &["none", "normal", "csd", "pixel"] {
            cmds.push(format!["border {b}"]);
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
            cmds.push(format!["inhibit_idle {x}"])
        }

        for l in &["default", "splith", "splitv", "stacking", "tiling"] {
            cmds.push(format!["layout {l}"])
        }

        for e in &["enable", "disable"] {
            cmds.push(format!["shortcuts_inhibitor {e}"])
        }

        for x in &["yes", "no", "always"] {
            cmds.push(format!["focus_follows_mouse {x}"])
        }

        for x in &["smart", "urgent", "focus", "none"] {
            cmds.push(format!["focus_on_window_activation {x}"])
        }

        for x in &["yes", "no", "force", "workspace"] {
            cmds.push(format!["focus_wrapping {x}"])
        }

        for x in &[
            "none",
            "vertical",
            "horizontal",
            "both",
            "smart",
            "smart_no_gaps",
        ] {
            cmds.push(format!["hide_edge_borders {x}"])
        }

        for x in &["on", "no_gaps", "off"] {
            cmds.push(format!["smart_borders {x}"])
        }

        for x in &["on", "off"] {
            cmds.push(format!["smart_gaps {x}"])
        }

        for x in &["output", "container", "none"] {
            cmds.push(format!["mouse_warping {x}"])
        }

        for x in &["smart", "ignore", "leave_fullscreen"] {
            cmds.push(format!["popup_during_fullscreen {x}"])
        }

        for x in &["yes", "no"] {
            cmds.push(format!["show_marks {x}"]);
            cmds.push(format!["workspace_auto_back_and_forth {x}"]);
        }

        for x in &["left", "center", "right"] {
            cmds.push(format!["title_align {x}"]);
        }

        for x in &["enable", "disable", "allow", "deny"] {
            cmds.push(format!["urgent {x}"])
        }

        cmds.sort();

        cmds.into_iter()
            .map(|c| SwaymsgCmd {
                label: None,
                cmd: c,
            })
            .for_each(|smc| sm_cmds.push(smc));
    }

    sm_cmds
}

struct SwaymsgCmd {
    label: Option<String>,
    cmd: String,
}

impl DisplayFormat for SwaymsgCmd {
    fn format_for_display(&self) -> String {
        if let Some(label) = &self.label {
            format!("{label}: {}", self.cmd)
        } else {
            self.cmd.clone()
        }
    }

    fn get_indent_level(&self) -> usize {
        0
    }
}

pub fn exec_swaymsg_command() -> Result<String, String> {
    let cmds = get_swaymsg_commands();
    let cmd = util::select_from_menu("Execute swaymsg command", &cmds);
    match cmd {
        Ok(cmd) => run_sway_command_1(&cmd.cmd),
        Err(cmd) if !cmd.is_empty() => {
            let cmd = chop_sway_shortcut(&cmd);
            run_sway_command_1(cmd)
        }
        Err(_) => {
            Err("No command selected nor manually typed command given."
                .to_owned())
        }
    }
}

pub fn configure_outputs() -> Result<String, String> {
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

    let cmds: Vec<SwaymsgCmd> = cmds
        .into_iter()
        .map(|c| SwaymsgCmd {
            label: None,
            cmd: c,
        })
        .collect();
    let mut last_cmd_result: Result<String, String> =
        Err("No output command selected.".to_owned());
    loop {
        match util::select_from_menu("Output command", &cmds) {
            Ok(cmd) => match run_sway_command_1(&cmd.cmd) {
                Ok(msg) => {
                    last_cmd_result = if last_cmd_result.is_ok() {
                        last_cmd_result.map(|s| s + "\n" + msg.as_str())
                    } else {
                        Ok(msg)
                    };
                }
                Err(_) => return last_cmd_result,
            },
            Err(_) => return last_cmd_result,
        }
    }
}
