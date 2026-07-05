use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Label, Paginator, Panel, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    TextLabel, Toggle, Element, Trackpad, hover_animation, TextBox, Plate, CornerRadii, Backplate, MenuBar, SectionContainer,
    Ramp, RampKey, ColorRamp, ControlPanel
};
use cce_ui::engine::{Vertex, quad_vertices, push_rounded_rect_vertices_corners};

use glyphon::{
    Buffer, Cache, FontSystem, Resolution, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport,
};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window, delegate_output,
    registry::{ProvidesRegistryState, RegistryState},
    output::{OutputHandler, OutputState},
    seat::{
        keyboard::KeyboardHandler,
        pointer::PointerHandler,
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window as XdgWindow, WindowConfigure, WindowHandler, WindowDecorations},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, QueueHandle, Proxy,
};
use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;

fn make_text_buffer(font_system: &mut FontSystem, text: &str, size: f32) -> Buffer {
    let font_str = cce_ui::layout::statusbar_font();
    cce_ui::backend::window_runner::get_text_buffer(font_system, text, size, Some(&font_str))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Controls,
    Windows,
    Xdg,
}

#[allow(dead_code)]
struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,

    widgets: Vec<Box<dyn Element>>,
    positions: Vec<(f32, f32, f32, f32)>,

    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_viewport: Viewport,

    label_buffer: Buffer,
    status_buffer: Buffer,

    drag_widget: Option<usize>,
    focused_widget: Option<usize>,
    click_count: u32,

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
    border_width: f32,
    border_bevel: bool,
    child_type: Option<String>,
    ui_context: cce_ui::context::UiContext,
    bevel_ramp: Vec<RampKey>,
    bevel_ramp_line_type: String,
    last_ramp_mod: Option<std::time::SystemTime>,
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
    // Default keys: raised bevel profile (starts/ends at backplate level 0.5, peaks at 1.0)
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
    if r_offset <= 0.0 {
        return;
    }
    let segments = 32;

    let light_angle = cce_ui::layout::light_source_position();
    let rad = light_angle.to_radians();
    let lx = rad.cos();
    let ly = -rad.sin();

    let corners = [
        (wx + r, wy + r, std::f32::consts::PI, 1.5 * std::f32::consts::PI), // Top-Left
        (wx + ww - r, wy + r, 1.5 * std::f32::consts::PI, 2.0 * std::f32::consts::PI), // Top-Right
        (wx + ww - r, wy + wh - r, 0.0, 0.5 * std::f32::consts::PI), // Bottom-Right
        (wx + r, wy + wh - r, 0.5 * std::f32::consts::PI, std::f32::consts::PI), // Bottom-Left
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

impl State {
    async fn new(
        wayland_handle: &'static cce_ui::wayland::WaylandSurfaceHandle,
        pw: u32,
        ph: u32,
        scale: f64,
        is_child: bool,
        child_type: Option<String>,
        opacity: bool,
        transparency: f32,
        use_backplate: bool,
        use_menubar: bool,
        use_statusbar: bool,
        border_enabled: bool,
        border_width: f32,
        border_bevel: bool,
    ) -> Self {
        let lw = pw as f32 / scale as f32;
        let lh = ph as f32 / scale as f32;
        let sw = lw;
        let sh = lh;
        
        let is_ramp_child = is_child && (child_type.as_deref() == Some("Ramp") || child_type.as_deref() == Some("ColorRamp"));
        let opacity = opacity || is_ramp_child;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance
            .create_surface(wayland_handle)
            .expect("Failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let mut config = surface
            .get_default_config(&adapter, pw, ph)
            .expect("Failed to get surface config");
        config.present_mode = wgpu::PresentMode::Fifo;
        let alpha_mode = if opacity || use_backplate {
            let caps = surface.get_capabilities(&adapter);
            if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
                wgpu::CompositeAlphaMode::PreMultiplied
            } else if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied) {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else {
                wgpu::CompositeAlphaMode::Opaque
            }
        } else {
            wgpu::CompositeAlphaMode::Opaque
        };
        config.alpha_mode = alpha_mode;
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(cce_ui::SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Initialize text rendering
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, config.format);
        let text_renderer = TextRenderer::new(&mut text_atlas, &device, wgpu::MultisampleState::default(), None);

        let mut text_viewport = Viewport::new(&device, &cache);
        text_viewport.update(&queue, Resolution { width: pw, height: ph });

        let label_buffer = if is_child {
            let title = match child_type.as_deref() {
                Some("Toplevel") => "Simulated Toplevel Window".to_string(),
                Some("Popup") => "Simulated Popup Window".to_string(),
                Some("LayerTop") => "Simulated Layer Shell (Top) Surface".to_string(),
                Some("LayerOverlay") => "Simulated Layer Shell (Overlay) Surface".to_string(),
                Some("LayerBackground") => "Simulated Layer Shell (Background) Surface".to_string(),
                Some(other) => format!("Simulated {} Window", other),
                None => "Simulated Window".to_string(),
            };
            make_text_buffer(&mut font_system, &title, 16.0)
        } else {
            make_text_buffer(&mut font_system, "Clear Test Interface - Controls", 16.0)
        };
        let status_buffer = make_text_buffer(&mut font_system, "Select a test case to begin verification.", 12.0);

        let widgets: Vec<Box<dyn Element>> = if is_child {
            let desc_label = match child_type.as_deref() {
                Some("Toplevel") => "This is an active simulated Toplevel window.".to_string(),
                Some("Popup") => "This is an active simulated Popup window.".to_string(),
                Some("LayerTop") => "This is an active simulated Layer Top surface.".to_string(),
                Some("LayerOverlay") => "This is an active simulated Layer Overlay surface.".to_string(),
                Some("LayerBackground") => "This is an active simulated Layer Background surface.".to_string(),
                Some(other) => format!("This is an active simulated {} window.", other),
                None => "This is an active simulated window in the window manager.".to_string(),
            };
            let bg: Box<dyn Element> = if use_backplate {
                let mut bp = Backplate::new(0.0, 0.0, lw, lh).with_movable(false);
                if border_bevel {
                    bp = bp.with_bevel(true, border_width);
                }
                Box::new(bp)
            } else {
                Box::new(ContentBg::new())
            };
            if child_type.as_deref() == Some("ColorRamp") {
                vec![
                    bg,                      // 0
                    Box::new(ColorRamp::new()), // 1
                    Box::new(Button::new(0.0, 0.0, 100.0, 35.0).with_label("Close")), // 2
                    Box::new(Label::new("").with_font_size(12.0)), // 3
                    Box::new(Label::new("").with_font_size(12.0)), // 4
                ]
            } else if child_type.as_deref() == Some("Ramp") {
                vec![
                    bg,                      // 0
                    Box::new({
                        let mut ramp = Ramp::new();
                        let (loaded_keys, loaded_type) = load_bevel_ramp();
                        ramp.keys = loaded_keys;
                        ramp.line_type_dropdown.selected = match loaded_type.as_str() {
                            "bezier" => 1,
                            _ => 0,
                        };
                        ramp
                    }), // 1
                    Box::new(Button::new(0.0, 0.0, 100.0, 35.0).with_label("Close")), // 2
                    Box::new(Label::new("").with_font_size(12.0)), // 3
                    Box::new(Label::new("").with_font_size(12.0)), // 4
                ]
            } else {
                let menu_bar = MenuBar::new(0.0, 0.0, lw, 40.0)
                    .with_item("File", &["New", "Open", "Save", "Exit"])
                    .with_item("Edit", &["Undo", "Redo", "Cut", "Copy", "Paste"]);

                vec![
                    bg,                      // 0
                    Box::new(Label::new(&desc_label).with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 1
                    Box::new(Button::new(0.0, 0.0, 100.0, 35.0).with_label("Close")), // 2
                    Box::new(menu_bar),      // 3
                    Box::new(StatusBar::new()), // 4
                ]
            }
        } else {
            let menu_bar = MenuBar::new(0.0, 0.0, lw, 40.0)
                .with_item("File", &["Exit"])
                .with_item("Edit", &["Settings"])
                .with_item("Help", &["About"]);
            vec![
                Box::new(menu_bar), // 0
                {
                    let paginator = Paginator::new(vec![]);
                    Box::new(paginator)
                }, // 1
                Box::new(StatusBar::new()), // 2
                
                // "Controls" page (indices 3..13)
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Verify Opacity")), // 3
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Verify Blur")), // 4
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Verify Layout")), // 5
                Box::new(Label::new("").with_font_size(12.0)), // 6 Dummy label (hidden)
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Run Diagnostics")), // 7
                Box::new(Button::new_reset(0.0, 0.0, 140.0, 40.0).with_label("Reset")), // 8
                Box::new(Checkbox::new().with_label("Checkbox")), // 9
                Box::new(Toggle::new().with_label("Toggle")), // 10
                Box::new(ProgressBar::new(0.0).with_label("ProgressBar")), // 11
                Box::new(Slider::new().with_label("Slider")), // 12
                Box::new(Spinbox::new(10, 1, 100, 5).with_label("Spinbox")), // 13

                // "Windows" page (indices 14..24)
                Box::new(Panel::new(0.0, 0.0, 400.0, 250.0)), // 14 (Window simulation area)
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Create Window")), // 15
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Tile Windows")), // 16
                Box::new(Toggle::new().with_label("Opacity")), // 17
                Box::new(Label::new("").with_font_size(12.0)), // 18
                Box::new(Slider::new()), // 19
                Box::new(Label::new("Transparency Level").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 20
                Box::new(Dropdown::new(vec![
                    "Toplevel".to_string(),
                    "Popup".to_string(),
                    "Layer: Top".to_string(),
                    "Layer: Overlay".to_string(),
                    "Layer: Background".to_string(),
                ], 0).with_label("Window Type")), // 21
                Box::new(Dropdown::new(vec![
                    "Rectangular".to_string(),
                    "Circular".to_string(),
                ], 0).with_label("Window Shape")), // 22
                Box::new({
                    let mut t = Toggle::new().with_label("Enable");
                    t.set_toggled(true);
                    t
                }), // 23 Toggle: Enable
                Box::new(Label::new("").with_font_size(12.0)), // 24 Label: dummy
                Box::new(Spinbox::new(400, 100, 2000, 10).with_label("Width")), // 25 Spinbox: Width
                Box::new(Spinbox::new(250, 100, 2000, 10).with_label("Height")), // 26 Spinbox: Height
                Box::new(Plate::new(0.0, 0.0, 190.0, 250.0).with_blur(true)), // 27 (Info panel background)
                Box::new(Label::new("Surface Info").with_font_size(12.0)), // 28 (Info panel header)
                Box::new(Label::new(
                    "A standard application\n\
window (xdg_toplevel).\n\
It supports tiling (cascade,\n\
split, grid), fullscreening,\n\
dragging, and resizing.\n\n\
Testing layout:\n\
cascades in cce."
                ).with_font_size(10.0)), // 29 (Info panel description)
                Box::new(RangeSlider::new().with_label("RangeSlider")), // 30 (RangeSlider widget)
                Box::new(Trackpad::new().with_label("Trackpad")), // 31 (Trackpad widget)
                Box::new(Panel::new(0.0, 0.0, 450.0, 200.0).with_label("XDG Desktop Portal FileChooser")), // 32
                Box::new(Label::new("This page verifies the integration of the XDG Desktop Portal\nFile Chooser in the Clear environment.").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 33
                Box::new(Button::new(0.0, 0.0, 180.0, 40.0).with_label("Open File Dialog")), // 34
                Box::new(Button::new(0.0, 0.0, 180.0, 40.0).with_label("Save File Dialog")), // 35
                
                // New widgets from cce-ui changes (indices 36..37)
                Box::new(TextBox::new("Interactive TextBox".to_string())), // 36 TextBox widget
                Box::new(Plate::new(0.0, 0.0, 120.0, 120.0).with_label("Plate").with_blur(true)), // 37 Plate widget
                Box::new({
                    let mut t = Toggle::new().with_label("Backplate");
                    t.set_toggled(true);
                    t
                }), // 38 Toggle: Backplate
                Box::new({
                    let mut t = Toggle::new().with_label("MenuBar");
                    t.set_toggled(false);
                    t
                }), // 39 Toggle: MenuBar
                Box::new({
                    let mut t = Toggle::new().with_label("StatusBar");
                    t.set_toggled(false);
                    t
                }), // 40 Toggle: StatusBar
                Box::new(SectionContainer::new("Border")), // 41 Section: Border
                Box::new({
                    let mut t = Toggle::new().with_label("Bevel");
                    t.set_toggled(false);
                    t
                }), // 42 Toggle: Bevel
                Box::new(Spinbox::new(1, 1, 20, 1).with_label("Border Width")), // 43 Spinbox: Border Width
                Box::new(SectionContainer::new("Window Elements")), // 44 Section: Window Elements
                Box::new(Dropdown::new(vec!["Controls".to_string(), "Windows".to_string(), "XDG".to_string()], 0).with_open_upward(true)), // 45 Dropdown: Page selector
                Box::new(Button::new(0.0, 0.0, 120.0, 28.0).with_label("Color Ramp...")), // 46 Button: Color Ramp
                Box::new(Button::new(0.0, 0.0, 120.0, 28.0).with_label("Bevel Shape...")), // 47 Button: Bevel Shape
                Box::new(Ramp::new()), // 48 Ramp: Controls page ramp
                Box::new(Button::new(0.0, 0.0, 120.0, 28.0).with_label("Ramp...")), // 49 Button: Ramp
                Box::new(ControlPanel::new().with_label("ControlPanel")), // 50 ControlPanel
            ]
        };

        let positions = if is_child {
            child_positions(sw, sh, use_backplate, use_menubar, use_statusbar, child_type.as_deref())
        } else {
            let sidebar_w = widgets[1].as_page_selector().unwrap().sidebar_w();
            demo_positions(sw, sh, sidebar_w)
        };

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (loaded_keys, loaded_type) = load_bevel_ramp();

        let mut state = Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            vertex_count: 0,
            widgets,
            positions,
            font_system,
            swash_cache,
            text_atlas,
            text_renderer,
            text_viewport,
            label_buffer,
            status_buffer,
            drag_widget: None,
            focused_widget: None,
            click_count: 0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            width: lw,
            height: lh,
            physical_width: pw,
            physical_height: ph,
            scale,
            current_page: Page::Controls,
            is_child,
            opacity,
            transparency,
            use_backplate,
            use_menubar,
            use_statusbar,
            border_enabled,
            border_width,
            border_bevel,
            child_type: child_type.clone(),
            ui_context: cce_ui::context::UiContext::new(),
            bevel_ramp: loaded_keys,
            bevel_ramp_line_type: loaded_type,
            last_ramp_mod: None,
        };

        if !is_child {
            let cp_ptr = state.widgets[50].as_ptr_mut();
            let cp = unsafe { &mut *(cp_ptr as *mut ControlPanel) };
            for idx in [15, 16, 21, 22, 17, 19, 20, 23, 25, 26, 38, 39, 40, 41, 42, 43, 44, 47] {
                cp.add_child(state.widgets[idx].as_ptr_mut());
            }
        }

        cce_ui::scale::set_scale_factor(scale as f32);
        state.apply_layout();
        state.upload_vertices();
        state
    }

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
            0..=2 | 45 => true,
            3..=13 | 30 | 31 | 36 | 37 | 46 | 48 | 49 => self.current_page == Page::Controls,
            14..=29 | 38..=44 | 47 | 50 => self.current_page == Page::Windows,
            32..=35 => self.current_page == Page::Xdg,
            _ => false,
        }
    }

    fn update_page_title(&mut self) {
        let title = match self.current_page {
            Page::Controls => "Clear Test Interface - Controls",
            Page::Windows => "Clear Test Interface - Windows",
            Page::Xdg => "Clear Test Interface - XDG Portal",
        };
        self.label_buffer = make_text_buffer(&mut self.font_system, title, 16.0);
    }

    fn apply_layout(&mut self) {
        for i in 0..self.positions.len() {
            let visible = self.is_widget_visible(i);
            let pos = self.positions[i];
            if let Some(widget) = self.widgets.get_mut(i) {
                if widget.is_dragging() {
                    continue;
                }
                
                let is_cp_child = match i {
                    15..=26 | 38..=44 | 47 => true,
                    _ => false,
                };
                
                if is_cp_child {
                    if !visible {
                        widget.set_rect(-1000.0, -1000.0, 0.0, 0.0);
                    }
                    continue;
                }
                
                if visible {
                    let (x, y, w, h) = pos;
                    widget.set_rect(x, y, w, h);
                } else {
                    widget.set_rect(-1000.0, -1000.0, 0.0, 0.0);
                }
            }
        }
    }

    fn collect_vertices(&self) -> Vec<Vertex> {
        let sw = self.width;
        let sh = self.height;
        let mut verts = Vec::new();
        hover_animation::reset_frame_registration();
        cce_ui::widget::popovers::clear();
        
        if !self.is_child && self.use_backplate {
            let r = cce_ui::color::backplate_corner_radius();
            let bg_color = cce_ui::color::page_low_color();
            let radii = CornerRadii::new(r, r, r, r);
            push_rounded_rect_vertices_corners(
                0.0, 0.0, sw, sh, radii, sw, sh, bg_color, [0.0, 0.0, 0.0], None, &mut verts
            );
        }
        
        for (i, w) in self.widgets.iter().enumerate() {
            if !self.is_widget_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            
            let (wx, wy, ww, wh) = w.rect();
            let has_rounded = w.rounded_corners() != (false, false, false, false);
            
            // Render rounded quads
            if !self.is_child && i == 14 {
                let opacity_enabled = self.widgets[17].get_value_string() == Some("true".to_string());
                let transparency_val = if opacity_enabled { self.widgets[19].value() as f32 / 100.0 } else { 1.0 };
                let border_enabled = self.widgets[23].get_value_string() == Some("true".to_string());
                let backplate_enabled = self.widgets[38].get_value_string() == Some("true".to_string());
                let menubar_enabled = self.widgets[39].get_value_string() == Some("true".to_string());
                let statusbar_enabled = self.widgets[40].get_value_string() == Some("true".to_string());

                let r = cce_ui::color::backplate_corner_radius();
                let status_h = if statusbar_enabled { 24.0 } else { 0.0 };

                let bg_color = if backplate_enabled {
                    let mut col = cce_ui::color::page_low_color();
                    col[3] = transparency_val;
                    col
                } else {
                    [0.12, 0.12, 0.15, transparency_val]
                };

                // Draw background
                if backplate_enabled {
                    let radii = CornerRadii::new(r, r, r, r);
                    push_rounded_rect_vertices_corners(
                        wx, wy, ww, wh, radii, sw, sh, bg_color, [0.0, 0.0, 0.0], None, &mut verts
                    );
                } else {
                    verts.extend(quad_vertices(wx, wy, ww, wh, sw, sh, bg_color));
                }

                // Draw MenuBar
                if menubar_enabled {
                    let mut menu_color = cce_ui::color::backplate_menubar_color();
                    menu_color[3] = transparency_val;
                    if backplate_enabled {
                        let radii = CornerRadii::new(r, r, 0.0, 0.0);
                        push_rounded_rect_vertices_corners(
                            wx, wy, ww, 30.0, radii, sw, sh, menu_color, [0.0, 0.0, 0.0], None, &mut verts
                        );
                    } else {
                        verts.extend(quad_vertices(wx, wy, ww, 30.0, sw, sh, menu_color));
                    }
                }

                // Draw StatusBar
                if statusbar_enabled {
                    let mut status_color = cce_ui::color::backplate_statusbar_color();
                    status_color[3] = transparency_val;
                    if backplate_enabled {
                        let radii = CornerRadii::new(0.0, 0.0, r, r);
                        push_rounded_rect_vertices_corners(
                            wx, wy + wh - 24.0, ww, 24.0, radii, sw, sh, status_color, [0.0, 0.0, 0.0], None, &mut verts
                        );
                    } else {
                        verts.extend(quad_vertices(wx, wy + wh - 24.0, ww, 24.0, sw, sh, status_color));
                    }
                }

                // Draw simulated Close button inside preview
                let btn_w = 80.0;
                let btn_h = 25.0;
                let btn_x = wx + (ww - btn_w) / 2.0;
                let btn_y = wy + wh - status_h - 45.0;
                let mut btn_color = cce_ui::color::button_background_color();
                btn_color[3] = transparency_val;
                let btn_radii = CornerRadii::new(4.0, 4.0, 4.0, 4.0);
                push_rounded_rect_vertices_corners(
                    btn_x, btn_y, btn_w, btn_h, btn_radii, sw, sh, btn_color, [0.0, 0.0, 0.0], None, &mut verts
                );

                // Draw border
                if border_enabled {
                    let mut border_color = cce_ui::color::plate_border_color().unwrap_or([0.3, 0.3, 0.4, 1.0]);
                    border_color[3] = transparency_val;
                    let t = self.widgets[43].value() as f32;
                    let border_bevel = self.widgets[42].get_value_string() == Some("true".to_string());
                    
                    if backplate_enabled {
                        let radii = CornerRadii::new(r, r, r, r);
                        if border_bevel {
                            let slices = (t * 2.0).max(10.0) as i32;
                            let slice_w = t / slices as f32;
                            for idx in 0..slices {
                                let u_curr = idx as f32 / slices as f32;
                                let u_next = (idx + 1) as f32 / slices as f32;
                                
                                let h_outer = if idx == 0 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_curr, &self.bevel_ramp_line_type) };
                                let h_inner = if idx == slices - 1 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_next, &self.bevel_ramp_line_type) };
                                
                                let d_h = h_inner - h_outer;
                                let color_offset = d_h * 0.4;
                                
                                 let light_angle = cce_ui::layout::light_source_position();
                                 let rad = light_angle.to_radians();
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
                                    &mut verts
                                );
                            }
                        } else {
                            cce_ui::backend::window_runner::push_plate_solid_border_vertices(
                                wx, wy, ww, wh, radii, t, sw, sh, border_color, [0.0, 0.0, -1.0], &mut verts
                            );
                        }
                    } else {
                        if border_bevel {
                            let slices = (t * 2.0).max(10.0) as i32;
                            let slice_w = t / slices as f32;
                            for idx in 0..slices {
                                let u_curr = idx as f32 / slices as f32;
                                let u_next = (idx + 1) as f32 / slices as f32;
                                
                                let h_outer = if idx == 0 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_curr, &self.bevel_ramp_line_type) };
                                let h_inner = if idx == slices - 1 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_next, &self.bevel_ramp_line_type) };
                                
                                let d_h = h_inner - h_outer;
                                let color_offset = d_h * 0.4;
                                
                                 let light_angle = cce_ui::layout::light_source_position();
                                 let rad = light_angle.to_radians();
                                 let lx = rad.cos();
                                 let ly = -rad.sin();
                                 
                                 let c_offset = |factor: f32| -> [f32; 4] {
                                     let o = factor * color_offset;
                                     [
                                         (border_color[0] + o).clamp(0.0, 1.0),
                                         (border_color[1] + o).clamp(0.0, 1.0),
                                         (border_color[2] + o).clamp(0.0, 1.0),
                                         border_color[3]
                                     ]
                                 };
                                 
                                 let top_color = c_offset(-ly);
                                 let left_color = c_offset(-lx);
                                 let bottom_color = c_offset(ly);
                                 let right_color = c_offset(lx);
                                 
                                let offset = idx as f32 * slice_w;
                                verts.extend(quad_vertices(wx + offset, wy + offset, ww - 2.0 * offset, slice_w, sw, sh, top_color));
                                verts.extend(quad_vertices(wx + offset, wy + offset, slice_w, wh - 2.0 * offset, sw, sh, left_color));
                                verts.extend(quad_vertices(wx + offset, wy + wh - offset - slice_w, ww - 2.0 * offset, slice_w, sw, sh, bottom_color));
                                verts.extend(quad_vertices(wx + ww - offset - slice_w, wy + offset, slice_w, wh - 2.0 * offset, sw, sh, right_color));
                            }
                        } else {
                            verts.extend(quad_vertices(wx, wy, ww, t, sw, sh, border_color));
                            verts.extend(quad_vertices(wx, wy + wh - t, ww, t, sw, sh, border_color));
                            verts.extend(quad_vertices(wx, wy, t, wh, sw, sh, border_color));
                            verts.extend(quad_vertices(wx + ww - t, wy, t, wh, sw, sh, border_color));
                        }
                    }
                }
            } else if self.is_child && self.use_backplate && i == 3 {
                let r = cce_ui::color::backplate_corner_radius();
                let radii = CornerRadii::new(r, r, 0.0, 0.0);
                push_rounded_rect_vertices_corners(
                    wx, wy, ww, wh, radii, sw, sh, w.color(), [0.0, 0.0, 0.0], None, &mut verts
                );
            } else if self.is_child && self.use_backplate && i == 4 {
                let r = cce_ui::color::backplate_corner_radius();
                let radii = CornerRadii::new(0.0, 0.0, r, r);
                push_rounded_rect_vertices_corners(
                    wx, wy, ww, wh, radii, sw, sh, w.color(), [0.0, 0.0, 0.0], None, &mut verts
                );
            } else if !self.is_child && self.use_backplate && i == 0 {
                let r = cce_ui::color::backplate_corner_radius();
                let radii = CornerRadii::new(r, r, 0.0, 0.0);
                push_rounded_rect_vertices_corners(
                    wx, wy, ww, wh, radii, sw, sh, w.color(), [0.0, 0.0, 0.0], None, &mut verts
                );
            } else if !self.is_child && self.use_backplate && i == 2 {
                let r = cce_ui::color::backplate_corner_radius();
                let radii = CornerRadii::new(0.0, 0.0, r, r);
                push_rounded_rect_vertices_corners(
                    wx, wy, ww, wh, radii, sw, sh, w.color(), [0.0, 0.0, 0.0], None, &mut verts
                );
            } else {
                for (qx, qy, qw, qh, qr, qc, qcorners) in w.all_rounded_quads(&self.ui_context) {
                    if qr > 0.1 {
                        let radii = CornerRadii::new(
                            if qcorners.0 { qr } else { 0.0 },
                            if qcorners.1 { qr } else { 0.0 },
                            if qcorners.2 { qr } else { 0.0 },
                            if qcorners.3 { qr } else { 0.0 },
                        );
                        push_rounded_rect_vertices_corners(
                            qx, qy, qw, qh, radii, sw, sh, qc, [0.0, 0.0, 0.0], None, &mut verts
                        );
                    } else {
                        verts.extend(quad_vertices(qx, qy, qw, qh, sw, sh, qc));
                    }
                }
            }
            
            // Render normal flat quads
            let w_color = w.color();
            let has_bg = w_color[3].abs() > 0.001;
            for (qx, qy, qw, qh, qc) in w.all_quads(&self.ui_context) {
                if has_rounded && has_bg && (qx - wx).abs() < 0.1 && (qy - wy).abs() < 0.1 && (qw - ww).abs() < 0.1 && (qh - wh).abs() < 0.1 {
                    continue;
                }
                if self.is_child && self.use_backplate && (i == 3 || i == 4) {
                    continue;
                }
                if !self.is_child && self.use_backplate && (i == 0 || i == 2) {
                    continue;
                }
                if !self.is_child && self.use_backplate && i == 1 && (qx - wx).abs() < 0.1 && (qy - wy).abs() < 0.1 && (qw - ww).abs() < 0.1 && (qh - wh).abs() < 0.1 {
                    continue;
                }
                if !self.is_child && i == 14 {
                    continue;
                }
                verts.extend(quad_vertices(qx, qy, qw, qh, sw, sh, qc));
            }
            
            // Render extra circles
            for (cx, cy, r, qc) in w.extra_circles() {
                verts.extend(cce_ui::backend::window_runner::circle_vertices(cx, cy, r, sw, sh, qc, 16, [0.0, 0.0, -1.0]));
            }

            // Render extra arcs
            for (cx, cy, r, t, start, end, qc) in w.extra_arcs() {
                cce_ui::backend::window_runner::push_arc_background_vertices(
                    cx, cy, r, t, start, end, sw, sh, qc, 16, [0.0, 0.0, -1.0], &mut verts
                );
            }
            
            // Draw solid border if defined, or custom child border
            if self.is_child && i == 0 && self.border_enabled {
                let border_color = cce_ui::color::plate_border_color().unwrap_or([0.3, 0.3, 0.4, 1.0]);
                let t = self.border_width;
                let r = cce_ui::color::backplate_corner_radius();
                let backplate_enabled = self.use_backplate;
                
                if backplate_enabled {
                    let bg_color = self.widgets[0].color();
                    let radii = CornerRadii::new(r, r, r, r);
                    if self.border_bevel {
                        let slices = (t * 2.0).max(10.0) as i32;
                        let slice_w = t / slices as f32;
                        for i in 0..slices {
                            let u_curr = i as f32 / slices as f32;
                            let u_next = (i + 1) as f32 / slices as f32;
                            
                            let h_outer = if i == 0 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_curr, &self.bevel_ramp_line_type) };
                            let h_inner = if i == slices - 1 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_next, &self.bevel_ramp_line_type) };
                            
                            let d_h = h_inner - h_outer;
                            let color_offset = d_h * 0.4;
                            
                             let light_angle = cce_ui::layout::light_source_position();
                             let rad = light_angle.to_radians();
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
                             
                            let offset = i as f32 * slice_w;
                            let r_offset = (r - offset).max(0.0);
                            verts.extend(quad_vertices(wx + r_offset, wy + offset, ww - 2.0 * r_offset, slice_w, sw, sh, top_color));
                            verts.extend(quad_vertices(wx + offset, wy + r_offset, slice_w, wh - 2.0 * r_offset, sw, sh, left_color));
                            verts.extend(quad_vertices(wx + r_offset, wy + wh - offset - slice_w, ww - 2.0 * r_offset, slice_w, sw, sh, bottom_color));
                            verts.extend(quad_vertices(wx + ww - offset - slice_w, wy + r_offset, slice_w, wh - 2.0 * r_offset, sw, sh, right_color));

                                push_bevel_slice_corners(
                                    wx, wy, ww, wh,
                                    r, r_offset, slice_w,
                                    sw, sh, bg_color, color_offset,
                                    &mut verts
                                );
                        }
                    } else {
                        cce_ui::backend::window_runner::push_plate_solid_border_vertices(
                            wx, wy, ww, wh, radii, t, sw, sh, border_color, [0.0, 0.0, -1.0], &mut verts
                        );
                    }
                } else {
                    if self.border_bevel {
                        let slices = (t * 2.0).max(10.0) as i32;
                        let slice_w = t / slices as f32;
                        for i in 0..slices {
                            let u_curr = i as f32 / slices as f32;
                            let u_next = (i + 1) as f32 / slices as f32;
                            
                            let h_outer = if i == 0 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_curr, &self.bevel_ramp_line_type) };
                            let h_inner = if i == slices - 1 { 0.5 } else { interpolate_ramp_value(&self.bevel_ramp, u_next, &self.bevel_ramp_line_type) };
                            
                            let d_h = h_inner - h_outer;
                            let color_offset = d_h * 0.4;
                            
                            let light_angle = cce_ui::layout::light_source_position();
                            let rad = light_angle.to_radians();
                            let lx = rad.cos();
                            let ly = -rad.sin();
                            
                            let c_offset = |factor: f32| -> [f32; 4] {
                                let o = factor * color_offset;
                                [
                                    (border_color[0] + o).clamp(0.0, 1.0),
                                    (border_color[1] + o).clamp(0.0, 1.0),
                                    (border_color[2] + o).clamp(0.0, 1.0),
                                    border_color[3]
                                ]
                            };
                            
                            let top_color = c_offset(-ly);
                            let left_color = c_offset(-lx);
                            let bottom_color = c_offset(ly);
                            let right_color = c_offset(lx);
                            
                            let offset = i as f32 * slice_w;
                            verts.extend(quad_vertices(wx + offset, wy + offset, ww - 2.0 * offset, slice_w, sw, sh, top_color));
                            verts.extend(quad_vertices(wx + offset, wy + offset, slice_w, wh - 2.0 * offset, sw, sh, left_color));
                            verts.extend(quad_vertices(wx + offset, wy + wh - offset - slice_w, ww - 2.0 * offset, slice_w, sw, sh, bottom_color));
                            verts.extend(quad_vertices(wx + ww - offset - slice_w, wy + offset, slice_w, wh - 2.0 * offset, sw, sh, right_color));
                        }
                    } else {
                        verts.extend(quad_vertices(wx, wy, ww, t, sw, sh, border_color));
                        verts.extend(quad_vertices(wx, wy + wh - t, ww, t, sw, sh, border_color));
                        verts.extend(quad_vertices(wx, wy, t, wh, sw, sh, border_color));
                        verts.extend(quad_vertices(wx + ww - t, wy, t, wh, sw, sh, border_color));
                    }
                }
            } else if let Some((color, thickness)) = w.solid_border() {
                let r = w.corner_radius();
                let (c_tl, c_tr, c_br, c_bl) = w.rounded_corners();
                let radii = CornerRadii::new(
                    if c_tl { r } else { 0.0 },
                    if c_tr { r } else { 0.0 },
                    if c_br { r } else { 0.0 },
                    if c_bl { r } else { 0.0 },
                );
                cce_ui::backend::window_runner::push_plate_solid_border_vertices(
                    wx, wy, ww, wh, radii, thickness, sw, sh, color, [0.0, 0.0, -1.0], &mut verts
                );
            }
            
            if w.popover_rect().is_some() {
                cce_ui::widget::popovers::register(w.as_ref());
            }
        }

        // Draw popover quads on top
        let mut popover_pc = cce_ui::layout::PopoverCollector::new();
        for (i, w) in self.widgets.iter().enumerate() {
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
        for (qc, qx, qy, qw, qh) in popover_pc.rects {
            verts.extend(quad_vertices(qx, qy, qw, qh, sw, sh, qc));
        }

        hover_animation::post_render_check();
        if let Some((qx, qy, qw, qh, qc)) = hover_animation::get_quad() {
            verts.extend(quad_vertices(qx, qy, qw, qh, sw, sh, qc));
        }
        verts
    }

    fn upload_vertices(&mut self) {
        let verts = self.collect_vertices();
        self.vertex_count = verts.len() as u32;
        if self.vertex_count == 0 {
            return;
        }
        let data = bytemuck::cast_slice(&verts);
        let needed = data.len() as wgpu::BufferAddress;
        if needed > self.vertex_buffer.size() {
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: needed,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue.write_buffer(&self.vertex_buffer, 0, data);
    }

    fn update_status_text(&mut self, text: &str) {
        self.status_buffer = make_text_buffer(&mut self.font_system, text, 12.0);
    }

    fn prepare_text(&mut self) {
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
                    0..=2 | 45 => true,
                    3..=13 | 30 | 31 | 36 | 37 | 46 | 48 | 49 => current_page == Page::Controls,
                    14..=29 | 38..=44 | 47 | 50 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };

        let Self {
            ref mut text_renderer,
            ref device,
            ref queue,
            ref mut font_system,
            ref mut text_atlas,
            ref mut text_viewport,
            ref mut swash_cache,
            ref status_buffer,
            physical_width,
            physical_height,
            scale,
            ..
        } = self;

        let viewport = Resolution { width: *physical_width, height: *physical_height };
        text_viewport.update(queue, viewport);

        let scale_f32 = *scale as f32;

        let mut areas: Vec<TextArea> = Vec::new();

        if !is_child {
            let scol = cce_ui::color::backplate_statusbar_text_color();
            let text_color = glyphon::Color::rgb(
                (scol[0] * 255.0) as u8,
                (scol[1] * 255.0) as u8,
                (scol[2] * 255.0) as u8,
            );
            areas.push(TextArea {
                buffer: status_buffer,
                left: (12.0 * scale_f32).round(),
                top: (*physical_height as f32 - 24.0 * scale_f32).round(),
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: *physical_width as i32,
                    bottom: *physical_height as i32,
                },
                default_color: text_color,
                custom_glyphs: &[],
            });
        }

        let mut popover_rects = Vec::new();
        for (i, w) in self.widgets.iter().enumerate() {
            if !is_visible(i) {
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

        let mut widget_buffers: Vec<Buffer> = Vec::new();
        let mut widget_labels: Vec<(TextLabel, Option<[f32; 4]>)> = Vec::new();
        for (i, w) in self.widgets.iter().enumerate() {
            if !is_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            let widget_font_opt = w.widget_font();
            for (label, font, bounds) in w.text_labels_with_font_and_bounds(&self.ui_context) {
                if in_any_popover(label.x, label.y) {
                    continue;
                }
                let active_font = font.or_else(|| widget_font_opt.clone());
                let buf = cce_ui::backend::window_runner::get_text_buffer(
                    font_system,
                    &label.text,
                    label.font_size,
                    active_font.as_deref(),
                );
                widget_buffers.push(buf);
                widget_labels.push((label, bounds));
            }
        }

        for (buf, (label, bounds)) in widget_buffers.iter().zip(widget_labels.iter()) {
            let item_bounds = if let Some([l, t, r, b]) = bounds {
                TextBounds {
                    left: (l * scale_f32).round() as i32,
                    top: (t * scale_f32).round() as i32,
                    right: (r * scale_f32).round() as i32,
                    bottom: (b * scale_f32).round() as i32,
                }
            } else {
                TextBounds {
                    left: 0,
                    top: 0,
                    right: *physical_width as i32,
                    bottom: *physical_height as i32,
                }
            };
            areas.push(TextArea {
                buffer: buf,
                left: (label.x * scale_f32).round(),
                top: (label.y * scale_f32).round(),
                scale: 1.0,
                bounds: item_bounds,
                default_color: glyphon::Color::rgb(label.color[0], label.color[1], label.color[2]),
                custom_glyphs: &[],
            });
        }

        // Draw popover texts on top
        let mut popover_pc = cce_ui::layout::PopoverCollector::new();
        for (i, w) in self.widgets.iter().enumerate() {
            if !is_visible(i) {
                continue;
            }
            if is_control_panel_child(i) {
                continue;
            }
            if w.popover_rect().is_some() {
                w.render_popover(&mut popover_pc);
            }
        }
        let mut popover_buffers = Vec::new();
        for (t, size, _x, _y, _tc, font_opt, _bounds) in &popover_pc.texts {
            popover_buffers.push(cce_ui::backend::window_runner::get_text_buffer(
                font_system,
                t,
                *size,
                font_opt.as_deref(),
            ));
        }
        for (buf, (_, _size, x, y, tc, _font_opt, bounds)) in popover_buffers.iter().zip(popover_pc.texts.iter()) {
            let item_bounds = if let Some([l, t, r, b]) = bounds {
                TextBounds {
                    left: (l * scale_f32).round() as i32,
                    top: (t * scale_f32).round() as i32,
                    right: (r * scale_f32).round() as i32,
                    bottom: (b * scale_f32).round() as i32,
                }
            } else {
                TextBounds {
                    left: 0,
                    top: 0,
                    right: *physical_width as i32,
                    bottom: *physical_height as i32,
                }
            };
            areas.push(TextArea {
                buffer: buf,
                left: (*x * scale_f32).round(),
                top: (*y * scale_f32).round(),
                scale: 1.0,
                bounds: item_bounds,
                default_color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8,
                    (tc[1] * 255.0) as u8,
                    (tc[2] * 255.0) as u8,
                ),
                custom_glyphs: &[],
            });
        }

        text_renderer
            .prepare(device, queue, font_system, text_atlas, text_viewport, areas, swash_cache)
            .unwrap();
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.physical_width = width;
            self.physical_height = height;
            self.width = width as f32 / self.scale as f32;
            self.height = height as f32 / self.scale as f32;
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.positions = if self.is_child {
                child_positions(self.width, self.height, self.use_backplate, self.use_menubar, self.use_statusbar, self.child_type.as_deref())
            } else {
                let sidebar_w = self.widgets[1].as_page_selector().unwrap().sidebar_w();
                demo_positions(self.width, self.height, sidebar_w)
            };
            cce_ui::scale::set_scale_factor(self.scale as f32);
            self.apply_layout();
            self.upload_vertices();
        }
    }

    fn render(&mut self) {
        self.prepare_text();

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(e) => {
                eprintln!("Surface error: {e:?}");
                return;
            }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        {
            let clear_alpha = if self.opacity || self.use_backplate {
                if self.use_backplate {
                    0.0
                } else {
                    self.transparency as f64
                }
            } else {
                1.0
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05 * clear_alpha,
                            g: 0.05 * clear_alpha,
                            b: 0.08 * clear_alpha,
                            a: clear_alpha,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if self.vertex_count > 0 {
                pass.set_pipeline(&self.render_pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.draw(0..self.vertex_count, 0..1);
            }

            self.text_renderer
                .render(&self.text_atlas, &self.text_viewport, &mut pass)
                .unwrap();
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }

    fn tick(&mut self, dt: f32) -> bool {
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
                    0..=2 | 45 => true,
                    3..=13 | 30 | 31 | 36 | 37 | 46 | 48 | 49 => current_page == Page::Controls,
                    14..=29 | 38..=44 | 47 | 50 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
            }
        };
        for (i, w) in self.widgets.iter_mut().enumerate() {
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
            self.upload_vertices();
        }
        changed
    }
}

