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
use lazy_static::lazy_static;
use rand::prelude::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic;
use std::sync::Arc;
use std::sync::RwLock;
use swayipc as s;

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
    /// Switch to next urgent window (if any) or to last recently used window.
    SwitchToUrgentOrLRUWindow,
    /// Focus the selected window.
    SwitchWindow,
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
    /// Quit the selected window.
    QuitWindow,
    /// Switch to the selected workspace.
    SwitchWorkspace,
    /// Switch to the selected workspace or focus the selected window.
    SwitchWorkspaceOrWindow,
    /// Quit all windows of selected workspace or the selected window.
    QuitWorkspaceOrWindow,
    /// Move the currently focused window or container to the selected workspace.
    MoveFocusedToWorkspace,
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
    pub extra_props: Arc<RwLock<HashMap<i64, con::ExtraProps>>>,
}

impl DisplayFormat for SwayrCommand {
    fn format_for_display(&self, _: &cfg::Config) -> std::string::String {
        // TODO: Add a format to Config
        format!("{:?}", self)
    }
}

fn always_true(_x: &con::DisplayNode) -> bool {
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
                val.last_focus_time_for_next_prev_seq = val.last_focus_time;
            }
        }
    } else {
        IN_NEXT_PREV_WINDOW_SEQ.store(false, atomic::Ordering::SeqCst);
    }

    match args.cmd {
        SwayrCommand::SwitchToUrgentOrLRUWindow => {
            switch_to_urgent_or_lru_window(&*props.read().unwrap())
        }
        SwayrCommand::SwitchWindow => switch_window(&*props.read().unwrap()),
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
            Box::new(|dn: &con::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            }),
        ),
        SwayrCommand::PrevTiledWindow { windows } => focus_window_in_direction(
            Direction::Backward,
            windows,
            &*props.read().unwrap(),
            Box::new(|dn: &con::DisplayNode| {
                !dn.node.is_floating()
                    && dn.tree.is_child_of_tiled_container(dn.node.id)
            }),
        ),
        SwayrCommand::NextTabbedOrStackedWindow { windows } => {
            focus_window_in_direction(
                Direction::Forward,
                windows,
                &*props.read().unwrap(),
                Box::new(|dn: &con::DisplayNode| {
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
                Box::new(|dn: &con::DisplayNode| {
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
                Box::new(|dn: &con::DisplayNode| dn.node.is_floating()),
            )
        }
        SwayrCommand::PrevFloatingWindow { windows } => {
            focus_window_in_direction(
                Direction::Backward,
                windows,
                &*props.read().unwrap(),
                Box::new(|dn: &con::DisplayNode| dn.node.is_floating()),
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
        SwayrCommand::MoveFocusedToWorkspace => {
            move_focused_container_to_workspace(&*props.read().unwrap())
        }
        SwayrCommand::QuitWindow => quit_window(&*props.read().unwrap()),
        SwayrCommand::SwitchWorkspace => {
            switch_workspace(&*props.read().unwrap())
        }
        SwayrCommand::SwitchWorkspaceOrWindow => {
            switch_workspace_or_window(&*props.read().unwrap())
        }
        SwayrCommand::QuitWorkspaceOrWindow => {
            quit_workspace_or_window(&*props.read().unwrap())
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
                SwayrCommand::MoveFocusedToWorkspace,
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

#[deprecated]
pub fn get_tree_old() -> s::Node {
    match s::Connection::new() {
        Ok(mut con) => con.get_tree().expect("Got no root node"),
        Err(err) => panic!("{}", err),
    }
}

pub fn get_tree(include_scratchpad: bool) -> s::Node {
    match s::Connection::new() {
        Ok(mut con) => {
            let mut root = con.get_tree().expect("Got no root node");
            if !include_scratchpad {
                root.nodes.retain(|o| !o.is_scratchpad());
            }
            root
        }
        Err(err) => panic!("{}", err),
    }
}

pub fn switch_to_urgent_or_lru_window(
    extra_props: &HashMap<i64, con::ExtraProps>,
) {
    let root = get_tree(false);
    let tree = con::get_tree(&root, extra_props);
    if let Some(win) = tree.get_windows().get(0) {
        println!(
            "Switching to {}, id: {}",
            win.node.get_app_name(),
            win.node.id
        );
        focus_window_by_id(win.node.id)
    } else {
        println!("No window to switch to.")
    }
}

lazy_static! {
    static ref DIGIT_AND_NAME: regex::Regex =
        regex::Regex::new(r"^(\d):(.*)").unwrap();
}

fn create_workspace(ws_name: &str) {
    if DIGIT_AND_NAME.is_match(ws_name) {
        run_sway_command(&["workspace", "number", ws_name]);
    } else {
        run_sway_command(&["workspace", ws_name]);
    }
}

lazy_static! {
    static ref SPECIAL_WORKSPACE: regex::Regex =
        regex::Regex::new(r"^#*w:(.*)").unwrap();
    static ref SPECIAL_SWAY: regex::Regex =
        regex::Regex::new(r"^#*s:(.*)").unwrap();
}

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

pub fn switch_window(extra_props: &HashMap<i64, con::ExtraProps>) {
    let root = get_tree(true);
    let tree = con::get_tree(&root, extra_props);

    match util::select_from_menu("Switch to window", &tree.get_windows()) {
        Ok(window) => focus_window_by_id(window.node.id),
        Err(non_matching_input) => {
            handle_non_matching_input(&non_matching_input)
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
    extra_props: &HashMap<i64, con::ExtraProps>,
    pred: Box<dyn Fn(&con::DisplayNode) -> bool>,
) {
    let root = get_tree(false);
    let tree = con::get_tree(&root, extra_props);
    let mut wins = tree.get_windows();

    if consider_wins == &ConsiderWindows::CurrentWorkspace {
        let cur_ws = tree.get_current_workspace();
        wins.retain(|w| {
            tree.get_workspace_node(w.node.id).unwrap().id == cur_ws.id
        });
    }

    wins.retain(pred);

    if wins.len() < 2 {
        return;
    }

    wins.sort_by(|a, b| {
        let lru_a = tree.last_focus_time_for_next_prev_seq(a.node.id);
        let lru_b = tree.last_focus_time_for_next_prev_seq(b.node.id);
        lru_a.cmp(&lru_b).reverse()
    });

    let is_focused_window: Box<dyn Fn(&con::DisplayNode) -> bool> =
        if !wins.iter().any(|w| w.node.focused) {
            let last_focused_win_id = wins.get(0).unwrap().node.id;
            Box::new(move |dn| dn.node.id == last_focused_win_id)
        } else {
            Box::new(|dn| dn.node.focused)
        };

    let mut iter: Box<dyn Iterator<Item = &con::DisplayNode>> = match dir {
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
    extra_props: &HashMap<i64, con::ExtraProps>,
) {
    let root = get_tree(false);
    let tree = con::get_tree(&root, extra_props);
    let wins = tree.get_windows();
    let cur_win = wins.get(0);

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

pub fn switch_workspace(extra_props: &HashMap<i64, con::ExtraProps>) {
    let root = get_tree(false);
    let tree = con::get_tree(&root, extra_props);

    match util::select_from_menu("Switch to workspace", &tree.get_workspaces())
    {
        Ok(workspace) => {
            run_sway_command(&["workspace", workspace.node.get_name()])
        }
        Err(non_matching_input) => {
            handle_non_matching_input(&non_matching_input)
        }
    }
}

pub fn move_focused_container_to_workspace(
    extra_props: &HashMap<i64, con::ExtraProps>,
) {
    let root = get_tree(true);
    let tree = con::get_tree(&root, extra_props);
    let workspaces = tree.get_workspaces();

    let val = util::select_from_menu(
        "Move focused container to workspace",
        &workspaces,
    );
    let ws_name = &match val {
        Ok(workspace) => String::from(workspace.node.get_name()),
        Err(input) => String::from(chop_workspace_shortcut(&input)),
    };

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

pub fn switch_workspace_or_window(extra_props: &HashMap<i64, con::ExtraProps>) {
    let root = get_tree(true);
    let tree = con::get_tree(&root, extra_props);
    let ws_or_wins = tree.get_workspaces_and_windows();
    match util::select_from_menu("Select workspace or window", &ws_or_wins) {
        Ok(tn) => match tn.node.get_type() {
            con::Type::Workspace => {
                if !tn.node.is_scratchpad() {
                    run_sway_command(&["workspace", tn.node.get_name()]);
                }
            }
            con::Type::Window => focus_window_by_id(tn.node.id),
            t => {
                eprintln!("Cannot handle {:?} in switch_workspace_or_window", t)
            }
        },
        Err(non_matching_input) => {
            handle_non_matching_input(&non_matching_input)
        }
    }
}

pub fn quit_window(extra_props: &HashMap<i64, con::ExtraProps>) {
    let root = get_tree(true);
    let tree = con::get_tree(&root, extra_props);

    if let Ok(window) =
        util::select_from_menu("Quit window", &tree.get_windows())
    {
        quit_window_by_id(window.node.id)
    }
}

pub fn quit_workspace_or_window(extra_props: &HashMap<i64, con::ExtraProps>) {
    let root = get_tree(true);
    let tree = con::get_tree(&root, extra_props);
    let ws_or_wins = tree.get_workspaces_and_windows();
    if let Ok(tn) =
        util::select_from_menu("Quit workspace or window", &ws_or_wins)
    {
        match tn.node.get_type() {
            con::Type::Workspace => {
                for win in
                    tn.node.iter().filter(|n| n.get_type() == con::Type::Window)
                {
                    quit_window_by_id(win.id)
                }
            }
            con::Type::Window => quit_window_by_id(tn.node.id),
            t => {
                eprintln!("Cannot handle {:?} in quit_workspace_or_window", t)
            }
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
        Err(err) => eprintln!("Error retiling workspace: {:?}", err),
    }
}

fn toggle_tab_tile_current_workspace(floating: &ConsiderFloating) {
    let tree = get_tree(false);
    let workspaces = tree.nodes_of_type(con::Type::Workspace);
    let cur_ws = workspaces.iter().find(|w| w.is_current()).unwrap();
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
    match cmd {
        Ok(cmd) => run_sway_command(&cmd.cmd),
        Err(cmd) if !cmd.is_empty() => {
            let cmd = chop_sway_shortcut(&cmd);
            run_sway_command(
                &cmd.split_ascii_whitespace().collect::<Vec<&str>>(),
            );
        }
        Err(_) => (),
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
