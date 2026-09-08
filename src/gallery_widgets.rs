//! Gallery-owned lookalikes of containers the toolkit no longer ships: the old Plate
//! (an exhibit) and the root plate (the child windows' background), reproducing their
//! colour rules, corner rounding, borders and labels on the narrow widget traits
//! wrapped in `Adapted<W>`.

use cce_ui::colors;
use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::PaintCtx;
use cce_ui::widget::*;

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
