//! Image mirror rasterizer: draws a `vt100` grid into an RGB picture with one monospace font.

use std::collections::HashMap;
use std::ops::Deref;
use vt100::Color;

const BUNDLED_FONT: &[u8] = include_bytes!("../assets/JetBrainsMono-Regular.ttf");
/// Glyph size in pixels. The terminal scales the picture to the peek, so this sets only sharpness and bandwidth.
const FONT_PX: f32 = 14.0;
const DEFAULT_FG: [u8; 3] = [0xd0, 0xd0, 0xd0];
const DEFAULT_BG: [u8; 3] = [0x14, 0x14, 0x18];

/// One RGB picture, three bytes per pixel, rows top to bottom.
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// A glyph's coverage bitmap, placed relative to its cell's top-left corner.
struct Glyph {
    left: i32,
    top: i32,
    width: usize,
    coverage: Vec<u8>,
}

pub struct Raster {
    font: fontdue::Font,
    cell_w: usize,
    cell_h: usize,
    baseline: i32,
    /// `None` marks a character the font lacks; it draws as a box.
    glyphs: HashMap<char, Option<Glyph>>,
}

impl Raster {
    /// The font named by `PIP_FONT`, else the bundled one. A `PIP_FONT` that fails to load
    /// falls back to the bundled font and returns why.
    pub fn load() -> (Raster, Option<String>) {
        let bundled = || Raster::from_font(BUNDLED_FONT).expect("bundled font loads");
        let Some(path) = std::env::var_os("PIP_FONT") else {
            return (bundled(), None);
        };
        match std::fs::read(&path).map_err(|err| err.to_string()).and_then(Raster::from_font) {
            Ok(raster) => (raster, None),
            Err(err) => (bundled(), Some(format!("PIP_FONT: {err}"))),
        }
    }

    fn from_font<Data: Deref<Target = [u8]>>(bytes: Data) -> Result<Raster, String> {
        let settings = fontdue::FontSettings { scale: FONT_PX, ..Default::default() };
        let font = fontdue::Font::from_bytes(bytes, settings).map_err(str::to_string)?;
        let line = font.horizontal_line_metrics(FONT_PX).ok_or("font has no horizontal metrics")?;
        let cell_w = font.metrics('M', FONT_PX).advance_width.round().max(1.0) as usize;
        let cell_h = (line.ascent - line.descent + line.line_gap).round().max(1.0) as usize;
        let baseline = (line.ascent + line.line_gap / 2.0).round() as i32;
        Ok(Raster { font, cell_w, cell_h, baseline, glyphs: HashMap::new() })
    }

    /// The top-left `cols`×`rows` cells of `screen`, one font cell per terminal cell.
    /// A visible cursor draws as an inverted cell.
    pub fn render(&mut self, screen: &vt100::Screen, cols: u16, rows: u16) -> Picture {
        let (width, height) = (usize::from(cols) * self.cell_w, usize::from(rows) * self.cell_h);
        let mut rgb = DEFAULT_BG.repeat(width * height);
        let cursor = (!screen.hide_cursor()).then(|| screen.cursor_position());
        for row in 0..rows {
            for col in 0..cols {
                let Some(cell) = screen.cell(row, col) else { continue };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut fg = color(cell.fgcolor(), cell.bold(), DEFAULT_FG);
                let mut bg = color(cell.bgcolor(), false, DEFAULT_BG);
                if cell.inverse() != (cursor == Some((row, col))) {
                    std::mem::swap(&mut fg, &mut bg);
                }
                let x0 = usize::from(col) * self.cell_w;
                let w = (if cell.is_wide() { 2 } else { 1 } * self.cell_w).min(width - x0);
                let rect = Rect { x0, y0: usize::from(row) * self.cell_h, w, h: self.cell_h };
                if bg != DEFAULT_BG {
                    fill(&mut rgb, width, &rect, bg);
                }
                if cell.underline() {
                    let y = (rect.y0 as i32 + self.baseline + 1) as usize;
                    fill(&mut rgb, width, &Rect { y0: y.min(rect.y0 + rect.h - 1), h: 1, ..rect }, fg);
                }
                if let Some(ch) = cell.contents().chars().next().filter(|ch| !ch.is_whitespace()) {
                    self.draw_glyph(&mut rgb, width, &rect, ch, fg);
                }
            }
        }
        Picture { width: width as u32, height: height as u32, rgb }
    }

