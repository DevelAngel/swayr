use crate::ipc;
use crate::util;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub struct Window<'a> {
    node: &'a ipc::Node,
    workspace: &'a ipc::Node,
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
        } else if self.node.urgent && !other.node.urgent
            || !self.node.focused && other.node.focused
        {
            cmp::Ordering::Less
        } else if !self.node.urgent && other.node.urgent
            || self.node.focused && !other.node.focused
        {
            std::cmp::Ordering::Greater
        } else {
            let lru_a =
                self.win_props.as_ref().map_or(0, |wp| wp.last_focus_time);
            let lru_b =
                other.win_props.as_ref().map_or(0, |wp| wp.last_focus_time);
            lru_a.cmp(&lru_b).reverse()
        }
    }
}

impl PartialOrd for Window<'_> {
    fn partial_cmp(&self, other: &Window) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> fmt::Display for Window<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "<span font_weight=\"bold\" {}>“{}”</span>   \
             <i>{}</i>   \
             on workspace <b>{}</b>   \
             <span alpha=\"20000\">id {}</span>", // Almost hide ID!
            if self.node.urgent {
                " background=\"darkred\" foreground=\"white\""
            } else {
                ""
            },
            self.get_title(),
            self.get_app_name(),
            self.workspace.name.as_ref().unwrap(),
            self.get_id()
        )
    }
}

fn build_windows(
    tree: &ipc::Node,
    mut win_props: HashMap<ipc::Id, ipc::WindowProps>,
) -> Vec<Window> {
    let mut v = vec![];
    for workspace in tree.workspaces() {
        for n in workspace.windows() {
            v.push(Window {
                node: &n,
                win_props: win_props.remove(&n.id),
                workspace: &workspace,
            })
        }
    }
    v
}

fn get_window_props(
) -> Result<HashMap<ipc::Id, ipc::WindowProps>, serde_json::Error> {
    if let Ok(sock) = UnixStream::connect(util::get_swayr_socket_path()) {
        serde_json::from_reader(sock)
    } else {
        panic!("Could not connect to socket!")
    }
}

/// Gets all application windows of the tree.
pub fn get_windows(root_node: &ipc::Node) -> Vec<Window> {
    let win_props = match get_window_props() {
        Ok(win_props) => Some(win_props),
        Err(e) => {
            eprintln!("Got no win_props: {:?}", e);
            None
        }
    };

    build_windows(&root_node, win_props.unwrap_or_default())
}

pub fn select_window<'a>(
    prompt: &'a str,
    windows: &'a [Window],
) -> Option<&'a Window<'a>> {
    util::wofi_select(prompt, windows)
}
