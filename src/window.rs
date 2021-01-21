use crate::ipc;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Window<'a> {
    pub node: &'a ipc::Node,
}

impl Window<'_> {
    pub fn get_id(&self) -> ipc::Id {
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
}

impl<'a> std::fmt::Display for Window<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "<b>{}</b> — {} [{}]",
            self.get_app_name(),
            self.get_title(),
            self.get_id()
        )
    }
}

/// Gets all application windows of the tree.
pub fn get_windows(tree: &ipc::Node) -> Vec<Window> {
    let mut v = vec![];
    for n in tree.iter() {
        if n.name.is_some()
            && (n.r#type == ipc::NodeType::Con || n.r#type == ipc::NodeType::FloatingCon)
        {
            v.push(Window { node: &n })
        }
    }
    v
}

/// Sorts windows so that urgent windows come first, the currently focused
/// window comes last, and otherwise windows are sorted in last-recently-used
/// order.
pub fn sort_windows(windows: &mut Vec<Window>, win_props: HashMap<ipc::Id, ipc::WindowProps>) {
    windows.sort_unstable_by(|a, b| {
        if a.node.urgent && !b.node.urgent {
            std::cmp::Ordering::Less
        } else if !a.node.urgent && b.node.urgent {
            std::cmp::Ordering::Greater
        } else if a.node.focused && !b.node.focused {
            std::cmp::Ordering::Greater
        } else if !a.node.focused && b.node.focused {
            std::cmp::Ordering::Less
        } else {
            let lru_a = win_props
                .get(&a.node.id)
                .map(|p| p.last_focus_time)
                .unwrap_or(0);
            let lru_b = win_props
                .get(&b.node.id)
                .map(|p| p.last_focus_time)
                .unwrap_or(0);
            lru_a.cmp(&lru_b).reverse()
        }
    });
}
