// Copyright (C) 2021  Tassilo Horn <tsdh@gnu.org>
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

use crate::con;
use crate::con::NodeMethods;
use crate::config;
use swayipc as s;

pub fn auto_tile(layout: &config::Layout) {
    if let Ok(mut con) = s::Connection::new() {
        if let Ok(tree) = con.get_tree() {
            let config_map =
                layout.auto_tile_min_window_width_per_output_width_as_map();
            let default_map = config::Layout::default()
                .auto_tile_min_window_width_per_output_width_as_map()
                .unwrap();
            for output in &tree.nodes {
                println!("output: {:?}", output.name);

                // Assert our assumption that all children of the tree's root
                // must be outputs.
                if output.node_type != s::NodeType::Output {
                    panic!(
                        "Child of Root is no Output but a {:?}",
                        output.node_type
                    );
                }

                let output_width = output.rect.width;
                let min_window_width = &config_map
                    .as_ref()
                    .unwrap_or(&default_map)
                    .get(&output_width);

                if let Some(min_window_width) = min_window_width {
                    for container in
                        con::NodeIter::new(output).filter(|n| n.is_container())
                    {
                        println!(
                            "  container: {:?}, layout {:?}, {} nodes",
                            container.node_type,
                            container.layout,
                            container.nodes.len(),
                        );
                        for child_win in
                            container.nodes.iter().filter(|n| n.is_window())
                        {
                            // Width if we'd split once more.
                            let estimated_width =
                                child_win.rect.width as f32 / 2.0;
                            println!(
                                "    child_win: {:?}, estimated_width {} px",
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
                                println!(
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
                                    Err(e) => eprintln!(
                                        "Couldn't set {} on con {}: {:?}",
                                        split, child_win.id, e
                                    ),
                                }
                            }
                        }
                    }
                } else {
                    eprintln!("No layout.auto_tile_min_window_width_per_output_width \
                               setting for output_width {}", output_width);
                }
            }
        } else {
            eprintln!("Couldn't call get_tree during auto_tile.");
        }
    } else {
        eprintln!("Couldn't get connection for auto_tile");
    }
}
