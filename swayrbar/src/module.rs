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

use std::collections::HashMap;

use crate::config;
use swaybar_types as s;

pub mod battery;
pub mod date;
pub mod pactl;
pub mod sysinfo;
pub mod wifi;
pub mod window;

#[derive(Debug, PartialEq, Eq)]
pub enum RefreshReason {
    ClickEvent,
    SwayEvent,
}

pub type NameInstanceAndReason = (String, String, RefreshReason);

pub trait BarModuleFn: Sync + Send {
    fn default_config(instance: String) -> config::ModuleConfig
    where
        Self: Sized;

    fn get_config(&self) -> &config::ModuleConfig;

    fn get_on_click_map(
        &self,
        name: &str,
        instance: &str,
    ) -> Option<&HashMap<String, Vec<String>>> {
        let cfg = self.get_config();
        if name == cfg.name && instance == cfg.instance {
            cfg.on_click.as_ref()
        } else {
            None
        }
    }

    fn build(&self, nai: &Option<NameInstanceAndReason>) -> s::Block;

    fn should_refresh(
        &self,
        nai: &Option<NameInstanceAndReason>,
        periodic: bool,
        reasons: &[RefreshReason],
    ) -> bool {
        let cfg = self.get_config();
        match nai {
            None => periodic,
            Some((n, i, r)) => {
                n == &cfg.name
                    && i == &cfg.instance
                    && reasons.iter().any(|x| x == r)
            }
        }
    }

    fn subst_args<'a>(&'a self, _cmd: &'a [String]) -> Option<Vec<String>>;
}
