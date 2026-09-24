//! Gallery-owned lookalikes of containers the toolkit no longer ships: the old Plate
//! (an exhibit) and the root plate (the child windows' background), reproducing their
//! colour rules, corner rounding, borders and labels on the narrow widget traits
//! wrapped in `Adapted<W>`.

use cce_ui::colors;
use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{PaintCtx, PlateSpec};
use cce_ui::widget::*;

/// Exhibit lookalike of the old cce-ui `Plate`: config plate colour (else page colour)
/// at plate opacity, alpha negated under blur, plate corner radius, config border, and the
/// adapter's detached control label above it.
pub struct Plate {
    blur: bool,
}

impl Plate {
    pub fn new(x: f32, y: f32, w: f32, h: f32, blur: bool) -> Adapted<Plate> {
        let mut plate = Adapted::new(Self { blur });
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

}

// The label is the adapter's detached control label, in the strip above the content rect
// `paint` fills — the one label convention every control follows.
impl cce_ui::widget::Layout for Plate {}

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

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        // Rounded background over the content rect, only while corners are on and the
        // colour has alpha.
        let radius = cce_ui::layout::plate_corner_radius();
        let c = self.plate_color();
        if radius > 0.0 && c[3].abs() > 0.001 {
            pc.rounded_rect(rect, radius, (true, true, true, true), c);
        }
    }
}

impl cce_ui::widget::Input for Plate {}

/// Child-window background: the DE's standard root plate (cce-ui `PlateSpec::window` — the
/// root material at its opacity, the window silhouette on all four corners, the standard
/// rolled rim), immovable.
pub struct RootPlate;

impl RootPlate {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Adapted<RootPlate> {
        let mut bp = Adapted::new(Self);
        bp.set_rect(x, y, w, h);
        bp
    }
}

impl cce_ui::widget::Layout for RootPlate {}

impl cce_ui::widget::Paint for RootPlate {
    /// The legacy fill read only: the geometry is the plate `paint` emits.
    fn color(&self) -> [f32; 4] {
        cce_ui::color::page_low_color()
    }

    fn corner_style(&self, _rect: Rect) -> Option<(f32, (bool, bool, bool, bool))> {
        let r = cce_ui::color::root_plate_corner_radius();
        let on = r > 0.1;
        Some((r, (on, on, on, on)))
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        // The standard root plate (cce-ui PlateSpec::window), placed at this widget's
        // rect — the child window's origin, so `root_at(rect)` IS `window(w, h)`.
        pc.plate_spec(&PlateSpec::root_at(rect));
    }
}

impl cce_ui::widget::Input for RootPlate {}
