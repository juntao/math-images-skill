use ab_glyph::{Font, FontRef, GlyphId, PxScale, ScaleFont};
use std::io::BufWriter;

use crate::parser::MathNode;

#[derive(Debug, Clone)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    pub fn bg(&self) -> [u8; 4] {
        match self {
            Theme::Dark => [43, 48, 59, 255],
            Theme::Light => [255, 255, 255, 255],
        }
    }
    pub fn fg(&self) -> [u8; 4] {
        match self {
            Theme::Dark => [192, 197, 206, 255],
            Theme::Light => [51, 51, 51, 255],
        }
    }
}

/// Simple RGBA image buffer.
struct ImageBuf {
    width: u32,
    height: u32,
    data: Vec<u8>, // RGBA, row-major
}

impl ImageBuf {
    fn new(width: u32, height: u32, bg: [u8; 4]) -> Self {
        let size = (width * height * 4) as usize;
        let mut data = Vec::with_capacity(size);
        for _ in 0..width * height {
            data.extend_from_slice(&bg);
        }
        ImageBuf { width, height, data }
    }

    fn put_pixel(&mut self, x: u32, y: u32, color: [u8; 4], alpha: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = ((y * self.width + x) * 4) as usize;
        let a = alpha as f32 / 255.0;
        let inv = 1.0 - a;
        self.data[idx] = (color[0] as f32 * a + self.data[idx] as f32 * inv) as u8;
        self.data[idx + 1] = (color[1] as f32 * a + self.data[idx + 1] as f32 * inv) as u8;
        self.data[idx + 2] = (color[2] as f32 * a + self.data[idx + 2] as f32 * inv) as u8;
        self.data[idx + 3] = 255;
    }

    fn save_png(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let file = std::fs::File::create(path)?;
        let w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, self.width, self.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.data)?;
        Ok(())
    }
}

/// Bounding box for a laid-out node (all in pixels).
#[derive(Debug, Clone, Copy)]
struct Dims {
    width: f32,
    ascent: f32,
    descent: f32,
}

impl Dims {
    fn height(&self) -> f32 {
        self.ascent + self.descent
    }
}

/// Positioned element ready to draw.
enum DrawCmd {
    Glyph { x: f32, y: f32, ch: char, size: f32 },
    HLine { x: f32, y: f32, width: f32, thickness: f32 },
    Text { x: f32, y: f32, text: String, size: f32 },
}

