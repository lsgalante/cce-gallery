use cce_ui::process::spawn_detached;
use cce_ui::context::UiContext;
use cce_ui::widget::{Handle, WidgetId};
use cce_ui::widget::{Button, Checkbox, ContentBg, Dropdown, Label, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar, Toggle, WidgetHost, Trackpad, TextBox, MenuBar, Group, Ramp, RampKey, ColorRamp, ColorPalette, MouseButton, ElementState, Key, NamedKey, KeyEvent, MouseScrollDelta, ColorSelector, FontSelector, KeybindRecorder, ButtonStrip, Float3, UsageBar, StatusDot, DotStatus, InfoBox, InteractiveListItem, Breadcrumb, TreeList, BevelPreview, RampPreview, Separator, Splitter, ScrollBox, SliderSpinbox, WidgetHostExt};
mod arrange;
use arrange::{arrange_sections, Arrangement, Block, Lasso};
mod gallery_widgets;
use gallery_widgets::{Effect, Plate, ReliefTile, RootPlate};
use cce_ui::widget::{Adapted, LayoutConstraints, Point, WidgetEvent};
use cce_ui::widget::input::Slider2D;
use cce_ui::engine::{LogicalSize, LogicalPosition, LayerAnchor, LayerKeyboardInteractivity, LayerKind, LayerSettings};
use cce_ui::scene::paint::Prim;

/// What a child window is, from `--type`. The first seven are the kinds of surface a
/// toolkit client can be under cce; the last two are the gallery's editor windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildKind {
    Floating,
    Fullscreen,
    Utility,
    LayerTop,
    LayerOverlay,
    LayerBackground,
    Status,
    Ramp,
    ColorRamp,
}

impl ChildKind {
    fn from_arg(s: &str) -> Option<ChildKind> {
        Some(match s {
            "Floating" => ChildKind::Floating,
            "Fullscreen" => ChildKind::Fullscreen,
            "Utility" => ChildKind::Utility,
            "LayerTop" => ChildKind::LayerTop,
            "LayerOverlay" => ChildKind::LayerOverlay,
            "LayerBackground" => ChildKind::LayerBackground,
            "Status" => ChildKind::Status,
            "Ramp" => ChildKind::Ramp,
            "ColorRamp" => ChildKind::ColorRamp,
            _ => return None,
        })
    }

    /// The `--type` spelling.
    fn arg(self) -> &'static str {
        match self {
            ChildKind::Floating => "Floating",
            ChildKind::Fullscreen => "Fullscreen",
            ChildKind::Utility => "Utility",
            ChildKind::LayerTop => "LayerTop",
            ChildKind::LayerOverlay => "LayerOverlay",
            ChildKind::LayerBackground => "LayerBackground",
            ChildKind::Status => "Status",
            ChildKind::Ramp => "Ramp",
            ChildKind::ColorRamp => "ColorRamp",
        }
    }

    fn title(self) -> &'static str {
        match self {
            ChildKind::Floating => "Floating Window",
            ChildKind::Fullscreen => "Fullscreen Window",
            ChildKind::Utility => "Utility Window",
            ChildKind::LayerTop => "Layer Shell (Top) Surface",
            ChildKind::LayerOverlay => "Layer Shell (Overlay) Surface",
            ChildKind::LayerBackground => "Layer Shell (Background) Surface",
            ChildKind::Status => "Status Segment",
            ChildKind::Ramp => "Ramp Editor",
            ChildKind::ColorRamp => "Color Ramp Editor",
        }
    }

    fn is_editor(self) -> bool {
        matches!(self, ChildKind::Ramp | ChildKind::ColorRamp)
    }

    fn default_size(self) -> (f32, f32) {
        match self {
            ChildKind::Floating | ChildKind::Fullscreen => (400.0, 250.0),
            ChildKind::Utility | ChildKind::LayerOverlay => (300.0, 180.0),
            ChildKind::LayerTop => (800.0, 40.0),
            ChildKind::LayerBackground => (800.0, 600.0),
            ChildKind::Status => (140.0, 24.0),
            // Each editor is as tall as the window's chrome (the title line, the root gaps
            // and insets, Close) and its ramp's own height: the Ramp's graph whole over its
            // control strip and key pad (a fixed height left the graph whatever the strip did
            // not take), the ColorRamp's track, pegs and colour selector row (a fixed 390,
            // sized for the slider rows it had, stood mostly empty).
            ChildKind::Ramp | ChildKind::ColorRamp => {
                let probe = 390.0;
                let chrome = probe - child_positions(450.0, probe, false, false)[1].3;
                let ramp = match self {
                    ChildKind::Ramp => Ramp::expanded_height(),
                    _ => ColorRamp::new().natural_height(),
                };
                (450.0, (chrome + ramp).ceil())
            }
        }
    }

    /// `cce-status-*` is the compositor's status-bar convention (`WindowRole::from_app_id`),
    /// with the edge read from the `-left-` / `-right-` infix; everything else is ours.
    fn app_id(self) -> String {
        match self {
            ChildKind::Status => "cce-status-right-gallery".to_string(),
            kind => format!("cce-gallery-child-{}", kind.arg().to_lowercase()),
        }
    }

    /// The wlr-layer-shell configuration for the three layer kinds.
    fn layer(self, height: i32) -> Option<LayerSettings> {
        let (layer, anchor, exclusive_zone) = match self {
            ChildKind::LayerTop => (LayerKind::Top, LayerAnchor::TOP | LayerAnchor::LEFT | LayerAnchor::RIGHT, height),
            ChildKind::LayerOverlay => (LayerKind::Overlay, LayerAnchor::empty(), 0),
            ChildKind::LayerBackground => (
                LayerKind::Background,
                LayerAnchor::TOP | LayerAnchor::BOTTOM | LayerAnchor::LEFT | LayerAnchor::RIGHT,
                -1,
            ),
            _ => return None,
        };
        Some(LayerSettings {
            layer,
            anchor,
            exclusive_zone,
            keyboard_interactivity: LayerKeyboardInteractivity::None,
            margin: (0, 0, 0, 0),
            namespace: "cce-gallery".to_string(),
        })
    }

    /// The line inside the child window.
    fn child_text(self) -> &'static str {
        match self {
            ChildKind::Floating => "A plain xdg_toplevel, mapped Floating by cce.",
            ChildKind::Fullscreen => "An xdg_toplevel that asked for fullscreen at map.",
            ChildKind::Utility => "A toplevel declared UTILITY over the cce protocol.",
            ChildKind::LayerTop => "A Top-layer wlr-layer-shell surface with an exclusive zone.",
            ChildKind::LayerOverlay => "An Overlay-layer wlr-layer-shell surface, unanchored.",
            ChildKind::LayerBackground => "A Background-layer wlr-layer-shell surface.",
            ChildKind::Status => "A cce-status-* toplevel, docked into the status bar.",
            ChildKind::Ramp | ChildKind::ColorRamp => "",
        }
    }
}

/// What the roster's visibility filter needs, copied out of `State` so the dispatch
/// loops can hold `&mut self.roster` while asking. The gallery shows its header and the
/// current page's exhibits — the lassos are the widget page's; a child window shows its
/// menu bar and status bar only when asked to.
#[derive(Clone, Copy)]
struct Visibility {
    is_child: bool,
    use_menubar: bool,
    use_statusbar: bool,
    page: Page,
    /// The first Relief-page tile's slot (`GallerySlots::tiles_start`).
    tiles_start: usize,
}

impl Visibility {
    fn is_visible(self, index: usize) -> bool {
        if self.is_child {
            return match index {
                0..=2 => true, // background, main, Close
                3 => self.use_menubar,
                4 => self.use_statusbar,
                _ => false,
            };
        }
        if is_exhibit(index) || is_overlay(index) {
            return (index >= self.tiles_start) == (self.page == Page::Relief);
        }
        true
    }
}

/// The gallery's pages, as the header's Page dropdown lists them: the widgets, one of each
/// in each of its styles, and the toolkit's relief effects, one tile each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Widgets,
    Relief,
}

/// The Page dropdown's entries, in order; the gallery opens on the first.
const PAGES: [(&str, Page); 2] = [("Widgets", Page::Widgets), ("Relief", Page::Relief)];

/// The gallery roster: 37 named slots. The numeric indexes used by the positions
/// table, the visibility filter and the dispatch loops address these slots through
/// `Roster::id` / `get_dyn` / `get_dyn_mut`.
pub struct GallerySlots {
    /// The Style dropdown (slot 0), beside Layout in the header: Relief or Flat
    /// for every control at once (`cce_ui::layout::set_control_relief`).
    pub style_dd: Handle<Adapted<Dropdown>>,
    pub button_demo: Handle<Adapted<Button>>,
    pub checkbox_demo: Handle<Adapted<Checkbox>>,
    pub toggle_demo: Handle<Adapted<Toggle>>,
    pub progress_demo: Handle<Adapted<ProgressBar>>,
    pub slider_demo: Handle<Adapted<Slider>>,
    pub spinbox_demo: Handle<Adapted<Spinbox>>,
    pub range_slider_demo: Handle<Adapted<RangeSlider>>,
    pub trackpad_demo: Handle<Adapted<Trackpad>>,
    pub textbox_demo: Handle<Adapted<TextBox>>,
    pub plate_demo: Handle<Adapted<Plate>>,
    pub color_ramp_btn: Handle<Adapted<Button>>,
    pub bevel_ramp: Handle<Adapted<Ramp>>,
    pub ramp_btn: Handle<Adapted<Button>>,
    pub layout_dd: Handle<Adapted<Dropdown>>,
    pub color_selector_demo: Handle<Adapted<ColorSelector>>,
    pub font_selector_demo: Handle<Adapted<FontSelector>>,
    pub keybind_demo: Handle<Adapted<KeybindRecorder>>,
    pub button_strip_demo: Handle<Adapted<ButtonStrip>>,
    pub slider2d_demo: Handle<Adapted<Slider2D>>,
    pub float3_demo: Handle<Adapted<Float3>>,
    pub usage_bar_demo: Handle<Adapted<UsageBar>>,
    pub status_dot_demo: Handle<Adapted<StatusDot>>,
    pub info_box_demo: Handle<Adapted<InfoBox>>,
    pub list_item_demo: Handle<Adapted<InteractiveListItem>>,
    pub breadcrumb_demo: Handle<Adapted<Breadcrumb>>,
    pub tree_list_demo: Handle<Adapted<TreeList>>,
    pub bevel_preview_demo: Handle<Adapted<BevelPreview>>,
    pub ramp_preview_demo: Handle<Adapted<RampPreview>>,
    pub separator_demo: Handle<Adapted<Separator>>,
    pub splitter_demo: Handle<Adapted<Splitter>>,
    /// Two `Group` lassos over other exhibits (slots 31 and 32): overlays, not
    /// exhibits — each arranged as one block around its members (`GROUPS`), drawn under
    /// the exhibit clip.
    pub group_loose: Handle<Adapted<Group>>,
    pub group_fitted: Handle<Adapted<Group>>,
    /// Slot 33, after the lassos: a widget newer than the rest of the roster.
    pub slider_spinbox_demo: Handle<Adapted<SliderSpinbox>>,
    /// Slot 34: the ColorRamp, embedded as the Ramp is (slot 12); `Color Ramp...`
    /// opens one in a window of its own.
    pub color_ramp_demo: Handle<Adapted<ColorRamp>>,
    /// The Page dropdown (slot 35), after Style in the header: which page's exhibits the
    /// area below shows (`PAGES`).
    pub page_dd: Handle<Adapted<Dropdown>>,
    /// Slot 36, after Page: the ColorPalette, an editable row of swatches (the ColorRamp's
    /// discrete kin), embedded as the ColorRamp is.
    pub color_palette_demo: Handle<Adapted<ColorPalette>>,
    /// The variant exhibits (`variant_exhibits`): every further style of a widget one of
    /// the named slots already shows, addressed as slots `GALLERY_COUNT..`.
    pub extra: Vec<Exhibit>,
    /// The Relief page's tiles (`relief_tiles`), one per effect, addressed as the slots
    /// after the variants' (`tiles_start..`).
    pub tiles: Vec<Exhibit>,
}

/// The number of NAMED gallery slots; the variant exhibits follow them, then the Relief
/// page's tiles, so the roster's `len` is this plus `GallerySlots::extra.len()` plus
/// `GallerySlots::tiles.len()`.
pub const GALLERY_COUNT: usize = 37;

/// The Page dropdown's slot, the last named one.
const PAGE_SLOT: usize = 35;

/// The header dropdowns: Layout, Style and Page, left to right.
const HEADER: [usize; 3] = [14, 0, PAGE_SLOT];

/// The header dropdowns' width.
const HEADER_W: f32 = 190.0;

/// A variant exhibit: a widget in a style other than the one its named slot shows, with
/// the content size `layout_exhibits` resets it to.
pub struct Exhibit {
    /// The widget, which the context owns.
    pub id: WidgetId,
    pub w: f32,
    pub h: f32,
    /// Whether Vertical and Adaptive Grid may widen it to their cell ([`ExhibitSize::stretch`]).
    pub stretch: bool,
    /// How many of Adaptive Grid's columns it lies across ([`ExhibitSize::span`]).
    pub span: usize,
    /// The named slot it is placed right after (with any other variant of that slot, in
    /// the order they are built), so a variant sits beside the widget it is a look of
    /// (`exhibit_table`). `None` for the Relief page's tiles, which have no named slot.
    pub after: Option<usize>,
}

impl Exhibit {
    /// Sized by the widget's own preferred (content) height, `fallback_h` when it declares none.
    fn new<W: WidgetHost + 'static>(ctx: &mut UiContext, widget: W, w: f32, fallback_h: f32) -> Self {
        let h = widget.preferred_height().unwrap_or(fallback_h);
        Exhibit { id: ctx.insert(widget).id(), w, h, stretch: true, span: 1, after: None }
    }

    /// Sized by hand: for widgets whose declared height is a single row (a multiline
    /// TextBox, a vertical ButtonStrip) when the exhibit wants several.
    fn sized<W: WidgetHost + 'static>(ctx: &mut UiContext, widget: W, w: f32, h: f32) -> Self {
        Exhibit { id: ctx.insert(widget).id(), w, h, stretch: true, span: 1, after: None }
    }

    /// Across `span` of Adaptive Grid's columns.
    fn spanning(mut self, span: usize) -> Self {
        self.span = span;
        self
    }

    /// Kept at its own width in every arrangement: a control with a size of its own.
    fn fixed(mut self) -> Self {
        self.stretch = false;
        self
    }

    /// Placed right after named slot `slot` ([`Exhibit::after`]).
    fn after(mut self, slot: usize) -> Self {
        self.after = Some(slot);
        self
    }
}

/// A SliderSpinbox exhibit's width: a control row's, as every other control exhibit's is —
/// the box and its gap, and an 82 px band. A band as long as a plain slider's (298 px in
/// all) was the widest exhibit on the page, and the grid, which never lets its widest
/// exhibit run into the next column, dropped columns for it.
const SLIDER_SPINBOX_W: f32 = 190.0;

/// An editor exhibit as the page shows it, a key (or swatch) in hand: the Ramp, ColorRamp and
/// ColorPalette each reserve the rows of the controls that edit a selected key whether one is
/// selected or not, so their rect never changes as one is, and with none selected those rows
/// were blank — the space under them the page's families then stood below. Selected, the
/// rows show the controls they are for.
fn with_key_in_hand<W: cce_ui::widget::Layout + cce_ui::widget::Paint + cce_ui::widget::Input>(mut editor: Adapted<W>, select: impl FnOnce(&mut W)) -> Adapted<W> {
    select(editor.inner_mut());
    editor
}

/// An exhibit's slot and the content size it is reset to before it is arranged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExhibitSize {
    pub slot: usize,
    pub w: f32,
    pub h: f32,
    /// Whether Vertical and Adaptive Grid may widen it to their cell. A control with a size
    /// of its own — the Splitter's grip, a StatusDot, the Slider2D's square pad, the
    /// Toggle's switch, an icon button — keeps `w` there, as it does under Mosaic.
    pub stretch: bool,
    /// Narrowed to its content: `w` at most, else where the label it draws inside its own
    /// rect ends ([`content_extent`]) — the Checkbox, a box and its label at the left of
    /// whatever rect it is given, which drew a frame 190 px wide round a 90 px control.
    pub hug: bool,
    /// How many of Adaptive Grid's columns it lies across ([`Block::span`]).
    pub span: usize,
    /// Arranged at `h` rather than at the height the widget declares, which otherwise
    /// wins (`Block::of`): a canvas the page shows larger than its default. The Ramp
    /// declares a 120 px graph over its control strip, and landed taller it gives the
    /// graph the rest (`Ramp::graph_h`) — so too with its controls collapsed from its
    /// context menu, when the graph takes the strip's room and the exhibit keeps its size.
    pub own_h: bool,
}

/// The Checkbox's slot: the one exhibit narrowed to its content ([`ExhibitSize::hug`]).
const CHECKBOX_SLOT: usize = 2;

/// The TreeList's slot: across two of Adaptive Grid's columns ([`ExhibitSize::span`]).
/// One column cut its key and type columns short.
const TREELIST_SLOT: usize = 26;

/// The Ramp's slot: across two of Adaptive Grid's columns, like the TreeList, and the one
/// exhibit sized here past its toolkit default ([`ExhibitSize::own_h`]). In one column at
/// its default height the curve was a 120 px strip its pegs crowded.
const RAMP_SLOT: usize = 12;

/// The ColorRamp's slot: across two of Adaptive Grid's columns, like the TreeList, for a
/// gradient longer than a control row and room for its colour selector's hex beside Delete.
const COLOR_RAMP_SLOT: usize = 34;

/// The ColorPalette's slot: across two of Adaptive Grid's columns, as the ColorRamp is, so its
/// strip holds its six swatches and the add slot at full spacing.
const COLOR_PALETTE_SLOT: usize = 36;

/// Whether an exhibit lies across two of Adaptive Grid's columns ([`ExhibitSize::span`]).
fn spans_two(slot: usize) -> bool {
    matches!(slot, TREELIST_SLOT | RAMP_SLOT | COLOR_RAMP_SLOT | COLOR_PALETTE_SLOT)
}

/// How far from its rect's left edge a control's drawn content reaches: the end of the
/// furthest text it paints (`painted_prims`), measured in its font, plus a pixel so the clip
/// it draws its label under never cuts the last glyph. `None` when it paints no text.
fn content_extent(w: &dyn WidgetHost) -> Option<f32> {
    let (x, ..) = w.rect();
    let (family, _) = cce_ui::layout::control_label_font_parsed();
    w.painted_prims()
        .into_iter()
        .filter_map(|p| match p {
            Prim::Text { text, x: tx, font_size, font, .. } => {
                let family = font.unwrap_or_else(|| family.clone());
                Some(tx + cce_ui::widget::display::measure_text_width(&text, &family, font_size) - x)
            }
            _ => None,
        })
        .reduce(f32::max)
        .map(|w| w.ceil() + 1.0)
}

/// The StatusDot's three variants' slots (`variant_exhibits`: the last three, after the
/// named slot's dot), which the loose lasso draws around with it.
/// `the_loose_lasso_holds_the_status_dots` holds the indexes to the list.
const STATUS_DOT_VARIANTS: [usize; 3] = [GALLERY_COUNT + 13, GALLERY_COUNT + 14, GALLERY_COUNT + 15];

/// The `Group` lassos (slots 31, 32) and the exhibit slots each draws around, in the order
/// they are laid out inside it: a loose one around the four StatusDots, and the fitted one
/// around the top row (Button, ButtonStrip, Checkbox, Toggle). Each lies within one family
/// (`FAMILY_HEADS`), so no frame straddles two.
const GROUPS: [(usize, &[usize]); 2] = [
    (31, &[22, STATUS_DOT_VARIANTS[0], STATUS_DOT_VARIANTS[1], STATUS_DOT_VARIANTS[2]]),
    (32, &[1, 18, 2, 3]),
];

/// The first exhibit of each family of the widget page (`exhibit_table`): each starts a new
/// section of the arrangement (`arrange_sections`), below everything before it, so the
/// families never interleave. Buttons, sliders and number inputs, colour, the ramps, text,
/// status and display, surfaces.
const FAMILY_HEADS: [usize; 7] = [1, 5, 15, 13, 9, 22, 10];

