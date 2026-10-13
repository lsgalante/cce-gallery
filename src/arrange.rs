//! Where the exhibits go: the seven arrangements the `Layout` dropdown offers, as pure
//! functions from each exhibit's [`Block`] to the rect its CONTENT lands in.
//!
//! A block is a widget as an arrangement sees it: its width, its content height (the
//! control below its detached label) and its label strip (the label's row above the
//! content). Every arrangement but Overlay reserves the tallest strip among the blocks —
//! the `lead` — above EVERY block's content, so a row of labeled and unlabeled controls
//! lines up by content, and puts `gap` between blocks in both axes. [`arrange`] returns
//! each block's content rect (its label hangs in the strip above it) and the height the
//! arrangement takes, which the exhibit area scrolls over.
//!
//! These were cce-ui's `ContainerLayout` strategies, which the gallery was the one user of;
//! they live here as arithmetic so the gallery lands its widgets itself (`layout_exhibits`),
//! and they reproduce the strategies number for number, quirks included (each is noted on
//! its arm).

use cce_ui::scene::layout::Rect;
use cce_ui::widget::{WidgetHost, WidgetHostExt};

/// One exhibit as an arrangement sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// The block's width: what Mosaic packs (Vertical, Columns and the grids stretch it).
    pub width: f32,
    /// The content height below the label strip; 0 or less means "none", and the
    /// arrangements that stack blocks give it [`FALLBACK_H`].
    pub content_h: f32,
    /// The detached-label strip above the content (0 unlabeled).
    pub strip: f32,
}

impl Block {
    /// A widget's block, read off it after it is seeded at its exhibit size: the width it
    /// landed at, its declared content height (else its rect less the strip) and its
    /// strip — what the toolkit strategies read off a child.
    pub fn of(w: &dyn WidgetHost) -> Block {
        let (_, _, width, h) = w.rect();
        let strip = w.label_strip();
        Block { width, content_h: w.preferred_height().unwrap_or(h - strip), strip }
    }
}

/// The content height a stacking arrangement gives a block that declares none.
pub const FALLBACK_H: f32 = 44.0;

/// The arrangements, as the `Layout` dropdown lists them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arrangement {
    /// One column, every block stretched to the area's width.
    Vertical,
    /// One equal column per block, every block the area's height.
    Columns,
    /// Masonry over a fixed number of columns: each block goes to the shortest column.
    Grid { columns: usize },
    /// [`Grid`](Arrangement::Grid) with as many columns of at least `min_col_width` as fit.
    AdaptiveGrid { min_col_width: f32 },
    /// Blocks packed at their own widths, top-left first.
    Mosaic,
    /// Mosaic's packing stretched, unevenly, over the whole area.
    ReverseMosaic,
    /// Every block over the whole area.
    Overlay,
}

/// Lay `blocks` out in `area` with `gap` between them: each block's content rect, in
/// order, and the height the arrangement takes.
pub fn arrange(kind: Arrangement, blocks: &[Block], area: Rect, gap: f32) -> (Vec<Rect>, f32) {
    let lead = blocks.iter().map(|b| b.strip).fold(0.0, f32::max);
    match kind {
        Arrangement::Vertical => vertical(blocks, area, gap, lead),
        Arrangement::Columns => columns(blocks, area, gap, lead),
        Arrangement::Grid { columns } => grid(blocks, area, gap, lead, columns),
        Arrangement::AdaptiveGrid { min_col_width } => {
            let usable = area.width.max(1.0);
            let columns = ((usable + gap) / (min_col_width + gap)).floor().max(1.0) as usize;
            grid(blocks, area, gap, lead, columns)
        }
        Arrangement::Mosaic => mosaic(blocks, area, gap, lead),
        Arrangement::ReverseMosaic => reverse_mosaic(blocks, area, gap, lead),
        // Each block's OWN strip comes off the area, not the lead; the height is the area's.
        Arrangement::Overlay => (
            blocks
                .iter()
                .map(|b| Rect { x: area.x, y: area.y + b.strip, width: area.width, height: area.height - b.strip })
                .collect(),
            area.height,
        ),
    }
}

/// A block's content height, [`FALLBACK_H`] when it declares none.
fn stacked_h(b: &Block) -> f32 {
    if b.content_h > 0.0 { b.content_h } else { FALLBACK_H }
}

/// The height counts the gap after the last block too.
fn vertical(blocks: &[Block], area: Rect, gap: f32, lead: f32) -> (Vec<Rect>, f32) {
    let width = area.width.max(1.0);
    let mut y = area.y;
    let mut rects = Vec::with_capacity(blocks.len());
    for b in blocks {
        let h = stacked_h(b);
        rects.push(Rect { x: area.x, y: y + lead, width, height: h });
        y += lead + h + gap;
    }
    (rects, (y - area.y).max(0.0))
}