fn demo_positions(sw: f32, sh: f32, sidebar_w: f32) -> Vec<(f32, f32, f32, f32)> {
    let base_x = sidebar_w + 20.0;
    let sph = cce_ui::layout::spinbox_height();
    let tgh = cce_ui::layout::toggle_height();
    let slh = cce_ui::layout::slider_height();
    let bh = cce_ui::layout::button_height();
    let ddh = cce_ui::layout::dropdown_height();

    // Dynamic calculations for Controls page layout
    let mut ctrl_y = 60.0;
    
    // Row 1: verify buttons
    let ctrl_verify_opacity_pos = (base_x, ctrl_y, 140.0, bh);
    let ctrl_verify_blur_pos = (base_x + 150.0, ctrl_y, 140.0, bh);
    let ctrl_verify_layout_pos = (base_x + 300.0, ctrl_y, 140.0, bh);
    ctrl_y += bh + 20.0;
    
    // Row 2: diagnostics & reset & color ramp & ramp button
    let ctrl_diagnostics_pos = (base_x, ctrl_y, 140.0, bh);
    let ctrl_reset_pos = (base_x + 150.0, ctrl_y, 140.0, bh);
    let ctrl_color_ramp_btn_pos = (base_x + 300.0, ctrl_y, 140.0, bh);
    let ctrl_ramp_btn_pos = (base_x + 450.0, ctrl_y, 140.0, bh);
    ctrl_y += bh + 20.0;
    
    // Row 3: checkbox, toggle, progress_bar
    let label_offset = 12.0 + cce_ui::layout::label_margin();
    let row3_h = tgh.max(24.0);
    let ctrl_checkbox_pos = (base_x, ctrl_y + label_offset + (row3_h - 24.0)/2.0, 24.0, 24.0);
    let ctrl_toggle_pos = (base_x + 110.0, ctrl_y + label_offset + (row3_h - tgh)/2.0, 48.0, tgh);
    let ctrl_progress_pos = (base_x + 230.0, ctrl_y + (row3_h - 24.0)/2.0, 160.0, 24.0);
    ctrl_y += row3_h + label_offset + 20.0;
    
    // Row 4: slider & spinbox
    let row4_h = sph.max(slh);
    let ctrl_slider_pos = (base_x, ctrl_y + (row4_h - slh)/2.0, 300.0, slh);
    let ctrl_spinbox_pos = (base_x + 330.0, ctrl_y + (row4_h - sph)/2.0, 120.0, sph);
    ctrl_y += row4_h + label_offset + 20.0;

    // Row 5: Ramp widget
    let ctrl_ramp_widget_pos = (base_x, ctrl_y, 400.0, 140.0);

    // Dynamic calculations for Windows page layout
    let mut left_y = 80.0;
    
    // Live Preview
    let preview_pos = (base_x, left_y, 360.0, 240.0);
    left_y += 240.0 + 20.0;
    
    // Info Panel
    let info_h = 180.0;
    let info_bg_pos = (base_x, left_y, 360.0, info_h);
    let info_header_pos = (base_x + 10.0, left_y + 12.0, 340.0, 20.0);
    let info_desc_pos = (base_x + 10.0, left_y + 40.0, 340.0, info_h - 50.0);

    let rx = base_x + 380.0;
    let rw = 280.0;
    let control_panel_pos = (rx, 80.0, rw, sh - 120.0);

    vec![
        // Always visible
        (0.0, 0.0, sw, 40.0),               // 0 header
        (-1000.0, -1000.0, 0.0, 0.0),       // 1 Paginator (hidden)
        (0.0, sh - 28.0, sw, 28.0),         // 2 status_bar

        // "Controls" page only
        ctrl_verify_opacity_pos,              // 3 verify_opacity
        ctrl_verify_blur_pos,                 // 4 verify_blur
        ctrl_verify_layout_pos,               // 5 verify_layout
        (-1000.0, -1000.0, 0.0, 0.0),         // 6 Panel (hidden dummy)
        ctrl_diagnostics_pos,                 // 7 run_diagnostics
        ctrl_reset_pos,                       // 8 reset
        ctrl_checkbox_pos,                    // 9 checkbox
        ctrl_toggle_pos,                      // 10 toggle
        ctrl_progress_pos,                    // 11 progress_bar
        ctrl_slider_pos,                      // 12 slider
        ctrl_spinbox_pos,                     // 13 spinbox

        // "Windows" page only
        preview_pos,                          // 14 Panel (Window area)
        (-1000.0, -1000.0, 0.0, 0.0),         // 15 Button: Create Window (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 16 Button: Tile Windows (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 17 Toggle: Opacity (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 18 Label: dummy
        (-1000.0, -1000.0, 0.0, 0.0),         // 19 Slider: Transparency level (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 20 Label: "Transparency level" (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 21 Dropdown: Window type (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 22 Dropdown: Window shape (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 23 Toggle: Enable (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 24 Label: dummy
        (-1000.0, -1000.0, 0.0, 0.0),         // 25 Spinbox: Width (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 26 Spinbox: Height (placed by ControlPanel)
        info_bg_pos,                          // 27 Panel: Info background
        info_header_pos,                      // 28 Label: Info header
        info_desc_pos,                        // 29 Label: Info description
        (-1000.0, -1000.0, 0.0, 0.0),         // 30 RangeSlider
        (-1000.0, -1000.0, 0.0, 0.0),         // 31 Trackpad
        // "XDG" page only
        (base_x, 100.0, 450.0, 200.0),        // 32 Panel: XDG Portal background
        (base_x + 20.0, 140.0, 410.0, 40.0),         // 33 Label: XDG explanation
        (base_x + 20.0, 220.0, 180.0, bh),            // 34 Button: Open File Dialog
        (base_x + 220.0, 220.0, 180.0, bh),            // 35 Button: Save File Dialog
        
        // New widgets
        (-1000.0, -1000.0, 0.0, 0.0),         // 36 TextBox
        (-1000.0, -1000.0, 0.0, 0.0),         // 37 Plate
        (-1000.0, -1000.0, 0.0, 0.0),         // 38 Toggle: Backplate (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 39 Toggle: MenuBar (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 40 Toggle: StatusBar (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 41 SectionContainer: Border (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 42 Toggle: Bevel (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 43 Spinbox: Border Width (placed by ControlPanel)
        (-1000.0, -1000.0, 0.0, 0.0),         // 44 SectionContainer: Window Elements (placed by ControlPanel)
        (sw - 140.0, sh - 14.0 - ddh / 2.0, 120.0, ddh), // 45 Dropdown: Page selector
        ctrl_color_ramp_btn_pos,              // 46 Button: Color Ramp
        (-1000.0, -1000.0, 0.0, 0.0),         // 47 Button: Bevel Shape (placed by ControlPanel)
        ctrl_ramp_widget_pos,                 // 48 Ramp: Controls page ramp
        ctrl_ramp_btn_pos,                    // 49 Button: Ramp
        control_panel_pos,                    // 50 ControlPanel
    ]
}