/// The lassos to arrange over blocks whose slots are `slots`, in order: each group's members
/// as indexes into the blocks, with the group's (padding, tab height) from `frame`. A member
/// with no block is left out, and a group with none is dropped.
fn lassos(slots: &[usize], frame: impl Fn(usize) -> (f32, f32)) -> Vec<Lasso> {
    GROUPS
        .iter()
        .filter_map(|&(group, members)| {
            let members: Vec<usize> = members.iter().filter_map(|m| slots.iter().position(|s| s == m)).collect();
            let (padding, tab) = frame(group);
            (!members.is_empty()).then_some(Lasso { members, padding, tab })
        })
        .collect()
}

impl GallerySlots {
    /// The `Group` lasso in slot `idx` (31 or 32).
    pub fn group(&self, idx: usize) -> Handle<Adapted<Group>> {
        match idx {
            31 => self.group_loose,
            32 => self.group_fitted,
            _ => panic!("gallery slot {idx} is not a Group"),
        }
    }

    /// The first Relief-page tile's slot: the one after the last variant's.
    pub fn tiles_start(&self) -> usize {
        GALLERY_COUNT + self.extra.len()
    }

    /// The widget in slot `idx`.
    pub fn id(&self, idx: usize) -> WidgetId {
        if idx >= self.tiles_start() {
            return self.tiles[idx - self.tiles_start()].id;
        }
        if idx >= GALLERY_COUNT {
            return self.extra[idx - GALLERY_COUNT].id;
        }
        match idx {
            0 => self.style_dd.id(),
            1 => self.button_demo.id(),
            2 => self.checkbox_demo.id(),
            3 => self.toggle_demo.id(),
            4 => self.progress_demo.id(),
            5 => self.slider_demo.id(),
            6 => self.spinbox_demo.id(),
            7 => self.range_slider_demo.id(),
            8 => self.trackpad_demo.id(),
            9 => self.textbox_demo.id(),
            10 => self.plate_demo.id(),
            11 => self.color_ramp_btn.id(),
            12 => self.bevel_ramp.id(),
            13 => self.ramp_btn.id(),
            14 => self.layout_dd.id(),
            15 => self.color_selector_demo.id(),
            16 => self.font_selector_demo.id(),
            17 => self.keybind_demo.id(),
            18 => self.button_strip_demo.id(),
            19 => self.slider2d_demo.id(),
            20 => self.float3_demo.id(),
            21 => self.usage_bar_demo.id(),
            22 => self.status_dot_demo.id(),
            23 => self.info_box_demo.id(),
            24 => self.list_item_demo.id(),
            25 => self.breadcrumb_demo.id(),
            26 => self.tree_list_demo.id(),
            27 => self.bevel_preview_demo.id(),
            28 => self.ramp_preview_demo.id(),
            29 => self.separator_demo.id(),
            30 => self.splitter_demo.id(),
            31 => self.group_loose.id(),
            32 => self.group_fitted.id(),
            33 => self.slider_spinbox_demo.id(),
            34 => self.color_ramp_demo.id(),
            PAGE_SLOT => self.page_dd.id(),
            COLOR_PALETTE_SLOT => self.color_palette_demo.id(),
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }
}

/// A child window's slots — background, main, Close, and for a simulated window the
/// optional menu bar and status bar rows (3, 4) — as the ids of the widgets the context
/// owns. An editor has the first three.
pub struct ChildSlots {
    pub ids: Vec<WidgetId>,
    pub close: Handle<Adapted<Button>>,
}

/// The most slots a child window has (a simulated window's).
pub const CHILD_COUNT: usize = 5;

/// The two roster modes. The widgets are the context's; a roster names them by slot.
pub enum Roster {
    Gallery(Box<GallerySlots>),
    Child(Box<ChildSlots>),
}

impl Roster {
    // No `is_empty`: both modes have fixed slots, so a roster is never empty.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Roster::Gallery(s) => s.tiles_start() + s.tiles.len(),
            Roster::Child(s) => s.ids.len(),
        }
    }

    /// The widget in slot `idx`.
    pub fn id(&self, idx: usize) -> WidgetId {
        match self {
            Roster::Gallery(s) => s.id(idx),
            Roster::Child(s) => *s.ids.get(idx).unwrap_or_else(|| panic!("child slot index out of range: {idx}")),
        }
    }

    pub fn get_dyn<'a>(&self, ui: &'a UiContext, idx: usize) -> &'a (dyn WidgetHost + 'static) {
        ui.get_widget(self.id(idx)).expect("a roster slot's widget is in the context")
    }

    pub fn get_dyn_mut<'a>(&self, ui: &'a mut UiContext, idx: usize) -> &'a mut (dyn WidgetHost + 'static) {
        ui.get_widget_mut(self.id(idx)).expect("a roster slot's widget is in the context")
    }

    /// A slot that is never dragged: the variant exhibits and the Relief page's tiles are
    /// looked at, and the lassos (31, 32) are frames.
    fn inert(&self, idx: usize) -> bool {
        matches!(self, Roster::Gallery(_)) && (idx >= GALLERY_COUNT || matches!(idx, 31 | 32))
    }

    pub fn draggable(&self, ui: &UiContext, idx: usize) -> bool {
        if self.inert(idx) {
            return false;
        }
        let w = self.get_dyn(ui, idx);
        w.input_model().draggable(w.content_rect())
    }

    pub fn is_dragging(&self, ui: &UiContext, idx: usize) -> bool {
        !self.inert(idx) && self.get_dyn(ui, idx).input_model().is_dragging()
    }

    /// The gallery slots; panics in child mode (gallery-only paths assert their mode).
    pub fn gallery(&self) -> &GallerySlots {
        match self {
            Roster::Gallery(s) => s,
            Roster::Child(_) => panic!("gallery slots requested in child mode"),
        }
    }

    /// The selected entry of a header dropdown (14 Layout, 0 Style, 35 Page).
    pub fn value(&self, ui: &UiContext, idx: usize) -> i32 {
        let s = self.gallery();
        match idx {
            14 => ui[s.layout_dd].value(),
            0 => ui[s.style_dd].value(),
            PAGE_SLOT => ui[s.page_dd].value(),
            _ => panic!("value: unwired gallery slot {idx}"),
        }
    }
}

struct State {
    roster: Roster,
    positions: Vec<(f32, f32, f32, f32)>,

    width: f32,
    height: f32,
    scale: f64,

    is_child: bool,
    opacity: bool,
    transparency: f32,
    use_root_plate: bool,
    use_menubar: bool,
    use_statusbar: bool,
    border_enabled: bool,
    child_kind: Option<ChildKind>,
    ui_context: cce_ui::context::UiContext,

    layout_idx: usize,
    /// The page the exhibit area shows (the Page dropdown).
    page: Page,
    /// The scroll frame the exhibits below the header scroll inside.
    exhibit_scroll: ScrollBox,
    /// Posts to `update`: how a hook that returns no message (tick, pointer motion, wheel)
    /// hands one on.
    sender: cce_ui::engine::AppSender<String>,
    /// A Ramp editor's curve as the bevel ramp file last had it (`bevel_ramp_kdl`): a
    /// tick reports focus changes and inertia as well as edits, and only a curve that
    /// differs is written.
    saved_ramp: Option<String>,
}

/// `~/.config/cce/bevel_ramp.kdl`: the Ramp editor's curve, kept between runs. Nothing
/// else reads it.
fn bevel_ramp_path() -> std::path::PathBuf {
    cce_ui::config::get_config_path().parent().unwrap().join("bevel_ramp.kdl")
}

/// The `line_type` a Ramp's line-type dropdown selects.
fn ramp_line_type(ramp: &Ramp) -> &'static str {
    match ramp.line_type_dropdown.selected {
        1 => "bezier",
        _ => "linear",
    }
}

/// The bevel ramp file's contents for `keys` and `line_type`.
fn bevel_ramp_kdl(keys: &[RampKey], line_type: &str) -> String {
    let mut s = String::new();
    s.push_str("keys {\n");
    for k in keys {
        s.push_str(&format!("    key pos={} val={}\n", k.pos, k.value));
    }
    s.push_str("}\n");
    s.push_str(&format!("line_type \"{}\"\n", line_type));
    s
}

/// Replace the bevel ramp file whole: written beside it, then renamed over it, so a
/// reader never finds it truncated or half-written.
fn save_bevel_ramp(contents: &str) -> std::io::Result<()> {
    let path = bevel_ramp_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("kdl.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, &path)
}

/// The Ramp editor's curve from the bevel ramp file, else the toolkit Ramp's default.
fn load_bevel_ramp() -> (Vec<RampKey>, String) {
    std::fs::read_to_string(bevel_ramp_path())
        .ok()
        .and_then(|content| parse_bevel_ramp(&content))
        .unwrap_or_else(|| {
            (
                vec![
                    RampKey { pos: 0.0, value: 0.5 },
                    RampKey { pos: 0.2, value: 1.0 },
                    RampKey { pos: 0.8, value: 1.0 },
                    RampKey { pos: 1.0, value: 0.5 },
                ],
                "linear".to_string(),
            )
        })
}

/// A bevel ramp file's keys, sorted by position, and its line type; `None` when it
/// holds no key.
fn parse_bevel_ramp(content: &str) -> Option<(Vec<RampKey>, String)> {
    let mut line_type = "linear".to_string();
    let mut keys = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("key ") {
            let mut pos: f32 = 0.0;
            let mut val: f32 = 0.5;
            for part in line.split_whitespace() {
                if let Some(rest) = part.strip_prefix("pos=") {
                    pos = rest.parse().unwrap_or(0.0);
                } else if let Some(rest) = part.strip_prefix("val=") {
                    val = rest.parse().unwrap_or(0.5);
                }
            }
            // A key the file cannot place (`pos=nan`) is dropped, not sorted.
            if pos.is_finite() && val.is_finite() {
                keys.push(RampKey { pos, value: val });
            }
        } else if line.starts_with("line_type ") {
            if let Some(val_str) = line.split_whitespace().nth(1) {
                line_type = val_str.trim_matches('"').to_string();
            }
        }
    }
    keys.sort_by(|a, b| a.pos.total_cmp(&b.pos));
    (!keys.is_empty()).then_some((keys, line_type))
}

/// Every further style of a widget the named slots show once: the toolkit's own
/// variants — a constructor (`Button::new_reset`) or a builder (`with_readout`) —
/// so the page shows each look a widget can take, in the toolkit's default size
/// for it.
///
/// The relief-off look is NOT a variant here: the header's Style dropdown switches
/// every control between Relief and Flat at once (`set_control_relief`), so each
/// exhibit shows both, and no widget appears twice for its style alone.
fn variant_exhibits(ctx: &mut UiContext) -> Vec<Exhibit> {
    const W: f32 = 190.0;
    let bh = cce_ui::layout::button_height();
    let slh = cce_ui::layout::slider_height();
    let tbh = cce_ui::layout::textbox_height();
    let ddh = cce_ui::layout::dropdown_height();
    let csh = cce_ui::layout::color_selector_height();
    // The sidebar width a vertical strip is drawn for.
    const TAB_COLUMN_W: f32 = 40.0;
    let three = || vec!["One".to_string(), "Two".to_string(), "Three".to_string()];
    // A column of three rotated tabs, each as long as its label: the height a vertical
    // strip of them takes (it declares none, so a fixed height cut the last tab short).
    let tabs_h = {
        let strip = ButtonStrip::new(0.0, 0.0, TAB_COLUMN_W, 0.0).with_buttons(three()).with_vertical(true);
        let (_, y, _, h) = strip.item_rect(2);
        y + h
    };
    vec![
        // Button: the other four kinds.
        Exhibit::new(ctx, Button::new_reset(0.0, 0.0, W, bh).with_label("Button (reset)"), W, bh).after(1),
        Exhibit::new(ctx, Button::new_list_row(0.0, 0.0, W, bh).with_label("Button (list row)"), W, bh).after(1),
        Exhibit::new(ctx, Button::new_menu_item(0.0, 0.0, W, bh).with_label("Button (menu item)"), W, bh).after(1),
        Exhibit::new(ctx, Button::new_copy_icon(0.0, 0.0, bh, bh), bh, bh).fixed().after(1),
        // Slider: with a readout.
        Exhibit::new(ctx, Slider::new().with_label("Slider (readout)").with_readout(true), W, slh).after(5),
        // SliderSpinbox: a float to two decimals, stepped by 0.05 (the named slot is an int).
        Exhibit::new(ctx, SliderSpinbox::new_float(0.5, 0.0, 1.0, 0.05, 2).with_label("SliderSpinbox (float)"), SLIDER_SPINBOX_W, slh).after(33),
        // TextBox: multiline, chromeless, password.
        Exhibit::sized(
            ctx,
            TextBox::new("TextBox (multiline)\nA second line of text.".to_string()).with_multiline(true),
            W,
            3.0 * tbh,
        )
        .after(9),
        Exhibit::new(ctx, TextBox::new("TextBox (chromeless)".to_string()).with_draw_bg_border(false), W, tbh).after(9),
        Exhibit::new(ctx, TextBox::new("hunter2".to_string()).with_password(true).with_label("TextBox (password)"), W, tbh).after(9),
        // ButtonStrip: the vertical column (rotated tabs).
        Exhibit::sized(
            ctx,
            Adapted::new(ButtonStrip::new(0.0, 0.0, W, tabs_h).with_buttons(three()).with_selected(Some(0)).with_vertical(true))
                .with_label("ButtonStrip (vertical)"),
            TAB_COLUMN_W,
            tabs_h,
        )
        .fixed()
        .after(18),
        // ColorSelector: the alpha swatch.
        Exhibit::new(ctx, ColorSelector::new_rgba([64, 128, 255, 128]).with_label("ColorSelector (alpha)"), W, csh).after(15),
        // InteractiveListItem: selected, its highlight wash (the named slot's shows only
        // under the pointer).
        Exhibit::sized(
            ctx,
            {
                // The style rides in the subtitle: "<Widget> (selected)" as one title
                // line is wider than the exhibit, and the row clips its text.
                let mut row = InteractiveListItem::new("InteractiveListItem").with_subtitle("(selected)");
                row.selected = true;
                row
            },
            W,
            2.0 * ddh,
        )
        .after(24),
        // Label: the plain text widget.
        Exhibit::new(ctx, Label::new("Label"), W, ddh).after(9),
        // StatusDot: the other three statuses. A dot is 12px square; its label is wider,
        // and the arrangement reserves the label's width (`Block::of`).
        Exhibit::new(ctx, StatusDot::new(DotStatus::Inactive).with_label("StatusDot (inactive)"), StatusDot::SIZE, StatusDot::SIZE).fixed().after(22),
        Exhibit::new(ctx, StatusDot::new(DotStatus::Warning).with_label("StatusDot (warning)"), StatusDot::SIZE, StatusDot::SIZE).fixed().after(22),
        Exhibit::new(ctx, StatusDot::new(DotStatus::Error).with_label("StatusDot (error)"), StatusDot::SIZE, StatusDot::SIZE).fixed().after(22),
    ]
}

/// The Controls exhibits in layout order, each with the content size it is reset to
/// before it is arranged: an exhibit's block is read off its own rect (`Block::of`) after
/// the reset, and the previous frame's landing may have stretched it. `s2d_w` is the
/// Slider2D's measured width, `crh` and `cph` the ColorRamp's and the ColorPalette's declared
/// heights; `extra` the variant exhibits, which follow the named slots.
fn exhibit_table(s2d_w: f32, crh: f32, cph: f32, extra: &[Exhibit]) -> Vec<ExhibitSize> {
    // Every height below but the Ramp's is the toolkit's default for that
    // control — its configured `style.control.<name>.height`, or the intrinsic
    // size the widget declares — so the page is a record of the defaults, not
    // of numbers chosen here. A height the widget declares wins over the
    // table's (`Block::of`), so for those the table only has to agree with it.
    // The Ramp is the exception (`ExhibitSize::own_h`): it declares a 120 px
    // graph, a strip sized for a host's row, and the page draws the curve at
    // twice that, so its height here wins and the extra goes to the graph.
    let bh = cce_ui::layout::button_height();
    let tgh = cce_ui::layout::toggle_height();
    let slh = cce_ui::layout::slider_height();
    let rsh = cce_ui::layout::rangeslider_height();
    let pbh = cce_ui::layout::progressbar_height();
    let sph = cce_ui::layout::spinbox_height();
    let ddh = cce_ui::layout::dropdown_height();
    let tbh = cce_ui::layout::textbox_height();
    let csh = cce_ui::layout::color_selector_height();
    let fsh = cce_ui::layout::font_selector_height();
    let rmh = cce_ui::layout::ramp_height();
    // The few exhibits with no toolkit default are canvases: an area to draw
    // or drag in, sized here and only here — and the Ramp's graph, one canvas
    // taller than its default.
    const CANVAS_H: f32 = 120.0;
    const W: f32 = 190.0;
    // Whether the exhibit stretches to a Vertical or grid cell (`ExhibitSize::stretch`).
    const FIT: bool = true;
    const OWN: bool = false;
    // In families, related controls side by side, each variant right after the widget it is
    // a look of (`Exhibit::after`), each family starting a section of its own
    // (`FAMILY_HEADS`): buttons, sliders and number inputs, colour, the ramps, text, then
    // status and display, then the surfaces. The ramps come before the text family: a
    // multiline TextBox keeps Tab (it types one), and Tab from the header must reach the
    // Ramp. A lasso is placed at its first member's place, so the fitted one heads the
    // buttons and the loose one, round the StatusDots, the status family.
    let raw: [(usize, f32, f32, bool); 32] = [
        // Buttons and toggles.
        (1, W, bh, FIT),                               // Button
        (18, W, bh, FIT),                              // ButtonStrip
        (CHECKBOX_SLOT, W, tgh, OWN),                  // Checkbox (narrowed to its box and label: `hug`)
        (3, W, tgh, OWN),                              // Toggle (its switch is drawn for its own width)
        (25, W, bh, FIT),                              // Breadcrumb (button plates)
        // Sliders and number inputs.
        (5, W, slh, FIT),                              // Slider
        (7, W, rsh, FIT),                              // RangeSlider
        (33, SLIDER_SPINBOX_W, slh.max(sph), FIT),     // SliderSpinbox (its band, then the box)
        (6, W, sph, FIT),                              // Spinbox
        (20, W, Float3::preferred_height(false), FIT), // Float3 (three slider rows)
        (19, s2d_w, 120.0, OWN),                       // Slider2D (a 120px pad)
        (8, W, CANVAS_H, FIT),                         // Trackpad
        // Colour.
        (15, W, csh, FIT),                             // ColorSelector
        (COLOR_PALETTE_SLOT, W, cph, FIT),             // ColorPalette (its strip and colour selector row, across two grid columns)
        (COLOR_RAMP_SLOT, W, crh, FIT),                // ColorRamp (its track, pegs and colour selector row, across two grid columns)
        (11, W, bh, FIT),                              // Color Ramp...
        // The ramps.
        (13, W, bh, FIT),                              // Ramp...
        (RAMP_SLOT, W, Ramp::expanded_height() + CANVAS_H, FIT), // Ramp (a 240 graph over its controls, across two grid columns: `own_h`, `span`)
        (28, W, 2.0 * rmh, FIT),                       // RampPreview (a ramp's curve)
        (27, W, CANVAS_H, FIT),                        // BevelPreview
        // Text.
        (9, W, tbh, FIT),                              // TextBox
        (17, W, tbh, FIT),                             // KeybindRecorder (a textbox)
        (16, W, fsh, FIT),                             // FontSelector
        // Status and display: the StatusDots first, in their lasso.
        (22, StatusDot::SIZE, StatusDot::SIZE, OWN),   // StatusDot (12px square; its label is wider)
        (4, W, pbh, FIT),                              // ProgressBar
        (21, W, pbh, FIT),                             // UsageBar
        (23, W, 3.0 * ddh, FIT),                       // InfoBox (title + two lines; the height it declares wins)
        (24, W, 2.0 * ddh, FIT),                       // InteractiveListItem (title + subtitle)
        (TREELIST_SLOT, W, 2.0 * CANVAS_H, FIT),       // TreeList (two canvases tall, across two grid columns: `span`)
        // Surfaces and dividers.
        (10, W, CANVAS_H, FIT),                        // Plate
        (29, W, 1.0, FIT),                             // Separator (a rule)
        (30, 6.0, CANVAS_H, OWN),                      // Splitter (a vertical grip)
    ];
    let mut sizes = Vec::new();
    let variants: Vec<ExhibitSize> = extra_sizes(GALLERY_COUNT, extra).collect();
    let mut placed = vec![false; variants.len()];
    for &(slot, w, h, stretch) in &raw {
        sizes.push(ExhibitSize {
            slot,
            w,
            h,
            stretch,
            hug: slot == CHECKBOX_SLOT,
            span: if spans_two(slot) { 2 } else { 1 },
            own_h: slot == RAMP_SLOT,
        });
        for (k, e) in extra.iter().enumerate() {
            if e.after == Some(slot) {
                sizes.push(variants[k]);
                placed[k] = true;
            }
        }
    }
    // A variant whose widget has no place in the table goes last rather than nowhere.
    sizes.extend(variants.into_iter().zip(placed).filter(|&(_, p)| !p).map(|(v, _)| v));
    sizes
}

