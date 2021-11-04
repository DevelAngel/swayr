swayr v0.9.0
============

- The commands `switch-workspace`, and `switch-workspace-or-window` now also
  show empty workspaces which makes it possible to switch to another output
  currently showing an empty workspace.
- All menu switching commands (`switch-window`, `switch-workspace`, and
  `switch-workspace-or-window`) now handle non-matching input instead of doing
  nothing.  The input should start with any number of `#` (in order to be able
  to force a non-match), a shortcut followed by a colon, and some string as
  required by the shortcut.  The following shortcuts are supported.
  - `w:<workspace>`: Switches to a possibly non-existing workspace.
    `<workspace>` must be a digit, a name, or `<digit>:<name>`.  The
    `<digit>:<name>` format is explained in `man 5 sway`.  If that format is
    given, `swayr` will create the workspace using `workspace number
    <digit>:<name>`.  If just a digit or name is given, the `number` argument
    is not used.
  - `s:<cmd>`: Executes the sway command `<cmd>` using `swaymsg`.
  - Any other input is assumed to be a workspace name and thus handled as
    `w:<input>` would do.
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
  current workspace's windows should be considered: `all-workspaces`, or
  `current-workspace`.
- Bugfix: `prev-window` has never worked correctly.  Instead of cycling through
  all windows in last-recently-used order, it switched between the current and
  last recently used window.  Now it works as expected.