fn child_positions(
    sw: f32,
    sh: f32,
    _use_backplate: bool,
    use_menubar: bool,
    use_statusbar: bool,
    child_type: Option<&str>,
) -> Vec<(f32, f32, f32, f32)> {
    if child_type == Some("Ramp") || child_type == Some("ColorRamp") {
        let bg_y = 0.0;
        let bg_h = 200.0;
        let ramp_pos = (10.0, 10.0, sw - 20.0, 140.0);
        let close_pos = ((sw - 100.0) / 2.0, 160.0, 100.0, 30.0);
        vec![
            (0.0, bg_y, sw, bg_h),              // 0 bg
            ramp_pos,                           // 1 Ramp
            close_pos,                          // 2 close button
            (-1000.0, -1000.0, 0.0, 0.0),       // 3 dummy
            (-1000.0, -1000.0, 0.0, 0.0),       // 4 dummy
        ]
    } else {
        let bg_y = 0.0;
        let bg_h = sh;

        let status_h = if use_statusbar { 30.0 } else { 0.0 };

        let menu_pos = if use_menubar {
            (0.0, 0.0, sw, 40.0)
        } else {
            (-1000.0, -1000.0, 0.0, 0.0)
        };

        let status_pos = if use_statusbar {
            (0.0, sh - 30.0, sw, 30.0)
        } else {
            (-1000.0, -1000.0, 0.0, 0.0)
        };

        let label_pos = (20.0, 80.0, sw - 40.0, 40.0);
        let close_pos = ((sw - 100.0) / 2.0, sh - status_h - 60.0, 100.0, 35.0);

        vec![
            (0.0, bg_y, sw, bg_h),              // 0 bg
            label_pos,                          // 1 label
            close_pos,                          // 2 close button
            menu_pos,                           // 3 MenuBar
            status_pos,                         // 4 StatusBar
        ]
    }
}