    /// Blends `ch` in `fg` into `rect`, clipped to it; a character the font lacks draws as a box.
    fn draw_glyph(&mut self, rgb: &mut [u8], width: usize, rect: &Rect, ch: char, fg: [u8; 3]) {
        let baseline = self.baseline;
        let glyph = self.glyphs.entry(ch).or_insert_with(|| {
            (self.font.lookup_glyph_index(ch) != 0).then(|| {
                let (metrics, coverage) = self.font.rasterize(ch, FONT_PX);
                let top = baseline - metrics.height as i32 - metrics.ymin;
                Glyph { left: metrics.xmin, top, width: metrics.width, coverage }
            })
        });
        let Some(glyph) = glyph else {
            return draw_box(rgb, width, rect, fg);
        };
        for (gy, line) in glyph.coverage.chunks(glyph.width.max(1)).enumerate() {
            let y = rect.y0 as i32 + glyph.top + gy as i32;
            if y < rect.y0 as i32 || y >= (rect.y0 + rect.h) as i32 {
                continue;
            }
            for (gx, &alpha) in line.iter().enumerate() {
                let x = rect.x0 as i32 + glyph.left + gx as i32;
                if alpha == 0 || x < rect.x0 as i32 || x >= (rect.x0 + rect.w) as i32 {
                    continue;
                }
                let at = (y as usize * width + x as usize) * 3;
                for (channel, &target) in rgb[at..at + 3].iter_mut().zip(&fg) {
                    let (from, to, a) = (i32::from(*channel), i32::from(target), i32::from(alpha));
                    *channel = (from + (to - from) * a / 255) as u8;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
}

fn fill(rgb: &mut [u8], width: usize, rect: &Rect, color: [u8; 3]) {
    for y in rect.y0..rect.y0 + rect.h {
        let start = (y * width + rect.x0) * 3;
        for pixel in rgb[start..start + rect.w * 3].chunks_exact_mut(3) {
            pixel.copy_from_slice(&color);
        }
    }
}

/// A one-pixel outline inset by one pixel from the cell's edges.
fn draw_box(rgb: &mut [u8], width: usize, rect: &Rect, color: [u8; 3]) {
    if rect.w < 3 || rect.h < 3 {
        return;
    }
    let inner = Rect { x0: rect.x0 + 1, y0: rect.y0 + 1, w: rect.w - 2, h: rect.h - 2 };
    for edge in [
        Rect { h: 1, ..inner },
        Rect { y0: inner.y0 + inner.h - 1, h: 1, ..inner },
        Rect { w: 1, ..inner },
        Rect { x0: inner.x0 + inner.w - 1, w: 1, ..inner },
    ] {
        fill(rgb, width, &edge, color);
    }
}

/// `color` as RGB; bold brightens the eight base colors, as terminals commonly do.
fn color(color: Color, bold: bool, default: [u8; 3]) -> [u8; 3] {
    match color {
        Color::Default => default,
        Color::Rgb(r, g, b) => [r, g, b],
        Color::Idx(i) if bold && i < 8 => indexed(i + 8),
        Color::Idx(i) => indexed(i),
    }
}

/// The xterm 256-color palette.
fn indexed(i: u8) -> [u8; 3] {
    const BASE: [[u8; 3]; 16] = [
        [0x00, 0x00, 0x00],
        [0xcd, 0x31, 0x31],
        [0x0d, 0xbc, 0x79],
        [0xe5, 0xe5, 0x10],
        [0x24, 0x72, 0xc8],
        [0xbc, 0x3f, 0xbc],
        [0x11, 0xa8, 0xcd],
        [0xe5, 0xe5, 0xe5],
        [0x66, 0x66, 0x66],
        [0xf1, 0x4c, 0x4c],
        [0x23, 0xd1, 0x8b],
        [0xf5, 0xf5, 0x43],
        [0x3b, 0x8e, 0xea],
        [0xd6, 0x70, 0xd6],
        [0x29, 0xb8, 0xdb],
        [0xff, 0xff, 0xff],
    ];
    match i {
        0..=15 => BASE[usize::from(i)],
        16..=231 => {
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            let i = i - 16;
            [level(i / 36), level(i / 6 % 6), level(i % 6)]
        }
        _ => [8 + (i - 232) * 10; 3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(cols: u16, rows: u16, ansi: &str) -> vt100::Parser {
        let mut parser = vt100::Parser::new(rows, cols, 0);
        parser.process(ansi.as_bytes());
        parser
    }

    fn pixel(raster: &Raster, picture: &Picture, col: usize, row: usize, dx: usize, dy: usize) -> [u8; 3] {
        let (x, y) = (col * raster.cell_w + dx, row * raster.cell_h + dy);
        let at = (y * picture.width as usize + x) * 3;
        picture.rgb[at..at + 3].try_into().unwrap()
    }

    fn cell_pixels(raster: &Raster, picture: &Picture, col: usize, row: usize) -> Vec<[u8; 3]> {
        let mut pixels = Vec::new();
        for dy in 0..raster.cell_h {
            for dx in 0..raster.cell_w {
                pixels.push(pixel(raster, picture, col, row, dx, dy));
            }
        }
        pixels
    }

    #[test]
    fn picture_is_one_font_cell_per_terminal_cell() {
        let (mut raster, err) = Raster::load();
        assert!(err.is_none());
        let parser = screen(4, 3, "\x1b[?25l");
        let picture = raster.render(parser.screen(), 4, 3);
        assert_eq!((picture.width, picture.height), (4 * raster.cell_w as u32, 3 * raster.cell_h as u32));
        assert!(picture.rgb.chunks(3).all(|p| p == DEFAULT_BG));
    }

    #[test]
    fn draws_glyph_in_its_cell_only() {
        let (mut raster, _) = Raster::load();
        let parser = screen(3, 1, "\x1b[?25l X");
        let picture = raster.render(parser.screen(), 3, 1);
        assert!(cell_pixels(&raster, &picture, 0, 0).iter().all(|&p| p == DEFAULT_BG));
        assert!(cell_pixels(&raster, &picture, 1, 0).iter().any(|&p| p != DEFAULT_BG));
        assert!(cell_pixels(&raster, &picture, 2, 0).iter().all(|&p| p == DEFAULT_BG));
    }

    #[test]
    fn fills_background_color() {
        let (mut raster, _) = Raster::load();
        let parser = screen(2, 1, "\x1b[?25l\x1b[41m \x1b[48;2;1;2;3m ");
        let picture = raster.render(parser.screen(), 2, 1);
        assert!(cell_pixels(&raster, &picture, 0, 0).iter().all(|&p| p == indexed(1)));
        assert!(cell_pixels(&raster, &picture, 1, 0).iter().all(|&p| p == [1, 2, 3]));
    }

    #[test]
    fn visible_cursor_inverts_its_cell() {
        let (mut raster, _) = Raster::load();
        let parser = screen(2, 1, "");
        let picture = raster.render(parser.screen(), 2, 1);
        assert!(cell_pixels(&raster, &picture, 0, 0).iter().all(|&p| p == DEFAULT_FG));
        assert!(cell_pixels(&raster, &picture, 1, 0).iter().all(|&p| p == DEFAULT_BG));
    }

    #[test]
    fn missing_glyph_draws_a_box() {
        let (mut raster, _) = Raster::load();
        // A private-use character the bundled font has no glyph for.
        let parser = screen(1, 1, "\x1b[?25l\u{f0000}");
        let picture = raster.render(parser.screen(), 1, 1);
        assert_eq!(pixel(&raster, &picture, 0, 0, 1, 1), DEFAULT_FG);
        assert_eq!(pixel(&raster, &picture, 0, 0, 0, 0), DEFAULT_BG);
        assert_eq!(pixel(&raster, &picture, 0, 0, raster.cell_w / 2, raster.cell_h / 2), DEFAULT_BG);
    }

    #[test]
    fn rejects_bytes_that_are_not_a_font() {
        assert!(Raster::from_font(&b"not a font"[..]).is_err());
    }

    #[test]
    fn palette_matches_xterm_cube_and_grays() {
        assert_eq!(indexed(16), [0, 0, 0]);
        assert_eq!(indexed(196), [255, 0, 0]);
        assert_eq!(indexed(232), [8, 8, 8]);
        assert_eq!(indexed(255), [238, 238, 238]);
    }
}
