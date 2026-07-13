//! Test-interface-owned copies of the dissolved cce-ui containers (Phase 6as): the
//! gallery is the last constructor of ControlPanel / SectionContainer / Plate /
//! Backplate, and it needs them only as demo chrome — the panel verbatim, the other
//! three as passive lookalikes replicating the legacy types' exact emission surfaces
//! (color rules, corner radius/rounding, borders, separators, labels, hit shapes).
//! Phase 6az: all four are on the narrow traits wrapped in `Adapted<W>`; the gallery
//! roster keeps them as `Box<dyn WidgetHost>` and reaches the models through `as_any`.

use cce_ui::colors;
use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::PaintCtx;
use cce_ui::widget::*;
use cce_ui::widget::ScrollBox;

pub struct ControlPanel {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    pub scroll_box: ScrollBox,
}

/// The panel is scroll chrome only (the ControlPanel endgame): background, borders, and
/// the ScrollBox machinery. Its former stored child pointers, label-matched arrangement,
/// aggregate views, and dyn event/tick/drag forwarding are DISSOLVED into the app —
/// main.rs lays the child slots out at SCREEN (scrolled) coordinates
/// (`arrange_control_panel`), emits their geometry/text clamped to the panel viewport in
/// `display_list`, and dispatches them as ordinary routed roots. No `*mut dyn` storage,
/// no dummy contexts, no scroll-translated coordinates anywhere.
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
    // The panel's own label ("ControlPanel") never rendered on the legacy paths and its
    // rect was never label-inflated — keep the adapter out of both.
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
        // Legacy: rounded_corners all-true with the WidgetHost-default 12.0 radius.
        Some((12.0, (true, true, true, true)))
    }

    /// Background + borders, as radius-0 rounded prims so they ride the adapter's
    /// rounded-tuple bridge exactly where the legacy aggregate emitted them (the app
    /// emits the children and the scrollbar after these, in the legacy order).
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

/// Gallery exhibit lookalike of the dissolved cce-ui `Plate`: default plate color rule
/// (config plate color, else page color) at plate opacity, alpha negated under blur,
/// plate corner radius/rounding, config border, detached-top label handled like the
/// legacy plate (background shifted below the label, label drawn from `paint`).
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

    /// The legacy `Widget::label_offset` rule for the model-held label copy.
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
    // The legacy plate never inflated its rect for the label; it shifted its background
    // below it instead (see `paint`).
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
        // Rounded background below the label region — the legacy all_rounded_quads
        // override: emitted only while corners are on and the color has alpha.
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

/// Gallery lookalike of the dissolved cce-ui `SectionContainer` as the ControlPanel uses
/// it: a childless section header row — the title label plus the SectionHeader separator
/// line. The panel matches sections by the adapter's base label; the model keeps a copy
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
    // The title is painted by the model at its legacy position; keep the adapter's
    // detached-label machinery (offset inflation + fallback label) out of it.
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
    fn blocks_backplate_drag(&self) -> bool {
        false
    }
}

/// Child-window background lookalike of the dissolved cce-ui `Backplate`: the base color
/// forced to the configured backplate opacity, backplate corner radius, immovable.
pub struct Backplate {
    bevel: Option<f32>,
}

impl Backplate {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Adapted<Backplate> {
        let mut bp = Adapted::new(Self { bevel: None });
        bp.set_rect(x, y, w, h);
        bp
    }

    pub fn set_bevel(&mut self, thickness: f32) {
        self.bevel = Some(thickness);
    }

    fn backplate_color(&self) -> [f32; 4] {
        let mut c = cce_ui::color::page_low_color();
        if c[3] > 0.001 {
            c[3] = cce_ui::color::active_backplate_opacity();
        }
        c
    }
}

impl cce_ui::widget::Layout for Backplate {}

impl cce_ui::widget::Paint for Backplate {
    fn color(&self) -> [f32; 4] {
        self.backplate_color()
    }

    fn corner_style(&self, _rect: Rect) -> Option<(f32, (bool, bool, bool, bool))> {
        let r = cce_ui::color::backplate_corner_radius();
        let on = r > 0.1;
        Some((r, (on, on, on, on)))
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        // The legacy rounded Backplate emitted its background only through
        // all_rounded_quads (nothing when the radius is off) — same shape here. The bevel
        // field is carried for the --border-bevel flag but, like the legacy lookalike, has
        // no reader on the gallery's display path.
        let radius = cce_ui::color::backplate_corner_radius();
        let c = self.backplate_color();
        if radius > 0.1 && c[3].abs() > 0.001 {
            pc.rounded_rect(rect, radius, (true, true, true, true), c);
        }
    }
}

impl cce_ui::widget::Input for Backplate {}