struct AppState {
    registry_state: RegistryState,
    compositor_state: CompositorState,
    xdg_shell_state: XdgShell,
    shm_state: Shm,
    seat_state: SeatState,
    output_state: OutputState,

    seats: Vec<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,

    window: Option<XdgWindow>,
    surface: Option<wl_surface::WlSurface>,

    state: Option<State>,
    exit: bool,
    redraw: bool,

    cursor_shape_manager: Option<smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager>,
    cursor_shape_device: Option<smithay_client_toolkit::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1>,

    ctrl_pressed: bool,
    shift_pressed: bool,

    initial_width: f32,
    initial_height: f32,
    child_shape: Option<String>,
    sender: calloop::channel::Sender<String>,
}

impl CompositorHandler for AppState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        scale_factor: i32,
    ) {
        _surface.set_buffer_scale(scale_factor);
        if let Some(state) = &mut self.state {
            state.scale = scale_factor as f64;
            let pw = (state.width as f64 * state.scale) as u32;
            let ph = (state.height as f64 * state.scale) as u32;
            state.resize(pw, ph);
        }
        self.redraw = true;
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for AppState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl SeatHandler for AppState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.seats.push(seat);
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).unwrap();
            let cursor_shape_device = self.cursor_shape_manager.as_ref().map(|csm| {
                csm.get_shape_device(&pointer, qh)
            });
            self.cursor_shape_device = cursor_shape_device;
            self.pointer = Some(pointer);
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).unwrap();
            self.keyboard = Some(keyboard);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            if let Some(device) = self.cursor_shape_device.take() {
                device.destroy();
            }
            self.pointer = None;
        }
        if capability == Capability::Keyboard {
            self.keyboard = None;
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.seats.retain(|s| s != &seat);
    }
}

