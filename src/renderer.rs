//! Swatch grid layout engine and Cairo/Pango renderer.
//!
//! This module is intentionally kept free of parser/token imports — it takes
//! a plain `&[SwatchItem]` slice so it can be unit-tested without GTK.

use gtk::cairo;
use gtk::pango;
use pangocairo::functions as pc;

// ── Public input type ────────────────────────────────────────────────────────

/// A single item ready to be rendered — colour components plus display strings.
#[derive(Debug, Clone)]
pub struct SwatchItem {
    pub name: String,
    /// Always `#rrggbb` lowercase.
    pub hex: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// ── Layout ───────────────────────────────────────────────────────────────────

const PADDING: f64 = 16.0;
const GAP: f64 = 12.0;
const SWATCH_H: f64 = 120.0;
const LABEL_H: f64 = 16.0;
const HEX_H: f64 = 14.0;
const LABEL_GAP: f64 = 4.0;
const RADIUS: f64 = 8.0;
const MAX_COLS: usize = 5;

/// A positioned swatch rectangle (pure geometry, no GTK).
#[derive(Debug, Clone)]
pub struct SwatchRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Compute swatch positions for `count` items in a canvas of `(width, height)`.
///
/// This is a pure function — no Cairo or GTK; fully unit-testable.
pub fn layout(count: usize, width: f64) -> Vec<SwatchRect> {
    if count == 0 {
        return vec![];
    }
    let cols = count.min(MAX_COLS);
    let total_gap = (cols - 1) as f64 * GAP;
    let swatch_w = ((width - 2.0 * PADDING - total_gap) / cols as f64).max(0.0);

    (0..count)
        .map(|i| {
            let col = i % cols;
            let row = i / cols;
            let x = PADDING + col as f64 * (swatch_w + GAP);
            let y = PADDING + row as f64 * (SWATCH_H + LABEL_H + HEX_H + LABEL_GAP * 2.0 + GAP);
            SwatchRect { x, y, w: swatch_w, h: SWATCH_H }
        })
        .collect()
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// Render a list of swatches into a Cairo context.
///
/// `dark_mode` flips the label foreground for contrast.
pub fn render(
    cr: &cairo::Context,
    items: &[SwatchItem],
    width: f64,
    _height: f64,
    dark_mode: bool,
) {
    if items.is_empty() {
        return;
    }

    let (label_a, secondary_a): (f64, f64) = if dark_mode {
        (0.87, 0.55)
    } else {
        (0.87, 0.55)
    };
    let (label_rgb, secondary_rgb): ((f64, f64, f64), (f64, f64, f64)) = if dark_mode {
        ((1.0, 1.0, 1.0), (1.0, 1.0, 1.0))
    } else {
        ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    };

    let rects = layout(items.len(), width);

    for (item, rect) in items.iter().zip(rects.iter()) {
        let r = item.r as f64 / 255.0;
        let g = item.g as f64 / 255.0;
        let b = item.b as f64 / 255.0;

        // Swatch fill
        rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
        cr.set_source_rgb(r, g, b);
        let _ = cr.fill();

        let text_y = rect.y + rect.h + LABEL_GAP;

        // Name label
        let (lr, lg, lb) = label_rgb;
        cr.set_source_rgba(lr, lg, lb, label_a);
        draw_text(cr, &item.name, rect.x, text_y, rect.w, 11.0, true);

        // Hex label
        let (sr, sg, sb) = secondary_rgb;
        cr.set_source_rgba(sr, sg, sb, secondary_a);
        draw_text(cr, &item.hex, rect.x, text_y + LABEL_H + LABEL_GAP, rect.w, 10.0, false);
    }
}

// ── Export ───────────────────────────────────────────────────────────────────

/// Render swatches to a PNG file at 2× resolution.
pub fn export_png(items: &[SwatchItem], width: u32, height: u32, path: &std::path::Path) -> Result<(), String> {
    let scale = 2.0_f64;
    let surf = cairo::ImageSurface::create(
        cairo::Format::Rgb24,
        (width as f64 * scale) as i32,
        (height as f64 * scale) as i32,
    )
    .map_err(|e| format!("create surface: {e}"))?;

    let cr = cairo::Context::new(&surf).map_err(|e| format!("create context: {e}"))?;
    cr.scale(scale, scale);
    // White background
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().map_err(|e| format!("paint: {e}"))?;

    render(&cr, items, width as f64, height as f64, false);

    let mut file = std::fs::File::create(path).map_err(|e| format!("create file: {e}"))?;
    surf.write_to_png(&mut file).map_err(|e| format!("write png: {e}"))?;
    Ok(())
}

/// Render swatches to an SVG file.
pub fn export_svg(items: &[SwatchItem], width: u32, height: u32, path: &std::path::Path) -> Result<(), String> {
    let surf = cairo::SvgSurface::new(width as f64, height as f64, Some(path))
        .map_err(|e| format!("create svg surface: {e}"))?;

    let cr = cairo::Context::new(&surf).map_err(|e| format!("create context: {e}"))?;
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().map_err(|e| format!("paint: {e}"))?;

    render(&cr, items, width as f64, height as f64, false);
    Ok(())
}

/// Generate CSS custom properties from swatches.
///
/// Produces a block like:
/// ```css
/// :root {
///   --color-primary: #3482e3;
///   --color-success: #2ec27e;
/// }
/// ```
pub fn to_css_variables(items: &[SwatchItem]) -> String {
    let mut out = String::from(":root {\n");
    for item in items {
        let slug = item.name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();
        let slug = slug.trim_matches('-').to_string();
        out.push_str(&format!("  --color-{slug}: {};\n", item.hex));
    }
    out.push('}');
    out
}

fn draw_text(cr: &cairo::Context, text: &str, x: f64, y: f64, max_w: f64, size_pt: f64, bold: bool) {
    let layout = pc::create_layout(cr);
    let mut desc = pango::FontDescription::new();
    desc.set_size((size_pt * pango::SCALE as f64) as i32);
    if bold {
        desc.set_weight(pango::Weight::Bold);
    }
    layout.set_font_description(Some(&desc));
    layout.set_width((max_w * pango::SCALE as f64) as i32);
    layout.set_ellipsize(pango::EllipsizeMode::End);
    layout.set_text(text);

    cr.move_to(x, y);
    pc::show_layout(cr, &layout);
}

fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::PI;
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r,     r, -0.5 * PI, 0.0);
    cr.arc(x + w - r, y + h - r, r,  0.0,      0.5 * PI);
    cr.arc(x + r,     y + h - r, r,  0.5 * PI, PI);
    cr.arc(x + r,     y + r,     r,  PI,        1.5 * PI);
    cr.close_path();
}
