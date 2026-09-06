# cce-gallery

A widget gallery and compositor-behaviour test bench for the `cce` desktop
environment. It is a single `cce-ui` client (a real Wayland surface, drawn as
GPU primitives) that shows the toolkit's widgets on one page, spawns simulated
client windows of different shell types on another, and exercises the XDG
file-chooser portal on a third. It ships a `.desktop` entry under
Utility/Development, so it appears in the launcher.

It is a *visual* test harness: you look at what it draws and what the
compositor does with the windows it spawns. It has no automated test suite.

## Pages

The sidebar on the left switches between three pages. The same navigation is
available from the dropdown in the status bar and from the keyboard (see
Keybindings).

**Controls** — the widget gallery. One of each of the toolkit's standard
widgets: Button, Checkbox, Toggle, ProgressBar, Slider, RangeSlider, Spinbox,
TextBox, Trackpad, Plate, plus the MenuBar and StatusBar that frame every
page. A `Layout` dropdown re-packs the gallery under each of the toolkit's
layout strategies (Vertical, Columns, Grid, Adaptive Grid, Overlay, Flex,
Splitter, Radial, Circular Pane, Mosaic, Reverse Mosaic). `Color Ramp...` and
`Ramp...` open the ColorRamp and Ramp editors in their own child windows.

**Windows** — spawns *simulated client windows* so the compositor's handling
of each surface kind can be observed. The control panel on the right chooses:

- window type: Toplevel, Popup, Layer Top, Layer Overlay, Layer Background
- shape: rectangular or circular
- size (width/height spinboxes), opacity on/off with a transparency slider
- window elements: backplate, menu bar, status bar
- border: enabled/disabled, width, bevel on/off, bevel depth, and the bevel
  cross-section shape (`Bevel Shape...` opens the Ramp editor)

`Create Window` re-executes this binary with `--child` and the matching flags
(see below). A panel in the main window previews the border and bevel that
the child will be drawn with, and a description plate explains what the
selected surface type is and how it is expected to be laid out.

**XDG** — two buttons, `Open File` and `Save File`, that call the toolkit's
`file_dialog` module (the XDG Desktop Portal file chooser, via `rfd`) on a
worker thread and report the chosen path, or the cancellation, in the status
bar.

## Child windows

The child windows spawned from the Windows page are the same binary run with
`--child`. The flags mirror the control panel:

```
cce-gallery --child --type <Toplevel|Popup|LayerTop|LayerOverlay|LayerBackground|Ramp|ColorRamp>
                   [--shape rectangular|circular] [--width N --height N]
                   [--opacity --transparency 0.0-1.0]
                   [--no-border | --border-width N [--border-bevel]]
                   [--backplate] [--menubar] [--statusbar]
```

A simulated window shows a description label, a `Close` button, and the
optional menu bar and status bar. `--border-width` and `--border-bevel` are
accepted so the argv mirrors the panel, but the child does not draw a bevel
yet; the bevel preview lives in the main window's panel. Its `app_id` encodes what it is, so
compositor rules can target it: `cce-gallery-child-<type>` with `-circular`
and/or `-noborder` appended when those apply. The main window's `app_id` is
`cce-gallery` (also with `-noborder` when the border is disabled).

`--type Ramp` and `--type ColorRamp` are editor windows rather than simulated
clients: they host the toolkit's `Ramp` and `ColorRamp` widgets on a
backplate.

## The bevel ramp file

The bevel cross-section is shared between the Ramp editor window and the main
window through `~/.config/cce/bevel_ramp.kdl` (next to the shared
`config.kdl`):

```kdl
keys {
    key pos=0 val=0
    key pos=0.5 val=1
    key pos=1 val=0
}
line_type "linear"   // or "bezier"
```

The Ramp child writes the file on every edit. The main window watches its
mtime each tick and reloads, so the bevel preview on the Windows page follows
the editor live.

## Keybindings

Page navigation is resolved through the `cce-gallery` domain of
`~/.config/cce/input.kdl`; the defaults are `ctrl+1`, `ctrl+2`, `ctrl+3`:

```kdl
cce-gallery {
    page_1 "ctrl+1"
    page_2 "ctrl+2"
    page_3 "ctrl+3"
}
```

Escape quits, unless a widget consumed it first (a dropdown closing its menu,
for example).

## Layout of the source

- `src/main.rs` — the `Application` impl. The gallery is a concretely typed
  roster of 45 named slots (`GallerySlots`) addressed by numeric index in the
  layout tables, visibility filters and dispatch loops; child windows use the
  five-slot `ChildSlots`. `demo_positions` and `child_positions` are the
  layout tables.
- `src/gallery_widgets.rs` — gallery-local widgets: `ControlPanel` (scroll chrome
  for the Windows page's option column), and lookalikes of the retired toolkit
  `Plate`, `SectionContainer` and `Backplate` kept as exhibits.

## Building and installing

This crate is one member of the `cce` multi-repo workspace; it builds
standalone or from the workspace root. Installation goes through `ccebuild`
(see `cce-compositor/WORKSPACE.md`):

```sh
cargo build --release -p cce-gallery
ccebuild install --no-build cce-gallery   # what `make install` runs
```

Run it from the workspace with `cargo run -p cce-gallery`, or from the
launcher once installed.
