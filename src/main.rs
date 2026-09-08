use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Label, Panel, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    Toggle, WidgetHost, Trackpad, hover_animation, TextBox, CornerRadii, MenuBar,
    Ramp, RampKey, ColorRamp, MouseButton, ElementState, Key, NamedKey, KeyEvent, MouseScrollDelta,
    ColorSelector, FontSelector, KeybindRecorder, ButtonStrip, Float3, UsageBar, StatusDot, DotStatus,
    InfoBox, InteractiveListItem, Breadcrumb, TreeList, BevelPreview, RampPreview, Separator, Splitter,
    VerticalLayout, ColumnsLayout, GridLayout, AdaptiveGridLayout, MosaicLayout, ReverseMosaicLayout, OverlayLayout, ScrollBox,
};
mod gallery_widgets;
use gallery_widgets::{RootPlate, ControlPanel, Plate, SectionContainer};
use cce_ui::widget::Adapted;
use cce_ui::widget::input::Slider2D;
use cce_ui::engine::{Vertex, quad_vertices, LogicalSize, LogicalPosition, LayerAnchor, LayerKeyboardInteractivity, LayerKind, LayerSettings};
use cce_ui::scene::paint::Prim;
use wayland_client::QueueHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Controls,
    Windows,
    Xdg,
}

impl Page {
    fn from_index(i: usize) -> Page {
        match i {
            0 => Page::Controls,
            1 => Page::Windows,
            _ => Page::Xdg,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Page::Controls => "Controls",
            Page::Windows => "Windows",
            Page::Xdg => "XDG",
        }
    }
}

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
    /// The Windows page's dropdown, in order.
    const WINDOW_KINDS: [ChildKind; 7] = [
        ChildKind::Floating, ChildKind::Fullscreen, ChildKind::Utility,
        ChildKind::LayerTop, ChildKind::LayerOverlay, ChildKind::LayerBackground, ChildKind::Status,
    ];

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

    fn from_dropdown(i: i32) -> ChildKind {
        Self::WINDOW_KINDS.get(i.max(0) as usize).copied().unwrap_or(ChildKind::Floating)
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

    /// The dropdown entry.
    fn label(self) -> &'static str {
        match self {
            ChildKind::Floating => "Floating",
            ChildKind::Fullscreen => "Fullscreen",
            ChildKind::Utility => "Utility",
            ChildKind::LayerTop => "Layer: Top",
            ChildKind::LayerOverlay => "Layer: Overlay",
            ChildKind::LayerBackground => "Layer: Background",
            ChildKind::Status => "Status segment",
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

    /// The Windows page's description plate (about 28 characters per line).
    fn description(self) -> &'static str {
        match self {
            ChildKind::Floating => "A plain xdg_toplevel. cce\n\
maps it Floating: moved and\n\
resized freely, its edges\n\
snapping to the desktop\n\
grid's cell edges. The\n\
toggle action tiles it: every\n\
edge on a cell edge, reported\n\
as maximized. A mode_rule in\n\
config.kdl can force a mode.",
            ChildKind::Fullscreen => "An xdg_toplevel that sets\n\
fullscreen before its first\n\
commit. cce maps it straight\n\
into Fullscreen mode, covering\n\
the output with no border;\n\
the fullscreen action toggles\n\
it back to the size it mapped\n\
at, and in again.",
            ChildKind::Utility => "A toplevel declaring\n\
set_utility on the cce\n\
window-management protocol:\n\
a tool window whose contents\n\
decide its size. It floats\n\
and moves like any window\n\
but gets no resize handle\n\
and no saved geometry.",
            ChildKind::LayerTop => "A wlr-layer-shell surface\n\
on the Top layer, anchored\n\
left-top-right with an\n\
exclusive zone.\n\n\
cce reserves that strip and\n\
lays normal windows out\n\
below it, like a panel.",
            ChildKind::LayerOverlay => "A wlr-layer-shell surface\n\
on the Overlay layer,\n\
unanchored: centred above\n\
every window, keyboard focus\n\
left with the windows below.\n\n\
cce-notifier's toasts are\n\
this kind of surface.",
            ChildKind::LayerBackground => "A wlr-layer-shell surface\n\
on the Background layer,\n\
anchored to all four edges.\n\n\
cce paints it over its flat\n\
backdrop colour and under\n\
the desktop grid's cells, so\n\
it shows through the gaps\n\
between them, like a\n\
wallpaper would.",
            ChildKind::Status => "An xdg_toplevel whose app_id\n\
starts with cce-status: the\n\
WindowRole::StatusBar\n\
convention. cce docks it\n\
into the status bar at\n\
bar_height, spaced among the\n\
other segments, the way\n\
cce-status-interface's\n\
modules are.",
            ChildKind::Ramp | ChildKind::ColorRamp => "",
        }
    }
}

/// What the roster's visibility filter needs, copied out of `State` so the dispatch
/// loops can hold `&mut self.roster` while asking.
#[derive(Clone, Copy)]
struct Visibility {
    is_child: bool,
    use_menubar: bool,
    use_statusbar: bool,
    page: Page,
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
        match index {
            0..=1 | 36 => true,
            2..=7 | 20 | 21 | 26 | 27 | 37 | 39 | 40 | 42..=58 => self.page == Page::Controls,
            8..=19 | 28..=35 | 38 | 41 => self.page == Page::Windows,
            22..=25 => self.page == Page::Xdg,
            _ => false,
        }
    }
}

/// The Windows page's preview panel look, shared by its three paint passes.
struct PreviewStyle {
    root_plate: bool,
    transparency: f32,
    bg_color: [f32; 4],
}

/// The gallery roster: 59 named slots. The numeric indexes used by the positions
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
    pub panel_demo: Adapted<Panel>,
    pub create_window_btn: Adapted<Button>,
    pub opacity_toggle: Adapted<Toggle>,
    pub transparency_slider: Adapted<Slider>,
    pub transparency_label: Adapted<Label>,
    pub window_type_dd: Adapted<Dropdown>,
    pub enable_toggle: Adapted<Toggle>,
    pub width_spin: Adapted<Spinbox>,
    pub height_spin: Adapted<Spinbox>,
    pub surface_plate: Adapted<Plate>,
    pub surface_info_label: Adapted<Label>,
    pub surface_desc_label: Adapted<Label>,
    pub range_slider_demo: Adapted<RangeSlider>,
    pub trackpad_demo: Adapted<Trackpad>,
    pub portal_panel: Adapted<Panel>,
    pub portal_label: Adapted<Label>,
    pub open_dialog_btn: Adapted<Button>,
    pub save_dialog_btn: Adapted<Button>,
    pub textbox_demo: Adapted<TextBox>,
    pub plate_demo: Adapted<Plate>,
    pub root_plate_toggle: Adapted<Toggle>,
    pub menubar_toggle: Adapted<Toggle>,
    pub statusbar_toggle: Adapted<Toggle>,
    pub border_section: Adapted<SectionContainer>,
    pub bevel_toggle: Adapted<Toggle>,
    pub border_width_spin: Adapted<Spinbox>,
    pub bevel_depth_spin: Adapted<Spinbox>,
    pub elements_section: Adapted<SectionContainer>,
    pub page_selector: Adapted<Dropdown>,
    pub color_ramp_btn: Adapted<Button>,
    pub bevel_shape_btn: Adapted<Button>,
    pub bevel_ramp: Adapted<Ramp>,
    pub ramp_btn: Adapted<Button>,
    pub control_panel: Adapted<ControlPanel>,
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
}

pub const GALLERY_COUNT: usize = 59;

impl GallerySlots {