pub struct Renderer {
    font_data: &'static [u8],
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            font_data: include_bytes!("../assets/STIXTwoMath-Regular.otf"),
        }
    }

    pub fn render_equation(
        &self,
        node: &MathNode,
        theme: &Theme,
        font_size: f32,
        scale: f32,
        output: &std::path::Path,
    ) -> anyhow::Result<()> {
        let font = FontRef::try_from_slice(self.font_data)
            .map_err(|e| anyhow::anyhow!("Font load error: {}", e))?;

        let px_size = font_size * scale;
        let sf = font.as_scaled(PxScale::from(px_size));
        let padding = (16.0 * scale) as u32;

        let dims = measure(&font, &sf, node, px_size);

        let img_w = (dims.width as u32 + padding * 2).max(1);
        let img_h = (dims.height() as u32 + padding * 2).max(1);

        let mut img = ImageBuf::new(img_w, img_h, theme.bg());

        let mut cmds = Vec::new();
        let origin_x = padding as f32;
        let origin_y = padding as f32 + dims.ascent;
        layout(&font, &sf, node, px_size, origin_x, origin_y, &mut cmds);

        let fg = theme.fg();
        for cmd in &cmds {
            match cmd {
                DrawCmd::Glyph { x, y, ch, size } => {
                    draw_char(&font, &mut img, *ch, *x, *y, *size, fg);
                }
                DrawCmd::HLine { x, y, width, thickness } => {
                    draw_hline(&mut img, *x, *y, *width, *thickness, fg);
                }
                DrawCmd::Text { x, y, text, size } => {
                    draw_text_str(&font, &mut img, text, *x, *y, *size, fg);
                }
            }
        }

        img.save_png(output)
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Returns true if this node should have extra spacing around it (binary ops, relations).
fn is_spaced_node(node: &MathNode) -> bool {
    match node {
        MathNode::Symbol(ch) => is_bin_or_rel(*ch),
        _ => false,
    }
}

fn is_bin_or_rel(ch: char) -> bool {
    matches!(ch,
        '=' | '<' | '>' | '+' | '-'
        | '\u{2264}' | '\u{2265}' | '\u{2260}' | '\u{2248}' | '\u{2261}' // ≤ ≥ ≠ ≈ ≡
        | '\u{223C}' | '\u{2243}' | '\u{2245}' | '\u{221D}'  // ~ ≃ ≅ ∝
        | '\u{2282}' | '\u{2283}' | '\u{2286}' | '\u{2287}'  // ⊂ ⊃ ⊆ ⊇
        | '\u{2208}' | '\u{2209}' | '\u{220B}'  // ∈ ∉ ∋
        | '\u{222A}' | '\u{2229}'  // ∪ ∩
        | '\u{2228}' | '\u{2227}'  // ∨ ∧
        | '\u{00D7}' | '\u{00F7}'  // × ÷
        | '\u{00B1}' | '\u{2213}'  // ± ∓
        | '\u{2192}' | '\u{2190}' | '\u{2194}'  // → ← ↔
        | '\u{21D2}' | '\u{21D0}' | '\u{21D4}'  // ⇒ ⇐ ⇔
        | '\u{21A6}'  // ↦
        | '\u{226A}' | '\u{226B}'  // ≪ ≫
    )
}

// ─── Measurement ────────────────────────────────────────────────────────────

fn measure(font: &FontRef, sf: &ab_glyph::PxScaleFont<&FontRef>, node: &MathNode, size: f32) -> Dims {
    match node {
        MathNode::Symbol(ch) => measure_char(sf, *ch, size),
        MathNode::Text(t) => measure_text(sf, t, size),
        MathNode::Space(em) => Dims { width: em * size, ascent: 0.0, descent: 0.0 },

        MathNode::Row(children) => {
            let gap = size * 0.05;
            let mut w = 0.0f32;
            let mut asc = 0.0f32;
            let mut desc = 0.0f32;
            for (i, child) in children.iter().enumerate() {
                let d = measure(font, sf, child, size);
                if i > 0 {
                    w += if is_spaced_node(child) || (i > 0 && is_spaced_node(&children[i - 1])) {
                        size * 0.2
                    } else {
                        gap
                    };
                }
                w += d.width;
                asc = asc.max(d.ascent);
                desc = desc.max(d.descent);
            }
            Dims { width: w, ascent: asc, descent: desc }
        }

        MathNode::Frac(num, den) => {
            let ns = size * 0.8;
            let nsf = font.as_scaled(PxScale::from(ns));
            let n = measure(font, &nsf, num, ns);
            let d = measure(font, &nsf, den, ns);
            let rule = size * 0.05;
            let gap = size * 0.15;
            let w = n.width.max(d.width) + size * 0.3;
            Dims {
                width: w,
                ascent: n.height() + gap + rule / 2.0,
                descent: d.height() + gap + rule / 2.0,
            }
        }

        MathNode::Sup(base, exp) => {
            let b = measure(font, sf, base, size);
            let es = size * 0.65;
            let esf = font.as_scaled(PxScale::from(es));
            let e = measure(font, &esf, exp, es);
            let shift = b.ascent * 0.5;
            Dims {
                width: b.width + e.width + size * 0.03,
                ascent: b.ascent.max(shift + e.ascent),
                descent: b.descent,
            }
        }

        MathNode::Sub(base, idx) => {
            let b = measure(font, sf, base, size);
            let is = size * 0.65;
            let isf = font.as_scaled(PxScale::from(is));
            let i = measure(font, &isf, idx, is);
            let shift = b.descent + b.ascent * 0.2;
            Dims {
                width: b.width + i.width + size * 0.03,
                ascent: b.ascent,
                descent: b.descent.max(shift + i.descent),
            }
        }

        MathNode::SubSup(base, sub, sup) => {
            let b = measure(font, sf, base, size);
            let sc = size * 0.65;
            let ssf = font.as_scaled(PxScale::from(sc));
            let sp = measure(font, &ssf, sup, sc);
            let sb = measure(font, &ssf, sub, sc);
            Dims {
                width: b.width + sp.width.max(sb.width) + size * 0.03,
                ascent: b.ascent.max(b.ascent * 0.5 + sp.ascent),
                descent: b.descent.max(b.descent + b.ascent * 0.2 + sb.descent),
            }
        }

        MathNode::Sqrt(content) => {
            let c = measure(font, sf, content, size);
            let rad_w = size * 0.5;
            Dims {
                width: rad_w + c.width + size * 0.1,
                ascent: c.ascent + size * 0.15,
                descent: c.descent + size * 0.1,
            }
        }

        MathNode::Overline(content) => {
            let c = measure(font, sf, content, size);
            Dims {
                width: c.width,
                ascent: c.ascent + size * 0.15,
                descent: c.descent,
            }
        }

        MathNode::Accent(_, content) => {
            let c = measure(font, sf, content, size);
            Dims {
                width: c.width,
                ascent: c.ascent + size * 0.15,
                descent: c.descent,
            }
        }

        MathNode::Matrix { rows, left_delim, right_delim } => {
            measure_matrix(font, sf, rows, left_delim.is_some(), right_delim.is_some(), size)
        }

        MathNode::Cases(rows) => {
            measure_matrix(font, sf, rows, true, false, size)
        }

        MathNode::Delimited { content, .. } => {
            let c = measure(font, sf, content, size);
            let dw = size * 0.25;
            Dims {
                width: c.width + dw * 2.0 + size * 0.1,
                ascent: c.ascent + size * 0.1,
                descent: c.descent + size * 0.1,
            }
        }
    }
}

fn measure_matrix(
    font: &FontRef,
    sf: &ab_glyph::PxScaleFont<&FontRef>,
    rows: &[Vec<MathNode>],
    has_left: bool,
    has_right: bool,
    size: f32,
) -> Dims {
    if rows.is_empty() {
        return Dims { width: 0.0, ascent: 0.0, descent: 0.0 };
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let gap_x = size * 0.6;
    let gap_y = size * 0.3;
    let dw = size * 0.3;

    let mut col_w = vec![0.0f32; ncols];
    let mut row_h = Vec::new();

    for row in rows {
        let mut ra = size * 0.4;
        let mut rd = size * 0.2;
        for (j, cell) in row.iter().enumerate() {
            let d = measure(font, sf, cell, size);
            if j < ncols { col_w[j] = col_w[j].max(d.width); }
            ra = ra.max(d.ascent);
            rd = rd.max(d.descent);
        }
        row_h.push((ra, rd));
    }

    let tw: f32 = col_w.iter().sum::<f32>()
        + gap_x * ncols.saturating_sub(1) as f32
        + if has_left { dw } else { 0.0 }
        + if has_right { dw } else { 0.0 }
        + size * 0.2;

    let th: f32 = row_h.iter().map(|(a, d)| a + d).sum::<f32>()
        + gap_y * rows.len().saturating_sub(1) as f32;

    Dims {
        width: tw,
        ascent: th / 2.0 + size * 0.15,
        descent: th / 2.0 - size * 0.15,
    }
}

fn measure_char(sf: &ab_glyph::PxScaleFont<&FontRef>, ch: char, size: f32) -> Dims {
    let gid = sf.font().glyph_id(ch);
    if gid == GlyphId(0) && ch != ' ' {
        return Dims { width: size * 0.6, ascent: size * 0.7, descent: size * 0.2 };
    }
    Dims {
        width: sf.h_advance(gid),
        ascent: sf.ascent(),
        descent: -sf.descent(),
    }
}

fn measure_text(sf: &ab_glyph::PxScaleFont<&FontRef>, text: &str, _size: f32) -> Dims {
    let mut w = 0.0;
    for ch in text.chars() {
        let gid = sf.font().glyph_id(ch);
        w += sf.h_advance(gid);
    }
    Dims { width: w, ascent: sf.ascent(), descent: -sf.descent() }
}

// ─── Layout ─────────────────────────────────────────────────────────────────

fn layout(
    font: &FontRef,
    sf: &ab_glyph::PxScaleFont<&FontRef>,
    node: &MathNode,
    size: f32,
    x: f32,
    by: f32, // baseline y
    cmds: &mut Vec<DrawCmd>,
) {
    match node {
        MathNode::Symbol(ch) => {
            cmds.push(DrawCmd::Glyph { x, y: by, ch: *ch, size });
        }
        MathNode::Text(t) => {
            cmds.push(DrawCmd::Text { x, y: by, text: t.clone(), size });
        }
        MathNode::Space(_) => {}

        MathNode::Row(children) => {
            let gap = size * 0.05;
            let mut cx = x;
            for (i, child) in children.iter().enumerate() {
                if i > 0 {
                    cx += if is_spaced_node(child) || is_spaced_node(&children[i - 1]) {
                        size * 0.2
                    } else {
                        gap
                    };
                }
                layout(font, sf, child, size, cx, by, cmds);
                cx += measure(font, sf, child, size).width;
            }
        }

        MathNode::Frac(num, den) => {
            let ns = size * 0.8;
            let nsf = font.as_scaled(PxScale::from(ns));
            let nd = measure(font, &nsf, num, ns);
            let dd = measure(font, &nsf, den, ns);
            let rule_t = size * 0.05;
            let gap = size * 0.15;
            let tw = nd.width.max(dd.width) + size * 0.3;
            // Math axis: slightly above baseline (approx x-height / 2)
            let axis = by - size * 0.22;

            cmds.push(DrawCmd::HLine { x, y: axis, width: tw, thickness: rule_t });

            let nx = x + (tw - nd.width) / 2.0;
            let nby = axis - gap - rule_t / 2.0 - nd.descent;
            layout(font, &nsf, num, ns, nx, nby, cmds);

            let dx = x + (tw - dd.width) / 2.0;
            let dby = axis + gap + rule_t / 2.0 + dd.ascent;
            layout(font, &nsf, den, ns, dx, dby, cmds);
        }

        MathNode::Sup(base, exp) => {
            let bd = measure(font, sf, base, size);
            layout(font, sf, base, size, x, by, cmds);
            let es = size * 0.65;
            let esf = font.as_scaled(PxScale::from(es));
            layout(font, &esf, exp, es, x + bd.width + size * 0.03, by - bd.ascent * 0.5, cmds);
        }

        MathNode::Sub(base, idx) => {
            let bd = measure(font, sf, base, size);
            layout(font, sf, base, size, x, by, cmds);
            let is = size * 0.65;
            let isf = font.as_scaled(PxScale::from(is));
            layout(font, &isf, idx, is, x + bd.width + size * 0.03, by + bd.descent + bd.ascent * 0.2, cmds);
        }

        MathNode::SubSup(base, sub, sup) => {
            let bd = measure(font, sf, base, size);
            layout(font, sf, base, size, x, by, cmds);
            let sc = size * 0.65;
            let ssf = font.as_scaled(PxScale::from(sc));
            let sx = x + bd.width + size * 0.03;
            layout(font, &ssf, sup, sc, sx, by - bd.ascent * 0.5, cmds);
            layout(font, &ssf, sub, sc, sx, by + bd.descent + bd.ascent * 0.2, cmds);
        }

        MathNode::Sqrt(content) => {
            let cd = measure(font, sf, content, size);
            let rw = size * 0.5;
            let rule_t = size * 0.05;
            cmds.push(DrawCmd::Glyph { x, y: by, ch: '\u{221A}', size: size * 1.1 });
            cmds.push(DrawCmd::HLine {
                x: x + rw, y: by - cd.ascent - size * 0.1, width: cd.width + size * 0.1, thickness: rule_t,
            });
            layout(font, sf, content, size, x + rw, by, cmds);
        }

        MathNode::Overline(content) => {
            let cd = measure(font, sf, content, size);
            cmds.push(DrawCmd::HLine {
                x, y: by - cd.ascent - size * 0.1, width: cd.width, thickness: size * 0.05,
            });
            layout(font, sf, content, size, x, by, cmds);
        }

        MathNode::Accent(ach, content) => {
            let cd = measure(font, sf, content, size);
            layout(font, sf, content, size, x, by, cmds);
            let as_ = size * 0.5;
            let asf = font.as_scaled(PxScale::from(as_));
            let agid = font.glyph_id(*ach);
            let aw = asf.h_advance(agid);
            cmds.push(DrawCmd::Glyph {
                x: x + (cd.width - aw) / 2.0,
                y: by - cd.ascent - size * 0.05,
                ch: *ach, size: as_,
            });
        }

        MathNode::Matrix { rows, left_delim, right_delim } => {
            layout_matrix(font, sf, rows, *left_delim, *right_delim, size, x, by, cmds);
        }

        MathNode::Cases(rows) => {
            layout_matrix(font, sf, rows, Some('{'), None, size, x, by, cmds);
        }

        MathNode::Delimited { left, right, content } => {
            let cd = measure(font, sf, content, size);
            let dw = size * 0.25;
            let ds = (cd.height() + size * 0.2).min(size * 2.5);
            if *left != '\0' {
                cmds.push(DrawCmd::Glyph { x, y: by, ch: *left, size: ds });
            }
            layout(font, sf, content, size, x + dw + size * 0.05, by, cmds);
            if *right != '\0' {
                cmds.push(DrawCmd::Glyph {
                    x: x + dw + size * 0.05 + cd.width + size * 0.05,
                    y: by, ch: *right, size: ds,
                });
            }
        }
    }
}

fn layout_matrix(
    font: &FontRef,
    sf: &ab_glyph::PxScaleFont<&FontRef>,
    rows: &[Vec<MathNode>],
    left: Option<char>,
    right: Option<char>,
    size: f32,
    x: f32,
    by: f32,
    cmds: &mut Vec<DrawCmd>,
) {
    if rows.is_empty() { return; }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let gap_x = size * 0.6;
    let gap_y = size * 0.3;
    let dw = size * 0.3;

    let mut col_w = vec![0.0f32; ncols];
    let mut row_m: Vec<(f32, f32)> = Vec::new();

    for row in rows {
        let mut ra = size * 0.4;
        let mut rd = size * 0.2;
        for (j, cell) in row.iter().enumerate() {
            let d = measure(font, sf, cell, size);
            if j < ncols { col_w[j] = col_w[j].max(d.width); }
            ra = ra.max(d.ascent);
            rd = rd.max(d.descent);
        }
        row_m.push((ra, rd));
    }

    let th: f32 = row_m.iter().map(|(a, d)| a + d).sum::<f32>()
        + gap_y * rows.len().saturating_sub(1) as f32;

    let mut cx = x;
    if let Some(ld) = left {
        let ds = th.min(size * 3.0);
        cmds.push(DrawCmd::Glyph { x: cx, y: by, ch: ld, size: ds });
        cx += dw;
    }
    cx += size * 0.1;

    let top = by - th / 2.0;
    let mut cy = top;

    for (i, row) in rows.iter().enumerate() {
        let (ra, rd) = row_m[i];
        let cell_by = cy + ra;
        let mut cell_x = cx;
        for (j, cell) in row.iter().enumerate() {
            let d = measure(font, sf, cell, size);
            let off = (col_w[j] - d.width) / 2.0;
            layout(font, sf, cell, size, cell_x + off, cell_by, cmds);
            cell_x += col_w[j] + gap_x;
        }
        cy += ra + rd + gap_y;
    }

    if let Some(rd) = right {
        let content_w: f32 = col_w.iter().sum::<f32>() + gap_x * ncols.saturating_sub(1) as f32;
        let ds = th.min(size * 3.0);
        cmds.push(DrawCmd::Glyph { x: cx + content_w + size * 0.1, y: by, ch: rd, size: ds });
    }
}

// ─── Drawing primitives ─────────────────────────────────────────────────────

fn draw_char(font: &FontRef, img: &mut ImageBuf, ch: char, x: f32, y: f32, size: f32, color: [u8; 4]) {
    let gid = font.glyph_id(ch);
    if gid == GlyphId(0) && ch != ' ' {
        draw_placeholder(img, x, y, size, color);
        return;
    }
    let glyph = gid.with_scale_and_position(PxScale::from(size), ab_glyph::point(x, y));
    if let Some(outlined) = font.outline_glyph(glyph) {
        let bounds = outlined.px_bounds();
        outlined.draw(|px, py, cov| {
            let ix = bounds.min.x as i32 + px as i32;
            let iy = bounds.min.y as i32 + py as i32;
            if ix >= 0 && iy >= 0 {
                img.put_pixel(ix as u32, iy as u32, color, (cov * 255.0) as u8);
            }
        });
    }
}

fn draw_text_str(font: &FontRef, img: &mut ImageBuf, text: &str, x: f32, y: f32, size: f32, color: [u8; 4]) {
    let sf = font.as_scaled(PxScale::from(size));
    let mut cx = x;
    for ch in text.chars() {
        draw_char(font, img, ch, cx, y, size, color);
        let gid = font.glyph_id(ch);
        cx += sf.h_advance(gid);
    }
}

fn draw_hline(img: &mut ImageBuf, x: f32, y: f32, width: f32, thickness: f32, color: [u8; 4]) {
    let ys = (y - thickness / 2.0).round() as i32;
    let ye = (y + thickness / 2.0).ceil() as i32;
    let xs = x.round() as i32;
    let xe = (x + width).round() as i32;
    for py in ys..=ye {
        for px in xs..xe {
            if px >= 0 && py >= 0 {
                img.put_pixel(px as u32, py as u32, color, 255);
            }
        }
    }
}

fn draw_placeholder(img: &mut ImageBuf, x: f32, y: f32, size: f32, color: [u8; 4]) {
    let w = (size * 0.5) as i32;
    let h = (size * 0.6) as i32;
    let x0 = x as i32;
    let y0 = (y - size * 0.5) as i32;
    for py in y0..y0 + h {
        for px in x0..x0 + w {
            if (py == y0 || py == y0 + h - 1 || px == x0 || px == x0 + w - 1)
                && px >= 0 && py >= 0
            {
                img.put_pixel(px as u32, py as u32, color, 128);
            }
        }
    }
}
