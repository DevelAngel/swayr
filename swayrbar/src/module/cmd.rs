// Copyright (C) 2024 Luca Matei Pintilie <luca@lucamatei.com>
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

//! The cmd `swayrbar` module.

use crate::config;
use crate::module::{BarModuleFn, RefreshReason};
use std::process::Command;
use std::string::String;
use std::sync::Mutex;
use swaybar_types as s;

const NAME: &str = "cmd";

struct State {
    cached_text: String,
}

pub struct BarModuleCmd {
    config: config::ModuleConfig,
    state: Mutex<State>,
}

fn refresh_state(program: &str) -> String {
    match Command::new("sh").arg("-c").arg(program).output() {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(err) => {
            log::error!("Could not run command: {err}");
            String::new()
        }
    }
}

pub fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
    Box::new(BarModuleCmd {
        config,
        state: Mutex::new(State {
            cached_text: String::new(),
        }),
    })
}

impl BarModuleFn for BarModuleCmd {
    fn default_config(instance: String) -> config::ModuleConfig
    where
        Self: Sized,
    {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: String::new(),
            html_escape: Some(true),
            on_click: None,
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
            state.cached_text = refresh_state(&self.config.format);
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
        cmd.to_vec()
    }
}