/// The blocks' own sizes play no part; the height is the area's less the lead.
fn columns(blocks: &[Block], area: Rect, gap: f32, lead: f32) -> (Vec<Rect>, f32) {
    if blocks.is_empty() {
        return (Vec::new(), 0.0);
    }
    let n = blocks.len();
    let col_w = (area.width - gap * (n - 1) as f32).max(1.0) / n as f32;
    let h = (area.height - lead).max(1.0);
    let top = area.y + lead;
    let mut x = area.x;
    let mut rects = Vec::with_capacity(n);
    for _ in blocks {
        rects.push(Rect { x, y: top, width: col_w, height: h });
        x += col_w + gap;
    }
    (rects, h)
}

/// A tie for the shortest column goes to the leftmost; the height is measured from the
/// tallest column's bottom, gap included.
fn grid(blocks: &[Block], area: Rect, gap: f32, lead: f32, columns: usize) -> (Vec<Rect>, f32) {
    if blocks.is_empty() {
        return (Vec::new(), 0.0);
    }
    let cols = columns.max(1);
    let col_w = (area.width - gap * (cols - 1) as f32).max(1.0) / cols as f32;
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
        rects.push(Rect { x, y: bottoms[col] + lead, width: col_w, height: h });
        bottoms[col] += lead + h + gap;
    }
    let bottom = bottoms.iter().copied().fold(0.0, f32::max);
    (rects, (bottom - area.y).max(0.0))
}

/// A block's content is clamped to the area's width, its packed footprint is not.
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
        rects.push(Rect { x, y: y + lead, width: b.width.min(avail), height: h });
    }
    (rects, (packer.max_h - area.y).max(0.0))
}

