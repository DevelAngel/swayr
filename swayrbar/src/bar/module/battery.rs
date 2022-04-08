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
use crate::shared::fmt::format_placeholders;
use battery as bat;
use std::collections::{HashMap, HashSet};
use swaybar_types as s;

const NAME: &str = "battery";

pub struct BarModuleBattery {
    config: config::ModuleConfig,
}

fn get_refreshed_batteries(
    manager: &bat::Manager,
) -> Result<Vec<bat::Battery>, bat::Error> {
    let mut bats = vec![];
    for bat in manager.batteries()? {
        let mut bat = bat?;
        if manager.refresh(&mut bat).is_ok() {
            bats.push(bat);
        }
    }

    Ok(bats)
}

fn get_text(cfg: &config::ModuleConfig) -> String {
    // FIXME: Creating the Manager on every refresh is bad but internally
    // it uses an Rc so if I keep it as a field of BarModuleBattery, that
    // cannot be Sync.
    let manager = battery::Manager::new().unwrap();
    match get_refreshed_batteries(&manager) {
        Ok(bats) => {
            if bats.is_empty() {
                return String::new();
            }
            format_placeholders!(&cfg.format, cfg.html_escape, {
                "state_of_charge" => bats.iter()
                    .map(|b| b.state_of_charge().value)
                    .sum::<f32>()
                    / bats.len() as f32 * 100_f32,
                "state_of_health" => bats.iter()
                    .map(|b| b.state_of_health().value)
                    .sum::<f32>()
                    / bats.len() as f32 * 100_f32,
                "state" => {
                    let states = bats.iter()
                        .map(|b| format!("{:?}", b.state()))
                        .collect::<HashSet<String>>();
                    if states.len() == 1 {
                        states.iter().next().unwrap().to_owned()
                    } else {
                        let mut comma_sep_string = String::from("[");
                        let mut first = true;
                        for state in states {
                            if first {
                                comma_sep_string = comma_sep_string + &state;
                                first = false;
                            } else {
                                comma_sep_string = comma_sep_string
                                    + ", " + &state;
                            }
                        }
                        comma_sep_string += "]";
                        comma_sep_string
                    }
                },
            })
        }
        Err(err) => format!("{}", err),
    }
}

impl BarModuleFn for BarModuleBattery {
    fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
        Box::new(BarModuleBattery { config })
    }

    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "🔋 Bat: {state_of_charge:{:5.1}}%, {state}, Health: {state_of_health:{:5.1}}%".to_owned(),
            html_escape: true,
            on_click: HashMap::new()
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self) -> s::Block {
        let text = get_text(&self.config);
        s::Block {
            name: Some(NAME.to_owned()),
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
