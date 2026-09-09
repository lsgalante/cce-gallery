use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Label, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    Toggle, WidgetHost, Trackpad, hover_animation, TextBox, MenuBar, Group,
    Ramp, RampKey, ColorRamp, MouseButton, ElementState, Key, NamedKey, KeyEvent, MouseScrollDelta,
    ColorSelector, FontSelector, KeybindRecorder, ButtonStrip, Float3, UsageBar, StatusDot, DotStatus,
    InfoBox, InteractiveListItem, Breadcrumb, TreeList, BevelPreview, RampPreview, Separator, Splitter, Paginator,
    VerticalLayout, ColumnsLayout, GridLayout, AdaptiveGridLayout, MosaicLayout, ReverseMosaicLayout, OverlayLayout, ScrollBox,
};
mod gallery_widgets;
use gallery_widgets::{RootPlate, Plate};
use cce_ui::widget::{Adapted, LayoutConstraints, Point};
use cce_ui::widget::input::Slider2D;
use cce_ui::engine::{LogicalSize, LogicalPosition, LayerAnchor, LayerKeyboardInteractivity, LayerKind, LayerSettings};
use cce_ui::scene::paint::Prim;
use wayland_client::QueueHandle;

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
            ChildKind::Ramp | ChildKind::ColorRamp => (450.0, 350.0),
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
/// `get_dyn`/`get_dyn_mut`.
pub struct GallerySlots {
    pub menu_bar: Adapted<MenuBar>,
    pub status_bar: Adapted<StatusBar>,
    pub button_demo: Adapted<Button>,
    pub checkbox_demo: Adapted<Checkbox>,
    pub toggle_demo: Adapted<Toggle>,
    pub progress_demo: Adapted<ProgressBar>,
    pub slider_demo: Adapted<Slider>,
    pub spinbox_demo: Adapted<Spinbox>,
    pub range_slider_demo: Adapted<RangeSlider>,
    pub trackpad_demo: Adapted<Trackpad>,
    pub textbox_demo: Adapted<TextBox>,
    pub plate_demo: Adapted<Plate>,
    pub color_ramp_btn: Adapted<Button>,
    pub bevel_ramp: Adapted<Ramp>,
    pub ramp_btn: Adapted<Button>,
    pub layout_dd: Adapted<Dropdown>,
    pub color_selector_demo: Adapted<ColorSelector>,
    pub font_selector_demo: Adapted<FontSelector>,
    pub keybind_demo: Adapted<KeybindRecorder>,
    pub button_strip_demo: Adapted<ButtonStrip>,
    pub slider2d_demo: Adapted<Slider2D>,
    pub float3_demo: Adapted<Float3>,
    pub usage_bar_demo: Adapted<UsageBar>,
    pub status_dot_demo: Adapted<StatusDot>,
    pub info_box_demo: Adapted<InfoBox>,
    pub list_item_demo: Adapted<InteractiveListItem>,
    pub breadcrumb_demo: Adapted<Breadcrumb>,
    pub tree_list_demo: Adapted<TreeList>,
    pub bevel_preview_demo: Adapted<BevelPreview>,
    pub ramp_preview_demo: Adapted<RampPreview>,
    pub separator_demo: Adapted<Separator>,
    pub splitter_demo: Adapted<Splitter>,
    /// Two `Group` lassos over other exhibits (slots 32 and 33): overlays, not
    /// exhibits — laid out by their members, drawn under the exhibit clip.
    pub group_loose: Adapted<Group>,
    pub group_fitted: Adapted<Group>,
    /// The Style dropdown (slot 34), beside Layout in the header: Relief or Flat
    /// for every control at once (`cce_ui::layout::set_control_relief`).
    pub style_dd: Adapted<Dropdown>,
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
    pub widget: Box<dyn WidgetHost + 'static>,
    pub w: f32,
    pub h: f32,
    /// A content width narrower than `w`: the strategy lays the exhibit out at `w` (room
    /// for its label) and the widget is then given this width — a StatusDot stays a dot.
    pub content_w: Option<f32>,
}

impl Exhibit {
    /// Sized by the widget's own preferred (content) height, `fallback_h` when it declares none.
    fn new<W: WidgetHost + 'static>(widget: W, w: f32, fallback_h: f32) -> Self {
        let h = widget.preferred_height().unwrap_or(fallback_h);
        Exhibit { widget: Box::new(widget), w, h, content_w: None }
    }

    /// Sized by hand: for widgets whose declared height is a single row (a multiline
    /// TextBox, a vertical ButtonStrip) when the exhibit wants several.
    fn sized<W: WidgetHost + 'static>(widget: W, w: f32, h: f32) -> Self {
        Exhibit { widget: Box::new(widget), w, h, content_w: None }
    }

    fn with_content_width(mut self, w: f32) -> Self {
        self.content_w = Some(w);
        self
    }
}

impl GallerySlots {

