# cce-gallery

A widget gallery and compositor-behaviour test bench for the `cce` desktop
environment. It is a single `cce-ui` client (a real Wayland surface, drawn as
GPU primitives) that shows the toolkit's widgets, in every style each can take,
on one page, and doubles as the binary behind a set of test windows of
different shell types. It ships a `.desktop` entry under Utility/Development,
so it appears in the launcher.

It is a *visual* test harness: you look at what it draws and what the
compositor does with the windows it spawns. Its unit tests (`cargo test -p
cce-gallery`) cover the exhibit arrangements (`src/arrange.rs`), that no
layout overlaps its exhibits or lets a Group frame take in one that is not its
member (the real page, at window sizes from 600 to 2400 wide), that the
exhibit table names every exhibit once, that every kind of child window lays
out on its own surface, and the bevel ramp file's parsing.

## The page

One page, the widget gallery: one of each widget `cce_ui::widget` exports
that stands on its own. Inputs: Button, ButtonStrip, Checkbox, Toggle, Slider and
RangeSlider (bands that swell at the value, the one slider style), Slider2D,
Spinbox, Float3, TextBox, KeybindRecorder,
ColorSelector, FontSelector, Trackpad. Display: ProgressBar, UsageBar,
StatusDot, Separator, Splitter, InfoBox, InteractiveListItem, Breadcrumb,
TreeList, Plate, BevelPreview, RampPreview, Ramp. No chrome frames it: the
Layout and Style dropdowns stand in the root plate's top-left corner, and the
exhibit area runs down to the window's foot.

After those come the variants: every further look a widget can take, one
exhibit each, labelled `<Widget> (<style>)`. The toolkit's own variants are a
constructor (`Button::new_reset`, `new_list_row`, `new_menu_item`,
`new_copy_icon`), a builder (`with_readout` on Slider; `with_multiline`,
`with_draw_bg_border(false)` and `with_password` on TextBox; `with_vertical`
on ButtonStrip, and the Paginator built on it). ColorSelector shows its alpha swatch, and the four StatusDot statuses and the
plain Label round it off. When a widget gains a style, it gains an exhibit
here.

The relief-off look is not a variant. The header's `Style` dropdown, beside
`Layout`, switches every control between `Relief` and `Flat` at once
(`cce_ui::layout::set_control_relief`, what `control_relief = 0` configures
DE-wide): the toolkit reads the switch live, so each exhibit shows both looks
in place and no widget appears twice for its style alone. A widget that does
not follow the switch is a toolkit bug — every control's `with_raised` /
`with_recessed` is an override on top of the live default, not a copy of it.

Two `Group` lassos lie over the named exhibits — `Group`, loose around the
FontSelector and the StatusDot, and `Group (fitted)` around the top row,
fitted to the exhibit area's edges (`GROUPS` in `src/main.rs`). A group's
frame is the padded hull of wherever its members land, so the arrangement
treats each group as ONE block: its members are laid out together inside it
(stacked under Vertical, packed otherwise), the block is their footprint plus
the group's padding, and its title tab rides in the label row the arrangement
reserves above every block. A frame therefore never takes in an exhibit that
is not its member, and its tab never lands on the row above. The exhibits
start two paddings in from the area's sides and top — one for the fitted
frame, which snaps out to its seat, and one for its members inside it — so
the tab stays inside the area and the row has the same padding on every side.

