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

use crate::bar::module::BarModuleFn;
use env_logger::Env;
use serde_json;
use std::io;
use std::{sync::Arc, thread};
use swaybar_types as sbt;

pub mod config;
pub mod module;

pub fn start() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .init();

    let config = config::Config::default();

    let mods: Arc<Vec<Box<dyn BarModuleFn>>> = Arc::new(vec![
        module::window::BarModuleWindow::create(
            module::window::BarModuleWindow::default_config("0".to_owned()),
        ),
        module::sysinfo::BarModuleSysInfo::create(
            module::sysinfo::BarModuleSysInfo::default_config("0".to_owned()),
        ),
        module::battery::BarModuleBattery::create(
            module::battery::BarModuleBattery::default_config("0".to_owned()),
        ),
        module::date::BarModuleDate::create(
            module::date::BarModuleDate::default_config("0".to_owned()),
        ),
    ]);
    let mods_for_input = mods.clone();
    thread::spawn(move || handle_input(mods_for_input));
    generate_status(&mods, config.refresh_interval);
}

pub fn handle_input(mods: Arc<Vec<Box<dyn BarModuleFn>>>) {
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
            buf.strip_prefix(",").unwrap_or(&buf),
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
    }
}

pub fn generate_status(mods: &[Box<dyn BarModuleFn>], refresh_interval: u64) {
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
        thread::sleep(std::time::Duration::from_millis(refresh_interval));
    }
}