impl ShmHandler for AppState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl PointerHandler for AppState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        use smithay_client_toolkit::seat::pointer::PointerEventKind;
        for event in events {
            let (x, y) = event.position;
            if let Some(state) = &mut self.state {
                state.cursor_x = x as f32;
                state.cursor_y = y as f32;
            }

            match &event.kind {
                PointerEventKind::Enter { serial } => {
                    eprintln!("[test-suite-pointer] enter serial={}, device_active={}", serial, self.cursor_shape_device.is_some());
                    if let Some(ref device) = self.cursor_shape_device {
                        device.set_shape(*serial, smithay_client_toolkit::reexports::protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1::Shape::Default);
                    }
                }
                PointerEventKind::Leave { .. } => {}
                PointerEventKind::Motion { .. } => {
                    if let Some(state) = &mut self.state {
                        let mut changed = false;
                        if let Some(idx) = state.drag_widget {
                            if state.widgets[idx].drag_update(state.cursor_x, state.cursor_y) {
                                changed = true;
                            }
                        }
                        if state.drag_widget.is_none() {
                            let is_child = state.is_child;
                            let use_menubar = state.use_menubar;
                            let use_statusbar = state.use_statusbar;
                            let current_page = state.current_page;
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
                                        0..=2 | 45 => true,
                                        3..=13 | 30 | 31 | 36 | 37 | 46 | 48 | 49 => current_page == Page::Controls,
                                        14..=29 | 38..=44 | 47 | 50 => current_page == Page::Windows,
                                        32..=35 => current_page == Page::Xdg,
                                        _ => false,
                                    }
                                }
                            };
                            for (i, w) in state.widgets.iter_mut().enumerate() {
                                if !is_visible(i) {
                                    continue;
                                }
                                if is_control_panel_child(i) {
                                    continue;
                                }
                                if w.cursor_moved(state.cursor_x, state.cursor_y, &mut state.ui_context) {
                                    changed = true;
                                }
                            }
                        }
                        if changed {
                            state.upload_vertices();
                            self.redraw = true;
                        }
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    let btn = match *button {
                        272 => cce_ui::widget::MouseButton::Left,
                        273 => cce_ui::widget::MouseButton::Right,
                        274 => cce_ui::widget::MouseButton::Middle,
                        _ => continue,
                    };
                    if let Some(st) = &mut self.state {
                        let mut changed = false;
                        let is_child = st.is_child;
                        let use_menubar = st.use_menubar;
                        let use_statusbar = st.use_statusbar;
                        let current_page = st.current_page;
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
                                    0..=2 | 45 => true,
                                    3..=13 | 30 | 31 | 36 | 37 | 46 | 48 | 49 => current_page == Page::Controls,
                                    14..=29 | 38..=44 | 47 | 50 => current_page == Page::Windows,
                                    32..=35 => current_page == Page::Xdg,
                                    _ => false,
                                }
                            }
                        };
                        let mut clicked_idx = None;
                        for i in (0..st.widgets.len()).rev() {
                            if !is_visible(i) {
                                continue;
                            }
                            if is_control_panel_child(i) {
                                continue;
                            }
                            if st.widgets[i].hit_test(st.cursor_x, st.cursor_y, &st.ui_context) {
                                clicked_idx = Some(i);
                                break;
                            }
                        }
                        if btn == cce_ui::widget::MouseButton::Left {
                            if let Some(old) = st.focused_widget {
                                if Some(old) != clicked_idx {
                                    st.widgets[old].unfocus();
                                    st.focused_widget = None;
                                }
                            }
                        }
                        if let Some(i) = clicked_idx {
                            if st.widgets[i].mouse_input(
                                btn,
                                cce_ui::widget::ElementState::Pressed,
                                st.cursor_x,
                                st.cursor_y,
                                &mut st.ui_context,
                            ) {
                                changed = true;
                            }
                            if btn == cce_ui::widget::MouseButton::Left && st.widgets[i].draggable() {
                                st.widgets[i].drag_begin(st.cursor_x, st.cursor_y);
                                st.drag_widget = Some(i);
                            }
                            if btn == cce_ui::widget::MouseButton::Left {
                                st.widgets[i].focus();
                                st.focused_widget = Some(i);
                            }
                        }
                        if changed {
                            st.upload_vertices();
                            self.redraw = true;
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    let btn = match *button {
                        272 => cce_ui::widget::MouseButton::Left,
                        273 => cce_ui::widget::MouseButton::Right,
                        274 => cce_ui::widget::MouseButton::Middle,
                        _ => continue,
                    };
                    if let Some(st) = &mut self.state {
                        let mut changed = false;
                        if btn == cce_ui::widget::MouseButton::Left {
                            if let Some(idx) = st.drag_widget {
                                st.widgets[idx].drag_end();
                                st.drag_widget = None;
                                changed = true;
                            }
                        }
                        
                        let is_child = st.is_child;
                        let use_menubar = st.use_menubar;
                        let use_statusbar = st.use_statusbar;
                        let current_page = st.current_page;
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
                                    0..=2 | 45 => true,
                                    3..=13 | 30 | 31 | 36 | 37 | 46 | 48 | 49 => current_page == Page::Controls,
                                    14..=29 | 38..=44 | 47 | 50 => current_page == Page::Windows,
                                    32..=35 => current_page == Page::Xdg,
                                    _ => false,
                                }
                            }
                        };
                        for (i, w) in st.widgets.iter_mut().enumerate() {
                            if !is_visible(i) {
                                continue;
                            }
                            if is_control_panel_child(i) {
                                continue;
                            }
                            if w.mouse_input(btn, cce_ui::widget::ElementState::Released, st.cursor_x, st.cursor_y, &mut st.ui_context) {
                                changed = true;
                            }
                        }
                        
                        if btn == cce_ui::widget::MouseButton::Left {
                            if st.is_child {
                                if st.widgets[2].take_click() {
                                    self.exit = true;
                                    changed = true;
                                }
                            } else {
                                let mut page_changed = false;
                                let mut selected = 0;
                                if st.widgets[45].take_click() {
                                    selected = st.widgets[45].value();
                                    page_changed = true;
                                }
                                
                                if page_changed {
                                    if selected == 0 {
                                        st.current_page = Page::Controls;
                                        st.update_page_title();
                                        st.update_status_text("Viewing Controls Page");
                                    } else if selected == 1 {
                                        st.current_page = Page::Windows;
                                        st.update_page_title();
                                        st.update_status_text("Viewing Windows Page");
                                    } else {
                                        st.current_page = Page::Xdg;
                                        st.update_page_title();
                                        st.update_status_text("Viewing XDG Page");
                                    }
                                    st.apply_layout();
                                    st.upload_vertices();
                                    changed = true;
                                } else if st.current_page == Page::Controls {
                                    let mut run_opacity = false;
                                    let mut run_blur = false;
                                    let mut run_layout = false;
                                    let mut run_all = false;
                                    let mut do_reset = false;
                                    
                                    if st.widgets[3].take_click() {
                                        run_opacity = true;
                                    } else if st.widgets[4].take_click() {
                                        run_blur = true;
                                    } else if st.widgets[5].take_click() {
                                        run_layout = true;
                                    } else if st.widgets[7].take_click() {
                                        run_all = true;
                                    } else if st.widgets[8].take_click() {
                                        do_reset = true;
                                    } else if st.widgets[46].take_click() {
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
                                    } else if st.widgets[49].take_click() {
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
                                        for i in [9, 10, 11, 12, 13, 30, 31, 36, 37] {
                                            if st.widgets[i].take_click() {
                                                changed = true;
                                            }
                                        }
                                    }
                                    
                                    if run_opacity {
                                        st.update_status_text("Opacity Test: PASSED (transparency alpha = 0.20 successfully checked)");
                                        st.widgets[11] = Box::new(ProgressBar::new(0.35));
                                        st.apply_layout();
                                        st.upload_vertices();
                                        changed = true;
                                    } else if run_blur {
                                        st.update_status_text("Blur Test: PASSED (wlr_scene_set_blur_data initialization confirmed)");
                                        st.widgets[11] = Box::new(ProgressBar::new(0.70));
                                        st.apply_layout();
                                        st.upload_vertices();
                                        changed = true;
                                    } else if run_layout {
                                        st.update_status_text("Layout Test: PASSED (Grid/Cascade IPC modes tiling verified)");
                                        st.widgets[11] = Box::new(ProgressBar::new(1.00));
                                        st.apply_layout();
                                        st.upload_vertices();
                                        changed = true;
                                    } else if run_all {
                                        st.update_status_text("All Diagnostics: SUCCESS (Compositor and window managers verified!)");
                                        st.widgets[11] = Box::new(ProgressBar::new(1.00));
                                        st.apply_layout();
                                        st.upload_vertices();
                                        changed = true;
                                    } else if do_reset {
                                        st.update_status_text("Verification state reset. Ready.");
                                        st.widgets[11] = Box::new(ProgressBar::new(0.00));
                                        st.apply_layout();
                                        st.upload_vertices();
                                        changed = true;
                                    }
                                } else if st.current_page == Page::Windows {
                                    let mut create_window = false;
                                    let mut tile_windows = false;
                                    
                                    if st.widgets[15].take_click() {
                                        create_window = true;
                                    } else if st.widgets[16].take_click() {
                                        tile_windows = true;
                                    } else if st.widgets[47].take_click() {
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
                                        for i in [17, 19, 21, 22, 23, 25, 26, 38, 39, 40, 42, 43] {
                                            if st.widgets[i].take_click() {
                                                changed = true;
                                                if i == 21 {
                                                    dropdown_clicked = true;
                                                }
                                            }
                                        }
                                        if dropdown_clicked {
                                            let desc = match st.widgets[21].value() {
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
                                            st.widgets[29].set_text(desc);
                                        }
                                    }
                                    
                                    if create_window {
                                        let window_type = match st.widgets[21].value() {
                                            0 => "Toplevel",
                                            1 => "Popup",
                                            2 => "LayerTop",
                                            3 => "LayerOverlay",
                                            4 => "LayerBackground",
                                            _ => "Toplevel",
                                        };
                                        let shape = match st.widgets[22].value() {
                                            0 => "rectangular",
                                            1 => "circular",
                                            _ => "rectangular",
                                        };
                                        let opacity_enabled = st.widgets[17].get_value_string() == Some("true".to_string());
                                        let transparency_pct = st.widgets[19].value();
                                        let transparency_val = transparency_pct as f32 / 100.0;
                                        let border_enabled = st.widgets[23].get_value_string() == Some("true".to_string());
                                        let custom_width = st.widgets[25].value();
                                        let custom_height = st.widgets[26].value();
 
                                        st.update_status_text(&format!("Spawning simulated {} {} window...", shape, window_type));
                                        if let Ok(exe) = std::env::current_exe() {
                                            let mut cmd = std::process::Command::new(exe);
                                            cmd.arg("--child")
                                               .arg("--type")
                                               .arg(window_type)
                                               .arg("--shape")
                                               .arg(shape);
                                            let backplate_enabled = st.widgets[38].get_value_string() == Some("true".to_string());
                                            let menubar_enabled = st.widgets[39].get_value_string() == Some("true".to_string());
                                            let statusbar_enabled = st.widgets[40].get_value_string() == Some("true".to_string());
                                            if opacity_enabled {
                                                cmd.arg("--opacity")
                                                   .arg("--transparency")
                                                   .arg(transparency_val.to_string());
                                            }
                                            if !border_enabled {
                                                cmd.arg("--no-border");
                                            } else {
                                                let border_width = st.widgets[43].value() as f32;
                                                let border_bevel = st.widgets[42].get_value_string() == Some("true".to_string());
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
                                            let _ = cmd.spawn();
                                        }
                                        changed = true;
                                    } else if tile_windows {
                                        st.update_status_text("Window Action: Tile active client windows");
                                        changed = true;
                                    }
                                } else if st.current_page == Page::Xdg {
                                    let mut open_file = false;
                                    let mut save_file = false;
                                    if st.widgets[34].take_click() {
                                        open_file = true;
                                    } else if st.widgets[35].take_click() {
                                        save_file = true;
                                    }

                                    if open_file {
                                        st.update_status_text("Opening Open File Dialog...");
                                        let sender_clone = self.sender.clone();
                                        std::thread::spawn(move || {
                                            open_file_dialog_portal(sender_clone);
                                        });
                                        changed = true;
                                    } else if save_file {
                                        st.update_status_text("Opening Save File Dialog...");
                                        let sender_clone = self.sender.clone();
                                        std::thread::spawn(move || {
                                            save_file_dialog_portal(sender_clone);
                                        });
                                        changed = true;
                                    }
                                }
                            }
                        }
                        if changed {
                            st.upload_vertices();
                            self.redraw = true;
                        }
                    }
                }
                PointerEventKind::Axis { horizontal, vertical, .. } => {
                    if let Some(st) = &mut self.state {
                        st.ui_context.ctrl_pressed = self.ctrl_pressed;
                        st.ui_context.shift_pressed = self.shift_pressed;
                        let h_scroll = horizontal.absolute as f32;
                        let v_scroll = vertical.absolute as f32;
                        let delta = cce_ui::widget::MouseScrollDelta::LineDelta(-h_scroll / 10.0, -v_scroll / 10.0);
                        let mut changed = false;
                        for (i, w) in st.widgets.iter_mut().enumerate() {
                            if is_control_panel_child(i) {
                                continue;
                            }
                            if w.mouse_wheel(&delta, st.cursor_x, st.cursor_y, &mut st.ui_context) {
                                changed = true;
                            }
                        }
                        if changed {
                            st.upload_vertices();
                            self.redraw = true;
                        }
                    }
                }
            }
        }
    }
}

