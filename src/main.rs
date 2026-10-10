use cce_ui::context::UiContext;
use cce_ui::widget::{Handle, WidgetId};
use cce_ui::widget::{Button, Checkbox, ContentBg, Dropdown, Label, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar, Toggle, WidgetHost, Trackpad, hover_animation, TextBox, MenuBar, Group, Ramp, RampKey, ColorRamp, MouseButton, ElementState, Key, NamedKey, KeyEvent, MouseScrollDelta, ColorSelector, FontSelector, KeybindRecorder, ButtonStrip, Float3, UsageBar, StatusDot, DotStatus, InfoBox, InteractiveListItem, Breadcrumb, TreeList, BevelPreview, RampPreview, Separator, Splitter, Paginator, VerticalLayout, ColumnsLayout, GridLayout, AdaptiveGridLayout, MosaicLayout, ReverseMosaicLayout, OverlayLayout, ScrollBox, WidgetHostExt};
mod gallery_widgets;
use gallery_widgets::{RootPlate, Plate};
use cce_ui::widget::{Adapted, LayoutConstraints, Point};
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
            // 390 tall: the editor keeps the 260px it had at hand-set margins now that
            // the title line, one root gap and the root insets stand around it.
            ChildKind::Ramp | ChildKind::ColorRamp => (450.0, 390.0),
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
/// loops can hold `&mut self.roster` while asking. The gallery shows every slot; a
/// child window shows its menu bar and status bar only when asked to.
#[derive(Clone, Copy)]
struct Visibility {
    is_child: bool,
    use_menubar: bool,
    use_statusbar: bool,
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
        true
    }
}

/// The gallery roster: 32 named slots. The numeric indexes used by the positions
/// table, the visibility filter and the dispatch loops address these slots through
/// `Roster::id` / `get_dyn` / `get_dyn_mut`.
pub struct GallerySlots {
    pub menu_bar: Handle<Adapted<MenuBar>>,
    pub status_bar: Handle<Adapted<StatusBar>>,
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
    /// Two `Group` lassos over other exhibits (slots 32 and 33): overlays, not
    /// exhibits — laid out by their members, drawn under the exhibit clip.
    pub group_loose: Handle<Adapted<Group>>,
    pub group_fitted: Handle<Adapted<Group>>,
    /// The Style dropdown (slot 34), beside Layout in the header: Relief or Flat
    /// for every control at once (`cce_ui::layout::set_control_relief`).
    pub style_dd: Handle<Adapted<Dropdown>>,
    /// The variant exhibits (`variant_exhibits`): every further style of a widget one of
    /// the named slots already shows, addressed as slots `GALLERY_COUNT..`.
    pub extra: Vec<Exhibit>,
}

/// The number of NAMED gallery slots; the variant exhibits follow them, so the roster's
/// `len` is this plus `GallerySlots::extra.len()`.
pub const GALLERY_COUNT: usize = 35;

/// A variant exhibit: a widget in a style other than the one its named slot shows, with
/// the content size `layout_exhibits` resets it to.
pub struct Exhibit {
    /// The widget, which the context owns.
    pub id: WidgetId,
    pub w: f32,
    pub h: f32,
    /// A content width narrower than `w`: the strategy lays the exhibit out at `w` (room
    /// for its label) and the widget is then given this width — a StatusDot stays a dot.
    pub content_w: Option<f32>,
}

impl Exhibit {
    /// Sized by the widget's own preferred (content) height, `fallback_h` when it declares none.
    fn new<W: WidgetHost + 'static>(ctx: &mut UiContext, widget: W, w: f32, fallback_h: f32) -> Self {
        let h = widget.preferred_height().unwrap_or(fallback_h);
        Exhibit { id: ctx.insert(widget).id(), w, h, content_w: None }
    }

    /// Sized by hand: for widgets whose declared height is a single row (a multiline
    /// TextBox, a vertical ButtonStrip) when the exhibit wants several.
    fn sized<W: WidgetHost + 'static>(ctx: &mut UiContext, widget: W, w: f32, h: f32) -> Self {
        Exhibit { id: ctx.insert(widget).id(), w, h, content_w: None }
    }

    fn with_content_width(mut self, w: f32) -> Self {
        self.content_w = Some(w);
        self
    }
}

impl GallerySlots {
    /// The widget in slot `idx`.
    pub fn id(&self, idx: usize) -> WidgetId {
        if idx >= GALLERY_COUNT {
            return self.extra[idx - GALLERY_COUNT].id;
        }
        match idx {
            0 => self.menu_bar.id(),
            1 => self.status_bar.id(),
            2 => self.button_demo.id(),
            3 => self.checkbox_demo.id(),
            4 => self.toggle_demo.id(),
            5 => self.progress_demo.id(),
            6 => self.slider_demo.id(),
            7 => self.spinbox_demo.id(),
            8 => self.range_slider_demo.id(),
            9 => self.trackpad_demo.id(),
            10 => self.textbox_demo.id(),
            11 => self.plate_demo.id(),
            12 => self.color_ramp_btn.id(),
            13 => self.bevel_ramp.id(),
            14 => self.ramp_btn.id(),
            15 => self.layout_dd.id(),
            16 => self.color_selector_demo.id(),
            17 => self.font_selector_demo.id(),
            18 => self.keybind_demo.id(),
            19 => self.button_strip_demo.id(),
            20 => self.slider2d_demo.id(),
            21 => self.float3_demo.id(),
            22 => self.usage_bar_demo.id(),
            23 => self.status_dot_demo.id(),
            24 => self.info_box_demo.id(),
            25 => self.list_item_demo.id(),
            26 => self.breadcrumb_demo.id(),
            27 => self.tree_list_demo.id(),
            28 => self.bevel_preview_demo.id(),
            29 => self.ramp_preview_demo.id(),
            30 => self.separator_demo.id(),
            31 => self.splitter_demo.id(),
            32 => self.group_loose.id(),
            33 => self.group_fitted.id(),
            34 => self.style_dd.id(),
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }
}

/// A child window's five slots — background, main, Close, and the optional menu bar
/// and status bar rows (3, 4) — as the ids of the widgets the context owns.
pub struct ChildSlots {
    pub ids: [WidgetId; CHILD_COUNT],
    pub close: Handle<Adapted<Button>>,
}

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
            Roster::Gallery(s) => GALLERY_COUNT + s.extra.len(),
            Roster::Child(_) => CHILD_COUNT,
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

