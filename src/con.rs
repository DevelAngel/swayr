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

//! Convenience data structures built from the IPC structs.

use crate::config as cfg;
use crate::util;
use crate::util::DisplayFormat;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::cmp;
use std::collections::HashMap;
use swayipc as s;

/// Immutable Node Iterator
///
/// Iterates nodes in depth-first order, tiled nodes before floating nodes.
pub struct NodeIter<'a> {
    stack: Vec<&'a s::Node>,
}

impl<'a> NodeIter<'a> {
    pub fn new(node: &'a s::Node) -> NodeIter {
        NodeIter { stack: vec![node] }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a s::Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            for n in &node.floating_nodes {
                self.stack.push(n);
            }
            for n in &node.nodes {
                self.stack.push(n);
            }
            Some(node)
        } else {
            None
        }
    }
}

/// Extension methods for [`swayipc::Node`].
pub trait NodeMethods {
    /// Returns an iterator for this [`swayipc::Node`] and its childres.
    fn iter(&self) -> NodeIter;

    fn is_window(&self) -> bool;

    /// Either a workspace or a con holding windows, e.g. a vertical split side
    /// in a horizontally split workspace.
    fn is_container(&self) -> bool;

    /// Returns all nodes being application windows.
    fn windows(&self) -> Vec<&s::Node>;

    /// Returns all nodes being workspaces.
    fn workspaces(&self) -> Vec<&s::Node>;

    fn is_scratchpad(&self) -> bool;
}

impl NodeMethods for s::Node {
    fn iter(&self) -> NodeIter {
        NodeIter::new(self)
    }

    fn is_window(&self) -> bool {
        (self.node_type == s::NodeType::Con
            || self.node_type == s::NodeType::FloatingCon)
            && self.name.is_some()
    }

    fn is_container(&self) -> bool {
        self.node_type == s::NodeType::Workspace
            || self.node_type == s::NodeType::Con
                && self.name.is_none()
                && self.layout != s::NodeLayout::None
    }

    fn windows(&self) -> Vec<&s::Node> {
        self.iter().filter(|n| n.is_window()).collect()
    }

    fn workspaces(&self) -> Vec<&s::Node> {
        self.iter()
            .filter(|n| n.node_type == s::NodeType::Workspace)
            .collect()
    }

    fn is_scratchpad(&self) -> bool {
        self.name.is_some() && self.name.as_ref().unwrap().eq("__i3_scratch")
    }
}

/// Extra properties gathered by swayrd for windows and workspaces.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExtraProps {
    /// Milliseconds since UNIX epoch.
    pub last_focus_time: u128,
    pub last_focus_time_for_next_prev_seq: u128,
}

#[derive(Debug)]
pub struct Window<'a> {
    node: &'a s::Node,
    workspace: &'a s::Node,
    extra_props: Option<ExtraProps>,
}

impl Window<'_> {
    pub fn get_id(&self) -> i64 {
        self.node.id
    }

    pub fn get_app_name(&self) -> &str {
        if let Some(app_id) = &self.node.app_id {
            app_id
        } else if let Some(wp_class) = self
            .node
            .window_properties
            .as_ref()
            .and_then(|wp| wp.class.as_ref())
        {
            wp_class
        } else {
            "<Unknown>"
        }
    }

    pub fn get_title(&self) -> &str {
        self.node.name.as_ref().unwrap()
    }

    pub fn is_urgent(&self) -> bool {
        self.node.urgent
    }

    pub fn is_focused(&self) -> bool {
        self.node.focused
    }

    pub fn is_floating(&self) -> bool {
        self.node.node_type == s::NodeType::FloatingCon
    }

    pub fn get_parent(&self) -> &s::Node {
        NodeIter::new(self.workspace)
            .find(|n| {
                n.nodes.contains(self.node)
                    || n.floating_nodes.contains(self.node)
            })
            .unwrap_or_else(|| panic!("Window {:?} has no parent node!", self))
    }

    pub fn is_child_of_tiled_container(&self) -> bool {
        let layout = &self.get_parent().layout;
        layout == &s::NodeLayout::SplitH || layout == &s::NodeLayout::SplitV
    }

    pub fn is_child_of_tabbed_or_stacked_container(&self) -> bool {
        let layout = &self.get_parent().layout;
        layout == &s::NodeLayout::Tabbed || layout == &s::NodeLayout::Stacked
    }

    pub fn last_focus_time_for_next_prev_seq(&self) -> u128 {
        self.extra_props
            .as_ref()
            .map_or(0, |wp| wp.last_focus_time_for_next_prev_seq)
    }
}

