//! Gallery-owned copies of containers the toolkit no longer ships: the ControlPanel
//! (scroll chrome the app lays children into) and passive lookalikes of the old
//! Plate, SectionContainer and root plate container, kept as exhibits that reproduce their colour
//! rules, corner rounding, borders, separators and labels. All four sit on the narrow
//! widget traits wrapped in `Adapted<W>`.

use cce_ui::colors;
use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::PaintCtx;
use cce_ui::widget::*;

pub struct ControlPanel {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pub scroll_box: ScrollBox,
}

/// Scroll chrome only: background, borders and the ScrollBox. main.rs lays the child
/// slots out at screen coordinates (`arrange_control_panel`), paints them clamped to
/// the panel viewport in `display_list`, and dispatches them as ordinary roots.
impl ControlPanel {
    pub fn new() -> Adapted<ControlPanel> {
        let mut sb = ScrollBox::new();
        sb.show_border = false;
        sb.show_background = false;
        Adapted::new(Self {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            scroll_box: sb,
        })
    }

    /// The laid-out rect, mirrored from the adapter by `Layout::rect_assigned`.
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.w, self.h)
    }
}

impl cce_ui::widget::Layout for ControlPanel {
    // The panel's label is never rendered and its rect is never label-inflated.
    fn inline_label(&self) -> bool {
        true
    }

    fn rect_assigned(&mut self, rect: Rect) {
        self.x = rect.x;
        self.y = rect.y;
        self.w = rect.width;
        self.h = rect.height;
        self.scroll_box.set_rect(rect.x, rect.y, rect.width, rect.height);
    }
}

impl cce_ui::widget::Paint for ControlPanel {
    fn color(&self) -> [f32; 4] {
        colors::control_panel_color()
    }

    fn corner_style(&self, _rect: Rect) -> Option<(f32, (bool, bool, bool, bool))> {
        // All corners, 12px.
        Some((12.0, (true, true, true, true)))
    }

    /// Background + borders as radius-0 rounded prims, so they land in the rounded pass
    /// ahead of the children and the scrollbar the app paints after them.
    fn paint(&self, _rect: Rect, pc: &mut PaintCtx) {
        let (x, y, w, h) = self.rect();
        let none = (false, false, false, false);
        pc.rounded_rect(Rect { x, y, width: w, height: h }, 0.0, none, colors::control_panel_color());
        let border_color = colors::control_panel_border_color();
        pc.rounded_rect(Rect { x, y, width: w, height: 1.0 }, 0.0, none, border_color);
        pc.rounded_rect(Rect { x, y: y + h - 1.0, width: w, height: 1.0 }, 0.0, none, border_color);
        pc.rounded_rect(Rect { x, y, width: 1.0, height: h }, 0.0, none, border_color);
        pc.rounded_rect(Rect { x: x + w - 1.0, y, width: 1.0, height: h }, 0.0, none, border_color);
    }
}

impl cce_ui::widget::Input for ControlPanel {
    fn on_event(&mut self, event: &Event, ectx: &mut EventCtx) -> bool {
        let Some(ctx) = ectx.ui.as_deref_mut() else { return false };
        match event {
            Event::MouseButton { button, state, x: px, y: py, .. } => {
                self.scroll_box.mouse_input(*button, *state, *px, *py, ctx)
            }
            Event::PointerMove { x: px, y: py, .. } => {
                self.scroll_box.cursor_moved(*px, *py, ctx)
            }
            Event::MouseWheel { delta, x: px, y: py, .. } => {
                self.scroll_box.mouse_wheel(delta, *px, *py, ctx)
            }
            _ => false,
        }
    }

    fn draggable(&self, _rect: Rect) -> bool {
        self.scroll_box.draggable()
    }

    fn drag_begin(&mut self, px: f32, py: f32, _rect: Rect) {
        if self.scroll_box.hit_test_scrollbar(px, py) {
            self.scroll_box.drag_begin(px, py);
        }
    }

    fn drag_update(&mut self, px: f32, py: f32, _rect: Rect) -> bool {
        if self.scroll_box.draggable() {
            return self.scroll_box.drag_update(px, py);
        }
        false
    }

    fn drag_end(&mut self) {
        self.scroll_box.drag_end();
    }

    fn tick_ctx(&mut self, dt: f32, ectx: &mut EventCtx) -> bool {
        let Some(ctx) = ectx.ui.as_deref_mut() else { return false };
        self.scroll_box.tick(dt, ctx)
    }
}

/// Exhibit lookalike of the old cce-ui `Plate`: config plate colour (else page colour)
/// at plate opacity, alpha negated under blur, plate corner radius, config border, and a
/// detached-top label with the background shifted below it.
pub struct Plate {
    blur: bool,
    label: Option<String>,
}

impl Plate {
    pub fn new(x: f32, y: f32, w: f32, h: f32, blur: bool) -> Adapted<Plate> {
        let mut plate = Adapted::new(Self { blur, label: None });
        plate.set_rect(x, y, w, h);
        plate
    }

