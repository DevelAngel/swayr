fn main() {
    let root_node = swayr::get_tree();
    for con in swayr::get_cons(&root_node) {
        println!("{}", con);
    }
    println!("Yes!")
}
