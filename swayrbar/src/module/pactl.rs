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

//! The pactl `swayrbar` module.

use crate::config;
use crate::module::{should_refresh, BarModuleFn, NameInstanceAndReason};
use crate::shared::fmt::subst_placeholders;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use swaybar_types as s;

const NAME: &str = "pactl";

struct State {
    volume: u8,
    muted: bool,
}

pub static VOLUME_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r".?* (\d+)%.*").unwrap());

fn run_pactl(args: &[&str]) -> String {
    match Command::new("pactl").args(args).output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(err) => {
            log::error!("Could not run pactl: {}", err);
            String::new()
        }
    }
}

fn get_volume() -> u8 {
    let output = run_pactl(&["get-sink-volume", "@DEFAULT_SINK@"]);
    VOLUME_RX
        .captures(&output)
        .map(|c| c.get(1).unwrap().as_str().parse::<u8>().unwrap())
        .unwrap_or(255_u8)
}

fn get_mute_state() -> bool {
    run_pactl(&["get-sink-mute", "@DEFAULT_SINK@"]).contains("yes")
}

pub struct BarModulePactl {
    config: config::ModuleConfig,
    state: Mutex<State>,
}

fn refresh_state(state: &mut State) {
    state.volume = get_volume();
    state.muted = get_mute_state();
}

fn get_text(fmt: &str, html_escape: bool, state: &State) -> String {
    subst_placeholders!(fmt, html_escape, {
        "volume" => {
            state.volume
        },
        "muted" =>{
            if state.muted {
                " muted"
            } else {
                ""
            }
        },
    })
}

impl BarModuleFn for BarModulePactl {
    fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn>
    where
        Self: Sized,
    {
        Box::new(BarModulePactl {
            config,
            state: Mutex::new(State {
                volume: 255_u8,
                muted: false,
            }),
        })
    }

    fn default_config(instance: String) -> config::ModuleConfig
    where
        Self: Sized,
    {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "🔈 Vol: {volume:{:3}}%{muted}".to_owned(),
            html_escape: Some(true),
            on_click: Some(HashMap::from([
                ("Left".to_owned(), vec!["pavucontrol".to_owned()]),
                (
                    "Right".to_owned(),
                    vec![
                        "pactl".to_owned(),
                        "set-sink-mute".to_owned(),
                        "@DEFAULT_SINK@".to_owned(),
                        "toggle".to_owned(),
                    ],
                ),
                (
                    "WheelUp".to_owned(),
                    vec![
                        "pactl".to_owned(),
                        "set-sink-volume".to_owned(),
                        "@DEFAULT_SINK@".to_owned(),
                        "+1%".to_owned(),
                    ],
                ),
                (
                    "WheelDown".to_owned(),
                    vec![
                        "pactl".to_owned(),
                        "set-sink-volume".to_owned(),
                        "@DEFAULT_SINK@".to_owned(),
                        "-1%".to_owned(),
                    ],
                ),
            ])),
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self, nai: &Option<NameInstanceAndReason>) -> s::Block {
        let mut state = self.state.lock().expect("Could not lock state.");

        if should_refresh(self, nai) {
            refresh_state(&mut state);
        }

        let text =
            get_text(&self.config.format, self.config.is_html_escape(), &state);
        s::Block {
            name: Some(NAME.to_owned()),
            instance: Some(self.config.instance.clone()),
            full_text: text,
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

    fn subst_args<'a>(&'a self, cmd: &'a [String]) -> Option<Vec<String>> {
        let state = self.state.lock().expect("Could not lock state.");
        Some(cmd.iter().map(|arg| get_text(arg, false, &state)).collect())
    }
}