    fn plate_color(&self) -> [f32; 4] {
        let mut c = if let Some(c) = colors::plate_color() {
            c
        } else {
            colors::page_low_color()
        };
        c[3] *= cce_ui::layout::plate_opacity();
        if self.blur && colors::plate_blur() {
            c[3] = -c[3].abs();
        }
        c
    }

    /// Label offset for the model-held label copy.
    fn label_offset(&self) -> f32 {
        if cce_ui::layout::control_label_layout() == "side" {
            return 0.0;
        }
        if self.label.is_some() {
            let (_, font_size) = cce_ui::layout::control_label_font_detached_parsed();
            font_size + cce_ui::layout::control_label_margin()
        } else {
            0.0
        }
    }
}

impl cce_ui::widget::Layout for Plate {
    // The rect is not inflated for the label; the background shifts below it instead
    // (see `paint`).
    fn inline_label(&self) -> bool {
        true
    }
}

impl cce_ui::widget::Paint for Plate {
    fn color(&self) -> [f32; 4] {
        self.plate_color()
    }

    fn corner_style(&self, _rect: Rect) -> Option<(f32, (bool, bool, bool, bool))> {
        let r = cce_ui::layout::plate_corner_radius();
        let on = r > 0.0;
        Some((r, (on, on, on, on)))
    }

    fn solid_border(&self) -> Option<([f32; 4], f32)> {
        colors::plate_border_color().map(|bc| (bc, colors::plate_border_thickness()))
    }

    fn sync_label(&mut self, label: &str) {
        self.label = Some(label.to_string());
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        // Rounded background below the label region, only while corners are on and the
        // colour has alpha.
        let radius = cce_ui::layout::plate_corner_radius();
        let c = self.plate_color();
        if radius > 0.0 && c[3].abs() > 0.001 {
            let label_off = self.label_offset();
            pc.rounded_rect(
                Rect { x: rect.x, y: rect.y + label_off, width: rect.width, height: rect.height - label_off },
                radius,
                (true, true, true, true),
                c,
            );
        }
        if let Some(ref label) = self.label {
            let (_, font_size) = cce_ui::layout::control_label_font_parsed();
            pc.text(label.clone(), rect.x, rect.y, font_size, colors::control_label_color_u8());
        }
    }
}

impl cce_ui::widget::Input for Plate {}

/// Lookalike of the old cce-ui `SectionContainer` as the panel used it: a childless
/// section header row, title plus separator line. The model keeps a copy of the label
/// (via `sync_label`) for its own painting.
pub struct SectionContainer {
    title: String,
}

impl SectionContainer {
    pub fn new(title: &str) -> Adapted<SectionContainer> {
        Adapted::new(Self { title: title.to_string() }).with_label(title)
    }
}

impl cce_ui::widget::Layout for SectionContainer {
    // The model paints the title itself; keep the adapter's detached-label machinery
    // (offset inflation + fallback label) out of it.
    fn inline_label(&self) -> bool {
        true
    }
}

impl cce_ui::widget::Paint for SectionContainer {
    fn color(&self) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn sync_label(&mut self, label: &str) {
        self.title = label.to_string();
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        pc.quad(
            Rect { x: rect.x + 8.0, y: rect.y + 22.0, width: rect.width - 16.0, height: 1.0 },
            [0.18, 0.18, 0.27, 1.0],
        );
        pc.text(self.title.clone(), rect.x + 12.0, rect.y, 14.0, [212, 212, 212]);
    }
}

impl cce_ui::widget::Input for SectionContainer {
    fn blocks_root_plate_drag(&self) -> bool {
        false
    }
}

/// Child-window background, a lookalike of the old cce-ui root plate container: page colour at
/// the configured root plate opacity, root plate corner radius, immovable.
pub struct RootPlate;

impl RootPlate {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Adapted<RootPlate> {
        let mut bp = Adapted::new(Self);
        bp.set_rect(x, y, w, h);
        bp
    }

    fn root_plate_color(&self) -> [f32; 4] {
        let mut c = cce_ui::color::page_low_color();
        if c[3] > 0.001 {
            c[3] = cce_ui::color::root_plate_opacity();
        }
        c
    }
}

impl cce_ui::widget::Layout for RootPlate {}

impl cce_ui::widget::Paint for RootPlate {
    fn color(&self) -> [f32; 4] {
        self.root_plate_color()
    }

    fn corner_style(&self, _rect: Rect) -> Option<(f32, (bool, bool, bool, bool))> {
        let r = cce_ui::color::root_plate_corner_radius();
        let on = r > 0.1;
        Some((r, (on, on, on, on)))
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        // Painted only while the radius is on.
        let radius = cce_ui::color::root_plate_corner_radius();
        let c = self.root_plate_color();
        if radius > 0.1 && c[3].abs() > 0.001 {
            pc.rounded_rect(rect, radius, (true, true, true, true), c);
        }
    }
}

impl cce_ui::widget::Input for RootPlate {}
