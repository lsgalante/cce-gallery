//! Test-interface-owned copies of the dissolved cce-ui containers (Phase 6as): the
//! gallery is the last constructor of ControlPanel / SectionContainer / Plate /
//! Backplate, and it needs them only as demo chrome — the panel verbatim, the other
//! three as passive lookalikes replicating the legacy types' exact emission surfaces
//! (color rules, corner radius/rounding, borders, separators, labels, hit shapes).
//! Phase 6az: all four are on the narrow traits wrapped in `Adapted<W>`; the gallery
//! roster keeps them as `Box<dyn Element>` and reaches the models through `as_any`.

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
    pub children: Vec<*mut (dyn Element + 'static)>,
    pub scroll_box: ScrollBox,
    pub active_drag_widget: Option<*mut (dyn Element + 'static)>,
}

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
            children: Vec::new(),
            scroll_box: sb,
            active_drag_widget: None,
        })
    }

    pub fn add_child(&mut self, child: *mut (dyn Element + 'static)) {
        self.children.push(child);
    }

    /// The laid-out rect, mirrored from the adapter by `Layout::rect_assigned`.
    fn rect(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.w, self.h)
    }

    /// First open child popover, in scrolled (screen) coordinates — the old
    /// `Element::popover_rect` override, verbatim (the temporary base-y shift reaches the
    /// children through their raw pointers).
    fn popover_scan(&self) -> Option<(f32, f32, f32, f32)> {
        let scroll_y = self.scroll_box.scroll_y;
        unsafe {
            for child_ptr in &self.children {
                let child = &mut **child_ptr;
                let old_y = child.base().map(|b| b.y).unwrap_or(0.0);
                if let Some(b) = child.base_mut() {
                    b.y = old_y - scroll_y;
                }
                let res = child.popover_rect();
                if let Some(b) = child.base_mut() {
                    b.y = old_y;
                }
                if res.is_some() {
                    return res;
                }
            }
        }
        None
    }

    /// The subtree's rounded view (the old `Element::all_rounded_quads` override):
    /// background, borders, children with the scroll shift + viewport clamp + border
    /// inset, scrollbar. External readers get it through the adapter's reverse bridge.
    fn aggregate_rounded(&self, ctx: &UiContext) -> Vec<(f32, f32, f32, f32, f32, [f32; 4], (bool, bool, bool, bool))> {
        let mut quads = Vec::new();
        let (x, y, w, h) = self.rect();

        // 1. Background
        quads.push((x, y, w, h, 0.0, colors::control_panel_color(), (false, false, false, false)));

        // 2. Borders
        let border_color = colors::control_panel_border_color();
        quads.push((x, y, w, 1.0, 0.0, border_color, (false, false, false, false)));
        quads.push((x, y + h - 1.0, w, 1.0, 0.0, border_color, (false, false, false, false)));
        quads.push((x, y, 1.0, h, 0.0, border_color, (false, false, false, false)));
        quads.push((x + w - 1.0, y, 1.0, h, 0.0, border_color, (false, false, false, false)));

        // 3. Child elements clipped to viewport bounds
        let scroll_y = self.scroll_box.scroll_y;
        let y_start = y;
        let y_end = y + h;

        unsafe {
            for child_ptr in &self.children {
                let child = &**child_ptr;
                let solid_border_opt = if child.type_name() == "Toggle" { None } else { child.solid_border() };
                let (cx, cy, cw, ch) = child.rect();

                if let Some((b_color, _thickness)) = solid_border_opt {
                    let cy_shifted = cy - scroll_y;
                    let cy_top = cy_shifted;
                    let cy_bottom = cy_shifted + ch;
                    if cy_bottom > y_start && cy_top < y_end {
                        let visible_top = cy_top.max(y_start);
                        let visible_bottom = cy_bottom.min(y_end);
                        let visible_h = visible_bottom - visible_top;
                        if visible_h > 0.0 {
                            let (child_r, child_corners) = child.corner_style();
                            let radii_adjusted = if visible_top > cy_top || visible_bottom < cy_bottom {
                                0.0
                            } else {
                                child_r
                            };
                            quads.push((
                                cx,
                                visible_top,
                                cw,
                                visible_h,
                                radii_adjusted,
                                b_color,
                                child_corners,
                            ));
                        }
                    }
                }

                for (qx, qy, qw, qh, qr, qc, qcorners) in child.all_rounded_quads(ctx) {
                    let mut rx = qx;
                    let mut ry = qy;
                    let mut rw = qw;
                    let mut rh = qh;
                    let mut rqr = qr;

                    if let Some((_, thickness)) = solid_border_opt {
                        if (qx - cx).abs() < 0.1 && (qy - cy).abs() < 0.1 && (qw - cw).abs() < 0.1 && (qh - ch).abs() < 0.1 {
                            rx += thickness;
                            ry += thickness;
                            rw -= 2.0 * thickness;
                            rh -= 2.0 * thickness;
                            rqr = (qr - thickness).max(0.0);
                        }
                    }

                    let qy_shifted = ry - scroll_y;
                    let qy_top = qy_shifted;
                    let qy_bottom = qy_shifted + rh;
                    if qy_bottom > y_start && qy_top < y_end {
                        let visible_top = qy_top.max(y_start);
                        let visible_bottom = qy_bottom.min(y_end);
                        let visible_h = visible_bottom - visible_top;
                        if visible_h > 0.0 {
                            let radii_adjusted = if visible_top > qy_top || visible_bottom < qy_bottom {
                                0.0
                            } else {
                                rqr
                            };
                            quads.push((rx, visible_top, rw, visible_h, radii_adjusted, qc, qcorners));
                        }
                    }
                }
            }
        }

        // 4. Scrollbar
        for (sx, sy, sw, sh, sc) in self.scroll_box.extra_quads() {
            quads.push((sx, sy, sw, sh, 0.0, sc, (false, false, false, false)));
        }

        quads
    }

    /// The subtree's plain view (the old `Element::extra_quads` override): children's
    /// decoration quads with the scroll shift + viewport clamp, plus the scrollbar.
    fn aggregate_plain(&self) -> Vec<(f32, f32, f32, f32, [f32; 4])> {
        let mut quads = Vec::new();
        let (_x, y, _w, h) = self.rect();
        let scroll_y = self.scroll_box.scroll_y;
        let y_start = y;
        let y_end = y + h;
        let ctx_dummy = cce_ui::context::UiContext::new();

        unsafe {
            for child_ptr in &self.children {
                let child = &**child_ptr;
                let (cx, cy, cw, ch) = child.rect();
                let has_rounded = child.corner_style().1 != (false, false, false, false);
                let has_bg = child.color()[3].abs() > 0.001;

                for (qx, qy, qw, qh, qc) in child.all_quads(&ctx_dummy) {
                    if has_rounded && has_bg && (qx - cx).abs() < 0.1 && (qy - cy).abs() < 0.1 && (qw - cw).abs() < 0.1 && (qh - ch).abs() < 0.1 {
                        continue;
                    }
                    let qy_shifted = qy - scroll_y;
                    let qy_top = qy_shifted;
                    let qy_bottom = qy_shifted + qh;
                    if qy_bottom > y_start && qy_top < y_end {
                        let visible_top = qy_top.max(y_start);
                        let visible_bottom = qy_bottom.min(y_end);
                        let visible_h = visible_bottom - visible_top;
                        if visible_h > 0.0 {
                            quads.push((qx, visible_top, qw, visible_h, qc));
                        }
                    }
                }
            }
        }

        quads.extend(self.scroll_box.extra_quads());
        quads
    }

    /// The children's arcs with the scroll shift (the old `Element::extra_arcs` override).
    fn aggregate_arcs(&self) -> Vec<(f32, f32, f32, f32, f32, f32, [f32; 4])> {
        let mut arcs = Vec::new();
        let scroll_y = self.scroll_box.scroll_y;
        let y_start = self.y;
        let y_end = self.y + self.h;

        unsafe {
            for child_ptr in &self.children {
                let child = &**child_ptr;
                for (cx, cy, r, t, start, end, color) in child.extra_arcs() {
                    let cy_shifted = cy - scroll_y;
                    if cy_shifted + r > y_start && cy_shifted - r < y_end {
                        arcs.push((cx, cy_shifted, r, t, start, end, color));
                    }
                }
            }
        }
        arcs
    }

    /// Children's walk text with the panel's scroll shift and viewport clamp — what the
    /// deleted fonted getter served: children are laid out UNSCROLLED and the offset is
    /// an aggregate-time transform.
    pub(crate) fn scrolled_child_labels(&self, ctx: &UiContext) -> Vec<(TextLabel, Option<String>, Option<[f32; 4]>)> {
        let scroll_y = self.scroll_box.scroll_y;
        let (x, y, w, h) = self.rect();
        let mut labels = Vec::new();
        for &child_ptr in &self.children {
            let mut scratch = cce_ui::scene::paint::PaintCtx::new();
            cce_ui::scene::painter::append_widget_text(ctx, unsafe { &*child_ptr }, &mut scratch);
            for item in scratch.finish().items {
                if let cce_ui::scene::paint::Prim::Text { text, x: lx, y: ly, font_size, color, font, bounds, .. } = item.prim {
                    let new_bounds = if let Some([l, t, r, b]) = bounds {
                        let nl = l.max(x);
                        let nt = (t - scroll_y).max(y);
                        let nr = r.min(x + w);
                        let nb = (b - scroll_y).min(y + h);
                        Some([nl, nt, nr, nb])
                    } else {
                        Some([x, y, x + w, y + h])
                    };
                    labels.push((
                        TextLabel { text, x: lx, y: ly - scroll_y, font_size, color },
                        font,
                        new_bounds,
                    ));
                }
            }
        }
        labels
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
    }

    // The old `set_rect` override's arrangement: children keep their label-matched slots
    // in a ColumnLayout, laid out UNSCROLLED; the scroll offset is an aggregate-time
    // transform. `host` (the adapter) becomes the children's parent, as the legacy panel
    // set itself.
    fn arrange_children(&mut self, rect: Rect, host: *mut (dyn Element + 'static)) {
        let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);

        self.scroll_box.set_rect(x, y, w, h);

        let mut dummy = cce_ui::context::UiContext::new();
        let self_ptr_option = Some(host);

        let padding = cce_ui::layout::control_panel_padding();
        let gap = cce_ui::layout::control_panel_gap();
        let mut col = ColumnLayout::new(x, y, w - 12.0, gap, padding); // 12px reserved for scrollbar track

        unsafe {
            let mut create_btn: Option<*mut dyn Element> = None;
            let mut tile_btn: Option<*mut dyn Element> = None;
            let mut opacity_toggle: Option<*mut dyn Element> = None;
            let mut enable_toggle: Option<*mut dyn Element> = None;
            let mut slider: Option<*mut dyn Element> = None;
            let mut slider_label: Option<*mut dyn Element> = None;
            let mut type_dd: Option<*mut dyn Element> = None;
            let mut shape_dd: Option<*mut dyn Element> = None;
            let mut border_style_dd: Option<*mut dyn Element> = None;
            let mut width_spin: Option<*mut dyn Element> = None;
            let mut height_spin: Option<*mut dyn Element> = None;
            let mut backplate_toggle: Option<*mut dyn Element> = None;
            let mut menubar_toggle: Option<*mut dyn Element> = None;
            let mut statusbar_toggle: Option<*mut dyn Element> = None;
            let mut border_sec: Option<*mut dyn Element> = None;
            let mut bevel_toggle: Option<*mut dyn Element> = None;
            let mut border_width_spin: Option<*mut dyn Element> = None;
            let mut bevel_depth_spin: Option<*mut dyn Element> = None;
            let mut win_sec: Option<*mut dyn Element> = None;
            let mut bevel_shape_btn: Option<*mut dyn Element> = None;

            for &child_ptr in &self.children {
                let child = &mut *child_ptr;
                child.set_parent(self_ptr_option, &mut dummy);
                let label = child.base().and_then(|b| b.label.as_ref()).map(|s| s.as_str()).unwrap_or("");
                match label {
                    "Create Window" => create_btn = Some(child_ptr),
                    "Tile Windows" => tile_btn = Some(child_ptr),
                    "Opacity" => opacity_toggle = Some(child_ptr),
                    "Enable" => enable_toggle = Some(child_ptr),
                    "Transparency Level" => slider_label = Some(child_ptr),
                    "Border" => border_sec = Some(child_ptr),
                    "Window Elements" => win_sec = Some(child_ptr),
                    "Window Type" => type_dd = Some(child_ptr),
                    "Window Shape" => shape_dd = Some(child_ptr),
                    "Border Style" => border_style_dd = Some(child_ptr),
                    "Width" => width_spin = Some(child_ptr),
                    "Height" => height_spin = Some(child_ptr),
                    "Backplate" => backplate_toggle = Some(child_ptr),
                    "MenuBar" => menubar_toggle = Some(child_ptr),
                    "StatusBar" => statusbar_toggle = Some(child_ptr),
                    "Bevel" => bevel_toggle = Some(child_ptr),
                    "Border Width" => border_width_spin = Some(child_ptr),
                    "Bevel Depth" => bevel_depth_spin = Some(child_ptr),
                    "Bevel Shape..." => bevel_shape_btn = Some(child_ptr),
                    _ => {
                        if child.base().is_some() && child.base().unwrap().label.is_none() {
                            slider = Some(child_ptr);
                        }
                    }
                }
            }

            if let (Some(c), Some(t)) = (create_btn, tile_btn) {
                col.add_row(&[c, t], 28.0, 12.0);
            } else {
                if let Some(c) = create_btn {
                    col.add_widget(&mut *c, 28.0);
                }
                if let Some(t) = tile_btn {
                    col.add_widget(&mut *t, 28.0);
                }
            }

            if let (Some(w_sp), Some(h_sp)) = (width_spin, height_spin) {
                col.add_row(&[w_sp, h_sp], 42.0, 12.0);
            } else {
                if let Some(w_sp) = width_spin {
                    col.add_widget(&mut *w_sp, 42.0);
                }
                if let Some(h_sp) = height_spin {
                    col.add_widget(&mut *h_sp, 42.0);
                }
            }

            if let Some(t_dd) = type_dd {
                col.add_widget(&mut *t_dd, 44.0);
            }
            if let Some(s_dd) = shape_dd {
                col.add_widget(&mut *s_dd, 44.0);
            }
            if let Some(op_t) = opacity_toggle {
                col.add_widget(&mut *op_t, 28.0);
            }
            if let Some(en_t) = enable_toggle {
                col.add_widget(&mut *en_t, 28.0);
            }
            if let Some(sl_lbl) = slider_label {
                col.add_widget(&mut *sl_lbl, 12.0);
            }
            if let Some(sl) = slider {
                col.add_widget(&mut *sl, 20.0);
            }

            if let Some(w_s) = win_sec {
                col.add_widget(&mut *w_s, 20.0);
            }
            let toggles = [backplate_toggle, menubar_toggle, statusbar_toggle];
            let active_toggles: Vec<*mut dyn Element> = toggles.iter().filter_map(|&t| t).collect();
            if !active_toggles.is_empty() {
                col.add_row(&active_toggles, 28.0, 10.0);
            }

            if let Some(b_s) = border_sec {
                col.add_widget(&mut *b_s, 20.0);
            }
            if let Some(bs_dd) = border_style_dd {
                col.add_widget(&mut *bs_dd, 44.0);
            }
            if let Some(bev_t) = bevel_toggle {
                col.add_widget(&mut *bev_t, 28.0);
            }

            if let (Some(bw_sp), Some(bd_sp)) = (border_width_spin, bevel_depth_spin) {
                col.add_row(&[bw_sp, bd_sp], 42.0, 12.0);
            } else {
                if let Some(bw_sp) = border_width_spin {
                    col.add_widget(&mut *bw_sp, 42.0);
                }
                if let Some(bd_sp) = bevel_depth_spin {
                    col.add_widget(&mut *bd_sp, 42.0);
                }
            }
            if let Some(bs_btn) = bevel_shape_btn {
                col.add_widget(&mut *bs_btn, 28.0);
            }

            let total_h = col.current_y();
            self.scroll_box.update_bounds(total_h, y, h);
        }
    }
}

