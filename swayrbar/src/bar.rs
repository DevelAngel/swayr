// Copyright (C) 2022  Tassilo Horn <tsdh@gnu.org>
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

//! `swayrbar` lib.

use crate::config;
use crate::module;
use crate::module::{BarModuleFn, RefreshReason};
use env_logger::Env;
use serde_json;
use std::io;
use std::path::Path;
use std::process as p;
use std::process::Stdio;
use std::sync::mpsc::sync_channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::time::Duration;
use std::{sync::Arc, thread};
use swaybar_types as sbt;
use swayipc as si;

#[derive(clap::Parser)]
#[clap(about, version, author)]
pub struct Opts {
    #[clap(
        short = 'c',
        long,
        help = "Path to a config.toml configuration file.
If not specified, the default config ~/.config/swayrbar/config.toml or
/etc/xdg/swayrbar/config.toml is used."
    )]
    config_file: Option<String>,
}

pub fn start(opts: Opts) {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .init();

    let config = match opts.config_file {
        None => config::load_config(),
        Some(config_file) => {
            let path = Path::new(&config_file);
            crate::shared::cfg::load_config_file(path)
        }
    };
    let refresh_interval = config.refresh_interval;
    let mods: Arc<Vec<Box<dyn BarModuleFn>>> = Arc::new(create_modules(config));
    let mods_for_input = mods.clone();

    let (sender, receiver) = sync_channel(16);
    let sender_for_ticker = sender.clone();
    thread::spawn(move || {
        tick_periodically(refresh_interval, sender_for_ticker)
    });

    let sender_for_input = sender.clone();
    thread::spawn(move || handle_input(mods_for_input, sender_for_input));

    let window_mods_active = mods
        .iter()
        .any(|m| m.get_config().name == crate::module::window::NAME);
    if window_mods_active {
        // There's at least one window module, so subscribe to focus events for
        // immediate refreshes.
        thread::spawn(move || handle_sway_events(sender));
    }

    generate_status(&mods, receiver);
}

fn tick_periodically(refresh_interval: u64, sender: SyncSender<RefreshReason>) {
    loop {
        send_refresh_event(&sender, RefreshReason::TimerEvent);
        thread::sleep(Duration::from_millis(refresh_interval));
    }
}

fn create_modules(config: config::Config) -> Vec<Box<dyn BarModuleFn>> {
    let mut mods = vec![];
    for mc in config.modules {
        let m = match mc.name.as_str() {
            "window" => module::window::create(mc),
            "sysinfo" => module::sysinfo::create(mc),
            "battery" => module::battery::create(mc),
            "date" => module::date::create(mc),
            "pactl" => module::pactl::create(mc),
            "nmcli" => module::wifi::create(module::wifi::WifiTool::Nmcli, mc),
            "iwctl" => module::wifi::create(module::wifi::WifiTool::Iwctl, mc),
            unknown => {
                log::warn!("Unknown module name '{}'.  Ignoring...", unknown);
                continue;
            }
        };
        mods.push(m);
    }
    mods
}

fn handle_input(
    mods: Arc<Vec<Box<dyn BarModuleFn>>>,
    sender: SyncSender<RefreshReason>,
) {
    let mut sb = String::new();
    io::stdin()
        .read_line(&mut sb)
        .expect("Could not read from stdin");

    if "[\n" != sb {
        log::error!("Expected [\\n but got {}", sb);
        log::error!("Sorry, input events won't work is this session.");
        return;
    }

    loop {
        let mut buf = String::new();
        if let Err(err) = io::stdin().read_line(&mut buf) {
            log::error!("Error while reading from stdin: {}", err);
            log::error!("Skipping this input line...");
            continue;
        }

        let click = match serde_json::from_str::<sbt::Click>(
            buf.strip_prefix(',').unwrap_or(&buf),
        ) {
            Ok(click) => click,
            Err(err) => {
                log::error!("Error while parsing str to Click: {}", err);
                log::error!("The string was '{}'.", buf);
                log::error!("Skipping this input line...");
                continue;
            }
        };
        log::debug!("Received click: {:?}", click);
        if let Some(event) = handle_click(click, mods.clone()) {
            send_refresh_event(&sender, event);
        }
    }
}

fn send_refresh_event(
    sender: &SyncSender<RefreshReason>,
    event: RefreshReason,
) {
    log::log!(
        if matches!(event, RefreshReason::TimerEvent) {
            log::Level::Trace
        } else {
            log::Level::Debug
        },
        "Sending refresh event {:?}",
        event
    );

    if let Err(err) = sender.send(event) {
        log::error!("Error at send: {}", err);
    }
}

