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

//! The date `swayrbar` module.

use crate::bar::module::BarModuleFn;
use crate::fmt_replace::fmt_replace;
use std::cell::RefCell;
use std::sync::Once;
use swaybar_types as s;
use sysinfo as si;
use sysinfo::ProcessorExt;
use sysinfo::SystemExt;

pub struct BarModuleSysInfo {
    pub instance: String,
    system: RefCell<si::System>,
}

struct Updater {
    cpu: Once,
    memory: Once,
}

impl Updater {
    fn new() -> Updater {
        Updater {
            cpu: Once::new(),
            memory: Once::new(),
        }
    }

    fn refresh_cpu(&self, sys: &RefCell<si::System>) {
        self.cpu.call_once(|| sys.borrow_mut().refresh_cpu());
    }

    fn refresh_memory(&self, sys: &RefCell<si::System>) {
        self.memory.call_once(|| sys.borrow_mut().refresh_memory());
    }
}

fn get_cpu_usage(sys: &RefCell<si::System>, upd: &Updater) -> f32 {
    upd.refresh_cpu(sys);
    sys.borrow().global_processor_info().cpu_usage()
}

fn get_memory_usage(sys: &RefCell<si::System>, upd: &Updater) -> f64 {
    upd.refresh_memory(sys);
    let sys = sys.borrow();
    sys.used_memory() as f64 * 100 as f64 / sys.total_memory() as f64
}

#[derive(Debug)]
enum LoadAvg {
    One,
    Five,
    Fifteen,
}

fn get_load_average(
    sys: &RefCell<si::System>,
    avg: LoadAvg,
    upd: &Updater,
) -> f64 {
    upd.refresh_cpu(sys);
    let load_avg = sys.borrow().load_average();
    match avg {
        LoadAvg::One => load_avg.one,
        LoadAvg::Five => load_avg.five,
        LoadAvg::Fifteen => load_avg.fifteen,
    }
}

impl BarModuleFn for BarModuleSysInfo {
    fn init() -> Box<dyn BarModuleFn> {
        Box::new(BarModuleSysInfo {
            instance: "0".to_string(),
            system: RefCell::new(si::System::new_all()),
        })
    }

    fn name() -> String {
        String::from("sysinfo")
    }

    fn instance(&self) -> String {
        self.instance.clone()
    }

    fn build(&self) -> s::Block {
        let fmt = "💻 CPU: {cpu_usage:{:4.1}}% Mem: {mem_usage:{:4.1}}% Load: {load_avg_1:{:4.2}} / {load_avg_5:{:4.2}} / {load_avg_15:{:4.2}}";
        let updater = Updater::new();
        s::Block {
            name: Some(Self::name()),
            instance: Some(self.instance.clone()),
            full_text: fmt_replace!(fmt, true, {
                "cpu_usage" => get_cpu_usage(&self.system, &updater),
                "mem_usage" => get_memory_usage(&self.system, &updater),
                "load_avg_1" => get_load_average(&self.system,
                                                 LoadAvg::One, &updater),
                "load_avg_5" => get_load_average(&self.system,
                                                 LoadAvg::Five, &updater),
                "load_avg_15" => get_load_average(&self.system,
                                                  LoadAvg::Fifteen, &updater),
            }),
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
}