/// Packs as Mosaic from the origin, then scales the packing's bounding box to the area —
/// x and y by their own factors, the lead with the rest, the unclamped widths too. The
/// height is the area's.
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
    let sy = area.height.max(1.0) / (y_max - y_min).max(1.0);
    let rects = packed
        .iter()
        .map(|&(px, py, pw, ph)| Rect {
            x: area.x + (px - x_min) * sx,
            y: area.y + (py - y_min) * sy + lead * sy,
            width: pw * sx,
            height: (ph - lead) * sy,
        })
        .collect();
    (rects, area.height)
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
        Block { width, content_h, strip: 0.0 }
    }

    fn area(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect { x, y, width, height }
    }

    /// cce-ui's `test_mosaic_layout` golden numbers (padding 5 is the area inset by 5).
    #[test]
    fn mosaic_packs_top_left_first() {
        let blocks = [block(100.0, 50.0), block(100.0, 80.0), block(80.0, 40.0)];
        let (r, _) = arrange(Arrangement::Mosaic, &blocks, area(15.0, 25.0, 240.0, 490.0), 10.0);
        assert_eq!((r[0].x, r[0].y), (15.0, 25.0));
        assert_eq!((r[1].x, r[1].y), (15.0, 85.0));
        assert_eq!((r[2].x, r[2].y), (125.0, 25.0));
    }

    /// cce-ui's `test_reverse_mosaic_layout` golden numbers.
    #[test]
    fn reverse_mosaic_stretches_the_packing_over_the_area() {
        let blocks = [block(100.0, 50.0), block(100.0, 80.0), block(80.0, 40.0)];
        let (r, h) = arrange(Arrangement::ReverseMosaic, &blocks, area(15.0, 25.0, 240.0, 290.0), 10.0);
        assert_eq!(h, 290.0);
        assert_eq!((r[0].x, r[0].y), (15.0, 25.0));
        assert!((r[0].width - 126.315).abs() < 0.01, "{:?}", r[0]);
        assert!((r[0].height - 103.57).abs() < 0.01, "{:?}", r[0]);
        assert!((r[2].x - 153.947).abs() < 0.01, "{:?}", r[2]);
        assert_eq!(r[2].y, 25.0);
        assert!((r[2].width - 101.05).abs() < 0.01, "{:?}", r[2]);
        assert!((r[2].height - 82.857).abs() < 0.01, "{:?}", r[2]);
    }

    #[test]
    fn a_block_without_a_height_gets_the_fallback() {
        let blocks = [block(100.0, 0.0), block(100.0, -3.0)];
        for kind in [Arrangement::Vertical, Arrangement::Grid { columns: 2 }, Arrangement::Mosaic] {
            let (r, _) = arrange(kind, &blocks, area(0.0, 0.0, 400.0, 400.0), 10.0);
            assert!(r.iter().all(|r| r.height == FALLBACK_H), "{kind:?}: {r:?}");
        }
    }

    #[test]
    fn vertical_counts_the_trailing_gap() {
        let blocks = [Block { width: 50.0, content_h: 20.0, strip: 6.0 }, block(50.0, 30.0)];
        let (r, h) = arrange(Arrangement::Vertical, &blocks, area(5.0, 10.0, 200.0, 0.0), 4.0);
        // Both blocks keep the lead row, the labeled one's and the unlabeled one's alike.
        assert_eq!(r, vec![area(5.0, 16.0, 200.0, 20.0), area(5.0, 46.0, 200.0, 30.0)]);
        assert_eq!(h, (6.0 + 20.0 + 4.0) + (6.0 + 30.0 + 4.0));
    }

    #[test]
    fn columns_ignore_the_blocks_and_fill_the_area_under_the_lead() {
        let blocks = [Block { width: 999.0, content_h: 5.0, strip: 8.0 }, block(1.0, 500.0)];
        let (r, h) = arrange(Arrangement::Columns, &blocks, area(0.0, 0.0, 210.0, 100.0), 10.0);
        assert_eq!(r, vec![area(0.0, 8.0, 100.0, 92.0), area(110.0, 8.0, 100.0, 92.0)]);
        assert_eq!(h, 92.0);
    }

    #[test]
    fn grid_ties_go_to_the_leftmost_column() {
        let blocks = [block(1.0, 10.0), block(1.0, 10.0), block(1.0, 30.0), block(1.0, 5.0)];
        let (r, h) = arrange(Arrangement::Grid { columns: 2 }, &blocks, area(0.0, 0.0, 90.0, 0.0), 10.0);
        let xy: Vec<_> = r.iter().map(|r| (r.x, r.y)).collect();
        assert_eq!(xy, vec![(0.0, 0.0), (50.0, 0.0), (0.0, 20.0), (50.0, 20.0)]);
        assert_eq!(h, 60.0);
    }

    #[test]
    fn adaptive_grid_fits_as_many_columns_as_the_minimum_allows() {
        let blocks = [block(1.0, 10.0); 4];
        let (r, _) = arrange(Arrangement::AdaptiveGrid { min_col_width: 100.0 }, &blocks, area(0.0, 0.0, 320.0, 0.0), 10.0);
        // (320 + 10) / (100 + 10) = 3 columns.
        let xs: Vec<_> = r.iter().map(|r| r.x).collect();
        assert_eq!(xs, vec![0.0, 110.0, 220.0, 0.0]);
    }

    #[test]
    fn overlay_takes_each_blocks_own_strip() {
        let blocks = [Block { width: 1.0, content_h: 1.0, strip: 12.0 }, block(1.0, 1.0)];
        let (r, h) = arrange(Arrangement::Overlay, &blocks, area(3.0, 4.0, 50.0, 60.0), 10.0);
        assert_eq!(r, vec![area(3.0, 16.0, 50.0, 48.0), area(3.0, 4.0, 50.0, 60.0)]);
        assert_eq!(h, 60.0);
    }

    /// A block too tall for any free rect opens a row at the first free rect's x — here the
    /// space beside the first block, not the area's left edge.
    #[test]
    fn mosaic_opens_a_row_at_the_first_free_rect() {
        let blocks = [block(50.0, 20.0), block(50.0, 200000.0)];
        let (r, _) = arrange(Arrangement::Mosaic, &blocks, area(0.0, 0.0, 200.0, 0.0), 10.0);
        assert_eq!((r[1].x, r[1].y), (60.0, 30.0));
    }

    #[test]
    fn nothing_to_arrange() {
        let a = area(1.0, 2.0, 30.0, 40.0);
        for kind in [
            Arrangement::Vertical,
            Arrangement::Columns,
            Arrangement::Grid { columns: 3 },
            Arrangement::AdaptiveGrid { min_col_width: 10.0 },
            Arrangement::Mosaic,
            Arrangement::ReverseMosaic,
        ] {
            assert_eq!(arrange(kind, &[], a, 5.0), (Vec::new(), 0.0), "{kind:?}");
        }
        assert_eq!(arrange(Arrangement::Overlay, &[], a, 5.0), (Vec::new(), 40.0));
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
            assert_eq!(Block::of(&m), Block { width: 190.0, content_h: 40.0, strip: 18.0 });
            m.declared = Declared(Some(24.0));
            assert_eq!(Block::of(&m), Block { width: 190.0, content_h: 24.0, strip: 18.0 });
        }

        /// Every entry of the Layout dropdown places every exhibit of the real table somewhere
        /// finite and non-empty.
        #[test]
        fn every_layout_places_every_exhibit() {
            let mut ui = UiContext::new();
            let extra = crate::variant_exhibits(&mut ui);
            let table = crate::exhibit_table(120.0, &extra);
            let blocks: Vec<Block> =
                table.iter().enumerate().map(|(k, &(_, width, content_h))| Block { width, content_h, strip: [0.0, 18.0][k % 2] }).collect();
            let area = Rect { x: 28.0, y: 131.0, width: 944.0, height: 489.0 };
            for (name, kind) in crate::LAYOUTS {
                let (rects, height) = arrange(kind, &blocks, area, cce_ui::layout::control_gap());
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
