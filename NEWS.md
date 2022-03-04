swayr v0.15.0
=============

- There's a new command `switch-to-app-or-urgent-or-lru-window` which given an
  application ID or window class switches to a matching window unless that's
  already the current window.  In that case, it acts just like
  `switch-to-urgent-or-lru-window`.

swayr v0.14.0
=============

- Instead of just printing everything to stdout and stderr, there's now proper
  logging with timestamps and filtering.  You can define the log level using an
  environment variable like so: `env RUST_LOG=swayr=debug swayrd`.  That would
  start swayr with log level `debug`.  Valid log levels in the order from
  logging more to logging less are: `trace`, `debug`, `info`, `warn`, `error`,
  `off`.

swayr v0.13.0
=============

- All the placeholders except `{app_icon}`, `{indent}`, `{urgency_start}`, and
  `{urgency_end}` may optionally provide a format string as specified by
  [Rust's std::fmt](https://doc.rust-lang.org/std/fmt/).  The syntax is
  `{<placeholder>:<fmt_str><clipped_str>}`.  For example,
  `{app_name:{:>10.10}}` would mean that the application name is printed with
  exactly 10 characters.  If it's shorter, it will be right-aligned (the `>`)
  and padded with spaces, if it's longer, it'll be cut after the 10th
  character.  Another example, `{app_name:{:.10}...}` would mean that the
  application name is truncated at 10 characters.  If it's shorter, it will be
  printed as-is (no padding), if it's longer, it'll be cut after the 10th
  character and the last 3 characters of that substring will be replaced with
  `...` (`<clipped_str>`).

swayr v0.12.0
=============

- The `quit-window` command now has an optional `--kill` / `-k` flag.  If
  given, the process of the window to be quit will be killed using `kill -9
  <pid>` instead of just sending sending the `kill` IPC message to sway.

swayr v0.11.1
=============

- Well, bumping the micro version usually indicates a bugfix release but I've
  forgotten to add the `switch-to` command in version 0.11.0.  It's the
  canonical "switch to anything" command, i.e., it offers outputs, workspaces,
  containers, and windows.

swayr v0.11.0
=============

- New command: `switch-output` shows all outputs in the menu and focuses the
  selected one.  Since outputs must now be printable in the menu program,
  there's a new `format.output_format` spec where you can use the output's
  `{name}` and `{id}` to identify it in the menu program.
- New command: `configure-outputs` lets you repeatedly issue output commands
  until you abort the menu program.
- `move-focused-to` now also supports outputs, i.e., you can move the currently
  focused container to some output which means it's moved to the workspace
  currently active on that output.
- Formats can now include an `{output_name}` placeholder which is replaced by
  the name of the output containing the shown workspace, container or window.

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