Every exhibit is drawn at its toolkit default size — the control's configured
`style.control.<name>.height`, or the intrinsic size the widget declares — so
the page is a record of the defaults; only the draw-in canvases (Trackpad,
Plate, TreeList, the previews, the Ramp) are sized here. The `Layout` dropdown
picks how the exhibits are arranged — Vertical, Grid, Adaptive Grid, Mosaic
(the default), Reverse Mosaic — at the toolkit's control gap
(`layout::control_gap()`, one control height, the one gap the row builders use
in both axes). Vertical and the grids widen a control to its cell, except one
with a size of its own (the Toggle, the Slider2D's pad, the Splitter's grip,
the StatusDots, the copy-icon button, the vertical ButtonStrip), which keeps
its width there; a grid drops columns rather than let its widest exhibit run
into the next. Reverse Mosaic spreads Mosaic's packing over the area: the
exhibits' places, and the widths of the ones that stretch, scale to fill it,
while every exhibit keeps its own height; when they run taller than the
window it keeps Mosaic's rows and scrolls, never squashing them. The arrangements are the gallery's own arithmetic
(`src/arrange.rs`: each exhibit's block in, its content rect out; they began
as cce-ui's `ContainerLayout` strategies), and the gallery lands each widget
at its rect through `layout`: an arrangement places controls' CONTENT boxes
and keeps the gap between FOOTPRINTS — a control, or its detached label where
that runs wider (the Splitter's, a StatusDot's) — a detached label hangs in
the gap above its control,
mixed labeled and unlabeled controls line up by content (an arrangement
reserves the tallest label row above every exhibit), every detached label is the adapter's — the
one label convention: drawn in the strip above the control at the same
`DETACHED_LABEL_INSET`, never inside the control's well (no control carves a
tab for its label; the recess is the control alone) — and every well, trough and raised plateau is carved
inside its widget's rect (`layout::carve_inside`) so a widget's footprint is
its rect. The exhibit
area below the dropdowns is a well sunk into the root plate (`well_floor` under
the exhibits, `well_rim` over them; flat, the shared hairline frame): the
exhibits are laid out on its floor inside the wall, and the area scrolls (wheel,
or the scrollbar down its centre line, which idles behind the floor until a
scroll raises it) when a layout runs past the window — an
exhibit scrolled to the edge slides under the wall, its text cut at the wall's
foot. `Color Ramp...` and `Ramp...` open the
ColorRamp and Ramp editors in their own child windows.

## Child windows

The editors, and a set of test windows with no button of their own, are the
same binary run with `--child`:

```
cce-gallery --child --type <Floating|Fullscreen|Utility|LayerTop|LayerOverlay|LayerBackground|Status|Ramp|ColorRamp>
                   [--width N --height N]
                   [--opacity --transparency 0.0-1.0]
                   [--no-border]
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
`cce-gallery`. A test window shows its title, a one-line description, a
`Close` button, and the optional menu bar and status bar. One too short to
stack those — LayerTop's 40px bar, the Status segment — lays the description
and `Close` out in one row instead, and paints no title. The editors have no
menu bar or status bar, and ignore `--menubar` and `--statusbar`.

The two buttons find the binary by name beside the running one
(`cce_core::process::de_bin`), not at `current_exe`: after `ccebuild install`
replaces the binary, a gallery that was already running reads its own path as
`… (deleted)`.

## The bevel ramp file

The Ramp editor keeps its curve between runs in `~/.config/cce/bevel_ramp.kdl`
(next to the shared `config.kdl`):

```kdl
keys {
    key pos=0 val=0
    key pos=0.5 val=1
    key pos=1 val=0
}
line_type "linear"   // or "bezier"
```

The editor reads the file when it opens (a key it cannot place, such as
`pos=nan`, is dropped) and writes it when the curve changes: whole, to a
temporary file renamed over it. Nothing else reads it — not the compositor,
and not the gallery's own Ramp exhibit, which shows the toolkit's default
curve.

## Keybindings

Escape quits, unless a widget consumed it first (a dropdown closing its menu,
for example). Nothing else is bound; the `cce-gallery` domain of
`~/.config/cce/input.kdl` is free for the toolkit-wide chords.

## Layout of the source

- `src/main.rs` — the `Application` impl. The gallery is a concretely typed
  roster of 33 named slots (`GallerySlots`) followed by the variant exhibits
  (`variant_exhibits`, a `Vec<Exhibit>` of widgets the context owns), all
  addressed by numeric index in the layout tables, visibility filters and
  dispatch loops; child windows use `ChildSlots` — five slots for a test
  window, three for an editor.
  `demo_positions` and `child_positions` are the layout tables for the chrome,
  `exhibit_table` the exhibits' sizes, `layout_exhibits` their placement.
  What the widgets report reaches the app in one sweep after every pointer,
  wheel, key and tick dispatch (`drain_widget_events`, over cce-ui's
  `UiContext::take_events`), so a keyboard activation acts at once.
- `src/arrange.rs` — the exhibit arrangements the `Layout` dropdown offers, as
  pure functions (`arrange`, which takes the Group lassos as blocks too), and
  their unit tests.
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
