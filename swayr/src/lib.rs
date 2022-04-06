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

//! **Swayr** is a LRU window-switcher and more for the sway window manager.
//! It consists of a demon, and a client.  The demon `swayrd` records
//! window/workspace creations, deletions, and focus changes using sway's JSON
//! IPC interface.  The client `swayr` offers subcommands, see `swayr --help`.

pub mod client;
pub mod cmds;
pub mod config;
pub mod demon;
pub mod fmt_replace;
pub mod ipc;
pub mod layout;
pub mod rtfmt;
pub mod tree;
pub mod util;