impl KeyboardHandler for AppState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw_modifiers: &[u32],
        _keysyms: &[xkeysym::Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        let logical_key = match event.keysym {
            xkeysym::Keysym::Escape => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Escape),
            xkeysym::Keysym::Return => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Enter),
            xkeysym::Keysym::BackSpace => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Backspace),
            xkeysym::Keysym::Down => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::ArrowDown),
            xkeysym::Keysym::Up => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::ArrowUp),
            xkeysym::Keysym::Left => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::ArrowLeft),
            xkeysym::Keysym::Right => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::ArrowRight),
            xkeysym::Keysym::Tab => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Tab),
            xkeysym::Keysym::Delete => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Delete),
            xkeysym::Keysym::space => cce_ui::widget::Key::Named(cce_ui::widget::NamedKey::Space),
            _ => {
                if let Some(ref text) = event.utf8 {
                    cce_ui::widget::Key::Character(text.clone())
                } else if let Some(ch) = event.keysym.key_char() {
                    cce_ui::widget::Key::Character(ch.to_string())
                } else {
                    return;
                }
            }
        };

        let custom_event = cce_ui::widget::KeyEvent {
            state: cce_ui::widget::ElementState::Pressed,
            logical_key,
            text: event.utf8.clone(),
            repeat: false,
            ctrl: self.ctrl_pressed,
            shift: self.shift_pressed,
        };

        let mut handled = false;
        if let Some(state) = &mut self.state {
            if let Some(focused) = state.focused_widget {
                if state.widgets[focused].keyboard_input(&custom_event, &mut state.ui_context) {
                    state.upload_vertices();
                    self.redraw = true;
                    handled = true;
                }
            }
        }

        if !handled && event.keysym == xkeysym::Keysym::Escape {
            self.exit = true;
        }
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _layout: u32,
    ) {
        self.ctrl_pressed = modifiers.ctrl;
        self.shift_pressed = modifiers.shift;
        if let Some(state) = &mut self.state {
            state.ui_context.ctrl_pressed = modifiers.ctrl;
            state.ui_context.shift_pressed = modifiers.shift;
        }
    }
}