fn handle_click(
    click: sbt::Click,
    mods: Arc<Vec<Box<dyn BarModuleFn>>>,
) -> Option<RefreshReason> {
    let name = click.name?;
    let instance = click.instance?;
    let button_str = format!("{:?}", click.button);
    for m in mods.iter() {
        if let Some(on_click) = m.get_on_click_map(&name, &instance) {
            if let Some(cmd) = on_click.get(&button_str) {
                let cmd = m.subst_cmd_args(cmd);
                execute_command(&cmd);
                let cfg = m.get_config();
                // No refresh for click events for window modules because the
                // refresh will be triggered by a sway event anyhow.
                //
                // TODO: That's too much coupling.  The bar module shouldn't do
                // specific stuff for certain modules.
                if cfg.name == module::window::NAME {
                    return None;
                }
                return Some(RefreshReason::ClickEvent {
                    name: cfg.name.clone(),
                    instance: cfg.instance.clone(),
                });
            }
        }
    }

    None
}

fn execute_command(cmd: &[String]) {
    log::debug!("Executing command: {:?}", cmd);
    let child = p::Command::new(&cmd[0])
        .args(&cmd[1..])
        // We must not write to stdout because swaybar interprets that!
        // Redirect command output to /dev/null.
        .stdout(Stdio::null())
        .spawn();
    match child {
        Ok(_child) => {
            // For now, if we could at least start the process, that's good
            // enough.  We could wait for it and check its exit state in
            // another thread and log if anything went wrong to give meaningful
            // log output.  But that's not implemented yet.
        }
        Err(err) => {
            log::error!("Error running shell command '{}':", cmd.join(" "));
            log::error!("{}", err);
        }
    }
}

fn sway_subscribe() -> si::Fallible<si::EventStream> {
    si::Connection::new()?.subscribe([
        si::EventType::Window,
        si::EventType::Shutdown,
        si::EventType::Workspace,
    ])
}

fn handle_sway_events(sender: SyncSender<RefreshReason>) {
    let mut resets = 0;
    let max_resets = 10;

    'reset: loop {
        if resets >= max_resets {
            break;
        }
        resets += 1;

        log::debug!("Connecting to sway for subscribing to events...");

        match sway_subscribe() {
            Err(err) => {
                log::warn!("Could not connect and subscribe: {}", err);
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            Ok(iter) => {
                for ev_result in iter {
                    resets = 0;
                    match ev_result {
                        Ok(ev) => match ev {
                            si::Event::Window(ev) => {
                                log::debug!(
                                    "Window or Workspace event: {:?}",
                                    ev
                                );
                                send_refresh_event(
                                    &sender,
                                    RefreshReason::SwayWindowEvent(ev),
                                );
                            }
                            si::Event::Workspace(ev) => {
                                log::debug!(
                                    "Window or Workspace event: {:?}",
                                    ev
                                );
                                send_refresh_event(
                                    &sender,
                                    RefreshReason::SwayWorkspaceEvent(ev),
                                );
                            }
                            si::Event::Shutdown(sd_ev) => {
                                log::debug!(
                                    "Sway shuts down with reason '{:?}'.",
                                    sd_ev.change
                                );
                                break 'reset;
                            }
                            _ => (),
                        },
                        Err(e) => {
                            log::warn!("Error while receiving events: {}", e);
                            std::thread::sleep(std::time::Duration::from_secs(
                                3,
                            ));
                            log::warn!("Resetting!");
                        }
                    }
                }
            }
        }
    }
}

fn generate_status_1(mods: &[Box<dyn BarModuleFn>], reason: RefreshReason) {
    let mut blocks = vec![];
    for m in mods {
        blocks.push(m.build(&reason));
    }
    let json = serde_json::to_string_pretty(&blocks)
        .unwrap_or_else(|_| "".to_string());
    println!("{},", json);
}

fn generate_status(
    mods: &[Box<dyn BarModuleFn>],
    receiver: Receiver<RefreshReason>,
) {
    println!("{{\"version\": 1, \"click_events\": true}}");
    // status_command should output an infinite array meaning we emit an
    // opening [ and never the closing bracket.
    println!("[");

    for ev in receiver.iter() {
        generate_status_1(mods, ev)
    }
}
