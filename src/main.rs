use std::fs;
use std::io;

use swayr::Node;

fn main() {
    let root_node = swayr::get_tree();
    for con in swayr::get_cons(&root_node) {
        println!("{}\n", con);
    }
    println!("Yes!")
}