    /// A slot that is never dragged or drained: the variant exhibits are looked at, and
    /// the lassos (32, 33) are frames.
    fn inert(&self, idx: usize) -> bool {
        matches!(self, Roster::Gallery(_)) && (idx >= GALLERY_COUNT || matches!(idx, 32 | 33))
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

    // --- Value drains: route a slot index to its widget's `take_click` / `value`.

    pub fn take_click(&self, ui: &mut UiContext, idx: usize) -> bool {
        match self {
            // The child roster drains one slot: its Close button.
            Roster::Child(c) => idx == 2 && ui[c.close].take_click(),
            Roster::Gallery(_) => !self.inert(idx) && self.get_dyn_mut(ui, idx).input_model_mut().take_click(),
        }
    }

    pub fn value(&self, ui: &UiContext, idx: usize) -> i32 {
        let s = self.gallery();
        match idx {
            15 => ui[s.layout_dd].value(),
            34 => ui[s.style_dd].value(),
            _ => panic!("value: unwired gallery slot {idx}"),
        }
    }
}

struct State {
    roster: Roster,
    positions: Vec<(f32, f32, f32, f32)>,

    status_text: String,

    focused_widget: Option<usize>,

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
    /// The scroll frame the exhibits below the Layout dropdown scroll inside.
    exhibit_scroll: ScrollBox,
}

fn save_bevel_ramp(keys: &[RampKey], line_type: &str) {
    let mut s = String::new();
    s.push_str("keys {\n");
    for k in keys {
        s.push_str(&format!("    key pos={} val={}\n", k.pos, k.value));
    }
    s.push_str("}\n");
    s.push_str(&format!("line_type \"{}\"\n", line_type));
    let path = cce_ui::config::get_config_path().parent().unwrap().join("bevel_ramp.kdl");
    let _ = std::fs::write(path, s);
}

fn load_bevel_ramp() -> (Vec<RampKey>, String) {
    let path = cce_ui::config::get_config_path().parent().unwrap().join("bevel_ramp.kdl");
    let mut line_type = "linear".to_string();
    let mut keys = Vec::new();
    if let Ok(content) = std::fs::read_to_string(path) {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("key ") {
                let mut pos = 0.0;
                let mut val = 0.5;
                for part in line.split_whitespace() {
                    if let Some(rest) = part.strip_prefix("pos=") {
                        pos = rest.parse().unwrap_or(0.0);
                    } else if let Some(rest) = part.strip_prefix("val=") {
                        val = rest.parse().unwrap_or(0.5);
                    }
                }
                keys.push(RampKey { pos, value: val });
            } else if line.starts_with("line_type ") {
                if let Some(val_str) = line.split_whitespace().nth(1) {
                    line_type = val_str.trim_matches('"').to_string();
                }
            }
        }
        if !keys.is_empty() {
            keys.sort_by(|a, b| a.pos.partial_cmp(&b.pos).unwrap());
            return (keys, line_type);
        }
    }
    (
        vec![
            RampKey { pos: 0.0, value: 0.5 },
            RampKey { pos: 0.2, value: 1.0 },
            RampKey { pos: 0.8, value: 1.0 },
            RampKey { pos: 1.0, value: 0.5 },
        ],
        "linear".to_string()
    )
}