/// The sizes of exhibits sized where they are built (`Exhibit`) — the variants, the Relief
/// page's tiles — in order, from slot `first`.
fn extra_sizes(first: usize, exhibits: &[Exhibit]) -> impl Iterator<Item = ExhibitSize> + '_ {
    exhibits.iter().enumerate().map(move |(k, e)| ExhibitSize { slot: first + k, w: e.w, h: e.h, stretch: e.stretch, hug: false, span: e.span, own_h: false })
}

/// The Relief page's tiles: one per effect (`Effect::ALL`), captioned by the adapter's
/// detached label, so the arrangement places them as it does the widgets. A tile declares
/// no height of its own; a series lies across two columns and is as wide as two in every
/// layout.
fn relief_tiles(ctx: &mut UiContext) -> Vec<Exhibit> {
    const W: f32 = 190.0;
    let gap = cce_ui::layout::control_gap();
    Effect::ALL
        .into_iter()
        .map(|effect| {
            if effect.is_series() {
                Exhibit::sized(ctx, ReliefTile::new(effect), 2.0 * W + gap, effect.height()).spanning(2)
            } else {
                Exhibit::sized(ctx, ReliefTile::new(effect), W, effect.height())
            }
        })
        .collect()
}

/// The exhibits, of either page: every slot but the lassos (31, 32) and the header
/// dropdowns (14 Layout, 0 Style, 35 Page), which `layout_exhibits` lays out under the
/// header.
fn is_exhibit(i: usize) -> bool {
    matches!(i, 1..=13 | 15..=30 | 33 | 34 | COLOR_PALETTE_SLOT) || i >= GALLERY_COUNT
}

/// The Group lassos: drawn under the exhibit clip like exhibits, laid out by
/// their members rather than by the arrangement.
fn is_overlay(i: usize) -> bool {
    matches!(i, 31 | 32)
}

impl State {
    fn visibility(&self) -> Visibility {
        Visibility {
            is_child: self.is_child,
            use_menubar: self.use_menubar,
            use_statusbar: self.use_statusbar,
            page: self.page,
            tiles_start: match &self.roster {
                Roster::Gallery(s) => s.tiles_start(),
                Roster::Child(_) => usize::MAX,
            },
        }
    }

    fn is_widget_visible(&self, index: usize) -> bool {
        self.visibility().is_visible(index)
    }

    /// The roster slot holding the window's focus, when a slot itself holds it. Read off
    /// the context, which Tab moves as well as a click: a widget inside a slot (a Ramp's
    /// field) is reached by the key sweep instead.
    fn focused_slot(&self) -> Option<usize> {
        let id = self.ui_context.focused_widget?;
        (0..self.roster.len()).find(|&i| self.roster.id(i) == id)
    }

    /// Whether the window's focus is on `root` or a widget under it in the ui tree. Asked of
    /// the holder (`focus_holder`): a Ramp's dropdown with the keyboard is a field the ramp
    /// holds by value, in no tree, so the ramp is where the focus is.
    fn focus_within(&self, root: WidgetId) -> bool {
        self.ui_context.focus_holder().is_some_and(|id| self.holds(root, id))
    }

    /// Whether `id` is `root` or a widget under it in the ui tree.
    fn holds(&self, root: WidgetId, id: WidgetId) -> bool {
        let mut at = Some(id);
        while let Some(cur) = at {
            if cur == root {
                return true;
            }
            at = self.ui_context.tree.parent_id(cur);
        }
        false
    }

    /// Write a Ramp editor's curve to the bevel ramp file if it differs from what the
    /// file holds. A failed write is reported once and retried on the next change.
    fn save_ramp(&mut self, kdl: String) {
        if self.saved_ramp.as_deref() == Some(kdl.as_str()) {
            return;
        }
        if let Err(e) = save_bevel_ramp(&kdl) {
            eprintln!("cce-gallery: writing {}: {e}", bevel_ramp_path().display());
        }
        self.saved_ramp = Some(kdl);
    }

    /// Recompute the positions table for the current size, mode and layout.
    fn relayout(&mut self) {
        self.positions = if self.is_child {
            child_positions(self.width, self.height, self.use_menubar, self.use_statusbar)
        } else {
            // A header row is as tall as a dropdown under its label.
            let dd = self.roster.get_dyn(&self.ui_context, 14);
            let row_h = dd.label_strip() + dd.preferred_height().unwrap_or(0.0);
            demo_positions(self.roster.len(), self.width, row_h)
        };
    }

    /// The page's exhibits in layout order, each with the content size it is reset to
    /// before it is arranged: the widgets' table (`exhibit_table`) or the Relief tiles'.
    fn exhibit_sizes(&self) -> Vec<ExhibitSize> {
        if self.page == Page::Relief {
            let s = self.roster.gallery();
            return extra_sizes(s.tiles_start(), &s.tiles).collect();
        }
        let s2d_w = self
            .roster
            .get_dyn(&self.ui_context, 19)
            .measure(LayoutConstraints::new(0.0, f32::MAX, 0.0, f32::MAX), &self.ui_context)
            .width;
        let crh = self.roster.get_dyn(&self.ui_context, COLOR_RAMP_SLOT).preferred_height().unwrap_or(0.0);
        let cph = self.roster.get_dyn(&self.ui_context, COLOR_PALETTE_SLOT).preferred_height().unwrap_or(0.0);
        exhibit_table(s2d_w, crh, cph, &self.roster.gallery().extra)
    }

    /// The exhibits' viewport: below the header, down to the window's foot.
    /// The exhibit area: a well sunk into the root plate, standing on it like the
    /// header dropdowns — their inset in from the window's sides and bottom (`x` is
    /// the Layout dropdown's), one root gap below the lowest of them (a narrow window
    /// wraps the header onto further rows).
    fn exhibit_viewport(&self) -> (f32, f32, f32, f32) {
        let gap = cce_ui::layout::root_plate_gap();
        let dx = self.roster.get_dyn(&self.ui_context, 14).rect().0;
        let foot = HEADER
            .iter()
            .map(|&i| {
                let (_, y, _, h) = self.roster.get_dyn(&self.ui_context, i).rect();
                y + h
            })
            .fold(f32::MIN, f32::max);
        let (x, y) = (dx, foot + gap);
        let w = (self.width - 2.0 * x).max(1.0);
        let h = ((self.height - x) - y).max(100.0);
        (x, y, w, h)
    }

    /// The well's rect, outer corner radius and wall depth. The wall is every
    /// well's rule (`well_rim`: the DE bevel width capped at a fifth of the
    /// height), taken in both styles so the page lays out the same whichever the
    /// Style dropdown selects — flat, it is the margin inside the hairline frame.
    fn exhibit_well(&self) -> (cce_ui::scene::layout::Rect, f32, f32) {
        let (x, y, w, h) = self.exhibit_viewport();
        let rect = cce_ui::scene::layout::Rect { x, y, width: w, height: h };
        let radius = cce_ui::layout::plate_corner_radius();
        let wall = cce_ui::layout::bevel_width().min(h * 0.2);
        (rect, radius, wall)
    }

    /// The well's floor inside its wall — the plate the exhibits sit on and the
    /// fitted lasso snaps to — with the floor's corner radius.
    fn exhibit_floor(&self) -> (cce_ui::scene::layout::Rect, f32) {
        let (rect, radius, wall) = self.exhibit_well();
        let (floor, radii) = cce_ui::layout::carve_inside(rect, (radius, radius, radius, radius), wall);
        (floor, radii.0)
    }

    /// Whether an exhibit on the page shown is being dragged: the page's layout and scroll
    /// are frozen until it is let go (`layout_exhibits`).
    fn exhibit_dragging(&self) -> bool {
        let vis = self.visibility();
        !self.is_child && (0..self.roster.len()).any(|i| is_exhibit(i) && vis.is_visible(i) && self.roster.is_dragging(&self.ui_context, i))
    }

    fn in_exhibit_viewport(&self, px: f32, py: f32) -> bool {
        let (x, y, w, h) = self.exhibit_viewport();
        px >= x && px <= x + w && py >= y && py <= y + h
    }

    /// Lay the page's exhibits out with the arrangement the Layout dropdown selects
    /// (`LAYOUTS`), each `Group` lasso as one block around its members (`GROUPS`), then shift
    /// them by the scroll frame's offset; painting clips them to the viewport. The other
    /// page's exhibits are parked. Runs from apply_layout and from display_list, which is
    /// what re-arranges after a scroll.
    ///
    /// While an exhibit is dragged (`exhibit_dragging`: a Ramp key held, a slider's thumb)
    /// the page is frozen: nothing is re-arranged or landed, the dragged exhibit included,
    /// so nothing moves out from under the pointer or slides into its place — not on a
    /// scroll, which is held for the drag too (`tick`, `handle_mouse_wheel`), nor on a
    /// resize. The first frame after the release arranges the page afresh.
    fn layout_exhibits(&mut self) {
        let (x, y, w, h) = self.exhibit_viewport();
        self.exhibit_scroll.set_rect(x, y, w, h);
        if self.exhibit_dragging() {
            return;
        }
        // The other page's exhibits are parked off-screen with `layout`, which moves their
        // embedded children with them — before they are hidden, as a hidden container
        // places no children — and hidden with its lassos, which keeps them out of the
        // paint walk: none is hit or a Tab stop.
        let vis = self.visibility();
        for i in (0..self.roster.len()).filter(|&i| is_exhibit(i) || is_overlay(i)) {
            let shown = vis.is_visible(i);
            let park = !shown && is_exhibit(i);
            self.ui_context.lend(self.roster.id(i), |widget, ctx| {
                let (px, _, pw, ph) = widget.rect();
                if park && px > PARKED / 2.0 {
                    let strip = widget.label_strip();
                    let ch = (ph - strip).max(1.0);
                    widget.layout(Point { x: PARKED, y: PARKED + strip }, LayoutConstraints::new(pw.max(1.0), pw.max(1.0), ch, ch), ctx);
                }
                if widget.visible() != shown {
                    widget.set_visible(shown);
                }
            });
        }
        // The exhibits sit on the well's floor, inside its wall; the fitted lasso's
        // plate is that floor, so its sides snap to the foot of the wall.
        let (floor, floor_r) = self.exhibit_floor();
        if let Roster::Gallery(s) = &mut self.roster {
            self.ui_context[s.group_fitted].inner_mut().set_plate(floor, floor_r);
        }
        // Seed each exhibit at its content size plus the label strip above it, and read
        // its block back: `set_rect` may adjust the rect, and a height the widget declares
        // overrides the table's unless the table's is its own (`ExhibitSize::own_h`).
        let mut exhibits: Vec<(usize, Block)> = Vec::new();
        for e in self.exhibit_sizes() {
            let widget = self.roster.get_dyn_mut(&mut self.ui_context, e.slot);
            let strip = widget.label_strip();
            widget.set_rect(0.0, 0.0, e.w, e.h + strip);
            if let Some(cw) = e.hug.then(|| content_extent(widget)).flatten() {
                widget.set_rect(0.0, 0.0, cw.min(e.w), e.h + strip);
            }
            let b = Block::of(widget, e.stretch);
            let b = Block { span: e.span, content_h: if e.own_h { e.h } else { b.content_h }, ..b };
            exhibits.push((e.slot, b));
        }
        // The exhibits start two paddings in from the floor's sides and top: one for the
        // fitted lasso's frame, which hangs one padding outside its members and so lands
        // on its seat, one padding in, and one between the frame and the members inside it
        // — the same padding as on its other sides. Its title tab rides in the label strip
        // the arrangement reserves above every row, which a lasso's tab counts toward.
        let s = self.roster.gallery();
        let pad = self.ui_context[s.group_fitted].inner().padding();
        let slots: Vec<usize> = exhibits.iter().map(|&(slot, _)| slot).collect();
        let lassos = lassos(&slots, |group| {
            let g = self.ui_context[s.group(group)].inner();
            (g.padding(), g.tab_height())
        });
        let kind = LAYOUTS.get(self.layout_idx).unwrap_or(&LAYOUTS[DEFAULT_LAYOUT]).1;
        let (lx, ly) = (floor.x + 2.0 * pad, floor.y + 2.0 * pad);
        let area = cce_ui::scene::layout::Rect { x: lx, y: ly, width: floor.width - 4.0 * pad, height: floor.y + floor.height - ly };
        let blocks: Vec<Block> = exhibits.iter().map(|&(_, b)| b).collect();
        let breaks: Vec<usize> = slots.iter().enumerate().filter(|(_, s)| FAMILY_HEADS.contains(s)).map(|(i, _)| i).collect();
        let (contents, _) = arrange_sections(kind, &blocks, &lassos, area, cce_ui::layout::control_gap(), &breaks);
        // The scroll runs to where the lowest exhibit ends — not the arrangement's height,
        // which counts a trailing gap under some layouts and none under others — plus the
        // same two paddings above the floor's foot that the first row has below its top,
        // and the wall below that: scrolled to the bottom, nothing ends under the wall.
        let bottom = contents.iter().map(|r| r.y + r.height).fold(ly, f32::max);
        let foot = (y + h) - (floor.y + floor.height) + 2.0 * pad;
        self.exhibit_scroll.update_bounds(bottom + foot - y, y, h);
        let scroll_y = self.exhibit_scroll.scroll_y;
        for (&(idx, _), r) in exhibits.iter().zip(&contents) {
            // Land each exhibit's content box where the arrangement put it, shifted by the
            // scroll; its label hangs in the strip above.
            self.ui_context.lend(self.roster.id(idx), |widget, ctx| {
                widget.layout(Point { x: r.x, y: r.y - scroll_y }, LayoutConstraints::new(r.width, r.width, r.height, r.height), ctx);
            });
        }
    }

    /// Hear what the widgets reported since the last sweep (`UiContext::take_events`) and
    /// act on it: Layout (14) picks the arrangement, Style (0) switches every control
    /// between Relief and Flat (`set_control_relief` — the toolkit reads it live at paint,
    /// so a rebuild is all it takes), Page (35) the page shown (`set_page`), `Color Ramp...`
    /// and `Ramp...` open their editors under themselves (`place_editor`), and a child's
    /// Close ends it — the returned message. Any report repaints; every other exhibit reacts
    /// by itself and only needs the frame.
    ///
    /// Runs after every dispatch path — pointer, wheel, key, tick and the context menu's
    /// rows — so a keyboard activation (Enter or Space on a focused button, a dropdown's
    /// Enter) or an assistive tool's lands at once rather than at the next click.
    fn drain_widget_events(&mut self, needs_rebuild: &mut bool) -> Option<String> {
        let (mut layout, mut style, mut page, mut exit) = (false, false, false, false);
        let mut editor = None;
        for (id, ev) in self.ui_context.take_events() {
            let clicked = match ev {
                WidgetEvent::Clicked => true,
                WidgetEvent::Changed => false,
                _ => continue,
            };
            *needs_rebuild = true;
            match &self.roster {
                Roster::Child(c) => exit |= clicked && id == c.close.id(),
                // A dropdown reports a pick as Changed.
                Roster::Gallery(s) if id == s.layout_dd.id() => layout = true,
                Roster::Gallery(s) if id == s.style_dd.id() => style = true,
                Roster::Gallery(s) if id == s.page_dd.id() => page = true,
                Roster::Gallery(s) if clicked && id == s.color_ramp_btn.id() => editor = Some((ChildKind::ColorRamp, id)),
                Roster::Gallery(s) if clicked && id == s.ramp_btn.id() => editor = Some((ChildKind::Ramp, id)),
                Roster::Gallery(_) => {}
            }
        }
        if let Some((kind, button)) = editor {
            let rect = self.ui_context.get_widget(button).map(|w| w.rect());
            if let Some(rect) = rect {
                place_editor(kind, rect);
            }
            spawn_editor(kind);
        }
        if layout {
            self.layout_idx = self.roster.value(&self.ui_context, 14) as usize;
        }
        if style {
            cce_ui::layout::set_control_relief(self.roster.value(&self.ui_context, 0) == 0);
        }
        if page {
            let picked = PAGES.get(self.roster.value(&self.ui_context, PAGE_SLOT) as usize).map_or(Page::Widgets, |p| p.1);
            self.set_page(picked);
        }
        if layout || style {
            self.relayout();
            self.apply_layout();
        }
        exit.then(|| "exit".to_string())
    }

    /// [`drain_widget_events`](Self::drain_widget_events) for a hook that returns no
    /// message: the message goes to `update` through the app's sender.
    fn drain_widget_events_posting(&mut self, needs_rebuild: &mut bool) {
        if let Some(msg) = self.drain_widget_events(needs_rebuild) {
            let _ = self.sender.send(msg);
        }
    }

    /// Show `page`: its exhibits laid out at the top of the area, the other page's parked
    /// (`layout_exhibits`), the focus let go if the switch hid the widget holding it, and
    /// the shared context menu closed — its rows act on the exhibit it opened over.
    ///
    /// A press or drag held across the switch (the page picked from the keyboard while the
    /// pointer holds a slider) ends first, while its exhibit is still shown: the slots that
    /// hold the router's drag target or grab, or are dragging, get a left release off every
    /// widget (twice [`PARKED`] out), from which the router sends DragEnd and drops its
    /// target and grab. Left held, the next pointer move dragged a hidden exhibit, and one
    /// still dragging was kept off the arrangement on its page's return. Off every widget,
    /// not at the cursor: a held Button commits its click on a release inside it, and the
    /// switch would have opened the editor a held "Ramp..." never finished clicking.
    fn set_page(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        let gesture = [self.ui_context.drag_target, self.ui_context.active_grab];
        let vis = self.visibility();
        let held: Vec<WidgetId> = (0..self.roster.len())
            .filter(|&i| vis.is_visible(i))
            .filter(|&i| {
                let root = self.roster.id(i);
                self.roster.is_dragging(&self.ui_context, i) || gesture.into_iter().flatten().any(|id| self.holds(root, id))
            })
            .map(|i| self.roster.id(i))
            .collect();
        let (x, y) = (2.0 * PARKED, 2.0 * PARKED);
        let release = cce_ui::widget::Event::MouseButton { button: MouseButton::Left, state: ElementState::Released, x, y, local_x: x, local_y: y };
        for root in held {
            self.ui_context.propagate_event(&release, root);
        }
        cce_ui::widget::context_menu::hide();
        self.page = page;
        // A host write the scroll frame adopts on its next tick, a glide included.
        self.exhibit_scroll.scroll_y = 0.0;
        let vis = self.visibility();
        let hidden = (0..self.roster.len()).any(|i| !vis.is_visible(i) && self.focus_within(self.roster.id(i)));
        if let Some(holder) = self.ui_context.focus_holder().filter(|_| hidden) {
            self.ui_context.unfocus_id(holder);
        }
        self.relayout();
        self.apply_layout();
    }

    fn apply_layout(&mut self) {
        for i in 0..self.roster.len() {
            if self.roster.is_dragging(&self.ui_context, i) {
                continue;
            }
            // The exhibits are laid out by layout_exhibits below.
            if !self.is_child && is_exhibit(i) {
                continue;
            }
            let visible = self.is_widget_visible(i);
            let (x, y, w, h) = self.positions[i];
            if !self.is_child && HEADER.contains(&i) {
                // The header dropdowns land through `layout` at their own preferred
                // height: `y` is where the label goes, the content sits a strip below.
                self.ui_context.lend(self.roster.id(i), |widget, ctx| {
                    let h = widget.preferred_height().unwrap_or(h);
                    let strip = widget.label_strip();
                    widget.layout(Point { x, y: y + strip }, LayoutConstraints::new(w, w, h, h), ctx);
                });
                continue;
            }
            let widget = self.roster.get_dyn_mut(&mut self.ui_context, i);
            if visible {
                widget.set_rect(x, y, w, h);
            } else {
                widget.set_rect(-1000.0, -1000.0, 0.0, 0.0);
            }
        }
        if !self.is_child {
            self.layout_exhibits();
        }
    }

}

