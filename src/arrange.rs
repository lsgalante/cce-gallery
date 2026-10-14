//! Where the exhibits go: the five arrangements the `Layout` dropdown offers, as pure
//! functions from each exhibit's [`Block`] to the rect its CONTENT lands in.
//!
//! A block is a widget as an arrangement sees it: its footprint width (the control, or its
//! label where that runs wider), its content width and height (the control below its
//! detached label) and its label strip (the label's row above the content). Mosaic and Reverse
//! Mosaic, which pack blocks side by side, reserve the tallest strip among the blocks — the
//! `lead` — above EVERY block's content, so labeled and unlabeled controls packed beside each
//! other line up by content. Vertical and Adaptive Grid stack blocks in columns, where nothing
//! stands beside a block to line up with: each block reserves its own strip and no more, so
//! the gap down a column is the same above a labeled block and an unlabeled one. Every
//! arrangement puts `gap` between footprints in both axes. [`arrange`] returns each block's content rect (its
//! label hangs in the strip above it) and the height the arrangement takes, which the
//! exhibit area scrolls over.
//!
//! A [`Lasso`] (a `Group` drawn around some of the blocks) is arranged as ONE block: its
//! members are laid out together inside it, each under its own label strip alone (no shared
//! label row inside a group), and the block is their footprint, with its title tab in the
//! strip above. The group's frame — the padded hull of wherever its members land — hangs one
//! padding outside that block on every side, into the gaps around it, so it never takes in an
//! exhibit that is not its own, its padding is the same on all four sides, and its members
//! start in the column every other exhibit does.
//!
//! These began as cce-ui's `ContainerLayout` strategies, which the gallery was the one user
//! of; they live here as arithmetic so the gallery lands its widgets itself
//! (`layout_exhibits`). The quirks they kept from the toolkit are noted on their arms.

use cce_ui::scene::layout::Rect;
use cce_ui::widget::{WidgetHost, WidgetHostExt};

/// One exhibit as an arrangement sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// The block's footprint: what Mosaic packs and every arrangement keeps `gap` between.
    /// At least the content width, wider where the detached label runs past the control.
    pub width: f32,
    /// The control's own width, which a block that does not stretch is always laid out at.
    pub content_w: f32,
    /// The content height below the label strip; 0 or less means "none", and the
    /// arrangements that stack blocks give it [`FALLBACK_H`].
    pub content_h: f32,
    /// The detached-label strip above the content (0 unlabeled).
    pub strip: f32,
    /// Whether Vertical and Adaptive Grid may widen the content to their cell. A control with
    /// a size of its own — a grip, a dot, a square pad — keeps its width in the cell.
    pub stretch: bool,
}

impl Block {
    /// A widget's block, read off it after it is seeded at its exhibit size: the width it
    /// landed at, widened to its detached label's extent; its declared content height (else
    /// its rect less the strip) and its strip.
    pub fn of(w: &dyn WidgetHost, stretch: bool) -> Block {
        let (x, _, width, h) = w.rect();
        let strip = w.label_strip();
        let label_w = w.detached_label_rect().map_or(0.0, |l| l.x + l.width - x);
        Block { width: width.max(label_w), content_w: width, content_h: w.preferred_height().unwrap_or(h - strip), strip, stretch }
    }
}

/// A `Group` drawn around some of the blocks, arranged as one block (see the module doc).
#[derive(Debug, Clone, PartialEq)]
pub struct Lasso {
    /// Indexes into the blocks, in the order they are laid out inside the lasso.
    pub members: Vec<usize>,
    /// The frame's inset around its members' hull.
    pub padding: f32,
    /// The title tab's height above the frame (0 untitled): the lasso block's strip.
    pub tab: f32,
}

/// The content height a stacking arrangement gives a block that declares none.
pub const FALLBACK_H: f32 = 44.0;

/// The arrangements, as the `Layout` dropdown lists them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arrangement {
    /// One column, every stretching block widened to the area's width.
    Vertical,
    /// Masonry over as many columns of at least `min_col_width` as fit — fewer where the
    /// widest block would not fit in one: each block goes to the shortest column.
    AdaptiveGrid { min_col_width: f32 },
    /// Blocks packed at their own widths, top-left first.
    Mosaic,
    /// Mosaic's packing spread, unevenly, over the whole area: the blocks' places and the
    /// stretching blocks' widths scale to fill it, every height stays the block's own.
    ReverseMosaic,
}