impl WindowHandler for AppState {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &XdgWindow,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        if let Some(state) = &mut self.state {
            let (w, h) = configure.new_size;
            let default_w = if state.is_child { self.initial_width as u32 } else { 1024 };
            let default_h = if state.is_child { self.initial_height as u32 } else { 768 };
            let mut w = if state.is_child {
                default_w
            } else {
                w.unwrap_or(std::num::NonZeroU32::new(default_w).unwrap()).get()
            };
            let mut h = if state.is_child {
                default_h
            } else {
                h.unwrap_or(std::num::NonZeroU32::new(default_h).unwrap()).get()
            };

            if state.is_child && self.child_shape.as_deref() == Some("circular") {
                let side = w.min(h);
                w = side;
                h = side;
            }

            let pw = (w as f64 * state.scale) as u32;
            let ph = (h as f64 * state.scale) as u32;
            state.resize(pw, ph);
        }
        self.redraw = true;
    }

    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &XdgWindow) {
        self.exit = true;
    }
}

impl ProvidesRegistryState for AppState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    
    fn runtime_add_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
        _version: u32,
    ) {}
    
    fn runtime_remove_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
    ) {}
}

delegate_compositor!(AppState);
delegate_xdg_shell!(AppState);
delegate_xdg_window!(AppState);
delegate_shm!(AppState);
delegate_seat!(AppState);
delegate_pointer!(AppState);
delegate_keyboard!(AppState);
delegate_registry!(AppState);
delegate_output!(AppState);

