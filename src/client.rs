extern crate serde_json;

use crate::ipc;
use crate::util;
use std::io::Write;
use std::os::unix::net::UnixStream;

pub fn send_swayr_cmd(
    cmd: ipc::SwayrCommand,
) -> std::result::Result<(), std::io::Error> {
    let mut sock = UnixStream::connect(util::get_swayr_socket_path())?;
    sock.write_all(serde_json::to_string(&cmd).unwrap().as_bytes())
}
