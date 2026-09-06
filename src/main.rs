use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Label, Paginator, Panel, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    Toggle, WidgetHost, Trackpad, hover_animation, TextBox, CornerRadii, MenuBar, 
    Ramp, RampKey, ColorRamp, MouseButton, ElementState, Key, NamedKey, KeyEvent, MouseScrollDelta
};
mod gallery_widgets;
use gallery_widgets::{Backplate, ControlPanel, Plate, SectionContainer};
use cce_ui::widget::Adapted;
use cce_ui::engine::{Vertex, quad_vertices, LogicalSize, LogicalPosition};
use wayland_client::QueueHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Controls,
    Windows,
    Xdg,
}


/// The gallery roster, concretely typed (Phase 6bb): 45 named slots replacing the erased
/// `Vec<Box<dyn WidgetHost>>`. The historical numeric indexes (positions vec, dispatch loops,
/// page layouts) keep addressing the same slots through `get_dyn`/`get_dyn_mut`.
pub struct GallerySlots {
    pub menu_bar: Adapted<MenuBar>,
    pub paginator: Adapted<Paginator>,
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
    pub window_shape_dd: Adapted<Dropdown>,
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
    pub backplate_toggle: Adapted<Toggle>,
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
}

pub const GALLERY_COUNT: usize = 45;

impl GallerySlots {

