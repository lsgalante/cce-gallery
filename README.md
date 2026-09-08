# cce-gallery

A widget gallery and compositor-behaviour test bench for the `cce` desktop
environment. It is a single `cce-ui` client (a real Wayland surface, drawn as
GPU primitives) that shows the toolkit's widgets, in every style each can take,
on one page, and doubles as the binary behind a set of test windows of
different shell types. It ships a `.desktop` entry under Utility/Development,
so it appears in the launcher.

It is a *visual* test harness: you look at what it draws and what the
compositor does with the windows it spawns. It has no automated test suite.

## The page

One page, the widget gallery: one of each widget `cce_ui::widget` exports
that stands on its own. Inputs: Button, ButtonStrip, Checkbox, Toggle, Slider,
RangeSlider, Slider2D, Spinbox, Float3, TextBox, KeybindRecorder,
ColorSelector, FontSelector, Trackpad. Display: ProgressBar, UsageBar,
StatusDot, Separator, Splitter, InfoBox, InteractiveListItem, Breadcrumb,
TreeList, Plate, BevelPreview, RampPreview, Ramp. The MenuBar and StatusBar
frame it.

After those come the variants: every further look a widget can take, one
exhibit each, labelled `<Widget> (<style>)`. The toolkit's own variants are a
constructor (`Button::new_reset`, `new_list_row`, `new_menu_item`,
`new_copy_icon`), a builder (`with_raised(false)` on Button, Toggle and
Dropdown; `with_band`, `with_recessed(false)` and `with_readout` on Slider;
`with_multiline`, `with_draw_bg_border(false)` and `with_password` on
TextBox; `with_vertical` on ButtonStrip, and the Paginator built on it), or a
per-widget override of a config-wide style (`Toggle::with_slide`, the
`style.control.toggle.style` look, next to `Slider::with_band` for
`style.control.slider.style`). The controls whose default look is worked
into the plate — ButtonStrip's well with its raised selected segment,
FontSelector's flush trough, the wells of KeybindRecorder, ColorSelector's
hex field, RangeSlider, ProgressBar, UsageBar and Trackpad — show their flat
look too (`with_recessed(false)` / `with_raised(false)`, what
`control_relief = 0` renders), and ColorSelector its alpha swatch. The four
StatusDot statuses and the plain Label round it off. When a widget gains a
style, it gains an exhibit here.

Every exhibit is drawn at its toolkit default size — the control's configured
`style.control.<name>.height`, or the intrinsic size the widget declares — so
the page is a record of the defaults; only the draw-in canvases (Trackpad,
Plate, TreeList, the previews, the Ramp) are sized here. The `Layout` dropdown
lays the exhibits out with the toolkit's own `ContainerLayout` strategies —
Vertical, Columns, Grid, Adaptive Grid, Mosaic, Reverse Mosaic, Overlay — so
what you see is what a container using that strategy does, and the exhibit
area below the dropdown scrolls (wheel, or the scrollbar at its right edge)
when a layout runs past the window. `Color Ramp...` and `Ramp...` open the
ColorRamp and Ramp editors in their own child windows.

## Child windows

The editors, and a set of test windows with no button of their own, are the
same binary run with `--child`:

```
cce-gallery --child --type <Floating|Fullscreen|Utility|LayerTop|LayerOverlay|LayerBackground|Status|Ramp|ColorRamp>
                   [--width N --height N]
                   [--opacity --transparency 0.0-1.0]
                   [--no-border | --border-width N [--border-bevel]]
                   [--root-plate] [--menubar] [--statusbar]
```

`--type Ramp` and `--type ColorRamp` host the toolkit's `Ramp` and `ColorRamp`
widgets on a root plate; the gallery's two buttons spawn them. The other seven
are the kinds of surface a toolkit client can be under cce, kept for driving
the compositor from a shadow session: Fullscreen sets the toplevel's
fullscreen flag, Utility answers the toolkit's `utility` hook, the three Layer
kinds answer its `layer` hook with a `LayerSettings` (Top anchored
left-top-right with an exclusive zone of its height, Overlay unanchored,
Background on every edge), and Status uses the app_id `cce-status-right-gallery`
so the compositor docks it into the bar. Every other kind's app_id is
`cce-gallery-child-<type>`, with `-noborder` appended when the border is
disabled, so `mode_rule`s can target it. The main window's `app_id` is
`cce-gallery`. A test window shows a one-line description, a `Close` button,
and the optional menu bar and status bar; `--border-width` and `--border-bevel`
are accepted but the child does not draw a bevel.

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

The Ramp child writes the file on every edit; the compositor and the other
apps that draw bevels read it.

## Keybindings

Escape quits, unless a widget consumed it first (a dropdown closing its menu,
for example). Nothing else is bound; the `cce-gallery` domain of
`~/.config/cce/input.kdl` is free for the toolkit-wide chords.

## Layout of the source

- `src/main.rs` — the `Application` impl. The gallery is a concretely typed
  roster of 32 named slots (`GallerySlots`) followed by the variant exhibits
  (`variant_exhibits`, a `Vec<Exhibit>` of boxed widgets), all addressed by
  numeric index in the layout tables, visibility filters and dispatch loops;
  child windows use the five-slot `ChildSlots`. `demo_positions` and
  `child_positions` are the layout tables.
- `src/gallery_widgets.rs` — lookalikes of the retired toolkit `Plate` (an
  exhibit) and root plate (the child windows' background).

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
