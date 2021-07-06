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

//! The `swayr` binary.

use clap::{crate_version, Clap};

/// Windows are sorted urgent first, then windows in LRU order, focused window
/// last.  Licensed under the GPLv3 (or later).
#[derive(Clap)]
#[clap(
    name = "swayr -- a window switcher (and more) for sway",
    version = crate_version!(),
    author = "Tassilo Horn <tsdh@gnu.org>"
)]
struct Opts {
    #[clap(subcommand)]
    command: swayr::cmds::SwayrCommand,
}

fn main() {
    let opts: Opts = Opts::parse();
    if let Err(err) = swayr::client::send_swayr_cmd(opts.command) {
        eprintln!("Could not send command: {}", err);
    }
}
