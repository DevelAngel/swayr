use crate::ipc;

#[allow(dead_code)]
pub struct Window<'a> {
    name: &'a str,
    id: ipc::Id,
    app_id: Option<&'a str>,
}

impl<'a> std::fmt::Display for Window<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} — {}", self.app_id.unwrap_or(""), self.name)
    }
}

/// Gets all application windows of the tree.
pub fn get_windows(tree: &ipc::Node) -> Vec<Window> {
    let mut v = vec![];
    for n in tree.iter() {
        if n.name.is_some()
            && (n.r#type == ipc::NodeType::Con || n.r#type == ipc::NodeType::FloatingCon)
        {
            v.push(Window {
                name: &n
                    .name
                    .as_ref()
                    .unwrap_or_else(|| panic!("Con without name. id = {}", n.id)),
                id: n.id,
                app_id: match &n.app_id {
                    Some(s) => Some(s.as_ref()),
                    // TODO: Use n.window_properties.class instead!
                    None => None,
                },
            })
        }
    }
    v
}

#[test]
fn test_get_windows() {
    let tree = ipc::get_tree();
    let cons = get_windows(&tree);

    println!("There are {} cons.", cons.len());

    for c in cons {
        println!("  {}", c);
    }
}
