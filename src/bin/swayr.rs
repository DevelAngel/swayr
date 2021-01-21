extern crate clap;
use clap::Clap;
use swayr::client;

/// Windows are sorted so that urgent windows come first, then windows in
/// last-recently-used order, and the focused window last.
#[derive(Clap)]
#[clap(
    name = "swayr -- a window switcher for sway",
    version = option_env!("CARGO_PKG_VERSION").unwrap_or("<unknown version>"),
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