/// Lay `blocks` out in `area` with `gap` between them, each of `lassos` as one block: each
/// block's content rect, in order, and the height the arrangement takes.
pub fn arrange(kind: Arrangement, blocks: &[Block], lassos: &[Lasso], area: Rect, gap: f32) -> (Vec<Rect>, f32) {
    // The units the arrangement places: every block outside a lasso, and each lasso at
    // the place of its first member.
    enum Unit {
        One(usize),
        Lasso(usize),
    }
    let mut owner = vec![None; blocks.len()];
    for (l, lasso) in lassos.iter().enumerate() {
        for &m in &lasso.members {
            owner[m] = Some(l);
        }
    }
    let mut units = Vec::new();
    let mut placed = vec![false; lassos.len()];
    for (i, o) in owner.iter().enumerate() {
        match *o {
            None => units.push(Unit::One(i)),
            Some(l) if !placed[l] => {
                placed[l] = true;
                units.push(Unit::Lasso(l));
            }
            Some(_) => {}
        }
    }
    // Each lasso's members laid out inside it — stacked under Vertical, else in one row
    // if they fit across it and stacked if not —
    // at the width of the cell the lasso will get, from the block's top-left. Each member
    // reserves its own label strip and no one else's: a block of content and strip together.
    let columns = grid_columns(kind, blocks, area, gap);
    let cell = match kind {
        Arrangement::AdaptiveGrid { .. } => column_width(area, gap, columns),
        Arrangement::Vertical | Arrangement::Mosaic | Arrangement::ReverseMosaic => area.width.max(1.0),
    };
    let inner: Vec<(Vec<Rect>, Block)> = lassos
        .iter()
        .map(|lasso| {
            let inner_w = cell.max(1.0);
            let members: Vec<Block> = lasso.members.iter().map(|&m| blocks[m]).collect();
            let whole: Vec<Block> = members.iter().map(|b| Block { content_h: stacked_h(b) + b.strip, strip: 0.0, ..*b }).collect();
            // Stacked under Vertical; else one row or one each (`flow`: Mosaic's packer
            // would put a member taller than the one before it on a row of its own).
            let at = Rect { x: 0.0, y: 0.0, width: inner_w, height: 0.0 };
            let placed = match kind {
                Arrangement::Vertical => arrange_blocks(Arrangement::Vertical, &whole, at, gap, 1).0,
                _ => flow(&whole, at, gap),
            };
            // Each member's content rect: its block less its own strip.
            let rects: Vec<Rect> = members
                .iter()
                .zip(&placed)
                .map(|(b, r)| Rect { y: r.y + b.strip, height: r.height - b.strip, ..*r })
                .collect();
            let w = members.iter().zip(&rects).map(|(b, r)| r.x + r.width.max(b.width.min(inner_w))).fold(0.0, f32::max);
            let h = rects.iter().map(|r| r.y + r.height).fold(0.0, f32::max);
            (rects, Block { width: w, content_w: w, content_h: h, strip: lasso.tab, stretch: true })
        })
        .collect();
    let unit_blocks: Vec<Block> = units
        .iter()
        .map(|u| match *u {
            Unit::One(i) => blocks[i],
            Unit::Lasso(l) => inner[l].1,
        })
        .collect();
    let (unit_rects, height) = arrange_blocks(kind, &unit_blocks, area, gap, columns);
    let mut rects = vec![Rect { x: 0.0, y: 0.0, width: 0.0, height: 0.0 }; blocks.len()];
    for (u, ur) in units.iter().zip(unit_rects) {
        match *u {
            Unit::One(i) => rects[i] = ur,
            Unit::Lasso(l) => {
                // Reverse Mosaic widens the lasso's block like any other stretching block; its
                // members spread across it, so they stay inside it. Every
                // arrangement gives the block the height it asked for, and every other one a
                // cell at least as wide as the members were laid out for.
                let kx = match kind {
                    Arrangement::ReverseMosaic => ur.width / inner[l].1.width,
                    _ => 1.0,
                };
                for (&m, mr) in lassos[l].members.iter().zip(&inner[l].0) {
                    let width = if blocks[m].stretch { mr.width * kx } else { mr.width };
                    rects[m] = Rect { x: ur.x + mr.x * kx, y: ur.y + mr.y, width, height: mr.height };
                }
            }
        }
    }
    (rects, height)
}