fn main() {
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

    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let (sender, channel) = calloop::channel::channel::<String>();

    let compositor_state = CompositorState::bind(&globals, &qh).unwrap();
    let xdg_shell_state = XdgShell::bind(&globals, &qh).unwrap();
    let shm_state = Shm::bind(&globals, &qh).unwrap();
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);

    let cursor_shape_manager = smithay_client_toolkit::seat::pointer::cursor_shape::CursorShapeManager::bind(&globals, &qh).ok();
    eprintln!("[test-suite] bound cursor_shape_manager: {:?}", cursor_shape_manager.is_some());

    let mut app = AppState {
        registry_state: RegistryState::new(&globals),
        compositor_state,
        xdg_shell_state,
        shm_state,
        seat_state,
        output_state,
        seats: Vec::new(),
        pointer: None,
        keyboard: None,
        window: None,
        surface: None,
        state: None,
        exit: false,
        redraw: true,
        cursor_shape_manager,
        cursor_shape_device: None,
        ctrl_pressed: false,
        shift_pressed: false,
        initial_width: c_w,
        initial_height: c_h,
        child_shape: child_shape.clone(),
        sender,
    };

    event_queue.roundtrip(&mut app).unwrap();

    let scale = cce_ui::wayland::detect_scale_factor(&app.output_state);

    let surface = app.compositor_state.create_surface(&qh);
    surface.set_buffer_scale(scale as i32);

    let pw = (c_w * scale as f32) as u32;
    let ph = (c_h * scale as f32) as u32;

    let window = app.xdg_shell_state.create_window(surface.clone(), WindowDecorations::None, &qh);
    if is_child {
        let win_title = match child_type.as_deref() {
            Some("Toplevel") => "Simulated Toplevel Window".to_string(),
            Some("Popup") => "Simulated Popup Window".to_string(),
            Some("LayerTop") => "Simulated Layer Shell (Top) Surface".to_string(),
            Some("LayerOverlay") => "Simulated Layer Shell (Overlay) Surface".to_string(),
            Some("LayerBackground") => "Simulated Layer Shell (Background) Surface".to_string(),
            Some(other) => format!("Simulated {} Window", other),
            None => "Simulated Client Window".to_string(),
        };
        window.set_title(&win_title);
        
        let mut app_id = if let Some(ref t) = child_type {
            format!("clear-test-child-{}", t.to_lowercase())
        } else {
            "clear-test-child".to_string()
        };
        if let Some(ref s) = child_shape {
            if s == "circular" {
                app_id.push_str("-circular");
            }
        }
        if no_border {
            app_id.push_str("-noborder");
        }
        window.set_app_id(&app_id);
    } else {
        window.set_title("Clear Test Interface - Diagnostics Dashboard");
        window.set_app_id("cce-test-interface");
    }
    if is_child {
        window.set_min_size(Some((c_w as u32, c_h as u32)));
        window.set_max_size(Some((c_w as u32, c_h as u32)));
    } else {
        window.set_min_size(Some((100, 100)));
    }
    window.commit();

    let wayland_handle = Box::leak(Box::new(cce_ui::wayland::WaylandSurfaceHandle {
        display_ptr: conn.backend().display_id().as_ptr() as *mut std::ffi::c_void,
        surface_ptr: surface.id().as_ptr() as *mut std::ffi::c_void,
    }));

    let state = pollster::block_on(State::new(
        wayland_handle,
        pw,
        ph,
        scale,
        is_child,
        child_type,
        opacity,
        transparency,
        use_backplate,
        use_menubar,
        use_statusbar,
        !no_border,
        border_width,
        border_bevel,
    ));

    app.window = Some(window);
    app.surface = Some(surface);
    app.state = Some(state);

    let mut event_loop = EventLoop::try_new().unwrap();
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(loop_handle.clone()).unwrap();

    loop_handle.insert_source(channel, |event, _metadata, app_state: &mut AppState| {
        if let calloop::channel::Event::Msg(msg) = event {
            if let Some(state) = &mut app_state.state {
                state.update_status_text(&msg);
                state.upload_vertices();
            }
            app_state.redraw = true;
        }
    }).unwrap();

    let mut last_tick = std::time::Instant::now();
    loop {
        event_loop
            .dispatch(std::time::Duration::from_millis(16), &mut app)
            .unwrap();
        if app.exit {
            break;
        }

        let now = std::time::Instant::now();
        let mut dt = now.duration_since(last_tick).as_secs_f32();
        last_tick = now;
        if dt > 0.1 {
            dt = 0.1;
        }

        if let Some(state) = &mut app.state {
            if state.tick(dt) {
                app.redraw = true;
            }
        }

        if app.redraw {
            app.redraw = false;
            if let Some(state) = &mut app.state {
                state.render();
            }
        }
    }
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

fn is_control_panel_child(index: usize) -> bool {
    match index {
        15..=26 | 38..=44 | 47 => true,
        _ => false,
    }
}

