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
use crate::module::BarModuleFn;
use env_logger::Env;
use serde_json;
use std::io;
use std::process as p;
use std::sync::Condvar;
use std::sync::Mutex;
use std::time::Duration;
use std::{sync::Arc, thread};
use swaybar_types as sbt;

pub fn start() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .init();

    let config = config::load_config();
    let refresh_interval = config.refresh_interval;
    let mods: Arc<Vec<Box<dyn BarModuleFn>>> = Arc::new(create_modules(config));
    let mods_for_input = mods.clone();
    let trigger = Arc::new((Mutex::new(()), Condvar::new()));
    let trigger_for_input = trigger.clone();
    thread::spawn(move || handle_input(mods_for_input, trigger_for_input));
    generate_status(&mods, trigger, refresh_interval);
}

fn create_modules(config: config::Config) -> Vec<Box<dyn BarModuleFn>> {
    let mut mods = vec![];
    for mc in config.modules {
        let m = match mc.name.as_str() {
            "window" => module::window::BarModuleWindow::create(mc),
            "sysinfo" => module::sysinfo::BarModuleSysInfo::create(mc),
            "battery" => module::battery::BarModuleBattery::create(mc),
            "date" => module::date::BarModuleDate::create(mc),
            unknown => {
                log::warn!("Unknown module name '{}'.  Ignoring...", unknown);
                continue;
            }
        };
        mods.push(m);
    }
    mods
}

pub fn handle_input(
    mods: Arc<Vec<Box<dyn BarModuleFn>>>,
    trigger: Arc<(Mutex<()>, Condvar)>,
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
                log::error!("The string was {}", buf);
                log::error!("Skipping this input line...");
                continue;
            }
        };
        log::debug!("Received click: {:?}", click);
        if handle_click(click, mods.clone()).is_some() {
            let (_, cvar) = &*trigger;
            cvar.notify_one();
        }
    }
}

fn handle_click(
    click: sbt::Click,
    mods: Arc<Vec<Box<dyn BarModuleFn>>>,
) -> Option<()> {
    let name = click.name?;
    let instance = click.instance?;
    let button_str = format!("{:?}", click.button);
    for m in mods.iter() {
        if let Some(on_click) = m.get_on_click_map(&name, &instance) {
            if let Some(cmd) = on_click.get(&button_str) {
                match m.subst_args(cmd) {
                    Some(cmd) => execute_command(&cmd),
                    None => execute_command(cmd),
                }
                // Wait a bit so that the action of the click has shown its
                // effect, e.g., the window has been switched.
                thread::sleep(Duration::from_millis(50));
                return Some(());
            }
        }
    }

    None
}

fn execute_command(cmd: &[String]) {
    log::debug!("Executing command: {:?}", cmd);
    match p::Command::new(&cmd[0]).args(&cmd[1..]).status() {
        Ok(exit_status) => {
            // TODO: Better use exit_ok() once that has stabilized.
            if !exit_status.success() {
                log::warn!(
                    "Command finished with status code {:?}.",
                    exit_status.code()
                )
            }
        }
        Err(err) => {
            log::error!("Error running shell command '{}':", cmd.join(" "));
            log::error!("{}", err);
        }
    }
}

pub fn generate_status(
    mods: &[Box<dyn BarModuleFn>],
    trigger: Arc<(Mutex<()>, Condvar)>,
    refresh_interval: u64,
) {
    println!("{{\"version\": 1, \"click_events\": true}}");
    // status_command should output an infinite array meaning we emit an
    // opening [ and never the closing bracket.
    println!("[");

    loop {
        let mut blocks = vec![];
        for m in mods {
            blocks.push(m.build());
        }
        let json = serde_json::to_string_pretty(&blocks)
            .unwrap_or_else(|_| "".to_string());
        println!("{},", json);

        let (lock, cvar) = &*trigger;
        let triggered = lock.lock().unwrap();
        let result = cvar
            .wait_timeout(triggered, Duration::from_millis(refresh_interval))
            .unwrap();
        if !result.1.timed_out() {
            log::debug!("Status writing thread waked up early by click event.");
        }
    }
}
