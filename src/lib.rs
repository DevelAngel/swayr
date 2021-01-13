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
    x: Dim,
    y: Dim,
    width: Dim,
    height: Dim,
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

#[derive(Deserialize, PartialEq)]
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
pub struct Node {
    id: Id,
    name: Option<String>,
    rect: Rect,
    focused: bool,
    focus: Vec<Id>,
    border: Border,
    current_border_width: Dim,
    layout: Layout,
    orientation: Orientation,
    percent: Option<f64>,
    window_rect: Rect,
    deco_rect: Rect,
    geometry: Rect,
    window: Option<Id>,
    urgent: bool,
    marks: Vec<String>,
    fullscreen_mode: u8, // TODO: actually, it's 0 or 1, i.e., a bool
    nodes: Vec<Node>,
    floating_nodes: Vec<Node>,
    sticky: bool,
    r#type: NodeType,
    app_id: Option<String>,
    visible: Option<bool>,
    max_render_time: Option<i32>,
    pid: Option<Pid>,
    shell: Option<ShellType>,
}

impl Node {
    fn iter(&self) -> PreOrderNodeIter {
        PreOrderNodeIter::new(self)
    }
}

struct PreOrderNodeIter<'a> {
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
            for n in &node.nodes {
                self.stack.push(&n);
            }
            for n in &node.floating_nodes {
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

#[allow(dead_code)]
pub struct Con<'a> {
    name: &'a str,
    id: Id,
    app_id: Option<&'a str>,
}

impl<'a> std::fmt::Display for Con<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} — {}", self.app_id.unwrap_or(""), self.name)
    }
}

/// Gets all cons (aka, application windows) of the tree.
pub fn get_cons<'a>(tree: &'a Node) -> Vec<Con<'a>> {
    let mut v = vec![];
    for n in tree.iter() {
        if n.r#type == NodeType::Con || n.r#type == NodeType::FloatingCon {
            v.push(Con {
                name: &n
                    .name
                    .as_ref()
                    .expect(format!("Con without name. id = {}", n.id).as_str()),
                id: n.id,
                app_id: match &n.app_id {
                    Some(s) => Some(s.as_ref()),
                    None => None,
                },
            })
        }
    }
    v
}

#[test]
fn test_get_cons() {
    let tree = get_tree();

    println!("Those IDs are in get_tree():");
    for n in tree.iter() {
        println!("  id: {}", n.id);
    }

    let cons = get_cons(&tree);

    println!("There are {} cons.", cons.len());

    for c in cons {
        println!("{}", c);
    }
}