impl cce_ui::widget::Paint for ControlPanel {
    fn color(&self) -> [f32; 4] {
        colors::control_panel_color()
    }

    fn corner_style(&self, _rect: Rect) -> Option<(f32, (bool, bool, bool, bool))> {
        // Legacy: rounded_corners all-true with the Element-default 12.0 radius.
        Some((12.0, (true, true, true, true)))
    }

    // ControlPanel is a legacy scroll frame: children are laid out UNSCROLLED and the
    // scroll offset is applied at aggregate time. The walk must emit these aggregates
    // and not descend — descending would paint the children unshifted, desyncing text
    // from geometry as soon as the panel scrolls.
    fn paints_own_subtree(&self) -> bool {
        true
    }

    fn paint(&self, _rect: Rect, pc: &mut PaintCtx) {
        // The children are Adapted widgets whose aggregate getters read nothing from the
        // routing context; a fresh one stands in for the ctx `Paint::paint` doesn't carry.
        let dummy = cce_ui::context::UiContext::new();
        for (x, y, qw, qh, r, c, corners) in self.aggregate_rounded(&dummy) {
            pc.rounded_rect(Rect { x, y, width: qw, height: qh }, r, corners, c);
        }
        for (x, y, qw, qh, c) in self.aggregate_plain() {
            pc.quad(Rect { x, y, width: qw, height: qh }, c);
        }
        for (cx, cy, r, t, s, e, c) in self.aggregate_arcs() {
            pc.arc(cx, cy, r, t, s, e, c);
        }
        for (tl, font, bounds) in self.scrolled_child_labels(&dummy) {
            pc.text_with(tl.text, tl.x, tl.y, tl.font_size, tl.color, font, bounds);
        }
    }

