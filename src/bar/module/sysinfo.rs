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
use std::cell::RefCell;
use swaybar_types as s;
use sysinfo;
use sysinfo::SystemExt;

pub struct BarModuleSysInfo {
    pub instance: String,
    system: RefCell<sysinfo::System>,
}

impl BarModuleFn for BarModuleSysInfo {
    fn init() -> Box<dyn BarModuleFn> {
        Box::new(BarModuleSysInfo {
            instance: "0".to_string(),
            system: RefCell::new(sysinfo::System::new_all()),
        })
    }

    fn name() -> String {
        String::from("sysinfo")
    }

    fn instance(&self) -> String {
        self.instance.clone()
    }

    fn build(&self) -> s::Block {
        let x = self.system.borrow().load_average().one.to_string();
        self.system.borrow_mut().refresh_specifics(
            sysinfo::RefreshKind::new().with_cpu().with_memory(),
        );

        s::Block {
            name: Some(Self::name()),
            instance: Some(self.instance.clone()),
            full_text: x,
            align: Some(s::Align::Right),
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
            separator: None,
            separator_block_width: None,
        }
    }
}
