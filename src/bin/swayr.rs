#[macro_use]
extern crate clap;
use clap::Clap;
use swayr::client;

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
    command: SwayrCommand,
}

#[derive(Clap)]
enum SwayrCommand {
    /// Switch window using wofi (urgent first, then LRU order, focused last)
    SwitchWindow,
}

fn main() {
    let opts: Opts = Opts::parse();
    match opts.command {
        SwayrCommand::SwitchWindow => client::switch_window(),
    }
}
