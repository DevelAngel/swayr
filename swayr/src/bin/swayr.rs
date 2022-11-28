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

//! The `swayr` binary.

use clap::Parser;

#[derive(clap::Parser)]
#[clap(about, version, author)]
struct Opts {
    #[clap(subcommand)]
    command: swayr::cmds::SwayrCommand,
}

fn main() -> Result<(), String> {
    let opts: Opts = Opts::parse();
    match swayr::client::send_swayr_cmd(opts.command) {
        Ok(val) => {
            println!("{}", val);
            Ok(())
        }
        Err(err) => Err(err),
    }
}