impl PartialEq for Window<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Eq for Window<'_> {}

impl Ord for Window<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        if self == other {
            cmp::Ordering::Equal
        } else if self.is_urgent() && !other.is_urgent()
            || !self.is_focused() && other.is_focused()
        {
            cmp::Ordering::Less
        } else if !self.is_urgent() && other.is_urgent()
            || self.is_focused() && !other.is_focused()
        {
            std::cmp::Ordering::Greater
        } else {
            let lru_a =
                self.extra_props.as_ref().map_or(0, |wp| wp.last_focus_time);
            let lru_b = other
                .extra_props
                .as_ref()
                .map_or(0, |wp| wp.last_focus_time);
            lru_a.cmp(&lru_b).reverse()
        }
    }
}

impl PartialOrd for Window<'_> {
    fn partial_cmp(&self, other: &Window) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

lazy_static! {
    static ref APP_NAME_AND_VERSION_RX: regex::Regex =
        regex::Regex::new("(.+)(-[0-9.]+)").unwrap();
}

fn maybe_html_escape(do_it: bool, text: &str) -> String {
    if do_it {
        text.replace("<", "&lt;")
            .replace(">", "&gt;")
            .replace("&", "&amp;")
    } else {
        text.to_string()
    }
}

impl<'a> DisplayFormat for Window<'a> {
    fn format_for_display(&self, cfg: &cfg::Config) -> String {
        let window_format = cfg.get_format_window_format();
        let urgency_start = cfg.get_format_urgency_start();
        let urgency_end = cfg.get_format_urgency_end();
        let html_escape = cfg.get_format_html_escape();
        let icon_dirs = cfg.get_format_icon_dirs();
        // fallback_icon has no default value.
        let fallback_icon: Option<Box<std::path::Path>> = cfg
            .get_format_fallback_icon()
            .as_ref()
            .map(|i| std::path::Path::new(i).to_owned().into_boxed_path());

        // Some apps report, e.g., Gimp-2.10 but the icon is still named
        // gimp.png.
        let app_name_no_version =
            APP_NAME_AND_VERSION_RX.replace(self.get_app_name(), "$1");

        window_format
            .replace("{id}", format!("{}", self.get_id()).as_str())
            .replace(
                "{urgency_start}",
                if self.is_urgent() {
                    urgency_start.as_str()
                } else {
                    ""
                },
            )
            .replace(
                "{urgency_end}",
                if self.is_urgent() {
                    urgency_end.as_str()
                } else {
                    ""
                },
            )
            .replace(
                "{app_name}",
                &maybe_html_escape(html_escape, self.get_app_name()),
            )
            .replace(
                "{workspace_name}",
                &maybe_html_escape(
                    html_escape,
                    self.workspace.name.as_ref().unwrap().as_str(),
                ),
            )
            .replace(
                "{marks}",
                &maybe_html_escape(html_escape, &self.node.marks.join(", ")),
            )
            .replace(
                "{app_icon}",
                util::get_icon(self.get_app_name(), &icon_dirs)
                    .or_else(|| {
                        util::get_icon(&app_name_no_version, &icon_dirs)
                    })
                    .or_else(|| {
                        util::get_icon(
                            &app_name_no_version.to_lowercase(),
                            &icon_dirs,
                        )
                    })
                    .or(fallback_icon)
                    .map(|i| i.to_string_lossy().into_owned())
                    .unwrap_or_else(String::new)
                    .as_str(),
            )
            .replace(
                "{title}",
                &maybe_html_escape(html_escape, self.get_title()),
            )
    }
}

fn build_windows<'a>(
    root: &'a s::Node,
    include_scratchpad_windows: bool,
    extra_props: &HashMap<i64, ExtraProps>,
) -> Vec<Window<'a>> {
    let mut v = vec![];
    for workspace in root.workspaces() {
        if !include_scratchpad_windows && workspace.is_scratchpad() {
            continue;
        }

        for n in workspace.windows() {
            v.push(Window {
                node: n,
                extra_props: extra_props.get(&n.id).cloned(),
                workspace,
            })
        }
    }
    v
}

