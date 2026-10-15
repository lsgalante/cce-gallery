# cce-gallery

A widget gallery and compositor-behaviour test bench for the `cce` desktop
environment. It is a single `cce-ui` client (a real Wayland surface, drawn as
GPU primitives) that shows the toolkit's widgets, in every style each can take,
on one page and its relief effects on another, and doubles as the binary behind
a set of test windows of different shell types. It ships a `.desktop` entry
under Utility/Development, so it appears in the launcher.

It is a *visual* test harness: you look at what it draws and what the
compositor does with the windows it spawns. Its unit tests (`cargo test -p
cce-gallery`) cover the exhibit arrangements (`src/arrange.rs`), that no
layout overlaps its exhibits or lets a Group frame take in one that is not its
member, that a Group's padding is even on every side, and that the gaps down
the stacking layouts' columns are all the control gap (the real page, at window
sizes from 600 to 2400 wide), that the Ramp and the ColorRamp lie across two
grid columns (the Ramp at the height the page gives it), that holding an
exhibit's drag moves nothing on the page — through a scroll and a resize — that the
exhibit table names every exhibit once and no exhibit is a Paginator, that the
Relief page shows every tile apart on the floor in every layout and none of the
widget page (not painted, not hit, not a Tab stop) and that switching back
restores it, that a switch scrolls to the top, ends a press or drag held across
it without clicking a held button and closes the context menu, that the header wraps where its
dropdowns do not fit one row (down to widths under one dropdown), where an
editor window is anchored, that every kind of child window lays
out on its own surface, the disc plate tile's face, and the bevel ramp file's
parsing.

## The widget page

The page the gallery opens on (`Widgets`): one of each widget `cce_ui::widget` exports
that stands on its own. Inputs: Button, ButtonStrip, Checkbox, Toggle, Slider and
RangeSlider (bands that swell at the value, the one slider style), Slider2D,
Spinbox, SliderSpinbox (a slider with a spinbox at its right end, here an
int), Float3, TextBox, KeybindRecorder,
ColorSelector, FontSelector, Trackpad. Display: ProgressBar, UsageBar,
StatusDot, Separator, Splitter, InfoBox, InteractiveListItem, Breadcrumb,
TreeList, Plate, BevelPreview, RampPreview, Ramp, ColorRamp. No chrome frames it: the
Layout, Style and Page dropdowns stand in a row in the root plate's top-left
corner — in a window too narrow for the three and its inset either side they
flow onto further rows, each starting a new row at the left inset, one root gap
below, when it would run past the right one — and the exhibit area runs from
one root gap below the lowest of them down to the window's foot, within the
insets however narrow the window.