    // Per-slot drag queries.
    pub fn draggable(&self, idx: usize) -> bool {
        match idx {
            0 => self.menu_bar.draggable(),
            1 => self.status_bar.draggable(),
            2 => self.button_demo.draggable(),
            3 => self.checkbox_demo.draggable(),
            4 => self.toggle_demo.draggable(),
            5 => self.progress_demo.draggable(),
            6 => self.slider_demo.draggable(),
            7 => self.spinbox_demo.draggable(),
            8 => self.panel_demo.draggable(),
            9 => self.create_window_btn.draggable(),
            10 => self.opacity_toggle.draggable(),
            11 => self.transparency_slider.draggable(),
            12 => self.transparency_label.draggable(),
            13 => self.window_type_dd.draggable(),
            14 => self.enable_toggle.draggable(),
            15 => self.width_spin.draggable(),
            16 => self.height_spin.draggable(),
            17 => self.surface_plate.draggable(),
            18 => self.surface_info_label.draggable(),
            19 => self.surface_desc_label.draggable(),
            20 => self.range_slider_demo.draggable(),
            21 => self.trackpad_demo.draggable(),
            22 => self.portal_panel.draggable(),
            23 => self.portal_label.draggable(),
            24 => self.open_dialog_btn.draggable(),
            25 => self.save_dialog_btn.draggable(),
            26 => self.textbox_demo.draggable(),
            27 => self.plate_demo.draggable(),
            28 => self.root_plate_toggle.draggable(),
            29 => self.menubar_toggle.draggable(),
            30 => self.statusbar_toggle.draggable(),
            31 => self.border_section.draggable(),
            32 => self.bevel_toggle.draggable(),
            33 => self.border_width_spin.draggable(),
            34 => self.bevel_depth_spin.draggable(),
            35 => self.elements_section.draggable(),
            36 => self.page_selector.draggable(),
            37 => self.color_ramp_btn.draggable(),
            38 => self.bevel_shape_btn.draggable(),
            39 => self.bevel_ramp.draggable(),
            40 => self.ramp_btn.draggable(),
            41 => self.control_panel.draggable(),
            42 => self.layout_dd.draggable(),
            43 => self.color_selector_demo.draggable(),
            44 => self.font_selector_demo.draggable(),
            45 => self.keybind_demo.draggable(),
            46 => self.button_strip_demo.draggable(),
            47 => self.slider2d_demo.draggable(),
            48 => self.float3_demo.draggable(),
            49 => self.usage_bar_demo.draggable(),
            50 => self.status_dot_demo.draggable(),
            51 => self.info_box_demo.draggable(),
            52 => self.list_item_demo.draggable(),
            53 => self.breadcrumb_demo.draggable(),
            54 => self.tree_list_demo.draggable(),
            55 => self.bevel_preview_demo.draggable(),
            56 => self.ramp_preview_demo.draggable(),
            57 => self.separator_demo.draggable(),
            58 => self.splitter_demo.draggable(),
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn is_dragging(&self, idx: usize) -> bool {
        match idx {
            0 => self.menu_bar.is_dragging(),
            1 => self.status_bar.is_dragging(),
            2 => self.button_demo.is_dragging(),
            3 => self.checkbox_demo.is_dragging(),
            4 => self.toggle_demo.is_dragging(),
            5 => self.progress_demo.is_dragging(),
            6 => self.slider_demo.is_dragging(),
            7 => self.spinbox_demo.is_dragging(),
            8 => self.panel_demo.is_dragging(),
            9 => self.create_window_btn.is_dragging(),
            10 => self.opacity_toggle.is_dragging(),
            11 => self.transparency_slider.is_dragging(),
            12 => self.transparency_label.is_dragging(),
            13 => self.window_type_dd.is_dragging(),
            14 => self.enable_toggle.is_dragging(),
            15 => self.width_spin.is_dragging(),
            16 => self.height_spin.is_dragging(),
            17 => self.surface_plate.is_dragging(),
            18 => self.surface_info_label.is_dragging(),
            19 => self.surface_desc_label.is_dragging(),
            20 => self.range_slider_demo.is_dragging(),
            21 => self.trackpad_demo.is_dragging(),
            22 => self.portal_panel.is_dragging(),
            23 => self.portal_label.is_dragging(),
            24 => self.open_dialog_btn.is_dragging(),
            25 => self.save_dialog_btn.is_dragging(),
            26 => self.textbox_demo.is_dragging(),
            27 => self.plate_demo.is_dragging(),
            28 => self.root_plate_toggle.is_dragging(),
            29 => self.menubar_toggle.is_dragging(),
            30 => self.statusbar_toggle.is_dragging(),
            31 => self.border_section.is_dragging(),
            32 => self.bevel_toggle.is_dragging(),
            33 => self.border_width_spin.is_dragging(),
            34 => self.bevel_depth_spin.is_dragging(),
            35 => self.elements_section.is_dragging(),
            36 => self.page_selector.is_dragging(),
            37 => self.color_ramp_btn.is_dragging(),
            38 => self.bevel_shape_btn.is_dragging(),
            39 => self.bevel_ramp.is_dragging(),
            40 => self.ramp_btn.is_dragging(),
            41 => self.control_panel.is_dragging(),
            42 => self.layout_dd.is_dragging(),
            43 => self.color_selector_demo.is_dragging(),
            44 => self.font_selector_demo.is_dragging(),
            45 => self.keybind_demo.is_dragging(),
            46 => self.button_strip_demo.is_dragging(),
            47 => self.slider2d_demo.is_dragging(),
            48 => self.float3_demo.is_dragging(),
            49 => self.usage_bar_demo.is_dragging(),
            50 => self.status_dot_demo.is_dragging(),
            51 => self.info_box_demo.is_dragging(),
            52 => self.list_item_demo.is_dragging(),
            53 => self.breadcrumb_demo.is_dragging(),
            54 => self.tree_list_demo.is_dragging(),
            55 => self.bevel_preview_demo.is_dragging(),
            56 => self.ramp_preview_demo.is_dragging(),
            57 => self.separator_demo.is_dragging(),
            58 => self.splitter_demo.is_dragging(),
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn(&self, idx: usize) -> &(dyn WidgetHost + 'static) {
        match idx {
            0 => &self.menu_bar,
            1 => &self.status_bar,
            2 => &self.button_demo,
            3 => &self.checkbox_demo,
            4 => &self.toggle_demo,
            5 => &self.progress_demo,
            6 => &self.slider_demo,
            7 => &self.spinbox_demo,
            8 => &self.panel_demo,
            9 => &self.create_window_btn,
            10 => &self.opacity_toggle,
            11 => &self.transparency_slider,
            12 => &self.transparency_label,
            13 => &self.window_type_dd,
            14 => &self.enable_toggle,
            15 => &self.width_spin,
            16 => &self.height_spin,
            17 => &self.surface_plate,
            18 => &self.surface_info_label,
            19 => &self.surface_desc_label,
            20 => &self.range_slider_demo,
            21 => &self.trackpad_demo,
            22 => &self.portal_panel,
            23 => &self.portal_label,
            24 => &self.open_dialog_btn,
            25 => &self.save_dialog_btn,
            26 => &self.textbox_demo,
            27 => &self.plate_demo,
            28 => &self.root_plate_toggle,
            29 => &self.menubar_toggle,
            30 => &self.statusbar_toggle,
            31 => &self.border_section,
            32 => &self.bevel_toggle,
            33 => &self.border_width_spin,
            34 => &self.bevel_depth_spin,
            35 => &self.elements_section,
            36 => &self.page_selector,
            37 => &self.color_ramp_btn,
            38 => &self.bevel_shape_btn,
            39 => &self.bevel_ramp,
            40 => &self.ramp_btn,
            41 => &self.control_panel,
            42 => &self.layout_dd,
            43 => &self.color_selector_demo,
            44 => &self.font_selector_demo,
            45 => &self.keybind_demo,
            46 => &self.button_strip_demo,
            47 => &self.slider2d_demo,
            48 => &self.float3_demo,
            49 => &self.usage_bar_demo,
            50 => &self.status_dot_demo,
            51 => &self.info_box_demo,
            52 => &self.list_item_demo,
            53 => &self.breadcrumb_demo,
            54 => &self.tree_list_demo,
            55 => &self.bevel_preview_demo,
            56 => &self.ramp_preview_demo,
            57 => &self.separator_demo,
            58 => &self.splitter_demo,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn WidgetHost + 'static) {
        match idx {
            0 => &mut self.menu_bar,
            1 => &mut self.status_bar,
            2 => &mut self.button_demo,
            3 => &mut self.checkbox_demo,
            4 => &mut self.toggle_demo,
            5 => &mut self.progress_demo,
            6 => &mut self.slider_demo,
            7 => &mut self.spinbox_demo,
            8 => &mut self.panel_demo,
            9 => &mut self.create_window_btn,
            10 => &mut self.opacity_toggle,
            11 => &mut self.transparency_slider,
            12 => &mut self.transparency_label,
            13 => &mut self.window_type_dd,
            14 => &mut self.enable_toggle,
            15 => &mut self.width_spin,
            16 => &mut self.height_spin,
            17 => &mut self.surface_plate,
            18 => &mut self.surface_info_label,
            19 => &mut self.surface_desc_label,
            20 => &mut self.range_slider_demo,
            21 => &mut self.trackpad_demo,
            22 => &mut self.portal_panel,
            23 => &mut self.portal_label,
            24 => &mut self.open_dialog_btn,
            25 => &mut self.save_dialog_btn,
            26 => &mut self.textbox_demo,
            27 => &mut self.plate_demo,
            28 => &mut self.root_plate_toggle,
            29 => &mut self.menubar_toggle,
            30 => &mut self.statusbar_toggle,
            31 => &mut self.border_section,
            32 => &mut self.bevel_toggle,
            33 => &mut self.border_width_spin,
            34 => &mut self.bevel_depth_spin,
            35 => &mut self.elements_section,
            36 => &mut self.page_selector,
            37 => &mut self.color_ramp_btn,
            38 => &mut self.bevel_shape_btn,
            39 => &mut self.bevel_ramp,
            40 => &mut self.ramp_btn,
            41 => &mut self.control_panel,
            42 => &mut self.layout_dd,
            43 => &mut self.color_selector_demo,
            44 => &mut self.font_selector_demo,
            45 => &mut self.keybind_demo,
            46 => &mut self.button_strip_demo,
            47 => &mut self.slider2d_demo,
            48 => &mut self.float3_demo,
            49 => &mut self.usage_bar_demo,
            50 => &mut self.status_dot_demo,
            51 => &mut self.info_box_demo,
            52 => &mut self.list_item_demo,
            53 => &mut self.breadcrumb_demo,
            54 => &mut self.tree_list_demo,
            55 => &mut self.bevel_preview_demo,
            56 => &mut self.ramp_preview_demo,
            57 => &mut self.separator_demo,
            58 => &mut self.splitter_demo,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }
}

/// Child-window slots: the background and three of the five slots vary by runtime flags
/// (`--type`, `--root-plate`), so those are typed enums rather than fields.
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
            Roster::Gallery(_) => GALLERY_COUNT,
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
    // `value` / `get_value_string` / `set_text`. Arms exist only for the slots the
    // page logic actually drains; a new drain site adds its arm.

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
            6 => s.slider_demo.take_click(),
            7 => s.spinbox_demo.take_click(),
            9 => s.create_window_btn.take_click(),
            10 => s.opacity_toggle.take_click(),
            11 => s.transparency_slider.take_click(),
            13 => s.window_type_dd.take_click(),
            14 => s.enable_toggle.take_click(),
            15 => s.width_spin.take_click(),
            16 => s.height_spin.take_click(),
            20 => s.range_slider_demo.take_click(),
            21 => s.trackpad_demo.take_click(),
            24 => s.open_dialog_btn.take_click(),
            25 => s.save_dialog_btn.take_click(),
            26 => s.textbox_demo.take_click(),
            27 => s.plate_demo.take_click(),
            28 => s.root_plate_toggle.take_click(),
            29 => s.menubar_toggle.take_click(),
            30 => s.statusbar_toggle.take_click(),
            32 => s.bevel_toggle.take_click(),
            33 => s.border_width_spin.take_click(),
            34 => s.bevel_depth_spin.take_click(),
            36 => s.page_selector.take_click(),
            37 => s.color_ramp_btn.take_click(),
            38 => s.bevel_shape_btn.take_click(),
            40 => s.ramp_btn.take_click(),
            42 => s.layout_dd.take_click(),
            43 => s.color_selector_demo.take_click(),
            44 => s.font_selector_demo.take_click(),
            45 => s.keybind_demo.take_click(),
            46 => s.button_strip_demo.take_click(),
            47 => s.slider2d_demo.take_click(),
            48 => s.float3_demo.take_click(),
            49 => s.usage_bar_demo.take_click(),
            50 => s.status_dot_demo.take_click(),
            51 => s.info_box_demo.take_click(),
            52 => s.list_item_demo.take_click(),
            53 => s.breadcrumb_demo.take_click(),
            54 => s.tree_list_demo.take_click(),
            55 => s.bevel_preview_demo.take_click(),
            56 => s.ramp_preview_demo.take_click(),
            57 => s.separator_demo.take_click(),
            58 => s.splitter_demo.take_click(),
            _ => panic!("take_click: unwired gallery slot {idx}"),
        }
    }

    pub fn value(&self, idx: usize) -> i32 {
        let s = self.gallery();
        match idx {
            11 => s.transparency_slider.value(),
            13 => s.window_type_dd.value(),
            15 => s.width_spin.value(),
            16 => s.height_spin.value(),
            33 => s.border_width_spin.value(),
            34 => s.bevel_depth_spin.value(),
            36 => s.page_selector.value(),
            42 => s.layout_dd.value(),
            _ => panic!("value: unwired gallery slot {idx}"),
        }
    }

    pub fn get_value_string(&self, idx: usize) -> Option<String> {
        let s = self.gallery();
        match idx {
            10 => s.opacity_toggle.get_value_string(),
            14 => s.enable_toggle.get_value_string(),
            28 => s.root_plate_toggle.get_value_string(),
            29 => s.menubar_toggle.get_value_string(),
            30 => s.statusbar_toggle.get_value_string(),
            32 => s.bevel_toggle.get_value_string(),
            _ => panic!("get_value_string: unwired gallery slot {idx}"),
        }
    }

    pub fn set_text(&mut self, idx: usize, text: &str) {
        let s = self.gallery_mut();
        match idx {
            19 => s.surface_desc_label.set_text(text),
            _ => panic!("set_text: unwired gallery slot {idx}"),
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

    current_page: Page,
    is_child: bool,
    opacity: bool,
    transparency: f32,
    use_root_plate: bool,
    use_menubar: bool,
    use_statusbar: bool,
    border_enabled: bool,
    child_kind: Option<ChildKind>,
    ui_context: cce_ui::context::UiContext,
    bevel_ramp: Vec<RampKey>,
    bevel_ramp_line_type: String,
    last_ramp_mod: Option<std::time::SystemTime>,

    sender: calloop::channel::Sender<String>,
    layout_idx: usize,
    /// The Controls page's scroll frame: the exhibits below the Layout dropdown scroll
    /// inside it, the way the Windows page's panel children scroll inside the panel.
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

fn interpolate_ramp_value(keys: &[RampKey], u: f32, line_type: &str) -> f32 {
    if keys.is_empty() {
        return 0.5;
    }
    if u <= keys[0].pos {
        return keys[0].value;
    }
    if u >= keys[keys.len() - 1].pos {
        return keys[keys.len() - 1].value;
    }
    for i in 0..keys.len() - 1 {
        let k1 = &keys[i];
        let k2 = &keys[i+1];
        if u >= k1.pos && u <= k2.pos {
            let range = k2.pos - k1.pos;
            if range.abs() < 0.0001 {
                return k1.value;
            }
            let w = (u - k1.pos) / range;
            if line_type == "bezier" {
                let w_smooth = w * w * (3.0 - 2.0 * w);
                return k1.value * (1.0 - w_smooth) + k2.value * w_smooth;
            } else {
                return k1.value * (1.0 - w) + k2.value * w;
            }
        }
    }
    keys[0].value
}

fn push_bevel_slice_corners(
    wx: f32, wy: f32, ww: f32, wh: f32,
    r: f32,
    r_offset: f32,
    slice_w: f32,
    sw: f32, sh: f32,
    border_color: [f32; 4],
    color_offset: f32,
    verts: &mut Vec<Vertex>,
) {
    if r <= 0.001 {
        return;
    }
    let segments = 32;

    let rad = cce_ui::layout::light_source_position();
    let lx = rad.cos();
    let ly = -rad.sin();

    let corners = [
        (wx + r, wy + r, std::f32::consts::PI, 1.5 * std::f32::consts::PI),
        (wx + ww - r, wy + r, 1.5 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
        (wx + ww - r, wy + wh - r, 0.0, 0.5 * std::f32::consts::PI),
        (wx + r, wy + wh - r, 0.5 * std::f32::consts::PI, std::f32::consts::PI),
    ];

    for &(cx, cy, start_angle, end_angle) in &corners {
        for j in 0..segments {
            let theta1 = start_angle + (j as f32) * (end_angle - start_angle) / (segments as f32);
            let theta2 = start_angle + ((j + 1) as f32) * (end_angle - start_angle) / (segments as f32);
            let theta_mid = 0.5 * (theta1 + theta2);

            let factor = (theta_mid.cos() * lx + theta_mid.sin() * ly).clamp(-1.0, 1.0);
            let offset = factor * color_offset;

            let segment_color = [
                (border_color[0] + offset).clamp(0.0, 1.0),
                (border_color[1] + offset).clamp(0.0, 1.0),
                (border_color[2] + offset).clamp(0.0, 1.0),
                border_color[3],
            ];

            cce_ui::backend::window_runner::push_arc_background_vertices(
                cx, cy, r_offset, slice_w,
                theta1, theta2,
                sw, sh, segment_color, 1, [0.0, 0.0, -1.0],
                verts
            );
        }
    }
}

/// The Controls page's exhibits: every slot `layout_exhibits` lays out under the
/// Layout dropdown (42), which stays put.
fn is_exhibit(i: usize) -> bool {
    matches!(i, 2..=7 | 20 | 21 | 26 | 27 | 37 | 39 | 40 | 43..=58)
}

fn is_control_panel_child(i: usize) -> bool {
    matches!(i, 9..=16 | 28..=35 | 38)
}

/// The control panel's child slots, in arrangement order: the app lays them out, paints
/// them clamped to the panel viewport, and dispatches them as ordinary roots.
const CP_CHILDREN: [usize; 17] = [9, 13, 10, 11, 12, 14, 15, 16, 28, 29, 30, 31, 32, 33, 34, 35, 38];

impl State {
    /// The dissolved panel's hit gate: pointer events reach the panel's children only
    /// inside the panel rect or an open child popover (the legacy `ControlPanel::hit` +
    /// viewport-Y forwarding rule). Children sit at real screen rects now — without this
    /// gate, slots clipped BELOW the visible fold would take clicks meant for the chrome
    /// beneath them.
    fn cp_gate(&self, x: f32, y: f32) -> bool {
        let (px, py, pw, ph) = self.roster.gallery().control_panel.rect();
        if x >= px && x <= px + pw && y >= py && y <= py + ph {
            return true;
        }
        for &ci in CP_CHILDREN.iter() {
            if let Some((rx, ry, rw, rh)) = self.roster.get_dyn(ci).popover_rect() {
                if x >= rx && x <= rx + rw && y >= ry && y <= ry + rh {
                    return true;
                }
            }
        }
        false
    }
}

/// Runs the toolkit file chooser on a worker thread (it blocks its caller) and
/// reports the outcome through the app's message channel, into the status bar.
fn run_file_dialog(save: bool, sender: calloop::channel::Sender<String>) {
    std::thread::spawn(move || {
        let result = if save {
            cce_ui::file_dialog::save_file("Save File", &[]).map(|p| format!("Saved to: {}", p.display()))
        } else {
            cce_ui::file_dialog::pick_file("Open File", &[]).map(|p| format!("Selected: {}", p.display()))
        };
        let _ = sender.send(result.unwrap_or_else(|| "File dialog cancelled".to_string()));
    });
}


impl State {
    fn visibility(&self) -> Visibility {
        Visibility {
            is_child: self.is_child,
            use_menubar: self.use_menubar,
            use_statusbar: self.use_statusbar,
            page: self.current_page,
        }
    }

    fn is_widget_visible(&self, index: usize) -> bool {
        self.visibility().is_visible(index)
    }

    /// A toggle slot's state.
    fn toggled(&self, idx: usize) -> bool {
        self.roster.get_value_string(idx) == Some("true".to_string())
    }

    fn go_to_page(&mut self, page: Page) {
        self.current_page = page;
        self.update_status_text(&format!("Viewing {} Page", page.label()));
        self.apply_layout();
    }

    /// Recompute the positions table for the current size, mode and layout.
    fn relayout(&mut self) {
        self.positions = if self.is_child {
            child_positions(self.width, self.height, self.use_menubar, self.use_statusbar, self.child_kind.is_some_and(ChildKind::is_editor))
        } else {
            demo_positions(self.width, self.height)
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
        let raw: [(usize, f32, f32); 29] = [
            (2, W, bh),                                   // Button
            (46, W, bh),                                  // ButtonStrip
            (3, W, tgh),                                  // Checkbox (a toggle row)
            (4, W, tgh),                                  // Toggle
            (6, W, slh),                                  // Slider
            (20, W, rsh),                                 // RangeSlider
            (47, 64.0, 64.0),                             // Slider2D (its intrinsic square)
            (7, W, sph),                                  // Spinbox
            (48, W, Float3::preferred_height(false)),     // Float3 (three slider rows)
            (26, W, tbh),                                 // TextBox
            (45, W, tbh),                                 // KeybindRecorder (a textbox)
            (43, W, csh),                                 // ColorSelector
            (44, W, fsh),                                 // FontSelector
            (5, W, pbh),                                  // ProgressBar
            (49, W, pbh),                                 // UsageBar
            (50, StatusDot::SIZE, StatusDot::SIZE),       // StatusDot
            (57, W, 1.0),                                 // Separator (a rule)
            (58, 6.0, CANVAS_H),                          // Splitter (a vertical grip)
            (51, W, 3.0 * ddh),                           // InfoBox (title + two lines)
            (52, W, 2.0 * ddh),                           // InteractiveListItem (title + subtitle)
            (53, W, bh),                                  // Breadcrumb (button plates)
            (54, W, CANVAS_H),                            // TreeList
            (21, W, CANVAS_H),                            // Trackpad
            (27, W, CANVAS_H),                            // Plate
            (55, W, CANVAS_H),                            // BevelPreview
            (56, W, 2.0 * rmh),                           // RampPreview (a ramp's curve)
            (39, W, CANVAS_H),                            // Ramp (its editor declares 150)
            (37, W, bh),                                  // Color Ramp...
            (40, W, bh),                                  // Ramp...
        ];
        raw.to_vec()
    }

    /// Lay the Controls exhibits out with the toolkit strategy the Layout dropdown selects
    /// (`LAYOUTS`), then clip whatever runs past the status bar.
    /// The exhibits' viewport: below the Layout dropdown, above the status bar.
    fn exhibit_viewport(&self) -> (f32, f32, f32, f32) {
        let (dx, dy, _, dh) = self.roster.get_dyn(42).rect();
        let (x, y) = (dx, dy + dh + 24.0);
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
        let mut children: Vec<*mut (dyn WidgetHost + 'static)> = Vec::new();
        for (idx, cw, ch) in self.exhibit_sizes() {
            if self.roster.is_dragging(idx) {
                continue;
            }
            let widget = self.roster.get_dyn_mut(idx);
            widget.set_rect(0.0, 0.0, cw, ch);
            children.push(widget as *mut (dyn WidgetHost + 'static));
        }
        let strategy = (LAYOUTS.get(self.layout_idx).unwrap_or(&LAYOUTS[DEFAULT_LAYOUT]).1)();
        let content_h = strategy.layout(x, y, w, h, &children, &mut self.ui_context);
        self.exhibit_scroll.update_bounds(content_h, y, h);
        let scroll_y = self.exhibit_scroll.scroll_y;
        for &child in &children {
            let widget = unsafe { &mut *child };
            // The adapter inflates SOME widgets' rects by their detached label on every
            // set_rect and reports the inflated height from rect(); others place the
            // label inside their rect and get no inflation. Measure it — the height a
            // zero-height set_rect comes back as — and work in content height from there.
            let (cx, cy, cw, ch) = widget.rect();
            widget.set_rect(cx, cy, cw, 0.0);
            let l = widget.rect().3;
            let mut content = ch - l;
            // A strategy sizes a child without an intrinsic height from its inflated
            // rect and assigns that back through set_rect, so the label landed twice.
            if widget.preferred_height().is_none() {
                content -= l;
            }
            widget.set_rect(cx, cy - scroll_y, cw, content);
        }
    }

    fn preview_style(&self) -> PreviewStyle {
        let root_plate = self.toggled(28);
        let transparency = if self.toggled(10) { self.roster.value(11) as f32 / 100.0 } else { 1.0 };
        let bg_color = if root_plate {
            let mut col = cce_ui::color::page_low_color();
            col[3] = transparency;
            col
        } else {
            [0.12, 0.12, 0.15, transparency]
        };
        PreviewStyle { root_plate, transparency, bg_color }
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

    fn apply_layout(&mut self) {
        let sh = self.height;
        let limit_y = if self.is_child {
            sh
        } else {
            sh - 24.0
        };

        for i in 0..self.roster.len() {
            if self.roster.is_dragging(i) {
                continue;
            }
            // The page selector (36) is placed inside the status bar below; the panel's
            // children are laid out by arrange_control_panel at real screen coordinates.
            if !self.is_child && (i == 36 || is_control_panel_child(i)) {
                continue;
            }
            let visible = self.is_widget_visible(i);
            let (x, y, w, h) = self.positions[i];
            let widget = self.roster.get_dyn_mut(i);
            if visible {
                // Everything but the chrome and the panel clips at the status bar.
                let clipped = i != 0 && i != 1 && i != 41 && y + h > limit_y;
                let final_h = if clipped { (limit_y - y).max(0.0) } else { h };
                widget.set_rect(x, y, w, final_h);
            } else {
                widget.set_rect(-1000.0, -1000.0, 0.0, 0.0);
            }
        }

        if !self.is_child {
            let (x, y, w, h) = self.positions[41];
            self.roster.get_dyn_mut(41).set_rect(x, y, w, h);

            let sb_rect = self.roster.get_dyn_mut(1).rect();
            let ddh = cce_ui::layout::dropdown_height();
            let pad_x = 10.0;
            let pad_y = (sb_rect.3 - ddh) / 2.0;
            let dw = 120.0;
            let dx = sb_rect.0 + sb_rect.2 - dw - pad_x;
            let dy = sb_rect.1 + pad_y;
            self.roster.get_dyn_mut(36).set_rect(dx, dy, dw, ddh);

            if self.current_page == Page::Controls {
                self.layout_exhibits();
            }
            self.arrange_control_panel();
        }
    }

    /// Lay the panel's child slots out at screen coordinates, scroll offset applied, so
    /// hit-testing, dispatch, text and popovers all see real positions. Runs from
    /// apply_layout and from display_list, which is what re-arranges after a scroll.
    fn arrange_control_panel(&mut self) {
        let s = self.roster.gallery_mut();
        let (x, y, w, h) = s.control_panel.rect();
        let padding = cce_ui::layout::control_panel_padding();
        let gap = cce_ui::layout::control_panel_gap();
        // 12px reserved for the scrollbar track.
        let mut col = cce_ui::widget::ColumnLayout::new(x, y, w - 12.0, gap, padding);

        col.add_widget(&mut s.create_window_btn, 28.0);
        let width_p: *mut (dyn WidgetHost + 'static) = &mut s.width_spin;
        let height_p: *mut (dyn WidgetHost + 'static) = &mut s.height_spin;
        col.add_row(&[width_p, height_p], 42.0, 12.0);
        col.add_widget(&mut s.window_type_dd, 44.0);
        col.add_widget(&mut s.opacity_toggle, 28.0);
        col.add_widget(&mut s.enable_toggle, 28.0);
        col.add_widget(&mut s.transparency_label, 12.0);
        col.add_widget(&mut s.transparency_slider, 20.0);
        col.add_widget(&mut s.elements_section, 20.0);
        let bp_p: *mut (dyn WidgetHost + 'static) = &mut s.root_plate_toggle;
        let mb_p: *mut (dyn WidgetHost + 'static) = &mut s.menubar_toggle;
        let sb_p: *mut (dyn WidgetHost + 'static) = &mut s.statusbar_toggle;
        col.add_row(&[bp_p, mb_p, sb_p], 28.0, 10.0);
        col.add_widget(&mut s.border_section, 20.0);
        col.add_widget(&mut s.bevel_toggle, 28.0);
        let bw_p: *mut (dyn WidgetHost + 'static) = &mut s.border_width_spin;
        let bd_p: *mut (dyn WidgetHost + 'static) = &mut s.bevel_depth_spin;
        col.add_row(&[bw_p, bd_p], 42.0, 12.0);
        col.add_widget(&mut s.bevel_shape_btn, 28.0);

        let total_h = col.current_y();
        s.control_panel.scroll_box.update_bounds(total_h, y, h);

        // Content coordinates -> screen coordinates.
        let scroll_y = s.control_panel.scroll_box.scroll_y;
        if scroll_y != 0.0 {
            for &ci in CP_CHILDREN.iter() {
                let w = self.roster.get_dyn_mut(ci);
                let (cx, cy, cw, ch) = w.rect();
                w.set_rect(cx, cy - scroll_y, cw, ch);
            }
        }
    }

    fn update_status_text(&mut self, text: &str) {
        self.status_text = text.to_string();
    }
}

impl cce_ui::engine::Application for State {
    type Message = String;

    fn new(_qh: &QueueHandle<cce_ui::engine::EngineState<Self>>, sender: calloop::channel::Sender<Self::Message>) -> Self {
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
                panel_demo: Panel::new(0.0, 0.0, 400.0, 250.0),
                create_window_btn: Button::new(0.0, 0.0, 140.0, 40.0).with_label("Create Window"),
                opacity_toggle: Toggle::new().with_label("Opacity"),
                transparency_slider: Slider::new(),
                transparency_label: Label::new("Transparency Level").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4]),
                window_type_dd: Dropdown::new(
                    ChildKind::WINDOW_KINDS.iter().map(|k| k.label().to_string()).collect(),
                    0,
                ).with_label("Window Type"),
                enable_toggle: {
                    let mut t = Toggle::new().with_label("Enable");
                    t.set_toggled(true);
                    t
                },
                width_spin: Spinbox::new(400, 100, 2000, 10).with_label("Width"),
                height_spin: Spinbox::new(250, 100, 2000, 10).with_label("Height"),
                surface_plate: Plate::new(0.0, 0.0, 190.0, 250.0, true),
                surface_info_label: Label::new("Surface Info").with_font_size(12.0),
                surface_desc_label: Label::new(ChildKind::WINDOW_KINDS[0].description()).with_font_size(10.0),
                range_slider_demo: RangeSlider::new().with_label("RangeSlider"),
                trackpad_demo: Trackpad::new().with_label("Trackpad"),
                portal_panel: Panel::new(0.0, 0.0, 450.0, 200.0).with_label("File chooser"),
                portal_label: Label::new("Opens the desktop file chooser through cce-ui's file_dialog\nand reports the chosen path in the status bar.").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4]),
                open_dialog_btn: Button::new(0.0, 0.0, 180.0, 40.0).with_label("Open File"),
                save_dialog_btn: Button::new(0.0, 0.0, 180.0, 40.0).with_label("Save File"),
                textbox_demo: TextBox::new("Interactive TextBox".to_string()),
                plate_demo: Plate::new(0.0, 0.0, 120.0, 120.0, true).with_label("Plate"),
                root_plate_toggle: {
                    let mut t = Toggle::new().with_label("Root plate");
                    t.set_toggled(true);
                    t
                },
                menubar_toggle: Toggle::new().with_label("MenuBar"),
                statusbar_toggle: Toggle::new().with_label("StatusBar"),
                border_section: SectionContainer::new("Border"),
                bevel_toggle: Toggle::new().with_label("Bevel"),
                border_width_spin: Spinbox::new(1, 1, 20, 1).with_label("Border Width"),
                bevel_depth_spin: {
                    let default_depth = (cce_ui::layout::bevel_depth() * 100.0) as i32;
                    Spinbox::new(default_depth, 0, 100, 1).with_label("Bevel Depth")
                },
                elements_section: SectionContainer::new("Window Elements"),
                page_selector: Dropdown::new(vec!["Controls".to_string(), "Windows".to_string(), "XDG".to_string()], 0).with_open_upward(true),
                color_ramp_btn: Button::new(0.0, 0.0, 120.0, 28.0).with_label("Color Ramp..."),
                bevel_shape_btn: Button::new(0.0, 0.0, 120.0, 28.0).with_label("Bevel Shape..."),
                bevel_ramp: Ramp::new(),
                ramp_btn: Button::new(0.0, 0.0, 120.0, 28.0).with_label("Ramp..."),
                control_panel: ControlPanel::new().with_label("ControlPanel"),
                layout_dd: Dropdown::new(
                    LAYOUTS.iter().map(|(name, _)| name.to_string()).collect(),
                    DEFAULT_LAYOUT,
                ).with_label("Layout"),
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
            }))
        };

        let (loaded_keys, loaded_type) = load_bevel_ramp();

        let mut state = Self {
            roster,
            positions: Vec::new(),
            status_text,
            focused_widget: None,
            width: c_w,
            height: c_h,
            scale: 1.0,
            current_page: Page::Controls,
            is_child,
            opacity,
            transparency,
            use_root_plate,
            use_menubar,
            use_statusbar,
            border_enabled,
            child_kind,
            ui_context: cce_ui::context::UiContext::new(),
            bevel_ramp: loaded_keys,
            bevel_ramp_line_type: loaded_type,
            last_ramp_mod: None,
            sender,
            layout_idx: DEFAULT_LAYOUT,
            exhibit_scroll: {
                let mut sb = ScrollBox::new();
                sb.show_border = false;
                sb.show_background = false;
                sb
            },
        };

        if !is_child {
            // Link the page selector (36) under the status bar (1) in the ui tree.
            let statusbar_ptr = state.roster.get_dyn_mut(1) as *mut (dyn WidgetHost + 'static);
            let dropdown_ptr = state.roster.get_dyn_mut(36) as *mut (dyn WidgetHost + 'static);
            unsafe {
                cce_ui::widget::focus::link_parent_child(
                    &mut *statusbar_ptr,
                    &mut *dropdown_ptr,
                    &mut state.ui_context,
                );
            }
        } else if let Some(ramp) = state.roster.get_dyn_mut(1).as_any_mut().downcast_mut::<Ramp>() {
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
        if !self.is_child {
            let path = cce_ui::config::get_config_path().parent().unwrap().join("bevel_ramp.kdl");
            if let Ok(metadata) = std::fs::metadata(&path) {
                if let Ok(mod_time) = metadata.modified() {
                     if Some(mod_time) != self.last_ramp_mod {
                        self.last_ramp_mod = Some(mod_time);
                        let (keys, line_type) = load_bevel_ramp();
                        self.bevel_ramp = keys;
                        self.bevel_ramp_line_type = line_type;
                        changed = true;
                    }
                }
            }
        }
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
            if self.current_page == Page::Controls {
                self.layout_exhibits();
            }
            self.arrange_control_panel();
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
            if is_control_panel_child(i) {
                continue;
            }
            if !self.is_child && i == 8 {
                let r = cce_ui::color::root_plate_corner_radius();
                let PreviewStyle { root_plate: root_plate_enabled, transparency: transparency_val, bg_color } = self.preview_style();
                let border_bevel = self.toggled(32);

                let (wx, wy, ww, wh) = w.rect();
                if root_plate_enabled {
                    if border_bevel {
                        let t = self.roster.value(33) as f32;
                        let r_inner = (r - t).max(0.0);
                        push_rounded(&mut pc, wx + t, wy + t, ww - 2.0 * t, wh - 2.0 * t, r_inner, bg_color, (true, true, true, true));
                    } else {
                        push_rounded(&mut pc, wx, wy, ww, wh, r, bg_color, (true, true, true, true));
                    }
                }

                let menubar_enabled = self.toggled(29);
                if menubar_enabled {
                    let mut menu_color = cce_ui::color::root_plate_menubar_color();
                    menu_color[3] = transparency_val;
                    if root_plate_enabled {
                        push_rounded(&mut pc, wx, wy, ww, 30.0, r, menu_color, (true, true, false, false));
                    } else {
                        push_rounded(&mut pc, wx, wy, ww, 30.0, 0.0, menu_color, (false, false, false, false));
                    }
                }

                let statusbar_enabled = self.toggled(30);
                if statusbar_enabled {
                    let mut status_color = cce_ui::color::root_plate_statusbar_color();
                    status_color[3] = transparency_val;
                    if root_plate_enabled {
                        push_rounded(&mut pc, wx, wy + wh - 24.0, ww, 24.0, r, status_color, (false, false, true, true));
                    } else {
                        push_rounded(&mut pc, wx, wy + wh - 24.0, ww, 24.0, 0.0, status_color, (false, false, false, false));
                    }
                }

                let btn_w = 80.0;
                let btn_h = 25.0;
                let btn_x = wx + (ww - btn_w) / 2.0;
                let status_h = if statusbar_enabled { 24.0 } else { 0.0 };
                let btn_y = wy + wh - status_h - 45.0;
                let mut btn_color = cce_ui::color::button_background_color();
                btn_color[3] = transparency_val;
                push_rounded(&mut pc, btn_x, btn_y, btn_w, btn_h, 4.0, btn_color, (true, true, true, true));
            } else {
                let clip = if !self.is_child && is_exhibit(i) { Some(self.exhibit_viewport()) } else { None };
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

        // ── ControlPanel children: clamped to the panel viewport (a partially clipped
        // quad loses its radius), solid borders synthesized and their fill inset, then
        // the scrollbar last, over the children. ──
        if !self.is_child && self.current_page == Page::Windows {
            let (_, panel_y, _, panel_h) = self.roster.gallery().control_panel.rect();
            let y_start = panel_y;
            let y_end = panel_y + panel_h;
            for &ci in CP_CHILDREN.iter() {
                let w = self.roster.get_dyn(ci);
                let solid_border_opt = if w.type_name() == "Toggle" { None } else { w.solid_border() };
                let (cx, cy, cw, ch) = w.rect();

                if let Some((b_color, _thickness)) = solid_border_opt {
                    let cy_top = cy;
                    let cy_bottom = cy + ch;
                    if cy_bottom > y_start && cy_top < y_end {
                        let visible_top = cy_top.max(y_start);
                        let visible_bottom = cy_bottom.min(y_end);
                        let visible_h = visible_bottom - visible_top;
                        if visible_h > 0.0 {
                            let (child_r, child_corners) = w.corner_style();
                            let radii_adjusted = if visible_top > cy_top || visible_bottom < cy_bottom {
                                0.0
                            } else {
                                child_r
                            };
                            push_rounded(&mut pc, cx, visible_top, cw, visible_h, radii_adjusted, b_color, child_corners);
                        }
                    }
                }

                for (qx, qy, qw, qh, qr, qc, qcorners) in w.all_rounded_quads(&self.ui_context) {
                    let mut rx = qx;
                    let mut ry = qy;
                    let mut rw = qw;
                    let mut rh = qh;
                    let mut rqr = qr;

                    if let Some((_, thickness)) = solid_border_opt {
                        if (qx - cx).abs() < 0.1 && (qy - cy).abs() < 0.1 && (qw - cw).abs() < 0.1 && (qh - ch).abs() < 0.1 {
                            rx += thickness;
                            ry += thickness;
                            rw -= 2.0 * thickness;
                            rh -= 2.0 * thickness;
                            rqr = (qr - thickness).max(0.0);
                        }
                    }

                    let qy_top = ry;
                    let qy_bottom = ry + rh;
                    if qy_bottom > y_start && qy_top < y_end {
                        let visible_top = qy_top.max(y_start);
                        let visible_bottom = qy_bottom.min(y_end);
                        let visible_h = visible_bottom - visible_top;
                        if visible_h > 0.0 {
                            let radii_adjusted = if visible_top > qy_top || visible_bottom < qy_bottom {
                                0.0
                            } else {
                                rqr
                            };
                            push_rounded(&mut pc, rx, visible_top, rw, visible_h, radii_adjusted, qc, qcorners);
                        }
                    }
                }
            }
            for (sx, sy, sw, sh, sc) in self.roster.gallery().control_panel.scroll_box.extra_quads() {
                push_rounded(&mut pc, sx, sy, sw, sh, 0.0, sc, (false, false, false, false));
            }
        }

        // ── Plain geometry ──
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            if !self.is_child && i == 8 {
                let style = self.preview_style();
                let (wx, wy, ww, wh) = w.rect();
                if !style.root_plate {
                    pc.quad(Rect { x: wx, y: wy, width: ww, height: wh }, style.bg_color);
                }
            } else {
                let clip = if !self.is_child && is_exhibit(i) { Some(self.exhibit_viewport()) } else { None };
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
        // The exhibit area's scrollbar, over the exhibits.
        if !self.is_child && self.current_page == Page::Controls {
            for (sx, sy, sw, sh, sc) in self.exhibit_scroll.extra_quads() {
                pc.quad(Rect { x: sx, y: sy, width: sw, height: sh }, sc);
            }
        }

        // ── ControlPanel children, plain decorations: a child's own rounded background
        // was painted above; the rest clamps to the panel viewport. ──
        if !self.is_child && self.current_page == Page::Windows {
            let (_panel_x, panel_y, _panel_w, panel_h) = self.roster.gallery().control_panel.rect();
            let y_start = panel_y;
            let y_end = panel_y + panel_h;
            for &ci in CP_CHILDREN.iter() {
                let w = self.roster.get_dyn(ci);
                let (cx, cy, cw, ch) = w.rect();
                let has_rounded = w.corner_style().1 != (false, false, false, false);
                let has_bg = w.color()[3].abs() > 0.001;
                for (qx, qy, qw, qh, qc) in w.all_quads(&self.ui_context) {
                    if has_rounded && has_bg && (qx - cx).abs() < 0.1 && (qy - cy).abs() < 0.1 && (qw - cw).abs() < 0.1 && (qh - ch).abs() < 0.1 {
                        continue;
                    }
                    let qy_top = qy;
                    let qy_bottom = qy + qh;
                    if qy_bottom > y_start && qy_top < y_end {
                        let visible_top = qy_top.max(y_start);
                        let visible_bottom = qy_bottom.min(y_end);
                        let visible_h = visible_bottom - visible_top;
                        if visible_h > 0.0 {
                            pc.quad(Rect { x: qx, y: visible_top, width: qw, height: visible_h }, qc);
                        }
                    }
                }
                // The child's non-quad prims, clipped to the panel viewport like its quads.
                let (px, _, pw, _) = self.roster.gallery().control_panel.rect();
                pc.push_clip(Rect { x: px, y: y_start, width: pw, height: y_end - y_start });
                replay_non_quad_prims(w, &self.ui_context, &mut pc);
                pc.pop_clip();
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
            None => format!("Gallery - {}", self.current_page.label()),
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
            let cp_clip = if is_control_panel_child(i) {
                let (px_, py_, pw_, ph_) = self.roster.gallery().control_panel.rect();
                Some([px_, py_, px_ + pw_, py_ + ph_])
            } else if !self.is_child && is_exhibit(i) {
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

    fn custom_vertices(&mut self, verts: &mut Vec<Vertex>, size: LogicalSize, _scale: f64) {
        let sw = size.width as f32;
        let sh = size.height as f32;

        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }

            if !self.is_child && i == 8 {
                let border_enabled = self.toggled(14);
                if border_enabled {
                    let PreviewStyle { root_plate: root_plate_enabled, transparency: transparency_val, bg_color } = self.preview_style();

                    let mut border_color = cce_ui::color::plate_border_color().unwrap_or([0.3, 0.3, 0.4, 1.0]);
                    border_color[3] = transparency_val;
                    let t = self.roster.value(33) as f32;
                    let border_bevel = self.toggled(32);
                    let r = cce_ui::color::root_plate_corner_radius();
                    let (wx, wy, ww, wh) = w.rect();

                    if root_plate_enabled {
                        let radii = CornerRadii::new(r, r, r, r);
                        if border_bevel {
                            let slices = (t * 2.0).max(10.0) as i32;
                            let slice_w = t / slices as f32;
                            for idx in 0..slices {
                                let u_curr = idx as f32 / slices as f32;
                                let u_next = (idx + 1) as f32 / slices as f32;

                                let h_outer = interpolate_ramp_value(&self.bevel_ramp, u_curr, &self.bevel_ramp_line_type);
                                let h_inner = interpolate_ramp_value(&self.bevel_ramp, u_next, &self.bevel_ramp_line_type);

                                let d_h = h_inner - h_outer;
                                let bevel_depth = self.roster.value(34) as f32 / 100.0;
                                let color_offset = d_h * bevel_depth * 2.667;

                                 let rad = cce_ui::layout::light_source_position();
                                 let lx = rad.cos();
                                 let ly = -rad.sin();

                                 let c_offset = |factor: f32| -> [f32; 4] {
                                     let o = factor * color_offset;
                                     [
                                         (bg_color[0] + o).clamp(0.0, 1.0),
                                         (bg_color[1] + o).clamp(0.0, 1.0),
                                         (bg_color[2] + o).clamp(0.0, 1.0),
                                         bg_color[3]
                                     ]
                                 };

                                 let top_color = c_offset(-ly);
                                 let left_color = c_offset(-lx);
                                 let bottom_color = c_offset(ly);
                                 let right_color = c_offset(lx);

                                 let offset = idx as f32 * slice_w;
                                 let r_offset = (r - offset).max(0.0);
                                 verts.extend(quad_vertices(wx + r_offset, wy + offset, ww - 2.0 * r_offset, slice_w, sw, sh, top_color));
                                 verts.extend(quad_vertices(wx + offset, wy + r_offset, slice_w, wh - 2.0 * r_offset, sw, sh, left_color));
                                 verts.extend(quad_vertices(wx + r_offset, wy + wh - offset - slice_w, ww - 2.0 * r_offset, slice_w, sw, sh, bottom_color));
                                 verts.extend(quad_vertices(wx + ww - offset - slice_w, wy + r_offset, slice_w, wh - 2.0 * r_offset, sw, sh, right_color));

                                 push_bevel_slice_corners(
                                     wx, wy, ww, wh,
                                     r, r_offset, slice_w,
                                     sw, sh, bg_color, color_offset,
                                     verts
                                 );
                            }
                        } else {
                            cce_ui::backend::window_runner::push_plate_solid_border_vertices(
                                wx, wy, ww, wh, radii, t, sw, sh, border_color, [0.0, 0.0, -1.0], verts
                            );
                        }
                    } else {
                        verts.extend(quad_vertices(wx, wy, ww, t, sw, sh, border_color));
                        verts.extend(quad_vertices(wx, wy, t, wh, sw, sh, border_color));
                        verts.extend(quad_vertices(wx, wy + wh - t, ww, t, sw, sh, border_color));
                        verts.extend(quad_vertices(wx + ww - t, wy, t, wh, sw, sh, border_color));
                    }
                }
            }
        }
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
        if !self.is_child
            && self.current_page == Page::Controls
            && self.exhibit_scroll.cursor_moved(lx, ly, &mut self.ui_context)
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
            let exhibit_bar = !self.is_child
                && self.current_page == Page::Controls
                && self.exhibit_scroll.mouse_input(button, state, lx, ly, &mut self.ui_context);
            if exhibit_bar {
                changed = true;
            }
            // Panel children first: they sit inside the panel's rect, so the plain
            // reverse scan below (where the panel's higher index wins) must only see
            // the panel when no child claims the point.
            if !self.is_child && self.current_page == Page::Windows && self.cp_gate(lx, ly) {
                for &ci in CP_CHILDREN.iter() {
                    if self.roster.get_dyn_mut(ci).hit_test(lx, ly, &self.ui_context) {
                        clicked_idx = Some(ci);
                        break;
                    }
                }
            }
            if clicked_idx.is_none() && !exhibit_bar {
                for i in (0..self.roster.len()).rev() {
                    if !is_visible(i) {
                        continue;
                    }
                    if is_control_panel_child(i) {
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
            if !self.is_child && self.current_page == Page::Controls {
                self.exhibit_scroll.mouse_input(button, state, lx, ly, &mut self.ui_context);
            }
            let cp_release_ok = !self.is_child && self.cp_gate(lx, ly);
            for i in 0..self.roster.len() {
                if !is_visible(i) {
                    continue;
                }
                if is_control_panel_child(i) && !cp_release_ok {
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
                    if self.roster.take_click(36) {
                        self.go_to_page(Page::from_index(self.roster.value(36) as usize));
                        changed = true;
                    } else if self.roster.take_click(42) {
                        self.layout_idx = self.roster.value(42) as usize;
                        self.relayout();
                        self.apply_layout();
                        changed = true;
                    } else if self.current_page == Page::Controls {
                        if self.roster.take_click(37) {
                            spawn_editor("ColorRamp");
                        } else if self.roster.take_click(40) {
                            spawn_editor("Ramp");
                        } else {
                            for i in [2, 3, 4, 6, 7, 20, 21, 26, 27, 43, 44, 45, 46, 47, 48, 52, 53, 54, 55, 56] {
                                if self.roster.take_click(i) {
                                    changed = true;
                                }
                            }
                        }
                    } else if self.current_page == Page::Windows {
                        let mut create_window = false;

                        if self.roster.take_click(9) {
                            create_window = true;
                        } else if self.roster.take_click(38) {
                            spawn_editor("Ramp");
                        } else {
                            let mut dropdown_clicked = false;
                            for i in [10, 11, 13, 14, 15, 16, 28, 29, 30, 32, 33, 34] {
                                if self.roster.take_click(i) {
                                    changed = true;
                                    if i == 13 {
                                        dropdown_clicked = true;
                                    }
                                    if i == 34 {
                                        let depth = self.roster.value(34) as f32 / 100.0;
                                        if let Ok(mut registry) = cce_ui::layout::get_style_registry().write() {
                                            registry.set_float("bevel_depth", depth);
                                        }
                                    }
                                }
                            }
                            if dropdown_clicked {
                                let desc = ChildKind::from_dropdown(self.roster.value(13)).description();
                                self.roster.set_text(19, desc);
                            }
                        }

                        if create_window {
                            let kind = ChildKind::from_dropdown(self.roster.value(13));
                            let opacity_enabled = self.toggled(10);
                            let transparency_val = self.roster.value(11) as f32 / 100.0;
                            let border_enabled = self.toggled(14);
                            let custom_width = self.roster.value(15);
                            let custom_height = self.roster.value(16);

                            self.update_status_text(&format!("Spawning a {} child window...", kind.label()));
                            if let Some(mut cmd) = child_command() {
                                cmd.args(["--type", kind.arg()]);
                                if opacity_enabled {
                                    cmd.args(["--opacity", "--transparency", &transparency_val.to_string()]);
                                }
                                if !border_enabled {
                                    cmd.arg("--no-border");
                                } else {
                                    cmd.args(["--border-width", &self.roster.value(33).to_string()]);
                                    if self.toggled(32) {
                                        cmd.arg("--border-bevel");
                                    }
                                }
                                for (flag, on) in [("--root-plate", self.toggled(28)), ("--menubar", self.toggled(29)), ("--statusbar", self.toggled(30))] {
                                    if on {
                                        cmd.arg(flag);
                                    }
                                }
                                cmd.args(["--width", &custom_width.to_string(), "--height", &custom_height.to_string()]);
                                let _ = cce_ui::process::spawn_tracked(cmd);
                            }
                            changed = true;
                        }
                    } else if self.current_page == Page::Xdg {
                        let save = if self.roster.take_click(24) {
                            Some(false)
                        } else if self.roster.take_click(25) {
                            Some(true)
                        } else {
                            None
                        };
                        if let Some(save) = save {
                            self.update_status_text(if save { "Opening the Save File dialog..." } else { "Opening the Open File dialog..." });
                            run_file_dialog(save, self.sender.clone());
                            changed = true;
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
        // The panel's scroll frame gets the wheel first; a consumed wheel never reaches
        // its children.
        let mut cp_took_wheel = false;
        if !self.is_child && is_visible(41) {
            let root = self.roster.get_dyn(41).base().id();
            if self.ui_context.propagate_event(&ev, root) {
                changed = true;
                cp_took_wheel = true;
            }
        }
        let cp_wheel_ok = !self.is_child && self.cp_gate(lx, ly);
        // The exhibit area's scroll frame likewise, on the Controls page.
        let exhibits_took_wheel = !self.is_child
            && self.current_page == Page::Controls
            && self.exhibit_scroll.mouse_wheel(delta, lx, ly, &mut self.ui_context);
        if exhibits_took_wheel {
            changed = true;
        }
        let in_exhibits = !self.is_child && self.in_exhibit_viewport(lx, ly);
        for i in 0..self.roster.len() {
            if i == 41 {
                continue;
            }
            if !is_visible(i) {
                continue;
            }
            if is_control_panel_child(i) && (cp_took_wheel || !cp_wheel_ok) {
                continue;
            }
            if !self.is_child && is_exhibit(i) && (exhibits_took_wheel || !in_exhibits) {
                continue;
            }
            let root = self.roster.get_dyn(i).base().id();
            if self.ui_context.propagate_event(&ev, root) {
                changed = true;
            }
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
                if is_control_panel_child(i) {
                    continue;
                }
                let root = self.roster.get_dyn(i).base().id();
                if self.ui_context.propagate_event(&key_ev, root) {
                    changed = true;
                    // A consumed key is HANDLED, not just repaint-worthy: the
                    // Escape-quits-app fallback below is gated on !handled,
                    // and without this a dropdown that took Escape through
                    // this sweep closed its menu AND exited the app.
                    handled = true;
                }
            }
        }

        if event.state == ElementState::Pressed && !self.is_child {
            // input.kdl `cce-gallery` domain
            let m = |name: &str, default: &str| {
                cce_ui::widget::match_key_shortcut(event, &cce_ui::input::app_chord(name, default))
            };
            let page = if m("page_1", "ctrl+1") {
                Some(Page::Controls)
            } else if m("page_2", "ctrl+2") {
                Some(Page::Windows)
            } else if m("page_3", "ctrl+3") {
                Some(Page::Xdg)
            } else {
                None
            };
            if let Some(page) = page {
                self.go_to_page(page);
                changed = true;
            }
        }

        if !handled && event.state == ElementState::Pressed && event.logical_key == Key::Named(NamedKey::Escape) {
            return Some("exit".to_string());
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
const LAYOUTS: [(&str, fn() -> Box<dyn cce_ui::layout::LayoutStrategy>); 7] = [
    ("Vertical", || {
        let mut v = VerticalLayout::default();
        v.spacing = 24.0;
        Box::new(v)
    }),
    ("Columns", || Box::new(ColumnsLayout { padding_x: 0.0, padding_y: 0.0, spacing: 24.0 })),
    ("Grid", || Box::new(GridLayout { columns: 3, gap: 24.0, padding_x: 0.0, padding_y: 0.0, grid: None })),
    ("Adaptive Grid", || Box::new(AdaptiveGridLayout { min_col_width: 190.0, gap: 24.0, padding_x: 0.0, padding_y: 0.0, grid: None })),
    ("Mosaic", || Box::new(MosaicLayout { gap: 24.0, padding_x: 0.0, padding_y: 0.0 })),
    ("Reverse Mosaic", || Box::new(ReverseMosaicLayout { gap: 24.0, padding_x: 0.0, padding_y: 0.0 })),
    ("Overlay", || Box::new(OverlayLayout::default())),
];
const DEFAULT_LAYOUT: usize = 4;

/// The fixed positions: the chrome, the Layout dropdown, and the Windows and XDG pages.
/// The Controls exhibits are laid out by `layout_exhibits` instead.
fn demo_positions(sw: f32, sh: f32) -> Vec<(f32, f32, f32, f32)> {
    let base_x = 20.0;
    let sph = cce_ui::layout::spinbox_height();
    let tgh = cce_ui::layout::toggle_height();
    let slh = cce_ui::layout::slider_height();
    let bh = cce_ui::layout::button_height();
    let ddh = cce_ui::layout::dropdown_height();

    let mut vec = vec![(0.0, 0.0, 0.0, 0.0); GALLERY_COUNT];

    // Common layout elements
    vec[0] = (0.0, 0.0, sw, 40.0); // 0 MenuBar
    vec[1] = (0.0, sh - 24.0, sw, 24.0); // 1 StatusBar

    // Page 0 (Controls): the Layout dropdown; the exhibits below it are laid out by
    // `layout_exhibits` with the strategy it selects.
    vec[42] = (base_x, 60.0, 190.0, ddh); // 42 Dropdown (Layout)

    // Page 1 (Windows)
    vec[8] = (base_x, 60.0, 400.0, 250.0); // 8 Panel (Window simulation area)
    vec[9] = (base_x + 420.0, 60.0, 140.0, bh); // 9 Button (Create Window)
    vec[10] = (base_x + 420.0, 160.0, 140.0, tgh); // 10 Toggle (Opacity)
    vec[11] = (base_x + 420.0, 210.0, 140.0, slh); // 11 Slider
    vec[12] = (base_x + 420.0, 250.0, 140.0, 20.0); // 12 Label
    vec[13] = (base_x + 580.0, 60.0, 180.0, ddh); // 13 Dropdown (Window Type)
    vec[14] = (base_x + 580.0, 170.0, 120.0, tgh); // 14 Toggle (Enable)
    vec[15] = (base_x + 580.0, 210.0, 140.0, sph); // 15 Spinbox (Width)
    vec[16] = (base_x + 580.0, 255.0, 140.0, sph); // 16 Spinbox (Height)
    vec[17] = (base_x, 320.0, 190.0, 250.0); // 17 Plate
    vec[18] = (base_x + 10.0, 330.0, 170.0, 20.0); // 18 Label
    vec[19] = (base_x + 10.0, 360.0, 170.0, 180.0); // 19 Label

    // Page 2 (XDG)
    vec[22] = (base_x, 60.0, 450.0, 200.0); // 22 Panel
    vec[23] = (base_x + 20.0, 80.0, 410.0, 60.0); // 23 Label
    vec[24] = (base_x + 20.0, 160.0, 180.0, bh); // 24 Button
    vec[25] = (base_x + 220.0, 160.0, 180.0, bh); // 25 Button

    // Window Simulation options (visible when Page::Windows is active)
    vec[28] = (base_x + 420.0, 300.0, 140.0, tgh); // 28 Toggle: root plate container
    vec[29] = (base_x + 420.0, 340.0, 140.0, tgh); // 29 Toggle: MenuBar
    vec[30] = (base_x + 420.0, 380.0, 140.0, tgh); // 30 Toggle: StatusBar
    vec[31] = (base_x + 420.0, 420.0, 140.0, 20.0); // 31 SectionContainer: Border
    vec[32] = (base_x + 420.0, 450.0, 140.0, tgh); // 32 Toggle: Bevel
    vec[33] = (base_x + 420.0, 490.0, 140.0, sph); // 33 Spinbox: Border Width
    vec[34] = (base_x + 420.0, 535.0, 140.0, sph); // 34 Spinbox: Bevel Depth
    vec[35] = (base_x + 420.0, 270.0, 140.0, 20.0); // 35 SectionContainer: Window Elements
    // 36 (page selector) is placed by apply_layout, inside the status bar.
    vec[38] = (base_x + 420.0, 580.0, 140.0, bh); // 38 Button: Bevel Shape
    vec[41] = (sw - 270.0, 60.0, 250.0, sh - 100.0); // 41 ControlPanel

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