impl cce_ui::engine::Application for State {
    type Message = String;

    fn layer_app(&mut self) -> Option<&mut dyn cce_ui::engine::LayerApp> {
        Some(self)
    }

    fn create(sender: cce_ui::engine::AppSender<Self::Message>) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let flag = |name: &str| args.iter().any(|a| a == name);
        let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
        let number = |name: &str| value(name).and_then(|s| s.parse::<f32>().ok());

        let is_child = flag("--child");
        let use_root_plate = if is_child { flag("--root-plate") } else { !flag("--no-root-plate") };
        let child_kind = if is_child {
            Some(value("--type").and_then(|t| ChildKind::from_arg(&t)).unwrap_or(ChildKind::Floating))
        } else {
            None
        };
        // An editor has no menu bar or status bar to show.
        let editor = child_kind.is_some_and(ChildKind::is_editor);
        let use_menubar = flag("--menubar") && !editor;
        let use_statusbar = flag("--statusbar") && !editor;
        let opacity = flag("--opacity");
        let transparency = number("--transparency").unwrap_or(1.0);
        let border_enabled = !flag("--no-border");
        let custom_width = number("--width");
        let custom_height = number("--height");

        let (c_w, c_h): (f32, f32) = match (child_kind, custom_width, custom_height) {
            (_, Some(w), Some(h)) => (w, h),
            (Some(kind), _, _) => kind.default_size(),
            (None, _, _) => (1000.0, 680.0),
        };

        let opacity = opacity || editor;

        // The context owns the widgets; the roster names them by slot.
        let mut ui_context = cce_ui::context::UiContext::new();
        let ctx = &mut ui_context;
        // The curve a Ramp editor opened on, as the file holds it.
        let mut saved_ramp = None;
        let roster = if let Some(kind) = child_kind {
            let bg = if use_root_plate {
                ctx.insert(RootPlate::new(0.0, 0.0, c_w, c_h)).id()
            } else {
                ctx.insert(ContentBg::new()).id()
            };
            // The main slot, then the bars a simulated window can show.
            let (main, bars) = if kind == ChildKind::ColorRamp {
                (ctx.insert(ColorRamp::new()).id(), None)
            } else if kind == ChildKind::Ramp {
                let mut ramp = Ramp::new();
                let (loaded_keys, loaded_type) = load_bevel_ramp();
                ramp.keys = loaded_keys;
                ramp.line_type_dropdown.selected = match loaded_type.as_str() {
                    "bezier" => 1,
                    _ => 0,
                };
                saved_ramp = Some(bevel_ramp_kdl(&ramp.keys, ramp_line_type(&ramp)));
                (ctx.insert(ramp).id(), None)
            } else {
                let menu_bar = MenuBar::new(0.0, 0.0, c_w, CHILD_MENUBAR_H)
                    .with_item("File", &["New", "Open", "Save", "Exit"])
                    .with_item("Edit", &["Undo", "Redo", "Cut", "Copy", "Paste"])
                    .with_right_aligned_title(true);
                (
                    ctx.insert(Label::new(kind.child_text()).with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])).id(),
                    Some([ctx.insert(menu_bar).id(), ctx.insert(StatusBar::new()).id()]),
                )
            };
            let close = ctx.insert(Button::new(0.0, 0.0, CHILD_CLOSE_W, cce_ui::layout::button_height()).with_label("Close"));
            let mut ids = vec![bg, main, close.id()];
            ids.extend(bars.into_iter().flatten());
            Roster::Child(Box::new(ChildSlots { ids, close }))
        } else {
            Roster::Gallery(Box::new(GallerySlots {
                button_demo: ctx.insert(Button::new(0.0, 0.0, 140.0, cce_ui::layout::button_height()).with_label("Button")),
                checkbox_demo: ctx.insert(Checkbox::new().with_label("Checkbox")),
                toggle_demo: ctx.insert(Toggle::new().with_label("Toggle")),
                progress_demo: ctx.insert(ProgressBar::new(0.43).with_label("ProgressBar")),
                slider_demo: ctx.insert(Slider::new().with_label("Slider")),
                spinbox_demo: ctx.insert(Spinbox::new(10, 1, 100, 5).with_label("Spinbox")),
                range_slider_demo: ctx.insert(RangeSlider::new().with_label("RangeSlider")),
                trackpad_demo: ctx.insert(Trackpad::new().with_label("Trackpad")),
                textbox_demo: ctx.insert(TextBox::new("Interactive TextBox".to_string())),
                plate_demo: ctx.insert(Plate::new(0.0, 0.0, 120.0, 120.0, true).with_label("Plate")),
                color_ramp_btn: ctx.insert(Button::new(0.0, 0.0, 120.0, cce_ui::layout::button_height()).with_label("Color Ramp...")),
                bevel_ramp: ctx.insert(with_key_in_hand(Ramp::new().with_label("Ramp"), |r| r.select(1))),
                ramp_btn: ctx.insert(Button::new(0.0, 0.0, 120.0, cce_ui::layout::button_height()).with_label("Ramp...")),
                layout_dd: ctx.insert(Dropdown::new(
                    LAYOUTS.iter().map(|(name, _)| name.to_string()).collect(),
                    DEFAULT_LAYOUT,
                ).with_label("Layout")),
                style_dd: ctx.insert(Dropdown::new(
                    vec!["Relief".to_string(), "Flat".to_string()],
                    if cce_ui::layout::control_relief() { 0 } else { 1 },
                ).with_label("Style")),
                page_dd: ctx.insert(Dropdown::new(PAGES.iter().map(|(name, _)| name.to_string()).collect(), 0).with_label("Page")),
                color_selector_demo: ctx.insert(ColorSelector::new([64, 128, 255]).with_label("ColorSelector")),
                font_selector_demo: ctx.insert(FontSelector::new("Sans".to_string()).with_label("FontSelector")),
                keybind_demo: ctx.insert(KeybindRecorder::new("ctrl+1".to_string()).with_label("KeybindRecorder")),
                button_strip_demo: ctx.insert(Adapted::new(
                    ButtonStrip::new(0.0, 0.0, 200.0, cce_ui::layout::button_height())
                        .with_buttons(vec!["One".to_string(), "Two".to_string(), "Three".to_string()])
                        .with_selected(Some(0)),
                ).with_label("ButtonStrip")),
                slider2d_demo: ctx.insert(Slider2D::new().with_label("Slider2D")),
                float3_demo: ctx.insert(Float3::new().with_label("Float3")),
                usage_bar_demo: ctx.insert(UsageBar::new(0.62).with_label("UsageBar")),
                status_dot_demo: ctx.insert(StatusDot::new(DotStatus::Active).with_label("StatusDot")),
                info_box_demo: ctx.insert(InfoBox::new("InfoBox", vec!["A titled box of".to_string(), "plain text lines.".to_string()])),
                // A row as the settings pages draw theirs: a name over a detail line.
                // Without the subtitle it was one line of text with no wash at rest,
                // indistinguishable from a Label.
                list_item_demo: ctx.insert(InteractiveListItem::new("InteractiveListItem").with_subtitle("A title over a subtitle")),
                breadcrumb_demo: {
                    let mut b = Breadcrumb::new();
                    b.path = vec!["home".to_string(), "lsgalante".to_string(), "projects".to_string()];
                    ctx.insert(b.with_label("Breadcrumb"))
                },
                tree_list_demo: {
                    let mut t = TreeList::new();
                    // More keys than its height shows, of each value type, so the
                    // exhibit fills its list and scrolls it.
                    t.set_flat_keys(vec![
                        ("layout/bar_height".to_string(), serde_json::json!(24)),
                        ("layout/control_relief".to_string(), serde_json::json!(true)),
                        ("layout/corner_radius".to_string(), serde_json::json!(6)),
                        ("layout/gap".to_string(), serde_json::json!(12)),
                        ("style/animations".to_string(), serde_json::json!(true)),
                        ("theme/accent".to_string(), serde_json::json!("#4080ff")),
                        ("theme/name".to_string(), serde_json::json!("cce")),
                        ("theme/opacity".to_string(), serde_json::json!(0.92)),
                    ]);
                    t.rebuild_tree();
                    ctx.insert(t.with_label("TreeList"))
                },
                bevel_preview_demo: ctx.insert(BevelPreview::new().with_label("BevelPreview")),
                ramp_preview_demo: ctx.insert(RampPreview::new().with_label("RampPreview")),
                separator_demo: ctx.insert(Separator::new(0.0, 0.0, 200.0, 1.0, [0.5, 0.5, 0.6, 1.0]).with_label("Separator")),
                splitter_demo: ctx.insert(Splitter::new(200.0).with_label("Splitter")),
                // Members are wired below, once the slots have ids.
                // style: deliberate — tight padding: the gallery packs its rows closer than a
                // settings page, and the fitted lasso's padding is what insets the exhibits.
                group_loose: ctx.insert(Group::new(Vec::new()).with_label("Group").with_padding(6.0)),
                group_fitted: ctx.insert(Group::new(Vec::new()).with_label("Group (fitted)").with_fit(true).with_padding(6.0)),
                slider_spinbox_demo: ctx.insert(SliderSpinbox::new_int(5, 0, 10, 1).with_label("SliderSpinbox")),
                // Unlabeled, as the Ramp is: it lays its track out from the top of the rect
                // it is given, where a detached label's strip would be, and drew over one.
                color_ramp_demo: ctx.insert(with_key_in_hand(ColorRamp::new().with_label("ColorRamp"), |r| r.select(1))),
                color_palette_demo: ctx.insert(with_key_in_hand(ColorPalette::new().with_label("ColorPalette"), |p| p.select(0))),
                extra: variant_exhibits(ctx),
                tiles: relief_tiles(ctx),
            }))
        };

        let roster = {
            let mut roster = roster;
            if let Roster::Gallery(s) = &mut roster {
                // The lassos (`GROUPS`): the fitted one fits the exhibit area's edges —
                // its top snaps to the area's top with the title tab kept inside, its
                // left to the area's left edge.
                for (group, members) in GROUPS {
                    let ids = members.iter().map(|&i| s.id(i)).collect();
                    ui_context[s.group(group)].inner_mut().set_members(ids);
                }
            }
            roster
        };
        let mut state = Self {
            roster,
            positions: Vec::new(),
            width: c_w,
            height: c_h,
            scale: 1.0,
            is_child,
            opacity,
            transparency,
            use_root_plate,
            use_menubar,
            use_statusbar,
            border_enabled,
            child_kind,
            ui_context,
            layout_idx: DEFAULT_LAYOUT,
            page: PAGES[0].1,
            exhibit_scroll: {
                let mut sb = ScrollBox::new();
                sb.show_border = false;
                sb.show_background = false;
                // The DE's one scrollbar: down the area's centre line, idling
                // behind the well's floor until a scroll raises it.
                sb.sink_behind = true;
                sb
            },
            sender,
            saved_ramp,
        };

        if child_kind == Some(ChildKind::Ramp) {
            // A Ramp editor opens with the ramp focused, its graph holding the keyboard: the
            // arrows pick a key at once, and Tab walks on to its controls. Every other child
            // starts with nothing focused: the key sweep in handle_key_input reaches every
            // visible slot anyway, and a click focuses whatever it lands on.
            let ramp_id = state.roster.id(1);
            state.ui_context.set_focused_id(ramp_id);
        }

        state.relayout();
        state.apply_layout();
        state
    }

    /// The gallery navigates in plate terms: Tab walks the exhibits.
    fn plate_navigation(&self) -> bool {
        true
    }

    fn settings(&self) -> cce_ui::engine::WindowSettings {
        let title = match self.child_kind {
            Some(kind) => kind.title().to_string(),
            None => "Gallery".to_string(),
        };

        let mut app_id = match self.child_kind {
            Some(kind) => kind.app_id(),
            None => "cce-gallery".to_string(),
        };
        if !self.border_enabled {
            app_id.push_str("-noborder");
        }

        cce_ui::engine::WindowSettings {
            title,
            app_id,
            width: self.width as u32,
            height: self.height as u32,
            fullscreen: self.child_kind == Some(ChildKind::Fullscreen),
            min_size: if self.is_child {
                Some((self.width as u32, self.height as u32))
            } else {
                Some((100, 100))
            },
        }
    }

    fn utility(&self) -> bool {
        self.child_kind == Some(ChildKind::Utility)
    }

    fn update(&mut self, msg: Self::Message, _needs_rebuild: &mut bool, exit: &mut bool) {
        if msg == "exit" {
            *exit = true;
        }
    }

    fn tick(&mut self, dt: f32, needs_rebuild: &mut bool) {
        let mut changed = false;
        // True while the area's bar is held up or fading, as well as while a
        // glide moves it, so the frames keep coming until the sink renders.
        // While an exhibit is dragged the offset is held where the drag found it,
        // as the frozen page is (`layout_exhibits`): putting it back is a host
        // write, which the frame adopts on its next tick, ending the glide.
        if !self.is_child {
            let (held, scroll_y) = (self.exhibit_dragging(), self.exhibit_scroll.scroll_y);
            if self.exhibit_scroll.tick(dt, &mut self.ui_context) {
                changed = true;
            }
            if held {
                self.exhibit_scroll.scroll_y = scroll_y;
            }
        }
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);
        for i in 0..self.roster.len() {
            if is_visible(i) {
                let ramp_child = self.child_kind == Some(ChildKind::Ramp) && i == 1;
                let ticked = self.ui_context.lend(self.roster.id(i), |w, ctx| {
                    let ticked = w.tick(dt, ctx);
                    // A Ramp editor's curve, to save if the tick changed it.
                    let ramp = (ticked && ramp_child)
                        .then(|| w.as_any().downcast_ref::<Ramp>().map(|r| bevel_ramp_kdl(&r.keys, ramp_line_type(r))))
                        .flatten();
                    (ticked, ramp)
                });
                if let Some((ticked, ramp)) = ticked {
                    changed |= ticked;
                    if let Some(kdl) = ramp {
                        self.save_ramp(kdl);
                    }
                }
            }
        }
        if changed {
            *needs_rebuild = true;
        }
        // A tick reports too: a FontSelector's picker returns its family here.
        self.drain_widget_events_posting(needs_rebuild);
    }

    fn display_list(&mut self, size: LogicalSize, scale: f64) -> Option<cce_ui::scene::paint::DisplayList> {
        // The whole frame — rounded geometry, plain geometry, popovers, then text — is
        // one display list, rebuilt each frame.
        use cce_ui::scene::layout::Rect;
        // Panel children follow the scroll offset at layout time: re-arrange every frame
        // so a wheel scroll moves the content on the frame it repaints. Idempotent and
        // cheap (~20 set_rects).
        if !self.is_child {
            self.layout_exhibits();
        }
        if (self.width - size.width).abs() > 0.001 || (self.height - size.height).abs() > 0.001 || (self.scale - scale).abs() > 0.001 {
            self.width = size.width;
            self.height = size.height;
            self.scale = scale;
            cce_ui::scale::set_scale_factor(scale as f32);

            self.relayout();
            self.apply_layout();
        }

        let sw = self.width;
        let sh = self.height;
        let mut pc = cce_ui::scene::paint::PaintCtx::new();

        // ── Rounded geometry ──
        if !self.is_child && self.use_root_plate {
            // The standard root plate (cce-ui PlateSpec::window).
            pc.root_plate(sw, sh);
        }
        // The exhibit area is a well in the root plate: its floor under the
        // exhibits here, its rim over them below (`well_rim`), so an exhibit
        // scrolled to the edge slides under the wall rather than sitting on it.
        // The area's scrollbar idles BEHIND the floor: its idle copy goes down
        // first, every frame and at full strength — raised or not, since the
        // fore copy fades in over it and dropping it at the latch would blink.
        if !self.is_child {
            self.exhibit_scroll.paint_scrollbar_pills(&mut pc, 1.0);
            let (well, radius, _) = self.exhibit_well();
            pc.well_floor(well, radius, &cce_ui::scene::Material::pane(), false);
        }

        // ── Geometry: each root widget's paint walk, everything but its text ──
        // A widget with a ui-tree parent (the page selector under the status bar) is
        // painted by that parent's walk. The text goes in the text pass below, which
        // culls it under open popovers and cuts it at the foot of the well's wall.
        // (Until 2026-10-08 this was the legacy tuple views — every rounded quad, then
        // every plain quad, then the other prims replayed — which put a widget's quads
        // over everything else it painted, whatever its own order.)
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(&self.ui_context, i);
            if !self.is_widget_visible(i) || self.ui_context.tree.parent_id(w.base().id()).is_some_and(|p| self.ui_context.tree.is_registered(p)) {
                continue;
            }
            let mut walk = cce_ui::scene::paint::PaintCtx::new();
            cce_ui::widget::painter::paint_root_into(&self.ui_context, w, &mut walk);
            let geometry = walk.finish().items.into_iter().filter(|it| !matches!(it.prim, Prim::Text { .. }));
            if !self.is_child && (is_exhibit(i) || is_overlay(i)) {
                let (vx, vy, vw, vh) = self.exhibit_viewport();
                pc.clip(Rect { x: vx, y: vy, width: vw, height: vh }, |pc| pc.append_items(geometry));
            } else {
                pc.append_items(geometry);
            }
        }
        // The well's rim over the exhibits, then the scrollbar's FORE copy over
        // the rim, at the fade a scroll raised it to (nothing while it is sunk:
        // then only the idle copy under the floor shows). Flat pills both
        // times — shader-lit relief does not fade with a vertex alpha.
        if !self.is_child {
            let (well, radius, _) = self.exhibit_well();
            pc.well_rim(well, radius, cce_ui::layout::control_relief());
            self.exhibit_scroll.paint_scrollbar_pills(&mut pc, self.exhibit_scroll.scrollbar_fade());
        }

        // ── Popovers: in-frame, on top of everything, into the frame's own
        // PaintCtx — real prims (the dropdown's expanded inset-plate surface)
        // with per-label bounds; glyphs render in the later text pass
        // regardless of emission order. ──
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(&self.ui_context, i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if w.popover_rect().is_some() {
                w.render_popover(&mut pc);
            }
        }

        // ── Text ──
        // A child window's title (the gallery paints none): on its MenuBar when it
        // has one, else on the root plate, one inset in from the window's corner —
        // when the window stacks (`child_stacks`); a one-row bar has no line for it.
        // Every simulated window builds a MenuBar (slot 3) but shows it only with
        // `--menubar`; a hidden one carries nothing.
        if let Some(kind) = self.child_kind {
            let label_text = kind.title().to_string();
            let vis = self.visibility();
            let mut has_menu_bar = false;
            for i in 0..self.roster.len() {
                if !vis.is_visible(i) {
                    continue;
                }
                let w = self.roster.get_dyn_mut(&mut self.ui_context, i);
                if let Some(menu_bar) = w.as_any_mut().downcast_mut::<MenuBar>() {
                    menu_bar.title = label_text.clone();
                    has_menu_bar = true;
                }
            }
            if !has_menu_bar && child_stacks(self.height, self.use_menubar, self.use_statusbar) {
                let inset = cce_ui::layout::root_plate_inset();
                pc.text_with(label_text, inset, inset, CHILD_TITLE_SIZE, [255, 255, 255], Some(cce_ui::layout::statusbar_font()), None);
            }
        }

        let mut popover_rects = Vec::new();
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(&self.ui_context, i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if let Some(rect) = w.popover_rect() {
                popover_rects.push(rect);
            }
        }
        let in_any_popover = |lx: f32, ly: f32| -> bool {
            for &(px, py, pw, ph) in &popover_rects {
                if lx >= px - 5.0 && lx <= px + pw + 5.0 && ly >= py - 5.0 && ly <= py + ph + 5.0 {
                    return true;
                }
            }
            false
        };

        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(&self.ui_context, i);
            if !self.is_widget_visible(i) {
                continue;
            }
            // A widget with a ui-tree parent (the page selector under the status bar) is
            // covered by that parent's walk — emitting it here too would draw its text
            // twice. Text inside a popover is culled; panel children clip to the panel.
            if self.ui_context.tree.parent_id(w.base().id()).is_some_and(|p| self.ui_context.tree.is_registered(p)) {
                continue;
            }
            // Text is cut at the foot of the wall: geometry slides under the rim,
            // words do not sit on it (the TreeList's rule).
            let cp_clip = if !self.is_child && (is_exhibit(i) || is_overlay(i)) {
                let (f, _) = self.exhibit_floor();
                let (vx, vy, vw, vh) = (f.x, f.y, f.width, f.height);
                Some([vx, vy, vx + vw, vy + vh])
            } else {
                None
            };
            let mut scratch = cce_ui::scene::paint::PaintCtx::new();
            cce_ui::widget::painter::append_widget_text(&self.ui_context, w, &mut scratch);
            for item in scratch.finish().items {
                if let cce_ui::scene::paint::Prim::Text { text, x, y, font_size, color, font, bounds, .. } = item.prim {
                    if in_any_popover(x, y) {
                        continue;
                    }
                    let bounds = match (bounds, cp_clip) {
                        (Some([l, t, r, b]), Some([pl, pt, pr, pb])) => {
                            Some([l.max(pl), t.max(pt), r.min(pr), b.min(pb)])
                        }
                        (None, Some(clip)) => Some(clip),
                        (b, None) => b,
                    };
                    pc.text_with(text, x, y, font_size, color, font, bounds);
                }
            }
        }


        Some(pc.finish())
    }

    fn display_list_text(&self) -> bool {
        true
    }

    fn clear_color(&self) -> [f32; 4] {
        let clear_alpha = if self.opacity || self.use_root_plate {
            if self.use_root_plate {
                0.0
            } else {
                self.transparency
            }
        } else {
            1.0
        };
        [0.05 * clear_alpha, 0.05 * clear_alpha, 0.08 * clear_alpha, clear_alpha]
    }

    fn desired_size(&self) -> Option<(u32, u32)> {
        Some((self.width as u32, self.height as u32))
    }

    fn ui_context(&self) -> Option<&cce_ui::context::UiContext> {
        Some(&self.ui_context)
    }

    fn ui_context_mut(&mut self) -> Option<&mut cce_ui::context::UiContext> {
        Some(&mut self.ui_context)
    }

    fn handle_pointer_move(&mut self, pos: LogicalPosition, needs_rebuild: &mut bool) {
        let (lx, ly) = (pos.x, pos.y);

        // The shared context menu (any exhibit's) gets the pointer to itself
        // while open: its row highlight and slider rows.
        if cce_ui::widget::context_menu::is_visible() {
            if cce_ui::widget::context_menu::cursor_moved(lx, ly) {
                *needs_rebuild = true;
            }
            return;
        }

        let mut changed = false;
        if !self.is_child && self.exhibit_scroll.cursor_moved(lx, ly, &mut self.ui_context)
        {
            changed = true;
        }
        // The router owns the drag lifecycle — one
        // PointerMove per visible root forwards DragUpdate to a live drag target and
        // runs hover bookkeeping otherwise.
        let mv = cce_ui::widget::Event::PointerMove { x: lx, y: ly, local_x: lx, local_y: ly };
        {
            let vis = self.visibility();
            let is_visible = move |index: usize| vis.is_visible(index);
            for i in 0..self.roster.len() {
                if !is_visible(i) {
                    continue;
                }
                let root = self.roster.id(i);
                if self.ui_context.propagate_event(&mv, root) {
                    changed = true;
                }
            }
            if self.ui_context.is_dragging {
                changed = true;
            }
        }
        if changed {
            *needs_rebuild = true;
        }
        self.drain_widget_events_posting(needs_rebuild);
    }

    fn handle_mouse_input(&mut self, button: MouseButton, state: ElementState, pos: LogicalPosition, needs_rebuild: &mut bool) -> Option<Self::Message> {
        let (lx, ly) = (pos.x, pos.y);

        // The shared context menu a right-click on an exhibit opens takes every
        // press while open, ahead of the exhibit scrollbar: a row runs, a press
        // anywhere else dismisses it. A release it does not use (its slider
        // rows take theirs) still reaches the exhibits, whose drags end on it.
        // The toolkit leaves this routing to the app; without it the menu could
        // not be closed by clicking outside it, and its rows did nothing.
        if cce_ui::widget::context_menu::is_visible() {
            let used = cce_ui::widget::context_menu::mouse_input(button, state, lx, ly, Some(&mut self.ui_context));
            if used {
                *needs_rebuild = true;
            }
            if used || state == ElementState::Pressed {
                // A row may have set its exhibit's value (Paste, Reset).
                return self.drain_widget_events(needs_rebuild);
            }
        }

        let mut changed = false;
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);

        if state == ElementState::Pressed {
            // A press that only moves the keyboard still redraws: a focused control's lit
            // rim (a ramp's graph, a well) follows the focus, and a press no widget
            // handled — on bare floor (the exhibit scroll frame drops the focus), or a
            // ramp's body below its track — set nothing else.
            let holder_before = self.ui_context.focus_holder();
            // The exhibit area's scrollbar takes a press on its track before any
            // exhibit — while it is RAISED. A sunk bar is behind the well's floor
            // and is not hit, so a press on its lane falls through to the exhibit
            // under it (`ScrollBox::hit_test_scrollbar` gates on the latch).
            let exhibit_bar = !self.is_child && self.exhibit_scroll.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if exhibit_bar {
                changed = true;
            }
            let clicked_idx = if exhibit_bar {
                None
            } else {
                (0..self.roster.len()).rev().find(|&i| {
                    // An exhibit scrolled out of the viewport is not there to click.
                    is_visible(i)
                        && (self.is_child || !is_exhibit(i) || self.in_exhibit_viewport(lx, ly))
                        && self.roster.get_dyn(&self.ui_context, i).hit_test(lx, ly, &self.ui_context)
                })
            };
            if button == MouseButton::Left {
                // A press anywhere else takes the focus from whatever holds it — the
                // slot last clicked, a widget Tab moved to, or one inside a slot (a
                // SliderSpinbox's box being typed in). A press on the holder itself keeps
                // it: a click that moves the caret must not end the typing. The holder is
                // the Ramp, not its dropdown, while a field of the ramp's has the keyboard.
                if let Some(holder) = self.ui_context.focus_holder() {
                    let on_holder = self.ui_context.get_widget(holder).is_some_and(|w| w.hit_test(lx, ly, &self.ui_context));
                    if !on_holder {
                        self.ui_context.unfocus_id(holder);
                    }
                }
            }
            if let Some(i) = clicked_idx {
                // The router records the drag target on a handled press and synthesizes
                // DragStart past its threshold; a draggable slot whose press handler
                // returned false is armed explicitly.
                let ev = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
                let root = self.roster.id(i);
                let press_handled = self.ui_context.propagate_event(&ev, root);
                if press_handled {
                    changed = true;
                }
                if button == MouseButton::Left && !press_handled && self.roster.draggable(&self.ui_context, i) {
                    let id = self.roster.id(i);
                    self.ui_context.drag_target = Some(id);
                }
                // The slot pressed takes the focus, unless the press gave it to a widget
                // inside the slot (an embedded child that opened for typing).
                if button == MouseButton::Left && !self.focus_within(self.roster.id(i)) {
                    self.ui_context.focus_id(self.roster.id(i));
                }
            }
            if self.ui_context.focus_holder() != holder_before {
                changed = true;
            }
        } else {
            // The router delivers DragEnd to the drag target on the first propagate call
            // of a release; every visible root then sees the release (commit contract).
            if self.ui_context.is_dragging {
                changed = true;
            }
            let ev = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
            if !self.is_child {
                self.exhibit_scroll.mouse_input(button, state, lx, ly, &mut self.ui_context);
            }
            for i in 0..self.roster.len() {
                if !is_visible(i) {
                    continue;
                }
                let root = self.roster.id(i);
                if self.ui_context.propagate_event(&ev, root) {
                    changed = true;
                }
            }
        }
        if changed {
            *needs_rebuild = true;
        }
        self.drain_widget_events(needs_rebuild)
    }

    fn handle_mouse_wheel(&mut self, delta: &MouseScrollDelta, pos: LogicalPosition, needs_rebuild: &mut bool) {
        let (lx, ly) = (pos.x, pos.y);
        let mut changed = false;
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);
        let ev = cce_ui::widget::Event::MouseWheel { delta: *delta, x: lx, y: ly, local_x: lx, local_y: ly };
        // The control under the pointer gets the wheel first (wheel events are
        // hit-gated in the toolkit, so only it can take one): a slider, spinbox
        // or scrolling list adjusts itself and the page stays put. Only an
        // unclaimed wheel scrolls the exhibit area, and none while an exhibit
        // is dragged: the page is frozen under the drag (`layout_exhibits`).
        let in_exhibits = !self.is_child && self.in_exhibit_viewport(lx, ly);
        let mut widget_took_wheel = false;
        for i in 0..self.roster.len() {
            if !is_visible(i) {
                continue;
            }
            if !self.is_child && is_exhibit(i) && !in_exhibits {
                continue;
            }
            let root = self.roster.id(i);
            if self.ui_context.propagate_event(&ev, root) {
                widget_took_wheel = true;
                changed = true;
            }
        }
        if !widget_took_wheel
            && !self.is_child
            && !self.exhibit_dragging()
            && self.exhibit_scroll.mouse_wheel(delta, lx, ly, &mut self.ui_context)
        {
            changed = true;
        }
        if changed {
            *needs_rebuild = true;
        }
        self.drain_widget_events_posting(needs_rebuild);
    }

    fn handle_key_input(&mut self, event: &KeyEvent, needs_rebuild: &mut bool) -> Option<Self::Message> {
        let mut changed = false;
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);
        let mut handled = false;
        let key_ev = cce_ui::widget::Event::KeyInput(event.clone());
        let focused = self.focused_slot();
        if let Some(focused) = focused {
            let root = self.roster.id(focused);
            if self.ui_context.propagate_event(&key_ev, root) {
                changed = true;
                handled = true;
            }
        }

        if !handled {
            for i in 0..self.roster.len() {
                if Some(i) == focused {
                    continue;
                }
                if !is_visible(i) {
                    continue;
                }
                // Panel children take keys only through the focused path above.
                let root = self.roster.id(i);
                if self.ui_context.propagate_event(&key_ev, root) {
                    changed = true;
                    // A consumed key is HANDLED, not just repaint-worthy: the
                    // Escape-quits-app fallback below is gated on !handled,
                    // and without this a dropdown that took Escape through
                    // this sweep closed its menu AND exited the app.
                    handled = true;
                    // And delivered ONCE: the context routes a key to the
                    // focused widget from ANY root, so without this break a
                    // Tab-focused slider took one Right press 58 times over —
                    // once per roster root.
                    break;
                }
            }
        }

        if !handled && event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) {
            return Some("exit".to_string());
        }

        if changed {
            *needs_rebuild = true;
        }
        // A header dropdown driven by the keyboard selects on Enter, a focused button
        // clicks on Enter or Space: act on it now.
        self.drain_widget_events(needs_rebuild)
    }

}

