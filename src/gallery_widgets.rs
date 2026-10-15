//! Gallery-owned widgets on the narrow widget traits, wrapped in `Adapted<W>`: lookalikes of
//! containers the toolkit no longer ships — the old Plate (an exhibit) and the root plate
//! (the child windows' background), reproducing their colour rules, corner rounding, borders
//! and labels — and the Relief page's tiles ([`ReliefTile`]), each one of the toolkit's
//! relief effects.

use cce_ui::color;
use cce_ui::scene::layout::Rect;
use cce_ui::scene::paint::{PaintCtx, PlateSpec};
use cce_ui::widget::*;

/// Exhibit lookalike of the old cce-ui `Plate`: config plate colour (else page colour)
/// at plate opacity, alpha negated under blur, the panel corner radius, config border, and the
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
        let mut c = if let Some(c) = color::plate_color() {
            c
        } else {
            color::page_low_color()
        };
        c[3] *= cce_ui::layout::plate_opacity();
        if self.blur && color::plate_blur() {
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
        let r = cce_ui::layout::panel_corner_radius();
        let on = r > 0.0;
        Some((r, (on, on, on, on)))
    }

    fn solid_border(&self) -> Option<([f32; 4], f32)> {
        color::plate_border_color().map(|bc| (bc, color::plate_border_thickness()))
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        // Rounded background over the content rect, only while corners are on and the
        // colour has alpha.
        let radius = cce_ui::layout::panel_corner_radius();
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

/// One of the toolkit's relief effects, as a tile on the Relief page shows it: what
/// `PaintCtx` draws a lit surface with — plates, carves, fields and wells, and the
/// compositions widgets make of them — each sized and placed as the toolkit's own
/// widgets draw it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    PanePlate,
    Bevel,
    Boss,
    ControlRaised,
    ControlFlush,
    ControlFlat,
    ControlFocused,
    InsetPlate,
    Recess,
    Trough,
    Ridge,
    FieldWell,
    FieldRun,
    FieldEndingInRun,
    FieldSlidingRun,
    FieldSpanning,
    CanvasWell,
    SectionWell,
    ConcaveFillet,
    Groove,
    CarveUnion,
    Lattice,
    Grout,
    Frame,
    DiscPlate,
    Sphere,
    Droplet,
    Materials,
    Depths,
}

impl Effect {
    /// Every effect, in the order the Relief page lays its tiles out: the plates, the
    /// control plate's stances, the carves, a field's forms, the wells, the composed
    /// shapes, the round ones, then the two series.
    pub const ALL: [Effect; 29] = [
        Effect::PanePlate,
        Effect::Bevel,
        Effect::Boss,
        Effect::ControlRaised,
        Effect::ControlFlush,
        Effect::ControlFlat,
        Effect::ControlFocused,
        Effect::InsetPlate,
        Effect::Recess,
        Effect::Trough,
        Effect::Ridge,
        Effect::FieldWell,
        Effect::FieldRun,
        Effect::FieldEndingInRun,
        Effect::FieldSlidingRun,
        Effect::FieldSpanning,
        Effect::CanvasWell,
        Effect::SectionWell,
        Effect::ConcaveFillet,
        Effect::Groove,
        Effect::CarveUnion,
        Effect::Lattice,
        Effect::Grout,
        Effect::Frame,
        Effect::DiscPlate,
        Effect::Sphere,
        Effect::Droplet,
        Effect::Materials,
        Effect::Depths,
    ];

