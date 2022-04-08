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

//! TOML configuration for swayrbar.

use crate::module::BarModuleFn;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// The status is refreshed every `refresh_interval` milliseconds.
    pub refresh_interval: u64,
    /// The list of modules to display in the given order, each one specified
    /// as `"<module_type>/<instance>"`.
    pub modules: Vec<String>,
    pub module_configs: Vec<ModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub name: String,
    pub instance: String,
    pub format: String,
    pub html_escape: bool,
    pub on_click: HashMap<String, Vec<String>>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            refresh_interval: 1000,
            modules: vec!["date/0".to_owned()],
            module_configs: vec![
                crate::module::date::BarModuleDate::default_config(
                    "0".to_owned(),
                ),
            ],
        }
    }
}
