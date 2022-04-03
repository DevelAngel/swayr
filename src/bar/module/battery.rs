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
use battery as bat;
use std::cell::RefCell;
use swaybar_types as s;

pub struct BarModuleBattery {
    pub instance: String,
    manager: RefCell<bat::Manager>,
}

fn get_refreshed_batteries(
    manager: &RefCell<bat::Manager>,
) -> Result<Vec<bat::Battery>, bat::Error> {
    let m = manager.borrow();

    let mut bats = vec![];
    for bat in m.batteries()? {
        let mut bat = bat?;
        if m.refresh(&mut bat).is_ok() {
            bats.push(bat);
        }
    }

    Ok(bats)
}

fn get_text(manager: &RefCell<bat::Manager>, fmt: &str) -> String {
    match get_refreshed_batteries(manager) {
        Ok(bats) => {
            fmt_replace!(&fmt, false, {
                "state_of_charge" => bats.iter()
                    .map(|b| b.state_of_charge().value)
                    .sum::<f32>()
                    / bats.len() as f32 * 100 as f32,
                "state_of_health" => bats.iter()
                    .map(|b| b.state_of_health().value)
                    .sum::<f32>()
                    / bats.len() as f32 * 100 as f32,

                "state" => bats.iter()
                    .map(|b| format!("{:?}", b.state()))
                    .next()
                    .unwrap_or(String::new()),
            })
        }
        Err(err) => format!("{}", err),
    }
}

impl BarModuleFn for BarModuleBattery {
    fn init() -> Box<dyn BarModuleFn> {
        Box::new(BarModuleBattery {
            instance: "0".to_string(),
            manager: RefCell::new(
                bat::Manager::new().expect("Could not create Manager"),
            ),
        })
    }

    fn name() -> String {
        String::from("battery")
    }

    fn instance(&self) -> String {
        self.instance.clone()
    }

    fn build(&self) -> s::Block {
        let fmt =
            "🔋 Bat: {state_of_charge:{:5.1}}% {state} Health: {state_of_health:{:5.1}}%";
        let s = get_text(&self.manager, fmt);
        s::Block {
            name: Some(Self::name()),
            instance: Some(self.instance.clone()),
            full_text: s,
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
