// Copyright (C) 2022-2023  Tassilo Horn <tsdh@gnu.org>
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

//! The sysinfo `swayrbar` module.

use crate::config;
use crate::module::{BarModuleFn, RefreshReason};
use crate::shared::fmt::subst_placeholders;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Once;
use swaybar_types as s;
use sysinfo as si;

const NAME: &str = "sysinfo";

struct State {
    cpu_usage: f32,
    mem_usage: f64,
    load_avg_1: f64,
    load_avg_5: f64,
    load_avg_15: f64,
    cached_text: String,
}

pub struct BarModuleSysInfo {
    config: config::ModuleConfig,
    system: Mutex<si::System>,
    state: Mutex<State>,
}

struct OnceRefresher {
    cpu: Once,
    memory: Once,
}

impl OnceRefresher {
    fn new() -> OnceRefresher {
        OnceRefresher {
            cpu: Once::new(),
            memory: Once::new(),
        }
    }

    fn refresh_cpu(&self, sys: &mut si::System) {
        self.cpu.call_once(|| sys.refresh_cpu_all());
    }

    fn refresh_memory(&self, sys: &mut si::System) {
        self.memory.call_once(|| sys.refresh_memory());
    }
}

fn get_cpu_usage(sys: &mut si::System, upd: &OnceRefresher) -> f32 {
    upd.refresh_cpu(sys);
    sys.global_cpu_usage()
}

fn get_memory_usage(sys: &mut si::System, upd: &OnceRefresher) -> f64 {
    upd.refresh_memory(sys);
    sys.used_memory() as f64 * 100_f64 / sys.total_memory() as f64
}

#[derive(Debug)]
enum LoadAvg {
    One,
    Five,
    Fifteen,
}

fn get_load_average(avg: LoadAvg) -> f64 {
    let load_avg = si::System::load_average();
    match avg {
        LoadAvg::One => load_avg.one,
        LoadAvg::Five => load_avg.five,
        LoadAvg::Fifteen => load_avg.fifteen,
    }
}

fn refresh_state(
    sys: &mut si::System,
    state: &mut State,
    fmt_str: &str,
    html_escape: bool,
) {
    let updater = OnceRefresher::new();
    state.cpu_usage = get_cpu_usage(sys, &updater);
    state.mem_usage = get_memory_usage(sys, &updater);
    state.load_avg_1 = get_load_average(LoadAvg::One);
    state.load_avg_5 = get_load_average(LoadAvg::Five);
    state.load_avg_15 = get_load_average(LoadAvg::Fifteen);
    state.cached_text = subst_placeholders(fmt_str, html_escape, state);
}

fn subst_placeholders(fmt: &str, html_escape: bool, state: &State) -> String {
    subst_placeholders!(fmt, html_escape, {
        "cpu_usage" => state.cpu_usage,
        "mem_usage" => state.mem_usage,
        "load_avg_1" => state.load_avg_1,
        "load_avg_5" => state.load_avg_5,
        "load_avg_15" => state.load_avg_15,
    })
}

pub fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
    Box::new(BarModuleSysInfo {
        config,
        system: Mutex::new(si::System::new_all()),
        state: Mutex::new(State {
            cpu_usage: 0.0,
            mem_usage: 0.0,
            load_avg_1: 0.0,
            load_avg_5: 0.0,
            load_avg_15: 0.0,
            cached_text: String::new(),
        }),
    })
}

impl BarModuleFn for BarModuleSysInfo {
    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "💻 CPU: {cpu_usage:{:5.1}}% Mem: {mem_usage:{:5.1}}% Load: {load_avg_1:{:5.2}} / {load_avg_5:{:5.2}} / {load_avg_15:{:5.2}}".to_owned(),
            html_escape: Some(false),
            on_click: Some(HashMap::from([
               ("Left".to_owned(),
                vec!["foot".to_owned(), "htop".to_owned()])])),
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self, reason: &RefreshReason) -> s::Block {
        let mut sys = self.system.lock().expect("Could not lock state.");
        let mut state = self.state.lock().expect("Could not lock state.");

        if matches!(reason, RefreshReason::TimerEvent) {
            refresh_state(
                &mut sys,
                &mut state,
                &self.config.format,
                self.config.is_html_escape(),
            );
        }

        s::Block {
            name: Some(NAME.to_owned()),
            instance: Some(self.config.instance.clone()),
            full_text: state.cached_text.to_owned(),
            align: Some(s::Align::Left),
            markup: Some(s::Markup::Pango),
            short_text: None,
            color: None,
            background: None,
            border: None,
            border_top: None,
            border_bottom: None,
            border_left: None,
            border_right: None,
            min_width: None,
            urgent: None,
            separator: Some(true),
            separator_block_width: None,
        }
    }

    fn subst_cmd_args<'a>(&'a self, cmd: &'a [String]) -> Vec<String> {
        let state = self.state.lock().expect("Could not lock state.");
        cmd.iter()
            .map(|arg| subst_placeholders(arg, false, &state))
            .collect()
    }
}
