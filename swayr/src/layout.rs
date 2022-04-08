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

//! Functions and data structures of the swayrd demon.

use crate::config;
use crate::shared::ipc;
use crate::shared::ipc::NodeMethods;
use std::collections::HashMap;
use swayipc as s;

pub fn auto_tile(res_to_min_width: &HashMap<i32, i32>) {
    if let Ok(mut con) = s::Connection::new() {
        if let Ok(tree) = con.get_tree() {
            for output in &tree.nodes {
                log::debug!("output: {:?}", output.name);

                // Assert our assumption that all children of the tree's root
                // must be outputs.
                if output.node_type != s::NodeType::Output {
                    panic!(
                        "Child of Root is no Output but a {:?}",
                        output.node_type
                    );
                }

                let output_width = output.rect.width;
                let min_window_width = &res_to_min_width.get(&output_width);

                if let Some(min_window_width) = min_window_width {
                    for container in output.iter().filter(|n| {
                        let t = n.get_type();
                        t == ipc::Type::Workspace || t == ipc::Type::Container
                    }) {
                        if container.is_scratchpad() {
                            log::debug!("  Skipping scratchpad");
                            continue;
                        }
                        log::debug!(
                            "  container: {:?}, layout {:?}, {} nodes",
                            container.node_type,
                            container.layout,
                            container.nodes.len(),
                        );
                        for child_win in container
                            .nodes
                            .iter()
                            .filter(|n| n.get_type() == ipc::Type::Window)
                        {
                            // Width if we'd split once more.
                            let estimated_width =
                                child_win.rect.width as f32 / 2.0;
                            log::debug!(
                                "    child_win: {:?}, estimated width after splith {} px",
                                child_win.app_id, estimated_width
                            );
                            let split = if container.layout
                                == s::NodeLayout::SplitH
                                && estimated_width <= **min_window_width as f32
                            {
                                Some("splitv")
                            } else if container.layout == s::NodeLayout::SplitV
                                && estimated_width > **min_window_width as f32
                            {
                                Some("splith")
                            } else {
                                None
                            };

                            if let Some(split) = split {
                                log::debug!(
                                    "Auto-tiling performing {} on window {} \
                                     because estimated width after another \
                                     split is {} and the minimum window width \
                                     is {} on this output.",
                                    split,
                                    child_win.id,
                                    estimated_width,
                                    min_window_width
                                );
                                match con.run_command(format!(
                                    "[con_id={}] {}",
                                    child_win.id, split
                                )) {
                                    Ok(_) => (),
                                    Err(e) => log::error!(
                                        "Couldn't set {} on con {}: {:?}",
                                        split,
                                        child_win.id,
                                        e
                                    ),
                                }
                            }
                        }
                    }
                } else {
                    log::error!("No layout.auto_tile_min_window_width_per_output_width \
                               setting for output_width {}", output_width);
                }
            }
        } else {
            log::error!("Couldn't call get_tree during auto_tile.");
        }
    } else {
        log::error!("Couldn't get connection for auto_tile");
    }
}

pub fn maybe_auto_tile(config: &config::Config) {
    if config.is_layout_auto_tile() {
        log::debug!("auto_tile: start");
        auto_tile(
            &config
                .get_layout_auto_tile_min_window_width_per_output_width_as_map(
                ),
        );
        log::debug!("auto_tile: end");
    }
}

const SWAYR_TMP_WORKSPACE: &str = "✨";

pub fn relayout_current_workspace(
    include_floating: bool,
    insert_win_fn: Box<
        dyn Fn(&mut [&s::Node], &mut s::Connection) -> s::Fallible<()>,
    >,
) -> s::Fallible<()> {
    let root = ipc::get_root_node(false);
    let workspaces: Vec<&s::Node> = root
        .iter()
        .filter(|n| n.get_type() == ipc::Type::Workspace)
        .collect();
    if let Some(cur_ws) = workspaces.iter().find(|ws| ws.is_current()) {
        if let Ok(mut con) = s::Connection::new() {
            let mut moved_wins: Vec<&s::Node> = vec![];
            let mut focused_win = None;
            for win in
                cur_ws.iter().filter(|n| n.get_type() == ipc::Type::Window)
            {
                if win.focused {
                    focused_win = Some(win);
                }
                if !include_floating && win.is_floating() {
                    continue;
                }
                moved_wins.push(win);
                con.run_command(format!(
                    "[con_id={}] move to workspace {}",
                    win.id, SWAYR_TMP_WORKSPACE
                ))?;
            }

            insert_win_fn(moved_wins.as_mut_slice(), &mut con)?;
            std::thread::sleep(std::time::Duration::from_millis(25));

            if let Some(win) = focused_win {
                con.run_command(format!("[con_id={}] focus", win.id))?;
            }
            Ok(())
        } else {
            Err(s::Error::CommandFailed(
                "Cannot create connection.".to_string(),
            ))
        }
    } else {
        Err(s::Error::CommandFailed(
            "No workspace is focused.".to_string(),
        ))
    }
}
