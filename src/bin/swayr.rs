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
    /// Switch window with display order urgent first, then LRU order, focused last
    SwitchWindow,
    /// Quit a window with display order focused first, then reverse-LRU order, urgent last
    QuitWindow,
    /// Switch workspace with LRU display order
    SwitchWorkspace,
    /// Select and execute a swaymsg command
    ExecuteSwaymsgCommand,
}

fn main() {
    let opts: Opts = Opts::parse();
    match opts.command {
        SwayrCommand::SwitchWindow => client::switch_window(),
        SwayrCommand::QuitWindow => client::quit_window(),
        SwayrCommand::SwitchWorkspace => client::switch_workspace(),
        SwayrCommand::ExecuteSwaymsgCommand => client::exec_swaymsg_command(),
    }
}
