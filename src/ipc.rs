//! Extensions of swayipc types and IPC structs.

extern crate serde;
extern crate serde_json;
extern crate swayipc;
extern crate users;

use clap::Clap;
use serde::{Deserialize, Serialize};
use swayipc::reply as r;

/// Immutable Node Iterator
///
/// Iterates nodes in depth-first order, tiled nodes before floating nodes.
pub struct NodeIter<'a> {
    stack: Vec<&'a r::Node>,
}

impl<'a> NodeIter<'a> {
    pub fn new(node: &'a r::Node) -> NodeIter {
        NodeIter { stack: vec![node] }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a r::Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.stack.pop() {
            for n in &node.floating_nodes {
                self.stack.push(&n);
            }
            for n in &node.nodes {
                self.stack.push(&n);
            }
            Some(node)
        } else {
            None
        }
    }
}

/// Extension methods for [`swayipc::reply::Node`].
pub trait NodeMethods {
    /// Returns an iterator for this [`swayipc::reply::Node`] and its childres.
    fn iter(&self) -> NodeIter;

    /// Returns all nodes being application windows.
    fn windows(&self) -> Vec<&r::Node>;

    /// Returns all nodes being workspaces.
    fn workspaces(&self) -> Vec<&r::Node>;
}

impl NodeMethods for r::Node {
    fn iter(&self) -> NodeIter {
        NodeIter::new(self)
    }

    fn windows(&self) -> Vec<&r::Node> {
        self.iter()
            .filter(|n| {
                (n.node_type == r::NodeType::Con
                    || n.node_type == r::NodeType::FloatingCon)
                    && n.name.is_some()
            })
            .collect()
    }

    fn workspaces(&self) -> Vec<&r::Node> {
        self.iter()
            .filter(|n| n.node_type == r::NodeType::Workspace)
            .collect()
    }
}

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

/// Extra properties gathered by swayrd for windows and workspaces.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExtraProps {
    /// Milliseconds since UNIX epoch.
    pub last_focus_time: u128,
}
