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

use crate::config;
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Type {
    Root,
    Output,
    Workspace,
    Container,
    Window,
}

/// Extension methods for [`swayipc::Node`].
pub trait NodeMethods {
    /// Returns an iterator for this [`swayipc::Node`] and its childres.
    fn iter(&self) -> NodeIter;

    /// Returns true if this node is an output.
    fn get_type(&self) -> Type;

    /// Returns the app_id if present, otherwise the window-properties class if
    /// present, otherwise "<unknown_app>".
    fn get_app_name(&self) -> &str;

    fn nodes_of_type(&self, t: Type) -> Vec<&s::Node>;
    fn get_name(&self) -> &str;

    // Returns true if this node is the scratchpad output or workspace.
    fn is_scratchpad(&self) -> bool;
    fn is_floating(&self) -> bool;

    fn is_current(&self) -> bool {
        self.iter().any(|n| n.focused)
    }
}

impl NodeMethods for s::Node {
    fn iter(&self) -> NodeIter {
        NodeIter::new(self)
    }

    fn get_type(&self) -> Type {
        match self.node_type {
            s::NodeType::Root => Type::Root,
            s::NodeType::Output => Type::Output,
            s::NodeType::Workspace => Type::Workspace,
            s::NodeType::FloatingCon => Type::Window,
            _ => {
                if self.node_type == s::NodeType::Con
                    && self.name.is_none()
                    && self.layout != s::NodeLayout::None
                {
                    Type::Container
                } else if (self.node_type == s::NodeType::Con
                    || self.node_type == s::NodeType::FloatingCon)
                    && self.name.is_some()
                {
                    Type::Window
                } else {
                    panic!(
                        "Don't know type of node with id {} and node_type {:?}",
                        self.id, self.node_type
                    )
                }
            }
        }
    }

    fn get_name(&self) -> &str {
        if let Some(name) = &self.name {
            name.as_ref()
        } else {
            "<unnamed>"
        }
    }

    fn get_app_name(&self) -> &str {
        if let Some(app_id) = &self.app_id {
            app_id
        } else if let Some(wp_class) = self
            .window_properties
            .as_ref()
            .and_then(|wp| wp.class.as_ref())
        {
            wp_class
        } else {
            "<unknown_app>"
        }
    }

    fn is_scratchpad(&self) -> bool {
        let name = self.get_name();
        name.eq("__i3") || name.eq("__i3_scratch")
    }

    fn nodes_of_type(&self, t: Type) -> Vec<&s::Node> {
        self.iter().filter(|n| n.get_type() == t).collect()
    }

    fn is_floating(&self) -> bool {
        self.node_type == s::NodeType::FloatingCon
    }
}

/// Extra properties gathered by swayrd for windows and workspaces.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExtraProps {
    /// Milliseconds since UNIX epoch.
    pub last_focus_time: u128,
    pub last_focus_time_for_next_prev_seq: u128,
}

pub struct Tree<'a> {
    root: &'a s::Node,
    id_node: HashMap<i64, &'a s::Node>,
    id_parent: HashMap<i64, i64>,
    extra_props: &'a HashMap<i64, ExtraProps>,
}

pub struct DisplayNode<'a> {
    pub node: &'a s::Node,
    pub tree: &'a Tree<'a>,
}

impl<'a> Tree<'a> {
    fn get_node_by_id(&self, id: i64) -> &&s::Node {
        self.id_node
            .get(&id)
            .unwrap_or_else(|| panic!("No node with id {}", id))
    }

    fn get_parent_node(&self, id: i64) -> Option<&&s::Node> {
        self.id_parent.get(&id).map(|pid| self.get_node_by_id(*pid))
    }

    pub fn get_workspace_node(&self, id: i64) -> Option<&&s::Node> {
        let n = self.get_node_by_id(id);
        if n.get_type() == Type::Workspace {
            Some(n)
        } else if let Some(pid) = self.id_parent.get(&id) {
            self.get_workspace_node(*pid)
        } else {
            None
        }
    }

    pub fn last_focus_time(&self, id: i64) -> u128 {
        self.extra_props.get(&id).map_or(0, |wp| wp.last_focus_time)
    }

    pub fn last_focus_time_for_next_prev_seq(&self, id: i64) -> u128 {
        self.extra_props
            .get(&id)
            .map_or(0, |wp| wp.last_focus_time_for_next_prev_seq)
    }

    fn sorted_nodes_of_type_1(
        &self,
        node: &'a s::Node,
        t: Type,
    ) -> Vec<&s::Node> {
        let mut v: Vec<&s::Node> = node.nodes_of_type(t);
        v.sort_by(|a, b| {
            if a.urgent && !b.urgent {
                cmp::Ordering::Less
            } else if !a.urgent && b.urgent {
                cmp::Ordering::Greater
            } else {
                let lru_a = self.last_focus_time(a.id);
                let lru_b = self.last_focus_time(b.id);
                lru_a.cmp(&lru_b).reverse()
            }
        });
        v
    }

    fn sorted_nodes_of_type(&self, t: Type) -> Vec<&s::Node> {
        self.sorted_nodes_of_type_1(self.root, t)
    }

    fn as_display_nodes(&self, v: Vec<&'a s::Node>) -> Vec<DisplayNode> {
        v.iter()
            .map(|n| DisplayNode {
                node: n,
                tree: self,
            })
            .collect()
    }

