// Copyright (C) 2021-2022  Tassilo Horn <tsdh@gnu.org>
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

//! Structure to hold window focus timestamps used by swayrd

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::RwLock;

/// Data tracking most recent focus events for Sway windows/containers
#[derive(Clone)]
pub struct FocusData {
    pub focus_tick_by_id: Arc<RwLock<HashMap<i64, u64>>>,
    pub focus_chan: mpsc::Sender<FocusMessage>,
}

impl FocusData {
    pub fn last_focus_tick(&self, id: i64) -> u64 {
        *self.focus_tick_by_id.read().unwrap().get(&id).unwrap_or(&0)
    }

    pub fn update_last_focus_tick(&self, id: i64, focus_val: u64) {
        let mut write_lock = self.focus_tick_by_id.write().unwrap();
        if let Some(tick) = write_lock.get_mut(&id) {
            *tick = focus_val;
        }
        // else the node has since been closed before this focus event got locked in
    }

    pub fn remove_focus_data(&self, id: i64) {
        self.focus_tick_by_id.write().unwrap().remove(&id);
    }

    /// Ensures that a given node_id is present in the ExtraProps map, this
    /// later used to distinguish between the case where a container was
    /// closed (it will no longer be in the map) or
    pub fn ensure_id(&self, id: i64) {
        let mut write_lock = self.focus_tick_by_id.write().unwrap();
        if write_lock.get(&id).is_none() {
            write_lock.insert(id, 0);
        }
    }

    pub fn send(&self, fmsg: FocusMessage) {
        // todo can this be removed?
        if let FocusMessage::FocusEvent(ref fev) = fmsg {
            self.ensure_id(fev.node_id);
        }
        self.focus_chan
            .send(fmsg)
            .expect("Failed to send focus event over channel");
    }
}

pub struct FocusEvent {
    pub node_id: i64,      // node receiving the focus
    pub ev_focus_ctr: u64, // Counter for this specific focus event
}

pub enum FocusMessage {
    TickUpdateInhibit,
    TickUpdateActivate,
    FocusEvent(FocusEvent),
}
