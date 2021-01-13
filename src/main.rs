use swayr::con;
use swayr::ipc;

fn main() {
    let root_node = ipc::get_tree();
    for con in con::get_cons(&root_node) {
        println!("{}", con);
    }
    println!("Yes!")
}