After those come the variants: every further look a widget can take, one
exhibit each, labelled `<Widget> (<style>)`. The toolkit's own variants are a
constructor (`Button::new_reset`, `new_list_row`, `new_menu_item`,
`new_copy_icon`; `SliderSpinbox::new_float`, to two decimals), a builder
(`with_readout` on Slider; `with_multiline`,
`with_draw_bg_border(false)` and `with_password` on TextBox; `with_vertical`
on ButtonStrip). ColorSelector shows its alpha swatch,
InteractiveListItem a selected row (its style in the subtitle: the row draws
its own text, and the name with the style does not fit one line), and the four StatusDot statuses and the
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
(stacked under Vertical; otherwise side by side, top-aligned, when they all
fit across the group's cell, and stacked when they do not), each under its own
label strip alone — a group has no shared label row, so an unlabelled member
sits at the top beside a labelled one — and the block is their footprint, its
title tab riding in the label strip the arrangement reserves above the block.
The frame hangs one padding outside the block on every side, into the gaps
around it. A frame therefore never takes in an exhibit that is not its member,
its padding is the same on all four sides, and its members start in the same
column as every other exhibit. The exhibits start two paddings in from the
area's sides and top: one for the fitted frame, which lands on its seat, and
one between it and its members.

Every exhibit is drawn at its toolkit default size — the control's configured
`style.control.<name>.height`, or the intrinsic size the widget declares — so
the page is a record of the defaults; only the draw-in canvases (Trackpad,
Plate, TreeList, the previews) are sized here, and the Ramp, the one exhibit
drawn past its default. It declares a 120 px graph over its control strip, a
height sized for a host's row; the page gives the graph twice that, and for the
Ramp alone the height the page gives wins over the one the widget declares
(`ExhibitSize::own_h`), the extra going to the graph. The `Layout` dropdown
picks how the exhibits are arranged — Vertical, Adaptive Grid (the default),
Mosaic, Reverse Mosaic — at the toolkit's control gap
(`layout::control_gap()`, one control height, the one gap the row builders use
in both axes). Vertical and Adaptive Grid widen a control to its cell, except one
with a size of its own (the Toggle, the Slider2D's pad, the Splitter's grip,
the StatusDots, the copy-icon button, the vertical ButtonStrip), which keeps
its width there. Adaptive Grid fits as many columns of at least an exhibit's
width (190 px) as the window holds, and drops one rather than let its widest
exhibit run into the next. The TreeList, the Ramp and the ColorRamp lie across
two of its columns — the TreeList so its key, type and value columns all show,
the Ramp for a curve wider than a control row, the ColorRamp so its R, G and B
sliders have length beside its delete button: each goes below the pair whose
taller column is shorter, and the shorter one keeps the space above it empty.
The span is Adaptive Grid's alone: the mosaics pack all three from a control
row's width, and Vertical widens every exhibit to its one column. Reverse Mosaic spreads Mosaic's packing over the area: the
exhibits' places, and the widths of the ones that stretch, scale to fill it,
while every exhibit keeps its own height; when they run taller than the
window it keeps Mosaic's rows and scrolls, never squashing them. The arrangements are the gallery's own arithmetic
(`src/arrange.rs`: each exhibit's block in, its content rect out; they began
as cce-ui's `ContainerLayout` strategies), and the gallery lands each widget
at its rect through `layout`: an arrangement places controls' CONTENT boxes
and keeps the gap between FOOTPRINTS — a control, or its detached label where
that runs wider (the Splitter's, a StatusDot's) — a detached label hangs in
the gap above its control,
mixed labeled and unlabeled controls packed side by side line up by content
(Mosaic and Reverse Mosaic reserve the tallest label row above every exhibit),
while Vertical and Adaptive Grid, which stack exhibits in columns, give each only
its own label strip, so the gap down a column is the same above a labeled
control and an unlabeled one; every detached label is the adapter's — the
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
foot. While an exhibit is dragged — a slider's thumb, a Ramp or ColorRamp key
held from the press on — the page is frozen: nothing is re-arranged or landed,
the dragged exhibit included, and the area does not scroll (a glide in flight
stops, an unclaimed wheel is dropped), so nothing moves out from under the
pointer or lands over the dragged exhibit, not even when the window is resized;
the first frame after the release arranges the page afresh. `Color Ramp...` and
`Ramp...` open the ColorRamp and Ramp editors in their own child windows, just
under the button: before spawning one the gallery hands the compositor a
`place-next` hint at the button's bottom-left corner on screen — its own
window's corner in `windows --json` (found by its pid) plus the button's offset
scaled by the desk's zoom, read from the compositor's `camera` reply, since the
window list gives a box unzoomed — or at the pointer when either reply is
missing. The window's box comes first so that a button pressed from the
keyboard anchors as a clicked one does.

## The Relief page

The header's `Page` dropdown switches the exhibit area between `Widgets` and
`Relief`: one tile for each of the toolkit's relief effects, the `PaintCtx`
emitters a lit surface is drawn with, each drawn as the toolkit's widgets draw
it (`ReliefTile` in `src/gallery_widgets.rs`):

- plates: a pane plate (`PlateSpec`, the pane material), a bevel, a boss; a
  control plate (`ControlPlate`) raised, flush, flat, and flush with the focus
  ring (`ControlPlate::focus_tint`); a faceless flush inset plate;
- carves: a recess, a trough, a ridge;
- a field in each of its forms (`Field`): all well, all run, a well ending in a
  run, a sliding run (a toggle's), a run spanning the middle;
- wells: a canvas well (`well_floor`, a curve clipped to the well's shape
  running under the rim, `well_rim`, at the panel radius), and a section well
  with its title tab, as a `Group` cuts its frame;
- composed shapes: a concave fillet rounding an inside corner, carved and
  raised; grooves (a Separator's rule, a Breadcrumb's slanted seam); a carve
  union (an L, one well with one wall); a lattice and grout; a frame (a plate
  whose opening has cove corners, the designer's playbar shelf, its unrolled
  outer edges run past the tile's view as past the window);
- round ones: a disc plate (`plate_shaped` at `Some(2.0)`, the shape of a Ramp's
  key peg, on the bevel's face so its roll reads plainly),
  a sphere (a RadioGroup's bead), and a droplet (`droplet`, the status bar's
  drop at `DropletSpec::default()`) hanging over stripes;
- two series across two grid columns, captioned under each sample: plates of
  three materials over stripes (opaque, translucent, frosted), and one recess
  at 2, 4 and 8 px walls.

The tiles are exhibits like the widgets: captioned by the adapter's detached
label, arranged by the `Layout` dropdown, scrolled and cut at the wall's foot
the same way. A switch opens the page at its top, closes the context menu and
first ends any press or drag held on the page it leaves (a left release off
every widget, so a held button lets go without clicking), so no gesture
carries over onto a hidden exhibit. The other page's exhibits are
parked off-screen and hidden, so they are not painted, take no pointer and are
no Tab stops.

The `Style` dropdown changes only the effects a widget draws a flat form of
with relief off, and they take that form: the pane plate (an InfoBox's themed
surface in a hairline frame), the bevel (a bordered plate), the control plates
(the Flat stance: the face alone, the focus ring a hairline), the inset plate,
the recesses and the fields (the hairline every well falls back to, a field's
run a lit face), the canvas well, the section well (its outline parted for the
title), the grooves (the Separator's rule), the frame (no roll) and the disc
plate (a flat disc of its face). The boss, trough, ridge, concave fillet, carve union,
lattice, grout, sphere, droplet and materials have no flat form, and are drawn
the same under both styles. What every plate in a frame shares — the light's
direction and strength, the roll's width and profile — can only change over
time, not side by side, so the page does not show it; nor does it show the
droplet's scrim (`droplet_scrim`, a flat vignette in the drop's shape, not a
lit surface), or draw anything on the legacy (non-SDF) shader path, where the
lattice, grout and carve union draw nothing.

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
  roster of 36 named slots (`GallerySlots`) followed by the variant exhibits
  (`variant_exhibits`) and the Relief page's tiles (`relief_tiles`), each a
  `Vec<Exhibit>` of widgets the context owns, all
  addressed by numeric index in the layout tables, visibility filters and
  dispatch loops; child windows use `ChildSlots` — five slots for a test
  window, three for an editor.
  `demo_positions` and `child_positions` are the layout tables for the chrome,
  `exhibit_table` the widget exhibits' sizes, `layout_exhibits` the placement
  of the page's exhibits (and the parking of the other page's).
  What the widgets report reaches the app in one sweep after every pointer,
  wheel, key and tick dispatch (`drain_widget_events`, over cce-ui's
  `UiContext::take_events`), so a keyboard activation acts at once.
- `src/arrange.rs` — the exhibit arrangements the `Layout` dropdown offers, as
  pure functions (`arrange`, which takes the Group lassos as blocks too), and
  their unit tests.
- `src/gallery_widgets.rs` — lookalikes of the retired toolkit `Plate` (an
  exhibit) and root plate (the child windows' background), and the Relief
  page's tiles (`ReliefTile`, one per `Effect`).

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
