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

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub refresh_interval: f32,
    pub modules: Vec<String>,
    pub module_configs: Vec<ModuleConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleConfig {
    pub module_type: String,
    pub instance: String,
    pub format: String,
    pub html_escape: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            refresh_interval: 1.0,
            modules: vec!["date/0".to_owned()],
            module_configs: vec![],
        }
    }
}
