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

//! The pactl `swayrbar` module.

use crate::config;
use crate::module::{BarModuleFn, RefreshReason};
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
    volume_source: u8,
    muted_source: bool,
    cached_text: String,
}

pub static VOLUME_RX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r".?* (\d+)%.*").unwrap());

fn run_pactl(args: &[&str]) -> String {
    match Command::new("pactl").args(args).output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(err) => {
            log::error!("Could not run pactl: {err}");
            String::new()
        }
    }
}

fn get_volume(get_volume: &str, device: &str) -> u8 {
    let output = run_pactl(&[get_volume, device]);
    VOLUME_RX
        .captures(&output)
        .map(|c| c.get(1).unwrap().as_str().parse::<u8>().unwrap())
        .unwrap_or(255_u8)
}

fn get_mute_state(get_mute: &str, device: &str) -> bool {
    run_pactl(&[get_mute, device]).contains("yes")
}

pub struct BarModulePactl {
    config: config::ModuleConfig,
    state: Mutex<State>,
}

fn refresh_state(state: &mut State, fmt_str: &str, html_escape: bool) {
    state.volume = get_volume("get-sink-volume", "@DEFAULT_SINK@");
    state.muted = get_mute_state("get-sink-mute", "@DEFAULT_SINK@");
    state.volume_source = get_volume("get-source-volume", "@DEFAULT_SOURCE@");
    state.muted_source = get_mute_state("get-source-mute", "@DEFAULT_SOURCE@");
    state.cached_text = subst_placeholders(fmt_str, html_escape, state);
}

fn subst_placeholders(fmt: &str, html_escape: bool, state: &State) -> String {
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
        "volume_source" => {
            state.volume_source
        },
        "muted_source" =>{
            if state.muted_source {
                " muted"
            } else {
                ""
            }
        },
    })
}

pub fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
    Box::new(BarModulePactl {
        config,
        state: Mutex::new(State {
            volume: 255_u8,
            muted: false,
            volume_source: 255_u8,
            muted_source: false,
            cached_text: String::new(),
        }),
    })
}

impl BarModuleFn for BarModulePactl {
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

    fn build(&self, reason: &RefreshReason) -> s::Block {
        let mut state = self.state.lock().expect("Could not lock state.");

        if match reason {
            RefreshReason::TimerEvent => true,
            RefreshReason::ClickEvent { name, instance } => {
                name == &self.config.name && instance == &self.config.instance
            }
            _ => false,
        } {
            refresh_state(
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
