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

//! The battery `swayrbar` module.

use crate::config;
use crate::module::{BarModuleFn, RefreshReason};
use crate::shared::fmt::subst_placeholders;
use battery as bat;
use std::collections::HashSet;
use std::sync::Mutex;
use swaybar_types as s;

const NAME: &str = "battery";

struct State {
    state_of_charge: f32,
    state_of_health: f32,
    state: String,
    cached_text: String,
}

pub struct BarModuleBattery {
    config: config::ModuleConfig,
    state: Mutex<State>,
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

fn refresh_state(state: &mut State, fmt_str: &str, html_escape: bool) {
    // FIXME: Creating the Manager on every refresh is bad but internally
    // it uses an Rc so if I keep it as a field of BarModuleBattery, that
    // cannot be Sync.
    let manager = battery::Manager::new().unwrap();
    match get_refreshed_batteries(&manager) {
        Ok(bats) => {
            state.state_of_charge =
                bats.iter().map(|b| b.state_of_charge().value).sum::<f32>()
                    / bats.len() as f32
                    * 100_f32;
            state.state_of_health =
                bats.iter().map(|b| b.state_of_health().value).sum::<f32>()
                    / bats.len() as f32
                    * 100_f32;
            state.state = {
                let states = bats
                    .iter()
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
                            comma_sep_string = comma_sep_string + ", " + &state;
                        }
                    }
                    comma_sep_string += "]";
                    comma_sep_string
                }
            };
            state.cached_text = subst_placeholders(fmt_str, html_escape, state);
        }
        Err(err) => {
            log::error!("Could not update battery state: {err}");
        }
    }
}

fn subst_placeholders(fmt: &str, html_escape: bool, state: &State) -> String {
    subst_placeholders!(fmt, html_escape, {
        "state_of_charge" => state.state_of_charge,
        "state_of_health" => state.state_of_health,
        "state" => state.state.as_str(),
    })
}

pub fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
    Box::new(BarModuleBattery {
        config,
        state: Mutex::new(State {
            state_of_charge: 0.0,
            state_of_health: 0.0,
            state: "Unknown".to_owned(),
            cached_text: String::new(),
        }),
    })
}

impl BarModuleFn for BarModuleBattery {
    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "🔋 Bat: {state_of_charge:{:5.1}}%, {state}, Health: {state_of_health:{:5.1}}%".to_owned(),
            html_escape: Some(false),
            on_click: None,
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self, reason: &RefreshReason) -> s::Block {
        let mut state = self.state.lock().expect("Could not lock state.");

        if matches!(reason, RefreshReason::TimerEvent) {
            refresh_state(
                &mut state,
                &self.config.format,
                self.get_config().is_html_escape(),
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