    // Per-slot drag queries (the ControlPanel endgame took `draggable`/`is_dragging`
    // off `WidgetHost`).
    pub fn draggable(&self, idx: usize) -> bool {
        match idx {
            0 => self.menu_bar.draggable(),
            1 => self.paginator.draggable(),
            2 => self.status_bar.draggable(),
            3 => self.button_demo.draggable(),
            4 => self.checkbox_demo.draggable(),
            5 => self.toggle_demo.draggable(),
            6 => self.progress_demo.draggable(),
            7 => self.slider_demo.draggable(),
            8 => self.spinbox_demo.draggable(),
            9 => self.panel_demo.draggable(),
            10 => self.create_window_btn.draggable(),
            11 => self.opacity_toggle.draggable(),
            12 => self.transparency_slider.draggable(),
            13 => self.transparency_label.draggable(),
            14 => self.window_type_dd.draggable(),
            15 => self.window_shape_dd.draggable(),
            16 => self.enable_toggle.draggable(),
            17 => self.width_spin.draggable(),
            18 => self.height_spin.draggable(),
            19 => self.surface_plate.draggable(),
            20 => self.surface_info_label.draggable(),
            21 => self.surface_desc_label.draggable(),
            22 => self.range_slider_demo.draggable(),
            23 => self.trackpad_demo.draggable(),
            24 => self.portal_panel.draggable(),
            25 => self.portal_label.draggable(),
            26 => self.open_dialog_btn.draggable(),
            27 => self.save_dialog_btn.draggable(),
            28 => self.textbox_demo.draggable(),
            29 => self.plate_demo.draggable(),
            30 => self.backplate_toggle.draggable(),
            31 => self.menubar_toggle.draggable(),
            32 => self.statusbar_toggle.draggable(),
            33 => self.border_section.draggable(),
            34 => self.bevel_toggle.draggable(),
            35 => self.border_width_spin.draggable(),
            36 => self.bevel_depth_spin.draggable(),
            37 => self.elements_section.draggable(),
            38 => self.page_selector.draggable(),
            39 => self.color_ramp_btn.draggable(),
            40 => self.bevel_shape_btn.draggable(),
            41 => self.bevel_ramp.draggable(),
            42 => self.ramp_btn.draggable(),
            43 => self.control_panel.draggable(),
            44 => self.layout_dd.draggable(),
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn is_dragging(&self, idx: usize) -> bool {
        match idx {
            0 => self.menu_bar.is_dragging(),
            1 => self.paginator.is_dragging(),
            2 => self.status_bar.is_dragging(),
            3 => self.button_demo.is_dragging(),
            4 => self.checkbox_demo.is_dragging(),
            5 => self.toggle_demo.is_dragging(),
            6 => self.progress_demo.is_dragging(),
            7 => self.slider_demo.is_dragging(),
            8 => self.spinbox_demo.is_dragging(),
            9 => self.panel_demo.is_dragging(),
            10 => self.create_window_btn.is_dragging(),
            11 => self.opacity_toggle.is_dragging(),
            12 => self.transparency_slider.is_dragging(),
            13 => self.transparency_label.is_dragging(),
            14 => self.window_type_dd.is_dragging(),
            15 => self.window_shape_dd.is_dragging(),
            16 => self.enable_toggle.is_dragging(),
            17 => self.width_spin.is_dragging(),
            18 => self.height_spin.is_dragging(),
            19 => self.surface_plate.is_dragging(),
            20 => self.surface_info_label.is_dragging(),
            21 => self.surface_desc_label.is_dragging(),
            22 => self.range_slider_demo.is_dragging(),
            23 => self.trackpad_demo.is_dragging(),
            24 => self.portal_panel.is_dragging(),
            25 => self.portal_label.is_dragging(),
            26 => self.open_dialog_btn.is_dragging(),
            27 => self.save_dialog_btn.is_dragging(),
            28 => self.textbox_demo.is_dragging(),
            29 => self.plate_demo.is_dragging(),
            30 => self.backplate_toggle.is_dragging(),
            31 => self.menubar_toggle.is_dragging(),
            32 => self.statusbar_toggle.is_dragging(),
            33 => self.border_section.is_dragging(),
            34 => self.bevel_toggle.is_dragging(),
            35 => self.border_width_spin.is_dragging(),
            36 => self.bevel_depth_spin.is_dragging(),
            37 => self.elements_section.is_dragging(),
            38 => self.page_selector.is_dragging(),
            39 => self.color_ramp_btn.is_dragging(),
            40 => self.bevel_shape_btn.is_dragging(),
            41 => self.bevel_ramp.is_dragging(),
            42 => self.ramp_btn.is_dragging(),
            43 => self.control_panel.is_dragging(),
            44 => self.layout_dd.is_dragging(),
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn(&self, idx: usize) -> &(dyn WidgetHost + 'static) {
        match idx {
            0 => &self.menu_bar,
            1 => &self.paginator,
            2 => &self.status_bar,
            3 => &self.button_demo,
            4 => &self.checkbox_demo,
            5 => &self.toggle_demo,
            6 => &self.progress_demo,
            7 => &self.slider_demo,
            8 => &self.spinbox_demo,
            9 => &self.panel_demo,
            10 => &self.create_window_btn,
            11 => &self.opacity_toggle,
            12 => &self.transparency_slider,
            13 => &self.transparency_label,
            14 => &self.window_type_dd,
            15 => &self.window_shape_dd,
            16 => &self.enable_toggle,
            17 => &self.width_spin,
            18 => &self.height_spin,
            19 => &self.surface_plate,
            20 => &self.surface_info_label,
            21 => &self.surface_desc_label,
            22 => &self.range_slider_demo,
            23 => &self.trackpad_demo,
            24 => &self.portal_panel,
            25 => &self.portal_label,
            26 => &self.open_dialog_btn,
            27 => &self.save_dialog_btn,
            28 => &self.textbox_demo,
            29 => &self.plate_demo,
            30 => &self.backplate_toggle,
            31 => &self.menubar_toggle,
            32 => &self.statusbar_toggle,
            33 => &self.border_section,
            34 => &self.bevel_toggle,
            35 => &self.border_width_spin,
            36 => &self.bevel_depth_spin,
            37 => &self.elements_section,
            38 => &self.page_selector,
            39 => &self.color_ramp_btn,
            40 => &self.bevel_shape_btn,
            41 => &self.bevel_ramp,
            42 => &self.ramp_btn,
            43 => &self.control_panel,
            44 => &self.layout_dd,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn WidgetHost + 'static) {
        match idx {
            0 => &mut self.menu_bar,
            1 => &mut self.paginator,
            2 => &mut self.status_bar,
            3 => &mut self.button_demo,
            4 => &mut self.checkbox_demo,
            5 => &mut self.toggle_demo,
            6 => &mut self.progress_demo,
            7 => &mut self.slider_demo,
            8 => &mut self.spinbox_demo,
            9 => &mut self.panel_demo,
            10 => &mut self.create_window_btn,
            11 => &mut self.opacity_toggle,
            12 => &mut self.transparency_slider,
            13 => &mut self.transparency_label,
            14 => &mut self.window_type_dd,
            15 => &mut self.window_shape_dd,
            16 => &mut self.enable_toggle,
            17 => &mut self.width_spin,
            18 => &mut self.height_spin,
            19 => &mut self.surface_plate,
            20 => &mut self.surface_info_label,
            21 => &mut self.surface_desc_label,
            22 => &mut self.range_slider_demo,
            23 => &mut self.trackpad_demo,
            24 => &mut self.portal_panel,
            25 => &mut self.portal_label,
            26 => &mut self.open_dialog_btn,
            27 => &mut self.save_dialog_btn,
            28 => &mut self.textbox_demo,
            29 => &mut self.plate_demo,
            30 => &mut self.backplate_toggle,
            31 => &mut self.menubar_toggle,
            32 => &mut self.statusbar_toggle,
            33 => &mut self.border_section,
            34 => &mut self.bevel_toggle,
            35 => &mut self.border_width_spin,
            36 => &mut self.bevel_depth_spin,
            37 => &mut self.elements_section,
            38 => &mut self.page_selector,
            39 => &mut self.color_ramp_btn,
            40 => &mut self.bevel_shape_btn,
            41 => &mut self.bevel_ramp,
            42 => &mut self.ramp_btn,
            43 => &mut self.control_panel,
            44 => &mut self.layout_dd,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }
}

/// Child-window slots: the background and three of the five slots vary by runtime flags
/// (`--type`, `--backplate`), so those are typed enums rather than fields.
pub enum ChildBg {
    Backplate(Adapted<Backplate>),
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
                ChildBg::Backplate(w) => w.draggable(),
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
                ChildBg::Backplate(w) => w.is_dragging(),
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
                ChildBg::Backplate(w) => w,
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
                ChildBg::Backplate(w) => w,
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

    // --- App-local value drains (6bd value shrink): the `WidgetHost` polling block
    // (`take_click`/`value`/`get_value_string`/`set_text`) went concrete-slot — these
    // route a gallery index to its concrete slot's inherent `Adapted` method. Arms exist
    // only for the slots the page logic actually drains; a new drain site adds its arm.

    pub fn take_click(&mut self, idx: usize) -> bool {
        let s = self.gallery_mut();
        match idx {
            2 => s.status_bar.take_click(),
            3 => s.button_demo.take_click(),
            4 => s.checkbox_demo.take_click(),
            5 => s.toggle_demo.take_click(),
            7 => s.slider_demo.take_click(),
            8 => s.spinbox_demo.take_click(),
            10 => s.create_window_btn.take_click(),
            11 => s.opacity_toggle.take_click(),
            12 => s.transparency_slider.take_click(),
            14 => s.window_type_dd.take_click(),
            15 => s.window_shape_dd.take_click(),
            16 => s.enable_toggle.take_click(),
            17 => s.width_spin.take_click(),
            18 => s.height_spin.take_click(),
            22 => s.range_slider_demo.take_click(),
            23 => s.trackpad_demo.take_click(),
            26 => s.open_dialog_btn.take_click(),
            27 => s.save_dialog_btn.take_click(),
            28 => s.textbox_demo.take_click(),
            29 => s.plate_demo.take_click(),
            30 => s.backplate_toggle.take_click(),
            31 => s.menubar_toggle.take_click(),
            32 => s.statusbar_toggle.take_click(),
            34 => s.bevel_toggle.take_click(),
            35 => s.border_width_spin.take_click(),
            36 => s.bevel_depth_spin.take_click(),
            38 => s.page_selector.take_click(),
            39 => s.color_ramp_btn.take_click(),
            40 => s.bevel_shape_btn.take_click(),
            42 => s.ramp_btn.take_click(),
            44 => s.layout_dd.take_click(),
            _ => panic!("take_click: unwired gallery slot {idx}"),
        }
    }

    pub fn value(&self, idx: usize) -> i32 {
        let s = self.gallery();
        match idx {
            12 => s.transparency_slider.value(),
            14 => s.window_type_dd.value(),
            15 => s.window_shape_dd.value(),
            17 => s.width_spin.value(),
            18 => s.height_spin.value(),
            35 => s.border_width_spin.value(),
            36 => s.bevel_depth_spin.value(),
            38 => s.page_selector.value(),
            44 => s.layout_dd.value(),
            _ => panic!("value: unwired gallery slot {idx}"),
        }
    }

    pub fn get_value_string(&self, idx: usize) -> Option<String> {
        let s = self.gallery();
        match idx {
            11 => s.opacity_toggle.get_value_string(),
            16 => s.enable_toggle.get_value_string(),
            30 => s.backplate_toggle.get_value_string(),
            31 => s.menubar_toggle.get_value_string(),
            32 => s.statusbar_toggle.get_value_string(),
            34 => s.bevel_toggle.get_value_string(),
            _ => panic!("get_value_string: unwired gallery slot {idx}"),
        }
    }

    pub fn set_text(&mut self, idx: usize, text: &str) {
        let s = self.gallery_mut();
        match idx {
            21 => s.surface_desc_label.set_text(text),
            _ => panic!("set_text: unwired gallery slot {idx}"),
        }
    }
}

struct State {
    roster: Roster,
    positions: Vec<(f32, f32, f32, f32)>,

    status_text: String,

    focused_widget: Option<usize>,

    cursor_x: f32,
    cursor_y: f32,

    width: f32,
    height: f32,
    physical_width: u32,
    physical_height: u32,
    scale: f64,

    current_page: Page,
    is_child: bool,
    opacity: bool,
    transparency: f32,
    use_backplate: bool,
    use_menubar: bool,
    use_statusbar: bool,
    border_enabled: bool,
    child_type: Option<String>,
    ui_context: cce_ui::context::UiContext,
    bevel_ramp: Vec<RampKey>,
    bevel_ramp_line_type: String,
    last_ramp_mod: Option<std::time::SystemTime>,
    child_shape: Option<String>,

    sender: calloop::channel::Sender<String>,
    layout_idx: usize,
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

fn is_control_panel_child(i: usize) -> bool {
    matches!(i, 10..=18 | 30..=37 | 40)
}

/// The panel's child slots in the legacy arrangement/aggregation order (the ControlPanel
/// endgame: the panel no longer stores pointers to them — the app lays them out, emits
/// them clamped to the panel viewport, and dispatches them as ordinary routed roots).
const CP_CHILDREN: [usize; 18] = [10, 14, 15, 11, 12, 13, 16, 17, 18, 30, 31, 32, 33, 34, 35, 36, 37, 40];

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
    fn is_widget_visible(&self, index: usize) -> bool {
        if self.is_child {
            return match index {
                0..=2 => true,
                3 => self.use_menubar,
                4 => self.use_statusbar,
                _ => false,
            };
        }
        match index {
            0..=2 | 38 => true,
            3..=8 | 22 | 23 | 28 | 29 | 39 | 41 | 42 | 44 => self.current_page == Page::Controls,
            9..=21 | 30..=37 | 40 | 43 => self.current_page == Page::Windows,
            24..=27 => self.current_page == Page::Xdg,
            _ => false,
        }
    }

    /// Register every roster widget in the ui_context (idempotent — `register` is
    /// id-keyed and the boxed slots keep pointers stable). The id-rooted router
    /// (`propagate_event(event, WidgetId)`) resolves roots through this registry;
    /// TI's custom paint loop never goes through `render_widget`, which is where
    /// other apps pick registration up as a side effect.
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

        for i in 0..self.positions.len() {
            let visible = self.is_widget_visible(i);
            let pos = self.positions[i];
            if i < self.roster.len() {
                if self.roster.is_dragging(i) {
                    continue;
                }
                let widget = self.roster.get_dyn_mut(i);
                
                if !self.is_child && i == 38 {
                    continue;
                }

                if is_control_panel_child(i) {
                    // Laid out by arrange_control_panel after this loop (real screen
                    // coordinates; every consumer gates on page visibility).
                    continue;
                }
                
                if visible {
                    let (x, y, w, h) = pos;
                    let mut final_h = h;
                    if i != 0 && i != 1 && i != 2 && i != 43 {
                        if y + h > limit_y {
                            final_h = (limit_y - y).max(0.0);
                        }
                    }
                    widget.set_rect(x, y, w, final_h);
                } else {
                    widget.set_rect(-1000.0, -1000.0, 0.0, 0.0);
                }
            }
        }
        
        if !self.is_child {
            let (x, y, w, h) = self.positions[43];
            self.roster.get_dyn_mut(43).set_rect(x, y, w, h);

            let sb_rect = self.roster.get_dyn_mut(2).rect();
            let ddh = cce_ui::layout::dropdown_height();
            let pad_x = 10.0;
            let pad_y = (sb_rect.3 - ddh) / 2.0;
            let dw = 120.0;
            let dx = sb_rect.0 + sb_rect.2 - dw - pad_x;
            let dy = sb_rect.1 + pad_y;
            self.roster.get_dyn_mut(38).set_rect(dx, dy, dw, ddh);

            self.arrange_control_panel();
        }
    }

    /// Lay the panel's child slots out at SCREEN coordinates (the ControlPanel endgame).
    /// The legacy panel arranged them in content space and shifted at aggregate time;
    /// applying the scroll offset at layout time means hit-testing, dispatch, text, and
    /// popovers all see real positions. Runs from apply_layout — every rebuild — which is
    /// also what re-arranges after a scroll (the wheel handler sets needs_rebuild).
    fn arrange_control_panel(&mut self) {
        let s = self.roster.gallery_mut();
        let (x, y, w, h) = s.control_panel.rect();
        let padding = cce_ui::layout::control_panel_padding();
        let gap = cce_ui::layout::control_panel_gap();
        // 12px reserved for the scrollbar track, like the legacy arrangement.
        let mut col = cce_ui::widget::ColumnLayout::new(x, y, w - 12.0, gap, padding);

        col.add_widget(&mut s.create_window_btn, 28.0);
        let width_p: *mut (dyn WidgetHost + 'static) = &mut s.width_spin;
        let height_p: *mut (dyn WidgetHost + 'static) = &mut s.height_spin;
        col.add_row(&[width_p, height_p], 42.0, 12.0);
        col.add_widget(&mut s.window_type_dd, 44.0);
        col.add_widget(&mut s.window_shape_dd, 44.0);
        col.add_widget(&mut s.opacity_toggle, 28.0);
        col.add_widget(&mut s.enable_toggle, 28.0);
        col.add_widget(&mut s.transparency_label, 12.0);
        col.add_widget(&mut s.transparency_slider, 20.0);
        col.add_widget(&mut s.elements_section, 20.0);
        let bp_p: *mut (dyn WidgetHost + 'static) = &mut s.backplate_toggle;
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
        let is_child = args.contains(&"--child".to_string());
        let use_backplate = if is_child {
            args.contains(&"--backplate".to_string())
        } else {
            !args.contains(&"--no-backplate".to_string())
        };
        let use_menubar = args.contains(&"--menubar".to_string());
        let use_statusbar = args.contains(&"--statusbar".to_string());
        let type_idx = args.iter().position(|a| a == "--type");
        let child_type = type_idx.and_then(|i| args.get(i + 1)).cloned();
        let shape_idx = args.iter().position(|a| a == "--shape");
        let child_shape = shape_idx.and_then(|i| args.get(i + 1)).cloned();
        let opacity = args.contains(&"--opacity".to_string());
        let transp_idx = args.iter().position(|a| a == "--transparency");
        let transparency = transp_idx
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0);
        let no_border = args.contains(&"--no-border".to_string());
        let border_enabled = !no_border;
        let border_width_idx = args.iter().position(|a| a == "--border-width");
        let border_width = border_width_idx
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(1.0);
        let border_bevel = args.contains(&"--border-bevel".to_string());
        let width_idx = args.iter().position(|a| a == "--width");
        let custom_width = width_idx
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok());
        let height_idx = args.iter().position(|a| a == "--height");
        let custom_height = height_idx
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<f32>().ok());

        let (mut c_w, mut c_h): (f32, f32) = if is_child {
            if let (Some(w), Some(h)) = (custom_width, custom_height) {
                (w, h)
            } else {
                match child_type.as_deref() {
                    Some("Toplevel") => (400.0, 250.0),
                    Some("Popup") => (250.0, 150.0),
                    Some("LayerTop") => (800.0, 40.0),
                    Some("LayerOverlay") => (300.0, 180.0),
                    Some("LayerBackground") => (800.0, 600.0),
                    Some("Ramp") | Some("ColorRamp") => (450.0, 350.0),
                    _ => (400.0, 250.0),
                }
            }
        } else {
            (800.0, 600.0)
        };
        if is_child && child_shape.as_deref() == Some("circular") {
            let side = c_w.min(c_h);
            c_w = side;
            c_h = side;
        }

        let is_ramp_child = is_child && (child_type.as_deref() == Some("Ramp") || child_type.as_deref() == Some("ColorRamp"));
        let opacity = opacity || is_ramp_child;
        let status_text = "Select a test case to begin verification.".to_string();

        let roster = if is_child {
            let desc_label = match child_type.as_deref() {
                Some("Toplevel") => "This is an active simulated Toplevel window.".to_string(),
                Some("Popup") => "This is an active simulated Popup window.".to_string(),
                Some("LayerTop") => "This is an active simulated Layer Top surface.".to_string(),
                Some("LayerOverlay") => "This is an active simulated Layer Overlay surface.".to_string(),
                Some("LayerBackground") => "This is an active simulated Layer Background surface.".to_string(),
                Some(other) => format!("This is an active simulated {} window.", other),
                None => "This is an active simulated window in the window manager.".to_string(),
            };
            let bg = if use_backplate {
                let mut bp = Backplate::new(0.0, 0.0, c_w, c_h);
                if border_bevel {
                    bp.set_bevel(border_width);
                }
                ChildBg::Backplate(bp)
            } else {
                ChildBg::ContentBg(ContentBg::new())
            };
            let (main, aux3, aux4) = if child_type.as_deref() == Some("ColorRamp") {
                (
                    ChildMain::ColorRamp(ColorRamp::new()),
                    ChildAux3::Label(Label::new("").with_font_size(12.0)),
                    ChildAux4::Label(Label::new("").with_font_size(12.0)),
                )
            } else if child_type.as_deref() == Some("Ramp") {
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
                    ChildMain::Desc(Label::new(&desc_label).with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])),
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
                menu_bar: menu_bar,
                paginator: {
                    let paginator = Paginator::new(vec![]);
                    paginator
                },
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
                window_type_dd: Dropdown::new(vec![
                    "Toplevel".to_string(),
                    "Popup".to_string(),
                    "Layer: Top".to_string(),
                    "Layer: Overlay".to_string(),
                    "Layer: Background".to_string(),
                ], 0).with_label("Window Type"),
                window_shape_dd: Dropdown::new(vec![
                    "Rectangular".to_string(),
                    "Circular".to_string(),
                ], 0).with_label("Window Shape"),
                enable_toggle: {
                    let mut t = Toggle::new().with_label("Enable");
                    t.set_toggled(true);
                    t
                },
                width_spin: Spinbox::new(400, 100, 2000, 10).with_label("Width"),
                height_spin: Spinbox::new(250, 100, 2000, 10).with_label("Height"),
                surface_plate: Plate::new(0.0, 0.0, 190.0, 250.0, true),
                surface_info_label: Label::new("Surface Info").with_font_size(12.0),
                surface_desc_label: Label::new(
                    "A standard application\n\
window (xdg_toplevel).\n\
It supports tiling (cascade,\n\
split, grid), fullscreening,\n\
dragging, and resizing.\n\n\
Testing layout:\n\
cascades in cce."
                ).with_font_size(10.0),
                range_slider_demo: RangeSlider::new().with_label("RangeSlider"),
                trackpad_demo: Trackpad::new().with_label("Trackpad"),
                portal_panel: Panel::new(0.0, 0.0, 450.0, 200.0).with_label("File chooser"),
                portal_label: Label::new("Opens the desktop file chooser through cce-ui's file_dialog\nand reports the chosen path in the status bar.").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4]),
                open_dialog_btn: Button::new(0.0, 0.0, 180.0, 40.0).with_label("Open File"),
                save_dialog_btn: Button::new(0.0, 0.0, 180.0, 40.0).with_label("Save File"),
                textbox_demo: TextBox::new("Interactive TextBox".to_string()),
                plate_demo: Plate::new(0.0, 0.0, 120.0, 120.0, true).with_label("Plate"),
                backplate_toggle: {
                    let mut t = Toggle::new().with_label("Backplate");
                    t.set_toggled(true);
                    t
                },
                menubar_toggle: {
                    let mut t = Toggle::new().with_label("MenuBar");
                    t.set_toggled(false);
                    t
                },
                statusbar_toggle: {
                    let mut t = Toggle::new().with_label("StatusBar");
                    t.set_toggled(false);
                    t
                },
                border_section: SectionContainer::new("Border"),
                bevel_toggle: {
                    let mut t = Toggle::new().with_label("Bevel");
                    t.set_toggled(false);
                    t
                },
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
                layout_dd: Dropdown::new(vec![
                    "Vertical".to_string(),
                    "Columns".to_string(),
                    "Grid".to_string(),
                    "Adaptive Grid".to_string(),
                    "Overlay".to_string(),
                    "Flex".to_string(),
                    "Splitter".to_string(),
                    "Radial".to_string(),
                    "Circular Pane".to_string(),
                    "Mosaic".to_string(),
                    "Reverse Mosaic".to_string(),
                ], 9).with_label("Layout"),
            }))
        };

