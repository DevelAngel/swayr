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

//! Basic sway IPC.

use std::{cell::RefCell, sync::Mutex};

use once_cell::sync::Lazy;
use swayipc as s;

static SWAY_IPC_CONNECTION: Lazy<Mutex<RefCell<s::Connection>>> =
    Lazy::new(|| {
        Mutex::new(RefCell::new(
            s::Connection::new().expect("Could not open sway IPC connection."),
        ))
    });

pub fn get_root_node(include_scratchpad: bool) -> s::Node {
    let mut root = match SWAY_IPC_CONNECTION.lock() {
        Ok(cell) => cell.borrow_mut().get_tree().expect("Couldn't get tree"),
        Err(err) => panic!("{}", err),
    };

    if !include_scratchpad {
        root.nodes.retain(|o| !o.is_scratchpad());
    }
    root
}

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
    fn iter(&self) -> NodeIter;
    fn get_type(&self) -> Type;
    fn get_app_name(&self) -> &str;
    fn nodes_of_type(&self, t: Type) -> Vec<&s::Node>;
    fn get_name(&self) -> &str;
    fn is_scratchpad(&self) -> bool;
    fn is_floating(&self) -> bool;
    fn is_current(&self) -> bool;
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
                    && self.app_id.is_none()
                    && self.pid.is_none()
                    && self.shell.is_none()
                    && self.window_properties.is_none()
                    && self.layout != s::NodeLayout::None
                {
                    Type::Container
                } else if (self.node_type == s::NodeType::Con
                    || self.node_type == s::NodeType::FloatingCon)
                    // Apparently there can be windows without app_id, name,
                    // and window_properties.class, e.g., dolphin-emu-nogui.
                    && self.pid.is_some()
                // FIXME: While technically correct, old sway versions (up to
                // at least sway-1.4) don't expose shell in IPC.  So comment in
                // again when all major distros have a recent enough sway
                // package.
                //&& self.shell.is_some()
                {
                    Type::Window
                } else {
                    panic!(
                        "Don't know type of node with id {} and node_type {:?}\n{:?}",
                        self.id, self.node_type, self
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

    fn is_current(&self) -> bool {
        self.iter().any(|n| n.focused)
    }
}
