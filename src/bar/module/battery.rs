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

use crate::bar::config;
use crate::bar::module::BarModuleFn;
use crate::fmt_replace::fmt_replace;
use battery as bat;
use std::cell::RefCell;
use swaybar_types as s;

pub struct BarModuleBattery {
    config: config::ModuleConfig,
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

fn get_text(
    manager: &RefCell<bat::Manager>,
    cfg: &config::ModuleConfig,
) -> String {
    match get_refreshed_batteries(manager) {
        Ok(bats) => {
            fmt_replace!(&cfg.format, cfg.html_escape, {
                "state_of_charge" => bats.iter()
                    .map(|b| b.state_of_charge().value)
                    .sum::<f32>()
                    / bats.len() as f32 * 100_f32,
                "state_of_health" => bats.iter()
                    .map(|b| b.state_of_health().value)
                    .sum::<f32>()
                    / bats.len() as f32 * 100_f32,
                "state" => bats.iter()
                    .map(|b| format!("{:?}", b.state()))
                    .next()
                    .unwrap_or_default(),
            })
        }
        Err(err) => format!("{}", err),
    }
}

impl BarModuleFn for BarModuleBattery {
    fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
        Box::new(BarModuleBattery {
            config,
            manager: RefCell::new(
                bat::Manager::new().expect("Could not create Manager"),
            ),
        })
    }

    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            module_type: Self::name().to_owned(),
            instance,
            format: "🔋 Bat: {state_of_charge:{:5.1}}%, {state}, Health: {state_of_health:{:5.1}}%".to_owned(),
            html_escape: true,
        }
    }

    fn name() -> &'static str {
        "battery"
    }

    fn instance(&self) -> &str {
        &self.config.instance
    }

    fn build(&self) -> s::Block {
        let text = get_text(&self.manager, &self.config);
        s::Block {
            name: Some(Self::name().to_owned()),
            instance: Some(self.config.instance.clone()),
            full_text: text,
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