extern crate serde;
extern crate serde_json;
extern crate users;

use serde::{Deserialize, Serialize};

pub type Id = u32;
pub type Dim = u16;
pub type Pos = i16; // Position can be off-screen, so i16 instead of u16.
pub type Pid = u32;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Rect {
    pub x: Pos,
    pub y: Pos,
    pub width: Dim,
    pub height: Dim,
}

// TODO: Maybe there are more?
#[derive(Deserialize, Debug)]
pub enum Border {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "pixel")]
    Pixel,
    #[serde(rename = "csd")]
    Csd,
}

// TODO: Maybe there are more?
#[derive(Deserialize, Debug)]
pub enum Layout {
    #[serde(rename = "splith")]
    SplitH,
    #[serde(rename = "splitv")]
    SplitV,
    #[serde(rename = "tabbed")]
    Tabbed,
    #[serde(rename = "stacked")]
    Stacked,
    #[serde(rename = "output")]
    Output,
    #[serde(rename = "none")]
    None,
}

#[derive(Deserialize, Debug)]
pub enum Orientation {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
    #[serde(rename = "none")]
    None,
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum NodeType {
    #[serde(rename = "root")]
    Root,
    #[serde(rename = "workspace")]
    Workspace,
    #[serde(rename = "output")]
    Output,
    #[serde(rename = "con")]
    Con,
    #[serde(rename = "floating_con")]
    FloatingCon,
}

#[derive(Deserialize, Debug)]
pub enum ShellType {
    #[serde(rename = "xdg_shell")]
    XdgShell,
    #[serde(rename = "xwayland")]
    XWayland,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WindowProperties {
    pub class: Option<String>,
    pub instance: Option<String>,
    pub title: Option<String>,
    //pub window_type: Option<WindowType>
    //pub transient_for: DONTKNOW,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Node {
    pub id: Id,
    pub name: Option<String>,
    pub rect: Rect,
    pub focused: bool,
    pub focus: Vec<Id>,
    pub border: Border,
    pub current_border_width: Dim,
    pub layout: Layout,
    pub orientation: Orientation,
    pub percent: Option<f64>,
    pub window_rect: Rect,
    pub deco_rect: Rect,
    pub geometry: Rect,
    pub window: Option<Id>,
    pub urgent: bool,
    pub marks: Vec<String>,
    pub fullscreen_mode: u8, // TODO: actually, it's 0 or 1, i.e., a bool
    pub nodes: Vec<Node>,
    pub floating_nodes: Vec<Node>,
    pub sticky: bool,
    pub r#type: NodeType,
    pub app_id: Option<String>,
    pub visible: Option<bool>,
    pub max_render_time: Option<i32>,
    pub pid: Option<Pid>,
    pub shell: Option<ShellType>,
    pub window_properties: Option<WindowProperties>,
}

impl Node {
    pub fn iter(&self) -> NodeIter {
        NodeIter::new(self)
    }
}

pub struct NodeIter<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> NodeIter<'a> {
    fn new(node: &'a Node) -> NodeIter {
        NodeIter { stack: vec![node] }
    }
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = &'a Node;

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

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub enum WindowEventType {
    #[serde(rename = "new")]
    New,
    #[serde(rename = "close")]
    Close,
    #[serde(rename = "focus")]
    Focus,
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "fullscreen_mode")]
    FullscreenMode,
    #[serde(rename = "move")]
    Move,
    #[serde(rename = "floating")]
    Floating,
    #[serde(rename = "urgent")]
    Urgent,
    #[serde(rename = "mark")]
    Mark,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WindowEvent {
    pub change: WindowEventType,
    pub container: Node,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WindowProps {
    /// Milliseconds since UNIX epoch.
    pub last_focus_time: u128,
}
