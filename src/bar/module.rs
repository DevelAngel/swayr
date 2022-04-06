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

use crate::bar::config;
use swaybar_types as s;

pub mod battery;
pub mod date;
pub mod sysinfo;
pub mod window;

pub trait BarModuleFn {
    fn create(config: config::ModuleConfig) -> Box<dyn BarModuleFn>
    where
        Self: Sized;
    fn default_config(instance: String) -> config::ModuleConfig
    where
        Self: Sized;
    fn name() -> &'static str
    where
        Self: Sized;
    fn matches(&self, name: &str, instance: &str) -> bool;
    fn build(&self) -> s::Block;
}