    pub fn get_current_workspace(&self) -> &s::Node {
        self.root
            .iter()
            .find(|n| n.get_type() == Type::Workspace && n.is_current())
            .expect("No current Workspace")
    }

    pub fn get_workspaces(&self) -> Vec<DisplayNode> {
        let mut v = self.sorted_nodes_of_type(Type::Workspace);
        v.rotate_left(1);
        self.as_display_nodes(v)
    }

    pub fn get_windows(&self) -> Vec<DisplayNode> {
        let mut v = self.sorted_nodes_of_type(Type::Window);
        v.rotate_left(1);
        self.as_display_nodes(v)
    }

    pub fn get_workspaces_and_windows(&self) -> Vec<DisplayNode> {
        let workspaces = self.sorted_nodes_of_type(Type::Workspace);
        let mut first = true;
        let mut v = vec![];
        for ws in workspaces {
            v.push(ws);
            let mut wins = self.sorted_nodes_of_type_1(ws, Type::Window);
            if first {
                wins.rotate_left(1);
                first = false;
            }
            v.append(&mut wins);
        }

        // Rotate until we have the second recently used workspace in front.
        v.rotate_left(1);
        while v[0].get_type() != Type::Workspace {
            v.rotate_left(1);
        }

        self.as_display_nodes(v)
    }

    pub fn is_child_of_tiled_container(&self, id: i64) -> bool {
        match self.get_parent_node(id) {
            Some(n) => {
                n.layout == s::NodeLayout::SplitH
                    || n.layout == s::NodeLayout::SplitV
            }
            None => false,
        }
    }

    pub fn is_child_of_tabbed_or_stacked_container(&self, id: i64) -> bool {
        match self.get_parent_node(id) {
            Some(n) => {
                n.layout == s::NodeLayout::Tabbed
                    || n.layout == s::NodeLayout::Stacked
            }
            None => false,
        }
    }
}

fn init_id_parent<'a>(
    n: &'a s::Node,
    parent: Option<&'a s::Node>,
    id_node: &mut HashMap<i64, &'a s::Node>,
    id_parent: &mut HashMap<i64, i64>,
) {
    id_node.insert(n.id, n);

    if let Some(p) = parent {
        id_parent.insert(n.id, p.id);
    }

    for c in &n.nodes {
        init_id_parent(c, Some(n), id_node, id_parent);
    }
    for c in &n.floating_nodes {
        init_id_parent(c, Some(n), id_node, id_parent);
    }
}

pub fn get_tree<'a>(
    root: &'a s::Node,
    extra_props: &'a HashMap<i64, ExtraProps>,
) -> Tree<'a> {
    let mut id_node: HashMap<i64, &s::Node> = HashMap::new();
    let mut id_parent: HashMap<i64, i64> = HashMap::new();
    init_id_parent(root, None, &mut id_node, &mut id_parent);

    Tree {
        root,
        id_node,
        id_parent,
        extra_props,
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

impl DisplayFormat for DisplayNode<'_> {
    fn format_for_display(&self, cfg: &config::Config) -> String {
        match self.node.get_type() {
            Type::Root => String::from("Cannot format Root"),
            Type::Output => String::from("Cannot format Output"),
            Type::Workspace => {
                let fmt = cfg.get_format_workspace_format();
                let html_escape = cfg.get_format_html_escape();

                fmt.replace("{id}", format!("{}", self.node.id).as_str())
                    .replace(
                        "{name}",
                        &maybe_html_escape(html_escape, self.node.get_name()),
                    )
            }
            Type::Container => {
                todo!("DisplayFormat for Container not yet implemented")
            }
            Type::Window => {
                let window_format = cfg.get_format_window_format();
                let urgency_start = cfg.get_format_urgency_start();
                let urgency_end = cfg.get_format_urgency_end();
                let html_escape = cfg.get_format_html_escape();
                let icon_dirs = cfg.get_format_icon_dirs();
                // fallback_icon has no default value.
                let fallback_icon: Option<Box<std::path::Path>> =
                    cfg.get_format_fallback_icon().as_ref().map(|i| {
                        std::path::Path::new(i).to_owned().into_boxed_path()
                    });

                // Some apps report, e.g., Gimp-2.10 but the icon is still named
                // gimp.png.
                let app_name_no_version = APP_NAME_AND_VERSION_RX
                    .replace(self.node.get_app_name(), "$1");

                window_format
                    .replace("{id}", format!("{}", self.node.id).as_str())
                    .replace(
                        "{urgency_start}",
                        if self.node.urgent {
                            urgency_start.as_str()
                        } else {
                            ""
                        },
                    )
                    .replace(
                        "{urgency_end}",
                        if self.node.urgent {
                            urgency_end.as_str()
                        } else {
                            ""
                        },
                    )
                    .replace(
                        "{app_name}",
                        &maybe_html_escape(
                            html_escape,
                            self.node.get_app_name(),
                        ),
                    )
                    .replace(
                        "{workspace_name}",
                        &maybe_html_escape(
                            html_escape,
                            self.tree
                                .get_workspace_node(self.node.id)
                                .map_or("<no_workspace>", |w| w.get_name()),
                        ),
                    )
                    .replace(
                        "{marks}",
                        &maybe_html_escape(
                            html_escape,
                            &self.node.marks.join(", "),
                        ),
                    )
                    .replace(
                        "{app_icon}",
                        util::get_icon(self.node.get_app_name(), &icon_dirs)
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
                        &maybe_html_escape(html_escape, self.node.get_name()),
                    )
            }
        }
    }
}
