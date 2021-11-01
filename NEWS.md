swayr v0.9.0
============

- The commands `switch-workspace`, and `switch-workspace-or-window` now also
  show empty workspaces which makes it possible to switch to another output
  currently showing an empty workspace.
- All menu switching commands (`switch-window`, `switch-workspace`, and
  `switch-workspace-or-window`) can now create and switch to not yet existing
  workspaces.  Just type a digit, a name, or `<digit>:<name>`, confirm your
  input, and it'll do.  The `<digit>:<name>` format is explained in `man 5
  sway`.  If that format is given, `swayr` will create the workspace using
  `workspace number <digit>:<name>`.  If just a digit or name is given, the
  `number` argument is not used.


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
