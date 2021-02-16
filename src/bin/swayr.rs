//! The `swayr` binary.

#[macro_use]
extern crate clap;
use clap::Clap;
use swayr::client;
use swayr::ipc;

/// Windows are sorted urgent first, then windows in LRU order, focused window
/// last.  Licensed under the GPLv3 (or later).
#[derive(Clap)]
#[clap(
    name = "swayr -- a window switcher for sway",
    version = crate_version!(),
    author = "Tassilo Horn <tsdh@gnu.org>"
)]
struct Opts {
    #[clap(subcommand)]
    command: ipc::SwayrCommand,
}

fn main() {
    let opts: Opts = Opts::parse();
    if let Err(err) = client::send_swayr_cmd(opts.command) {
        eprintln!("Could not send command: {}", err);
    }
}
