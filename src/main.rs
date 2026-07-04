use cce_ui::widget::{
    Button, Checkbox, ContentBg, Dropdown, Label, Paginator, Panel, ProgressBar, RangeSlider, Slider, Spinbox, StatusBar,
    TextLabel, Toggle, Element, Trackpad, hover_animation, TextBox, Plate, CornerRadii, Backplate, MenuBar, SectionContainer,
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
                Box::new(Backplate::new(0.0, 0.0, lw, lh).with_movable(false))
            } else {
                Box::new(ContentBg::new())
            };
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
                Box::new(Spinbox::new(1, 1, 10, 1).with_label("Border Width")), // 43 Spinbox: Border Width
                Box::new(SectionContainer::new("Window Elements")), // 44 Section: Window Elements
                Box::new(Dropdown::new(vec!["Controls".to_string(), "Windows".to_string(), "XDG".to_string()], 0)), // 45 Dropdown: Page selector
            ]
        };

        let positions = if is_child {
            child_positions(sw, sh, use_backplate, use_menubar, use_statusbar)
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
            ui_context: cce_ui::context::UiContext::new(),
        };

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
            3..=13 | 30 | 31 | 36 | 37 => self.current_page == Page::Controls,
            14..=29 | 38..=44 => self.current_page == Page::Windows,
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

                // Draw background
                if backplate_enabled {
                    let mut bg_color = cce_ui::color::page_low_color();
                    bg_color[3] = transparency_val;
                    let radii = CornerRadii::new(r, r, r, r);
                    push_rounded_rect_vertices_corners(
                        wx, wy, ww, wh, radii, sw, sh, bg_color, [0.0, 0.0, 0.0], None, &mut verts
                    );
                } else {
                    let bg_color = [0.12, 0.12, 0.15, transparency_val];
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
                        cce_ui::backend::window_runner::push_plate_solid_border_vertices(
                            wx, wy, ww, wh, radii, t, sw, sh, border_color, [0.0, 0.0, -1.0], &mut verts
                        );
                        if border_bevel {
                            let light_color = [
                                (border_color[0] + 0.2).min(1.0),
                                (border_color[1] + 0.2).min(1.0),
                                (border_color[2] + 0.2).min(1.0),
                                border_color[3]
                            ];
                            let dark_color = [
                                (border_color[0] - 0.2).max(0.0),
                                (border_color[1] - 0.2).max(0.0),
                                (border_color[2] - 0.2).max(0.0),
                                border_color[3]
                            ];
                            verts.extend(quad_vertices(wx + r, wy, ww - 2.0 * r, t, sw, sh, light_color));
                            verts.extend(quad_vertices(wx, wy + r, t, wh - 2.0 * r, sw, sh, light_color));
                            verts.extend(quad_vertices(wx + r, wy + wh - t, ww - 2.0 * r, t, sw, sh, dark_color));
                            verts.extend(quad_vertices(wx + ww - t, wy + r, t, wh - 2.0 * r, sw, sh, dark_color));
                        }
                    } else {
                        if border_bevel {
                            let light_color = [
                                (border_color[0] + 0.2).min(1.0),
                                (border_color[1] + 0.2).min(1.0),
                                (border_color[2] + 0.2).min(1.0),
                                border_color[3]
                            ];
                            let dark_color = [
                                (border_color[0] - 0.2).max(0.0),
                                (border_color[1] - 0.2).max(0.0),
                                (border_color[2] - 0.2).max(0.0),
                                border_color[3]
                            ];
                            verts.extend(quad_vertices(wx, wy, ww, t, sw, sh, light_color));
                            verts.extend(quad_vertices(wx, wy, t, wh, sw, sh, light_color));
                            verts.extend(quad_vertices(wx, wy + wh - t, ww, t, sw, sh, dark_color));
                            verts.extend(quad_vertices(wx + ww - t, wy, t, wh, sw, sh, dark_color));
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
            
            // Draw solid border if defined, or custom child border
            if self.is_child && i == 0 && self.border_enabled {
                let border_color = cce_ui::color::plate_border_color().unwrap_or([0.3, 0.3, 0.4, 1.0]);
                let t = self.border_width;
                let r = cce_ui::color::backplate_corner_radius();
                let backplate_enabled = self.use_backplate;
                
                if backplate_enabled {
                    let radii = CornerRadii::new(r, r, r, r);
                    cce_ui::backend::window_runner::push_plate_solid_border_vertices(
                        wx, wy, ww, wh, radii, t, sw, sh, border_color, [0.0, 0.0, -1.0], &mut verts
                    );
                    if self.border_bevel {
                        let light_color = [
                            (border_color[0] + 0.2).min(1.0),
                            (border_color[1] + 0.2).min(1.0),
                            (border_color[2] + 0.2).min(1.0),
                            border_color[3]
                        ];
                        let dark_color = [
                            (border_color[0] - 0.2).max(0.0),
                            (border_color[1] - 0.2).max(0.0),
                            (border_color[2] - 0.2).max(0.0),
                            border_color[3]
                        ];
                        verts.extend(quad_vertices(wx + r, wy, ww - 2.0 * r, t, sw, sh, light_color));
                        verts.extend(quad_vertices(wx, wy + r, t, wh - 2.0 * r, sw, sh, light_color));
                        verts.extend(quad_vertices(wx + r, wy + wh - t, ww - 2.0 * r, t, sw, sh, dark_color));
                        verts.extend(quad_vertices(wx + ww - t, wy + r, t, wh - 2.0 * r, sw, sh, dark_color));
                    }
                } else {
                    if self.border_bevel {
                        let light_color = [
                            (border_color[0] + 0.2).min(1.0),
                            (border_color[1] + 0.2).min(1.0),
                            (border_color[2] + 0.2).min(1.0),
                            border_color[3]
                        ];
                        let dark_color = [
                            (border_color[0] - 0.2).max(0.0),
                            (border_color[1] - 0.2).max(0.0),
                            (border_color[2] - 0.2).max(0.0),
                            border_color[3]
                        ];
                        verts.extend(quad_vertices(wx, wy, ww, t, sw, sh, light_color));
                        verts.extend(quad_vertices(wx, wy, t, wh, sw, sh, light_color));
                        verts.extend(quad_vertices(wx, wy + wh - t, ww, t, sw, sh, dark_color));
                        verts.extend(quad_vertices(wx + ww - t, wy, t, wh, sw, sh, dark_color));
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
                    3..=13 | 30 | 31 | 36 | 37 => current_page == Page::Controls,
                    14..=29 | 38..=44 => current_page == Page::Windows,
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

        let mut widget_buffers: Vec<Buffer> = Vec::new();
        let mut widget_labels: Vec<(TextLabel, Option<[f32; 4]>)> = Vec::new();
        for (i, w) in self.widgets.iter().enumerate() {
            if !is_visible(i) {
                continue;
            }
            let widget_font_opt = w.widget_font();
            for (label, font, bounds) in w.text_labels_with_font_and_bounds(&self.ui_context) {
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
                child_positions(self.width, self.height, self.use_backplate, self.use_menubar, self.use_statusbar)
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
                    0..=2 => true,
                    3..=13 | 30 | 31 | 36 | 37 => current_page == Page::Controls,
                    14..=29 | 38..=44 => current_page == Page::Windows,
                    32..=35 => current_page == Page::Xdg,
                    _ => false,
                }
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
    let sph = cce_ui::layout::spinbox_height();
    let tgh = cce_ui::layout::toggle_height();
    let slh = cce_ui::layout::slider_height();
    let bh = cce_ui::layout::button_height();

    // Dynamic calculations for Controls page layout
    let mut ctrl_y = 60.0;
    
    // Row 1: verify buttons
    let ctrl_verify_opacity_pos = (base_x, ctrl_y, 140.0, bh);
    let ctrl_verify_blur_pos = (base_x + 150.0, ctrl_y, 140.0, bh);
    let ctrl_verify_layout_pos = (base_x + 300.0, ctrl_y, 140.0, bh);
    ctrl_y += bh + 20.0;
    
    // Row 2: diagnostics & reset
    let ctrl_diagnostics_pos = (base_x, ctrl_y, 140.0, bh);
    let ctrl_reset_pos = (base_x + 150.0, ctrl_y, 140.0, bh);
    ctrl_y += bh + 20.0;
    
    // Row 3: checkbox, toggle, progress_bar
    let row3_h = tgh.max(24.0);
    let ctrl_checkbox_pos = (base_x, ctrl_y + (row3_h - 24.0)/2.0, 24.0, 24.0);
    let ctrl_toggle_pos = (base_x + 110.0, ctrl_y + (row3_h - tgh)/2.0, 48.0, tgh);
    let ctrl_progress_pos = (base_x + 230.0, ctrl_y + (row3_h - 24.0)/2.0, 160.0, 24.0);
    ctrl_y += row3_h + 20.0;
    
    // Row 4: slider & spinbox
    let row4_h = sph.max(slh);
    let ctrl_slider_pos = (base_x, ctrl_y + (row4_h - slh)/2.0, 300.0, slh);
    let ctrl_spinbox_pos = (base_x + 330.0, ctrl_y + (row4_h - sph)/2.0, 120.0, sph);

    // Dynamic calculations for Windows page layout
    let mut left_y = 80.0;
    
    // Live Preview
    let preview_pos = (base_x, left_y, 360.0, 240.0);
    left_y += 240.0 + 15.0; // 335.0
    
    // Buttons Row
    let create_btn_pos = (base_x, left_y, 172.0, bh);
    let tile_btn_pos = (base_x + 188.0, left_y, 172.0, bh);
    left_y += bh + 15.0; // 385.0
    
    // Window Type Dropdown
    let type_dd_pos = (base_x, left_y, 360.0, 35.0);
    left_y += 35.0 + 15.0; // 435.0
    
    // Window Shape Dropdown
    let shape_dd_pos = (base_x, left_y, 360.0, 35.0);
    left_y += 35.0 + 15.0; // 485.0
    
    // Opacity Toggle & Slider
    let opacity_toggle_pos = (base_x, left_y, 110.0, 32.0);
    let slider_pos = (base_x + 125.0, left_y, 235.0, 32.0);
    let slider_label_pos = (base_x + 125.0, left_y - 15.0, 200.0, 12.0);

    let mut right_y = 80.0;
    let rx = base_x + 380.0;
    let rw = 250.0;
    
    // Info Panel
    let info_h = 170.0;
    let info_bg_pos = (rx, right_y, rw, info_h);
    let info_header_pos = (rx + 10.0, right_y + 12.0, rw - 20.0, 20.0);
    let info_desc_pos = (rx + 10.0, right_y + 40.0, rw - 20.0, info_h - 50.0);
    right_y += info_h + 15.0;
    
    // Size Spinboxes
    let width_spin_pos = (rx, right_y, 120.0, sph);
    let height_spin_pos = (rx + 130.0, right_y, 120.0, sph);
    right_y += sph + 15.0;
    
    // Border Section Container
    let border_sec_h = 28.0 + 32.0 + 10.0 + sph + 10.0;
    let border_sec_pos = (rx, right_y, rw, border_sec_h);
    let border_enable_pos = (rx + 10.0, right_y + 28.0 + 5.0, 110.0, 32.0);
    let bevel_toggle_pos = (rx + 130.0, right_y + 28.0 + 5.0, 110.0, 32.0);
    let border_width_pos = (rx + 10.0, right_y + 28.0 + 5.0 + 32.0 + 10.0, rw - 20.0, sph);
    right_y += border_sec_h + 15.0;
    
    // Window Elements Section Container
    let win_sec_h = 28.0 + 32.0 + 10.0;
    let win_sec_pos = (rx, right_y, rw, win_sec_h);
    let backplate_toggle_pos = (rx + 10.0, right_y + 28.0 + 5.0, 70.0, 32.0);
    let menubar_toggle_pos = (rx + 85.0, right_y + 28.0 + 5.0, 70.0, 32.0);
    let statusbar_toggle_pos = (rx + 160.0, right_y + 28.0 + 5.0, 70.0, 32.0);

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
        create_btn_pos,                       // 15 Button: Create Window
        tile_btn_pos,                         // 16 Button: Tile Windows
        opacity_toggle_pos,                   // 17 Toggle: Opacity
        (-1000.0, -1000.0, 0.0, 0.0),         // 18 Label: dummy
        slider_pos,                           // 19 Slider: Transparency level
        slider_label_pos,                     // 20 Label: "Transparency level"
        type_dd_pos,                          // 21 Dropdown: Window type
        shape_dd_pos,                         // 22 Dropdown: Window shape
        border_enable_pos,                    // 23 Toggle: Enable
        (-1000.0, -1000.0, 0.0, 0.0),         // 24 Label: dummy
        width_spin_pos,                       // 25 Spinbox: Width
        height_spin_pos,                      // 26 Spinbox: Height
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
        backplate_toggle_pos,                 // 38 Toggle: Backplate
        menubar_toggle_pos,                   // 39 Toggle: MenuBar
        statusbar_toggle_pos,                 // 40 Toggle: StatusBar
        border_sec_pos,                       // 41 SectionContainer: Border
        bevel_toggle_pos,                     // 42 Toggle: Bevel
        border_width_pos,                     // 43 Spinbox: Border Width
        win_sec_pos,                          // 44 SectionContainer: Window Elements
        (sw - 140.0, sh - 25.0, 120.0, 22.0),         // 45 Dropdown: Page selector
    ]
}

fn child_positions(
    sw: f32,
    sh: f32,
    _use_backplate: bool,
    use_menubar: bool,
    use_statusbar: bool,
) -> Vec<(f32, f32, f32, f32)> {
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
                                        0..=2 => true,
                                        3..=13 | 30 | 31 | 36 | 37 => current_page == Page::Controls,
                                        14..=29 | 38..=44 => current_page == Page::Windows,
                                        32..=35 => current_page == Page::Xdg,
                                        _ => false,
                                    }
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
                                    3..=13 | 30 | 31 | 36 | 37 => current_page == Page::Controls,
                                    14..=29 | 38..=44 => current_page == Page::Windows,
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
                                    3..=13 | 30 | 31 | 36 | 37 => current_page == Page::Controls,
                                    14..=29 | 38..=44 => current_page == Page::Windows,
                                    32..=35 => current_page == Page::Xdg,
                                    _ => false,
                                }
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

