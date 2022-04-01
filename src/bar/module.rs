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

use swaybar_types as s;

pub mod date;
pub mod sysinfo;
pub mod window;

pub trait BarModuleFn {
    fn init() -> Box<dyn BarModuleFn>
    where
        Self: Sized;
    fn name() -> String
    where
        Self: Sized;
    fn instance(&self) -> String;
    fn build(&self) -> s::Block;
}
