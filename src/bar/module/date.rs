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
use swaybar_types as s;

pub struct BarModuleDate {
    pub instance: String,
}

impl BarModuleFn for BarModuleDate {
    fn init() -> Box<dyn BarModuleFn> {
        Box::new(BarModuleDate {
            instance: "0".to_string(),
        })
    }

    fn name() -> String {
        String::from("date")
    }

    fn instance(&self) -> String {
        self.instance.clone()
    }

    fn build(&self) -> s::Block {
        let d = chrono::Local::now().format("⏰ %F %X").to_string();
        s::Block {
            name: Some(Self::name()),
            instance: Some(self.instance.clone()),
            full_text: d,
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
            separator: Some(true),
            separator_block_width: None,
        }
    }
}