        let positions = if is_child {
            child_positions(c_w, c_h, use_menubar, use_statusbar, child_type.as_deref())
        } else {
            let sidebar_w = cce_ui::widget::PageSelector::sidebar_w(
                roster.gallery().paginator.as_any().downcast_ref::<cce_ui::widget::Paginator>().expect("slot 1 must be the Paginator"),
            );
            demo_positions(c_w, c_h, sidebar_w, 9, roster.gallery())
        };

        let (loaded_keys, loaded_type) = load_bevel_ramp();

        let mut state = Self {
            roster,
            positions,
            status_text,
            focused_widget: None,
            cursor_x: 0.0,
            cursor_y: 0.0,
            width: c_w,
            height: c_h,
            physical_width: c_w as u32,
            physical_height: c_h as u32,
            scale: 1.0,
            current_page: Page::Controls,
            is_child,
            opacity,
            transparency,
            use_backplate,
            use_menubar,
            use_statusbar,
            border_enabled,
            child_type: child_type.clone(),
            ui_context: cce_ui::context::UiContext::new(),
            bevel_ramp: loaded_keys,
            bevel_ramp_line_type: loaded_type,
            last_ramp_mod: None,
            child_shape: child_shape.clone(),
            sender,
            layout_idx: 9,
        };

        if !is_child {
            // Link page selector (38) under StatusBar (2) — the old set_parent + add_child
            // pair as the one tree link it always was (6bd batch 4).
            let statusbar_ptr = state.roster.get_dyn_mut(2) as *mut (dyn WidgetHost + 'static);
            let dropdown_ptr = state.roster.get_dyn_mut(38) as *mut (dyn WidgetHost + 'static);
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

        state.apply_layout();
        state
    }

    fn settings(&self) -> cce_ui::engine::WindowSettings {
        let title = if self.is_child {
            match self.child_type.as_deref() {
                Some("Toplevel") => "Simulated Toplevel Window".to_string(),
                Some("Popup") => "Simulated Popup Window".to_string(),
                Some("LayerTop") => "Simulated Layer Shell (Top) Surface".to_string(),
                Some("LayerOverlay") => "Simulated Layer Shell (Overlay) Surface".to_string(),
                Some("LayerBackground") => "Simulated Layer Shell (Background) Surface".to_string(),
                Some(other) => format!("Simulated {} Window", other),
                None => "Simulated Client Window".to_string(),
            }
        } else {
            "Gallery".to_string()
        };
        
        let mut app_id = if self.is_child {
            if let Some(ref t) = self.child_type {
                format!("cce-gallery-child-{}", t.to_lowercase())
            } else {
                "cce-gallery-child".to_string()
            }
        } else {
            "cce-gallery".to_string()
        };
        if self.is_child && self.child_shape.as_deref() == Some("circular") {
            app_id.push_str("-circular");
        }
        if !self.border_enabled {
            app_id.push_str("-noborder");
        }

        cce_ui::engine::WindowSettings {
            title,
            app_id,
            width: self.width as u32,
            height: self.height as u32,
            fullscreen: false,
            min_size: if self.is_child {
                Some((self.width as u32, self.height as u32))
            } else {
                Some((100, 100))
            },
        }
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
        let is_child = self.is_child;
        let use_menubar = self.use_menubar;
        let use_statusbar = self.use_statusbar;
        let current_page = self.current_page;
        let is_visible = move |index: usize| -> bool {
            if is_child {
                match index {
                    0..=2 => true,
                    3 => use_menubar,
                    4 => use_statusbar,
                    _ => false,
                }
            } else {
                match index {
                    0..=2 | 38 => true,
                    3..=8 | 22 | 23 | 28 | 29 | 39 | 41 | 42 | 44 => current_page == Page::Controls,
                    9..=21 | 30..=37 | 40 | 43 => current_page == Page::Windows,
                    24..=27 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
            if is_visible(i) {
                if w.tick(dt, &mut self.ui_context) {
                    changed = true;
                    if self.is_child && self.child_type.as_deref() == Some("Ramp") && i == 1 {
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
        // Phase 6af single paint path: the whole frame — the legacy view_rounded_quads
        // geometry, then the view() geometry (the wrapper's order), then every text
        // label as a prim — is this one list, rebuilt fresh each frame (the old
        // rebuild_text_items cache and its 14 invalidation call sites are gone).
        use cce_ui::scene::layout::Rect;
        self.register_roster();
        // Panel children follow the scroll offset at LAYOUT time (the dissolved panel
        // shifted at aggregate time): re-arrange every frame so a wheel scroll moves the
        // content on the frame it repaints. Idempotent and cheap (~20 set_rects).
        if !self.is_child {
            self.arrange_control_panel();
        }
        if (self.width - size.width as f32).abs() > 0.001 || (self.height - size.height as f32).abs() > 0.001 || (self.scale - scale).abs() > 0.001 {
            self.width = size.width as f32;
            self.height = size.height as f32;
            self.physical_width = (size.width * scale as f32) as u32;
            self.physical_height = (size.height * scale as f32) as u32;
            self.scale = scale;
            cce_ui::scale::set_scale_factor(scale as f32);

            self.positions = if self.is_child {
                child_positions(self.width, self.height, self.use_menubar, self.use_statusbar, self.child_type.as_deref())
            } else {
                let sidebar_w = cce_ui::widget::PageSelector::sidebar_w(
                    self.roster.get_dyn_mut(1).as_any().downcast_ref::<cce_ui::widget::Paginator>().expect("widgets[1] must be the Paginator"),
                );
                demo_positions(self.width, self.height, sidebar_w, self.layout_idx, self.roster.gallery())
            };
            self.apply_layout();
            let text = self.status_text.clone();
            self.update_status_text(&text);
        }

        let sw = self.width;
        let sh = self.height;
        let mut pc = cce_ui::scene::paint::PaintCtx::new();

        // ── Rounded geometry (the legacy view_rounded_quads body) ──
        if !self.is_child && self.use_backplate {
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
            if !self.is_child && i == 9 {
                let r = cce_ui::color::root_plate_corner_radius();
                let backplate_enabled = self.roster.get_value_string(30) == Some("true".to_string());
                let border_bevel = self.roster.get_value_string(34) == Some("true".to_string());
                let opacity_enabled = self.roster.get_value_string(11) == Some("true".to_string());
                let transparency_val = if opacity_enabled { self.roster.value(12) as f32 / 100.0 } else { 1.0 };
                let bg_color = if backplate_enabled {
                    let mut col = cce_ui::color::page_low_color();
                    col[3] = transparency_val;
                    col
                } else {
                    [0.12, 0.12, 0.15, transparency_val]
                };

                let (wx, wy, ww, wh) = w.rect();
                if backplate_enabled {
                    if border_bevel {
                        let t = self.roster.value(35) as f32;
                        let r_inner = (r - t).max(0.0);
                        push_rounded(&mut pc, wx + t, wy + t, ww - 2.0 * t, wh - 2.0 * t, r_inner, bg_color, (true, true, true, true));
                    } else {
                        push_rounded(&mut pc, wx, wy, ww, wh, r, bg_color, (true, true, true, true));
                    }
                }

                let menubar_enabled = self.roster.get_value_string(31) == Some("true".to_string());
                if menubar_enabled {
                    let mut menu_color = cce_ui::color::root_plate_menubar_color();
                    menu_color[3] = transparency_val;
                    if backplate_enabled {
                        push_rounded(&mut pc, wx, wy, ww, 30.0, r, menu_color, (true, true, false, false));
                    } else {
                        push_rounded(&mut pc, wx, wy, ww, 30.0, 0.0, menu_color, (false, false, false, false));
                    }
                }

                let statusbar_enabled = self.roster.get_value_string(32) == Some("true".to_string());
                if statusbar_enabled {
                    let mut status_color = cce_ui::color::root_plate_statusbar_color();
                    status_color[3] = transparency_val;
                    if backplate_enabled {
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
                for (qx, qy, qw, qh, qr, qc, qcorners) in w.all_rounded_quads(&self.ui_context) {
                    push_rounded(&mut pc, qx, qy, qw, qh, qr, qc, qcorners);
                }
            }
        }

        // ── ControlPanel children (the dissolved panel's aggregate, app-side): the slots
        // are at real screen coordinates now; the legacy clamp-to-viewport, partial-clip
        // radius zeroing, and solid-border synthesis/inset rules apply verbatim, and the
        // scrollbar draws last, over the children, like the aggregate did. ──
        if !self.is_child && self.current_page == Page::Windows {
            let (panel_x, panel_y, panel_w, panel_h) = self.roster.gallery().control_panel.rect();
            let _ = (panel_x, panel_w);
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

        // ── Plain geometry (the legacy view() body) ──
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            if !self.is_child && i == 9 {
                let backplate_enabled = self.roster.get_value_string(30) == Some("true".to_string());
                let opacity_enabled = self.roster.get_value_string(11) == Some("true".to_string());
                let transparency_val = if opacity_enabled { self.roster.value(12) as f32 / 100.0 } else { 1.0 };
                let bg_color = if backplate_enabled {
                    let mut col = cce_ui::color::page_low_color();
                    col[3] = transparency_val;
                    col
                } else {
                    [0.12, 0.12, 0.15, transparency_val]
                };
                let (wx, wy, ww, wh) = w.rect();
                if !backplate_enabled {
                    pc.quad(Rect { x: wx, y: wy, width: ww, height: wh }, bg_color);
                }
            } else {
                for (qx, qy, qw, qh, qc) in w.all_quads(&self.ui_context) {
                    pc.quad(Rect { x: qx, y: qy, width: qw, height: qh }, qc);
                }
            }
        }

        // ── ControlPanel children, plain decorations (the dissolved aggregate_plain):
        // the child's own rounded background is skipped like the legacy rule, the rest
        // clamps to the panel viewport. ──
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

        // ── Text, as prims (the legacy rebuild_text_items assembly, uncached) ──
        let label_text = if self.is_child {
            match self.child_type.as_deref() {
                Some("Toplevel") => "Simulated Toplevel Window".to_string(),
                Some("Popup") => "Simulated Popup Window".to_string(),
                Some("LayerTop") => "Simulated Layer Shell (Top) Surface".to_string(),
                Some("LayerOverlay") => "Simulated Layer Shell (Overlay) Surface".to_string(),
                Some("LayerBackground") => "Simulated Layer Shell (Background) Surface".to_string(),
                Some(other) => format!("Simulated {} Window", other),
                None => "Simulated Window".to_string(),
            }
        } else {
            match self.current_page {
                Page::Controls => "Gallery - Controls".to_string(),
                Page::Windows => "Gallery - Windows".to_string(),
                Page::Xdg => "Gallery - XDG".to_string(),
            }
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
            // Text via the paint walk (not the legacy getters): same labels with the
            // widget's content font and clip bounds; the popover cull stays on the prim
            // coordinates. Any widget with a ui-tree parent (the page selector under the
            // status bar) is covered by that parent's descent — walking it here too would
            // emit its text twice. ControlPanel children clamp their bounds to the panel
            // viewport (the dissolved scrolled_child_labels rule, minus the shift — the
            // slots sit at real screen coordinates now).
            if self.ui_context.tree.parent_ptr(w.base().id()).is_some() {
                continue;
            }
            let cp_clip = if is_control_panel_child(i) {
                let (px_, py_, pw_, ph_) = self.roster.gallery().control_panel.rect();
                Some([px_, py_, px_ + pw_, py_ + ph_])
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


        // The status line the old text_areas() override appended.
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

            if !self.is_child && i == 9 {
                let border_enabled = self.roster.get_value_string(16) == Some("true".to_string());
                if border_enabled {
                    let backplate_enabled = self.roster.get_value_string(30) == Some("true".to_string());
                    let opacity_enabled = self.roster.get_value_string(11) == Some("true".to_string());
                    let transparency_val = if opacity_enabled { self.roster.value(12) as f32 / 100.0 } else { 1.0 };
                    let bg_color = if backplate_enabled {
                        let mut col = cce_ui::color::page_low_color();
                        col[3] = transparency_val;
                        col
                    } else {
                        [0.12, 0.12, 0.15, transparency_val]
                    };

                    let mut border_color = cce_ui::color::plate_border_color().unwrap_or([0.3, 0.3, 0.4, 1.0]);
                    border_color[3] = transparency_val;
                    let t = self.roster.value(35) as f32;
                    let border_bevel = self.roster.get_value_string(34) == Some("true".to_string());
                    let r = cce_ui::color::root_plate_corner_radius();
                    let (wx, wy, ww, wh) = w.rect();
                    
                    if backplate_enabled {
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
                                let bevel_depth = self.roster.value(36) as f32 / 100.0;
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
        let clear_alpha = if self.opacity || self.use_backplate {
            if self.use_backplate {
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
        self.cursor_x = lx;
        self.cursor_y = ly;

        let mut changed = false;
        // Routed dispatch (6bd shrink): the router owns the drag lifecycle — one
        // PointerMove per visible root forwards DragUpdate to a live drag target and
        // runs hover bookkeeping otherwise.
        let mv = cce_ui::widget::Event::PointerMove { x: lx, y: ly, local_x: lx, local_y: ly };
        {
            let is_child = self.is_child;
            let use_menubar = self.use_menubar;
            let use_statusbar = self.use_statusbar;
            let current_page = self.current_page;
            let is_visible = move |index: usize| -> bool {
                if is_child {
                    match index {
                        0..=2 => true,
                        3 => use_menubar,
                        4 => use_statusbar,
                        _ => false,
                    }
                } else {
                    match index {
                        0..=2 | 38 => true,
                        3..=8 | 22 | 23 | 28 | 29 | 39 | 41 | 42 | 44 => current_page == Page::Controls,
                        9..=21 | 30..=37 | 40 | 43 => current_page == Page::Windows,
                        24..=27 => current_page == Page::Xdg,
                        _ => false,
                    }
                }
            };
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
        self.cursor_x = lx;
        self.cursor_y = ly;

        let mut changed = false;
        let is_child = self.is_child;
        let use_menubar = self.use_menubar;
        let use_statusbar = self.use_statusbar;
        let current_page = self.current_page;
        let is_visible = move |index: usize| -> bool {
            if is_child {
                match index {
                    0..=2 => true,
                    3 => use_menubar,
                    4 => use_statusbar,
                    _ => false,
                }
            } else {
                match index {
                    0..=2 | 38 => true,
                    3..=8 | 22 | 23 | 28 | 29 | 39 | 41 | 42 | 44 => current_page == Page::Controls,
                    9..=21 | 30..=37 | 40 | 43 => current_page == Page::Windows,
                    24..=27 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };

        if state == ElementState::Pressed {
            let mut clicked_idx = None;
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
            if clicked_idx.is_none() {
                for i in (0..self.roster.len()).rev() {
                    if !is_visible(i) {
                        continue;
                    }
                    if is_control_panel_child(i) {
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
                // Routed press: the router records the drag target on a handled press
                // and synthesizes DragStart past its threshold (the old immediate
                // drag_begin). Legacy also armed drags whose press handler returned
                // false — preserve that by recording the target explicitly.
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
                    let mut page_changed = false;
                    let mut selected = 0;
                    if self.roster.take_click(38) {
                        selected = self.roster.value(38);
                        page_changed = true;
                    }
                    
                    if page_changed {
                        if selected == 0 {
                            self.current_page = Page::Controls;
                            self.update_status_text("Viewing Controls Page");
                        } else if selected == 1 {
                            self.current_page = Page::Windows;
                            self.update_status_text("Viewing Windows Page");
                        } else {
                            self.current_page = Page::Xdg;
                            self.update_status_text("Viewing XDG Page");
                        }
                        self.apply_layout();
                        changed = true;
                    } else if self.roster.take_click(44) {
                        self.layout_idx = self.roster.value(44) as usize;
                        self.positions = demo_positions(self.width, self.height, cce_ui::widget::PageSelector::sidebar_w(self.roster.gallery().paginator.as_any().downcast_ref::<cce_ui::widget::Paginator>().expect("slot 1 must be the Paginator")), self.layout_idx, self.roster.gallery());
                        self.apply_layout();
                        changed = true;
                    } else if self.current_page == Page::Controls {
                        if self.roster.take_click(3) {
                            // Does nothing
                        } else if self.roster.take_click(39) {
                            if let Ok(exe) = std::env::current_exe() {
                                let mut cmd = std::process::Command::new(exe);
                                cmd.arg("--child")
                                   .arg("--type")
                                   .arg("ColorRamp")
                                   .arg("--width")
                                   .arg("450")
                                   .arg("--height")
                                   .arg("350")
                                   .arg("--backplate");
                                let _ = cmd.spawn();
                            }
                        } else if self.roster.take_click(42) {
                            if let Ok(exe) = std::env::current_exe() {
                                let mut cmd = std::process::Command::new(exe);
                                cmd.arg("--child")
                                   .arg("--type")
                                   .arg("Ramp")
                                   .arg("--width")
                                   .arg("450")
                                   .arg("--height")
                                   .arg("350")
                                   .arg("--backplate");
                                let _ = cmd.spawn();
                            }
                        } else {
                            for i in [4, 5, 7, 8, 22, 23, 28, 29] {
                                if self.roster.take_click(i) {
                                    changed = true;
                                }
                            }
                        }
                    } else if self.current_page == Page::Windows {
                        let mut create_window = false;
                        
                        if self.roster.take_click(10) {
                            create_window = true;
                        } else if self.roster.take_click(40) {
                            if let Ok(exe) = std::env::current_exe() {
                                let mut cmd = std::process::Command::new(exe);
                                cmd.arg("--child")
                                   .arg("--type")
                                   .arg("Ramp")
                                   .arg("--width")
                                   .arg("450")
                                   .arg("--height")
                                   .arg("350")
                                   .arg("--backplate");
                                let _ = cmd.spawn();
                            }
                        } else {
                            let mut dropdown_clicked = false;
                            for i in [11, 12, 14, 15, 16, 17, 18, 30, 31, 32, 34, 35, 36] {
                                if self.roster.take_click(i) {
                                    changed = true;
                                    if i == 14 {
                                        dropdown_clicked = true;
                                    }
                                    if i == 36 {
                                        let depth = self.roster.value(36) as f32 / 100.0;
                                        if let Ok(mut registry) = cce_ui::layout::get_style_registry().write() {
                                            registry.set_float("bevel_depth", depth);
                                        }
                                    }
                                }
                            }
                            if dropdown_clicked {
                                let desc = match self.roster.value(14) {
                                    0 => "A standard application\n\
window (xdg_toplevel).\n\
It supports tiling (cascade,\n\
split, grid), fullscreening,\n\
dragging, and resizing.\n\n\
Testing layout:\n\
cascades in cce.",
                                    1 => "An anchored sub-surface\n\
popup (xdg_popup).\n\
Usually transient context\n\
menus, tooltips, or dropdown\n\
lists. Bypasses tiling.\n\n\
Testing layout:\n\
maps floating on top.",
                                    2 => "A high-level Layer Shell\n\
surface (anchored to top).\n\
Typically status bars, menus,\n\
or docks. Reserves panel\n\
space, limiting workspace.\n\n\
Testing layout:\n\
full width at top.",
                                    3 => "A high-priority Layer Shell\n\
surface (overlay layer).\n\
Used for screensavers, lock\n\
screens, or overlay HUDs.\n\
Sits above tiling windows.\n\n\
Testing layout:\n\
center floating window.",
                                    4 => "A bottom-level Layer Shell\n\
surface (background layer).\n\
Used for desktop wallpaper.\n\
Renders below all other\n\
client windows.\n\n\
Testing layout:\n\
full screen background.",
                                    _ => "",
                                };
                                self.roster.set_text(21, desc);
                            }
                        }
                        
                        if create_window {
                            let window_type = match self.roster.value(14) {
                                0 => "Toplevel",
                                1 => "Popup",
                                2 => "LayerTop",
                                3 => "LayerOverlay",
                                4 => "LayerBackground",
                                _ => "Toplevel",
                            };
                            let shape = match self.roster.value(15) {
                                0 => "rectangular",
                                1 => "circular",
                                _ => "rectangular",
                            };
                            let opacity_enabled = self.roster.get_value_string(11) == Some("true".to_string());
                            let transparency_pct = self.roster.value(12);
                            let transparency_val = transparency_pct as f32 / 100.0;
                            let border_enabled = self.roster.get_value_string(16) == Some("true".to_string());
                            let custom_width = self.roster.value(17);
                            let custom_height = self.roster.value(18);
     
                            self.update_status_text(&format!("Spawning simulated {} {} window...", shape, window_type));
                            if let Ok(exe) = std::env::current_exe() {
                                let mut cmd = std::process::Command::new(exe);
                                cmd.arg("--child")
                                   .arg("--type")
                                   .arg(window_type)
                                   .arg("--shape")
                                   .arg(shape);
                                let backplate_enabled = self.roster.get_value_string(30) == Some("true".to_string());
                                let menubar_enabled = self.roster.get_value_string(31) == Some("true".to_string());
                                let statusbar_enabled = self.roster.get_value_string(32) == Some("true".to_string());
                                if opacity_enabled {
                                    cmd.arg("--opacity")
                                       .arg("--transparency")
                                       .arg(transparency_val.to_string());
                                }
                                if !border_enabled {
                                    cmd.arg("--no-border");
                                } else {
                                    let border_width = self.roster.value(35) as f32;
                                    let border_bevel = self.roster.get_value_string(34) == Some("true".to_string());
                                    cmd.arg("--border-width")
                                       .arg(border_width.to_string());
                                    if border_bevel {
                                        cmd.arg("--border-bevel");
                                    }
                                }
                                if backplate_enabled {
                                    cmd.arg("--backplate");
                                }
                                if menubar_enabled {
                                    cmd.arg("--menubar");
                                }
                                if statusbar_enabled {
                                    cmd.arg("--statusbar");
                                }
                                 cmd.arg("--width")
                                    .arg(custom_width.to_string())
                                    .arg("--height")
                                    .arg(custom_height.to_string());
                                 let _ = cce_ui::process::spawn_tracked(cmd);
                             }
                            changed = true;
                        }
                    } else if self.current_page == Page::Xdg {
                        let save = if self.roster.take_click(26) {
                            Some(false)
                        } else if self.roster.take_click(27) {
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
        let is_child = self.is_child;
        let use_menubar = self.use_menubar;
        let use_statusbar = self.use_statusbar;
        let current_page = self.current_page;
        let is_visible = move |index: usize| -> bool {
            if is_child {
                match index {
                    0..=2 => true,
                    3 => use_menubar,
                    4 => use_statusbar,
                    _ => false,
                }
            } else {
                match index {
                    0..=2 | 38 => true,
                    3..=8 | 22 | 23 | 28 | 29 | 39 | 41 | 42 | 44 => current_page == Page::Controls,
                    9..=21 | 30..=37 | 40 | 43 => current_page == Page::Windows,
                    24..=27 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
        let ev = cce_ui::widget::Event::MouseWheel { delta: *delta, x: lx, y: ly, local_x: lx, local_y: ly };
        // Panel scroll first (legacy: the panel's scroll frame consumed the wheel before
        // its children saw it); a consumed wheel never reaches the panel's children.
        let mut cp_took_wheel = false;
        if !self.is_child && is_visible(43) {
            let root = self.roster.get_dyn(43).base().id();
            if self.ui_context.propagate_event(&ev, root) {
                changed = true;
                cp_took_wheel = true;
            }
        }
        let cp_wheel_ok = !self.is_child && self.cp_gate(lx, ly);
        for i in 0..self.roster.len() {
            if i == 43 {
                continue;
            }
            if !is_visible(i) {
                continue;
            }
            if is_control_panel_child(i) && (cp_took_wheel || !cp_wheel_ok) {
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
        let is_child = self.is_child;
        let use_menubar = self.use_menubar;
        let use_statusbar = self.use_statusbar;
        let current_page = self.current_page;
        let is_visible = move |index: usize| -> bool {
            if is_child {
                match index {
                    0..=2 => true,
                    3 => use_menubar,
                    4 => use_statusbar,
                    _ => false,
                }
            } else {
                match index {
                    0..=2 | 38 => true,
                    3..=8 | 22 | 23 | 28 | 29 | 39 | 41 | 42 | 44 => current_page == Page::Controls,
                    9..=21 | 30..=37 | 40 | 43 => current_page == Page::Windows,
                    24..=27 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
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
                // Panel children take keys only through the focused path above (the
                // legacy panel never forwarded keys).
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

        if event.state == ElementState::Pressed {
            let mut page_nav = false;
            let mut selected = 0;
            {
                // input.kdl `cce-gallery` domain
                let m = |name: &str, default: &str| {
                    cce_ui::widget::match_key_shortcut(event, &cce_ui::input::app_chord(name, default))
                };
                if m("page_1", "ctrl+1") {
                    selected = 0;
                    page_nav = true;
                } else if m("page_2", "ctrl+2") {
                    selected = 1;
                    page_nav = true;
                } else if m("page_3", "ctrl+3") {
                    selected = 2;
                    page_nav = true;
                }
            }
            if page_nav && !self.is_child {
                if selected == 0 {
                    self.current_page = Page::Controls;
                    self.update_status_text("Viewing Controls Page");
                } else if selected == 1 {
                    self.current_page = Page::Windows;
                    self.update_status_text("Viewing Windows Page");
                } else {
                    self.current_page = Page::Xdg;
                    self.update_status_text("Viewing XDG Page");
                }
                self.apply_layout();
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

    fn adjust_size(&self, w: f32, h: f32) -> (f32, f32) {
        if self.is_child && self.child_shape.as_deref() == Some("circular") {
            let side = w.min(h);
            (side, side)
        } else {
            (w, h)
        }
    }
}

fn demo_positions(sw: f32, sh: f32, sidebar_w: f32, layout_idx: usize, slots: &GallerySlots) -> Vec<(f32, f32, f32, f32)> {
    let base_x = sidebar_w + 20.0;
    let sph = cce_ui::layout::spinbox_height();
    let tgh = cce_ui::layout::toggle_height();
    let slh = cce_ui::layout::slider_height();
    let bh = cce_ui::layout::button_height();
    let ddh = cce_ui::layout::dropdown_height();
    
    let mut vec = vec![(0.0, 0.0, 0.0, 0.0); 45];
    
    // Common layout elements
    vec[0] = (0.0, 0.0, sw, 40.0); // 0 MenuBar
    vec[1] = (0.0, 40.0, sidebar_w, sh - 40.0 - 24.0); // 1 Page selector (Sidebar)
    vec[2] = (0.0, sh - 24.0, sw, 24.0); // 2 StatusBar
    
    // Page 1 (Windows)
    vec[9] = (base_x, 60.0, 400.0, 250.0); // 9 Panel (Window simulation area)
    vec[10] = (base_x + 420.0, 60.0, 140.0, bh); // 10 Button (Create Window)
    vec[11] = (base_x + 420.0, 160.0, 140.0, tgh); // 11 Toggle (Opacity)
    vec[12] = (base_x + 420.0, 210.0, 140.0, slh); // 12 Slider
    vec[13] = (base_x + 420.0, 250.0, 140.0, 20.0); // 13 Label
    vec[14] = (base_x + 580.0, 60.0, 180.0, ddh); // 14 Dropdown (Window Type)
    vec[15] = (base_x + 580.0, 115.0, 180.0, ddh); // 15 Dropdown (Window Shape)
    vec[16] = (base_x + 580.0, 170.0, 120.0, tgh); // 16 Toggle (Enable)
    vec[17] = (base_x + 580.0, 210.0, 140.0, sph); // 17 Spinbox (Width)
    vec[18] = (base_x + 580.0, 255.0, 140.0, sph); // 18 Spinbox (Height)
    vec[19] = (base_x, 320.0, 190.0, 250.0); // 19 Plate
    vec[20] = (base_x + 10.0, 330.0, 170.0, 20.0); // 20 Label
    vec[21] = (base_x + 10.0, 360.0, 170.0, 180.0); // 21 Label
    
    // Page 2 (XDG)
    vec[24] = (base_x, 60.0, 450.0, 200.0); // 24 Panel
    vec[25] = (base_x + 20.0, 80.0, 410.0, 60.0); // 25 Label
    vec[26] = (base_x + 20.0, 160.0, 180.0, bh); // 26 Button
    vec[27] = (base_x + 220.0, 160.0, 180.0, bh); // 27 Button
    
    // Window Simulation options (visible when Page::Windows is active)
    vec[30] = (base_x + 420.0, 300.0, 140.0, tgh); // 30 Toggle: Backplate
    vec[31] = (base_x + 420.0, 340.0, 140.0, tgh); // 31 Toggle: MenuBar
    vec[32] = (base_x + 420.0, 380.0, 140.0, tgh); // 32 Toggle: StatusBar
    vec[33] = (base_x + 420.0, 420.0, 140.0, 20.0); // 33 SectionContainer: Border
    vec[34] = (base_x + 420.0, 450.0, 140.0, tgh); // 34 Toggle: Bevel
    vec[35] = (base_x + 420.0, 490.0, 140.0, sph); // 35 Spinbox: Border Width
    vec[36] = (base_x + 420.0, 535.0, 140.0, sph); // 36 Spinbox: Bevel Depth
    vec[37] = (base_x + 420.0, 270.0, 140.0, 20.0); // 37 SectionContainer: Window Elements
    vec[38] = (sw - 140.0, sh - 14.0 - ddh / 2.0, 120.0, ddh); // 38 Dropdown: Page selector
    vec[40] = (base_x + 420.0, 580.0, 140.0, bh); // 40 Button: Bevel Shape
    vec[43] = (sw - 270.0, 60.0, 250.0, sh - 100.0); // 43 ControlPanel
    
    // Dynamically position Page 0 (Controls) elements using the selected layout index
    let available_w = (sw - base_x - 290.0).max(300.0);
    
    struct DemoPacker {
        free_rects: Vec<(f32, f32, f32, f32)>,
        max_w: f32,
        max_h: f32,
        gap: f32,
    }

    impl DemoPacker {
        fn new(start_x: f32, start_y: f32, max_width: f32, max_height: f32, gap: f32) -> Self {
            Self {
                free_rects: vec![(start_x, start_y, max_width, max_height)],
                max_w: max_width,
                max_h: 0.0,
                gap,
            }
        }

        fn pack(&mut self, cw: f32, ch: f32) -> (f32, f32) {
            let cw_clamped = cw.min(self.max_w);
            
            let mut best_idx = None;
            let mut best_y = f32::MAX;
            let mut best_x = f32::MAX;

            for (idx, &(rx, ry, rw, rh)) in self.free_rects.iter().enumerate() {
                if rw >= cw_clamped && rh >= ch {
                    if ry < best_y || (ry == best_y && rx < best_x) {
                        best_y = ry;
                        best_x = rx;
                        best_idx = Some(idx);
                    }
                }
            }

            let chosen_idx = match best_idx {
                Some(idx) => idx,
                None => {
                    let new_y = self.max_h + self.gap;
                    let new_rect = (self.free_rects[0].0, new_y, self.max_w, 100000.0);
                    self.free_rects.push(new_rect);
                    self.free_rects.len() - 1
                }
            };

            let (fx, fy, fw, fh) = self.free_rects.remove(chosen_idx);
            let px = fx;
            let py = fy;

            let rx = fx + cw_clamped + self.gap;
            let rw = fw - cw_clamped - self.gap;
            let by = py + ch + self.gap;
            let bh = fh - ch - self.gap;

            let split_horizontally = rw * fh > fw * bh;

            if split_horizontally {
                if rw > 0.0 && fh > 0.0 {
                    self.free_rects.push((rx, py, rw, fh));
                }
                if cw_clamped > 0.0 && bh > 0.0 {
                    self.free_rects.push((fx, by, cw_clamped, bh));
                }
            } else {
                if rw > 0.0 && ch > 0.0 {
                    self.free_rects.push((rx, py, rw, ch));
                }
                if fw > 0.0 && bh > 0.0 {
                    self.free_rects.push((fx, by, fw, bh));
                }
            }

            self.max_h = self.max_h.max(py + ch);

            (px, py)
        }
    }

    let label_off = |idx: usize| -> f32 {
        cce_ui::widget::label_offset(slots.get_dyn(idx))
    };

    let mut items: Vec<(usize, f32, f32, f32)> = vec![
        // Button
        (3, 200.0, bh + label_off(3), bh),
        // Progress Bar
        (6, 415.0, cce_ui::layout::progressbar_height() + label_off(6), cce_ui::layout::progressbar_height()),
        // Inputs
        (44, 200.0, ddh + label_off(44), ddh),
        (4, 200.0, tgh + label_off(4), tgh),
        (5, 200.0, tgh + label_off(5), tgh),
        (7, 200.0, slh + label_off(7), slh),
        (8, 200.0, sph + label_off(8), sph),
        (28, 200.0, ddh + label_off(28), ddh),
        (22, 200.0, cce_ui::layout::rangeslider_height() + label_off(22), cce_ui::layout::rangeslider_height()),
        (23, 200.0, 100.0 + label_off(23), 100.0),
        (29, 200.0, 120.0 + label_off(29), 120.0),
        (39, 200.0, bh + label_off(39), bh),
        (42, 200.0, bh + label_off(42), bh),
        (41, 200.0, 150.0 + label_off(41), 150.0),
    ];
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    match layout_idx {
        0 => {
            // Vertical Layout
            let mut cur_y = 60.0;
            for (idx, _w_item, h_item_total, h_item_content) in items {
                vec[idx] = (base_x, cur_y, available_w, h_item_content);
                cur_y += h_item_total + 15.0;
            }
        }
        1 => {
            // Columns Layout
            let num_cols = 3;
            let col_w = (available_w - (num_cols - 1) as f32 * 15.0) / num_cols as f32;
            let mut col_y = vec![60.0; num_cols];
            for (i, (idx, _w_item, h_item_total, h_item_content)) in items.into_iter().enumerate() {
                let col = i % num_cols;
                let cx = base_x + col as f32 * (col_w + 15.0);
                let cy = col_y[col];
                vec[idx] = (cx, cy, col_w, h_item_content);
                col_y[col] += h_item_total + 15.0;
            }
        }
        2 | 3 => {
            // Grid Layout and Adaptive Grid Layout
            let num_cols = if layout_idx == 2 {
                3
            } else {
                let min_col_w = 200.0;
                (((available_w + 15.0) / (min_col_w + 15.0)).floor().max(1.0)) as usize
            };
            let col_w = (available_w - (num_cols - 1) as f32 * 15.0) / num_cols as f32;
            let mut col_y = vec![60.0; num_cols];
            for (idx, _w_item, h_item_total, h_item_content) in items {
                let mut shortest_col = 0;
                let mut min_h = col_y[0];
                for col in 1..num_cols {
                    if col_y[col] < min_h {
                        min_h = col_y[col];
                        shortest_col = col;
                    }
                }
                let cx = base_x + shortest_col as f32 * (col_w + 15.0);
                let cy = col_y[shortest_col];
                vec[idx] = (cx, cy, col_w, h_item_content);
                col_y[shortest_col] += h_item_total + 15.0;
            }
        }
        4 => {
            // Overlay Layout
            for (idx, _w_item, _h_item_total, h_item_content) in items {
                vec[idx] = (base_x, 60.0, available_w, h_item_content);
            }
        }
        5 => {
            // Flex Layout
            let mut cur_x = base_x;
            let mut cur_y = 60.0;
            let mut row_h = 0.0f32;
            for (idx, w_item, h_item_total, h_item_content) in items {
                let target_w = w_item.min(available_w);
                if cur_x + target_w > base_x + available_w && cur_x > base_x {
                    cur_x = base_x;
                    cur_y += row_h + 15.0;
                    row_h = 0.0;
                }
                vec[idx] = (cur_x, cur_y, target_w, h_item_content);
                cur_x += target_w + 15.0;
                row_h = row_h.max(h_item_total);
            }
        }
        6 => {
            // Splitter Layout
            let count = items.len();
            let total_h = (sh - 120.0).max(300.0);
            let single_h = (total_h - (count - 1) as f32 * 10.0) / count as f32;
            let mut cur_y = 60.0;
            for (idx, _w_item, _h_item_total, _h_item_content) in items {
                let content_h = (single_h - label_off(idx)).max(10.0);
                vec[idx] = (base_x, cur_y, available_w, content_h);
                cur_y += single_h + 10.0;
            }
        }
        7 => {
            // Radial Layout
            let cx = base_x + available_w / 2.0;
            let cy = 300.0;
            let radius = 180.0;
            let count = items.len();
            for (i, (idx, w_item, h_item_total, h_item_content)) in items.into_iter().enumerate() {
                let angle = (i as f32 / count as f32) * 2.0 * std::f32::consts::PI;
                let px = cx + radius * angle.cos() - w_item / 2.0;
                let py = cy + radius * angle.sin() - h_item_total / 2.0;
                vec[idx] = (px, py, w_item, h_item_content);
            }
        }
        8 => {
            // Circular Pane Layout
            let cx = base_x + available_w / 2.0;
            let cy = 300.0;
            let count = items.len();
            for (i, (idx, w_item, h_item_total, h_item_content)) in items.into_iter().enumerate() {
                let radius = 100.0 + (i as f32 * 12.0);
                let angle = (i as f32 / count as f32) * 2.0 * std::f32::consts::PI;
                let px = cx + radius * angle.cos() - w_item / 2.0;
                let py = cy + radius * angle.sin() - h_item_total / 2.0;
                vec[idx] = (px, py, w_item, h_item_content);
            }
        }
        9 => {
            // Mosaic Layout (Guillotine Packer)
            let max_h = (sh - 99.0).max(300.0);
            let mut packer = DemoPacker::new(base_x, 60.0, available_w, max_h, 15.0);
            for (idx, w_item, h_item_total, h_item_content) in items {
                let (px, py) = packer.pack(w_item, h_item_total);
                vec[idx] = (px, py, w_item.min(available_w), h_item_content);
            }
        }
        _ => {
            // Reverse Mosaic Layout
            let max_h = (sh - 99.0).max(300.0);
            let mut packer = DemoPacker::new(base_x, 60.0, available_w, max_h, 15.0);
            let mut temp = Vec::with_capacity(items.len());
            for &(idx, w_item, h_item_total, h_item_content) in &items {
                let (px, py) = packer.pack(w_item, h_item_total);
                temp.push((px, py, w_item, h_item_total, h_item_content, idx));
            }
            let mut x_min = f32::MAX;
            let mut x_max = f32::MIN;
            let mut y_min = f32::MAX;
            let mut y_max = f32::MIN;
            for &(px, py, pw, ph_total, _, _) in &temp {
                x_min = x_min.min(px);
                x_max = x_max.max(px + pw);
                y_min = y_min.min(py);
                y_max = y_max.max(py + ph_total);
            }
            let src_w = (x_max - x_min).max(1.0);
            let src_h = (y_max - y_min).max(1.0);
            let dst_w = available_w;
            let dst_h = (sh - 120.0).max(300.0);
            let scale_x = dst_w / src_w;
            let scale_y = dst_h / src_h;
            for (px, py, pw, _ph_total, ph_content, idx) in temp {
                let new_x = base_x + (px - x_min) * scale_x;
                let new_y = 60.0 + (py - y_min) * scale_y;
                let new_w = pw * scale_x;
                let new_h = ph_content * scale_y;
                vec[idx] = (new_x, new_y, new_w, new_h);
            }
        }
    }
    
    
    vec
}

/// The five `ChildSlots` rects for a child window of `sw`x`sh`: 0 background,
/// 1 main (editor or description), 2 Close, 3 and 4 the MenuBar / StatusBar of a
/// simulated window or the two aux Labels of a Ramp / ColorRamp editor.
fn child_positions(sw: f32, sh: f32, use_menubar: bool, use_statusbar: bool, child_type: Option<&str>) -> Vec<(f32, f32, f32, f32)> {
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
    if matches!(child_type, Some("Ramp") | Some("ColorRamp")) {
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
