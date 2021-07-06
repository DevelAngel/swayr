// Copyright (C) 2021  Tassilo Horn <tsdh@gnu.org>
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::cmds;
use crate::util;
use std::io::Write;
use std::os::unix::net::UnixStream;

pub fn send_swayr_cmd(
    cmd: cmds::SwayrCommand,
) -> std::result::Result<(), std::io::Error> {
    let mut sock = UnixStream::connect(util::get_swayr_socket_path())?;
    sock.write_all(serde_json::to_string(&cmd).unwrap().as_bytes())
}