impl cce_ui::engine::LayerApp for State {
    fn layer(&self) -> Option<LayerSettings> {
        self.child_kind.and_then(|kind| kind.layer(self.height as i32))
    }
}

/// A `Command` that re-runs this binary as a child window; the caller adds the flags.
/// The binary is found by name beside the running one (`de_bin`), not at
/// `current_exe`: once `ccebuild install` has replaced it, the running process's own
/// path reads `… (deleted)` and names nothing that can be run.
fn child_command() -> std::process::Command {
    let mut cmd = std::process::Command::new(cce_ui::process::de_bin("cce-gallery"));
    cmd.arg("--child");
    cmd
}

/// Where the editor opened from the button at `button` (its rect, in the gallery's logical
/// px) is placed: the button's bottom-left corner on screen. `window` is the gallery
/// window's top-left corner there, in layout px, as `windows --json` reports it, and `zoom`
/// the desk's zoom, as the compositor's `camera` reply reports it (`camera_zoom`): the
/// window list gives a window's box unzoomed — the zoom is the window's scale, applied when
/// it is drawn — so the button's offset into the window is scaled here. The compositor
/// lands a window a little below-right of the point it is given (`place-next`), so the
/// editor opens just under its button.
fn editor_anchor(window: (f64, f64), zoom: f64, button: (f32, f32, f32, f32)) -> (f64, f64) {
    let (wx, wy) = window;
    let (bx, by, _, bh) = button;
    (wx + bx as f64 * zoom, wy + (by + bh) as f64 * zoom)
}

/// The desk's zoom in a compositor `camera` reply (`… zoom=<z> …`); `None` when it names
/// none, or one that is not a finite positive number.
fn camera_zoom(reply: &str) -> Option<f64> {
    reply
        .split_whitespace()
        .find_map(|word| word.strip_prefix("zoom="))
        .and_then(|z| z.parse::<f64>().ok())
        .filter(|z| z.is_finite() && *z > 0.0)
}

/// Ask the compositor to open `kind`'s editor under its button instead of at the editor's
/// remembered spot — `place-next`, as a ColorSelector places its picker. The anchor is the
/// button's corner on screen (`editor_anchor`), from the gallery window's own box, found by
/// this process's pid in the window list, and the desk's zoom from `camera`; failing
/// either, the pointer, which a click leaves on the button. The window's box comes first,
/// not the pointer, so that a button pressed from the keyboard anchors as well as a
/// clicked one, and a click anchors at the button's corner rather than wherever on the
/// button it landed. One-shot and best-effort: the hint is consumed when the editor maps,
/// and ignored off cce; any failure leaves the editor to open where it would have, and the
/// spawn goes ahead. Each round trip is bounded, since this runs on the click.
fn place_editor(kind: ChildKind, button: (f32, f32, f32, f32)) {
    use cce_ui::ipc::ctl::{self, Request, WindowInfo};
    const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
    let pid = std::process::id();
    let own = ctl::request(&Request::WindowsJson, Some(REPLY_TIMEOUT))
        .ok()
        .and_then(|reply| reply.lines().filter_map(WindowInfo::from_json_line).find(|w| w.pid == Some(pid)));
    let zoom = || ctl::send("camera", Some(REPLY_TIMEOUT)).ok().as_deref().and_then(camera_zoom);
    let (x, y) = match own.and_then(|w| Some((w, zoom()?))) {
        Some((w, zoom)) => editor_anchor((w.x as f64, w.y as f64), zoom, button),
        None => {
            let Ok(reply) = ctl::request(&Request::PointerLocation, Some(REPLY_TIMEOUT)) else { return };
            let Some(at) = ctl::parse_pointer_location(&reply) else { return };
            at
        }
    };
    let place = Request::PlaceNext { app_id: kind.app_id(), x: x.round(), y: y.round(), cell: false };
    let _ = ctl::request(&place, Some(REPLY_TIMEOUT));
}

/// Opens a Ramp or ColorRamp editor window, at the kind's `default_size`. Untracked on
/// purpose: an editor outlives the gallery.
fn spawn_editor(kind: ChildKind) {
    let mut cmd = child_command();
    cmd.args(["--type", kind.arg(), "--root-plate"]);
    if let Err(e) = spawn_detached(cmd) {
        eprintln!("cce-gallery: opening the {}: {e}", kind.title());
    }
}

/// The arrangements the Layout dropdown offers, by name (`arrange`). `layout_exhibits` runs
/// them at the toolkit's control gap (`layout::control_gap()`) with no padding — the
/// exhibit area is already inset — so the page shows the rhythm controls get by default.
const LAYOUTS: [(&str, Arrangement); 4] = [
    ("Vertical", Arrangement::Vertical),
    ("Adaptive Grid", Arrangement::AdaptiveGrid { min_col_width: 190.0 }),
    ("Mosaic", Arrangement::Mosaic),
    ("Reverse Mosaic", Arrangement::ReverseMosaic),
];
/// Adaptive Grid: what the gallery opens on.
const DEFAULT_LAYOUT: usize = 1;

/// The fixed positions in a `width`-wide window: the header dropdowns, in rows `row_h`
/// tall. The exhibits are laid out by `layout_exhibits` instead.
fn demo_positions(count: usize, width: f32, row_h: f32) -> Vec<(f32, f32, f32, f32)> {
    let mut vec = vec![(0.0, 0.0, 0.0, 0.0); count];
    // The header dropdowns; the exhibits below them are laid out by
    // `layout_exhibits` with the arrangement Layout selects. Their heights are the
    // widgets' own preferred heights (`apply_layout`); the ones here are placeholders.
    // They stand on the root plate, one inset in from the window's top-left corner,
    // one root gap apart, flowing in HEADER order: each goes on the current row if it
    // ends inside the window's right inset, else starts a new row at the left inset,
    // one root gap below. A window narrower than one dropdown and its insets puts each
    // on a row of its own.
    let inset = cce_ui::layout::root_plate_inset();
    let gap = cce_ui::layout::root_plate_gap();
    let (mut x, mut y) = (inset, inset);
    for &slot in HEADER.iter() {
        if x > inset && x + HEADER_W > width - inset {
            (x, y) = (inset, y + row_h + gap);
        }
        vec[slot] = (x, y, HEADER_W, 0.0);
        x += HEADER_W + gap;
    }
    vec
}

/// Where the other page's exhibits are parked: their rects' top-left corner, far enough
/// off-screen that the whole of any exhibit lies left of and above the window.
const PARKED: f32 = -10_000.0;

/// A child window's title, painted by `display_list` when it has no MenuBar to
/// carry it; the description below leaves this line of headroom.
const CHILD_TITLE_SIZE: f32 = 16.0;

/// A simulated window's MenuBar and StatusBar rows.
const CHILD_MENUBAR_H: f32 = 40.0;
const CHILD_STATUSBAR_H: f32 = 24.0;

/// A child window's Close button's width (its height is the toolkit's button height).
const CHILD_CLOSE_W: f32 = 100.0;

/// The height a child of `sh` has between its menu bar and status bar.
fn child_inner_h(sh: f32, use_menubar: bool, use_statusbar: bool) -> f32 {
    let dy = if use_menubar { CHILD_MENUBAR_H } else { 0.0 };
    let dh = if use_statusbar { CHILD_STATUSBAR_H } else { 0.0 };
    sh - dy - dh
}

/// Whether a child window of `sh` is tall enough to stack its title line, a main slot
/// at least one control tall, and Close, each one root gap apart inside the root inset.
/// A shorter one — the Top-layer bar, a status segment — lays them out in one row.
fn child_stacks(sh: f32, use_menubar: bool, use_statusbar: bool) -> bool {
    let inset = cce_ui::layout::root_plate_inset();
    let gap = cce_ui::layout::root_plate_gap();
    let bh = cce_ui::layout::button_height();
    child_inner_h(sh, use_menubar, use_statusbar) >= 2.0 * inset + CHILD_TITLE_SIZE + 2.0 * gap + 2.0 * bh
}

/// The `ChildSlots` rects for a child window of `sw`x`sh`: 0 background, 1 main
/// (editor or description), 2 Close, 3 and 4 a simulated window's MenuBar and
/// StatusBar (an editor has no 3 or 4, and leaves those entries unused).
/// Everything stands on the child's root plate: one root inset in from the
/// window's edges, one root gap between the main slot and Close.
fn child_positions(sw: f32, sh: f32, use_menubar: bool, use_statusbar: bool) -> Vec<(f32, f32, f32, f32)> {
    let dy = if use_menubar { CHILD_MENUBAR_H } else { 0.0 };
    let inner_h = child_inner_h(sh, use_menubar, use_statusbar);
    let inset = cce_ui::layout::root_plate_inset();
    let gap = cce_ui::layout::root_plate_gap();
    let close_h = cce_ui::layout::button_height();
    let mut vec = vec![(-1000.0, -1000.0, 0.0, 0.0); CHILD_COUNT];

    if use_menubar {
        vec[3] = (0.0, 0.0, sw, CHILD_MENUBAR_H);
    }
    if use_statusbar {
        vec[4] = (0.0, sh - CHILD_STATUSBAR_H, sw, CHILD_STATUSBAR_H);
    }

    vec[0] = (0.0, dy, sw, inner_h);
    if child_stacks(sh, use_menubar, use_statusbar) {
        // Close is centred, one inset up from the bottom edge; the main slot fills the
        // room above it, one root gap off, under the title line's headroom (kept whether
        // the title is painted or a MenuBar carries it).
        let close_y = dy + inner_h - inset - close_h;
        vec[2] = ((sw - CHILD_CLOSE_W) / 2.0, close_y, CHILD_CLOSE_W, close_h);
        let main_y = dy + inset + CHILD_TITLE_SIZE + gap;
        vec[1] = (inset, main_y, sw - 2.0 * inset, close_y - gap - main_y);
    } else {
        // One row: Close at the right end, the description filling the room to its
        // left one root gap off (a Label clips what does not fit), both centred in the
        // band. The margin is the root inset where the band has room for it, else
        // what is left around the button — never less than half a gap at the sides.
        let close_h = close_h.min(inner_h).max(0.0);
        let close_w = CHILD_CLOSE_W.min(sw / 2.0);
        let margin = inset.min((inner_h - close_h) / 2.0).max(0.0);
        let side = margin.max(gap / 2.0);
        let close_x = sw - side - close_w;
        vec[2] = (close_x, dy + (inner_h - close_h) / 2.0, close_w, close_h);
        vec[1] = (side, dy + margin, (close_x - gap - side).max(0.0), inner_h - 2.0 * margin);
    }

    vec
}

