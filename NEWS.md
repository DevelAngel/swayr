swayr v0.11.0
=============

- New command: `configure-outputs` lets you repeatedly issue output commands
  until you abort the menu program.
  

swayr v0.10.0
=============

- The `con` module which enhances the sway IPC container tree structure has
  been replaced by `tree` which achieves the same job but is not restricted to
  only handle workspaces and windows.
- There's a new `format.container_format` for formatting the line showing a
  container.
- Formats such as `format.workspace_format`, `format.container_format`, and
  `format.window_format` can now include a `{indent}` placeholder which will be
  replaced with N times the new `format.indent` value.  N is the depth in the
  shown menu input, e.g., with `swayr switch-workspace-or-window` the indent
  level for workspaces is 0 and 1 for windows.
- The `format.workspace_format` and `format.container_format` may include a
  `{layout}` placeholder which is replaced with the container's layout.
- New command: `switch-workspace-container-or-window` shows workspaces,
  containers, and their windows in the menu program and switches to the
  selected one.
- New command: `quit-workspace-container-or-window` shows workspaces,
  containers, and their windows in the menu program and quits all windows of
  the selected workspace/container or the selected window.
- New command: `swap-focused-with` swaps the currently focused window or
  container with the one selected from the menu program.
- New command: `move-focused-to` moves the currently focused container or
  window to the selected one.  Non-matching input will create a new workspace
  of that name and move the focused container or window there.
  

swayr v0.9.0
============

- The commands `switch-workspace` and `switch-workspace-or-window` now also
  show empty workspaces which makes it possible to switch to another output
  currently showing an empty workspace.
- All menu switching commands (`switch-window`, `switch-workspace`, and
  `switch-workspace-or-window`) now handle non-matching input instead of doing
  nothing.  The input should start with any number of `#` (in order to be able
  to force a non-match), a shortcut followed by a colon, and some string as
  required by the shortcut.  The following shortcuts are supported.
  - `w:<workspace>`: Switches to a possibly non-existing workspace.
    `<workspace>` must be a digit, a name or `<digit>:<name>`.  The
    `<digit>:<name>` format is explained in `man 5 sway`.  If that format is
    given, `swayr` will create the workspace using `workspace number
    <digit>:<name>`.  If just a digit or name is given, the `number` argument
    is not used.
  - `s:<cmd>`: Executes the sway command `<cmd>` using `swaymsg`.
  - Any other input is assumed to be a workspace name and thus handled as
    `w:<input>` would do.
- The command `execute-swaymsg-command` executes non-matching input as
  described by the `s:<cmd>` shortcut above.
- There's a new command `move-focused-to-workspace` which moves the currently
  focused window or container to another workspace selected with the menu
  program.  Non-matching input of the form `#w:<workspace>` where the hash and
  `w:` shortcut are optional can be used to move it to a new workspace.


swayr v0.8.0
============

- There's now the possibility to define a system-wide config file
  `/etc/xdg/swayr/config.toml`.  It is used when no
  `~/.config/swayr/config.toml` exists.
- New commands: `next-tiled-window`, `prev-tiled-window`,
  `next-tabbed-or-stacked-window`, `prev-tabbed-or-stacked-window`,
  `next-floating-window`, `prev-floating-window`, `next-window-of-same-layout`,
  and `prev-window-of-same-layout`.
- **Incompatible change**: All `next/prev-window` commands (including the new
  ones above) now have a mandatory subcommand determining if all or only the
  current workspace's windows should be considered: `all-workspaces` or
  `current-workspace`.
- Bugfix: `prev-window` has never worked correctly.  Instead of cycling through
  all windows in last-recently-used order, it switched between the current and
  last recently used window.  Now it works as expected.
