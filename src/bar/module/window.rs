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

use crate::bar::module::BarModuleFn;
use crate::tree::NodeIter;
use std::cell::RefCell;
use swaybar_types as s;
use swayipc as ipc;

pub struct BarModuleWindow {
    pub instance: String,
    connection: RefCell<swayipc::Connection>,
}

impl BarModuleFn for BarModuleWindow {
    fn init() -> Box<dyn BarModuleFn> {
        Box::new(BarModuleWindow {
            instance: "0".to_string(),
            connection: RefCell::new(
                ipc::Connection::new()
                    .expect("Couldn't get a sway IPC connection"),
            ),
        })
    }

    fn name() -> String {
        String::from("window")
    }

    fn instance(&self) -> String {
        self.instance.clone()
    }

    fn build(&self) -> s::Block {
        let x: String = match self.connection.borrow_mut().get_tree() {
            Ok(root) => {
                let o: Option<&ipc::Node> =
                    NodeIter::new(&root).find(|n| n.focused);
                o.map(|w| w.name.clone().unwrap_or_default())
                    .unwrap_or_else(String::new)
            }
            Err(err) => format!("{}", err),
        };
        s::Block {
            name: Some(Self::name()),
            instance: Some(self.instance.clone()),
            full_text: x,
            align: Some(s::Align::Right),
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
            separator: None,
            separator_block_width: None,
        }
    }
}
