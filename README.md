# Swayr is a window switcher for sway

Swayr consists of a demon, and a client.  The demon `swayrd` records window
creations, deletions, and focus changes using sway's JSON IPC interface.  The
client `swayr` offers subcommands, see `swayr --help`.

Right now, there is only ony subcommand `swayr switch-window` which is a
wofi-based window switcher sorting the windows in the order urgent first, then
LRU, focused last.

Swayr is licensed under the
[GPLv3](https://www.gnu.org/licenses/gpl-3.0.en.html) (or later).
