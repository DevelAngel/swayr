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
* `execute-swayr-command` displays all commands above and executes the selected
  one.  (This is useful for accessing swayr commands which are not bound to a
  key.)

## Usage

You need to start the swayr demon `swayrd` in your sway config
(`~/.config/sway/config`) like so:

```
exec env RUST_BACKTRACE=1 swayrd > /tmp/swayrd.log 2>&1
```

The setting of `RUST_BACKTRACE=1` and the redirection of the output to some
logfile is optional but helps a lot when something doesn't work.  Especially,
if you encounter a crash in certain situations and you want to report a bug, it
would be utmost helpful if you could reproduce the issue with backtrace and
logging and attach that to your bug report.

Next to starting the demon, you want to bind swayr commands to some keys like
so:

```
bindsym $mod+Delete exec env RUST_BACKTRACE=1 swayr quit-window > /tmp/swayr.log 2>&1
bindsym $mod+Space exec env RUST_BACKTRACE=1 swayr switch-window >> /tmp/swayr.log 2>&1
bindsym $mod+Shift+Space exec env RUST_BACKTRACE=1 swayr switch-workspace-or-window >> /tmp/swayr.log 2>&1
bindsym $mod+c exec env RUST_BACKTRACE=1 swayr execute-swaymsg-command >> /tmp/swayr.log 2>&1
bindsym $mod+Shift+c exec env RUST_BACKTRACE=1 swayr execute-swayr-command >> /tmp/swa
```

Of course, configure the keys to your liking.  Again, enabling rust backtraces
and logging are optional.

## Bugs

Bugs and requests can be reported [here](https://todo.sr.ht/~tsdh/swayr).

## License

Swayr is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html) (or later).
