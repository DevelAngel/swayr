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
use std::thread;

pub mod module;

pub fn start() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn"))
        .init();

    thread::spawn(handle_input);
    let mods: Vec<Box<dyn BarModuleFn>> = vec![
        crate::bar::module::window::BarModuleWindow::init(),
        crate::bar::module::sysinfo::BarModuleSysInfo::init(),
        crate::bar::module::battery::BarModuleBattery::init(),
        crate::bar::module::date::BarModuleDate::init(),
    ];
    generate_status(&mods);
}

pub fn handle_input() {
    // TODO: Read stdin and react to click events.
}

pub fn generate_status(mods: &[Box<dyn BarModuleFn>]) {
    println!("{{\"version\": 1}}");
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
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