/// How many columns of at least `width` fit in `area`, one at the least.
fn columns_of(area: Rect, gap: f32, width: f32) -> usize {
    ((area.width.max(1.0) + gap) / (width + gap)).floor().max(1.0) as usize
}

/// Adaptive Grid's column count: as many of the minimum width as fit, but never so many
/// that the widest block (a footprint, label included) runs past its column into the
/// next. 1 for the arrangements without columns.
fn grid_columns(kind: Arrangement, blocks: &[Block], area: Rect, gap: f32) -> usize {
    let asked = match kind {
        Arrangement::AdaptiveGrid { min_col_width } => columns_of(area, gap, min_col_width),
        Arrangement::Vertical | Arrangement::Mosaic | Arrangement::ReverseMosaic => return 1,
    };
    let widest = blocks.iter().map(|b| b.width).fold(0.0, f32::max);
    asked.min(columns_of(area, gap, widest))
}

fn column_width(area: Rect, gap: f32, columns: usize) -> f32 {
    let cols = columns.max(1);
    (area.width - gap * (cols - 1) as f32).max(1.0) / cols as f32
}

/// [`arrange`] without lassos: every block placed on its own, Adaptive Grid over `columns`.
fn arrange_blocks(kind: Arrangement, blocks: &[Block], area: Rect, gap: f32, columns: usize) -> (Vec<Rect>, f32) {
    let lead = blocks.iter().map(|b| b.strip).fold(0.0, f32::max);
    match kind {
        Arrangement::Vertical | Arrangement::AdaptiveGrid { .. } => {
            // Stacked in columns: each block's strip is its own, part of its height, and no
            // lead is shared — a lead row over an unlabeled block was 22 px of nothing, and
            // the gap above it nearly twice the gap above a labeled one.
            let whole: Vec<Block> = blocks.iter().map(|b| Block { content_h: stacked_h(b) + b.strip, strip: 0.0, ..*b }).collect();
            let (rects, h) = match kind {
                Arrangement::Vertical => vertical(&whole, area, gap, 0.0),
                _ => grid(&whole, area, gap, 0.0, columns),
            };
            let content = blocks.iter().zip(rects).map(|(b, r)| Rect { y: r.y + b.strip, height: r.height - b.strip, ..r }).collect();
            (content, h)
        }
        Arrangement::Mosaic => mosaic(blocks, area, gap, lead),
        Arrangement::ReverseMosaic => reverse_mosaic(blocks, area, gap, lead),
    }
}

/// Blocks in ONE row, left to right, each at its row's top and at its own size, when they all
/// fit across the area; else one to a row, each at the area's left edge. What a lasso's
/// members are laid out by outside Vertical. Not a row that wraps wherever the widths happen
/// to break: that put one member on a row, two on the next and one on the last, the middle
/// one lined up with nothing, and the breaks moved with the window.
fn flow(blocks: &[Block], area: Rect, gap: f32) -> Vec<Rect> {
    let avail = area.width.max(1.0);
    let row_w: f32 = blocks.iter().map(|b| b.width.min(avail)).sum::<f32>() + gap * blocks.len().saturating_sub(1) as f32;
    let one_row = row_w <= avail;
    let (mut x, mut y) = (area.x, area.y);
    let mut out = Vec::with_capacity(blocks.len());
    for b in blocks {
        let (w, h) = (b.width.min(avail), stacked_h(b));
        out.push(Rect { x, y, width: b.content_w.min(avail), height: h });
        if one_row {
            x += w + gap;
        } else {
            y += h + gap;
        }
    }
    out
}

/// A block's content height, [`FALLBACK_H`] when it declares none.
fn stacked_h(b: &Block) -> f32 {
    if b.content_h > 0.0 { b.content_h } else { FALLBACK_H }
}

/// The content width a block gets in a cell `cell` wide: the cell's if it stretches, else
/// its own, clamped to the cell.
fn in_cell(b: &Block, cell: f32) -> f32 {
    if b.stretch { cell } else { b.content_w.min(cell) }
}

/// The height counts the gap after the last block too.
fn vertical(blocks: &[Block], area: Rect, gap: f32, lead: f32) -> (Vec<Rect>, f32) {
    let width = area.width.max(1.0);
    let mut y = area.y;
    let mut rects = Vec::with_capacity(blocks.len());
    for b in blocks {
        let h = stacked_h(b);
        rects.push(Rect { x: area.x, y: y + lead, width: in_cell(b, width), height: h });
        y += lead + h + gap;
    }
    (rects, (y - area.y).max(0.0))
}