fn build_workspaces<'a>(
    root: &'a s::Node,
    include_scratchpad: bool,
    extra_props: &HashMap<i64, ExtraProps>,
) -> Vec<Workspace<'a>> {
    let mut v = vec![];
    for workspace in root.workspaces() {
        if workspace.is_scratchpad() && !include_scratchpad {
            continue;
        }

        let mut wins: Vec<Window> = workspace
            .windows()
            .iter()
            .map(|w| Window {
                node: w,
                extra_props: extra_props.get(&w.id).cloned(),
                workspace,
            })
            .collect();
        if !extra_props.is_empty() {
            wins.sort();
        }
        v.push(Workspace {
            node: workspace,
            extra_props: extra_props.get(&workspace.id).cloned(),
            windows: wins,
        })
    }
    if !extra_props.is_empty() {
        v.sort();
    }
    v
}

/// Gets all application windows of the tree.
pub fn get_windows<'a>(
    root: &'a s::Node,
    include_scratchpad_windows: bool,
    extra_props: &HashMap<i64, ExtraProps>,
) -> Vec<Window<'a>> {
    let mut wins = build_windows(root, include_scratchpad_windows, extra_props);
    if !extra_props.is_empty() {
        wins.sort();
    }
    wins
}

/// Gets all workspaces of the tree.
pub fn get_workspaces<'a>(
    root: &'a s::Node,
    include_scratchpad: bool,
    extra_props: &HashMap<i64, ExtraProps>,
) -> Vec<Workspace<'a>> {
    let mut workspaces =
        build_workspaces(root, include_scratchpad, extra_props);
    workspaces.rotate_left(1);
    workspaces
}

pub enum WsOrWin<'a> {
    Ws { ws: &'a Workspace<'a> },
    Win { win: &'a Window<'a> },
}

impl DisplayFormat for WsOrWin<'_> {
    fn format_for_display(&self, cfg: &cfg::Config) -> String {
        match self {
            WsOrWin::Ws { ws } => ws.format_for_display(cfg),
            WsOrWin::Win { win } => win.format_for_display(cfg),
        }
    }
}

impl WsOrWin<'_> {
    pub fn from_workspaces<'a>(
        workspaces: &'a [Workspace],
    ) -> Vec<WsOrWin<'a>> {
        let mut v = vec![];
        for ws in workspaces {
            v.push(WsOrWin::Ws { ws });
            for win in &ws.windows {
                v.push(WsOrWin::Win { win });
            }
        }
        v
    }
}

pub struct Workspace<'a> {
    pub node: &'a s::Node,
    extra_props: Option<ExtraProps>,
    pub windows: Vec<Window<'a>>,
}

impl Workspace<'_> {
    pub fn get_name(&self) -> &str {
        self.node.name.as_ref().unwrap()
    }

    pub fn get_id(&self) -> i64 {
        self.node.id
    }

    pub fn is_scratchpad(&self) -> bool {
        self.node.is_scratchpad()
    }

    pub fn is_current(&self) -> bool {
        is_current_container(self.node)
    }
}

pub fn is_current_container(node: &s::Node) -> bool {
    node.focused || NodeIter::new(node).any(|c| c.focused)
}

impl PartialEq for Workspace<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Eq for Workspace<'_> {}

impl Ord for Workspace<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        if self == other {
            cmp::Ordering::Equal
        } else {
            let lru_a =
                self.extra_props.as_ref().map_or(0, |wp| wp.last_focus_time);
            let lru_b = other
                .extra_props
                .as_ref()
                .map_or(0, |wp| wp.last_focus_time);
            lru_a.cmp(&lru_b).reverse()
        }
    }
}

impl PartialOrd for Workspace<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> DisplayFormat for Workspace<'a> {
    fn format_for_display(&self, cfg: &cfg::Config) -> String {
        let fmt = cfg.get_format_workspace_format();
        let html_escape = cfg.get_format_html_escape();

        fmt.replace("{id}", format!("{}", self.get_id()).as_str())
            .replace("{name}", &maybe_html_escape(html_escape, self.get_name()))
    }
}
