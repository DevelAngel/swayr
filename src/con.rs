use crate::ipc;

#[allow(dead_code)]
pub struct Con<'a> {
    name: &'a str,
    id: ipc::Id,
    app_id: Option<&'a str>,
}

impl<'a> std::fmt::Display for Con<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} — {}", self.app_id.unwrap_or(""), self.name)
    }
}

/// Gets all cons (aka, application windows) of the tree.
pub fn get_cons<'a>(tree: &'a ipc::Node) -> Vec<Con<'a>> {
    let mut v = vec![];
    for n in tree.iter() {
        if n.r#type == ipc::NodeType::Con || n.r#type == ipc::NodeType::FloatingCon {
            v.push(Con {
                name: &n
                    .name
                    .as_ref()
                    .expect(format!("Con without name. id = {}", n.id).as_str()),
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
fn test_get_cons() {
    let tree = ipc::get_tree();
    let cons = get_cons(&tree);

    println!("There are {} cons.", cons.len());

    for c in cons {
        println!("  {}", c);
    }
}
