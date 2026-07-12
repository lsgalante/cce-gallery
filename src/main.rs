use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Label, Paginator, Panel, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    Toggle, Element, Trackpad, hover_animation, TextBox, CornerRadii, MenuBar, 
    Ramp, RampKey, ColorRamp, MouseButton, ElementState, Key, NamedKey, KeyEvent, MouseScrollDelta
};
mod ti_widgets;
use ti_widgets::{Backplate, ControlPanel, Plate, SectionContainer};
use cce_ui::widget::Adapted;
use cce_ui::engine::{Vertex, quad_vertices, LogicalSize, LogicalPosition};
use wayland_client::QueueHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Controls,
    Windows,
    Xdg,
}


/// The gallery roster, concretely typed (Phase 6bb): 53 named slots replacing the erased
/// `Vec<Box<dyn Element>>`. The historical numeric indexes (positions vec, dispatch loops,
/// page layouts) keep addressing the same slots through `get_dyn`/`get_dyn_mut`.
pub struct GallerySlots {
    pub menu_bar: Adapted<MenuBar>,
    pub paginator: Adapted<Paginator>,
    pub status_bar: Adapted<StatusBar>,
    pub button_demo: Adapted<Button>,
    pub label4: Adapted<Label>,
    pub label5: Adapted<Label>,
    pub label6: Adapted<Label>,
    pub label7: Adapted<Label>,
    pub label8: Adapted<Label>,
    pub checkbox_demo: Adapted<Checkbox>,
    pub toggle_demo: Adapted<Toggle>,
    pub progress_demo: Adapted<ProgressBar>,
    pub slider_demo: Adapted<Slider>,
    pub spinbox_demo: Adapted<Spinbox>,
    pub panel_demo: Adapted<Panel>,
    pub create_window_btn: Adapted<Button>,
    pub tile_windows_btn: Adapted<Button>,
    pub opacity_toggle: Adapted<Toggle>,
    pub label18: Adapted<Label>,
    pub transparency_slider: Adapted<Slider>,
    pub transparency_label: Adapted<Label>,
    pub window_type_dd: Adapted<Dropdown>,
    pub window_shape_dd: Adapted<Dropdown>,
    pub enable_toggle: Adapted<Toggle>,
    pub label24: Adapted<Label>,
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

pub const GALLERY_COUNT: usize = 53;

impl GallerySlots {
    pub fn get_dyn(&self, idx: usize) -> &(dyn Element + 'static) {
        match idx {
            0 => &self.menu_bar,
            1 => &self.paginator,
            2 => &self.status_bar,
            3 => &self.button_demo,
            4 => &self.label4,
            5 => &self.label5,
            6 => &self.label6,
            7 => &self.label7,
            8 => &self.label8,
            9 => &self.checkbox_demo,
            10 => &self.toggle_demo,
            11 => &self.progress_demo,
            12 => &self.slider_demo,
            13 => &self.spinbox_demo,
            14 => &self.panel_demo,
            15 => &self.create_window_btn,
            16 => &self.tile_windows_btn,
            17 => &self.opacity_toggle,
            18 => &self.label18,
            19 => &self.transparency_slider,
            20 => &self.transparency_label,
            21 => &self.window_type_dd,
            22 => &self.window_shape_dd,
            23 => &self.enable_toggle,
            24 => &self.label24,
            25 => &self.width_spin,
            26 => &self.height_spin,
            27 => &self.surface_plate,
            28 => &self.surface_info_label,
            29 => &self.surface_desc_label,
            30 => &self.range_slider_demo,
            31 => &self.trackpad_demo,
            32 => &self.portal_panel,
            33 => &self.portal_label,
            34 => &self.open_dialog_btn,
            35 => &self.save_dialog_btn,
            36 => &self.textbox_demo,
            37 => &self.plate_demo,
            38 => &self.backplate_toggle,
            39 => &self.menubar_toggle,
            40 => &self.statusbar_toggle,
            41 => &self.border_section,
            42 => &self.bevel_toggle,
            43 => &self.border_width_spin,
            44 => &self.bevel_depth_spin,
            45 => &self.elements_section,
            46 => &self.page_selector,
            47 => &self.color_ramp_btn,
            48 => &self.bevel_shape_btn,
            49 => &self.bevel_ramp,
            50 => &self.ramp_btn,
            51 => &self.control_panel,
            52 => &self.layout_dd,
            _ => panic!("gallery slot index out of range: {idx}"),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn Element + 'static) {
        match idx {
            0 => &mut self.menu_bar,
            1 => &mut self.paginator,
            2 => &mut self.status_bar,
            3 => &mut self.button_demo,
            4 => &mut self.label4,
            5 => &mut self.label5,
            6 => &mut self.label6,
            7 => &mut self.label7,
            8 => &mut self.label8,
            9 => &mut self.checkbox_demo,
            10 => &mut self.toggle_demo,
            11 => &mut self.progress_demo,
            12 => &mut self.slider_demo,
            13 => &mut self.spinbox_demo,
            14 => &mut self.panel_demo,
            15 => &mut self.create_window_btn,
            16 => &mut self.tile_windows_btn,
            17 => &mut self.opacity_toggle,
            18 => &mut self.label18,
            19 => &mut self.transparency_slider,
            20 => &mut self.transparency_label,
            21 => &mut self.window_type_dd,
            22 => &mut self.window_shape_dd,
            23 => &mut self.enable_toggle,
            24 => &mut self.label24,
            25 => &mut self.width_spin,
            26 => &mut self.height_spin,
            27 => &mut self.surface_plate,
            28 => &mut self.surface_info_label,
            29 => &mut self.surface_desc_label,
            30 => &mut self.range_slider_demo,
            31 => &mut self.trackpad_demo,
            32 => &mut self.portal_panel,
            33 => &mut self.portal_label,
            34 => &mut self.open_dialog_btn,
            35 => &mut self.save_dialog_btn,
            36 => &mut self.textbox_demo,
            37 => &mut self.plate_demo,
            38 => &mut self.backplate_toggle,
            39 => &mut self.menubar_toggle,
            40 => &mut self.statusbar_toggle,
            41 => &mut self.border_section,
            42 => &mut self.bevel_toggle,
            43 => &mut self.border_width_spin,
            44 => &mut self.bevel_depth_spin,
            45 => &mut self.elements_section,
            46 => &mut self.page_selector,
            47 => &mut self.color_ramp_btn,
            48 => &mut self.bevel_shape_btn,
            49 => &mut self.bevel_ramp,
            50 => &mut self.ramp_btn,
            51 => &mut self.control_panel,
            52 => &mut self.layout_dd,
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
    pub fn get_dyn(&self, idx: usize) -> &(dyn Element + 'static) {
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

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn Element + 'static) {
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

    pub fn get_dyn(&self, idx: usize) -> &(dyn Element + 'static) {
        match self {
            Roster::Gallery(s) => s.get_dyn(idx),
            Roster::Child(s) => s.get_dyn(idx),
        }
    }

    pub fn get_dyn_mut(&mut self, idx: usize) -> &mut (dyn Element + 'static) {
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
}

struct State {
    roster: Roster,
    positions: Vec<(f32, f32, f32, f32)>,

    status_text: String,

    drag_widget: Option<usize>,
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
    matches!(i, 15..=26 | 38..=45 | 48)
}

fn open_file_dialog_portal(sender: calloop::channel::Sender<String>) {
    if let Some(path) = cce_ui::file_dialog::pick_file("Open File Dialog", &[]) {
        let _ = sender.send(format!("Selected: {}", path.display()));
    } else {
        let _ = sender.send("File dialog cancelled by user".to_string());
    }
}

fn save_file_dialog_portal(sender: calloop::channel::Sender<String>) {
    if let Some(path) = cce_ui::file_dialog::save_file("Save File Dialog", &[]) {
        let _ = sender.send(format!("Saved to: {}", path.display()));
    } else {
        let _ = sender.send("Save dialog cancelled by user".to_string());
    }
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
            0..=2 | 46 => true,
            3..=13 | 30 | 31 | 36 | 37 | 47 | 49 | 50 | 52 => self.current_page == Page::Controls,
            14..=29 | 38..=45 | 48 | 51 => self.current_page == Page::Windows,
            32..=35 => self.current_page == Page::Xdg,
            _ => false,
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
                let widget = self.roster.get_dyn_mut(i);
                if widget.is_dragging() {
                    continue;
                }
                
                if !self.is_child && i == 46 {
                    continue;
                }

                let is_cp_child = is_control_panel_child(i);
                if is_cp_child {
                    if !visible {
                        widget.set_rect(-1000.0, -1000.0, 0.0, 0.0);
                    }
                    continue;
                }
                
                if visible {
                    let (x, y, w, h) = pos;
                    let mut final_h = h;
                    if i != 0 && i != 1 && i != 2 && i != 51 {
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
            let (x, y, w, h) = self.positions[51];
            self.roster.get_dyn_mut(51).set_rect(x, y, w, h);

            let sb_rect = self.roster.get_dyn_mut(2).rect();
            let ddh = cce_ui::layout::dropdown_height();
            let pad_x = 10.0;
            let pad_y = (sb_rect.3 - ddh) / 2.0;
            let dw = 120.0;
            let dx = sb_rect.0 + sb_rect.2 - dw - pad_x;
            let dy = sb_rect.1 + pad_y;
            self.roster.get_dyn_mut(46).set_rect(dx, dy, dw, ddh);
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
                label4: Label::new("").with_font_size(12.0),
                label5: Label::new("").with_font_size(12.0),
                label6: Label::new("").with_font_size(12.0),
                label7: Label::new("").with_font_size(12.0),
                label8: Label::new("").with_font_size(12.0),
                checkbox_demo: Checkbox::new().with_label("Checkbox"),
                toggle_demo: Toggle::new().with_label("Toggle"),
                progress_demo: ProgressBar::new(0.43).with_label("ProgressBar"),
                slider_demo: Slider::new().with_label("Slider"),
                spinbox_demo: Spinbox::new(10, 1, 100, 5).with_label("Spinbox"),
                panel_demo: Panel::new(0.0, 0.0, 400.0, 250.0),
                create_window_btn: Button::new(0.0, 0.0, 140.0, 40.0).with_label("Create Window"),
                tile_windows_btn: Button::new(0.0, 0.0, 140.0, 40.0).with_label("Tile Windows"),
                opacity_toggle: Toggle::new().with_label("Opacity"),
                label18: Label::new("").with_font_size(12.0),
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
                label24: Label::new("").with_font_size(12.0),
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
                portal_panel: Panel::new(0.0, 0.0, 450.0, 200.0).with_label("XDG Desktop Portal FileChooser"),
                portal_label: Label::new("This page verifies the integration of the XDG Desktop Portal\nFile Chooser in the Clear environment.").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4]),
                open_dialog_btn: Button::new(0.0, 0.0, 180.0, 40.0).with_label("Open File Dialog"),
                save_dialog_btn: Button::new(0.0, 0.0, 180.0, 40.0).with_label("Save File Dialog"),
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
            child_positions(c_w, c_h, use_backplate, use_menubar, use_statusbar, child_type.as_deref())
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
            drag_widget: None,
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
            let child_ptrs: Vec<*mut (dyn Element + 'static)> = [15, 16, 21, 22, 17, 19, 20, 23, 25, 26, 38, 39, 40, 41, 42, 43, 44, 45, 48]
                .iter()
                .map(|&idx| state.roster.get_dyn_mut(idx).as_ptr_mut())
                .collect();
            let cp = state.roster.get_dyn_mut(51).as_any_mut().downcast_mut::<ControlPanel>().expect("widget 51 must be a ControlPanel");
            for ptr in child_ptrs {
                cp.add_child(ptr);
            }

            // Link page selector (46) under StatusBar (2) — the old set_parent + add_child
            // pair as the one tree link it always was (6bd batch 4).
            let statusbar_ptr = state.roster.get_dyn_mut(2).as_ptr_mut();
            let dropdown_ptr = state.roster.get_dyn_mut(46).as_ptr_mut();
            unsafe {
                cce_ui::widget::focus::link_parent_child(
                    &mut *statusbar_ptr,
                    &mut *dropdown_ptr,
                    &mut state.ui_context,
                );
            }
        } else {
            state.focused_widget = Some(1);
            let ramp = state.roster.get_dyn_mut(1).as_any_mut().downcast_mut::<Ramp>().expect("child ramp widget");
            let preset_ptr = ramp.preset_dropdown.as_ptr_mut();
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
            "Clear Test Interface - Diagnostics Dashboard".to_string()
        };
        
        let mut app_id = if self.is_child {
            if let Some(ref t) = self.child_type {
                format!("clear-test-child-{}", t.to_lowercase())
            } else {
                "clear-test-child".to_string()
            }
        } else {
            "cce-test-interface".to_string()
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
                    0..=2 | 46 => true,
                    3..=13 | 30 | 31 | 36 | 37 | 47 | 49 | 50 | 52 => current_page == Page::Controls,
                    14..=29 | 38..=45 | 48 | 51 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
            if is_visible(i) {
                if is_control_panel_child(i) {
                    continue;
                }
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
        if (self.width - size.width as f32).abs() > 0.001 || (self.height - size.height as f32).abs() > 0.001 || (self.scale - scale).abs() > 0.001 {
            self.width = size.width as f32;
            self.height = size.height as f32;
            self.physical_width = (size.width * scale as f32) as u32;
            self.physical_height = (size.height * scale as f32) as u32;
            self.scale = scale;
            cce_ui::scale::set_scale_factor(scale as f32);

            self.positions = if self.is_child {
                child_positions(self.width, self.height, self.use_backplate, self.use_menubar, self.use_statusbar, self.child_type.as_deref())
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
            let r = cce_ui::color::backplate_corner_radius();
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
            if !self.is_child && i == 14 {
                let r = cce_ui::color::backplate_corner_radius();
                let backplate_enabled = self.roster.get_dyn(38).get_value_string() == Some("true".to_string());
                let border_bevel = self.roster.get_dyn(42).get_value_string() == Some("true".to_string());
                let opacity_enabled = self.roster.get_dyn(17).get_value_string() == Some("true".to_string());
                let transparency_val = if opacity_enabled { self.roster.get_dyn(19).value() as f32 / 100.0 } else { 1.0 };
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
                        let t = self.roster.get_dyn(43).value() as f32;
                        let r_inner = (r - t).max(0.0);
                        push_rounded(&mut pc, wx + t, wy + t, ww - 2.0 * t, wh - 2.0 * t, r_inner, bg_color, (true, true, true, true));
                    } else {
                        push_rounded(&mut pc, wx, wy, ww, wh, r, bg_color, (true, true, true, true));
                    }
                }

                let menubar_enabled = self.roster.get_dyn(39).get_value_string() == Some("true".to_string());
                if menubar_enabled {
                    let mut menu_color = cce_ui::color::backplate_menubar_color();
                    menu_color[3] = transparency_val;
                    if backplate_enabled {
                        push_rounded(&mut pc, wx, wy, ww, 30.0, r, menu_color, (true, true, false, false));
                    } else {
                        push_rounded(&mut pc, wx, wy, ww, 30.0, 0.0, menu_color, (false, false, false, false));
                    }
                }

                let statusbar_enabled = self.roster.get_dyn(40).get_value_string() == Some("true".to_string());
                if statusbar_enabled {
                    let mut status_color = cce_ui::color::backplate_statusbar_color();
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

        // ── Plain geometry (the legacy view() body) ──
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            if !self.is_child && i == 14 {
                let backplate_enabled = self.roster.get_dyn(38).get_value_string() == Some("true".to_string());
                let opacity_enabled = self.roster.get_dyn(17).get_value_string() == Some("true".to_string());
                let transparency_val = if opacity_enabled { self.roster.get_dyn(19).value() as f32 / 100.0 } else { 1.0 };
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

        // ── Popovers: geometry then labels, in-frame, on top of everything ──
        let mut popover_pc = cce_ui::layout::PopoverCollector::new();
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn(i);
            if !self.is_widget_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            if w.popover_rect().is_some() {
                w.render_popover(&mut popover_pc);
            }
        }
        for &(qc, qx, qy, qw, qh) in &popover_pc.rects {
            pc.quad(Rect { x: qx, y: qy, width: qw, height: qh }, qc);
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
                Page::Controls => "Clear Test Interface - Controls".to_string(),
                Page::Windows => "Clear Test Interface - Windows".to_string(),
                Page::Xdg => "Clear Test Interface - XDG Portal".to_string(),
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
            if is_control_panel_child(i) {
                continue;
            }
            // Text via the paint walk (not the legacy getters): same labels with the
            // widget's content font and clip bounds; the popover cull stays on the prim
            // coordinates. ControlPanel children are covered by the panel's walk descent,
            // and any widget with a ui-tree parent (the page selector under the status
            // bar) is covered by that parent's descent — walking it here too would emit
            // its text twice.
            if w.parent(&self.ui_context).is_some() {
                continue;
            }
            let mut scratch = cce_ui::scene::paint::PaintCtx::new();
            cce_ui::scene::painter::append_widget_text(&self.ui_context, w, &mut scratch);
            for item in scratch.finish().items {
                if let cce_ui::scene::paint::Prim::Text { text, x, y, font_size, color, font, bounds, .. } = item.prim {
                    if in_any_popover(x, y) {
                        continue;
                    }
                    pc.text_with(text, x, y, font_size, color, font, bounds);
                }
            }
        }

        for (t, size, x, y, tc, font_opt, bounds) in popover_pc.texts {
            pc.text_with(
                t,
                x,
                y,
                size,
                [
                    (tc[0] * 255.0) as u8,
                    (tc[1] * 255.0) as u8,
                    (tc[2] * 255.0) as u8,
                ],
                font_opt,
                bounds,
            );
        }

        // The status line the old text_areas() override appended.
        if !self.is_child {
            let (_, status_font_size) = cce_ui::layout::statusbar_font_parsed();
            let status_size = if status_font_size > 0.0 { status_font_size } else { 12.0 };
            let scol = cce_ui::color::backplate_statusbar_text_color();
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

            if !self.is_child && i == 14 {
                let border_enabled = self.roster.get_dyn(23).get_value_string() == Some("true".to_string());
                if border_enabled {
                    let backplate_enabled = self.roster.get_dyn(38).get_value_string() == Some("true".to_string());
                    let opacity_enabled = self.roster.get_dyn(17).get_value_string() == Some("true".to_string());
                    let transparency_val = if opacity_enabled { self.roster.get_dyn(19).value() as f32 / 100.0 } else { 1.0 };
                    let bg_color = if backplate_enabled {
                        let mut col = cce_ui::color::page_low_color();
                        col[3] = transparency_val;
                        col
                    } else {
                        [0.12, 0.12, 0.15, transparency_val]
                    };

                    let mut border_color = cce_ui::color::plate_border_color().unwrap_or([0.3, 0.3, 0.4, 1.0]);
                    border_color[3] = transparency_val;
                    let t = self.roster.get_dyn(43).value() as f32;
                    let border_bevel = self.roster.get_dyn(42).get_value_string() == Some("true".to_string());
                    let r = cce_ui::color::backplate_corner_radius();
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
                                let bevel_depth = self.roster.get_dyn(44).value() as f32 / 100.0;
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
        if let Some(idx) = self.drag_widget {
            if self.roster.get_dyn_mut(idx).drag_update(lx, ly) {
                changed = true;
            }
        }
        if self.drag_widget.is_none() {
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
                        0..=2 | 46 => true,
                        3..=13 | 30 | 31 | 36 | 37 | 47 | 49 | 50 | 52 => current_page == Page::Controls,
                        14..=29 | 38..=45 | 48 | 51 => current_page == Page::Windows,
                        32..=35 => current_page == Page::Xdg,
                        _ => false,
                    }
                }
            };
            for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
                if !is_visible(i) {
                    continue;
                }
                if is_control_panel_child(i) {
                    continue;
                }
                if w.cursor_moved(lx, ly, &mut self.ui_context) {
                    changed = true;
                }
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
                    0..=2 | 46 => true,
                    3..=13 | 30 | 31 | 36 | 37 | 47 | 49 | 50 | 52 => current_page == Page::Controls,
                    14..=29 | 38..=45 | 48 | 51 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };

        if state == ElementState::Pressed {
            let mut clicked_idx = None;
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
            if button == MouseButton::Left {
                if let Some(old) = self.focused_widget {
                    if Some(old) != clicked_idx {
                        self.roster.get_dyn_mut(old).unfocus();
                        self.focused_widget = None;
                    }
                }
            }
            if let Some(i) = clicked_idx {
                if self.roster.get_dyn_mut(i).mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    changed = true;
                }
                if button == MouseButton::Left && self.roster.get_dyn_mut(i).draggable() {
                    self.roster.get_dyn_mut(i).drag_begin(lx, ly);
                    self.drag_widget = Some(i);
                }
                if button == MouseButton::Left {
                    self.roster.get_dyn_mut(i).focus();
                    self.focused_widget = Some(i);
                }
            }
        } else {
            if button == MouseButton::Left {
                if let Some(idx) = self.drag_widget {
                    self.roster.get_dyn_mut(idx).drag_end();
                    self.drag_widget = None;
                    changed = true;
                }
            }

            for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
                if !is_visible(i) {
                    continue;
                }
                if is_control_panel_child(i) {
                    continue;
                }
                if w.mouse_input(button, state, lx, ly, &mut self.ui_context) {
                    changed = true;
                }
            }

            if button == MouseButton::Left {
                if self.is_child {
                    if self.roster.get_dyn_mut(2).take_click() {
                        return Some("exit".to_string());
                    }
                } else {
                    let mut page_changed = false;
                    let mut selected = 0;
                    if self.roster.get_dyn_mut(46).take_click() {
                        selected = self.roster.get_dyn(46).value();
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
                    } else if self.roster.get_dyn_mut(52).take_click() {
                        self.layout_idx = self.roster.get_dyn(52).value() as usize;
                        self.positions = demo_positions(self.width, self.height, cce_ui::widget::PageSelector::sidebar_w(self.roster.gallery().paginator.as_any().downcast_ref::<cce_ui::widget::Paginator>().expect("slot 1 must be the Paginator")), self.layout_idx, self.roster.gallery());
                        self.apply_layout();
                        changed = true;
                    } else if self.current_page == Page::Controls {
                        if self.roster.get_dyn_mut(3).take_click() {
                            // Does nothing
                        } else if self.roster.get_dyn_mut(47).take_click() {
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
                        } else if self.roster.get_dyn_mut(50).take_click() {
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
                            for i in [9, 10, 12, 13, 30, 31, 36, 37] {
                                if self.roster.get_dyn_mut(i).take_click() {
                                    changed = true;
                                }
                            }
                        }
                    } else if self.current_page == Page::Windows {
                        let mut create_window = false;
                        let mut tile_windows = false;
                        
                        if self.roster.get_dyn_mut(15).take_click() {
                            create_window = true;
                        } else if self.roster.get_dyn_mut(16).take_click() {
                            tile_windows = true;
                        } else if self.roster.get_dyn_mut(48).take_click() {
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
                            for i in [17, 19, 21, 22, 23, 25, 26, 38, 39, 40, 42, 43, 44] {
                                if self.roster.get_dyn_mut(i).take_click() {
                                    changed = true;
                                    if i == 21 {
                                        dropdown_clicked = true;
                                    }
                                    if i == 44 {
                                        let depth = self.roster.get_dyn(44).value() as f32 / 100.0;
                                        if let Ok(mut registry) = cce_ui::layout::get_style_registry().write() {
                                            registry.set_float("bevel_depth", depth);
                                        }
                                    }
                                }
                            }
                            if dropdown_clicked {
                                let desc = match self.roster.get_dyn(21).value() {
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
                                self.roster.get_dyn_mut(29).set_text(desc);
                            }
                        }
                        
                        if create_window {
                            let window_type = match self.roster.get_dyn(21).value() {
                                0 => "Toplevel",
                                1 => "Popup",
                                2 => "LayerTop",
                                3 => "LayerOverlay",
                                4 => "LayerBackground",
                                _ => "Toplevel",
                            };
                            let shape = match self.roster.get_dyn(22).value() {
                                0 => "rectangular",
                                1 => "circular",
                                _ => "rectangular",
                            };
                            let opacity_enabled = self.roster.get_dyn(17).get_value_string() == Some("true".to_string());
                            let transparency_pct = self.roster.get_dyn(19).value();
                            let transparency_val = transparency_pct as f32 / 100.0;
                            let border_enabled = self.roster.get_dyn(23).get_value_string() == Some("true".to_string());
                            let custom_width = self.roster.get_dyn(25).value();
                            let custom_height = self.roster.get_dyn(26).value();
     
                            self.update_status_text(&format!("Spawning simulated {} {} window...", shape, window_type));
                            if let Ok(exe) = std::env::current_exe() {
                                let mut cmd = std::process::Command::new(exe);
                                cmd.arg("--child")
                                   .arg("--type")
                                   .arg(window_type)
                                   .arg("--shape")
                                   .arg(shape);
                                let backplate_enabled = self.roster.get_dyn(38).get_value_string() == Some("true".to_string());
                                let menubar_enabled = self.roster.get_dyn(39).get_value_string() == Some("true".to_string());
                                let statusbar_enabled = self.roster.get_dyn(40).get_value_string() == Some("true".to_string());
                                if opacity_enabled {
                                    cmd.arg("--opacity")
                                       .arg("--transparency")
                                       .arg(transparency_val.to_string());
                                }
                                if !border_enabled {
                                    cmd.arg("--no-border");
                                } else {
                                    let border_width = self.roster.get_dyn(43).value() as f32;
                                    let border_bevel = self.roster.get_dyn(42).get_value_string() == Some("true".to_string());
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
                        } else if tile_windows {
                            self.update_status_text("Window Action: Tile active client windows");
                            changed = true;
                        }
                    } else if self.current_page == Page::Xdg {
                        let mut open_file = false;
                        let mut save_file = false;
                        if self.roster.get_dyn_mut(34).take_click() {
                            open_file = true;
                        } else if self.roster.get_dyn_mut(35).take_click() {
                            save_file = true;
                        }
 
                        if open_file {
                            self.update_status_text("Opening Open File Dialog...");
                            let sender_clone = self.sender.clone();
                            std::thread::spawn(move || {
                                open_file_dialog_portal(sender_clone);
                            });
                            changed = true;
                        } else if save_file {
                            self.update_status_text("Opening Save File Dialog...");
                            let sender_clone = self.sender.clone();
                            std::thread::spawn(move || {
                                save_file_dialog_portal(sender_clone);
                            });
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
                    0..=2 | 46 => true,
                    3..=13 | 30 | 31 | 36 | 37 | 47 | 49 | 50 | 52 => current_page == Page::Controls,
                    14..=29 | 38..=45 | 48 | 51 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
        for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
            if !is_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            if w.mouse_wheel(delta, lx, ly, &mut self.ui_context) {
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
                    0..=2 | 46 => true,
                    3..=13 | 30 | 31 | 36 | 37 | 47 | 49 | 50 | 52 => current_page == Page::Controls,
                    14..=29 | 38..=45 | 48 | 51 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
        let mut handled = false;
        if let Some(focused) = self.focused_widget {
            if self.roster.get_dyn_mut(focused).keyboard_input(event, &mut self.ui_context) {
                changed = true;
                handled = true;
            }
        }

        if !handled {
            for i in 0..self.roster.len() {
            let w = self.roster.get_dyn_mut(i);
                if Some(i) == self.focused_widget {
                    continue;
                }
                if !is_visible(i) {
                    continue;
                }
                if is_control_panel_child(i) {
                    continue;
                }
                if w.keyboard_input(event, &mut self.ui_context) {
                    changed = true;
                }
            }
        }

        if event.state == ElementState::Pressed {
            let mut page_nav = false;
            let mut selected = 0;
            if event.ctrl {
                match &event.logical_key {
                    Key::Character(c) if c == "1" => {
                        selected = 0;
                        page_nav = true;
                    }
                    Key::Character(c) if c == "2" => {
                        selected = 1;
                        page_nav = true;
                    }
                    Key::Character(c) if c == "3" => {
                        selected = 2;
                        page_nav = true;
                    }
                    _ => {}
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
    
    let mut vec = vec![(0.0, 0.0, 0.0, 0.0); 53];
    
    // Common layout elements
    vec[0] = (0.0, 0.0, sw, 40.0); // 0 MenuBar
    vec[1] = (0.0, 40.0, sidebar_w, sh - 40.0 - 24.0); // 1 Page selector (Sidebar)
    vec[2] = (0.0, sh - 24.0, sw, 24.0); // 2 StatusBar
    
    // Page 1 (Windows)
    vec[14] = (base_x, 60.0, 400.0, 250.0); // 14 Panel (Window simulation area)
    vec[15] = (base_x + 420.0, 60.0, 140.0, bh); // 15 Button (Create Window)
    vec[16] = (base_x + 420.0, 110.0, 140.0, bh); // 16 Button (Tile Windows)
    vec[17] = (base_x + 420.0, 160.0, 140.0, tgh); // 17 Toggle (Opacity)
    vec[18] = (-1000.0, -1000.0, 0.0, 0.0); // 18
    vec[19] = (base_x + 420.0, 210.0, 140.0, slh); // 19 Slider
    vec[20] = (base_x + 420.0, 250.0, 140.0, 20.0); // 20 Label
    vec[21] = (base_x + 580.0, 60.0, 180.0, ddh); // 21 Dropdown (Window Type)
    vec[22] = (base_x + 580.0, 115.0, 180.0, ddh); // 22 Dropdown (Window Shape)
    vec[23] = (base_x + 580.0, 170.0, 120.0, tgh); // 23 Toggle (Enable)
    vec[24] = (-1000.0, -1000.0, 0.0, 0.0); // 24
    vec[25] = (base_x + 580.0, 210.0, 140.0, sph); // 25 Spinbox (Width)
    vec[26] = (base_x + 580.0, 255.0, 140.0, sph); // 26 Spinbox (Height)
    vec[27] = (base_x, 320.0, 190.0, 250.0); // 27 Plate
    vec[28] = (base_x + 10.0, 330.0, 170.0, 20.0); // 28 Label
    vec[29] = (base_x + 10.0, 360.0, 170.0, 180.0); // 29 Label
    
    // Page 2 (XDG FileChooser)
    vec[32] = (base_x, 60.0, 450.0, 200.0); // 32 Panel
    vec[33] = (base_x + 20.0, 80.0, 410.0, 60.0); // 33 Label
    vec[34] = (base_x + 20.0, 160.0, 180.0, bh); // 34 Button
    vec[35] = (base_x + 220.0, 160.0, 180.0, bh); // 35 Button
    
    // Window Simulation options (visible when Page::Windows is active)
    vec[38] = (base_x + 420.0, 300.0, 140.0, tgh); // 38 Toggle: Backplate
    vec[39] = (base_x + 420.0, 340.0, 140.0, tgh); // 39 Toggle: MenuBar
    vec[40] = (base_x + 420.0, 380.0, 140.0, tgh); // 40 Toggle: StatusBar
    vec[41] = (base_x + 420.0, 420.0, 140.0, 20.0); // 41 SectionContainer: Border
    vec[42] = (base_x + 420.0, 450.0, 140.0, tgh); // 42 Toggle: Bevel
    vec[43] = (base_x + 420.0, 490.0, 140.0, sph); // 43 Spinbox: Border Width
    vec[44] = (base_x + 420.0, 535.0, 140.0, sph); // 44 Spinbox: Bevel Depth
    vec[45] = (base_x + 420.0, 270.0, 140.0, 20.0); // 45 SectionContainer: Window Elements
    vec[46] = (sw - 140.0, sh - 14.0 - ddh / 2.0, 120.0, ddh); // 46 Dropdown: Page selector
    vec[48] = (base_x + 420.0, 580.0, 140.0, bh); // 48 Button: Bevel Shape
    vec[51] = (sw - 270.0, 60.0, 250.0, sh - 100.0); // 51 ControlPanel
    
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
        (11, 415.0, cce_ui::layout::progressbar_height() + label_off(11), cce_ui::layout::progressbar_height()),
        // Inputs
        (52, 200.0, ddh + label_off(52), ddh),
        (9, 200.0, tgh + label_off(9), tgh),
        (10, 200.0, tgh + label_off(10), tgh),
        (12, 200.0, slh + label_off(12), slh),
        (13, 200.0, sph + label_off(13), sph),
        (36, 200.0, ddh + label_off(36), ddh),
        (30, 200.0, cce_ui::layout::rangeslider_height() + label_off(30), cce_ui::layout::rangeslider_height()),
        (31, 200.0, 100.0 + label_off(31), 100.0),
        (37, 200.0, 120.0 + label_off(37), 120.0),
        (47, 200.0, bh + label_off(47), bh),
        (50, 200.0, bh + label_off(50), bh),
        (49, 200.0, 150.0 + label_off(49), 150.0),
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
    
    vec[6] = (-1000.0, -1000.0, 0.0, 0.0); // 6 is spacer Label, keep hidden/out of sight
    
    vec
}

fn child_positions(sw: f32, sh: f32, _use_backplate: bool, use_menubar: bool, use_statusbar: bool, child_type: Option<&str>) -> Vec<(f32, f32, f32, f32)> {
    let dy = if use_menubar { 40.0 } else { 0.0 };
    let dh = if use_statusbar { 24.0 } else { 0.0 };
    let mut vec = vec![
        (-1000.0, -1000.0, 0.0, 0.0), // 0
        (-1000.0, -1000.0, 0.0, 0.0), // 1
        (-1000.0, -1000.0, 0.0, 0.0), // 2
        
        // Page 0
        (-1000.0, -1000.0, 0.0, 0.0), // 3
        (-1000.0, -1000.0, 0.0, 0.0), // 4
        (-1000.0, -1000.0, 0.0, 0.0), // 5
        (-1000.0, -1000.0, 0.0, 0.0), // 6
        (-1000.0, -1000.0, 0.0, 0.0), // 7
        (-1000.0, -1000.0, 0.0, 0.0), // 8
        (-1000.0, -1000.0, 0.0, 0.0), // 9
        (-1000.0, -1000.0, 0.0, 0.0), // 10
        (-1000.0, -1000.0, 0.0, 0.0), // 11
        (-1000.0, -1000.0, 0.0, 0.0), // 12
        (-1000.0, -1000.0, 0.0, 0.0), // 13
        
        // Page 1
        (-1000.0, -1000.0, 0.0, 0.0), // 14
        (-1000.0, -1000.0, 0.0, 0.0), // 15
        (-1000.0, -1000.0, 0.0, 0.0), // 16
        (-1000.0, -1000.0, 0.0, 0.0), // 17
        (-1000.0, -1000.0, 0.0, 0.0), // 18
        (-1000.0, -1000.0, 0.0, 0.0), // 19
        (-1000.0, -1000.0, 0.0, 0.0), // 20
        (-1000.0, -1000.0, 0.0, 0.0), // 21
        (-1000.0, -1000.0, 0.0, 0.0), // 22
        (-1000.0, -1000.0, 0.0, 0.0), // 23
        (-1000.0, -1000.0, 0.0, 0.0), // 24
        (-1000.0, -1000.0, 0.0, 0.0), // 25
        (-1000.0, -1000.0, 0.0, 0.0), // 26
        (-1000.0, -1000.0, 0.0, 0.0), // 27
        (-1000.0, -1000.0, 0.0, 0.0), // 28
        (-1000.0, -1000.0, 0.0, 0.0), // 29
        (-1000.0, -1000.0, 0.0, 0.0), // 30
        (-1000.0, -1000.0, 0.0, 0.0), // 31
        
        // Page 2
        (-1000.0, -1000.0, 0.0, 0.0), // 32
        (-1000.0, -1000.0, 0.0, 0.0), // 33
        (-1000.0, -1000.0, 0.0, 0.0), // 34
        (-1000.0, -1000.0, 0.0, 0.0), // 35
        
        // New widgets
        (-1000.0, -1000.0, 0.0, 0.0), // 36
        (-1000.0, -1000.0, 0.0, 0.0), // 37
        (-1000.0, -1000.0, 0.0, 0.0), // 38
        (-1000.0, -1000.0, 0.0, 0.0), // 39
        (-1000.0, -1000.0, 0.0, 0.0), // 40
        (-1000.0, -1000.0, 0.0, 0.0), // 41
        (-1000.0, -1000.0, 0.0, 0.0), // 42
        (-1000.0, -1000.0, 0.0, 0.0), // 43
        (-1000.0, -1000.0, 0.0, 0.0), // 44
        (-1000.0, -1000.0, 0.0, 0.0), // 45
        (-1000.0, -1000.0, 0.0, 0.0), // 46
        (-1000.0, -1000.0, 0.0, 0.0), // 47
        (-1000.0, -1000.0, 0.0, 0.0), // 48
        (-1000.0, -1000.0, 0.0, 0.0), // 49
        (-1000.0, -1000.0, 0.0, 0.0), // 50
        (-1000.0, -1000.0, 0.0, 0.0), // 51
    ];

    if use_menubar {
        vec[3] = (0.0, 0.0, sw, 40.0);
    }
    if use_statusbar {
        vec[4] = (0.0, sh - 24.0, sw, 24.0);
    }

    let inner_h = sh - dy - dh;
    if child_type == Some("ColorRamp") {
        vec[0] = (0.0, dy, sw, inner_h);
        vec[1] = (20.0, dy + 20.0, sw - 40.0, inner_h - 90.0);
        vec[2] = ((sw - 100.0) / 2.0, dy + inner_h - 55.0, 100.0, 35.0);
        vec[3] = (20.0, dy + inner_h - 90.0, 200.0, 20.0);
        vec[4] = (sw - 220.0, dy + inner_h - 90.0, 200.0, 20.0);
    } else if child_type == Some("Ramp") {
        vec[0] = (0.0, dy, sw, inner_h);
        vec[1] = (20.0, dy + 20.0, sw - 40.0, inner_h - 90.0);
        vec[2] = ((sw - 100.0) / 2.0, dy + inner_h - 55.0, 100.0, 35.0);
        vec[3] = (20.0, dy + inner_h - 90.0, 200.0, 20.0);
        vec[4] = (sw - 220.0, dy + inner_h - 90.0, 200.0, 20.0);
    } else {
        vec[0] = (0.0, dy, sw, inner_h);
        vec[1] = (20.0, dy + 40.0, sw - 40.0, inner_h - 110.0);
        vec[2] = ((sw - 100.0) / 2.0, dy + inner_h - 60.0, 100.0, 35.0);
    }

    vec
}

fn main() {
    cce_ui::engine::run::<State>();
}
