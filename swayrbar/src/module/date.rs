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

use crate::module::config;
use crate::module::{BarModuleFn, NameAndInstance};
use swaybar_types as s;

const NAME: &str = "date";

pub struct BarModuleDate {
    config: config::ModuleConfig,
}

fn chrono_format(s: &str) -> String {
    chrono::Local::now().format(s).to_string()
}

impl BarModuleFn for BarModuleDate {
    fn create(cfg: config::ModuleConfig) -> Box<dyn BarModuleFn> {
        Box::new(BarModuleDate { config: cfg })
    }

    fn default_config(instance: String) -> config::ModuleConfig {
        config::ModuleConfig {
            name: NAME.to_owned(),
            instance,
            format: "⏰ %F %X".to_owned(),
            html_escape: Some(false),
            on_click: None,
        }
    }

    fn get_config(&self) -> &config::ModuleConfig {
        &self.config
    }

    fn get_on_click_map(
        &self,
        name: &str,
        instance: &str,
    ) -> Option<&std::collections::HashMap<String, Vec<String>>> {
        let cfg = self.get_config();
        if name == cfg.name && instance == cfg.instance {
            cfg.on_click.as_ref()
        } else {
            None
        }
    }

    fn build(&self, _nai: &Option<NameAndInstance>) -> s::Block {
        let text = chrono_format(&self.config.format);
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
        Some(cmd.iter().map(|arg| chrono_format(arg)).collect())
    }
}
