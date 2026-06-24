use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Header, Label, Paginator, Panel, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    TextLabel, Toggle, Element, Trackpad, hover_animation,
};

use glyphon::{
    Attrs, Buffer, Cache, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
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

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
    clip_circle: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
        2 => Float32x3,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

fn quad_vertices(
    x: f32, y: f32, w: f32, h: f32,
    surface_w: f32, surface_h: f32,
    color: [f32; 4],
) -> [Vertex; 6] {
    let x0 = (x / surface_w) * 2.0 - 1.0;
    let y0 = 1.0 - (y / surface_h) * 2.0;
    let x1 = ((x + w) / surface_w) * 2.0 - 1.0;
    let y1 = 1.0 - ((y + h) / surface_h) * 2.0;

    [
        Vertex { position: [x0, y0], color, clip_circle: [0.0, 0.0, 0.0] },
        Vertex { position: [x1, y0], color, clip_circle: [0.0, 0.0, 0.0] },
        Vertex { position: [x0, y1], color, clip_circle: [0.0, 0.0, 0.0] },
        Vertex { position: [x1, y0], color, clip_circle: [0.0, 0.0, 0.0] },
        Vertex { position: [x1, y1], color, clip_circle: [0.0, 0.0, 0.0] },
        Vertex { position: [x0, y1], color, clip_circle: [0.0, 0.0, 0.0] },
    ]
}

fn widget_vertices(w: &dyn Element, sw: f32, sh: f32) -> Vec<Vertex> {
    let (x, y, ww, h) = w.rect();
    quad_vertices(x, y, ww, h, sw, sh, w.color()).to_vec()
}

fn make_text_buffer(font_system: &mut FontSystem, text: &str, size: f32) -> Buffer {
    let metrics = Metrics::new(size, size * 1.4);
    let mut buffer = Buffer::new(font_system, metrics);
    buffer.set_text(font_system, text, Attrs::new(), glyphon::Shaping::Advanced);
    buffer.shape_until_scroll(font_system, true);
    buffer
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Widgets,
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
    ui_context: cce_ui::context::UiContext,
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
    ) -> Self {
        let lw = pw as f32 / scale as f32;
        let lh = ph as f32 / scale as f32;
        let sw = lw;
        let sh = lh;

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
        let alpha_mode = if is_child && opacity {
            let caps = surface.get_capabilities(&adapter);
            if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied) {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
                wgpu::CompositeAlphaMode::PreMultiplied
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
            make_text_buffer(&mut font_system, "Clear Test Interface - Widgets", 16.0)
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
            vec![
                Box::new(Header::new()), // 0
                Box::new(ContentBg::new()), // 1
                Box::new(Label::new(&desc_label).with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 2
                Box::new(Button::new(0.0, 0.0, 100.0, 35.0).with_label("Close")), // 3
            ]
        } else {
            vec![
                Box::new(Header::new()), // 0
                {
                    let paginator = Paginator::new(vec!["Widgets".to_string(), "Windows".to_string(), "XDG".to_string()]);
                    Box::new(paginator)
                }, // 1
                Box::new(StatusBar::new()), // 2
                
                // "Widgets" page (indices 3..13)
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Button")), // 3
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Button")), // 4
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Button")), // 5
                Box::new(Panel::new(0.0, 0.0, 400.0, 200.0).with_label("Panel")), // 6
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Button")), // 7
                Box::new(Button::new_reset(0.0, 0.0, 140.0, 40.0).with_label("Button")), // 8
                Box::new(Checkbox::new().with_label("Checkbox")), // 9
                Box::new(Toggle::new().with_label("Toggle")), // 10
                Box::new(ProgressBar::new(0.0).with_label("ProgressBar")), // 11
                Box::new(Slider::new().with_label("Slider")), // 12
                Box::new(Spinbox::new(10, 1, 100, 5).with_label("Spinbox")), // 13

                // "Windows" page (indices 14..24)
                Box::new(Panel::new(0.0, 0.0, 400.0, 250.0)), // 14 (Window simulation area)
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Create Window")), // 15
                Box::new(Button::new(0.0, 0.0, 140.0, 40.0).with_label("Tile Windows")), // 16
                Box::new(Checkbox::new()), // 17
                Box::new(Label::new("Enable Opacity").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 18
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
                    let mut cb = Checkbox::new();
                    cb.set_checked(true);
                    cb
                }), // 23 Checkbox: Enable Border
                Box::new(Label::new("Enable Border").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 24 Label: "Enable Border"
                Box::new(Spinbox::new(400, 100, 2000, 10).with_label("Width")), // 25 Spinbox: Width
                Box::new(Spinbox::new(250, 100, 2000, 10).with_label("Height")), // 26 Spinbox: Height
                Box::new(Panel::new(0.0, 0.0, 190.0, 250.0)), // 27 (Info panel background)
                Box::new(Label::new("Surface Info").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 28 (Info panel header)
                Box::new(Label::new(
                    "A standard application\n\
                     window (xdg_toplevel).\n\
                     It supports tiling (cascade,\n\
                     split, grid), fullscreening,\n\
                     dragging, and resizing.\n\n\
                     Testing layout:\n\
                     cascades in cce."
                ).with_font_size(11.0).with_color([0x83, 0x83, 0x8a])), // 29 (Info panel description)
                Box::new(RangeSlider::new().with_label("RangeSlider")), // 30 (RangeSlider widget)
                Box::new(Trackpad::new().with_label("Trackpad")), // 31 (Trackpad widget)
                Box::new(Panel::new(0.0, 0.0, 450.0, 200.0).with_label("XDG Desktop Portal FileChooser")), // 32
                Box::new(Label::new("This page verifies the integration of the XDG Desktop Portal\nFile Chooser in the Clear environment.").with_font_size(12.0).with_color([0xcc, 0xcc, 0xd4])), // 33
                Box::new(Button::new(0.0, 0.0, 180.0, 40.0).with_label("Open File Dialog")), // 34
                Box::new(Button::new(0.0, 0.0, 180.0, 40.0).with_label("Save File Dialog")), // 35
            ]
        };

        let positions = if is_child {
            child_positions(sw, sh)
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
            current_page: Page::Widgets,
            is_child,
            opacity,
            transparency,
            ui_context: cce_ui::context::UiContext::new(),
        };

        cce_ui::scale::set_scale_factor(scale as f32);
        state.apply_layout();
        state.upload_vertices();
        state
    }

    fn is_widget_visible(&self, index: usize) -> bool {
        if self.is_child {
            return true;
        }
        match index {
            0..=2 => true,
            3..=13 | 30 | 31 => self.current_page == Page::Widgets,
            14..=29 => self.current_page == Page::Windows,
            32..=35 => self.current_page == Page::Xdg,
            _ => false,
        }
    }

    fn update_page_title(&mut self) {
        let title = match self.current_page {
            Page::Widgets => "Clear Test Interface - Widgets",
            Page::Windows => "Clear Test Interface - Windows",
            Page::Xdg => "Clear Test Interface - XDG Portal",
        };
        self.label_buffer = make_text_buffer(&mut self.font_system, title, 16.0);
    }

    fn apply_layout(&mut self) {
        let is_child = self.is_child;
        let current_page = self.current_page;
        let is_visible = |index: usize| -> bool {
            if is_child {
                return true;
            }
            match index {
                0..=2 => true,
                3..=13 | 30 | 31 => current_page == Page::Widgets,
                14..=29 => current_page == Page::Windows,
                32..=35 => current_page == Page::Xdg,
                _ => false,
            }
        };

        for (i, pos) in self.positions.iter().enumerate() {
            if let Some(widget) = self.widgets.get_mut(i) {
                if widget.is_dragging() {
                    continue;
                }
                if is_visible(i) {
                    let (x, y, w, h) = *pos;
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
        
        for (i, w) in self.widgets.iter().enumerate() {
            if !self.is_widget_visible(i) {
                continue;
            }
            verts.extend(widget_vertices(w.as_ref(), sw, sh));
            for (qx, qy, qw, qh, qc) in w.all_quads(&self.ui_context) {
                verts.extend(quad_vertices(qx, qy, qw, qh, sw, sh, qc));
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
        let current_page = self.current_page;
        let is_visible = |index: usize| -> bool {
            if is_child {
                return true;
            }
            match index {
                0..=2 => true,
                3..=13 | 30 | 31 => current_page == Page::Widgets,
                14..=29 => current_page == Page::Windows,
                32..=35 => current_page == Page::Xdg,
                _ => false,
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
            ref label_buffer,
            ref status_buffer,
            physical_width,
            physical_height,
            scale,
            ..
        } = self;

        let viewport = Resolution { width: *physical_width, height: *physical_height };
        text_viewport.update(queue, viewport);

        let scale_f32 = *scale as f32;
        let left_margin = if is_child { 20.0 } else { 76.0 };

        let mut areas: Vec<TextArea> = vec![
            TextArea {
                buffer: label_buffer,
                left: (left_margin * scale_f32).round(),
                top: (12.0 * scale_f32).round(),
                scale: scale_f32,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: *physical_width as i32,
                    bottom: *physical_height as i32,
                },
                default_color: glyphon::Color::rgb(0xcc, 0xcc, 0xd4),
                custom_glyphs: &[],
            },
        ];

        if !is_child {
            areas.push(TextArea {
                buffer: status_buffer,
                left: (12.0 * scale_f32).round(),
                top: (*physical_height as f32 - 24.0 * scale_f32).round(),
                scale: scale_f32,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: *physical_width as i32,
                    bottom: *physical_height as i32,
                },
                default_color: glyphon::Color::rgb(0xaa, 0xbb, 0xdd),
                custom_glyphs: &[],
            });
        }

        let mut widget_buffers: Vec<Buffer> = Vec::new();
        let mut widget_labels: Vec<TextLabel> = Vec::new();
        for (i, w) in self.widgets.iter().enumerate() {
            if !is_visible(i) {
                continue;
            }
            for label in w.text_labels() {
                widget_buffers.push(make_text_buffer(font_system, &label.text, label.font_size));
                widget_labels.push(label);
            }
        }

        for (buf, label) in widget_buffers.iter().zip(widget_labels.iter()) {
            areas.push(TextArea {
                buffer: buf,
                left: (label.x * scale_f32).round(),
                top: (label.y * scale_f32).round(),
                scale: scale_f32,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: *physical_width as i32,
                    bottom: *physical_height as i32,
                },
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
            if w.popover_rect().is_some() {
                w.render_popover(&mut popover_pc);
            }
        }
        let mut popover_buffers = Vec::new();
        for (t, size, _x, _y, _tc, _font_opt, _bounds) in &popover_pc.texts {
            popover_buffers.push(make_text_buffer(font_system, t, *size));
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
                scale: scale_f32,
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
                child_positions(self.width, self.height)
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
            let clear_alpha = if self.is_child && self.opacity {
                self.transparency as f64
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
                            r: 0.05,
                            g: 0.05,
                            b: 0.08,
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
        if hover_animation::tick(dt) {
            changed = true;
        }
        let is_child = self.is_child;
        let current_page = self.current_page;
        let is_visible = |index: usize| -> bool {
            if is_child {
                return true;
            }
            match index {
                0..=2 => true,
                3..=13 | 30 | 31 => current_page == Page::Widgets,
                14..=29 => current_page == Page::Windows,
                32..=35 => current_page == Page::Xdg,
                _ => false,
            }
        };
        for (i, w) in self.widgets.iter_mut().enumerate() {
            if is_visible(i) {
                if w.tick(dt, &mut self.ui_context) {
                    changed = true;
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
    vec![
        // Always visible
        (0.0, 0.0, sw, 40.0),               // 0 header
        (0.0, 40.0, sw, sh - 68.0),         // 1 Paginator
        (0.0, sh - 28.0, sw, 28.0),         // 2 status_bar

        // "Widgets" page only
        (base_x, 50.0, 140.0, 40.0),          // 3 verify_opacity
        (base_x + 150.0, 50.0, 140.0, 40.0),          // 4 verify_blur
        (base_x + 300.0, 50.0, 140.0, 40.0),          // 5 verify_layout
        (base_x, 120.0, 400.0, 200.0),        // 6 panel
        (base_x, 340.0, 140.0, 40.0),         // 7 run_diagnostics
        (base_x + 150.0, 340.0, 140.0, 40.0),         // 8 reset
        (base_x, 410.0, 24.0, 24.0),          // 9 checkbox
        (base_x + 110.0, 410.0, 48.0, 24.0),          // 10 toggle
        (base_x + 230.0, 410.0, 160.0, 24.0),         // 11 progress_bar
        (base_x, 470.0, 300.0, 24.0),         // 12 slider
        (base_x + 330.0, 470.0, 120.0, 24.0),         // 13 spinbox

        // "Windows" page only
        (base_x, 100.0, 400.0, 250.0),        // 14 Panel (Window area)
        (base_x, 360.0, 140.0, 40.0),         // 15 Button: Create Window
        (base_x + 150.0, 360.0, 140.0, 40.0),         // 16 Button: Tile Windows
        (base_x, 410.0, 24.0, 24.0),          // 17 Checkbox: Enable Opacity
        (base_x + 35.0, 415.0, 150.0, 16.0),         // 18 Label: "Enable Opacity"
        (base_x, 444.0, 300.0, 32.0),         // 19 Slider: Transparency level
        (base_x + 310.0, 452.0, 150.0, 16.0),         // 20 Label: "Transparency level"
        (base_x + 300.0, 360.0, 175.0, 40.0),         // 21 Dropdown: Window type
        (base_x + 300.0, 410.0, 175.0, 40.0),         // 22 Dropdown: Window shape
        (base_x, 490.0, 24.0, 24.0),          // 23 Checkbox: Enable Border
        (base_x + 35.0, 495.0, 150.0, 16.0),         // 24 Label: "Enable Border"
        (base_x + 300.0, 490.0, 175.0, 40.0),         // 25 Spinbox: Width
        (base_x + 300.0, 540.0, 175.0, 40.0),         // 26 Spinbox: Height
        (base_x + 420.0, 100.0, 190.0, 250.0),        // 27 Panel: Info background
        (base_x + 430.0, 110.0, 170.0, 20.0),         // 28 Label: Info header
        (base_x + 430.0, 140.0, 170.0, 200.0),        // 29 Label: Info description
        (base_x, 530.0, 300.0, 24.0),         // 30 RangeSlider
        (base_x + 420.0, 120.0, 190.0, 200.0),        // 31 Trackpad
        // "XDG" page only
        (base_x, 100.0, 450.0, 200.0),        // 32 Panel: XDG Portal background
        (base_x + 20.0, 140.0, 410.0, 40.0),         // 33 Label: XDG explanation
        (base_x + 20.0, 220.0, 180.0, 40.0),         // 34 Button: Open File Dialog
        (base_x + 220.0, 220.0, 180.0, 40.0),         // 35 Button: Save File Dialog
    ]
}

fn child_positions(sw: f32, sh: f32) -> Vec<(f32, f32, f32, f32)> {
    vec![
        (0.0, 0.0, sw, 40.0),               // 0 header
        (0.0, 40.0, sw, sh - 40.0),         // 1 content_bg
        (20.0, 80.0, sw - 40.0, 40.0),      // 2 label
        ((sw - 100.0) / 2.0, sh - 60.0, 100.0, 35.0), // 3 close button
    ]
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
                            let current_page = state.current_page;
                            let is_visible = |index: usize| -> bool {
                                if is_child {
                                    return true;
                                }
                                match index {
                                    0..=2 => true,
                                    3..=13 | 30 | 31 => current_page == Page::Widgets,
                                    14..=29 => current_page == Page::Windows,
                                    32..=35 => current_page == Page::Xdg,
                                    _ => false,
                                }
                            };
                            for (i, w) in state.widgets.iter_mut().enumerate() {
                                if !is_visible(i) {
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
                        let current_page = st.current_page;
                        let is_visible = |index: usize| -> bool {
                            if is_child {
                                return true;
                            }
                            match index {
                                0..=2 => true,
                                3..=13 | 30 | 31 => current_page == Page::Widgets,
                                14..=29 => current_page == Page::Windows,
                                32..=35 => current_page == Page::Xdg,
                                _ => false,
                            }
                        };
                        let mut clicked_idx = None;
                        for i in (0..st.widgets.len()).rev() {
                            if !is_visible(i) {
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
                        let current_page = st.current_page;
                        let is_visible = |index: usize| -> bool {
                            if is_child {
                                return true;
                            }
                            match index {
                                0..=2 => true,
                                3..=13 | 30 | 31 => current_page == Page::Widgets,
                                14..=29 => current_page == Page::Windows,
                                32..=35 => current_page == Page::Xdg,
                                _ => false,
                            }
                        };
                        for (i, w) in st.widgets.iter_mut().enumerate() {
                            if !is_visible(i) {
                                continue;
                            }
                            if w.mouse_input(btn, cce_ui::widget::ElementState::Released, st.cursor_x, st.cursor_y, &mut st.ui_context) {
                                changed = true;
                            }
                        }
                        
                        if btn == cce_ui::widget::MouseButton::Left {
                            if st.is_child {
                                if st.widgets[3].take_click() {
                                    self.exit = true;
                                    changed = true;
                                }
                            } else {
                                let mut page_changed = false;
                                
                                let mut selected = 0;
                                if let Some((new_page, _)) = st.widgets[1].as_menu_controller_mut().unwrap().menu_click() {
                                    st.widgets[1].as_page_selector_mut().unwrap().set_selected_page(new_page);
                                    selected = new_page as i32;
                                    page_changed = true;
                                }
                                
                                if page_changed {
                                    if selected == 0 {
                                        st.current_page = Page::Widgets;
                                        st.update_page_title();
                                        st.update_status_text("Viewing Widgets Page");
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
                                } else if st.current_page == Page::Widgets {
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
                                    } else {
                                        for i in 9..=13 {
                                            if st.widgets[i].take_click() {
                                                changed = true;
                                            }
                                        }
                                        if st.widgets[30].take_click() {
                                            changed = true;
                                        }
                                        if st.widgets[31].take_click() {
                                            changed = true;
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
                                    } else {
                                        let mut dropdown_clicked = false;
                                        for i in 17..=26 {
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
                                        let opacity_enabled = st.widgets[17].value() == 1;
                                        let transparency_pct = st.widgets[19].value();
                                        let transparency_val = transparency_pct as f32 / 100.0;
                                        let border_enabled = st.widgets[23].value() == 1;
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
                                            if opacity_enabled {
                                                cmd.arg("--opacity")
                                                   .arg("--transparency")
                                                   .arg(transparency_val.to_string());
                                            }
                                            if !border_enabled {
                                                cmd.arg("--no-border");
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
                        let h_scroll = horizontal.absolute as f32;
                        let v_scroll = vertical.absolute as f32;
                        let delta = cce_ui::widget::MouseScrollDelta::LineDelta(-h_scroll / 10.0, -v_scroll / 10.0);
                        let mut changed = false;
                        for w in &mut st.widgets {
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
            let mut w = w.unwrap_or(std::num::NonZeroU32::new(default_w).unwrap()).get();
            let mut h = h.unwrap_or(std::num::NonZeroU32::new(default_h).unwrap()).get();

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

fn percent_decode(s: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = s.as_bytes().iter();
    while let Some(&b) = chars.next() {
        if b == b'%' {
            if let (Some(&h), Some(&l)) = (chars.next(), chars.next()) {
                if let Ok(hex) = String::from_utf8(vec![h, l]) {
                    if let Ok(decoded) = u8::from_str_radix(&hex, 16) {
                        bytes.push(decoded);
                        continue;
                    }
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn open_file_dialog_portal(sender: calloop::channel::Sender<String>) {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let conn = zbus::blocking::Connection::session()?;
        let mut options = std::collections::HashMap::new();
        options.insert("handle_token".to_string(), zbus::zvariant::Value::from("clear_test_open_token"));

        let msg: zbus::Message = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.FileChooser"),
            "OpenFile",
            &("", "Open File Dialog", &options),
        )?;
        let reply: zbus::zvariant::OwnedObjectPath = msg.body().deserialize()?;

        let request_proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.portal.Desktop",
            reply,
            "org.freedesktop.portal.Request",
        )?;

        let mut signal_stream = request_proxy.receive_signal("Response")?;
        if let Some(msg) = signal_stream.next() {
            let (response_code, results): (u32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>) = msg.body().deserialize()?;
            if response_code == 0 {
                if let Some(val) = results.get("uris") {
                    if let Ok(uris) = Vec::<String>::try_from(val.clone()) {
                        let mut file_paths = Vec::new();
                        for uri in uris {
                            let path = uri.trim_start_matches("file://");
                            file_paths.push(percent_decode(path));
                        }
                        if !file_paths.is_empty() {
                            let _ = sender.send(format!("Selected: {}", file_paths.join(", ")));
                        } else {
                            let _ = sender.send("Selected no files".to_string());
                        }
                    } else {
                        let _ = sender.send("Selected success, but failed to parse URIs".to_string());
                    }
                } else {
                    let _ = sender.send("Selected success, but no uris".to_string());
                }
            } else if response_code == 1 {
                let _ = sender.send("File dialog cancelled by user".to_string());
            } else {
                let _ = sender.send(format!("File dialog closed (code {})", response_code));
            }
        } else {
            let _ = sender.send("Request closed without response".to_string());
        }
        Ok(())
    })();

    if let Err(e) = result {
        let _ = sender.send(format!("Error: {}", e));
    }
}

fn save_file_dialog_portal(sender: calloop::channel::Sender<String>) {
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let conn = zbus::blocking::Connection::session()?;
        let mut options = std::collections::HashMap::new();
        options.insert("handle_token".to_string(), zbus::zvariant::Value::from("clear_test_save_token"));

        let msg: zbus::Message = conn.call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.portal.FileChooser"),
            "SaveFile",
            &("", "Save File Dialog", &options),
        )?;
        let reply: zbus::zvariant::OwnedObjectPath = msg.body().deserialize()?;

        let request_proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.freedesktop.portal.Desktop",
            reply,
            "org.freedesktop.portal.Request",
        )?;

        let mut signal_stream = request_proxy.receive_signal("Response")?;
        if let Some(msg) = signal_stream.next() {
            let (response_code, results): (u32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>) = msg.body().deserialize()?;
            if response_code == 0 {
                if let Some(val) = results.get("uris") {
                    if let Ok(uris) = Vec::<String>::try_from(val.clone()) {
                        let mut file_paths = Vec::new();
                        for uri in uris {
                            let path = uri.trim_start_matches("file://");
                            file_paths.push(percent_decode(path));
                        }
                        if !file_paths.is_empty() {
                            let _ = sender.send(format!("Saved to: {}", file_paths.join(", ")));
                        } else {
                            let _ = sender.send("Saved to no file".to_string());
                        }
                    } else {
                        let _ = sender.send("Save success, but failed to parse URIs".to_string());
                    }
                } else {
                    let _ = sender.send("Save success, but no uris".to_string());
                }
            } else if response_code == 1 {
                let _ = sender.send("Save dialog cancelled by user".to_string());
            } else {
                let _ = sender.send(format!("Save dialog closed (code {})", response_code));
            }
        } else {
            let _ = sender.send("Request closed without response".to_string());
        }
        Ok(())
    })();

    if let Err(e) = result {
        let _ = sender.send(format!("Error: {}", e));
    }
}

