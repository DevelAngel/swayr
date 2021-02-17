//! Convenience data structures built from the IPC structs.

use crate::ipc;
use crate::util;
use ipc::NodeMethods;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use swayipc::reply as r;

#[derive(Debug)]
pub struct Window<'a> {
    node: &'a r::Node,
    workspace: &'a r::Node,
    extra_props: Option<ipc::ExtraProps>,
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

fn build_windows<'a>(
    root: &'a r::Node,
    include_scratchpad_windows: bool,
    extra_props: Option<&HashMap<i64, ipc::ExtraProps>>,
) -> Vec<Window<'a>> {
    let mut v = vec![];
    for workspace in root.workspaces() {
        if !include_scratchpad_windows && workspace.is_scratchpad() {
            continue;
        }

        for n in workspace.windows() {
            v.push(Window {
                node: &n,
                extra_props: extra_props.and_then(|m| m.get(&n.id).cloned()),
                workspace: &workspace,
            })
        }
    }
    v
}

fn build_workspaces<'a>(
    root: &'a r::Node,
    include_scratchpad: bool,
    extra_props: Option<&HashMap<i64, ipc::ExtraProps>>,
) -> Vec<Workspace<'a>> {
    let mut v = vec![];
    for workspace in root.workspaces() {
        if !include_scratchpad && workspace.is_scratchpad() {
            continue;
        }

        let mut wins: Vec<Window> = workspace
            .windows()
            .iter()
            .map(|w| Window {
                node: &w,
                extra_props: extra_props.and_then(|m| m.get(&w.id).cloned()),
                workspace: &workspace,
            })
            .collect();
        wins.sort();
        v.push(Workspace {
            node: &workspace,
            extra_props: extra_props
                .and_then(|m| m.get(&workspace.id).cloned()),
            windows: wins,
        })
    }
    v.sort();
    v
}

/// Gets all application windows of the tree.
pub fn get_windows<'a>(
    root: &'a r::Node,
    include_scratchpad_windows: bool,
    extra_props: Option<&HashMap<i64, ipc::ExtraProps>>,
) -> Vec<Window<'a>> {
    let extra_props_given = extra_props.is_some();
    let mut wins = build_windows(root, include_scratchpad_windows, extra_props);
    if extra_props_given {
        wins.sort();
    }
    wins
}

/// Gets all application windows of the tree.
pub fn get_workspaces<'a>(
    root: &'a r::Node,
    include_scratchpad: bool,
    extra_props: Option<&HashMap<i64, ipc::ExtraProps>>,
) -> Vec<Workspace<'a>> {
    let mut workspaces =
        build_workspaces(root, include_scratchpad, extra_props);
    workspaces.rotate_left(1);
    workspaces
}

pub fn select_window<'a>(
    prompt: &str,
    windows: &'a [Window],
) -> Option<&'a Window<'a>> {
    util::wofi_select(prompt, windows)
}

pub fn select_workspace<'a>(
    prompt: &str,
    workspaces: &'a [Workspace],
) -> Option<&'a Workspace<'a>> {
    util::wofi_select(prompt, workspaces)
}

pub enum WsOrWin<'a> {
    Ws { ws: &'a Workspace<'a> },
    Win { win: &'a Window<'a> },
}

impl<'a> fmt::Display for WsOrWin<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            WsOrWin::Ws { ws } => ws.fmt(f),
            WsOrWin::Win { win } => match f.write_str("\t") {
                Ok(()) => win.fmt(f),
                Err(e) => Err(e),
            },
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
                v.push(WsOrWin::Win { win: &win });
            }
        }
        v
    }
}

pub fn select_workspace_or_window<'a>(
    prompt: &'a str,
    ws_or_wins: &'a [WsOrWin<'a>],
) -> Option<&'a WsOrWin<'a>> {
    util::wofi_select(prompt, ws_or_wins)
}

pub struct Workspace<'a> {
    node: &'a r::Node,
    extra_props: Option<ipc::ExtraProps>,
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

impl<'a> fmt::Display for Workspace<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "<span font_weight=\"bold\">“Workspace {}”</span>   \
             <span alpha=\"20000\">id {}</span>", // Almost hide ID!
            self.get_name(),
            self.get_id()
        )
    }
}