    // Per-slot drag queries.
    pub fn draggable(&self, idx: usize) -> bool {
        if idx >= GALLERY_COUNT {
            return false; // variant exhibits never drag
        }
        match idx {
            0 => self.menu_bar.draggable(),
            1 => self.status_bar.draggable(),
            2 => self.button_demo.draggable(),
            3 => self.checkbox_demo.draggable(),
            4 => self.toggle_demo.draggable(),
            5 => self.progress_demo.draggable(),
            6 => self.slider_demo.draggable(),
            7 => self.spinbox_demo.draggable(),
            8 => self.range_slider_demo.draggable(),
            9 => self.trackpad_demo.draggable(),
            10 => self.textbox_demo.draggable(),
            11 => self.plate_demo.draggable(),
            12 => self.color_ramp_btn.draggable(),
            13 => self.bevel_ramp.draggable(),
            14 => self.ramp_btn.draggable(),
            15 => self.layout_dd.draggable(),
            34 => self.style_dd.draggable(),
            16 => self.color_selector_demo.draggable(),
            17 => self.font_selector_demo.draggable(),
            18 => self.keybind_demo.draggable(),
            19 => self.button_strip_demo.draggable(),
            20 => self.slider2d_demo.draggable(),
            21 => self.float3_demo.draggable(),
            22 => self.usage_bar_demo.draggable(),
            23 => self.status_dot_demo.draggable(),
            24 => self.info_box_demo.draggable(),
            25 => self.list_item_demo.draggable(),
            26 => self.breadcrumb_demo.draggable(),
            27 => self.tree_list_demo.draggable(),
            28 => self.bevel_preview_demo.draggable(),
            29 => self.ramp_preview_demo.draggable(),
            30 => self.separator_demo.draggable(),
            31 => self.splitter_demo.draggable(),
            32 | 33 => false,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn is_dragging(&self, idx: usize) -> bool {
        if idx >= GALLERY_COUNT {
            return false;
        }
        match idx {
            0 => self.menu_bar.is_dragging(),
            1 => self.status_bar.is_dragging(),
            2 => self.button_demo.is_dragging(),
            3 => self.checkbox_demo.is_dragging(),
            4 => self.toggle_demo.is_dragging(),
            5 => self.progress_demo.is_dragging(),
            6 => self.slider_demo.is_dragging(),
            7 => self.spinbox_demo.is_dragging(),
            8 => self.range_slider_demo.is_dragging(),
            9 => self.trackpad_demo.is_dragging(),
            10 => self.textbox_demo.is_dragging(),
            11 => self.plate_demo.is_dragging(),
            12 => self.color_ramp_btn.is_dragging(),
            13 => self.bevel_ramp.is_dragging(),
            14 => self.ramp_btn.is_dragging(),
            15 => self.layout_dd.is_dragging(),
            34 => self.style_dd.is_dragging(),
            16 => self.color_selector_demo.is_dragging(),
            17 => self.font_selector_demo.is_dragging(),
            18 => self.keybind_demo.is_dragging(),
            19 => self.button_strip_demo.is_dragging(),
            20 => self.slider2d_demo.is_dragging(),
            21 => self.float3_demo.is_dragging(),
            22 => self.usage_bar_demo.is_dragging(),
            23 => self.status_dot_demo.is_dragging(),
            24 => self.info_box_demo.is_dragging(),
            25 => self.list_item_demo.is_dragging(),
            26 => self.breadcrumb_demo.is_dragging(),
            27 => self.tree_list_demo.is_dragging(),
            28 => self.bevel_preview_demo.is_dragging(),
            29 => self.ramp_preview_demo.is_dragging(),
            30 => self.separator_demo.is_dragging(),
            31 => self.splitter_demo.is_dragging(),
            32 | 33 => false,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn(&self, idx: usize) -> &(dyn WidgetHost + 'static) {
        if idx >= GALLERY_COUNT {
            return &*self.extra[idx - GALLERY_COUNT].widget;
        }
        match idx {
            0 => &self.menu_bar,
            1 => &self.status_bar,
            2 => &self.button_demo,
            3 => &self.checkbox_demo,
            4 => &self.toggle_demo,
            5 => &self.progress_demo,
            6 => &self.slider_demo,
            7 => &self.spinbox_demo,
            8 => &self.range_slider_demo,
            9 => &self.trackpad_demo,
            10 => &self.textbox_demo,
            11 => &self.plate_demo,
            12 => &self.color_ramp_btn,
            13 => &self.bevel_ramp,
            14 => &self.ramp_btn,
            15 => &self.layout_dd,
            16 => &self.color_selector_demo,
            17 => &self.font_selector_demo,
            18 => &self.keybind_demo,
            19 => &self.button_strip_demo,
            20 => &self.slider2d_demo,
            21 => &self.float3_demo,
            22 => &self.usage_bar_demo,
            23 => &self.status_dot_demo,
            24 => &self.info_box_demo,
            25 => &self.list_item_demo,
            26 => &self.breadcrumb_demo,
            27 => &self.tree_list_demo,
            28 => &self.bevel_preview_demo,
            29 => &self.ramp_preview_demo,
            30 => &self.separator_demo,
            31 => &self.splitter_demo,
            32 => &self.group_loose,
            33 => &self.group_fitted,
            34 => &self.style_dd,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn WidgetHost + 'static) {
        if idx >= GALLERY_COUNT {
            return &mut *self.extra[idx - GALLERY_COUNT].widget;
        }
        match idx {
            0 => &mut self.menu_bar,
            1 => &mut self.status_bar,
            2 => &mut self.button_demo,
            3 => &mut self.checkbox_demo,
            4 => &mut self.toggle_demo,
            5 => &mut self.progress_demo,
            6 => &mut self.slider_demo,
            7 => &mut self.spinbox_demo,
            8 => &mut self.range_slider_demo,
            9 => &mut self.trackpad_demo,
            10 => &mut self.textbox_demo,
            11 => &mut self.plate_demo,
            12 => &mut self.color_ramp_btn,
            13 => &mut self.bevel_ramp,
            14 => &mut self.ramp_btn,
            15 => &mut self.layout_dd,
            16 => &mut self.color_selector_demo,
            17 => &mut self.font_selector_demo,
            18 => &mut self.keybind_demo,
            19 => &mut self.button_strip_demo,
            20 => &mut self.slider2d_demo,
            21 => &mut self.float3_demo,
            22 => &mut self.usage_bar_demo,
            23 => &mut self.status_dot_demo,
            24 => &mut self.info_box_demo,
            25 => &mut self.list_item_demo,
            26 => &mut self.breadcrumb_demo,
            27 => &mut self.tree_list_demo,
            28 => &mut self.bevel_preview_demo,
            29 => &mut self.ramp_preview_demo,
            30 => &mut self.separator_demo,
            31 => &mut self.splitter_demo,
            32 => &mut self.group_loose,
            33 => &mut self.group_fitted,
            34 => &mut self.style_dd,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }
}

pub enum ChildBg {
    RootPlate(Adapted<RootPlate>),
    ContentBg(Adapted<ContentBg>),
}

pub enum ChildMain {
    ColorRamp(Adapted<ColorRamp>),
    Ramp(Adapted<Ramp>),
    Desc(Adapted<Label>),
}

pub enum ChildAux3 {
    Label(Adapted<Label>),
    MenuBar(Adapted<MenuBar>),
}

pub enum ChildAux4 {
    Label(Adapted<Label>),
    StatusBar(Adapted<StatusBar>),
}

pub struct ChildSlots {
    pub bg: ChildBg,
    pub main: ChildMain,
    pub close: Adapted<Button>,
    pub aux3: ChildAux3,
    pub aux4: ChildAux4,
}

pub const CHILD_COUNT: usize = 5;

impl ChildSlots {

    pub fn draggable(&self, idx: usize) -> bool {
        match idx {
            0 => match &self.bg {
                ChildBg::RootPlate(w) => w.draggable(),
                ChildBg::ContentBg(w) => w.draggable(),
            },
            1 => match &self.main {
                ChildMain::ColorRamp(w) => w.draggable(),
                ChildMain::Ramp(w) => w.draggable(),
                ChildMain::Desc(w) => w.draggable(),
            },
            2 => self.close.draggable(),
            3 => match &self.aux3 {
                ChildAux3::Label(w) => w.draggable(),
                ChildAux3::MenuBar(w) => w.draggable(),
            },
            4 => match &self.aux4 {
                ChildAux4::Label(w) => w.draggable(),
                ChildAux4::StatusBar(w) => w.draggable(),
            },
            _ => panic!("child slot index out of range: {idx}"),
        }
    }

    pub fn is_dragging(&self, idx: usize) -> bool {
        match idx {
            0 => match &self.bg {
                ChildBg::RootPlate(w) => w.is_dragging(),
                ChildBg::ContentBg(w) => w.is_dragging(),
            },
            1 => match &self.main {
                ChildMain::ColorRamp(w) => w.is_dragging(),
                ChildMain::Ramp(w) => w.is_dragging(),
                ChildMain::Desc(w) => w.is_dragging(),
            },
            2 => self.close.is_dragging(),
            3 => match &self.aux3 {
                ChildAux3::Label(w) => w.is_dragging(),
                ChildAux3::MenuBar(w) => w.is_dragging(),
            },
            4 => match &self.aux4 {
                ChildAux4::Label(w) => w.is_dragging(),
                ChildAux4::StatusBar(w) => w.is_dragging(),
            },
            _ => panic!("child slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn(&self, idx: usize) -> &(dyn WidgetHost + 'static) {
        match idx {
            0 => match &self.bg {
                ChildBg::RootPlate(w) => w,
                ChildBg::ContentBg(w) => w,
            },
            1 => match &self.main {
                ChildMain::ColorRamp(w) => w,
                ChildMain::Ramp(w) => w,
                ChildMain::Desc(w) => w,
            },
            2 => &self.close,
            3 => match &self.aux3 {
                ChildAux3::Label(w) => w,
                ChildAux3::MenuBar(w) => w,
            },
            4 => match &self.aux4 {
                ChildAux4::Label(w) => w,
                ChildAux4::StatusBar(w) => w,
            },
            _ => panic!("child slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn WidgetHost + 'static) {
        match idx {
            0 => match &mut self.bg {
                ChildBg::RootPlate(w) => w,
                ChildBg::ContentBg(w) => w,
            },
            1 => match &mut self.main {
                ChildMain::ColorRamp(w) => w,
                ChildMain::Ramp(w) => w,
                ChildMain::Desc(w) => w,
            },
            2 => &mut self.close,
            3 => match &mut self.aux3 {
                ChildAux3::Label(w) => w,
                ChildAux3::MenuBar(w) => w,
            },
            4 => match &mut self.aux4 {
                ChildAux4::Label(w) => w,
                ChildAux4::StatusBar(w) => w,
            },
            _ => panic!("child slot index out of range: {idx}"),
        }
    }
}

/// The two roster modes. Boxed slot structs keep registered widget pointers stable while
/// the containing `State` moves.
pub enum Roster {
    Gallery(Box<GallerySlots>),
    Child(Box<ChildSlots>),
}

impl Roster {
    pub fn len(&self) -> usize {
        match self {
            Roster::Gallery(s) => GALLERY_COUNT + s.extra.len(),
            Roster::Child(_) => CHILD_COUNT,
        }
    }

    pub fn get_dyn(&self, idx: usize) -> &(dyn WidgetHost + 'static) {
        match self {
            Roster::Gallery(s) => s.get_dyn(idx),
            Roster::Child(s) => s.get_dyn(idx),
        }
    }

    pub fn draggable(&self, idx: usize) -> bool {
        match self {
            Roster::Gallery(s) => s.draggable(idx),
            Roster::Child(s) => s.draggable(idx),
        }
    }

    pub fn is_dragging(&self, idx: usize) -> bool {
        match self {
            Roster::Gallery(s) => s.is_dragging(idx),
            Roster::Child(s) => s.is_dragging(idx),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn WidgetHost + 'static) {
        match self {
            Roster::Gallery(s) => s.get_dyn_mut(idx),
            Roster::Child(s) => s.get_dyn_mut(idx),
        }
    }

    /// The gallery slots; panics in child mode (gallery-only paths assert their mode).
    pub fn gallery(&self) -> &GallerySlots {
        match self {
            Roster::Gallery(s) => s,
            Roster::Child(_) => panic!("gallery slots requested in child mode"),
        }
    }

    pub fn gallery_mut(&mut self) -> &mut GallerySlots {
        match self {
            Roster::Gallery(s) => s,
            Roster::Child(_) => panic!("gallery slots requested in child mode"),
        }
    }

    // --- Value drains: route a slot index to its concrete slot's `take_click` /
    // `value`. Arms exist for every slot the page logic drains.

    pub fn take_click(&mut self, idx: usize) -> bool {
        let s = match self {
            Roster::Gallery(s) => s,
            // The child roster drains one slot: its Close button.
            Roster::Child(c) => return idx == 2 && c.close.take_click(),
        };
        match idx {
            2 => s.button_demo.take_click(),
            3 => s.checkbox_demo.take_click(),
            4 => s.toggle_demo.take_click(),
            5 => s.progress_demo.take_click(),
            6 => s.slider_demo.take_click(),
            7 => s.spinbox_demo.take_click(),
            8 => s.range_slider_demo.take_click(),
            9 => s.trackpad_demo.take_click(),
            10 => s.textbox_demo.take_click(),
            11 => s.plate_demo.take_click(),
            12 => s.color_ramp_btn.take_click(),
            13 => s.bevel_ramp.take_click(),
            14 => s.ramp_btn.take_click(),
            15 => s.layout_dd.take_click(),
            34 => s.style_dd.take_click(),
            16 => s.color_selector_demo.take_click(),
            17 => s.font_selector_demo.take_click(),
            18 => s.keybind_demo.take_click(),
            19 => s.button_strip_demo.take_click(),
            20 => s.slider2d_demo.take_click(),
            21 => s.float3_demo.take_click(),
            22 => s.usage_bar_demo.take_click(),
            23 => s.status_dot_demo.take_click(),
            24 => s.info_box_demo.take_click(),
            25 => s.list_item_demo.take_click(),
            26 => s.breadcrumb_demo.take_click(),
            27 => s.tree_list_demo.take_click(),
            28 => s.bevel_preview_demo.take_click(),
            29 => s.ramp_preview_demo.take_click(),
            30 => s.separator_demo.take_click(),
            31 => s.splitter_demo.take_click(),
            32 | 33 => false,
            // The variant exhibits are looked at, not drained.
            i if i >= GALLERY_COUNT => false,
            _ => panic!("take_click: unwired gallery slot {idx}"),
        }
    }

    pub fn value(&self, idx: usize) -> i32 {
        let s = self.gallery();
        match idx {
            15 => s.layout_dd.value(),
            34 => s.style_dd.value(),
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
/// variants — a constructor (`Button::new_reset`), a builder (`with_band`), or a
/// per-widget override of a config style (`with_slide`) — so the page shows each
/// look a widget can take, in the toolkit's default size for it.
///
/// The relief-off look is NOT a variant here: the header's Style dropdown switches
/// every control between Relief and Flat at once (`set_control_relief`), so each
/// exhibit shows both, and no widget appears twice for its style alone.
fn variant_exhibits() -> Vec<Exhibit> {
    const W: f32 = 190.0;
    let bh = cce_ui::layout::button_height();
    let tgh = cce_ui::layout::toggle_height();
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
        Exhibit::new(Button::new_reset(0.0, 0.0, W, bh).with_label("Button (reset)"), W, bh),
        Exhibit::new(Button::new_list_row(0.0, 0.0, W, bh).with_label("Button (list row)"), W, bh),
        Exhibit::new(Button::new_menu_item(0.0, 0.0, W, bh).with_label("Button (menu item)"), W, bh),
        Exhibit::new(Button::new_copy_icon(0.0, 0.0, bh, bh), bh, bh),
        // Toggle: the slide style (config `style.control.toggle.style`).
        Exhibit::new(Toggle::new().with_label("Toggle (slide)").with_slide(true), W, tgh),
        // Slider: with a readout.
        Exhibit::new(Slider::new().with_label("Slider (readout)").with_readout(true), W, slh),
        // TextBox: multiline, chromeless, password.
        Exhibit::sized(
            TextBox::new("TextBox (multiline)\nA second line of text.".to_string()).with_multiline(true),
            W,
            3.0 * tbh,
        ),
        Exhibit::new(TextBox::new("TextBox (chromeless)".to_string()).with_draw_bg_border(false), W, tbh),
        Exhibit::new(TextBox::new("hunter2".to_string()).with_password(true).with_label("TextBox (password)"), W, tbh),
        // ButtonStrip: the vertical column (rotated tabs), and the Paginator sidebar built on it.
        Exhibit::sized(
            Adapted::new(ButtonStrip::new(0.0, 0.0, W, TABS_H).with_buttons(three()).with_selected(Some(0)).with_vertical(true))
                .with_label("ButtonStrip (vertical)"),
            W,
            TABS_H,
        )
        .with_content_width(TAB_COLUMN_W),
        Exhibit::sized(Paginator::new(three()).with_label("Paginator"), W, TABS_H),
        // ColorSelector: the alpha swatch.
        Exhibit::new(ColorSelector::new_rgba([64, 128, 255, 128]).with_label("ColorSelector (alpha)"), W, csh),
        // Label: the plain text widget.
        Exhibit::new(Label::new("Label"), W, ddh),
        // StatusDot: the other three statuses.
        Exhibit::new(StatusDot::new(DotStatus::Inactive).with_label("StatusDot (inactive)"), DOT_W, StatusDot::SIZE)
            .with_content_width(StatusDot::SIZE),
        Exhibit::new(StatusDot::new(DotStatus::Warning).with_label("StatusDot (warning)"), DOT_W, StatusDot::SIZE)
            .with_content_width(StatusDot::SIZE),
        Exhibit::new(StatusDot::new(DotStatus::Error).with_label("StatusDot (error)"), DOT_W, StatusDot::SIZE)
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
            .get_dyn(20)
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
    fn exhibit_viewport(&self) -> (f32, f32, f32, f32) {
        let (dx, dy, _, dh) = self.roster.get_dyn(15).rect();
        let (x, y) = (dx, dy + dh + GAP);
        let w = (self.width - 2.0 * x).max(300.0);
        let h = ((self.height - 24.0) - y).max(100.0);
        (x, y, w, h)
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
        // The fitted lasso's plate is the exhibit area: its sides snap to these edges.
        if let Roster::Gallery(s) = &mut self.roster {
            s.group_fitted.inner_mut().set_plate(cce_ui::scene::layout::Rect { x, y, width: w, height: h }, cce_ui::color::root_plate_corner_radius());
        }
        let mut children: Vec<*mut (dyn WidgetHost + 'static)> = Vec::new();
        let mut indices: Vec<usize> = Vec::new();
        for (idx, cw, ch) in self.exhibit_sizes() {
            if self.roster.is_dragging(idx) {
                continue;
            }
            let widget = self.roster.get_dyn_mut(idx);
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
                let g = s.group_fitted.inner();
                (2.0 * g.padding(), g.padding() + g.headroom())
            }
            _ => (0.0, 0.0),
        };
        let strategy = (LAYOUTS.get(self.layout_idx).unwrap_or(&LAYOUTS[DEFAULT_LAYOUT]).1)();
        let content_h = strategy.layout(x + inset_x, y + inset_top, w - 2.0 * inset_x, h - inset_top, &children, &mut self.ui_context);
        self.exhibit_scroll.update_bounds(content_h + inset_top, y, h);
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
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
            let (id, ptr) = (w.base().id(), w as *mut (dyn WidgetHost + 'static));
            self.ui_context.register_widget(id, ptr);
        }
    }

    /// Drain the header dropdowns' selections and apply them: Layout (15) picks the
    /// strategy, Style (34) switches every control between Relief and Flat
    /// (`set_control_relief` — the toolkit reads it live at paint, so a rebuild is
    /// all it takes). Called after mouse AND key input: a Dropdown selects from the
    /// keyboard too (Down, Enter), and a selection must not wait for the next click.
    fn apply_header_dropdowns(&mut self) -> bool {
        let mut applied = false;
        if self.roster.take_click(15) {
            self.layout_idx = self.roster.value(15) as usize;
            applied = true;
        }
        if self.roster.take_click(34) {
            cce_ui::layout::set_control_relief(self.roster.value(34) == 0);
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
            if self.roster.is_dragging(i) {
                continue;
            }
            // The exhibits are laid out by layout_exhibits below.
            if !self.is_child && is_exhibit(i) {
                continue;
            }
            let visible = self.is_widget_visible(i);
            let (x, y, w, h) = self.positions[i];
            let widget = self.roster.get_dyn_mut(i);
            if visible && (i == 15 || i == 34) {
                // The header dropdowns land through `layout` at their own preferred
                // height: `y` is where the label goes, the content sits a strip below.
                let h = widget.preferred_height().unwrap_or(h);
                let strip = widget.label_strip();
                widget.layout(Point { x, y: y + strip }, LayoutConstraints::new(w, w, h, h), &mut self.ui_context);
                continue;
            }
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

    fn new(_qh: &QueueHandle<cce_ui::engine::EngineState<Self>>, _sender: calloop::channel::Sender<Self::Message>) -> Self {
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

        let roster = if let Some(kind) = child_kind {
            let bg = if use_root_plate {
                ChildBg::RootPlate(RootPlate::new(0.0, 0.0, c_w, c_h))
            } else {
                ChildBg::ContentBg(ContentBg::new())
            };
            let (main, aux3, aux4) = if kind == ChildKind::ColorRamp {
                (
                    ChildMain::ColorRamp(ColorRamp::new()),
                    ChildAux3::Label(Label::new("").with_font_size(12.0)),
                    ChildAux4::Label(Label::new("").with_font_size(12.0)),
                )
            } else if kind == ChildKind::Ramp {
                (
                    ChildMain::Ramp({
                        let mut ramp = Ramp::new();
                        let (loaded_keys, loaded_type) = load_bevel_ramp();
                        ramp.keys = loaded_keys;
                        ramp.line_type_dropdown.selected = match loaded_type.as_str() {
                            "bezier" => 1,
                            _ => 0,
                        };
                        ramp
                    }),
                    ChildAux3::Label(Label::new("").with_font_size(12.0)),
                    ChildAux4::Label(Label::new("").with_font_size(12.0)),
                )
            } else {
                let menu_bar = MenuBar::new(0.0, 0.0, c_w, 40.0)
                    .with_item("File", &["New", "Open", "Save", "Exit"])
                    .with_item("Edit", &["Undo", "Redo", "Cut", "Copy", "Paste"])
                    .with_right_aligned_title(true);
                (
                    ChildMain::Desc(Label::new(kind.child_text()).with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])),
                    ChildAux3::MenuBar(menu_bar),
                    ChildAux4::StatusBar(StatusBar::new()),
                )
            };
            Roster::Child(Box::new(ChildSlots {
                bg,
                main,
                close: Button::new(0.0, 0.0, 100.0, 35.0).with_label("Close"),
                aux3,
                aux4,
            }))
        } else {
            let menu_bar = MenuBar::new(0.0, 0.0, c_w, 40.0)
                .with_item("File", &["Exit"])
                .with_item("Edit", &["Settings"])
                .with_item("Help", &["About"])
                .with_right_aligned_title(true);
            Roster::Gallery(Box::new(GallerySlots {
                menu_bar,
                status_bar: StatusBar::new(),
                button_demo: Button::new(0.0, 0.0, 140.0, 40.0).with_label("Button"),
                checkbox_demo: Checkbox::new().with_label("Checkbox"),
                toggle_demo: Toggle::new().with_label("Toggle"),
                progress_demo: ProgressBar::new(0.43).with_label("ProgressBar"),
                slider_demo: Slider::new().with_label("Slider"),
                spinbox_demo: Spinbox::new(10, 1, 100, 5).with_label("Spinbox"),
                range_slider_demo: RangeSlider::new().with_label("RangeSlider"),
                trackpad_demo: Trackpad::new().with_label("Trackpad"),
                textbox_demo: TextBox::new("Interactive TextBox".to_string()),
                plate_demo: Plate::new(0.0, 0.0, 120.0, 120.0, true).with_label("Plate"),
                color_ramp_btn: Button::new(0.0, 0.0, 120.0, 28.0).with_label("Color Ramp..."),
                bevel_ramp: Ramp::new(),
                ramp_btn: Button::new(0.0, 0.0, 120.0, 28.0).with_label("Ramp..."),
                layout_dd: Dropdown::new(
                    LAYOUTS.iter().map(|(name, _)| name.to_string()).collect(),
                    DEFAULT_LAYOUT,
                ).with_label("Layout"),
                style_dd: Dropdown::new(
                    vec!["Relief".to_string(), "Flat".to_string()],
                    if cce_ui::layout::control_relief() { 0 } else { 1 },
                ).with_label("Style"),
                color_selector_demo: ColorSelector::new([64, 128, 255]).with_label("ColorSelector"),
                font_selector_demo: FontSelector::new("Sans".to_string()).with_label("FontSelector"),
                keybind_demo: KeybindRecorder::new("ctrl+1".to_string()).with_label("KeybindRecorder"),
                button_strip_demo: Adapted::new(
                    ButtonStrip::new(0.0, 0.0, 200.0, 28.0)
                        .with_buttons(vec!["One".to_string(), "Two".to_string(), "Three".to_string()])
                        .with_selected(Some(0)),
                ).with_label("ButtonStrip"),
                slider2d_demo: Slider2D::new().with_label("Slider2D"),
                float3_demo: Float3::new().with_label("Float3"),
                usage_bar_demo: UsageBar::new(0.62).with_label("UsageBar"),
                status_dot_demo: StatusDot::new(DotStatus::Active).with_label("StatusDot"),
                info_box_demo: InfoBox::new("InfoBox", vec!["A titled box of".to_string(), "plain text lines.".to_string()]),
                list_item_demo: InteractiveListItem::new("InteractiveListItem"),
                breadcrumb_demo: {
                    let mut b = Breadcrumb::new();
                    b.path = vec!["home".to_string(), "lsgalante".to_string(), "projects".to_string()];
                    b.with_label("Breadcrumb")
                },
                tree_list_demo: {
                    let mut t = TreeList::new();
                    t.set_flat_keys(vec![
                        ("layout/bar_height".to_string(), serde_json::json!(24)),
                        ("layout/gap".to_string(), serde_json::json!(12)),
                        ("theme/name".to_string(), serde_json::json!("cce")),
                    ]);
                    t.rebuild_tree();
                    t.with_label("TreeList")
                },
                bevel_preview_demo: BevelPreview::new().with_label("BevelPreview"),
                ramp_preview_demo: RampPreview::new().with_label("RampPreview"),
                separator_demo: Separator::new(0.0, 0.0, 200.0, 1.0, [0.5, 0.5, 0.6, 1.0]).with_label("Separator"),
                splitter_demo: Splitter::new(200.0).with_label("Splitter"),
                // Members are wired below, once the slots have ids.
                // Tight padding: the gallery packs its rows closer than a settings page.
                group_loose: Group::new(Vec::new()).with_label("Group").with_padding(6.0),
                group_fitted: Group::new(Vec::new()).with_label("Group (fitted)").with_fit(true).with_padding(6.0),
                extra: variant_exhibits(),
            }))
        };

        let roster = {
            let mut roster = roster;
            if let Roster::Gallery(s) = &mut roster {
                // The lassos: a loose one around the FontSelector and the StatusDot, and
                // one around the top row (Button, ButtonStrip, Checkbox, Toggle) that
                // fits the exhibit area's edges — its top snaps to the area's top with
                // the title tab kept inside, its left to the area's left edge.
                let ids = |s: &GallerySlots, idx: &[usize]| idx.iter().map(|&i| s.get_dyn(i).base().id()).collect::<Vec<_>>();
                let loose = ids(s, &[17, 23]);
                let fitted = ids(s, &[2, 19, 3, 4]);
                s.group_loose.inner_mut().set_members(loose);
                s.group_fitted.inner_mut().set_members(fitted);
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
            ui_context: cce_ui::context::UiContext::new(),
            layout_idx: DEFAULT_LAYOUT,
            exhibit_scroll: {
                let mut sb = ScrollBox::new();
                sb.show_border = false;
                sb.show_background = false;
                sb
            },
        };

        if let Some(ramp) = state.roster.get_dyn_mut(1).as_any_mut().downcast_mut::<Ramp>().filter(|_| is_child) {
            // Only a `--type Ramp` child hosts a Ramp in slot 1: it opens with the
            // preset dropdown focused so the keyboard drives it at once. Every other
            // child kind (ColorRamp, or the description Label of the Toplevel /
            // Popup / Layer* windows) starts with nothing focused — the key sweep
            // in handle_key reaches every visible child slot anyway, and a click
            // focuses whatever it lands on. This used to downcast unconditionally
            // and panic for those kinds ("child ramp widget"), which is why Create
            // Window on the Windows page spawned children that died at startup.
            let preset_ptr = ramp.preset_dropdown.as_ptr_mut();
            state.focused_widget = Some(1);
            state.ui_context.set_focused_ptr(preset_ptr);
            unsafe {
                (*preset_ptr).focus();
            }
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
        if !self.is_child && self.exhibit_scroll.tick(dt, &mut self.ui_context) {
            changed = true;
        }
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
            if is_visible(i) {
                if w.tick(dt, &mut self.ui_context) {
                    changed = true;
                    if self.child_kind == Some(ChildKind::Ramp) && i == 1 {
                        if let Some(ramp) = w.as_any().downcast_ref::<Ramp>() {
                            let line_type_str = match ramp.line_type_dropdown.selected {
                                1 => "bezier",
                                _ => "linear",
                            };
                            save_bevel_ramp(&ramp.keys, line_type_str);
                        }
                    }
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
        if (self.width - size.width as f32).abs() > 0.001 || (self.height - size.height as f32).abs() > 0.001 || (self.scale - scale).abs() > 0.001 {
            self.width = size.width as f32;
            self.height = size.height as f32;
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
            let r = cce_ui::color::root_plate_corner_radius();
            let bg_color = cce_ui::color::page_low_color();
            pc.rounded_rect(Rect { x: 0.0, y: 0.0, width: sw, height: sh }, r, (true, true, true, true), bg_color);
        }

        let push_rounded = |pc: &mut cce_ui::scene::paint::PaintCtx, qx: f32, qy: f32, qw: f32, qh: f32, qr: f32, qc: [f32; 4], qcorners: (bool, bool, bool, bool)| {
            let rect = Rect { x: qx, y: qy, width: qw, height: qh };
            if qr > 0.1 {
                pc.rounded_rect(rect, qr, qcorners, qc);
            } else {
                pc.quad(rect, qc);
            }
        };

        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            {
                let clip = if !self.is_child && (is_exhibit(i) || is_overlay(i)) { Some(self.exhibit_viewport()) } else { None };
                if let Some((vx, vy, vw, vh)) = clip {
                    pc.push_clip(Rect { x: vx, y: vy, width: vw, height: vh });
                }
                for (qx, qy, qw, qh, qr, qc, qcorners) in w.all_rounded_quads(&self.ui_context) {
                    push_rounded(&mut pc, qx, qy, qw, qh, qr, qc, qcorners);
                }
                if clip.is_some() {
                    pc.pop_clip();
                }
            }
        }

        // ── Plain geometry ──
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            {
                let clip = if !self.is_child && (is_exhibit(i) || is_overlay(i)) { Some(self.exhibit_viewport()) } else { None };
                if let Some((vx, vy, vw, vh)) = clip {
                    pc.push_clip(Rect { x: vx, y: vy, width: vw, height: vh });
                }
                for (qx, qy, qw, qh, qc) in w.all_quads(&self.ui_context) {
                    pc.quad(Rect { x: qx, y: qy, width: qw, height: qh }, qc);
                }
                // The quad bridges above carry only quads. A widget that paints
                // borders, circles, arcs or vectors (the round Checkbox's ring and
                // dot, a slider's knob, a button's border) would lose them, so
                // replay every other own prim; text stays with the text pass.
                replay_non_quad_prims(w, &self.ui_context, &mut pc);
                if clip.is_some() {
                    pc.pop_clip();
                }
            }
        }
        // The exhibit area's scrollbar, over the exhibits: the toolkit's relief
        // scrollbar (a groove track, a raised thumb — the TreeList's), the flat
        // quads only when relief is off.
        if !self.is_child {
            if cce_ui::layout::control_relief() {
                self.exhibit_scroll.paint_scrollbar_relief(&mut pc);
            } else {
                for (sx, sy, sw, sh, sc) in self.exhibit_scroll.extra_quads() {
                    pc.quad(Rect { x: sx, y: sy, width: sw, height: sh }, sc);
                }
            }
        }

        // ── Popovers: in-frame, on top of everything. PaintCtx is a
        // RenderTarget — real prims (the dropdown's expanded inset-plate
        // surface) with per-label bounds; glyphs render in the later text pass
        // regardless of emission order. ──
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
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
            let w = self.roster.get_dyn_mut(i);
            if let Some(menu_bar) = w.as_any_mut().downcast_mut::<MenuBar>() {
                menu_bar.title = label_text.clone();
                has_menu_bar = true;
            }
        }
        if !has_menu_bar {
            pc.text_with(label_text, 20.0, 12.0, 16.0, [255, 255, 255], Some(cce_ui::layout::statusbar_font()), None);
        }

        let mut popover_rects = Vec::new();
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
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
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            // A widget with a ui-tree parent (the page selector under the status bar) is
            // covered by that parent's walk — emitting it here too would draw its text
            // twice. Text inside a popover is culled; panel children clip to the panel.
            if self.ui_context.tree.parent_ptr(w.base().id()).is_some() {
                continue;
            }
            let cp_clip = if !self.is_child && (is_exhibit(i) || is_overlay(i)) {
                let (vx, vy, vw, vh) = self.exhibit_viewport();
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
        let (lx, ly) = (pos.x as f32, pos.y as f32);

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
                let root = self.roster.get_dyn(i).base().id();
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
        let (lx, ly) = (pos.x as f32, pos.y as f32);

        let mut changed = false;
        let vis = self.visibility();
        let is_visible = move |index: usize| vis.is_visible(index);

        if state == ElementState::Pressed {
            let mut clicked_idx = None;
            // The exhibit area's scrollbar takes a press on its track before any exhibit.
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
                    if self.roster.get_dyn_mut(i).hit_test(lx, ly, &self.ui_context) {
                        clicked_idx = Some(i);
                        break;
                    }
                }
            }
            if button == MouseButton::Left {
                if let Some(old) = self.focused_widget {
                    if Some(old) != clicked_idx {
                        self.roster.get_dyn_mut(old).unfocus();
                        self.focused_widget = None;
                    }
                }
            }
            if let Some(i) = clicked_idx {
                // The router records the drag target on a handled press and synthesizes
                // DragStart past its threshold; a draggable slot whose press handler
                // returned false is armed explicitly.
                let ev = cce_ui::widget::Event::MouseButton { button, state, x: lx, y: ly, local_x: lx, local_y: ly };
                let root = self.roster.get_dyn(i).base().id();
                let press_handled = self.ui_context.propagate_event(&ev, root);
                if press_handled {
                    changed = true;
                }
                if button == MouseButton::Left && !press_handled && self.roster.draggable(i) {
                    let id = self.roster.get_dyn(i).base().id();
                    self.ui_context.drag_target = Some(id);
                }
                if button == MouseButton::Left {
                    self.roster.get_dyn_mut(i).focus();
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
                let root = self.roster.get_dyn(i).base().id();
                if self.ui_context.propagate_event(&ev, root) {
                    changed = true;
                }
            }

            if button == MouseButton::Left {
                if self.is_child {
                    if self.roster.take_click(2) {
                        return Some("exit".to_string());
                    }
                } else {
                    if self.apply_header_dropdowns() {
                        changed = true;
                    } else if self.roster.take_click(12) {
                        spawn_editor("ColorRamp");
                    } else if self.roster.take_click(14) {
                        spawn_editor("Ramp");
                    } else {
                        for i in [2, 3, 4, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 25, 26, 27, 28, 29] {
                            if self.roster.take_click(i) {
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
        let (lx, ly) = (pos.x as f32, pos.y as f32);
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
            let root = self.roster.get_dyn(i).base().id();
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
            let root = self.roster.get_dyn(focused).base().id();
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
                let root = self.roster.get_dyn(i).base().id();
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

/// Re-emit a widget's own prims other than quads, rounded rects and text: the
/// gallery's paint loop bridges quads through `all_quads`/`all_rounded_quads` and
/// text through the text pass, and would otherwise drop a widget's borders,
/// circles, arcs and vectors.
fn replay_non_quad_prims(w: &(dyn WidgetHost + 'static), ui: &cce_ui::context::UiContext, pc: &mut cce_ui::scene::paint::PaintCtx) {
    let mut scratch = cce_ui::scene::paint::PaintCtx::new();
    w.paint_self(ui, &mut scratch);
    for item in scratch.finish().items {
        match item.prim {
            Prim::Quad { .. } | Prim::RoundedRect { .. } | Prim::Text { .. } => {}
            prim => {
                let _ = pc.replay(prim);
            }
        }
    }
}

/// A `Command` that re-runs this binary as a child window; the caller adds the flags.
fn child_command() -> Option<std::process::Command> {
    let exe = std::env::current_exe().ok()?;
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("--child");
    Some(cmd)
}

/// Opens a Ramp or ColorRamp editor window. Untracked on purpose: an editor outlives
/// the gallery, unlike the simulated windows Create Window spawns.
fn spawn_editor(kind: &str) {
    if let Some(mut cmd) = child_command() {
        cmd.args(["--type", kind, "--width", "450", "--height", "350", "--root-plate"]);
        let _ = cmd.spawn();
    }
}

/// The toolkit container layouts the Layout dropdown offers, by name.
/// The strategies at the toolkit's own spacing (`layout::CONTROL_GAP`, every strategy's
/// default gap), no padding — the exhibit viewport is already inset — so the page shows
/// the rhythm a container gets by default.
const LAYOUTS: [(&str, fn() -> Box<dyn cce_ui::layout::LayoutStrategy>); 7] = [
    ("Vertical", || Box::new(VerticalLayout::default())),
    ("Columns", || Box::new(ColumnsLayout { padding_x: 0.0, padding_y: 0.0, ..ColumnsLayout::default() })),
    ("Grid", || Box::new(GridLayout { columns: 3, gap: GAP, padding_x: 0.0, padding_y: 0.0, grid: None })),
    ("Adaptive Grid", || Box::new(AdaptiveGridLayout { min_col_width: 190.0, gap: GAP, padding_x: 0.0, padding_y: 0.0, grid: None })),
    ("Mosaic", || Box::new(MosaicLayout { padding_x: 0.0, padding_y: 0.0, ..MosaicLayout::default() })),
    ("Reverse Mosaic", || Box::new(ReverseMosaicLayout { padding_x: 0.0, padding_y: 0.0, ..ReverseMosaicLayout::default() })),
    ("Overlay", || Box::new(OverlayLayout::default())),
];
const GAP: f32 = cce_ui::layout::CONTROL_GAP;
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
    vec[15] = (20.0, 60.0, 190.0, 0.0);
    // The Style dropdown, one gap to its right.
    vec[34] = (20.0 + 190.0 + GAP, 60.0, 190.0, 0.0);
    vec
}

/// The five `ChildSlots` rects for a child window of `sw`x`sh`: 0 background,
/// 1 main (editor or description), 2 Close, 3 and 4 the MenuBar / StatusBar of a
/// simulated window or the two aux Labels of a Ramp / ColorRamp editor.
fn child_positions(sw: f32, sh: f32, use_menubar: bool, use_statusbar: bool, editor: bool) -> Vec<(f32, f32, f32, f32)> {
    let dy = if use_menubar { 40.0 } else { 0.0 };
    let dh = if use_statusbar { 24.0 } else { 0.0 };
    let inner_h = sh - dy - dh;
    let mut vec = vec![(-1000.0, -1000.0, 0.0, 0.0); CHILD_COUNT];

    if use_menubar {
        vec[3] = (0.0, 0.0, sw, 40.0);
    }
    if use_statusbar {
        vec[4] = (0.0, sh - 24.0, sw, 24.0);
    }

    vec[0] = (0.0, dy, sw, inner_h);
    if editor {
        vec[1] = (20.0, dy + 20.0, sw - 40.0, inner_h - 90.0);
        vec[2] = ((sw - 100.0) / 2.0, dy + inner_h - 55.0, 100.0, 35.0);
        vec[3] = (20.0, dy + inner_h - 90.0, 200.0, 20.0);
        vec[4] = (sw - 220.0, dy + inner_h - 90.0, 200.0, 20.0);
    } else {
        vec[1] = (20.0, dy + 40.0, sw - 40.0, inner_h - 110.0);
        vec[2] = ((sw - 100.0) / 2.0, dy + inner_h - 60.0, 100.0, 35.0);
    }

    vec
}

fn main() {
    cce_ui::engine::run::<State>();
}