/// A tie for the shortest column goes to the leftmost; the height is measured from the
/// tallest column's bottom, gap included.
fn grid(blocks: &[Block], area: Rect, gap: f32, lead: f32, columns: usize) -> (Vec<Rect>, f32) {
    if blocks.is_empty() {
        return (Vec::new(), 0.0);
    }
    let cols = columns.max(1);
    let col_w = column_width(area, gap, cols);
    let mut bottoms = vec![area.y; cols];
    let mut rects = Vec::with_capacity(blocks.len());
    for b in blocks {
        let h = stacked_h(b);
        let mut col = 0;
        for i in 1..cols {
            if bottoms[i] < bottoms[col] {
                col = i;
            }
        }
        let x = area.x + col as f32 * (col_w + gap);
        rects.push(Rect { x, y: bottoms[col] + lead, width: in_cell(b, col_w), height: h });
        bottoms[col] += lead + h + gap;
    }
    let bottom = bottoms.iter().copied().fold(0.0, f32::max);
    (rects, (bottom - area.y).max(0.0))
}

/// Packs footprints; a block's content is its own width, clamped to the area's (its packed
/// footprint is not).
fn mosaic(blocks: &[Block], area: Rect, gap: f32, lead: f32) -> (Vec<Rect>, f32) {
    if blocks.is_empty() {
        return (Vec::new(), 0.0);
    }
    let avail = area.width.max(1.0);
    let mut packer = Packer::new(area.x, area.y, avail, gap);
    let mut rects = Vec::with_capacity(blocks.len());
    for b in blocks {
        let h = stacked_h(b);
        let (x, y) = packer.pack(b.width, lead + h);
        rects.push(Rect { x, y: y + lead, width: b.content_w.min(avail), height: h });
    }
    (rects, (packer.max_h - area.y).max(0.0))
}

/// Packs as Mosaic from the origin, then spreads the packing's bounding box over the area:
/// the blocks' places scale, x and y by their own factors, and so do the unclamped
/// footprints of the stretching blocks (a block that does not stretch keeps its own width).
/// Heights do not: every block keeps its own, under the lead, and the vertical factor is
/// never below 1 — a packing taller than the area keeps Mosaic's rows and runs past it to
/// scroll, rather than being squashed into it. Scaling places by 1 or more only widens the
/// gaps the packing left, so blocks that did not overlap packed do not overlap spread.
fn reverse_mosaic(blocks: &[Block], area: Rect, gap: f32, lead: f32) -> (Vec<Rect>, f32) {
    if blocks.is_empty() {
        return (Vec::new(), 0.0);
    }
    let avail = area.width.max(1.0);
    let mut packer = Packer::new(0.0, 0.0, avail, gap);
    let packed: Vec<(f32, f32, f32, f32)> = blocks
        .iter()
        .map(|b| {
            let ph = lead + stacked_h(b);
            let (px, py) = packer.pack(b.width, ph);
            (px, py, b.width, ph)
        })
        .collect();
    let (mut x_min, mut x_max, mut y_min, mut y_max) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(px, py, pw, ph) in &packed {
        x_min = x_min.min(px);
        x_max = x_max.max(px + pw);
        y_min = y_min.min(py);
        y_max = y_max.max(py + ph);
    }
    let sx = avail / (x_max - x_min).max(1.0);
    let sy = (area.height.max(1.0) / (y_max - y_min).max(1.0)).max(1.0);
    let rects = blocks
        .iter()
        .zip(&packed)
        .map(|(b, &(px, py, pw, ph))| Rect {
            x: area.x + (px - x_min) * sx,
            y: area.y + (py - y_min) * sy + lead,
            width: if b.stretch { pw * sx } else { b.content_w },
            height: ph - lead,
        })
        .collect();
    (rects, (y_max - y_min) * sy)
}

/// A guillotine packer over free rects: each block takes the topmost (then leftmost) free
/// rect it fits, which splits into what is left beside it and below it.
struct Packer {
    /// `(x, y, w, h)`.
    free: Vec<(f32, f32, f32, f32)>,
    max_w: f32,
    /// The lowest bottom packed so far (from 0, not from the start).
    max_h: f32,
    gap: f32,
}