fn main() {
    cce_ui::engine::run::<State>();
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [ChildKind; 9] = [
        ChildKind::Floating,
        ChildKind::Fullscreen,
        ChildKind::Utility,
        ChildKind::LayerTop,
        ChildKind::LayerOverlay,
        ChildKind::LayerBackground,
        ChildKind::Status,
        ChildKind::Ramp,
        ChildKind::ColorRamp,
    ];

    #[test]
    fn every_kind_round_trips_through_its_type_arg() {
        for kind in KINDS {
            assert_eq!(ChildKind::from_arg(kind.arg()), Some(kind));
        }
    }

    /// The slot numbers `exhibit_table` sizes are exactly the ones `is_exhibit` lays out:
    /// a slot missing from the table is never placed, one listed twice is placed twice.
    #[test]
    fn the_exhibit_table_lists_every_exhibit_once() {
        let mut ui = UiContext::new();
        let extra = variant_exhibits(&mut ui);
        let mut listed: Vec<usize> = exhibit_table(120.0, 300.0, 250.0, &extra).iter().map(|e| e.slot).collect();
        listed.sort_unstable();
        let exhibits: Vec<usize> = (0..GALLERY_COUNT + extra.len()).filter(|&i| is_exhibit(i)).collect();
        assert_eq!(listed, exhibits);
    }

    /// The loose lasso's variant slots are the StatusDot's variants, so the frame holds the
    /// four dots and nothing else.
    #[test]
    fn the_loose_lasso_holds_the_status_dots() {
        let mut ui = UiContext::new();
        let extra = variant_exhibits(&mut ui);
        for slot in STATUS_DOT_VARIANTS {
            let e = &extra[slot - GALLERY_COUNT];
            assert_eq!(e.after, Some(22), "slot {slot} is no StatusDot variant");
            let label = ui.get_widget(e.id).and_then(|w| w.label()).unwrap_or_default();
            assert!(label.starts_with("StatusDot ("), "slot {slot} is {label:?}");
        }
    }

    /// Every variant names the widget it is a look of, and the table places it right after
    /// that widget, with only that widget's other variants between: related exhibits sit side
    /// by side.
    #[test]
    fn every_variant_follows_its_widget() {
        let mut ui = UiContext::new();
        let extra = variant_exhibits(&mut ui);
        let order: Vec<usize> = exhibit_table(120.0, 300.0, 250.0, &extra).iter().map(|e| e.slot).collect();
        for (k, e) in extra.iter().enumerate() {
            let base = e.after.unwrap_or_else(|| panic!("variant {k} names no widget"));
            let (b, v) = (order.iter().position(|&s| s == base).unwrap(), order.iter().position(|&s| s == GALLERY_COUNT + k).unwrap());
            assert!(v > b, "variant {k} is before slot {base}");
            for &between in &order[b + 1..v] {
                let of = between.checked_sub(GALLERY_COUNT).and_then(|j| extra[j].after);
                assert_eq!(of, Some(base), "slot {between} parts variant {k} from slot {base}");
            }
        }
    }

    /// The gallery as it starts, in gallery mode.
    fn gallery() -> State {
        let (tx, _rx) = calloop::channel::channel::<String>();
        <State as cce_ui::engine::Application>::create(tx.into())
    }

    /// Whether slot `i` is an exhibit of the page `st` shows.
    fn on_page(st: &State, i: usize) -> bool {
        is_exhibit(i) && st.is_widget_visible(i)
    }

    fn overlap(a: cce_ui::scene::layout::Rect, b: cce_ui::scene::layout::Rect) -> bool {
        // Half a pixel of slack: abutting rects are not overlapping ones.
        let w = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
        let h = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
        w > 0.5 && h > 0.5
    }

    /// In every layout, at window sizes from narrow to wide, no exhibit overlaps another —
    /// control or detached label — and every exhibit lies across the well's floor; no
    /// Group's frame (out to its wall's outer half) or title tab takes in an exhibit that is
    /// not its member, its tab covers no member either, and the two Groups stay apart. The
    /// sizes run from ones the exhibits overflow (they scroll) to ones they fit with room
    /// to spare (Reverse Mosaic spreads them out).
    #[test]
    fn no_layout_overlaps_its_exhibits() {
        use cce_ui::scene::layout::Rect;
        let mut st = gallery();
        let mut failures = Vec::new();
        let sizes = [(600.0, 800.0), (726.0, 912.0), (800.0, 600.0), (1000.0, 680.0), (1280.0, 800.0), (1600.0, 1000.0), (2400.0, 1400.0)];
        for (sw, sh) in sizes {
            st.width = sw;
            st.height = sh;
            st.relayout();
            for (li, (name, _)) in LAYOUTS.iter().enumerate() {
                st.layout_idx = li;
                st.apply_layout();
                let ui = &st.ui_context;
                let (floor, _) = st.exhibit_floor();
                let at = format!("{sw}x{sh} {name}");
                // Each exhibit's footprint: its rect (content and label strip) and its label.
                let footprints: Vec<(usize, Rect)> = (0..st.roster.len())
                    .filter(|&i| on_page(&st, i))
                    .map(|i| {
                        let w = st.roster.get_dyn(ui, i);
                        let (x, y, ww, hh) = w.rect();
                        let mut r = Rect { x, y, width: ww, height: hh };
                        if let Some(l) = w.detached_label_rect() {
                            let (x1, y1) = ((r.x + r.width).max(l.x + l.width), (r.y + r.height).max(l.y + l.height));
                            r.x = r.x.min(l.x);
                            r.y = r.y.min(l.y);
                            r.width = x1 - r.x;
                            r.height = y1 - r.y;
                        }
                        (i, r)
                    })
                    .collect();
                for (k, &(i, a)) in footprints.iter().enumerate() {
                    for &(j, b) in &footprints[k + 1..] {
                        if overlap(a, b) {
                            failures.push(format!("{at}: slot {i} {a:?} overlaps slot {j} {b:?}"));
                        }
                    }
                    if a.x < floor.x - 0.5 || a.x + a.width > floor.x + floor.width + 0.5 {
                        failures.push(format!("{at}: slot {i} {a:?} runs off the floor {floor:?}"));
                    }
                }
                let s = st.roster.gallery();
                let wall = cce_ui::layout::bevel_width() * 0.5;
                let mut frames = Vec::new();
                for (group, members) in GROUPS {
                    let Some(f) = ui[s.group(group)].inner().frame(ui) else {
                        failures.push(format!("{at}: Group {group} has no frame"));
                        continue;
                    };
                    let body = Rect { x: f.body.x - wall, y: f.body.y - wall, width: f.body.width + 2.0 * wall, height: f.body.height + 2.0 * wall };
                    for &(i, r) in &footprints {
                        let member = members.contains(&i);
                        if !member && overlap(body, r) {
                            failures.push(format!("{at}: Group {group}'s frame {body:?} takes in slot {i} {r:?}"));
                        }
                        if f.tab.is_some_and(|t| overlap(t, r)) {
                            failures.push(format!("{at}: Group {group}'s tab {:?} covers slot {i} {r:?}", f.tab));
                        }
                    }
                    frames.push(body);
                }
                if let [a, b] = frames[..] {
                    if overlap(a, b) {
                        failures.push(format!("{at}: the Groups overlap, {a:?} and {b:?}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{} overlaps:\n{}", failures.len(), failures.join("\n"));
    }

    /// Each Group's frame stands one padding off its members on every side — their footprints,
    /// labels included — in every layout and at every size: the padding hangs outside the
    /// members' block, so the fitted lasso, whose sides nearest the well's edge land on their
    /// seat, has nothing to snap. (A side the fit does move, landing within `snap` of the
    /// edge, is on its seat, and is let be.)
    #[test]
    fn a_groups_padding_is_even_on_every_side() {
        use cce_ui::scene::layout::Rect;
        let mut st = gallery();
        let mut failures = Vec::new();
        for (sw, sh) in [(600.0, 800.0), (726.0, 912.0), (800.0, 600.0), (1000.0, 680.0), (1280.0, 800.0), (1600.0, 1000.0)] {
            st.width = sw;
            st.height = sh;
            st.relayout();
            for (li, (name, _)) in LAYOUTS.iter().enumerate() {
                st.layout_idx = li;
                st.apply_layout();
                let ui = &st.ui_context;
                let (floor, _) = st.exhibit_floor();
                let s = st.roster.gallery();
                for (group, members) in GROUPS {
                    let g = ui[s.group(group)].inner();
                    let (p, f) = (g.padding(), g.frame(ui).expect("a frame"));
                    let hull = members
                        .iter()
                        .map(|&i| {
                            let w = st.roster.get_dyn(ui, i);
                            let (x, y, ww, hh) = w.rect();
                            let mut r = Rect { x, y, width: ww, height: hh };
                            if let Some(l) = w.detached_label_rect() {
                                let (x1, y1) = ((r.x + r.width).max(l.x + l.width), (r.y + r.height).max(l.y + l.height));
                                (r.x, r.y) = (r.x.min(l.x), r.y.min(l.y));
                                (r.width, r.height) = (x1 - r.x, y1 - r.y);
                            }
                            r
                        })
                        .reduce(|a, b| {
                            let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
                            Rect { x: x0, y: y0, width: (a.x + a.width).max(b.x + b.width) - x0, height: (a.y + a.height).max(b.y + b.height) - y0 }
                        })
                        .unwrap();
                    let b = f.body;
                    let tab = g.tab_height();
                    let sides = [
                        ("left", hull.x - b.x, (b.x - (floor.x + p)).abs()),
                        ("top", hull.y - b.y, (b.y - (floor.y + p + tab)).abs()),
                        ("right", b.x + b.width - (hull.x + hull.width), (floor.x + floor.width - p - (b.x + b.width)).abs()),
                        ("bottom", b.y + b.height - (hull.y + hull.height), (floor.y + floor.height - p - (b.y + b.height)).abs()),
                    ];
                    for (side, gap, off_seat) in sides {
                        if (gap - p).abs() > 0.5 && off_seat > 0.5 {
                            failures.push(format!("{sw}x{sh} {name}: Group {group}'s {side} padding is {gap:.1}, not {p}"));
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The Checkbox exhibit is as wide as what it draws — its box and its label — not the
    /// 190 px a control row gets: the label ends within a couple of pixels of its rect's right
    /// edge, and inside it, in every layout.
    #[test]
    fn the_checkbox_exhibit_hugs_its_box_and_label() {
        let mut st = gallery();
        for (li, (name, _)) in LAYOUTS.iter().enumerate() {
            st.layout_idx = li;
            st.apply_layout();
            let w = st.roster.get_dyn(&st.ui_context, CHECKBOX_SLOT);
            let (x, _, width, _) = w.rect();
            let (family, _) = cce_ui::layout::control_label_font_parsed();
            let label_end = w
                .painted_prims()
                .into_iter()
                .find_map(|p| match p {
                    Prim::Text { text, x, font_size, .. } => Some(x + cce_ui::widget::display::measure_text_width(&text, &family, font_size)),
                    _ => None,
                })
                .expect("the Checkbox draws its label");
            assert!(width < 150.0, "{name}: {width} wide");
            assert!(label_end <= x + width, "{name}: the label ends at {label_end}, past {}", x + width);
            assert!(x + width - label_end <= 2.0, "{name}: {} px past the label", x + width - label_end);
        }
    }

    /// The default is an index into `LAYOUTS`: adding or removing a layout shifts it.
    #[test]
    fn the_gallery_opens_on_adaptive_grid() {
        assert_eq!(LAYOUTS[DEFAULT_LAYOUT].0, "Adaptive Grid");
    }

    /// Adaptive Grid keeps every column its minimum width allows at a tiled window's width
    /// and at the default one: no exhibit is wider than the minimum (`grid_columns` drops a
    /// column for one that is). Every exhibit outside a Group starts one of the columns.
    #[test]
    fn adaptive_grid_keeps_every_column_its_minimum_allows() {
        let mut st = gallery();
        let gap = cce_ui::layout::control_gap();
        let (grid, min_col_width) = LAYOUTS
            .iter()
            .enumerate()
            .find_map(|(i, (_, k))| match *k {
                Arrangement::AdaptiveGrid { min_col_width } => Some((i, min_col_width)),
                _ => None,
            })
            .unwrap();
        for (sw, sh) in [(726.0, 912.0), (1000.0, 680.0)] {
            st.width = sw;
            st.height = sh;
            st.relayout();
            st.layout_idx = grid;
            st.apply_layout();
            // The area `layout_exhibits` arranges in: the floor, two paddings in a side.
            let pad = st.ui_context[st.roster.gallery().group_fitted].inner().padding();
            let width = st.exhibit_floor().0.width - 4.0 * pad;
            let columns = ((width + gap) / (min_col_width + gap)).floor() as usize;
            // A Group's members share a row inside their column when they fit: not columns.
            let member = |i: usize| GROUPS.iter().any(|(_, m)| m.contains(&i));
            let mut starts: Vec<i32> = (0..st.roster.len())
                .filter(|&i| on_page(&st, i) && !member(i))
                .map(|i| st.roster.get_dyn(&st.ui_context, i).rect().0.round() as i32)
                .collect();
            starts.sort_unstable();
            starts.dedup();
            assert_eq!(starts.len(), columns, "{sw}x{sh}: exhibits start at {starts:?}");
        }
    }

    /// Down every column of the stacking layouts — Vertical and Adaptive Grid — the gap from one
    /// exhibit's foot to the next one's top is the control gap, whether the next has a label
    /// or not: each reserves its own label strip and no shared row (a shared one put 46 px
    /// over an unlabeled exhibit and 26 over a labeled one). A Group counts by its frame,
    /// tab to foot: measured by its members, as it was, two stacked groups stood half a
    /// gap apart. An exhibit across several columns counts in each of them, and above it a
    /// shorter column may leave more than the gap (a hole the grid never fills) — but the
    /// tallest column of its run, the one it is placed below, leaves exactly the gap. So
    /// across a family's start (`FAMILY_HEADS`): the family begins exactly the gap below the
    /// lowest foot of all before it, and a shorter column leaves more.
    #[test]
    fn stacked_layouts_space_their_columns_evenly() {
        let mut st = gallery();
        let gap = cce_ui::layout::control_gap();
        let mut failures = Vec::new();
        let stacking = |k: &Arrangement| matches!(k, Arrangement::Vertical | Arrangement::AdaptiveGrid { .. });
        for (sw, sh) in [(600.0, 800.0), (726.0, 912.0), (1000.0, 680.0), (1600.0, 1000.0)] {
            st.width = sw;
            st.height = sh;
            st.relayout();
            for (li, (name, kind)) in LAYOUTS.iter().enumerate() {
                if !stacking(kind) {
                    continue;
                }
                st.layout_idx = li;
                st.apply_layout();
                let ui = &st.ui_context;
                let member = |i: usize| GROUPS.iter().any(|(_, m)| m.contains(&i));
                // (left, right, top, bottom, what)
                let mut found: Vec<(f32, f32, f32, f32, String)> = (0..st.roster.len())
                    .filter(|&i| on_page(&st, i) && !member(i))
                    .map(|i| {
                        let w = st.roster.get_dyn(ui, i);
                        let (x, y, width, h) = w.rect();
                        let top = w.detached_label_rect().map_or(y, |l| l.y.min(y));
                        (x, x + width, top, y + h, format!("slot {i}"))
                    })
                    .collect();
                for (group, _) in GROUPS {
                    let g = ui[st.roster.gallery().group(group)].inner();
                    let (p, f) = (g.padding(), g.frame(ui).expect("a frame"));
                    let tab = f.tab.map_or(f.body.y - g.tab_height(), |t| t.y);
                    found.push((f.body.x + p, f.body.x + f.body.width - p, tab, f.body.y + f.body.height, format!("Group {group}")));
                }
                // Each unit in every column it lies across: (column x, top, bottom, what,
                // whether it spans).
                let mut starts: Vec<i32> = found.iter().map(|u| u.0.round() as i32).collect();
                starts.sort_unstable();
                starts.dedup();
                let mut units: Vec<(i32, f32, f32, String, bool)> = Vec::new();
                for (left, right, top, bottom, what) in &found {
                    let cols: Vec<i32> = starts.iter().copied().filter(|&c| c >= left.round() as i32 && (c as f32) < *right).collect();
                    let spans = cols.len() > 1;
                    units.extend(cols.into_iter().map(|c| (c, *top, *bottom, what.clone(), spans)));
                }
                units.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
                // Each unit's family, in the table's order; a lasso is its first member's.
                let mut family: std::collections::HashMap<String, usize> = Default::default();
                let mut f = 0;
                for (k, e) in st.exhibit_sizes().iter().enumerate() {
                    if k > 0 && FAMILY_HEADS.contains(&e.slot) {
                        f += 1;
                    }
                    family.insert(format!("slot {}", e.slot), f);
                }
                for (group, members) in GROUPS {
                    let of = family[&format!("slot {}", members[0])];
                    family.insert(format!("Group {group}"), of);
                }
                // Each family starts the gap below the lowest foot of every family before it.
                let families = family.values().copied().max().unwrap_or(0);
                for f in 1..=families {
                    let top = found.iter().filter(|u| family[&u.4] == f).map(|u| u.2).fold(f32::MAX, f32::min);
                    let above = found.iter().filter(|u| family[&u.4] < f).map(|u| u.3).fold(f32::MIN, f32::max);
                    if (top - above - gap).abs() > 0.5 {
                        failures.push(format!("{sw}x{sh} {name}: family {f} starts {:.1} below the one before, not {gap}", top - above));
                    }
                }
                // Above each spanning unit, the smallest gap over the columns it lies across.
                let mut above_span: std::collections::BTreeMap<String, f32> = Default::default();
                for pair in units.windows(2) {
                    let (a, b) = (&pair[0], &pair[1]);
                    if a.0 != b.0 {
                        continue;
                    }
                    let between = b.1 - a.2;
                    if family[&a.3] != family[&b.3] {
                        // Across a family's start: levelled above, checked as a whole.
                        if between < gap - 0.5 {
                            failures.push(format!("{sw}x{sh} {name}: {} to {} is {between:.1}, under {gap}", a.3, b.3));
                        }
                    } else if b.4 {
                        let least = above_span.entry(b.3.clone()).or_insert(f32::MAX);
                        *least = least.min(between);
                    } else if (between - gap).abs() > 0.5 {
                        failures.push(format!("{sw}x{sh} {name}: {} to {} is {between:.1}, not {gap}", a.3, b.3));
                    }
                }
                for (what, least) in above_span {
                    if (least - gap).abs() > 0.5 {
                        failures.push(format!("{sw}x{sh} {name}: above {what} the least gap is {least:.1}, not {gap}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The index of Adaptive Grid in `LAYOUTS`.
    fn adaptive_grid() -> usize {
        LAYOUTS.iter().position(|(_, k)| matches!(k, Arrangement::AdaptiveGrid { .. })).unwrap()
    }

    /// Under Adaptive Grid the Ramp lies across two columns, as the TreeList does, at the
    /// height the table gives it rather than the one it declares (`own_h`): its graph is
    /// twice its default. The ColorRamp lies across two as well, at the height it declares.
    #[test]
    fn the_ramps_lie_across_two_grid_columns_the_ramp_at_its_table_height() {
        let mut st = gallery();
        st.layout_idx = adaptive_grid();
        for (sw, sh) in [(726.0, 912.0), (1000.0, 680.0), (1400.0, 1000.0)] {
            st.width = sw;
            st.height = sh;
            st.relayout();
            st.apply_layout();
            let sizes = st.exhibit_sizes();
            let size = |slot: usize| *sizes.iter().find(|e| e.slot == slot).unwrap();
            let rect = |slot: usize| st.roster.get_dyn(&st.ui_context, slot).rect();
            // The content's height: the rect less the label strip above it.
            let content_h = |slot: usize| {
                let w = st.roster.get_dyn(&st.ui_context, slot);
                w.rect().3 - w.label_strip()
            };
            let tree_w = rect(TREELIST_SLOT).2;
            assert!(tree_w > 2.0 * 190.0, "{sw}x{sh}: the TreeList is {tree_w} wide");
            for slot in [RAMP_SLOT, COLOR_RAMP_SLOT] {
                assert_eq!(size(slot).span, 2, "slot {slot}");
                assert!((rect(slot).2 - tree_w).abs() < 0.5, "{sw}x{sh}: slot {slot} is {} wide, the TreeList {tree_w}", rect(slot).2);
            }
            let ramp = size(RAMP_SLOT);
            assert!(ramp.own_h);
            assert_eq!(ramp.h, Ramp::expanded_height() + 120.0);
            assert!((content_h(RAMP_SLOT) - ramp.h).abs() < 0.5, "{sw}x{sh}: the Ramp is {} tall, not {}", content_h(RAMP_SLOT), ramp.h);
            let crh = st.ui_context[st.roster.gallery().color_ramp_demo].natural_height();
            assert!((content_h(COLOR_RAMP_SLOT) - crh).abs() < 0.5, "{sw}x{sh}: the ColorRamp is {} tall, not {crh}", content_h(COLOR_RAMP_SLOT));
        }
    }

    /// A point on the well's floor, inside the viewport, that no exhibit on the page takes:
    /// where a wheel scrolls the area rather than a control.
    fn bare_floor_point(st: &State) -> (f32, f32) {
        let (floor, _) = st.exhibit_floor();
        let (_, vy, _, vh) = st.exhibit_viewport();
        let bottom = (floor.y + floor.height).min(vy + vh) - 4.0;
        let mut y = floor.y + 4.0;
        while y < bottom {
            let mut x = floor.x + 4.0;
            while x < floor.x + floor.width - 4.0 {
                let taken = (0..st.roster.len()).any(|i| st.is_widget_visible(i) && st.roster.get_dyn(&st.ui_context, i).hit_test(x, y, &st.ui_context));
                if !taken && st.in_exhibit_viewport(x, y) {
                    return (x, y);
                }
                x += 8.0;
            }
            y += 8.0;
        }
        panic!("no bare floor at {}x{}", st.width, st.height);
    }

    /// While an exhibit is dragged — a Ramp key or a ColorRamp key held from the press on —
    /// the page is frozen: no exhibit moves, the dragged one included, and none lands over
    /// it — not when a wheel glide set going before the drag coasts on, nor when a wheel
    /// the dragged exhibit does not take turns on the floor, nor when the window is resized.
    /// Let go, the page arranges afresh for the size it now has. Left out of the
    /// arrangement, the dragged exhibit let the ones after it slide into its place; held in
    /// it while the rest were landed, a scroll or a resize landed them over it.
    #[test]
    fn a_dragged_exhibit_keeps_its_place_and_the_page_does_not_reflow() {
        use cce_ui::engine::{Application, LogicalPosition};
        let rects = |st: &State| -> Vec<(usize, (f32, f32, f32, f32))> {
            (0..st.roster.len()).filter(|&i| on_page(st, i)).map(|i| (i, st.roster.get_dyn(&st.ui_context, i).rect())).collect()
        };
        let mut st = gallery();
        let mut fresh = gallery();
        let mut scrolled = false;
        for (sw, sh) in [(726.0, 912.0), (1400.0, 1000.0)] {
            let (rw, rh) = (sw + 180.0, sh - 120.0);
            for (li, (name, _)) in LAYOUTS.iter().enumerate() {
                for slot in [RAMP_SLOT, COLOR_RAMP_SLOT] {
                    (st.width, st.height) = (sw, sh);
                    st.layout_idx = li;
                    st.exhibit_scroll.scroll_y = 0.0;
                    st.relayout();
                    st.apply_layout();
                    st.exhibit_scroll.tick(0.016, &mut st.ui_context);
                    // A wheel on the bare floor, then a frame: a glide in flight when the drag
                    // begins, where the motion settings make one.
                    let bare = bare_floor_point(&st);
                    let bare = LogicalPosition::new(bare.0, bare.1);
                    let wheel = MouseScrollDelta::LineDelta(0.0, -3.0);
                    let mut nr = false;
                    st.handle_mouse_wheel(&wheel, bare, &mut nr);
                    st.tick(0.016, &mut nr);
                    st.layout_exhibits();
                    scrolled |= st.exhibit_scroll.scroll_y > 0.0;
                    let before = rects(&st);
                    let set = |st: &mut State, on: bool| {
                        let s = st.roster.gallery();
                        let (ramp, color_ramp) = (s.bevel_ramp, s.color_ramp_demo);
                        match slot {
                            RAMP_SLOT => st.ui_context[ramp].is_dragging_key = on,
                            _ => st.ui_context[color_ramp].is_dragging_key = on,
                        }
                    };
                    set(&mut st, true);
                    assert!(st.roster.is_dragging(&st.ui_context, slot));
                    let held_y = st.exhibit_scroll.scroll_y;
                    let at = format!("{sw}x{sh} {name}, slot {slot} dragged");
                    // The glide coasts, a wheel turns on the floor, the window is resized.
                    for _ in 0..10 {
                        st.tick(0.05, &mut nr);
                        st.layout_exhibits();
                    }
                    st.handle_mouse_wheel(&wheel, bare, &mut nr);
                    st.layout_exhibits();
                    (st.width, st.height) = (rw, rh);
                    st.relayout();
                    st.apply_layout();
                    st.layout_exhibits();
                    assert_eq!(st.exhibit_scroll.scroll_y, held_y, "{at}: the area scrolled");
                    let during = rects(&st);
                    let held = before.iter().find(|(i, _)| *i == slot).unwrap().1;
                    let held = cce_ui::scene::layout::Rect { x: held.0, y: held.1, width: held.2, height: held.3 };
                    for ((i, a), (_, b)) in before.iter().zip(&during) {
                        assert_eq!(a, b, "{at}: slot {i} moved");
                        let r = cce_ui::scene::layout::Rect { x: b.0, y: b.1, width: b.2, height: b.3 };
                        assert!(*i == slot || !overlap(held, r), "{at}: slot {i} {r:?} lands over the dragged slot {slot}");
                    }
                    // Let go: arranged as a page that was never frozen is at this size.
                    set(&mut st, false);
                    st.layout_exhibits();
                    (fresh.width, fresh.height) = (rw, rh);
                    fresh.layout_idx = li;
                    fresh.relayout();
                    fresh.exhibit_scroll.scroll_y = st.exhibit_scroll.scroll_y;
                    fresh.apply_layout();
                    assert_eq!(rects(&st), rects(&fresh), "{at}: not re-arranged on the release");
                }
            }
        }
        assert!(scrolled, "the wheel never scrolled the area: the test scrolls nothing");
    }

    /// An editor opens under its button: the anchor is the button's bottom-left corner on
    /// screen — the gallery window's corner plus the button's offset into it, scaled by the
    /// desk's zoom. The window list gives the window's box unzoomed, so the zoom is the
    /// `camera` reply's, read off it as the compositor words it.
    #[test]
    fn an_editor_is_anchored_at_its_buttons_foot() {
        let button = (40.0, 300.0, 190.0, 32.0);
        assert_eq!(editor_anchor((100.0, 50.0), 1.0, button), (140.0, 382.0));
        // Zoomed out to half: the 1000 px window (as `windows --json` still reports it) is
        // drawn 500 px wide, and the button's offset into it halves.
        let zoom = camera_zoom("pan_x=-12.0 pan_y=4.5 zoom=0.500 target_x=- target_y=- anim=false mode=Normal\n").unwrap();
        assert_eq!(editor_anchor((100.0, 50.0), zoom, button), (120.0, 216.0));
        // The `zoom-out` step, as two skeptics read it off a shadow.
        let zoom = camera_zoom("pan_x=0.0 pan_y=0.0 zoom=0.909 target_x=- target_y=- anim=false mode=Normal").unwrap();
        let (x, y) = editor_anchor((100.0, 50.0), zoom, button);
        assert!((x - 136.36).abs() < 0.01 && (y - 351.79).abs() < 0.01, "({x}, {y})");
        // A reply with no usable zoom gives none: the pointer anchors instead.
        for reply in ["", "error: unknown command", "zoom=nan", "zoom=0", "zoom=-1", "pan_x=0 zoom="] {
            assert_eq!(camera_zoom(reply), None, "{reply:?}");
        }
    }

    /// The variant exhibits are widgets in another style: the Paginator, which holds no
    /// pages of its own here and is not a widget that stands alone, is not among them.
    #[test]
    fn no_exhibit_is_a_paginator() {
        let st = gallery();
        for i in (0..st.roster.len()).filter(|&i| on_page(&st, i)) {
            let w = st.roster.get_dyn(&st.ui_context, i);
            assert!(w.as_any().downcast_ref::<cce_ui::widget::Paginator>().is_none(), "slot {i} is a Paginator");
        }
    }

    /// Scrolled all the way down, in every layout and at every window size, the lowest
    /// exhibit ends as far above the well's floor as the first row starts below it — two
    /// paddings — so none is cut off by the wall at the foot of the area.
    #[test]
    fn scrolled_to_the_bottom_every_exhibit_clears_the_floor() {
        let mut st = gallery();
        let pad = st.ui_context[st.roster.gallery().group_fitted].inner().padding();
        let mut failures = Vec::new();
        for (sw, sh) in [(600.0, 800.0), (726.0, 912.0), (800.0, 600.0), (1000.0, 680.0), (1280.0, 800.0), (1600.0, 1000.0)] {
            st.width = sw;
            st.height = sh;
            st.relayout();
            for (li, (name, _)) in LAYOUTS.iter().enumerate() {
                st.layout_idx = li;
                st.exhibit_scroll.scroll_y = f32::MAX;
                st.apply_layout();
                let (floor, _) = st.exhibit_floor();
                let foot = floor.y + floor.height - 2.0 * pad;
                let lowest = (0..st.roster.len())
                    .filter(|&i| on_page(&st, i))
                    .map(|i| {
                        let (_, y, _, h) = st.roster.get_dyn(&st.ui_context, i).rect();
                        (y + h, i)
                    })
                    .fold((f32::MIN, 0), |a, b| if b.0 > a.0 { b } else { a });
                if lowest.0 > foot + 0.5 {
                    failures.push(format!("{sw}x{sh} {name}: slot {} ends at {} below {foot}", lowest.1, lowest.0));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// An exhibit's footprint: its rect (content and label strip) and its detached label.
    fn footprint(w: &dyn WidgetHost) -> cce_ui::scene::layout::Rect {
        let (x, y, ww, hh) = w.rect();
        let mut r = cce_ui::scene::layout::Rect { x, y, width: ww, height: hh };
        if let Some(l) = w.detached_label_rect() {
            let (x1, y1) = ((r.x + r.width).max(l.x + l.width), (r.y + r.height).max(l.y + l.height));
            (r.x, r.y) = (r.x.min(l.x), r.y.min(l.y));
            (r.width, r.height) = (x1 - r.x, y1 - r.y);
        }
        r
    }

    /// The roster slot whose widget is `id` or holds it in the ui tree.
    fn slot_holding(st: &State, id: WidgetId) -> Option<usize> {
        let mut at = Some(id);
        while let Some(cur) = at {
            if let Some(i) = (0..st.roster.len()).find(|&i| st.roster.id(i) == cur) {
                return Some(i);
            }
            at = st.ui_context.tree.parent_id(cur);
        }
        None
    }

    /// The text a frame of the gallery paints.
    fn painted_text(st: &mut State) -> Vec<String> {
        let size = cce_ui::engine::LogicalSize::new(st.width, st.height);
        let list = <State as cce_ui::engine::Application>::display_list(st, size, 1.0).expect("a frame");
        list.items
            .into_iter()
            .filter_map(|it| match it.prim {
                Prim::Text { text, .. } => Some(text),
                _ => None,
            })
            .collect()
    }

    /// On the Relief page no widget exhibit is painted, hit or a Tab stop — they are hidden
    /// and parked off-screen, embedded children and all — and every tile is shown. A press
    /// where a widget exhibit stood reaches none of them; the frame's text has every tile's
    /// caption and none of the widgets'.
    #[test]
    fn the_relief_page_hides_every_widget_exhibit_and_shows_its_tiles() {
        use cce_ui::engine::{Application, LogicalPosition};
        let mut st = gallery();
        (st.width, st.height) = (1000.0, 680.0);
        st.relayout();
        st.apply_layout();
        let len = st.roster.len();
        let tiles_start = st.roster.gallery().tiles_start();
        let widgets: Vec<usize> = (0..len).filter(|&i| on_page(&st, i)).collect();
        let tiles: Vec<usize> = (tiles_start..len).collect();
        assert!(widgets.iter().all(|&i| i < tiles_start), "a tile is on the widget page");
        assert_eq!(tiles.len(), Effect::ALL.len());
        for &i in &tiles {
            assert!(!on_page(&st, i) && !st.roster.get_dyn(&st.ui_context, i).visible(), "tile {i} shows on the widget page");
        }
        // The widget page paints its own captions, which the Relief page must not.
        let shown = painted_text(&mut st);
        let names = ["Checkbox", "Slider", "ProgressBar", "Trackpad", "TreeList", "Group (fitted)"];
        for name in names {
            assert!(shown.iter().any(|t| t == name), "the widget page paints no {name:?}");
        }
        let centres: Vec<(f32, f32)> = widgets
            .iter()
            .map(|&i| {
                let (x, y, w, h) = st.roster.get_dyn(&st.ui_context, i).rect();
                (x + w * 0.5, y + h * 0.5)
            })
            .filter(|&(x, y)| st.in_exhibit_viewport(x, y))
            .collect();
        assert!(centres.len() > 10);

        st.set_page(Page::Relief);
        for &i in &widgets {
            let w = st.roster.get_dyn(&st.ui_context, i);
            let (x, y, ww, hh) = w.rect();
            assert!(!st.is_widget_visible(i) && !w.visible(), "slot {i} is visible");
            assert!(x + ww <= 0.0 && y + hh <= 0.0, "slot {i} is on screen at {:?}", w.rect());
        }
        for &i in &tiles {
            assert!(on_page(&st, i) && st.roster.get_dyn(&st.ui_context, i).visible(), "tile {i} is hidden");
        }
        // The Tab stops are the header's: no widget exhibit, nor a field inside one.
        for id in st.ui_context.focus_clusters().concat() {
            let slot = slot_holding(&st, id);
            let w = st.ui_context.get_widget(id).unwrap();
            assert!(slot.is_some_and(|i| HEADER.contains(&i)), "a Tab stop in slot {slot:?}: {:?} at {:?}", w.focus_role(), w.rect());
        }
        for (x, y) in centres {
            let mut nr = false;
            let at = LogicalPosition::new(x, y);
            st.handle_mouse_input(MouseButton::Left, ElementState::Pressed, at, &mut nr);
            st.handle_mouse_input(MouseButton::Left, ElementState::Released, at, &mut nr);
            let holder = st.ui_context.focused_widget.and_then(|id| slot_holding(&st, id));
            assert!(holder.is_none_or(|i| i >= tiles_start), "a press at ({x}, {y}) focused slot {holder:?}");
        }
        let shown = painted_text(&mut st);
        for name in names {
            assert!(!shown.iter().any(|t| t == name), "the Relief page paints {name:?}");
        }
        for effect in Effect::ALL {
            assert!(shown.iter().any(|t| t == effect.caption()), "no caption {:?}", effect.caption());
        }
    }

    /// Back on the widget page every exhibit is where it was, shown and a Tab stop again,
    /// and the tiles are parked in their turn.
    #[test]
    fn switching_back_restores_the_widget_page() {
        let mut st = gallery();
        for (sw, sh) in [(726.0, 912.0), (1400.0, 1000.0)] {
            (st.width, st.height) = (sw, sh);
            st.relayout();
            st.apply_layout();
            let rects = |st: &State| -> Vec<(usize, (f32, f32, f32, f32))> {
                (0..st.roster.len()).filter(|&i| on_page(st, i)).map(|i| (i, st.roster.get_dyn(&st.ui_context, i).rect())).collect()
            };
            let before = rects(&st);
            st.set_page(Page::Relief);
            st.set_page(Page::Widgets);
            assert_eq!(rects(&st), before, "{sw}x{sh}");
            for &(i, _) in &before {
                assert!(st.roster.get_dyn(&st.ui_context, i).visible(), "{sw}x{sh}: slot {i} is hidden");
            }
            for i in st.roster.gallery().tiles_start()..st.roster.len() {
                let w = st.roster.get_dyn(&st.ui_context, i);
                assert!(!w.visible() && w.rect().0 + w.rect().2 <= 0.0, "{sw}x{sh}: tile {i} is out");
            }
            let stops = st.ui_context.focus_clusters().concat();
            assert!(stops.contains(&st.roster.id(1)), "{sw}x{sh}: the Button is no Tab stop");
        }
    }

    /// A page opens at its top: switching scrolls the area back to 0, and the scroll frame
    /// keeps it there on its next tick.
    #[test]
    fn a_page_switch_scrolls_to_the_top() {
        let mut st = gallery();
        (st.width, st.height) = (726.0, 912.0);
        st.relayout();
        for (from, to) in [(Page::Widgets, Page::Relief), (Page::Relief, Page::Widgets)] {
            st.set_page(from);
            st.exhibit_scroll.scroll_y = f32::MAX;
            st.apply_layout();
            assert!(st.exhibit_scroll.scroll_y > 0.0, "{from:?} does not scroll at 726x912");
            st.set_page(to);
            assert_eq!(st.exhibit_scroll.scroll_y, 0.0, "{from:?} to {to:?}");
            st.exhibit_scroll.tick(0.016, &mut st.ui_context);
            assert_eq!(st.exhibit_scroll.scroll_y, 0.0, "{from:?} to {to:?}, a tick later");
        }
    }

    /// The Relief page as the widget page is held to it: in every layout, at window sizes from
    /// narrow to wide, no tile overlaps another — caption included — every tile lies across
    /// the well's floor at least as large as its table size, and scrolled all the way down
    /// the lowest ends two paddings above the floor's foot.
    #[test]
    fn the_relief_tiles_lie_apart_on_the_floor_in_every_layout() {
        let mut st = gallery();
        st.set_page(Page::Relief);
        let pad = st.ui_context[st.roster.gallery().group_fitted].inner().padding();
        let mut failures = Vec::new();
        for (sw, sh) in [(600.0, 800.0), (726.0, 912.0), (800.0, 600.0), (1000.0, 680.0), (1400.0, 1000.0), (2400.0, 1400.0)] {
            (st.width, st.height) = (sw, sh);
            st.relayout();
            for (li, (name, _)) in LAYOUTS.iter().enumerate() {
                st.layout_idx = li;
                st.exhibit_scroll.scroll_y = 0.0;
                st.apply_layout();
                let at = format!("{sw}x{sh} {name}");
                let (floor, _) = st.exhibit_floor();
                let sizes = st.exhibit_sizes();
                let tiles: Vec<(usize, cce_ui::scene::layout::Rect)> =
                    (0..st.roster.len()).filter(|&i| on_page(&st, i)).map(|i| (i, footprint(st.roster.get_dyn(&st.ui_context, i)))).collect();
                assert_eq!(tiles.len(), Effect::ALL.len(), "{at}");
                for (k, &(i, a)) in tiles.iter().enumerate() {
                    for &(j, b) in &tiles[k + 1..] {
                        if overlap(a, b) {
                            failures.push(format!("{at}: tile {i} {a:?} overlaps tile {j} {b:?}"));
                        }
                    }
                    if a.x < floor.x - 0.5 || a.x + a.width > floor.x + floor.width + 0.5 {
                        failures.push(format!("{at}: tile {i} {a:?} runs off the floor {floor:?}"));
                    }
                    let w = st.roster.get_dyn(&st.ui_context, i);
                    let e = sizes.iter().find(|e| e.slot == i).unwrap();
                    let (_, _, width, h) = w.rect();
                    if width < e.w - 0.5 || h - w.label_strip() < e.h - 0.5 {
                        failures.push(format!("{at}: tile {i} is {width}x{}, under its {}x{}", h - w.label_strip(), e.w, e.h));
                    }
                }
                st.exhibit_scroll.scroll_y = f32::MAX;
                st.apply_layout();
                let foot = floor.y + floor.height - 2.0 * pad;
                let lowest = (0..st.roster.len())
                    .filter(|&i| on_page(&st, i))
                    .map(|i| {
                        let (_, y, _, h) = st.roster.get_dyn(&st.ui_context, i).rect();
                        y + h
                    })
                    .fold(f32::MIN, f32::max);
                if lowest > foot + 0.5 {
                    failures.push(format!("{at}: scrolled to the bottom, a tile ends at {lowest} below {foot}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The header's dropdowns flow in rows: each on the row before it where it ends inside
    /// the window's right inset, else at the start of the next, one root gap below. At every
    /// width — down to ones narrower than a single dropdown and its insets — none overlaps
    /// another, label included, each lies inside the insets (or, where one cannot, starts its
    /// own row at the left inset), and the exhibit area starts one root gap below the lowest.
    #[test]
    fn the_header_wraps_where_its_dropdowns_do_not_fit_one_row() {
        let mut st = gallery();
        let inset = cce_ui::layout::root_plate_inset();
        let gap = cce_ui::layout::root_plate_gap();
        for sw in [100.0, 200.0, 300.0, 400.0, 430.0, 435.0, 600.0, 726.0, 1000.0, 1400.0] {
            (st.width, st.height) = (sw, 800.0);
            st.relayout();
            st.apply_layout();
            let header: Vec<cce_ui::scene::layout::Rect> = HEADER.iter().map(|&i| footprint(st.roster.get_dyn(&st.ui_context, i))).collect();
            // How many share a row: as many as fit between the insets, one at least.
            let per_row = (((sw - 2.0 * inset + gap) / (HEADER_W + gap)).floor() as usize).max(1);
            let fits = 2.0 * inset + HEADER_W <= sw;
            for (k, a) in header.iter().enumerate() {
                for b in &header[k + 1..] {
                    assert!(!overlap(*a, *b), "{sw}: {a:?} overlaps {b:?}");
                }
                if fits {
                    assert!(a.x >= inset - 0.5 && a.x + a.width <= sw - inset + 0.5, "{sw}: {a:?} past the inset");
                } else {
                    assert!((a.x - inset).abs() < 0.5, "{sw}: {a:?} does not start its row at the inset");
                }
                if k == 0 {
                    continue;
                }
                let prev = header[k - 1];
                if k % per_row == 0 {
                    assert!((a.x - inset).abs() < 0.5, "{sw}: dropdown {k} {a:?} does not start a row at the inset");
                    assert!(a.y >= prev.y + prev.height + gap - 0.5, "{sw}: dropdown {k} {a:?} is not below {prev:?}");
                } else {
                    assert!((a.y - prev.y).abs() < 0.5, "{sw}: dropdown {k} {a:?} is not on the row of {prev:?}");
                }
            }
            let foot = header.iter().map(|r| r.y + r.height).fold(f32::MIN, f32::max);
            let (_, vy, _, _) = st.exhibit_viewport();
            assert!((vy - (foot + gap)).abs() < 0.5, "{sw}: the area starts at {vy}, the header ends at {foot}");
        }
        assert!(2.0 * inset + 3.0 * HEADER_W + 2.0 * gap > 600.0, "600 px no longer wraps the header");
        assert!(2.0 * inset + 2.0 * HEADER_W + gap > 400.0, "400 px no longer wraps the header twice");
    }

    /// A page switched from the keyboard while the pointer holds an exhibit — the Slider,
    /// pressed and moved, or pressed and held still — ends the gesture where it is, before
    /// the exhibit is hidden: the next pointer move drags nothing on the hidden page, the
    /// router holds no target or grab, and back on the widget page the Slider is let go and
    /// arranged where it was.
    #[test]
    fn a_page_switch_ends_a_held_press_or_drag() {
        use cce_ui::engine::{Application, LogicalPosition};
        for moved in [true, false] {
            let mut st = gallery();
            (st.width, st.height) = (1000.0, 680.0);
            st.relayout();
            st.apply_layout();
            let before: Vec<_> = (0..st.roster.len()).filter(|&i| on_page(&st, i)).map(|i| st.roster.get_dyn(&st.ui_context, i).rect()).collect();
            let slider = st.roster.gallery().slider_demo;
            let (x, y, w, h) = st.roster.get_dyn(&st.ui_context, 5).rect();
            let strip = st.roster.get_dyn(&st.ui_context, 5).label_strip();
            let (px, py) = (x + w * 0.25, y + strip + (h - strip) * 0.5);
            assert!(st.in_exhibit_viewport(px, py), "the Slider is not in view at 1000x680");
            let mut nr = false;
            st.handle_mouse_input(MouseButton::Left, ElementState::Pressed, LogicalPosition::new(px, py), &mut nr);
            if moved {
                st.handle_pointer_move(LogicalPosition::new(px + 6.0, py), &mut nr);
            }
            let value = st.ui_context[slider].inner().value();
            assert!(st.roster.is_dragging(&st.ui_context, 5), "moved {moved}: the press did not take the Slider");
            st.set_page(Page::Relief);
            assert!(!st.roster.is_dragging(&st.ui_context, 5), "moved {moved}: the Slider is still dragging on the hidden page");
            let ui = &st.ui_context;
            assert!(ui.drag_target.is_none() && ui.active_grab.is_none() && !ui.is_dragging, "moved {moved}: the router still holds the gesture");
            st.handle_pointer_move(LogicalPosition::new(px + w * 2.0, py), &mut nr);
            assert_eq!(st.ui_context[slider].inner().value(), value, "moved {moved}: a move on the Relief page set the hidden Slider");
            st.handle_mouse_input(MouseButton::Left, ElementState::Released, LogicalPosition::new(px + w * 2.0, py), &mut nr);
            st.set_page(Page::Widgets);
            st.layout_exhibits();
            assert!(!st.roster.is_dragging(&st.ui_context, 5), "moved {moved}: the Slider is dragging back on its page");
            let after: Vec<_> = (0..st.roster.len()).filter(|&i| on_page(&st, i)).map(|i| st.roster.get_dyn(&st.ui_context, i).rect()).collect();
            assert_eq!(after, before, "moved {moved}: the widget page is not arranged as it was");
        }
    }

    /// A button held across a page switch is let go, not clicked: the switch's release lands
    /// off every widget, so a held "Ramp..." opens no editor.
    #[test]
    fn a_page_switch_does_not_click_a_held_button() {
        use cce_ui::engine::{Application, LogicalPosition};
        for slot in [11, 13] {
            let mut st = gallery();
            (st.width, st.height) = (1400.0, 2400.0);
            st.relayout();
            st.apply_layout();
            let button = st.roster.id(slot);
            let (x, y, w, h) = st.roster.get_dyn(&st.ui_context, slot).rect();
            let (px, py) = (x + w * 0.5, y + h * 0.5);
            assert!(st.in_exhibit_viewport(px, py), "slot {slot} is not in view");
            let mut nr = false;
            // The pointer comes to the button first, as a real click's does: the router's
            // cursor is where a release at the cursor would have landed.
            st.handle_pointer_move(LogicalPosition::new(px, py), &mut nr);
            st.handle_mouse_input(MouseButton::Left, ElementState::Pressed, LogicalPosition::new(px, py), &mut nr);
            st.ui_context.take_events();
            st.set_page(Page::Relief);
            let clicked = st.ui_context.take_events().into_iter().any(|(id, ev)| id == button && matches!(ev, WidgetEvent::Clicked));
            assert!(!clicked, "slot {slot}: the page switch clicked the held button");
        }
    }

    /// However narrow the window, the exhibit area stays inside it: its width followed the
    /// window down only to 300, past the right edge of a window under ~343 px.
    #[test]
    fn the_exhibit_area_stays_inside_a_narrow_window() {
        let mut st = gallery();
        for sw in [100.0, 200.0, 300.0, 343.0, 400.0, 600.0] {
            (st.width, st.height) = (sw, 800.0);
            st.relayout();
            st.apply_layout();
            let (x, _, w, _) = st.exhibit_viewport();
            assert!(x + w <= sw - x + 0.5, "{sw}: the exhibit area ends at {}, past {}", x + w, sw - x);
        }
    }

    /// A press that only moves the keyboard asks for a frame: with the ColorRamp focused (its
    /// track lit), a press on bare floor clears the focus and must redraw, or the rim stays
    /// lit until something else repaints.
    #[test]
    fn a_press_that_only_moves_the_focus_redraws() {
        use cce_ui::engine::{Application, LogicalPosition};
        let mut st = gallery();
        (st.width, st.height) = (1400.0, 1000.0);
        st.relayout();
        st.apply_layout();
        let ramp = st.roster.id(COLOR_RAMP_SLOT);
        st.ui_context.set_focused_id(ramp);
        assert_eq!(st.ui_context.focus_holder(), Some(ramp));
        // A point on the exhibit floor that no slot covers.
        let (vx, vy, vw, vh) = st.exhibit_viewport();
        let bare = (0..60)
            .flat_map(|i| (0..60).map(move |j| (vx + vw * (i as f32 + 0.5) / 60.0, vy + vh * (j as f32 + 0.5) / 60.0)))
            .find(|&(x, y)| (0..st.roster.len()).all(|k| !st.roster.get_dyn(&st.ui_context, k).hit_test(x, y, &st.ui_context)))
            .expect("some bare floor at 1400x1000");
        let mut needs_rebuild = false;
        st.handle_mouse_input(MouseButton::Left, ElementState::Pressed, LogicalPosition::new(bare.0, bare.1), &mut needs_rebuild);
        assert_eq!(st.ui_context.focus_holder(), None, "the press on bare floor took the focus");
        assert!(needs_rebuild, "the focus moved but no frame was asked for");
    }

    /// A page switch closes the shared context menu an exhibit's right-click opened: its
    /// rows act on that exhibit, which the switch hides.
    #[test]
    fn a_page_switch_closes_the_context_menu() {
        use cce_ui::engine::{Application, LogicalPosition};
        let mut st = gallery();
        (st.width, st.height) = (1000.0, 680.0);
        st.relayout();
        st.apply_layout();
        let (x, y, w, h) = st.roster.get_dyn(&st.ui_context, 5).rect();
        let at = LogicalPosition::new(x + w * 0.5, y + h * 0.5);
        let mut nr = false;
        st.handle_mouse_input(MouseButton::Right, ElementState::Pressed, at, &mut nr);
        assert!(cce_ui::widget::context_menu::is_visible(), "a right-press on the Slider opened no menu");
        st.set_page(Page::Relief);
        assert!(!cce_ui::widget::context_menu::is_visible(), "the menu is open over the Relief page");
    }

    /// At its default size every kind of child window has its description (or editor)
    /// and its Close button on the surface, with room, and apart.
    #[test]
    fn every_child_kind_lays_out_on_its_surface() {
        for kind in KINDS {
            let (sw, sh) = kind.default_size();
            let p = child_positions(sw, sh, false, false);
            let on_surface = |(x, y, w, h): (f32, f32, f32, f32)| {
                w > 0.0 && h > 0.0 && x >= 0.0 && y >= 0.0 && x + w <= sw && y + h <= sh
            };
            let (main, close) = (p[1], p[2]);
            assert!(on_surface(main), "{kind:?} main {main:?} on {sw}x{sh}");
            assert!(on_surface(close), "{kind:?} Close {close:?} on {sw}x{sh}");
            let apart = main.0 + main.2 <= close.0 || close.0 + close.2 <= main.0
                || main.1 + main.3 <= close.1 || close.1 + close.3 <= main.1;
            assert!(apart, "{kind:?}: main {main:?} overlaps Close {close:?}");
            if child_stacks(sh, false, false) {
                // The title line is painted above the main slot.
                let title_foot = cce_ui::layout::root_plate_inset() + CHILD_TITLE_SIZE;
                assert!(main.1 >= title_foot, "{kind:?}: main {main:?} under the title");
            }
        }
    }

    /// At its default size the Ramp editor gives the Ramp its whole height (its graph and
    /// control strip with the key pad), and the ColorRamp editor its declared height — and
    /// no more than a pixel past it, so the editor is not left standing half empty.
    #[test]
    fn the_ramp_editors_hold_their_ramps_whole() {
        let (sw, sh) = ChildKind::Ramp.default_size();
        let main = child_positions(sw, sh, false, false)[1];
        assert!(main.3 >= Ramp::expanded_height(), "the Ramp gets {} of {}", main.3, Ramp::expanded_height());
        let (sw, sh) = ChildKind::ColorRamp.default_size();
        let main = child_positions(sw, sh, false, false)[1];
        let natural = ColorRamp::new().natural_height();
        assert!(main.3 >= natural && main.3 < natural + 1.0, "the ColorRamp gets {} of {natural}", main.3);
    }

    #[test]
    fn the_short_kinds_lay_out_in_one_row() {
        for kind in [ChildKind::LayerTop, ChildKind::Status] {
            let (_, sh) = kind.default_size();
            assert!(!child_stacks(sh, false, false), "{kind:?}");
        }
        for kind in [ChildKind::Floating, ChildKind::Utility, ChildKind::Ramp, ChildKind::ColorRamp] {
            let (_, sh) = kind.default_size();
            assert!(child_stacks(sh, false, false), "{kind:?}");
        }
    }

    #[test]
    fn a_bevel_ramp_file_round_trips() {
        let keys = vec![RampKey { pos: 0.0, value: 0.25 }, RampKey { pos: 0.5, value: 1.0 }];
        let (parsed, line_type) = parse_bevel_ramp(&bevel_ramp_kdl(&keys, "bezier")).unwrap();
        assert_eq!(parsed.iter().map(|k| (k.pos, k.value)).collect::<Vec<_>>(), vec![(0.0, 0.25), (0.5, 1.0)]);
        assert_eq!(line_type, "bezier");
    }

    /// A key it cannot place is dropped rather than handed to the sort, which a NaN
    /// position used to panic; the rest come back in position order.
    #[test]
    fn a_malformed_bevel_ramp_file_does_not_panic() {
        let file = "keys {\n    key pos=nan val=1\n    key pos=0.9 val=0.5\n    key pos=0.1 val=inf\n    key pos=0.2 val=0.3\n}\n";
        let (keys, line_type) = parse_bevel_ramp(file).unwrap();
        assert_eq!(keys.iter().map(|k| k.pos).collect::<Vec<_>>(), vec![0.2, 0.9]);
        assert_eq!(line_type, "linear");
        assert!(parse_bevel_ramp("keys {\n    key pos=nan val=0\n}\n").is_none());
    }

    /// Tab from the header walks the stops through the runner (`Driver::key`, as the shell
    /// calls it) to the Ramp exhibit, which it enters on the graph — the ramp itself focused
    /// — then walks into the ramp's fields, which it holds (`focus_holder`): its dropdowns,
    /// and, the exhibit showing a key in hand, the key pad and Delete; then out to the next
    /// stop. Shift+Tab from there comes back into the ramp, on its graph.
    #[test]
    fn tab_from_the_header_reaches_the_ramp_and_walks_into_it() {
        use cce_ui::backend::driver::{Driver, Modifiers, Turn};
        let mut st = gallery();
        (st.width, st.height) = (1400.0, 1000.0);
        st.relayout();
        st.apply_layout();
        let ramp = st.roster.gallery().bevel_ramp;
        let (preset, line) = (st.ui_context[ramp].preset_dropdown.base().id(), st.ui_context[ramp].line_type_dropdown.base().id());
        let (pad, delete) = (st.ui_context[ramp].key_pad.base().id(), st.ui_context[ramp].del_button.base().id());
        assert!(st.ui_context[ramp].selected_key_idx.is_some(), "the exhibit shows a key in hand");
        let stops = st.ui_context.focus_clusters().concat();
        assert!(stops.contains(&ramp.id()), "the Ramp is no Tab stop");
        let first = st.roster.id(HEADER[0]);
        st.ui_context.set_focused_id(first);
        let (mut d, mut redraw, mut exit) = (Driver::new(), false, false);
        let mut tab = |st: &mut State, d: &mut Driver| {
            d.key(Turn { app: st, redraw: &mut redraw, exit: &mut exit }, Key::Named(NamedKey::Tab), Some("\t".into()), ElementState::Pressed);
            d.key(Turn { app: st, redraw: &mut redraw, exit: &mut exit }, Key::Named(NamedKey::Tab), None, ElementState::Released);
        };
        let mut presses = 0;
        while st.ui_context.focus_holder() != Some(ramp.id()) {
            assert!(presses <= stops.len(), "Tab from the header never reached the Ramp");
            tab(&mut st, &mut d);
            presses += 1;
        }
        assert_eq!(st.ui_context.focused_widget, Some(ramp.id()), "Tab enters the ramp on its graph");
        let mut walked = Vec::new();
        for _ in 0..5 {
            tab(&mut st, &mut d);
            walked.push((st.ui_context.focused_widget, st.ui_context.focus_holder()));
        }
        let (next, holder) = walked[4];
        let held = |id| (Some(id), Some(ramp.id()));
        assert_eq!(walked[..4], [held(preset), held(line), held(pad), held(delete)], "Tab walks the ramp's fields, which the ramp holds");
        assert!(next.is_some() && holder != Some(ramp.id()), "Tab past Delete leaves the ramp, for {next:?}");
        d.set_modifiers(&mut st, Modifiers { ctrl: false, shift: true, alt: false, logo: false });
        tab(&mut st, &mut d);
        assert_eq!(st.ui_context.focused_widget, Some(ramp.id()), "Shift+Tab comes back into the ramp, on its graph");
    }

    /// A click on the Ramp exhibit's Preset dropdown leaves the keyboard on the dropdown, a
    /// field the ramp holds: the press that follows it does not hand the ramp the focus again
    /// (which would put the keyboard back on its graph). A press on nothing takes the
    /// keyboard from the ramp, and so does a page switch that hides the ramp.
    #[test]
    fn a_click_on_a_ramps_dropdown_keeps_the_keyboard_there() {
        use cce_ui::engine::{Application, LogicalPosition};
        let mut st = gallery();
        (st.width, st.height) = (2400.0, 1400.0);
        st.relayout();
        st.apply_layout();
        let ramp = st.roster.gallery().bevel_ramp;
        let preset = st.ui_context[ramp].preset_dropdown.base().id();
        let click_preset = |st: &mut State| {
            let (x, y, w, h) = st.ui_context[ramp].preset_dropdown.rect();
            let at = LogicalPosition::new(x + w * 0.5, y + h * 0.5);
            assert!(st.in_exhibit_viewport(at.x, at.y), "the Preset dropdown at {at:?} is out of view");
            let mut nr = false;
            st.handle_mouse_input(MouseButton::Left, ElementState::Pressed, at, &mut nr);
            st.handle_mouse_input(MouseButton::Left, ElementState::Released, at, &mut nr);
            assert_eq!((st.ui_context.focused_widget, st.ui_context.focus_holder()), (Some(preset), Some(ramp.id())), "the click left the keyboard elsewhere");
        };
        click_preset(&mut st);
        // Close the menu the click opened, so the press below lands on the window.
        let mut nr = false;
        st.handle_key_input(&KeyEvent { state: ElementState::Pressed, logical_key: Key::Named(NamedKey::Escape), text: None, repeat: false, ctrl: false, shift: false, alt: false }, &mut nr);
        let (sw, sh) = (st.width, st.height);
        let nothing = (0..(sh / 8.0) as usize)
            .flat_map(|r| (0..(sw / 8.0) as usize).map(move |c| (c as f32 * 8.0 + 4.0, r as f32 * 8.0 + 4.0)))
            .find(|&(x, y)| (0..st.roster.len()).all(|i| !st.is_widget_visible(i) || !st.roster.get_dyn(&st.ui_context, i).hit_test(x, y, &st.ui_context)))
            .expect("no spot of the window is clear of every slot");
        let at = LogicalPosition::new(nothing.0, nothing.1);
        st.handle_mouse_input(MouseButton::Left, ElementState::Pressed, at, &mut nr);
        st.handle_mouse_input(MouseButton::Left, ElementState::Released, at, &mut nr);
        assert_eq!(st.ui_context.focus_holder(), None, "a press on nothing at {at:?} left the ramp the keyboard");

        click_preset(&mut st);
        st.set_page(Page::Relief);
        assert_eq!(st.ui_context.focus_holder(), None, "the hidden ramp kept the keyboard");
    }
}
