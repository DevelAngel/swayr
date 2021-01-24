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
    /// Focus the selected window
    SwitchWindow,
    /// Quit the selected window
    QuitWindow,
    /// Switch to the selected workspace
    SwitchWorkspace,
    /// Switch to the selected workspace or focus the selected window
    SwitchWorkspaceOrWindow,
    /// Quit all windows of selected workspace or the selected window
    QuitWorkspaceOrWindow,
    /// Select and execute a swaymsg command
    ExecuteSwaymsgCommand,
}

fn main() {
    let opts: Opts = Opts::parse();
    match opts.command {
        SwayrCommand::SwitchWindow => client::switch_window(),
        SwayrCommand::QuitWindow => client::quit_window(),
        SwayrCommand::SwitchWorkspace => client::switch_workspace(),
        SwayrCommand::SwitchWorkspaceOrWindow => {
            client::switch_workspace_or_window()
        }
        SwayrCommand::QuitWorkspaceOrWindow => {
            client::quit_workspace_or_window()
        }
        SwayrCommand::ExecuteSwaymsgCommand => client::exec_swaymsg_command(),
    }
}