    /// The tile's caption: the adapter's detached label.
    pub fn caption(self) -> &'static str {
        match self {
            Effect::PanePlate => "Pane plate",
            Effect::Bevel => "Bevel",
            Effect::Boss => "Boss",
            Effect::ControlRaised => "Control plate (raised)",
            Effect::ControlFlush => "Control plate (flush)",
            Effect::ControlFlat => "Control plate (flat)",
            Effect::ControlFocused => "Control plate (focused)",
            Effect::InsetPlate => "Inset plate",
            Effect::Recess => "Recess",
            Effect::Trough => "Trough",
            Effect::Ridge => "Ridge",
            Effect::FieldWell => "Field (well)",
            Effect::FieldRun => "Field (run)",
            Effect::FieldEndingInRun => "Field (ending in run)",
            Effect::FieldSlidingRun => "Field (sliding run)",
            Effect::FieldSpanning => "Field (spanning)",
            Effect::CanvasWell => "Canvas well",
            Effect::SectionWell => "Section well",
            Effect::ConcaveFillet => "Concave fillet",
            Effect::Groove => "Groove",
            Effect::CarveUnion => "Carve union",
            Effect::Lattice => "Lattice",
            Effect::Grout => "Grout",
            Effect::Frame => "Frame",
            Effect::DiscPlate => "Disc plate",
            Effect::Sphere => "Sphere",
            Effect::Droplet => "Droplet",
            Effect::Materials => "Materials",
            Effect::Depths => "Recess depths",
        }
    }

    /// A series — several samples side by side, each captioned under itself — which lies
    /// across two columns.
    pub fn is_series(self) -> bool {
        matches!(self, Effect::Materials | Effect::Depths)
    }

    /// The tile's content height: a control row's effects stand in a row's room, the rest
    /// in a canvas's.
    pub fn height(self) -> f32 {
        match self {
            Effect::ControlRaised
            | Effect::ControlFlush
            | Effect::ControlFlat
            | Effect::ControlFocused
            | Effect::FieldWell
            | Effect::FieldRun
            | Effect::FieldEndingInRun
            | Effect::FieldSlidingRun
            | Effect::FieldSpanning => ROW_TILE_H,
            _ => CANVAS_TILE_H,
        }
    }
}

/// A canvas effect's tile height, and the sample it draws centred in it.
const CANVAS_TILE_H: f32 = 120.0;
const SAMPLE_W: f32 = 150.0;
const SAMPLE_H: f32 = 80.0;
/// A control row's effect's tile height: the row and room round it.
const ROW_TILE_H: f32 = 60.0;
/// The Separator exhibit's flat rule colour, which a flat groove is drawn in.
const RULE_COLOR: [f32; 4] = [0.5, 0.5, 0.6, 1.0];

/// A Relief-page tile: ONE effect ([`Effect`]), or a small captioned series of one, drawn
/// centred in the content rect with the `PaintCtx` relief emitters, under the adapter's
/// detached label. Where a widget draws a flat equivalent with relief off, the tile does
/// too (`layout::control_relief`, the Style dropdown); an effect no widget draws flat is
/// drawn under both styles, and says so.
pub struct ReliefTile {
    effect: Effect,
}

impl ReliefTile {
    pub fn new(effect: Effect) -> Adapted<ReliefTile> {
        Adapted::new(Self { effect }).with_label(effect.caption())
    }
}

impl cce_ui::widget::Layout for ReliefTile {}

impl cce_ui::widget::Paint for ReliefTile {
    /// No fill: the tile is the well's floor, which every effect is drawn on or cut into.
    fn color(&self) -> [f32; 4] {
        [0.0; 4]
    }

    fn paint(&self, rect: Rect, pc: &mut PaintCtx) {
        paint_effect(self.effect, rect, pc);
    }
}

impl cce_ui::widget::Input for ReliefTile {}

/// A `w` x `h` rect centred in `r`.
fn centred(r: Rect, w: f32, h: f32) -> Rect {
    Rect { x: r.x + (r.width - w) * 0.5, y: r.y + (r.height - h) * 0.5, width: w, height: h }
}

fn all(r: f32) -> (f32, f32, f32, f32) {
    (r, r, r, r)
}

/// The hairline frame every well falls back to with relief off (`well_rim`'s flat arm).
fn hairline(pc: &mut PaintCtx, rect: Rect, radii: (f32, f32, f32, f32)) {
    pc.border(rect, radii, [0.0; 4], color::well_frame_color(false, false), 1.0);
}

/// A series sample's caption, centred on `cx` with its top at `y`, in the control label font.
fn caption(pc: &mut PaintCtx, text: &str, cx: f32, y: f32) {
    let (family, size) = cce_ui::layout::control_label_font_parsed();
    let w = cce_ui::widget::display::measure_text_width(text, &family, size);
    pc.text_with(text.to_string(), cx - w * 0.5, y, size, color::control_label_color_u8(), None, None);
}

