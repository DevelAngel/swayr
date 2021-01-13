extern crate serde;
extern crate serde_json;

use serde::Deserialize;

use std::process as proc;

pub type Id = u32;
pub type Dim = u16;
pub type Pid = u16;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Rect {
    pub x: Dim,
    pub y: Dim,
    pub width: Dim,
    pub height: Dim,
}

// TODO: Maybe there are more?
#[derive(Deserialize)]
pub enum Border {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "pixel")]
    Pixel,
    #[serde(rename = "csd")]
    Csd,
}

// TODO: Maybe there are more?
#[derive(Deserialize)]
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

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub enum ShellType {
    #[serde(rename = "xdg_shell")]
    XdgShell,
    #[serde(rename = "xwayland")]
    XWayland,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct WindowProperties {
    pub class: Option<String>,
    pub instance: Option<String>,
    pub title: Option<String>,
    //pub window_type: Option<WindowType>
    //pub transient_for: DONTKNOW,
}

#[derive(Deserialize)]
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
    pub fn iter(&self) -> PreOrderNodeIter {
        PreOrderNodeIter::new(self)
    }
}

pub struct PreOrderNodeIter<'a> {
    stack: Vec<&'a Node>,
}

impl<'a> PreOrderNodeIter<'a> {
    fn new(node: &'a Node) -> PreOrderNodeIter {
        PreOrderNodeIter { stack: vec![node] }
    }
}

impl<'a> Iterator for PreOrderNodeIter<'a> {
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

pub fn get_tree() -> Node {
    let output = proc::Command::new("swaymsg")
        .arg("-t")
        .arg("get_tree")
        .output()
        .expect("Error running swaymsg!");
    let result = serde_json::from_str(
        String::from_utf8(output.stdout)
            .expect("Wrong string data!")
            .as_str(),
    );

    match result {
        Ok(node) => node,
        Err(e) => {
            eprintln!("Error: {}", e);
            panic!()
        }
    }
}

#[test]
fn test_get_tree() {
    let tree = get_tree();

    println!("Those IDs are in get_tree():");
    for n in tree.iter() {
        println!("  id: {}, type: {:?}", n.id, n.r#type);
    }
}