/// Every further style of a widget the named slots show once: the toolkit's own
/// variants — a constructor (`Button::new_reset`) or a builder (`with_band`) —
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
    // A StatusDot is 12px square; its exhibit is as wide as its label.
    const DOT_W: f32 = 200.0;
    // A column of three rotated tabs, and the sidebar width a vertical strip is drawn for.
    const TABS_H: f32 = 120.0;
    const TAB_COLUMN_W: f32 = 40.0;
    let three = || vec!["One".to_string(), "Two".to_string(), "Three".to_string()];
    vec![
        // Button: the other four kinds.
        Exhibit::new(ctx, Button::new_reset(0.0, 0.0, W, bh).with_label("Button (reset)"), W, bh),
        Exhibit::new(ctx, Button::new_list_row(0.0, 0.0, W, bh).with_label("Button (list row)"), W, bh),
        Exhibit::new(ctx, Button::new_menu_item(0.0, 0.0, W, bh).with_label("Button (menu item)"), W, bh),
        Exhibit::new(ctx, Button::new_copy_icon(0.0, 0.0, bh, bh), bh, bh),
        // Slider: with a readout.
        Exhibit::new(ctx, Slider::new().with_label("Slider (readout)").with_readout(true), W, slh),
        // TextBox: multiline, chromeless, password.
        Exhibit::sized(
            ctx,
            TextBox::new("TextBox (multiline)\nA second line of text.".to_string()).with_multiline(true),
            W,
            3.0 * tbh,
        ),
        Exhibit::new(ctx, TextBox::new("TextBox (chromeless)".to_string()).with_draw_bg_border(false), W, tbh),
        Exhibit::new(ctx, TextBox::new("hunter2".to_string()).with_password(true).with_label("TextBox (password)"), W, tbh),
        // ButtonStrip: the vertical column (rotated tabs), and the Paginator sidebar built on it.
        Exhibit::sized(
            ctx,
            Adapted::new(ButtonStrip::new(0.0, 0.0, W, TABS_H).with_buttons(three()).with_selected(Some(0)).with_vertical(true))
                .with_label("ButtonStrip (vertical)"),
            W,
            TABS_H,
        )
        .with_content_width(TAB_COLUMN_W),
        Exhibit::sized(ctx, Paginator::new(three()).with_label("Paginator"), W, TABS_H),
        // ColorSelector: the alpha swatch.
        Exhibit::new(ctx, ColorSelector::new_rgba([64, 128, 255, 128]).with_label("ColorSelector (alpha)"), W, csh),
        // Label: the plain text widget.
        Exhibit::new(ctx, Label::new("Label"), W, ddh),
        // StatusDot: the other three statuses.
        Exhibit::new(ctx, StatusDot::new(DotStatus::Inactive).with_label("StatusDot (inactive)"), DOT_W, StatusDot::SIZE)
            .with_content_width(StatusDot::SIZE),
        Exhibit::new(ctx, StatusDot::new(DotStatus::Warning).with_label("StatusDot (warning)"), DOT_W, StatusDot::SIZE)
            .with_content_width(StatusDot::SIZE),
        Exhibit::new(ctx, StatusDot::new(DotStatus::Error).with_label("StatusDot (error)"), DOT_W, StatusDot::SIZE)
            .with_content_width(StatusDot::SIZE),
    ]
}

/// The exhibits: every slot but the chrome (0, 1), the lassos (32, 33) and the
/// header dropdowns (15 Layout, 34 Style), which `layout_exhibits` lays out under
/// the header row.
fn is_exhibit(i: usize) -> bool {
    matches!(i, 2..=14 | 16..=31) || i >= GALLERY_COUNT
}

/// The Group lassos: drawn under the exhibit clip like exhibits, laid out by
/// their members rather than by the strategy.
fn is_overlay(i: usize) -> bool {
    matches!(i, 32 | 33)
}

impl State {
    fn visibility(&self) -> Visibility {
        Visibility {
            is_child: self.is_child,
            use_menubar: self.use_menubar,
            use_statusbar: self.use_statusbar,
        }
    }

    fn is_widget_visible(&self, index: usize) -> bool {
        self.visibility().is_visible(index)
    }

    /// Recompute the positions table for the current size, mode and layout.
    fn relayout(&mut self) {
        self.positions = if self.is_child {
            child_positions(self.width, self.height, self.use_menubar, self.use_statusbar, self.child_kind.is_some_and(ChildKind::is_editor))
        } else {
            demo_positions(self.width, self.height, self.roster.len())
        };
    }

    /// The Controls exhibits in layout order, each with the content size it is reset to
    /// before a strategy runs: the toolkit layouts read a child's own rect for its width
    /// and preferred height, and a previous strategy may have stretched it.
    fn exhibit_sizes(&self) -> Vec<(usize, f32, f32)> {
        // Every height below is the toolkit's default for that control — its
        // configured `style.control.<name>.height`, or the intrinsic size the
        // widget declares — so the page is a record of the defaults, not of
        // numbers chosen here. The strategies read a widget's own intrinsic
        // height anyway; the table only has to agree with it.
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
        // or drag in, sized here and only here.
        const CANVAS_H: f32 = 120.0;
        const W: f32 = 190.0;
        let s2d_w = self
            .roster
            .get_dyn(&self.ui_context, 20)
            .measure(LayoutConstraints::new(0.0, f32::MAX, 0.0, f32::MAX), &self.ui_context)
            .width;
        // A StatusDot is 12px square; its exhibit is as wide as its label
        // (`content_width` hands the dot its own size back after layout).
        const DOT_W: f32 = 200.0;
        let raw: [(usize, f32, f32); 29] = [
            (2, W, bh),                                   // Button
            (19, W, bh),                                  // ButtonStrip
            (3, W, tgh),                                  // Checkbox (a toggle row)
            (4, W, tgh),                                  // Toggle
            (6, W, slh),                                  // Slider
            (8, W, rsh),                                 // RangeSlider
            (20, s2d_w, 64.0),                            // Slider2D (a 64px pad, wide enough for its label)
            (7, W, sph),                                  // Spinbox
            (21, W, Float3::preferred_height(false)),     // Float3 (three slider rows)
            (10, W, tbh),                                 // TextBox
            (18, W, tbh),                                 // KeybindRecorder (a textbox)
            (16, W, csh),                                 // ColorSelector
            (17, W, fsh),                                 // FontSelector
            (5, W, pbh),                                  // ProgressBar
            (22, W, pbh),                                 // UsageBar
            (23, DOT_W, StatusDot::SIZE),                 // StatusDot (its label needs the width)
            (30, W, 1.0),                                 // Separator (a rule)
            (31, 6.0, CANVAS_H),                          // Splitter (a vertical grip)
            (24, W, 3.0 * ddh),                           // InfoBox (title + two lines)
            (25, W, 2.0 * ddh),                           // InteractiveListItem (title + subtitle)
            (26, W, bh),                                  // Breadcrumb (button plates)
            (27, W, CANVAS_H),                            // TreeList
            (9, W, CANVAS_H),                            // Trackpad
            (11, W, CANVAS_H),                            // Plate
            (28, W, CANVAS_H),                            // BevelPreview
            (29, W, 2.0 * rmh),                           // RampPreview (a ramp's curve)
            (13, W, CANVAS_H),                            // Ramp (its editor declares 150)
            (12, W, bh),                                  // Color Ramp...
            (14, W, bh),                                  // Ramp...
        ];
        let mut sizes = raw.to_vec();
        let extra = &self.roster.gallery().extra;
        sizes.extend(extra.iter().enumerate().map(|(k, e)| (GALLERY_COUNT + k, e.w, e.h)));
        sizes
    }

