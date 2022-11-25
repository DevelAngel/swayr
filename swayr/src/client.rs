// Copyright (C) 2021-2022  Tassilo Horn <tsdh@gnu.org>
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
use crate::cmds::SwayrCmdRetVal;
use crate::util;
use std::os::unix::net::UnixStream;

pub fn send_swayr_cmd(
    cmd: cmds::SwayrCommand,
) -> Result<SwayrCmdRetVal, String> {
    let stream = UnixStream::connect(util::get_swayr_socket_path())
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(&stream, &cmd).map_err(|e| e.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    let result =
        serde_json::from_reader::<_, Result<SwayrCmdRetVal, String>>(&stream)
            .expect("Could not read response from swayrd");
    result
}
