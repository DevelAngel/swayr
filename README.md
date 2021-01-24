# Swayr is a window switcher for sway

Swayr consists of a demon, and a client.  The demon `swayrd` records window
creations, deletions, and focus changes using sway's JSON IPC interface.  The
client `swayr` offers subcommands, see `swayr --help`.

Right now, there are these subcommands:
* `switch-window` displays all windows in the order urgent first, then LRU,
  focused last and focuses the selected.
* `quit-window` displays all windows and quits the selected one.
* `switch-workspace` displays all workspaces in LRU order and switches to the
  selected one.
* `switch-workspace-or-window` displays all workspaces and their windows and
   switches to the selected workspace or window.
* `quit-workspace-or-window` displays all workspaces and their windows and
  allows to quit either the selected workspace (all its windows) or the
  selected window.
* `execute-swaymsg-command` displays most swaymsg which don't require
  additional input and executes the selected one.  That's handy especially for
  less often used commands not bound to a key.


Swayr is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html) (or later).
