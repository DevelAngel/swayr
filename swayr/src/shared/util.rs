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

/// Utility stuff shared by swayr and swayrbar.
use directories::ProjectDirs;
use std::fs::DirBuilder;
use std::path::Path;

pub fn get_config_file_path(project: &str) -> Box<Path> {
    let proj_dirs = ProjectDirs::from("", "", project).expect("");
    let user_config_dir = proj_dirs.config_dir();
    if !user_config_dir.exists() {
        let sys_path = format!("/etc/xdg/{}/config.toml", project);
        let sys_config_file = Path::new(sys_path.as_str());
        if sys_config_file.exists() {
            return sys_config_file.into();
        }
        DirBuilder::new()
            .recursive(true)
            .create(user_config_dir)
            .unwrap();
    }
    user_config_dir.join("config.toml").into_boxed_path()
}
