# Swayr is a window switcher (and more) for sway

[![builds.sr.ht status](https://builds.sr.ht/~tsdh/swayr.svg)](https://builds.sr.ht/~tsdh/swayr?)
[![latest release](https://img.shields.io/crates/v/swayr.svg)](https://crates.io/crates/swayr)
[![License GPL 3 or later](https://img.shields.io/crates/l/swayr.svg)](https://www.gnu.org/licenses/gpl-3.0.en.html)

Swayr consists of a demon, and a client.  The demon `swayrd` records
window/workspace creations, deletions, and focus changes using sway's JSON IPC
interface.  The client `swayr` offers subcommands, see `swayr --help`, and
sends them to the demon which executes them.

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

## Screenshots

![A screenshot of swayr switch-window](misc/switch-window.png "swayr
switch-window")

![A screenshot of swayr
switch-workspace-or-window](misc/switch-workspace-or-window.png "swayr
switch-workspace-or-window")

## Installation

You'll need to install the current stable rust toolchain using the one-liner
shown at the [official rust installation
page](https://www.rust-lang.org/tools/install).

Then you can install swayr like so:
```sh
cargo install swayr
```

For getting updates easily, I recommend the cargo `install-update` plugin.
```sh
# Install it once.
cargo install install-update

# Then you can update all installed rust binary crates including swayr using:
cargo install-update --all

# If you only want to update swayr, you can do so using:
cargo install-update -- swayr
```

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
    swayr execute-swayr-command >> /tmp/swayr.log 2>&1
```

Of course, configure the keys to your liking.  Again, enabling rust backtraces
and logging are optional.

## Configuration

Swayr can be configured using the `~/.config/swayr/config.toml` config file.

If it doesn't exist, a simple default configuration will be created on the
first invocation for use with the [wofi](https://todo.sr.ht/~scoopta/wofi)
menu program.

It should be easy to adapt that default config for usage with other menu
programs such as [dmenu](https://tools.suckless.org/dmenu/),
[bemenu](https://github.com/Cloudef/bemenu),
[rofi](https://github.com/davatorium/rofi), a script spawning a terminal with
[fzf](https://github.com/junegunn/fzf), or whatever.  The only requirement is
that the launcher needs to be able to read the items to choose from from stdin,
and spit out the selected item to stdout.

The default config looks like this:

```toml
[menu]
executable = 'wofi'
args = [
    '--show=dmenu',
    '--allow-markup',
    '--allow-images',
    '--insensitive',
    '--cache-file=/dev/null',
    '--parse-search',
    '--prompt={prompt}',
]

[format]
window_format = '{urgency_start}<b>“{title}”</b>{urgency_end} — <i>{app_name}</i> on workspace {workspace_name}   <span alpha="20000">({id})</span>'
workspace_format = '<b>Workspace {name}</b>   <span alpha="20000">({id})</span>'
urgency_start = '<span background="darkred" foreground="yellow">'
urgency_end = '</span>'
icon_dirs = [
    '/usr/share/icons/hicolor/scalable/apps',
    '/usr/share/icons/Adwaita/48x48/apps',
    '/usr/share/icons/hicolor/48x48/apps',
    '/usr/share/pixmaps',
]
fallback_icon = '/usr/share/icons/gnome/48x48/apps/kwin.png'
```

In the `[menu]` section, you can specify the menu program using the
`executable` name or full path, and the `args` (flags and options) it should
get passed.  If some argument contains the placeholder `{prompt}`, it is
replaced with a prompt such as "Switch to window" depending on context.

In the `[format]` section, format strings are specified defining how selection
choises are to be layed out.  `wofi` supports [pango
markup](https://docs.gtk.org/Pango/pango_markup.html) which makes it possible
to style the text using HTML and CSS.  The following formats are supported
right now.
* `window_format` defines how windows are displayed.  The placeholder `{title}`
  is replaced with the window's title, `{app_name}` with the application name,
  `{app_icon}` with the application's icon (a path to a PNG or SVG file),
  `{workspace_name}` with the name or number of the workspace the window is
  shown, and `{id}` is the window's sway-internal con id.  There are also the
  placeholders `{urcency_start}` and `{urgency_end}` which get replaced by the
  empty string if the window has no urgency flag, and with the values of the
  same-named formats if the window has the urgency flag set.  That makes it
  possible to highlight urgent windows as shown in the default config.
* `workspace_format` defines how workspaces are displayed.  There are the
  placeholders `{name}` which gets replaced by the workspace's number or name,
  and `{id}` which gets replaced by the sway-internal con id of the workspace.
* `urgency_start` is a string which replaces the `{urgency_start}` placeholder
  in `window_format`.
* `urgency_end` is a string which replaces the `{urgency_end}` placeholder in
  `window_format`.
* `icon_dirs` is a vector of directories in which to look for application icons
  in order to compute the `{app_icon}` replacement.
* `fallback_icon` is a path to some PNG/SVG icon which will be used as
  `{app_icon}` if no application-specific icon can be determined.

It is crucial that during selection (using wofi or some other menu program)
each window has a different display string.  Therefore, it is highly
recommended to include the `{id}` placeholder at least in `window_format`.
Otherwise, e.g., two terminals (of the same terminal app) with the same working
directory (and therefore, the same title) wouldn't be distinguishable.

Hint: `wofi` supports icons with the syntax `img:<image-file>:text:<text>`, so
a suitable `window_format` with application icon should start with
`img:{app_icon}:text:`.

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