    /// Lay the Controls exhibits out with the toolkit strategy the Layout dropdown selects
    /// (`LAYOUTS`), then clip whatever runs past the status bar.
    /// The exhibits' viewport: below the Layout dropdown, above the status bar.
    /// The exhibit area: a well sunk into the root plate, standing on it like the
    /// header dropdowns — their inset in from the window's sides (`x` is the Layout
    /// dropdown's), one root gap below them and one root gap above the status band.
    fn exhibit_viewport(&self) -> (f32, f32, f32, f32) {
        let gap = cce_ui::layout::root_plate_gap();
        let (dx, dy, _, dh) = self.roster.get_dyn(&self.ui_context, 15).rect();
        let (x, y) = (dx, dy + dh + gap);
        let w = (self.width - 2.0 * x).max(300.0);
        let h = ((self.height - 24.0 - gap) - y).max(100.0);
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

    fn in_exhibit_viewport(&self, px: f32, py: f32) -> bool {
        let (x, y, w, h) = self.exhibit_viewport();
        px >= x && px <= x + w && py >= y && py <= y + h
    }

    /// Lay the Controls exhibits out with the toolkit strategy the Layout dropdown selects
    /// (`LAYOUTS`), then shift them by the scroll frame's offset; painting clips them to
    /// the viewport. Runs from apply_layout and from display_list, which is what
    /// re-arranges after a scroll.
    fn layout_exhibits(&mut self) {
        let (x, y, w, h) = self.exhibit_viewport();
        self.exhibit_scroll.set_rect(x, y, w, h);
        // The exhibits sit on the well's floor, inside its wall; the fitted lasso's
        // plate is that floor, so its sides snap to the foot of the wall.
        let (floor, floor_r) = self.exhibit_floor();
        if let Roster::Gallery(s) = &mut self.roster {
            self.ui_context[s.group_fitted].inner_mut().set_plate(floor, floor_r);
        }
        let mut children: Vec<*mut (dyn WidgetHost + 'static)> = Vec::new();
        let mut indices: Vec<usize> = Vec::new();
        for (idx, cw, ch) in self.exhibit_sizes() {
            if self.roster.is_dragging(&self.ui_context, idx) {
                continue;
            }
            let widget = self.roster.get_dyn_mut(&mut self.ui_context, idx);
            // Seed the block: the content height plus the label strip above it.
            let strip = widget.label_strip();
            widget.set_rect(0.0, 0.0, cw, ch + strip);
            children.push(widget as *mut (dyn WidgetHost + 'static));
            indices.push(idx);
        }
        // The exhibits start at the fitted lasso's seat: one padding in from the
        // area's sides for the frame plus one for the members inside it, and the
        // lasso's headroom (padding + title tab) plus a padding down. A fitted group
        // snaps to the area's edges one padding in, and its tab needs room inside
        // the area above its members — laid out flush with the corner, the row had
        // the frame's wall on its left edge and the tab over its labels.
        let (inset_x, inset_top) = match &self.roster {
            Roster::Gallery(s) => {
                let g = self.ui_context[s.group_fitted].inner();
                (2.0 * g.padding(), g.padding() + g.headroom())
            }
            _ => (0.0, 0.0),
        };
        let strategy = (LAYOUTS.get(self.layout_idx).unwrap_or(&LAYOUTS[DEFAULT_LAYOUT]).1)();
        let (lx, ly) = (floor.x + inset_x, floor.y + inset_top);
        let content_h = strategy.layout(lx, ly, floor.width - 2.0 * inset_x, floor.y + floor.height - ly, &children, &mut self.ui_context);
        self.exhibit_scroll.update_bounds(content_h + (ly - y), y, h);
        let scroll_y = self.exhibit_scroll.scroll_y;
        for (&child, &idx) in children.iter().zip(&indices) {
            let widget = unsafe { &mut *child };
            // The strategy placed each exhibit's content box (its label hanging in the
            // strip above); re-land it through `layout` shifted by the scroll — the
            // landed rect is the occupied one, so the content origin is `strip` below
            // its top and the content height is the rest.
            let (cx, cy, cw, ch) = widget.rect();
            let strip = widget.label_strip();
            let content = ch - strip;
            let cw = self.content_width(idx).unwrap_or(cw);
            widget.layout(
                Point { x: cx, y: cy + strip - scroll_y },
                LayoutConstraints::new(cw, cw, content, content),
                &mut self.ui_context,
            );
        }
    }

    /// An exhibit's content width where it is narrower than the width it is laid out at
    /// (`Exhibit::content_w`; the named StatusDot slot likewise).
    fn content_width(&self, idx: usize) -> Option<f32> {
        if idx == 23 {
            return Some(StatusDot::SIZE);
        }
        idx.checked_sub(GALLERY_COUNT).and_then(|k| self.roster.gallery().extra[k].content_w)
    }

    /// Register every roster widget in the ui_context (idempotent — `register` is
    /// id-keyed and the boxed slots keep pointers stable). The id-rooted router
    /// resolves roots through this registry; the gallery's own paint loop never goes
    /// through `render_widget`, where other apps pick registration up as a side effect.
    fn register_roster(&mut self) {
    }

    /// Drain the header dropdowns' selections and apply them: Layout (15) picks the
    /// strategy, Style (34) switches every control between Relief and Flat
    /// (`set_control_relief` — the toolkit reads it live at paint, so a rebuild is
    /// all it takes). Called after mouse AND key input: a Dropdown selects from the
    /// keyboard too (Down, Enter), and a selection must not wait for the next click.
    fn apply_header_dropdowns(&mut self) -> bool {
        let mut applied = false;
        if self.roster.take_click(&mut self.ui_context, 15) {
            self.layout_idx = self.roster.value(&self.ui_context, 15) as usize;
            applied = true;
        }
        if self.roster.take_click(&mut self.ui_context, 34) {
            cce_ui::layout::set_control_relief(self.roster.value(&self.ui_context, 34) == 0);
            applied = true;
        }
        if applied {
            self.relayout();
            self.apply_layout();
        }
        applied
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
            if visible && (i == 15 || i == 34) {
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

    fn update_status_text(&mut self, text: &str) {
        self.status_text = text.to_string();
    }
}

impl cce_ui::engine::Application for State {
    type Message = String;

    fn create(_sender: cce_ui::engine::AppSender<Self::Message>) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let flag = |name: &str| args.iter().any(|a| a == name);
        let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
        let number = |name: &str| value(name).and_then(|s| s.parse::<f32>().ok());

        let is_child = flag("--child");
        let use_root_plate = if is_child { flag("--root-plate") } else { !flag("--no-root-plate") };
        let use_menubar = flag("--menubar");
        let use_statusbar = flag("--statusbar");
        let child_kind = if is_child {
            Some(value("--type").and_then(|t| ChildKind::from_arg(&t)).unwrap_or(ChildKind::Floating))
        } else {
            None
        };
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

        let opacity = opacity || child_kind.is_some_and(ChildKind::is_editor);
        let status_text = "Ready.".to_string();

        // The context owns the widgets; the roster names them by slot.
        let mut ui_context = cce_ui::context::UiContext::new();
        let ctx = &mut ui_context;
        let roster = if let Some(kind) = child_kind {
            let bg = if use_root_plate {
                ctx.insert(RootPlate::new(0.0, 0.0, c_w, c_h)).id()
            } else {
                ctx.insert(ContentBg::new()).id()
            };
            let (main, aux3, aux4) = if kind == ChildKind::ColorRamp {
                (
                    ctx.insert(ColorRamp::new()).id(),
                    ctx.insert(Label::new("").with_font_size(12.0)).id(),
                    ctx.insert(Label::new("").with_font_size(12.0)).id(),
                )
            } else if kind == ChildKind::Ramp {
                (
                    ctx.insert({
                        let mut ramp = Ramp::new();
                        let (loaded_keys, loaded_type) = load_bevel_ramp();
                        ramp.keys = loaded_keys;
                        ramp.line_type_dropdown.selected = match loaded_type.as_str() {
                            "bezier" => 1,
                            _ => 0,
                        };
                        ramp
                    })
                    .id(),
                    ctx.insert(Label::new("").with_font_size(12.0)).id(),
                    ctx.insert(Label::new("").with_font_size(12.0)).id(),
                )
            } else {
                let menu_bar = MenuBar::new(0.0, 0.0, c_w, 40.0)
                    .with_item("File", &["New", "Open", "Save", "Exit"])
                    .with_item("Edit", &["Undo", "Redo", "Cut", "Copy", "Paste"])
                    .with_right_aligned_title(true);
                (
                    ctx.insert(Label::new(kind.child_text()).with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])).id(),
                    ctx.insert(menu_bar).id(),
                    ctx.insert(StatusBar::new()).id(),
                )
            };
            let close = ctx.insert(Button::new(0.0, 0.0, 100.0, cce_ui::layout::button_height()).with_label("Close"));
            Roster::Child(Box::new(ChildSlots { ids: [bg, main, close.id(), aux3, aux4], close }))
        } else {
            let menu_bar = MenuBar::new(0.0, 0.0, c_w, 40.0)
                .with_item("File", &["Exit"])
                .with_item("Edit", &["Settings"])
                .with_item("Help", &["About"])
                .with_right_aligned_title(true);
            Roster::Gallery(Box::new(GallerySlots {
                menu_bar: ctx.insert(menu_bar),
                status_bar: ctx.insert(StatusBar::new()),
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
                bevel_ramp: ctx.insert(Ramp::new()),
                ramp_btn: ctx.insert(Button::new(0.0, 0.0, 120.0, cce_ui::layout::button_height()).with_label("Ramp...")),
                layout_dd: ctx.insert(Dropdown::new(
                    LAYOUTS.iter().map(|(name, _)| name.to_string()).collect(),
                    DEFAULT_LAYOUT,
                ).with_label("Layout")),
                style_dd: ctx.insert(Dropdown::new(
                    vec!["Relief".to_string(), "Flat".to_string()],
                    if cce_ui::layout::control_relief() { 0 } else { 1 },
                ).with_label("Style")),
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
                list_item_demo: ctx.insert(InteractiveListItem::new("InteractiveListItem")),
                breadcrumb_demo: {
                    let mut b = Breadcrumb::new();
                    b.path = vec!["home".to_string(), "lsgalante".to_string(), "projects".to_string()];
                    ctx.insert(b.with_label("Breadcrumb"))
                },
                tree_list_demo: {
                    let mut t = TreeList::new();
                    t.set_flat_keys(vec![
                        ("layout/bar_height".to_string(), serde_json::json!(24)),
                        ("layout/gap".to_string(), serde_json::json!(12)),
                        ("theme/name".to_string(), serde_json::json!("cce")),
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
                extra: variant_exhibits(ctx),
            }))
        };

        let roster = {
            let mut roster = roster;
            if let Roster::Gallery(s) = &mut roster {
                // The lassos: a loose one around the FontSelector and the StatusDot, and
                // one around the top row (Button, ButtonStrip, Checkbox, Toggle) that
                // fits the exhibit area's edges — its top snaps to the area's top with
                // the title tab kept inside, its left to the area's left edge.
                let ids = |s: &GallerySlots, idx: &[usize]| idx.iter().map(|&i| s.id(i)).collect::<Vec<_>>();
                let loose = ids(s, &[17, 23]);
                let fitted = ids(s, &[2, 19, 3, 4]);
                ui_context[s.group_loose].inner_mut().set_members(loose);
                ui_context[s.group_fitted].inner_mut().set_members(fitted);
            }
            roster
        };
        let mut state = Self {
            roster,
            positions: Vec::new(),
            status_text,
            focused_widget: None,
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
            exhibit_scroll: {
                let mut sb = ScrollBox::new();
                sb.show_border = false;
                sb.show_background = false;
                // The DE's one scrollbar: down the area's centre line, idling
                // behind the well's floor until a scroll raises it.
                sb.sink_behind = true;
                sb
            },
        };

        let main = state.roster.get_dyn_mut(&mut state.ui_context, 1);
        if main.as_any_mut().downcast_mut::<Ramp>().is_some() && is_child {
            // Only a `--type Ramp` child hosts a Ramp in slot 1: it opens with the
            // preset dropdown focused so the keyboard drives it at once — the ramp
            // takes the window's focus and gives its first field, the preset
            // dropdown, the keyboard. Every other child kind (ColorRamp, or the
            // description Label of the Toplevel / Popup / Layer* windows) starts with
            // nothing focused — the key sweep in handle_key reaches every visible
            // child slot anyway, and a click focuses whatever it lands on. This used
            // to downcast unconditionally and panic for those kinds ("child ramp
            // widget"), which is why Create Window on the Windows page spawned
            // children that died at startup.
            state.focused_widget = Some(1);
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

    fn layer(&self) -> Option<LayerSettings> {
        self.child_kind.and_then(|kind| kind.layer(self.height as i32))
    }

    fn utility(&self) -> bool {
        self.child_kind == Some(ChildKind::Utility)
    }

    fn update(&mut self, msg: Self::Message, needs_rebuild: &mut bool, exit: &mut bool) {
        if msg == "exit" {
            *exit = true;
        } else {
            self.update_status_text(&msg);
            *needs_rebuild = true;
        }
    }

    fn tick(&mut self, dt: f32, needs_rebuild: &mut bool) {
        let mut changed = false;
        if hover_animation::tick(dt) {
            changed = true;
        }
        // True while the area's bar is held up or fading, as well as while a
        // glide moves it, so the frames keep coming until the sink renders.
        if !self.is_child && self.exhibit_scroll.tick(dt, &mut self.ui_context) {
            changed = true;
        }
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);
        for i in 0..self.roster.len() {
            if is_visible(i) {
                let ramp_child = self.child_kind == Some(ChildKind::Ramp) && i == 1;
                let ticked = self.ui_context.lend(self.roster.id(i), |w, ctx| {
                    let ticked = w.tick(dt, ctx);
                    if ticked && ramp_child {
                        if let Some(ramp) = w.as_any().downcast_ref::<Ramp>() {
                            let line_type_str = match ramp.line_type_dropdown.selected {
                                1 => "bezier",
                                _ => "linear",
                            };
                            save_bevel_ramp(&ramp.keys, line_type_str);
                        }
                    }
                    ticked
                });
                if ticked == Some(true) {
                    changed = true;
                }
            }
        }
        if changed {
            *needs_rebuild = true;
        }
    }

    fn display_list(&mut self, size: LogicalSize, scale: f64) -> Option<cce_ui::scene::paint::DisplayList> {
        // The whole frame — rounded geometry, plain geometry, popovers, then text — is
        // one display list, rebuilt each frame.
        use cce_ui::scene::layout::Rect;
        self.register_roster();
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
            let text = self.status_text.clone();
            self.update_status_text(&text);
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
            cce_ui::scene::painter::paint_root_into(&self.ui_context, w, &mut walk);
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

        // ── Popovers: in-frame, on top of everything. PaintCtx is a
        // RenderTarget — real prims (the dropdown's expanded inset-plate
        // surface) with per-label bounds; glyphs render in the later text pass
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
        let label_text = match self.child_kind {
            Some(kind) => kind.title().to_string(),
            None => "Gallery".to_string(),
        };
        let mut has_menu_bar = false;
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(&mut self.ui_context, i);
            if let Some(menu_bar) = w.as_any_mut().downcast_mut::<MenuBar>() {
                menu_bar.title = label_text.clone();
                has_menu_bar = true;
            }
        }
        if !has_menu_bar {
            // The title stands on the root plate: one inset in from the window's corner.
            let inset = cce_ui::layout::root_plate_inset();
            pc.text_with(label_text, inset, inset, CHILD_TITLE_SIZE, [255, 255, 255], Some(cce_ui::layout::statusbar_font()), None);
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
            cce_ui::scene::painter::append_widget_text(&self.ui_context, w, &mut scratch);
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


        // The status line.
        if !self.is_child {
            let (_, status_font_size) = cce_ui::layout::statusbar_font_parsed();
            let status_size = if status_font_size > 0.0 { status_font_size } else { 12.0 };
            let scol = cce_ui::color::root_plate_statusbar_text_color();
            // style: deliberate — 12px in from the bar's edge is the toolkit StatusBar's
            // own default text offset, so the line sits where a StatusBar's text would.
            pc.text_with(
                self.status_text.clone(),
                12.0,
                self.height - 24.0,
                status_size,
                [
                    (scol[0] * 255.0) as u8,
                    (scol[1] * 255.0) as u8,
                    (scol[2] * 255.0) as u8,
                ],
                Some(cce_ui::layout::statusbar_font()),
                None,
            );
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
                return None;
            }
        }

        let mut changed = false;
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);

        if state == ElementState::Pressed {
            let mut clicked_idx = None;
            // The exhibit area's scrollbar takes a press on its track before any
            // exhibit — while it is RAISED. A sunk bar is behind the well's floor
            // and is not hit, so a press on its lane falls through to the exhibit
            // under it (`ScrollBox::hit_test_scrollbar` gates on the latch).
            let exhibit_bar = !self.is_child && self.exhibit_scroll.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if exhibit_bar {
                changed = true;
            }
            if clicked_idx.is_none() && !exhibit_bar {
                for i in (0..self.roster.len()).rev() {
                    if !is_visible(i) {
                        continue;
                    }
                    // An exhibit scrolled out of the viewport is not there to click.
                    if !self.is_child && is_exhibit(i) && !self.in_exhibit_viewport(lx, ly) {
                        continue;
                    }
                    if self.roster.get_dyn(&self.ui_context, i).hit_test(lx, ly, &self.ui_context) {
                        clicked_idx = Some(i);
                        break;
                    }
                }
            }
            if button == MouseButton::Left {
                if let Some(old) = self.focused_widget {
                    if Some(old) != clicked_idx {
                        self.ui_context.unfocus_id(self.roster.id(old));
                        self.focused_widget = None;
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
                if button == MouseButton::Left {
                    self.ui_context.focus_id(self.roster.id(i));
                    self.focused_widget = Some(i);
                }
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

            if button == MouseButton::Left {
                if self.is_child {
                    if self.roster.take_click(&mut self.ui_context, 2) {
                        return Some("exit".to_string());
                    }
                } else {
                    if self.apply_header_dropdowns() {
                        changed = true;
                    } else if self.roster.take_click(&mut self.ui_context, 12) {
                        spawn_editor("ColorRamp");
                    } else if self.roster.take_click(&mut self.ui_context, 14) {
                        spawn_editor("Ramp");
                    } else {
                        for i in [2, 3, 4, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 25, 26, 27, 28, 29] {
                            if self.roster.take_click(&mut self.ui_context, i) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
        if changed {
            *needs_rebuild = true;
        }
        None
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
        // unclaimed wheel scrolls the exhibit area.
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
            && self.exhibit_scroll.mouse_wheel(delta, lx, ly, &mut self.ui_context)
        {
            changed = true;
        }
        if changed {
            *needs_rebuild = true;
        }
    }

    fn handle_key_input(&mut self, event: &KeyEvent, needs_rebuild: &mut bool) -> Option<Self::Message> {
        let mut changed = false;
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);
        let mut handled = false;
        let key_ev = cce_ui::widget::Event::KeyInput(event.clone());
        if let Some(focused) = self.focused_widget {
            let root = self.roster.id(focused);
            if self.ui_context.propagate_event(&key_ev, root) {
                changed = true;
                handled = true;
            }
        }

        if !handled {
            for i in 0..self.roster.len() {
                if Some(i) == self.focused_widget {
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

        // A header dropdown driven by the keyboard selects on Enter: apply it now.
        if !self.is_child && self.apply_header_dropdowns() {
            changed = true;
        }

        if changed {
            *needs_rebuild = true;
        }
        None
    }

}

/// A `Command` that re-runs this binary as a child window; the caller adds the flags.
fn child_command() -> Option<std::process::Command> {
    let exe = std::env::current_exe().ok()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--child");
    Some(cmd)
}

/// Spawn `cmd` and reap it on a background thread, so the child never lingers
/// as a zombie once it exits. The same helper cce-mail, cce-files, cce-terminal
/// and cce-system-interface each keep; cce-ui's shared `process::spawn_detached`
/// went away in cce-ui 4e94236.
fn spawn_detached(mut cmd: std::process::Command) -> std::io::Result<()> {
    let mut child = cmd.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Opens a Ramp or ColorRamp editor window. Untracked on purpose: an editor outlives
/// the gallery, unlike the simulated windows Create Window spawns.
fn spawn_editor(kind: &str) {
    if let Some(mut cmd) = child_command() {
        // The editor kinds' `default_size`, spelled out for the child's argv.
        cmd.args(["--type", kind, "--width", "450", "--height", "390", "--root-plate"]);
        let _ = spawn_detached(cmd);
    }
}

/// The toolkit container layouts the Layout dropdown offers, by name.
/// The strategies at the toolkit's own spacing (`layout::control_gap()`, every strategy's
/// default gap), no padding — the exhibit viewport is already inset — so the page shows
/// the rhythm a container gets by default.
const LAYOUTS: [(&str, fn() -> Box<dyn cce_ui::widget::ContainerLayout>); 7] = [
    ("Vertical", || Box::new(VerticalLayout::default())),
    ("Columns", || Box::new(ColumnsLayout { padding_x: 0.0, padding_y: 0.0, ..ColumnsLayout::default() })),
    ("Grid", || Box::new(GridLayout { columns: 3, gap: cce_ui::layout::control_gap(), padding_x: 0.0, padding_y: 0.0 })),
    ("Adaptive Grid", || Box::new(AdaptiveGridLayout { min_col_width: 190.0, gap: cce_ui::layout::control_gap(), padding_x: 0.0, padding_y: 0.0 })),
    ("Mosaic", || Box::new(MosaicLayout { padding_x: 0.0, padding_y: 0.0, ..MosaicLayout::default() })),
    ("Reverse Mosaic", || Box::new(ReverseMosaicLayout { padding_x: 0.0, padding_y: 0.0, ..ReverseMosaicLayout::default() })),
    ("Overlay", || Box::new(OverlayLayout)),
];
const DEFAULT_LAYOUT: usize = 4;

/// The fixed positions: the chrome and the Layout dropdown. The exhibits are laid out
/// by `layout_exhibits` instead.
fn demo_positions(sw: f32, sh: f32, count: usize) -> Vec<(f32, f32, f32, f32)> {
    let mut vec = vec![(0.0, 0.0, 0.0, 0.0); count];
    vec[0] = (0.0, 0.0, sw, 40.0); // MenuBar
    vec[1] = (0.0, sh - 24.0, sw, 24.0); // StatusBar
    // The header dropdowns; the exhibits below them are laid out by
    // `layout_exhibits` with the strategy Layout selects. Their heights are the
    // widgets' own preferred heights (`apply_layout`); the ones here are placeholders.
    // Both stand on the root plate: one inset in from the window's side, one root
    // gap below the menu bar's band.
    let inset = cce_ui::layout::root_plate_inset();
    let gap = cce_ui::layout::root_plate_gap();
    let header_y = 40.0 + gap;
    vec[15] = (inset, header_y, 190.0, 0.0);
    // The Style dropdown, one root gap to its right.
    vec[34] = (inset + 190.0 + gap, header_y, 190.0, 0.0);
    vec
}

/// A child window's title, painted by `display_list` when it has no MenuBar to
/// carry it; the description below leaves this line of headroom.
const CHILD_TITLE_SIZE: f32 = 16.0;

/// The five `ChildSlots` rects for a child window of `sw`x`sh`: 0 background,
/// 1 main (editor or description), 2 Close, 3 and 4 the MenuBar / StatusBar of a
/// simulated window or the two aux Labels of a Ramp / ColorRamp editor.
/// Everything stands on the child's root plate: one root inset in from the
/// window's edges, one root gap between the main slot and Close.
fn child_positions(sw: f32, sh: f32, use_menubar: bool, use_statusbar: bool, editor: bool) -> Vec<(f32, f32, f32, f32)> {
    let dy = if use_menubar { 40.0 } else { 0.0 };
    let dh = if use_statusbar { 24.0 } else { 0.0 };
    let inner_h = sh - dy - dh;
    let inset = cce_ui::layout::root_plate_inset();
    let gap = cce_ui::layout::root_plate_gap();
    let (close_w, close_h) = (100.0, cce_ui::layout::button_height());
    let mut vec = vec![(-1000.0, -1000.0, 0.0, 0.0); CHILD_COUNT];

    if use_menubar {
        vec[3] = (0.0, 0.0, sw, 40.0);
    }
    if use_statusbar {
        vec[4] = (0.0, sh - 24.0, sw, 24.0);
    }

    vec[0] = (0.0, dy, sw, inner_h);
    // Close is centred, one inset up from the bottom edge; the main slot fills the
    // room above it, one root gap off, under the title line's headroom (kept whether
    // the title is painted or a MenuBar carries it).
    let close_y = dy + inner_h - inset - close_h;
    vec[2] = ((sw - close_w) / 2.0, close_y, close_w, close_h);
    let main_y = dy + inset + CHILD_TITLE_SIZE + gap;
    let main_h = close_y - gap - main_y;
    vec[1] = (inset, main_y, sw - 2.0 * inset, main_h);
    if editor {
        // TODO(style): the two aux labels' 20px row lies over the editor's bottom
        // strip, as it always has; that overlap is the editor's layout, not a rung.
        let label_y = main_y + main_h - 20.0;
        vec[3] = (inset, label_y, 200.0, 20.0);
        vec[4] = (sw - inset - 200.0, label_y, 200.0, 20.0);
    }

    vec
}

fn main() {
    cce_ui::engine::run::<State>();
}