    fn popover(&self, _rect: Rect) -> Option<(f32, f32, f32, f32)> {
        self.popover_scan()
    }

    fn draw_popover(&self, _rect: Rect, pc: &mut dyn cce_ui::layout::RenderTarget) {
        let scroll_y = self.scroll_box.scroll_y;
        unsafe {
            for child_ptr in &self.children {
                if (**child_ptr).popover_rect().is_some() {
                    let child = &mut **child_ptr;
                    let old_y = child.base().map(|b| b.y).unwrap_or(0.0);
                    if let Some(b) = child.base_mut() {
                        b.y = old_y - scroll_y;
                    }
                    child.render_popover(pc);
                    if let Some(b) = child.base_mut() {
                        b.y = old_y;
                    }
                }
            }
        }
    }
}

impl cce_ui::widget::Input for ControlPanel {
    /// The legacy hit shape: an open child popover (scrolled coordinates) or the panel rect.
    fn hit(&self, rect: Rect, x: f32, y: f32) -> bool {
        if let Some(pop_rect) = self.popover_scan() {
            if x >= pop_rect.0 && x <= pop_rect.0 + pop_rect.2 && y >= pop_rect.1 && y <= pop_rect.1 + pop_rect.3 {
                return true;
            }
        }
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return false;
        }
        x >= rect.x && x <= rect.x + rect.width && y >= rect.y && y <= rect.y + rect.height
    }

    // The legacy direct-dispatch broadcast reached mouse_input ungated: the panel resets
    // its drag target on every press and lets children with open popovers see off-panel
    // presses.
    fn gates_presses(&self) -> bool {
        false
    }

    fn on_event(&mut self, event: &Event, ectx: &mut EventCtx) -> bool {
        let Some(ctx) = ectx.ui.as_deref_mut() else { return false };
        match event {
            Event::MouseButton { button, state, x: px, y: py, .. } => {
                if *state == ElementState::Pressed {
                    self.active_drag_widget = None;
                }

                if self.scroll_box.mouse_input(*button, *state, *px, *py, ctx) {
                    return true;
                }

                let scroll_y = self.scroll_box.scroll_y;
                let py_translated = *py + scroll_y;
                let (_x, y, _w, h) = self.rect();

                unsafe {
                    for child_ptr in &self.children {
                        let has_popover = (**child_ptr).popover_rect().is_some();
                        // Only dispatch if the click Y is inside the viewport or the child has an active popover
                        if has_popover || (*py >= y && *py <= y + h) {
                            if (**child_ptr).mouse_input(*button, *state, *px, py_translated, ctx) {
                                if *state == ElementState::Pressed && (**child_ptr).draggable() {
                                    self.active_drag_widget = Some(*child_ptr);
                                }
                                return true;
                            }
                        }
                    }
                }
                false
            }
            Event::PointerMove { x: px, y: py, .. } => {
                let mut changed = self.scroll_box.cursor_moved(*px, *py, ctx);

                let scroll_y = self.scroll_box.scroll_y;
                let py_translated = *py + scroll_y;
                unsafe {
                    for child_ptr in &self.children {
                        if (**child_ptr).cursor_moved(*px, py_translated, ctx) {
                            changed = true;
                        }
                    }
                }
                changed
            }
            Event::MouseWheel { delta, x: px, y: py, .. } => {
                if self.scroll_box.mouse_wheel(delta, *px, *py, ctx) {
                    return true;
                }

                let scroll_y = self.scroll_box.scroll_y;
                let py_translated = *py + scroll_y;
                unsafe {
                    for child_ptr in &self.children {
                        if (**child_ptr).mouse_wheel(delta, *px, py_translated, ctx) {
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn draggable(&self, _rect: Rect) -> bool {
        self.scroll_box.draggable() || self.active_drag_widget.is_some()
    }

    fn drag_begin(&mut self, px: f32, py: f32, _rect: Rect) {
        if self.scroll_box.hit_test_scrollbar(px, py) {
            self.scroll_box.drag_begin(px, py);
            return;
        }

        let scroll_y = self.scroll_box.scroll_y;
        let py_translated = py + scroll_y;
        if let Some(child_ptr) = self.active_drag_widget {
            unsafe {
                (*child_ptr).drag_begin(px, py_translated);
            }
        }
    }

    fn drag_update(&mut self, px: f32, py: f32, _rect: Rect) -> bool {
        if self.scroll_box.draggable() {
            return self.scroll_box.drag_update(px, py);
        }

        let scroll_y = self.scroll_box.scroll_y;
        let py_translated = py + scroll_y;
        if let Some(child_ptr) = self.active_drag_widget {
            unsafe {
                return (*child_ptr).drag_update(px, py_translated);
            }
        }
        false
    }

    fn drag_end(&mut self) {
        self.scroll_box.drag_end();
        if let Some(child_ptr) = self.active_drag_widget {
            unsafe {
                (*child_ptr).drag_end();
            }
            self.active_drag_widget = None;
        }
    }

    fn tick_ctx(&mut self, dt: f32, ectx: &mut EventCtx) -> bool {
        let Some(ctx) = ectx.ui.as_deref_mut() else { return false };
        let mut changed = self.scroll_box.tick(dt, ctx);
        unsafe {
            for child_ptr in &self.children {
                if (**child_ptr).tick(dt, ctx) {
                    changed = true;
                }
            }
        }
        changed
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
