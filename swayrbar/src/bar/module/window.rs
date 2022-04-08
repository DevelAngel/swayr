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

use crate::bar::config;
use crate::bar::module::BarModuleFn;
use crate::fmt_replace::fmt_replace;
use crate::ipc;
use crate::ipc::NodeMethods;
use swaybar_types as s;

const NAME: &str = "window";

pub struct BarModuleWindow {
    config: config::ModuleConfig,
}

impl BarModuleFn for BarModuleWindow {
    fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn> {
        Box::new(BarModuleWindow { config })
    }

    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "🪟 {title} — {app_name}".to_owned(),
            html_escape: true,
            on_click: HashMap::new(),
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn build(&self) -> s::Block {
        let root = ipc::get_root_node(false);
        let focused_win = root
            .iter()
            .find(|n| n.focused && n.get_type() == ipc::Type::Window);
        let text = match focused_win {
            Some(win) => {
                fmt_replace!(&self.config.format, self.config.html_escape, {
                    "title" | "name"  =>  win.get_name(),
                    "app_name" => win.get_app_name(),
                })
            }
            None => String::new(),
        };
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
}