/// The room a series keeps under its samples for their captions.
fn caption_h() -> f32 {
    cce_ui::layout::control_label_font_parsed().1 + 6.0
}

/// A face for the effects that show one: the DE's control material where it has a tint,
/// else an opaque slate a step above the floor. A control's configured face can be clear
/// (the surface below is the face, `ControlPlate::faced`), and then the faced stances and
/// the bevel and frame would show only their edges, or nothing at all flat.
fn face() -> cce_ui::scene::Material {
    let m = cce_ui::scene::Material::control();
    if m.tint[3] > 0.001 { m } else { cce_ui::scene::Material::opaque(SLATE) }
}

/// The face [`face`] falls back to, linear RGB.
const SLATE: [f32; 4] = [0.16, 0.18, 0.26, 1.0];

/// The DE accent, opaque.
fn accent() -> [f32; 4] {
    let a = color::highlight_primary_color();
    [a[0], a[1], a[2], 1.0]
}

/// `effect` drawn centred in `r`. Carves come after what they cut (here the well's floor
/// the gallery paints first) and are taken inside their footprint (`carve_inside`), so a
/// wall's outer half stays on the tile.
fn paint_effect(effect: Effect, r: Rect, pc: &mut PaintCtx) {
    use cce_ui::layout::carve_inside;
    use cce_ui::scene::paint::{Cap, ControlPlate, Field, PlateStance};
    use cce_ui::scene::{Frost, Material, PlateRole};
    let relief = cce_ui::layout::control_relief();
    let depth = cce_ui::layout::bevel_width();
    let (cx, cy) = (r.x + r.width * 0.5, r.y + r.height * 0.5);
    let sample = centred(r, SAMPLE_W, SAMPLE_H);
    let radius = cce_ui::layout::panel_corner_radius();
    match effect {
        Effect::PanePlate => {
            // A nested plate (no corner on the window's silhouette) of the pane rung, as
            // an InfoBox stands; flat, the InfoBox's themed surface in a hairline frame.
            let wall = depth.min(sample.height * 0.2);
            if relief {
                pc.plate_spec(&PlateSpec { rect: sample, material: Material::pane(), window_corners: (false, false, false, false), depth: wall });
            } else {
                let theme = color::active_theme();
                pc.border(sample, all(cce_ui::layout::plate_corner_radius()), theme.surface_bg, theme.surface_border, 1.0);
            }
        }
        Effect::Bevel => {
            // A widget's bordered plate (`append_widget_plate`): flat, its fill and border line.
            let m = face();
            if relief {
                pc.bevel(sample, all(radius), &m, depth.min(sample.height * 0.2));
            } else {
                pc.border(sample, all(radius), m.fill(PlateRole::Nested), color::active_theme().surface_border, 1.0);
            }
        }
        Effect::Boss => {
            // Under both styles: a boss is edges only, its face the surface below, so it
            // has no flat form to fall back to.
            let (plateau, radii) = carve_inside(sample, all(radius), depth);
            pc.boss(plateau, radii, depth);
        }
        Effect::ControlRaised | Effect::ControlFlush | Effect::ControlFlat | Effect::ControlFocused => {
            // A button-sized control plate with a face. Flat, every stance is the Flat one:
            // the face alone, and the focus ring drawn as a hairline.
            let rect = centred(r, SAMPLE_W, cce_ui::layout::button_height());
            let stance = match effect {
                Effect::ControlRaised if relief => PlateStance::Raised,
                Effect::ControlFlush | Effect::ControlFocused if relief => PlateStance::Flush,
                _ => PlateStance::Flat,
            };
            let tint = (effect == Effect::ControlFocused).then(ControlPlate::focus_tint);
            let plate = ControlPlate::control(rect, cce_ui::layout::button_corner_radius(), stance, Some(face())).with_tint(tint);
            pc.control_plate(&plate);
        }
        Effect::InsetPlate => {
            // A faceless flush plate — the seam alone, the floor its face — as an app's own
            // flush panel is cut; flat, the well's hairline.
            if relief {
                let wall = depth.min(sample.height * 0.2);
                let (plate, radii) = carve_inside(sample, all(radius), wall);
                pc.inset_plate(plate, radii, None, wall);
            } else {
                hairline(pc, sample, all(radius));
            }
        }
        Effect::Recess => {
            if relief {
                let (well, radii) = carve_inside(sample, all(radius), depth);
                pc.recess(well, radii, depth);
            } else {
                hairline(pc, sample, all(radius));
            }
        }
        Effect::Trough => {
            // Under both styles: no widget draws a trough, so there is no flat form of one.
            let (seam, radii) = carve_inside(sample, all(radius), depth);
            pc.trough(seam, radii, depth);
        }
        Effect::Ridge => {
            // Under both styles, as the trough.
            let (rim, radii) = carve_inside(sample, all(radius), depth);
            pc.ridge(rim, radii, depth);
        }
        Effect::FieldWell | Effect::FieldRun | Effect::FieldEndingInRun | Effect::FieldSlidingRun | Effect::FieldSpanning => {
            // A text box's height; the sliding run a toggle's pill, its run at the "on" end.
            let h = cce_ui::layout::textbox_height();
            let (w, round) = match effect {
                Effect::FieldSlidingRun => (90.0, h * 0.5),
                _ => (SAMPLE_W, cce_ui::layout::button_corner_radius()),
            };
            let wall = depth.min(h * 0.2);
            let (o, radii) = carve_inside(centred(r, w, h), all(round), wall);
            let field = match effect {
                Effect::FieldWell => Field::well(o, radii, wall),
                Effect::FieldRun => Field::run(o, radii, wall),
                Effect::FieldEndingInRun => Field::ending_in_run(o, radii, wall, o.x + o.width * 0.7),
                Effect::FieldSlidingRun => Field::sliding_run(o, radii, wall, o.width * 0.5, 1.0),
                _ => Field::spanning(o, radii, wall, o.x + o.width * 0.35, o.x + o.width * 0.65),
            };
            if relief {
                pc.field(&field);
            } else {
                // The Toggle's flat field: the well's hairline, and the run a lit face.
                hairline(pc, field.rect, field.radii);
                if let Some((a, b)) = field.run_span() {
                    let face = Rect { x: a + 2.0, y: o.y + 2.0, width: (b - a - 4.0).max(0.0), height: (o.height - 4.0).max(0.0) };
                    let lit = (0.16 * (cce_ui::layout::bevel_depth() / 0.15)).clamp(0.0, 0.5);
                    pc.rounded_rect(face, (radii.0 - 2.0).max(0.0), (true, true, true, true), [1.0, 1.0, 1.0, lit]);
                }
            }
        }
        Effect::CanvasWell => {
            // The floor, then the content running to its edges, clipped to the well's shape
            // as the ColorRamp's gradient is, then the rim over it, at the panel radius — the
            // Trackpad's and the Ramp's well. `well_rim` draws the flat hairline itself.
            pc.well_floor(sample, radius, &Material::pane(), false);
            let n = 32;
            let at = |i: usize| {
                let t = i as f32 / n as f32;
                (sample.x + t * sample.width, cy - (t * std::f32::consts::TAU).sin() * sample.height * 0.3)
            };
            pc.clip_rounded(sample, radius, |pc| {
                for i in 0..n {
                    let ((x0, y0), (x1, y1)) = (at(i), at(i + 1));
                    pc.vector(x0, y0, x1, y1, 2.0, accent(), Cap::Round);
                }
            });
            pc.well_rim(sample, radius, relief);
        }
        Effect::SectionWell => {
            // A titled section as a Group draws its frame: the body and the title tab on its
            // top edge carved as one well; flat, the hairline outline parted for the title.
            let (family, size) = cce_ui::layout::parse_font_string(&cce_ui::layout::section_label_font());
            let size = size.unwrap_or(14.0);
            let title = "Section";
            let tab_h = size + 8.0;
            let body = centred(r, SAMPLE_W + 10.0, SAMPLE_H - tab_h);
            let body = Rect { y: body.y + tab_h * 0.5, ..body };
            let tw = cce_ui::widget::display::measure_text_width(title, &family, size) + 16.0;
            let tab = Rect { x: body.x, y: body.y - tab_h, width: tw.min(body.width), height: tab_h };
            if relief {
                let wall = (depth * cce_ui::layout::section_depth()).min(body.height * 0.2).max(0.5);
                pc.section_well(body, Some(tab), all(radius), wall);
            } else {
                let c = [0.25, 0.25, 0.35, 1.0];
                let (x0, y0, x1, y1) = (body.x, body.y, body.x + body.width, body.y + body.height);
                let (gap0, gap1) = (tab.x + 2.0, tab.x + tab.width - 2.0);
                if gap0 > x0 {
                    pc.vector(x0, y0, gap0, y0, 1.0, c, Cap::Flat);
                }
                pc.vector(gap1, y0, x1, y0, 1.0, c, Cap::Flat);
                pc.vector(x0, y1, x1, y1, 1.0, c, Cap::Flat);
                pc.vector(x0, y0, x0, y1, 1.0, c, Cap::Flat);
                pc.vector(x1, y0, x1, y1, 1.0, c, Cap::Flat);
            }
            let ty = if relief { tab.y + 4.0 } else { tab.y + tab.height - size - 2.0 };
            let color = color::control_label_color_for_state(false, false);
            pc.text_with(title.to_string(), tab.x + 8.0, ty, size, color, Some(format!("{family} {size}")), None);
        }
        Effect::ConcaveFillet => {
            // Under both styles: a fillet only rounds a composed carve's inside corner, which
            // no widget draws flat. Two inside corners, as a section's throat is rounded: a
            // wall down from the top meeting one along to the right, the fillet between them
            // where their tangents end — cut down at the left, raised at the right.
            let rho = 16.0;
            let half_w = r.width * 0.5;
            let foot = r.y + r.height - caption_h();
            for (k, raised) in [(0.0, false), (1.0, true)] {
                let x0 = r.x + k * half_w;
                let (xw, yh) = (x0 + half_w * 0.35, r.y + (foot - r.y) * 0.65);
                let (top, right) = (r.y + 10.0, x0 + half_w - 10.0);
                // Each straight wall runs a wall's depth past its tangent, as `section_well`'s do.
                let upright = Rect { x: xw - 24.0, y: top, width: 24.0, height: (yh - rho) - top + depth };
                let along = Rect { x: xw + rho - depth, y: yh, width: right - (xw + rho - depth), height: 24.0 };
                if raised {
                    pc.boss_edges(upright, all(0.0), depth, (false, true, false, false));
                    pc.boss_edges(along, all(0.0), depth, (true, false, false, false));
                } else {
                    pc.recess_edges(upright, all(0.0), depth, (false, true, false, false));
                    pc.recess_edges(along, all(0.0), depth, (true, false, false, false));
                }
                pc.concave_fillet(xw + rho, yh - rho, rho, depth, std::f32::consts::FRAC_PI_2, raised);
                caption(pc, if raised { "raised" } else { "carved" }, x0 + half_w * 0.5, foot);
            }
        }
        Effect::Groove => {
            // A Separator's groove, and a Breadcrumb's slanted seam: an engraved line fading
            // into the host it is cut into; flat, the Separator's rule.
            let gd = cce_ui::widget::Separator::groove_depth();
            let reach = 3.0 * gd;
            let width = cce_ui::widget::Breadcrumb::SEAM_WIDTH;
            let (x0, x1) = (sample.x, sample.x + sample.width);
            let y = sample.y + 16.0;
            let (a, b) = ((cx - 30.0, sample.y + sample.height), (cx + 30.0, sample.y + 34.0));
            if relief {
                pc.groove((x0, y), (x1, y), width, gd, Rect { x: x0, y: y - reach, width: x1 - x0, height: 2.0 * reach });
                let host = Rect { x: a.0 - reach, y: b.1, width: b.0 - a.0 + 2.0 * reach, height: a.1 - b.1 };
                pc.groove(a, b, width, gd, host);
            } else {
                pc.quad(Rect { x: x0, y: y - 0.5, width: x1 - x0, height: 1.0 }, RULE_COLOR);
                pc.vector(a.0, a.1, b.0, b.1, 1.0, RULE_COLOR, Cap::Flat);
            }
        }
        Effect::CarveUnion => {
            // Under both styles: no widget carves a union. An L, its two boxes one well with
            // one wall, the joint a mitre.
            let upright = Rect { x: sample.x + 20.0, y: sample.y, width: 40.0, height: sample.height };
            let foot = Rect { x: sample.x + 20.0, y: sample.y + sample.height - 36.0, width: sample.width - 40.0, height: 36.0 };
            pc.carve_union(vec![(upright, all(8.0)), (foot, all(8.0))], depth, false);
        }
        Effect::Lattice | Effect::Grout => {
            // A grid of cells, as the desktop carves its tiles (lattice) and a graph lays its
            // grid lines (grout). Under both styles: neither has a flat form (grout is flat
            // already).
            let cell = (22.0, 20.0);
            let period = (cell.0 + 8.0, cell.1 + 8.0);
            if effect == Effect::Lattice {
                pc.lattice(sample, period, (cx, cy), cell, 4.0, 3.0);
            } else {
                pc.grout(sample, period, (cx, cy), cell, 4.0, [0.25, 0.25, 0.35, 1.0]);
            }
        }
        Effect::Frame => {
            // A plate with an opening cut into its top edge, its inside corners coves — the
            // designer's playbar shelf, seen through the sample as through its window. As
            // there, the face's own edges, which are not rolled, run out past the view on both
            // sides and below, and the opening spans the view: the only edge on screen is the
            // opening's rolled outline, the shelf's top turning up into each side in a cove.
            // Flat, as there, the same face with no roll.
            let cove = cce_ui::layout::plate_corner_radius().min(16.0);
            let over = color::plate_bevel_width() + 1.0;
            let top = cy - 4.0;
            let band = Rect { x: sample.x - over, y: top, width: sample.width + 2.0 * over, height: sample.y + sample.height + over - top };
            let hole = Rect { x: sample.x, y: sample.y - over, width: sample.width, height: top + cove - (sample.y - over) };
            let d = if relief { color::plate_bevel_width() } else { 0.0 };
            pc.clip(sample, |pc| pc.frame(band, hole, (0.0, 0.0, cove, cove), &face(), d));
        }
        Effect::DiscPlate => {
            // A plate drawn as a true circle (`Some(2.0)`: circular arcs, its radii half its
            // side), the shape of a Ramp's key peg, on the bevel's face (`face`): a plate
            // colour reads its roll plainly, where the peg's accent glass over the bare
            // floor showed it faintly. Flat, a flat disc of that face.
            let rr = 30.0;
            let m = face();
            if relief {
                let disc = Rect { x: cx - rr, y: cy - rr, width: 2.0 * rr, height: 2.0 * rr };
                pc.plate_shaped(disc, all(rr), &m, depth.min(0.4 * rr), Some(2.0));
            } else {
                pc.circle(cx, cy, rr, m.fill(PlateRole::Nested));
            }
        }
        Effect::Sphere => {
            // A RadioGroup's bead: the disc a flat host keeps, the lit ball over it. Under
            // both styles, as the radio draws it.
            let rr = 30.0;
            pc.circle(cx, cy, rr, accent());
            pc.sphere(cx, cy, rr, &Material::from_fill(accent()));
        }
        Effect::Droplet => {
            // The status bar's drop (`DropletSpec::default()`, its finish the spec's), hanging
            // from the top of a striped band so its clearer rim shows against the stripes, and
            // inset at the foot for its contact shadow as the bar insets it. Under both styles:
            // the bar draws its drops whatever the relief.
            let stripes = (sample.width / 16.0).floor() as usize;
            for i in 0..stripes {
                let c = if i % 2 == 0 { [0.85, 0.85, 0.88, 1.0] } else { [0.05, 0.05, 0.07, 1.0] };
                pc.quad(Rect { x: sample.x + i as f32 * 16.0, y: sample.y, width: 16.0, height: sample.height }, c);
            }
            use cce_ui::scene::paint::{DropletFinish, DropletSpec};
            let spec = DropletSpec::default();
            let drop = centred(sample, 110.0, 56.0);
            let drop = Rect { y: sample.y, height: drop.height - 1.0 - spec.shadow_gap(drop.height), ..drop };
            let water = Material::from_fill([0.1, 0.1, 0.12, 0.6]).with_finish(spec.finish());
            pc.droplet(drop, &water, spec);
        }
        Effect::Materials => {
            // Three nested plates over stripes, so a material's coverage shows: opaque, a
            // translucent tint (the stripes sharp through it), frosted glass (blurred and
            // pulled toward its tint). Under both styles: a material is no relief.
            let foot = r.y + r.height - caption_h();
            let band = Rect { x: r.x + 8.0, y: r.y, width: r.width - 16.0, height: foot - r.y - 4.0 };
            let stripes = (band.width / 16.0).floor() as usize;
            for i in 0..stripes {
                let c = if i % 2 == 0 { [0.85, 0.85, 0.88, 1.0] } else { [0.05, 0.05, 0.07, 1.0] };
                pc.quad(Rect { x: band.x + i as f32 * 16.0, y: band.y, width: 16.0, height: band.height }, c);
            }
            let samples = [
                ("opaque", Material::opaque([0.18, 0.22, 0.32, 1.0])),
                ("translucent", Material::opaque([0.18, 0.22, 0.32, 0.45])),
                (
                    "frosted",
                    Material::opaque([0.0015, 0.0015, 0.0024, 0.25])
                        .with_frost(Frost::Frosted { compression: 0.6, refraction: 0.0, radius: Frost::DEFAULT_RADIUS }),
                ),
            ];
            let gap = 16.0;
            let w = (band.width - 4.0 * gap) / 3.0;
            for (k, (name, material)) in samples.into_iter().enumerate() {
                let rect = Rect { x: band.x + gap + k as f32 * (w + gap), y: band.y + 12.0, width: w, height: band.height - 24.0 };
                pc.plate_spec(&PlateSpec { rect, material, window_corners: (false, false, false, false), depth: depth.min(rect.height * 0.2) });
                caption(pc, name, rect.x + w * 0.5, foot);
            }
        }
        Effect::Depths => {
            // One recess at three wall widths; flat, each the well's hairline.
            let foot = r.y + r.height - caption_h();
            let gap = 24.0;
            let w = (r.width - 4.0 * gap) / 3.0;
            let h = (foot - r.y - 16.0).min(80.0);
            for (k, d) in [2.0, 4.0, 8.0].into_iter().enumerate() {
                let rect = Rect { x: r.x + gap + k as f32 * (w + gap), y: foot - 6.0 - h, width: w, height: h };
                if relief {
                    let (well, radii) = carve_inside(rect, all(radius), d);
                    pc.recess(well, radii, d);
                } else {
                    hairline(pc, rect, all(radius));
                }
                caption(pc, &format!("{d} px"), rect.x + w * 0.5, foot);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cce_ui::scene::paint::Prim;
    use cce_ui::scene::PlateRole;

    /// The disc plate is drawn on the bevel's face (`face`), not the accent: a lit plate of
    /// that material, round, under relief; flat, a disc of its fill.
    #[test]
    fn the_disc_plate_wears_the_bevels_face() {
        let mut pc = PaintCtx::new();
        paint_effect(Effect::DiscPlate, Rect { x: 0.0, y: 0.0, width: 190.0, height: Effect::DiscPlate.height() }, &mut pc);
        let items = pc.finish().items;
        let disc = items.iter().find_map(|it| match &it.prim {
            Prim::Plate { material, shape: Some(_), .. } => Some(Err(*material)),
            Prim::Circle { color, .. } => Some(Ok(*color)),
            _ => None,
        });
        match disc.expect("the tile paints a disc") {
            Err(material) => assert_eq!(material, face(), "relief"),
            Ok(color) => assert_eq!(color, face().fill(PlateRole::Nested), "flat"),
        }
    }
}
