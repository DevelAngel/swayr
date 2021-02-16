// TODO: Possibly just include README.md when this feature is in the release
// channel.
//
// #![doc(include = "../README.md")]

//! **Swayr** is a wofi-based LRU window-switcher and more for the sway window
//! manager.  It consists of a demon, and a client.  The demon `swayrd` records
//! window/workspace creations, deletions, and focus changes using sway's JSON
//! IPC interface.  The client `swayr` offers subcommands, see `swayr --help`.

pub mod client;
pub mod cmds;
pub mod con;
pub mod demon;
pub mod ipc;
pub mod util;
