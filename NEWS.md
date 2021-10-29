swayr v8.0.0
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
