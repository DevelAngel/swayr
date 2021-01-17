use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use swayr::ipc;
use swayr::window;

fn main() {
    println!("sway here!");
    let root_node = ipc::get_tree();
    for win in window::get_windows(&root_node) {
        println!("  {}", win);
    }

    if let Ok(sock) = UnixStream::connect(ipc::SWAYR_SOCKET_PATH) {
        let win_props: Result<HashMap<ipc::Id, ipc::WindowProps>, serde_json::Error> =
            serde_json::from_reader(sock);
        println!("Here are the window properties:\n{:#?}", win_props)
    } else {
        panic!("Could not connect to socket!")
    }
}
