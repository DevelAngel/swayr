# Swayr is a window switcher (and more) for sway

[![Build Badge]][builds.sr.ht] [![Version Badge]][crates.io] [![License Badge]][license]
[Build Badge]: https://builds.sr.ht/~tsdh/swayr.svg
[builds.sr.ht]: https://builds.sr.ht/~tsdh/swayr?
[Version Badge]: https://img.shields.io/crates/v/swayr.svg
[crates.io]: https://crates.io/crates/swayr
[License Badge]: https://img.shields.io/crates/l/swayr.svg
[license]: https://www.gnu.org/licenses/gpl-3.0.en.html

Swayr consists of a demon, and a client.  The demon `swayrd` records
window/workspace creations, deletions, and focus changes using sway's JSON IPC
interface.  The client `swayr` offers subcommands, see `swayr --help`.

Right now, there are these subcommands:
* `next-window` focuses the next window in depth-first iteration order of the
  tree.
* `prev-window` focuses the previous window in depth-first iteration order of
  the tree.
* `switch-window` displays all windows in the order urgent first, then
  last-recently-used, focused last and focuses the selected.
* `quit-window` displays all windows and quits the selected one.
* `switch-to-urgent-or-lru-window` switches to the next window with urgency
  hint (if any) or to the last recently used window.
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
bindsym $mod+Space exec env RUST_BACKTRACE=1 \
    swayr switch-window >> /tmp/swayr.log 2>&1

bindsym $mod+Delete exec env RUST_BACKTRACE=1 \
    swayr quit-window > /tmp/swayr.log 2>&1

bindsym $mod+Tab exec env RUST_BACKTRACE=1 \
    swayr switch-to-urgent-or-lru-window >> /tmp/swayr.log 2>&1

bindsym $mod+Next exec env RUST_BACKTRACE=1 \
    swayr next-window >> /tmp/swayr.log 2>&2

bindsym $mod+Prior exec env RUST_BACKTRACE=1 \
    swayr prev-window >> /tmp/swayr.log 2>&2

bindsym $mod+Shift+Space exec env RUST_BACKTRACE=1 \
    swayr switch-workspace-or-window >> /tmp/swayr.log 2>&1

bindsym $mod+c exec env RUST_BACKTRACE=1 \
    swayr execute-swaymsg-command >> /tmp/swayr.log 2>&1

bindsym $mod+Shift+c exec env RUST_BACKTRACE=1 \
    swayr execute-swayr-command >> /tmp/swa
```

Of course, configure the keys to your liking.  Again, enabling rust backtraces
and logging are optional.

## Questions & Patches

For asking questions, sending feedback, or patches, refer to [my public inbox
(mailinglist)](https://lists.sr.ht/~tsdh/public-inbox).  Please mention the
project you are referring to in the subject.

## Bugs

Bugs and requests can be reported [here](https://todo.sr.ht/~tsdh/swayr).

## Build status

[![builds.sr.ht status](https://builds.sr.ht/~tsdh/swayr.svg)](https://builds.sr.ht/~tsdh/swayr?)

## License

Swayr is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html) (or later).
