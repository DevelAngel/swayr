use crate::ipc;
use std::cmp;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Window<'a> {
    pub node: &'a ipc::Node,
    win_props: Option<ipc::WindowProps>,
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

impl PartialEq for Window<'_> {
    fn eq(&self, other: &Window) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Eq for Window<'_> {}

impl Ord for Window<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        if self == other {
            cmp::Ordering::Equal
        } else if self.node.urgent && !other.node.urgent {
            cmp::Ordering::Less
        } else if !self.node.urgent && other.node.urgent {
            std::cmp::Ordering::Greater
        } else if self.node.focused && !other.node.focused {
            std::cmp::Ordering::Greater
        } else if !self.node.focused && other.node.focused {
            std::cmp::Ordering::Less
        } else {
            let lru_a = self.win_props.as_ref().map_or(0, |wp| wp.last_focus_time);
            let lru_b = other.win_props.as_ref().map_or(0, |wp| wp.last_focus_time);
            lru_a.cmp(&lru_b).reverse()
        }
    }
}

impl PartialOrd for Window<'_> {
    fn partial_cmp(&self, other: &Window) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> std::fmt::Display for Window<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "<span font_weight=\"bold\" {}>{}</span> — {} [{}]",
            if self.node.urgent {
                " background=\"darkred\" foreground=\"white\""
            } else {
                ""
            },
            self.get_app_name(),
            self.get_title(),
            self.get_id()
        )
    }
}

/// Gets all application windows of the tree.
pub fn get_windows(
    tree: &ipc::Node,
    mut win_props: HashMap<ipc::Id, ipc::WindowProps>,
) -> Vec<Window> {
    let mut v = vec![];
    for n in tree.iter() {
        if n.name.is_some()
            && (n.r#type == ipc::NodeType::Con || n.r#type == ipc::NodeType::FloatingCon)
        {
            v.push(Window {
                node: &n,
                win_props: win_props.remove(&n.id),
            })
        }
    }
    v
}