impl Packer {
    /// The free space is one rect `max_width` wide and practically bottomless.
    fn new(x: f32, y: f32, max_width: f32, gap: f32) -> Self {
        Packer { free: vec![(x, y, max_width, 100000.0)], max_w: max_width, max_h: 0.0, gap }
    }

    /// Where a `w` x `h` block goes. A block that fits nowhere opens a row below everything
    /// at the FIRST free rect's x, which need not be the leftmost.
    fn pack(&mut self, w: f32, h: f32) -> (f32, f32) {
        let w = w.min(self.max_w);
        let mut best = None;
        let (mut best_y, mut best_x) = (f32::MAX, f32::MAX);
        for (i, &(rx, ry, rw, rh)) in self.free.iter().enumerate() {
            if rw >= w && rh >= h && (ry < best_y || (ry == best_y && rx < best_x)) {
                best_y = ry;
                best_x = rx;
                best = Some(i);
            }
        }
        let chosen = match best {
            Some(i) => i,
            None => {
                self.free.push((self.free[0].0, self.max_h + self.gap, self.max_w, 100000.0));
                self.free.len() - 1
            }
        };
        let (fx, fy, fw, fh) = self.free.remove(chosen);
        let right_w = fw - w - self.gap;
        if right_w > 0.0 && h > 0.0 {
            self.free.push((fx + w + self.gap, fy, right_w, h));
        }
        let below_h = fh - h - self.gap;
        if below_h > 0.0 && fw > 0.0 {
            self.free.push((fx, fy + h + self.gap, fw, below_h));
        }
        self.max_h = self.max_h.max(fy + h);
        (fx, fy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(width: f32, content_h: f32) -> Block {
        Block { width, content_w: width, content_h, strip: 0.0, stretch: true }
    }

    fn labeled(width: f32, content_h: f32, strip: f32) -> Block {
        Block { strip, ..block(width, content_h) }
    }

    /// Place blocks with no lassos.
    fn place(kind: Arrangement, blocks: &[Block], a: Rect, gap: f32) -> (Vec<Rect>, f32) {
        arrange(kind, blocks, &[], a, gap)
    }

    fn overlaps(a: Rect, b: Rect) -> bool {
        a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
    }

    fn area(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect { x, y, width, height }
    }

    /// cce-ui's `test_mosaic_layout` golden numbers (padding 5 is the area inset by 5).
    #[test]
    fn mosaic_packs_top_left_first() {
        let blocks = [block(100.0, 50.0), block(100.0, 80.0), block(80.0, 40.0)];
        let (r, _) = place(Arrangement::Mosaic, &blocks, area(15.0, 25.0, 240.0, 490.0), 10.0);
        assert_eq!((r[0].x, r[0].y), (15.0, 25.0));
        assert_eq!((r[1].x, r[1].y), (15.0, 85.0));
        assert_eq!((r[2].x, r[2].y), (125.0, 25.0));
    }

    /// The packing (0,0 100x50; 0,60 100x80; 110,0 80x40 — 190 by 140) spread over 240 by
    /// 290: places and widths by 240/190, places by 290/140, every height its own.
    #[test]
    fn reverse_mosaic_spreads_the_packing_over_the_area() {
        let blocks = [block(100.0, 50.0), block(100.0, 80.0), block(80.0, 40.0)];
        let (r, h) = place(Arrangement::ReverseMosaic, &blocks, area(15.0, 25.0, 240.0, 290.0), 10.0);
        assert_eq!(h, 290.0);
        assert_eq!((r[0].x, r[0].y), (15.0, 25.0));
        assert!((r[0].width - 126.315).abs() < 0.01, "{:?}", r[0]);
        assert!((r[1].y - (25.0 + 60.0 * 290.0 / 140.0)).abs() < 0.01, "{:?}", r[1]);
        assert!((r[2].x - 153.947).abs() < 0.01, "{:?}", r[2]);
        assert_eq!(r[2].y, 25.0);
        assert!((r[2].width - 101.05).abs() < 0.01, "{:?}", r[2]);
        assert_eq!([r[0].height, r[1].height, r[2].height], [50.0, 80.0, 40.0]);
    }

    /// A packing taller than the area keeps Mosaic's rows and its own height, to scroll: the
    /// widths still spread, nothing is squashed.
    #[test]
    fn reverse_mosaic_never_squashes_a_tall_packing() {
        let blocks = [block(100.0, 50.0), block(100.0, 80.0), block(80.0, 40.0)];
        let (r, h) = place(Arrangement::ReverseMosaic, &blocks, area(15.0, 25.0, 240.0, 100.0), 10.0);
        assert_eq!(h, 140.0);
        assert_eq!(r[1].y, 25.0 + 60.0);
        assert_eq!([r[0].height, r[1].height, r[2].height], [50.0, 80.0, 40.0]);
        assert!((r[0].width - 126.315).abs() < 0.01, "{:?}", r[0]);
    }

    #[test]
    fn a_block_without_a_height_gets_the_fallback() {
        let blocks = [block(100.0, 0.0), block(100.0, -3.0)];
        for kind in [Arrangement::Vertical, Arrangement::AdaptiveGrid { min_col_width: 100.0 }, Arrangement::Mosaic] {
            let (r, _) = place(kind, &blocks, area(0.0, 0.0, 400.0, 400.0), 10.0);
            assert!(r.iter().all(|r| r.height == FALLBACK_H), "{kind:?}: {r:?}");
        }
    }

    #[test]
    fn vertical_counts_the_trailing_gap() {
        let blocks = [labeled(50.0, 20.0, 6.0), block(50.0, 30.0)];
        let (r, h) = place(Arrangement::Vertical, &blocks, area(5.0, 10.0, 200.0, 0.0), 4.0);
        // Each block keeps its own strip: the labeled one its 6, the unlabeled one none.
        assert_eq!(r, vec![area(5.0, 16.0, 200.0, 20.0), area(5.0, 40.0, 200.0, 30.0)]);
        assert_eq!(h, (6.0 + 20.0 + 4.0) + (30.0 + 4.0));
    }

    #[test]
    fn grid_ties_go_to_the_leftmost_column() {
        let blocks = [block(1.0, 10.0), block(1.0, 10.0), block(1.0, 30.0), block(1.0, 5.0)];
        // (90 + 10) / (40 + 10) = 2 columns.
        let (r, h) = place(Arrangement::AdaptiveGrid { min_col_width: 40.0 }, &blocks, area(0.0, 0.0, 90.0, 0.0), 10.0);
        let xy: Vec<_> = r.iter().map(|r| (r.x, r.y)).collect();
        assert_eq!(xy, vec![(0.0, 0.0), (50.0, 0.0), (0.0, 20.0), (50.0, 20.0)]);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn adaptive_grid_fits_as_many_columns_as_the_minimum_allows() {
        let blocks = [block(1.0, 10.0); 4];
        let (r, _) = place(Arrangement::AdaptiveGrid { min_col_width: 100.0 }, &blocks, area(0.0, 0.0, 320.0, 0.0), 10.0);
        // (320 + 10) / (100 + 10) = 3 columns.
        let xs: Vec<_> = r.iter().map(|r| r.x).collect();
        assert_eq!(xs, vec![0.0, 110.0, 220.0, 0.0]);
    }

    /// A block too tall for any free rect opens a row at the first free rect's x — here the
    /// space beside the first block, not the area's left edge.
    #[test]
    fn mosaic_opens_a_row_at_the_first_free_rect() {
        let blocks = [block(50.0, 20.0), block(50.0, 200000.0)];
        let (r, _) = place(Arrangement::Mosaic, &blocks, area(0.0, 0.0, 200.0, 0.0), 10.0);
        assert_eq!((r[1].x, r[1].y), (60.0, 30.0));
    }

    #[test]
    fn nothing_to_arrange() {
        let a = area(1.0, 2.0, 30.0, 40.0);
        for kind in [
            Arrangement::Vertical,
            Arrangement::AdaptiveGrid { min_col_width: 10.0 },
            Arrangement::Mosaic,
            Arrangement::ReverseMosaic,
        ] {
            assert_eq!(place(kind, &[], a, 5.0), (Vec::new(), 0.0), "{kind:?}");
        }
    }

    /// A label wider than its control widens the block's footprint, not the control: the
    /// next block packs past the label, and the control keeps its own width.
    #[test]
    fn a_wide_label_widens_the_footprint() {
        let grip = Block { width: 70.0, content_w: 6.0, ..labeled(70.0, 100.0, 20.0) };
        let (r, _) = place(Arrangement::Mosaic, &[grip, block(100.0, 50.0)], area(0.0, 0.0, 400.0, 0.0), 24.0);
        assert_eq!(r[0], area(0.0, 20.0, 6.0, 100.0));
        assert_eq!(r[1].x, 70.0 + 24.0);
    }

    /// Vertical and Adaptive Grid widen a stretching block to its cell; a block that does not
    /// stretch keeps its own width there.
    #[test]
    fn a_fixed_block_keeps_its_width_in_a_cell() {
        let dot = Block { stretch: false, content_w: 12.0, ..block(80.0, 12.0) };
        for kind in [Arrangement::Vertical, Arrangement::AdaptiveGrid { min_col_width: 100.0 }] {
            let (r, _) = place(kind, &[dot, block(80.0, 12.0)], area(0.0, 0.0, 300.0, 0.0), 10.0);
            assert_eq!(r[0].width, 12.0, "{kind:?}");
            assert!(r[1].width > 80.0, "{kind:?}: {:?}", r[1]);
        }
    }

    /// A lasso's block is its members alone: at the area's left edge they start at its edge,
    /// as any block does, and the frame's padding hangs outside them. Inside it there is no
    /// shared label row: an unlabelled member beside a labelled one starts at the labelled
    /// one's strip, not under an empty one of its own.
    #[test]
    fn a_lassos_members_start_at_its_block_under_their_own_strips() {
        let blocks = [block(100.0, 24.0), labeled(100.0, 24.0, 20.0), block(80.0, 30.0)];
        let lasso = Lasso { members: vec![0, 1], padding: 6.0, tab: 22.0 };
        let (r, _) = arrange(Arrangement::Mosaic, &blocks, std::slice::from_ref(&lasso), area(10.0, 50.0, 600.0, 0.0), 24.0);
        assert_eq!(r[0].x, 10.0, "the first member starts the column");
        // The lead (the tab, 22) is above the block; the members share no label row in it.
        assert_eq!(r[0].y, 50.0 + 22.0, "the unlabelled member at the block's top");
        assert_eq!(r[1].y - 20.0, r[0].y, "the labelled one's strip at the same top");
        assert_eq!(r[2].x, 10.0 + 100.0 + 24.0 + 100.0 + 24.0, "the next block a gap past the members");
    }

    /// A lasso's members share one row when they all fit across its cell, and otherwise take a
    /// row each at the cell's left edge — never a row wrapped wherever the widths break.
    #[test]
    fn a_lassos_members_share_a_row_or_take_one_each() {
        let blocks = [block(100.0, 24.0), labeled(100.0, 24.0, 20.0), block(50.0, 24.0)];
        let lasso = Lasso { members: vec![0, 1, 2], padding: 6.0, tab: 22.0 };
        let place_in = |w: f32| arrange(Arrangement::Mosaic, &blocks, std::slice::from_ref(&lasso), area(0.0, 0.0, w, 0.0), 24.0).0;
        // 100 + 24 + 100 + 24 + 50 = 298: one row at 300.
        let r = place_in(300.0);
        assert_eq!(r.iter().map(|r| r.x).collect::<Vec<_>>(), vec![0.0, 124.0, 248.0]);
        assert!(r[0].y == r[2].y);
        // At 250 the last would have wrapped beside nothing; instead every member stacks.
        let r = place_in(250.0);
        assert!(r.iter().all(|r| r.x == 0.0), "{r:?}");
        let tops: Vec<f32> = r.iter().map(|r| r.y).collect();
        assert_eq!(tops, vec![22.0, 22.0 + 24.0 + 24.0 + 20.0, 22.0 + 24.0 + 24.0 + 44.0 + 24.0]);
    }

    /// The grid holds fewer columns than its minimum allows where its widest block would not
    /// fit in one: the three a 90px minimum fits in 300px are two with a 120px block.
    #[test]
    fn a_grid_drops_columns_its_widest_block_would_overrun() {
        let blocks = [block(120.0, 10.0), block(10.0, 10.0), block(10.0, 10.0)];
        let (r, _) = place(Arrangement::AdaptiveGrid { min_col_width: 90.0 }, &blocks, area(0.0, 0.0, 300.0, 0.0), 10.0);
        let xs: Vec<_> = r.iter().map(|r| r.x).collect();
        assert_eq!(xs, vec![0.0, 155.0, 0.0]);
    }

    /// A lasso is one block: its members land together inside it, one padding in, below its
    /// tab, and the padded hull they make — the group's frame — takes in no other block,
    /// whatever lies between the members in order, in every arrangement.
    #[test]
    fn a_lasso_keeps_its_members_together_and_others_out() {
        let blocks = [
            labeled(100.0, 40.0, 20.0),
            labeled(100.0, 40.0, 20.0), // member
            block(150.0, 60.0),
            labeled(80.0, 12.0, 20.0), // member
            block(120.0, 30.0),
        ];
        let (p, tab) = (6.0, 22.0);
        let lasso = Lasso { members: vec![1, 3], padding: p, tab };
        for kind in [
            Arrangement::Vertical,
            Arrangement::AdaptiveGrid { min_col_width: 150.0 },
            Arrangement::Mosaic,
            Arrangement::ReverseMosaic,
        ] {
            let (r, _) = arrange(kind, &blocks, std::slice::from_ref(&lasso), area(0.0, 0.0, 400.0, 2000.0), 24.0);
            // The footprints: content plus strip above, at the footprint width.
            let fp = |i: usize| Rect { x: r[i].x, y: r[i].y - blocks[i].strip, width: r[i].width.max(blocks[i].width), height: r[i].height + blocks[i].strip };
            let hull = [1, 3].iter().map(|&i| fp(i)).reduce(|a, b| {
                let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
                Rect { x: x0, y: y0, width: (a.x + a.width).max(b.x + b.width) - x0, height: (a.y + a.height).max(b.y + b.height) - y0 }
            }).unwrap();
            let frame = Rect { x: hull.x - p, y: hull.y - p - tab, width: hull.width + 2.0 * p, height: hull.height + 2.0 * p + tab };
            for i in [0, 2, 4] {
                assert!(!overlaps(frame, fp(i)), "{kind:?}: the frame {frame:?} takes in block {i} {:?}", fp(i));
            }
        }
    }

    /// What the gallery reads off a widget, and what it arranges: its real exhibits.
    mod exhibits {
        use super::super::*;
        use cce_ui::context::UiContext;
        use cce_ui::scene::layout::Size;
        use cce_ui::widget::{Layout, Widget};

        /// The content height an exhibit declares (`preferred_height` reads it), if any.
        struct Declared(Option<f32>);

        impl Layout for Declared {
            fn intrinsic_size(&self) -> Option<Size> {
                self.0.map(|height| Size { width: 0.0, height })
            }
        }

        struct Mock {
            base: Widget,
            strip: f32,
            declared: Declared,
        }

        impl WidgetHost for Mock {
            cce_ui::impl_widget_base!(Mock);
            fn label_strip(&self) -> f32 {
                self.strip
            }
            fn layout_model(&self) -> &dyn Layout {
                &self.declared
            }
        }

        #[test]
        fn a_block_takes_the_declared_height_else_the_rect_less_its_strip() {
            let mut m = Mock { base: Widget::new(), strip: 18.0, declared: Declared(None) };
            m.set_rect(5.0, 6.0, 190.0, 18.0 + 40.0);
            let b = Block { width: 190.0, content_w: 190.0, content_h: 40.0, strip: 18.0, stretch: true };
            assert_eq!(Block::of(&m, true), b);
            m.declared = Declared(Some(24.0));
            assert_eq!(Block::of(&m, false), Block { content_h: 24.0, stretch: false, ..b });
        }

        /// Every entry of the Layout dropdown places every exhibit of the real table somewhere
        /// finite and non-empty.
        #[test]
        fn every_layout_places_every_exhibit() {
            let mut ui = UiContext::new();
            let extra = crate::variant_exhibits(&mut ui);
            let table = crate::exhibit_table(120.0, &extra);
            let blocks: Vec<Block> = table
                .iter()
                .enumerate()
                .map(|(k, e)| Block { width: e.w, content_w: e.w, content_h: e.h, strip: [0.0, 18.0][k % 2], stretch: e.stretch })
                .collect();
            let slots: Vec<usize> = table.iter().map(|e| e.slot).collect();
            let lassos = crate::lassos(&slots, |_| (6.0, 22.0));
            let area = Rect { x: 28.0, y: 131.0, width: 944.0, height: 489.0 };
            for (name, kind) in crate::LAYOUTS {
                let (rects, height) = arrange(kind, &blocks, &lassos, area, cce_ui::layout::control_gap());
                assert_eq!(rects.len(), blocks.len(), "{name}");
                assert!(height.is_finite() && height > 0.0, "{name}: {height}");
                for r in &rects {
                    assert!([r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite()), "{name}: {r:?}");
                    assert!(r.width > 0.0 && r.height > 0.0, "{name}: {r:?}");
                }
            }
        }
    }
}
