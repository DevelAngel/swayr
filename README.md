# Swayr is a window switcher (and more) for sway

[![builds.sr.ht status](https://builds.sr.ht/~tsdh/swayr.svg)](https://builds.sr.ht/~tsdh/swayr?)
[![latest release](https://img.shields.io/crates/v/swayr.svg)](https://crates.io/crates/swayr)
[![License GPL 3 or later](https://img.shields.io/crates/l/swayr.svg)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![dependency status](https://deps.rs/repo/sourcehut/~tsdh/swayr/status.svg)](https://deps.rs/repo/sourcehut/~tsdh/swayr)
[![Hits-of-Code](https://hitsofcode.com/sourcehut/~tsdh/swayr?branch=main)](https://hitsofcode.com/sourcehut/~tsdh/swayr/view?branch=main)

Swayr consists of a demon, and a client.  The demon `swayrd` records
window/workspace creations, deletions, and focus changes using sway's JSON IPC
interface.  The client `swayr` offers subcommands, see `swayr --help`, and
sends them to the demon which executes them.

Right now, there are these subcommands:
* `switch-to-urgent-or-lru-window` switches to the next window with urgency
  hint (if any) or to the last recently used window.
* `switch-window` displays all windows in the order urgent first, then
  last-recently-used, focused last and focuses the selected.
* `switch-workspace` displays all workspaces in LRU order and switches to the
  selected one.
* `switch-workspace-or-window` displays all workspaces and their windows and
   switches to the selected workspace or window.
* `quit-window` displays all windows and quits the selected one.
* `quit-workspace-or-window` displays all workspaces and their windows and
  allows to quit either the selected workspace (all its windows) or the
  selected window.
* `move-focused-to-workspace` moves the currently focused window or container
  to another workspace selected with the menu program.  Non-matching input of
  the form `#w:<workspace>` where the hash and `w:` shortcut are optional can
  be used to move it to a new workspace.
* `next-window (all-workspaces|current-workspace)` & `prev-window
  (all-workspaces|current-workspace)` focus the next/previous window in
  depth-first iteration order of the tree.  The argument `all-workspaces` or
  `current-workspace` define if all windows of all workspaces or only those of
  the current workspace are considered.
* `next-tiled-window` & `prev-tiled-window` do the same as `next-window` &
  `prev-window` but switch only between windows contained in a tiled container.
* `next-tabbed-or-stacked-window` & `prev-tabbed-or-stacked-window` do the same
  as `next-window` & `prev-window` but switch only between windows contained in
  a tabbed or stacked container.
* `next-floating-window` & `prev-floating-window` do the same as `next-window`
  & `prev-window` but switch only between floating windows.
* `next-window-of-same-layout` & `prev-window-of-same-layout` is like
  `next-floating-window` / `prev-floating-window` if the current window is
  floating, it is like `next-tabbed-or-stacked-window` /
  `prev-tabbed-or-stacked-window` if the current window is in a tabbed, or
  stacked container, it is like `next-tiled-window` / `prev-tiled-window` if
  the current windows is in a tiled container, and is like `next-window` /
  `prev-window` otherwise.
* `tile-workspace exclude-floating|include-floating` tiles all windows on the
  current workspace (excluding or including floating ones).  That's done by
  moving all windows away to some special workspace, setting the current
  workspace to `splith` layout, and then moving the windows back.  If the
  `auto_tile` feature is used, see the Configuration section below, it'll
  change from splitting horizontally to vertically during re-insertion.
* `shuffle-tile-workspace exclude-floating|include-floating` shuffles & tiles
  all windows on the current workspace.  The shuffle part means that (a) the
  windows are shuffled before re-insertion, and (b) a randomly chosen already
  re-inserted window is focused before re-inserting another window.  So while
  `tile-workspace` on a typical horizontally oriented screen and 5 windows will
  usually result in a layout with one window on the left and all four others
  tiled vertially on the right, `shuffle-tile-workspace` in combination with
  `auto_tile` usually results in a more balanced layout, i.e., 2 windows tiled
  vertically on the right and the other 4 tiled vertially on the left.  If you
  have less than a handful of windows, just repeat `shuffle-tile-workspace` a
  few times until happenstance creates the layout you wanted.
* `tab-workspace exclude-floating|include-floating` puts all windows of the
  current workspace into a tabbed container.
* `toggle-tab-shuffle-tile-workspace exclude-floating|include-floating` toggles
  between a tabbed and tiled layout, i.e., it calls `shuffle-tile-workspace` if
  it is currently tabbed, and calls `shuffle-tile-workspace` if it is currently
  tiled.
* `execute-swaymsg-command` displays most swaymsg which don't require
  additional input and executes the selected one.  That's handy especially for
  less often used commands not bound to a key.  Non-matching input will be
  executed executed as-is with `swaymsg`.
* `execute-swayr-command` displays all commands above and executes the selected
  one.  (This is useful for accessing swayr commands which are not bound to a
  key.)

### Menu shortcuts for non-matching input

All menu switching commands (`switch-window`, `switch-workspace`, and
`switch-workspace-or-window`) now handle non-matching input instead of doing
nothing.  The input should start with any number of `#` (in order to be able to
force a non-match), a shortcut followed by a colon, and some string as required
by the shortcut.  The following shortcuts are supported.
- `w:<workspace>`: Switches to a possibly non-existing workspace.
  `<workspace>` must be a digit, a name, or `<digit>:<name>`.  The
  `<digit>:<name>` format is explained in `man 5 sway`.  If that format is
  given, `swayr` will create the workspace using `workspace number
  <digit>:<name>`.  If just a digit or name is given, the `number` argument is
  not used.
- `s:<cmd>`: Executes the sway command `<cmd>` using `swaymsg`.
- Any other input is assumed to be a workspace name and thus handled as
  `w:<input>` would do.

## Screenshots

![A screenshot of swayr switch-window](misc/switch-window.png "swayr
switch-window")

![A screenshot of swayr
switch-workspace-or-window](misc/switch-workspace-or-window.png "swayr
switch-workspace-or-window")

## Installation

Some distros have packaged swayr so that you can install it using your distro's
package manager.  Alternatively, it's easy to build and install it yourself
using `cargo`.

### Distro packages

The following GNU/Linux and BSD distros package swayr.  Thanks a lot to the
respective package maintainers!  Refer to the [repology
site](https://repology.org/project/swayr/versions) for details.

[![Packaging status](https://repology.org/badge/vertical-allrepos/swayr.svg)](https://repology.org/project/swayr/versions)

### Building with cargo

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
    swayr next-window all-workspaces >> /tmp/swayr.log 2>&2

bindsym $mod+Prior exec env RUST_BACKTRACE=1 \
    swayr prev-window all-workspaces >> /tmp/swayr.log 2>&2

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

Swayr can be configured using the `~/.config/swayr/config.toml` or
`/etc/xdg/swayr/config.toml` config file.

If no config files exists, a simple default configuration will be created on the
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
html_escape = true
urgency_start = '<span background="darkred" foreground="yellow">'
urgency_end = '</span>'
icon_dirs = [
    '/usr/share/icons/hicolor/scalable/apps',
    '/usr/share/icons/Adwaita/48x48/apps',
    '/usr/share/icons/hicolor/48x48/apps',
    '/usr/share/pixmaps',
]
fallback_icon = '/usr/share/pixmaps/archlinux-logo.png'

[layout]
auto_tile = false
auto_tile_min_window_width_per_output_width = [
    [1024, 500],
    [1280, 600],
    [1400, 680],
    [1440, 700],
    [1600, 780],
    [1920, 920],
    [2560, 1000],
    [3440, 1000],
    [4096, 1200],
]
```

In the following, all sections are explained.

### The menu section

In the `[menu]` section, you can specify the menu program using the
`executable` name or full path, and the `args` (flags and options) it should
get passed.  If some argument contains the placeholder `{prompt}`, it is
replaced with a prompt such as "Switch to window" depending on context.

### The format section

In the `[format]` section, format strings are specified defining how selection
choices are to be layed out.  `wofi` supports [pango
markup](https://docs.gtk.org/Pango/pango_markup.html) which makes it possible
to style the text using HTML and CSS.  The following formats are supported
right now.
* `window_format` defines how windows are displayed.  The placeholder `{title}`
  is replaced with the window's title, `{app_name}` with the application name,
  `{marks}` with a comma-separated list of the window's marks, `{app_icon}`
  with the application's icon (a path to a PNG or SVG file), `{workspace_name}`
  with the name or number of the workspace the window is shown, and `{id}` is
  the window's sway-internal con id.  There are also the placeholders
  `{urcency_start}` and `{urgency_end}` which get replaced by the empty string
  if the window has no urgency flag, and with the values of the same-named
  formats if the window has the urgency flag set.  That makes it possible to
  highlight urgent windows as shown in the default config.
* `workspace_format` defines how workspaces are displayed.  There are the
  placeholders `{name}` which gets replaced by the workspace's number or name,
  and `{id}` which gets replaced by the sway-internal con id of the workspace.
* `html_escape` defines if the strings replacing the placeholders above (except
  for `{urgency_start}` and `{urgency_end}`) should be HTML-escaped.
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

**Hint for wofi**: `wofi` supports icons with the syntax
`'img:<image-file>:text:<text>'`, so a suitable `window_format` with
application icon should start with `img:{app_icon}:text:`.

**Hint for rofi**: `rofi` supports icons with the syntax
`"<text>\u0000icon\u00001f<image-file>"`, so a suitable `window_format` with
application icon should end with `"\u0000icon\u001f<image-file>"`.  Also note
that you must enclose your `window_format` value with double-quotes and not
with single-quotes.  Singe-quote strings are literal strings in
[TOML](https://toml.io/en/v1.0.0#string) where no escape-sequences are
processed whereas for double-quoted strings (so-called basic strings)
escape-sequences are processed.  `rofi` requires a null character and a
PARAGRAPH SEPARATOR for image sequences.

### The layout section

In the `[layout]` section, you can enable auto-tiling by setting `auto_tile` to
`true` (the default is `false`).  The option
`auto_tile_min_window_width_per_output_width` defines the minimum width in
pixels which your windows should have per output width.  For example, the
example setting above says that on an output which is 1600 pixels wide, each
window should have at least a width of 780 pixels, thus there may be at most
two side-by-side windows (Caution, include your borders and gaps in your
calculation!).  There will be no auto-tiling doesn't include your output's
exact width.

If `auto_tile` is enabled, swayr will automatically split either vertically or
horizontally according to this algorithm:
- For all outputs:
  + For all (nested) containers on that output (except the scratchpad):
    - For all child windows of that container:
      + If the container is split horizontally and creating another window
        would make the current child window smaller than the minimum width,
        execute `split vertical` (the `swaymsg` command over IPC) on the child.
      + Else if the container is split vertically and now there is enough space
        so that creating another window would still leave the current child
        window above or equal to the minimum width, call `split horizontal` on
        the child.
      + Otherwise, do nothing for this container.  This means that stacked or
        tabbed containers will never be affected by auto-tiling.

There is one caveat: it would be nice to also trigger auto-tiling when windows
or containers are resized but unfortunately, resizing doesn't issue any events
over IPC.  Therefore, auto-tiling is triggered by new-window events,
close-events, move-events, floating-events, and also focus-events.  The latter
are a workaround and wouldn't be required if there were resize-events.

## Version Changes

Since version 0.8.0, I've started writing a [NEWS](NEWS.md) file listing the
news, and changes to `swayr` commands or configuration options.  If something
doesn't seem to work as expected after an update, please consult this file to
check if there has been some (possibly incompatible) change requiring an update
of your config.

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
