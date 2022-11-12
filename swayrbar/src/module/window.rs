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

//! The window `swayrbar` module.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::config;
use crate::module::{BarModuleFn, NameInstanceAndReason};
use crate::shared::fmt::subst_placeholders;
use crate::shared::ipc;
use crate::shared::ipc::NodeMethods;
use swaybar_types as s;

use super::RefreshReason;

pub const NAME: &str = "window";

const INITIAL_PID: i32 = -128;
const NO_WINDOW_PID: i32 = -1;
const UNKNOWN_PID: i32 = -2;

struct State {
    name: String,
    app_name: String,
    pid: i32,
    cached_text: String,
}

pub struct BarModuleWindow {
    config: config::ModuleConfig,
    state: Mutex<State>,
}

fn refresh_state(state: &mut State, fmt_str: &str, html_escape: bool) {
    let root = ipc::get_root_node(false);
    let focused_win = root
        .iter()
        .find(|n| n.focused && n.get_type() == ipc::Type::Window);

    match focused_win {
        Some(win) => {
            state.name = win.get_name().to_owned();
            state.app_name = win.get_app_name().to_owned();
            state.pid = win.pid.unwrap_or(UNKNOWN_PID);
            state.cached_text = subst_placeholders(fmt_str, html_escape, state);
        }
        None => {
            state.name.clear();
            state.app_name.clear();
            state.pid = NO_WINDOW_PID;
            state.cached_text.clear();
        }
    };
}

fn subst_placeholders(s: &str, html_escape: bool, state: &State) -> String {
    subst_placeholders!(s, html_escape, {
        "title" | "name"  => state.name.clone(),
        "app_name" => state.app_name.clone(),
        "pid" => state.pid,
    })
}

pub fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
    Box::new(BarModuleWindow {
        config,
        state: Mutex::new(State {
            name: String::new(),
            app_name: String::new(),
            pid: INITIAL_PID,
            cached_text: String::new(),
        }),
    })
}

impl BarModuleFn for BarModuleWindow {
    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "🪟 {title} — {app_name}".to_owned(),
            html_escape: Some(false),
            on_click: Some(HashMap::from([
                (
                    "Left".to_owned(),
                    vec![
                        "swayr".to_owned(),
                        "switch-to-urgent-or-lru-window".to_owned(),
                    ],
                ),
                (
                    "Right".to_owned(),
                    vec!["kill".to_owned(), "{pid}".to_owned()],
                ),
            ])),
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self, nai: &Option<NameInstanceAndReason>) -> s::Block {
        let mut state = self.state.lock().expect("Could not lock state.");

        // In contrast to other modules, this one should only refresh its state
        // initially at startup and when explicitly named by `nai` (caused by a
        // window or workspace event).
        if state.pid == INITIAL_PID
            || (self.should_refresh(nai, false, &[RefreshReason::SwayEvent]))
        {
            refresh_state(
                &mut state,
                &self.config.format,
                self.config.is_html_escape(),
            );
        }

        s::Block {
            name: Some(NAME.to_owned()),
            instance: Some(self.config.instance.clone()),
            full_text: state.cached_text.clone(),
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

    fn subst_args<'b>(&'b self, cmd: &'b [String]) -> Option<Vec<String>> {
        let state = self.state.lock().expect("Could not lock state.");
        let cmd = cmd
            .iter()
            .map(|arg| subst_placeholders(arg, false, &state))
            .collect();
        Some(cmd)
    }
}
