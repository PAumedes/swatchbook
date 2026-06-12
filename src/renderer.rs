//! Swatch grid layout engine and Cairo/Pango renderer.
//!
//! This module is intentionally kept free of parser/token imports — it takes
//! a plain `&[RenderCard]` slice so it can be unit-tested without GTK.

use gtk::cairo;
use gtk::pango;
use pangocairo::functions as pc;

// ── Public input types ────────────────────────────────────────────────────────

/// A single colour item ready to be rendered — components plus display strings.
/// Kept as a separate type so `to_css_variables` stays colour-only.
#[derive(Debug, Clone)]
pub struct SwatchItem {
    pub name: String,
    /// `#rrggbb` when opaque, `#rrggbbaa` when transparent — lowercase.
    pub hex: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// Alpha channel, 0..=255 (255 = fully opaque).
    pub a: u8,
}

/// A design-token card to be rendered on the canvas.
///
/// `Color` wraps the existing swatch. The other variants render as purpose-built
/// preview cards that visualise the token's meaning rather than just its value.
#[derive(Debug, Clone)]
pub enum RenderCard {
    Color(SwatchItem),
    Font {
        name: String,
        family: String,
        size_px: f64,
        weight: u16,
        line_height: Option<f64>,
        /// Original text after `font:`, shown as the value label.
        display: String,
    },
    Space {
        name: String,
        value_px: f64,
        display: String,
    },
    Radius {
        name: String,
        value_px: f64,
        display: String,
    },
    Shadow {
        name: String,
        css: String,
    },
}

impl RenderCard {
    pub fn name(&self) -> &str {
        match self {
            RenderCard::Color(item) => &item.name,
            RenderCard::Font { name, .. } => name,
            RenderCard::Space { name, .. } => name,
            RenderCard::Radius { name, .. } => name,
            RenderCard::Shadow { name, .. } => name,
        }
    }

    /// The short value string shown below the card name and copied to clipboard.
    pub fn value_label(&self) -> String {
        match self {
            RenderCard::Color(item) => item.hex.clone(),
            RenderCard::Font { display, .. } => display.clone(),
            RenderCard::Space { display, .. } => display.clone(),
            RenderCard::Radius { display, .. } => display.clone(),
            RenderCard::Shadow { css, .. } => css.clone(),
        }
    }

    /// The text that lands on the clipboard when the card is clicked or Enter is pressed.
    pub fn copy_value(&self) -> String {
        self.value_label()
    }

    /// Return the inner `SwatchItem` for colour cards (used by CSS export).
    pub fn as_color(&self) -> Option<&SwatchItem> {
        match self {
            RenderCard::Color(item) => Some(item),
            _ => None,
        }
    }
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

/// Total pixel height needed to render `count` swatches at the given `width`.
///
/// Pure function — safe to call without a live GTK canvas.
pub fn content_height(count: usize, width: f64) -> f64 {
    if count == 0 {
        return 0.0;
    }
    let rects = layout(count, width);
    let last = rects
        .last()
        .expect("non-empty count yields non-empty rects");
    // last swatch bottom + name label + hex label + bottom padding
    last.y + SWATCH_H + LABEL_GAP + LABEL_H + LABEL_GAP + HEX_H + PADDING
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
            SwatchRect {
                x,
                y,
                w: swatch_w,
                h: SWATCH_H,
            }
        })
        .collect()
}

// ── Rendering ────────────────────────────────────────────────────────────────

/// Render a list of design-token cards into a Cairo context.
///
/// `dark_mode` flips the label foreground for contrast.
/// `focused` draws an accent-coloured focus ring around that card index.
pub fn render(
    cr: &cairo::Context,
    cards: &[RenderCard],
    width: f64,
    _height: f64,
    dark_mode: bool,
    focused: Option<usize>,
) {
    if cards.is_empty() {
        return;
    }

    let (label_a, secondary_a) = (0.87_f64, 0.55_f64);
    let (label_rgb, secondary_rgb): ((f64, f64, f64), (f64, f64, f64)) = if dark_mode {
        ((1.0, 1.0, 1.0), (1.0, 1.0, 1.0))
    } else {
        ((0.0, 0.0, 0.0), (0.0, 0.0, 0.0))
    };

    let rects = layout(cards.len(), width);

    for (i, (card, rect)) in cards.iter().zip(rects.iter()).enumerate() {
        // Focus ring — drawn behind the card
        if focused == Some(i) {
            rounded_rect(
                cr,
                rect.x - 3.0,
                rect.y - 3.0,
                rect.w + 6.0,
                rect.h + 6.0,
                RADIUS + 3.0,
            );
            cr.set_source_rgba(0.208, 0.518, 0.894, 1.0); // #3584e4 GNOME accent blue
            cr.set_line_width(3.0);
            let _ = cr.stroke();
        }

        // Card body — dispatches per token type
        match card {
            RenderCard::Color(item) => render_color_card(cr, item, rect),
            RenderCard::Font { .. } => render_font_card(cr, card, rect, dark_mode),
            RenderCard::Space { .. } => render_space_card(cr, card, rect, dark_mode),
            RenderCard::Radius { .. } => render_radius_card(cr, card, rect, dark_mode),
            RenderCard::Shadow { .. } => render_shadow_card(cr, card, rect, dark_mode),
        }

        let text_y = rect.y + rect.h + LABEL_GAP;

        // Name label
        let (lr, lg, lb) = label_rgb;
        cr.set_source_rgba(lr, lg, lb, label_a);
        draw_text(cr, card.name(), rect.x, text_y, rect.w, 11.0, true);

        // Value label
        let (sr, sg, sb) = secondary_rgb;
        cr.set_source_rgba(sr, sg, sb, secondary_a);
        draw_text(
            cr,
            &card.value_label(),
            rect.x,
            text_y + LABEL_H + LABEL_GAP,
            rect.w,
            10.0,
            false,
        );
    }
}

fn render_color_card(cr: &cairo::Context, item: &SwatchItem, rect: &SwatchRect) {
    let r = item.r as f64 / 255.0;
    let g = item.g as f64 / 255.0;
    let b = item.b as f64 / 255.0;
    let alpha = item.a as f64 / 255.0;

    if item.a < 255 {
        rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
        let _ = cr.save();
        cr.clip();
        draw_checkerboard(cr, rect);
        let _ = cr.restore();
    }
    rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
    cr.set_source_rgba(r, g, b, alpha);
    let _ = cr.fill();
}

fn render_font_card(
    cr: &cairo::Context,
    card: &RenderCard,
    rect: &SwatchRect,
    dark_mode: bool,
) {
    let (family, size_px, weight) = match card {
        RenderCard::Font {
            family,
            size_px,
            weight,
            ..
        } => (family.as_str(), *size_px, *weight),
        _ => return,
    };

    // Background
    rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
    if dark_mode {
        cr.set_source_rgb(0.15, 0.15, 0.18);
    } else {
        cr.set_source_rgb(0.93, 0.93, 0.95);
    }
    let _ = cr.fill();

    // Type specimen — "Ag" rendered in the target font, scaled to fit the card.
    let layout = pc::create_layout(cr);
    let mut desc = pango::FontDescription::new();
    desc.set_family(family);
    // Cap display size to avoid overflowing the card; minimum 14px for legibility.
    let display_size = size_px.clamp(14.0, 48.0);
    desc.set_absolute_size(display_size * pango::SCALE as f64);
    desc.set_weight(pango::Weight::__Unknown(weight as i32));
    layout.set_font_description(Some(&desc));
    layout.set_text("Ag");
    layout.set_alignment(pango::Alignment::Center);
    layout.set_width((rect.w * pango::SCALE as f64) as i32);

    let (_, text_h) = layout.pixel_size();
    let text_y = rect.y + (rect.h - text_h as f64) / 2.0;
    if dark_mode {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.87);
    } else {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.87);
    }
    cr.move_to(rect.x, text_y);
    pc::show_layout(cr, &layout);
}

fn render_space_card(
    cr: &cairo::Context,
    card: &RenderCard,
    rect: &SwatchRect,
    dark_mode: bool,
) {
    let value_px = match card {
        RenderCard::Space { value_px, .. } => *value_px,
        _ => return,
    };

    rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
    if dark_mode {
        cr.set_source_rgb(0.15, 0.15, 0.18);
    } else {
        cr.set_source_rgb(0.93, 0.93, 0.95);
    }
    let _ = cr.fill();

    // Horizontal bar whose width represents the spacing value.
    // 64 px or the card interior width at 100 % — whichever is smaller.
    let max_bar = rect.w - 2.0 * PADDING;
    let fill_ratio = (value_px / 64.0).min(1.0);
    let bar_w = (fill_ratio * max_bar).max(2.0);
    let bar_h = 20.0;
    let bar_x = rect.x + PADDING;
    let bar_y = rect.y + (rect.h - bar_h) / 2.0;

    // Filled bar
    cr.rectangle(bar_x, bar_y, bar_w, bar_h);
    cr.set_source_rgba(0.208, 0.518, 0.894, 0.75);
    let _ = cr.fill();

    // End tick marks
    cr.set_source_rgba(0.208, 0.518, 0.894, 1.0);
    cr.set_line_width(1.5);
    for tick_x in [bar_x, bar_x + bar_w] {
        cr.move_to(tick_x, bar_y - 5.0);
        cr.line_to(tick_x, bar_y + bar_h + 5.0);
    }
    let _ = cr.stroke();
}

fn render_radius_card(
    cr: &cairo::Context,
    card: &RenderCard,
    rect: &SwatchRect,
    dark_mode: bool,
) {
    let value_px = match card {
        RenderCard::Radius { value_px, .. } => *value_px,
        _ => return,
    };

    rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
    if dark_mode {
        cr.set_source_rgb(0.15, 0.15, 0.18);
    } else {
        cr.set_source_rgb(0.93, 0.93, 0.95);
    }
    let _ = cr.fill();

    // Square preview box with the target corner radius
    let box_size = ((rect.w - 4.0 * PADDING).min(rect.h - 4.0 * PADDING)).max(20.0);
    let box_x = rect.x + (rect.w - box_size) / 2.0;
    let box_y = rect.y + (rect.h - box_size) / 2.0;
    let clamped_r = value_px.min(box_size / 2.0);

    // Fill
    rounded_rect(cr, box_x, box_y, box_size, box_size, clamped_r);
    cr.set_source_rgba(0.208, 0.518, 0.894, 0.18);
    let _ = cr.fill();
    // Stroke
    rounded_rect(cr, box_x, box_y, box_size, box_size, clamped_r);
    cr.set_source_rgba(0.208, 0.518, 0.894, 0.85);
    cr.set_line_width(2.0);
    let _ = cr.stroke();
}

fn render_shadow_card(
    cr: &cairo::Context,
    card: &RenderCard,
    rect: &SwatchRect,
    dark_mode: bool,
) {
    // Background
    rounded_rect(cr, rect.x, rect.y, rect.w, rect.h, RADIUS);
    if dark_mode {
        cr.set_source_rgb(0.15, 0.15, 0.18);
    } else {
        cr.set_source_rgb(0.93, 0.93, 0.95);
    }
    let _ = cr.fill();

    // White preview card with a layered-offset shadow approximation
    let margin = 2.5 * PADDING;
    let box_x = rect.x + margin;
    let box_y = rect.y + margin;
    let box_w = rect.w - 2.0 * margin;
    let box_h = rect.h - 2.0 * margin;

    // Shadow layers — offset rectangles at decreasing opacity
    for layer in 0..6u32 {
        let spread = layer as f64;
        cr.rectangle(
            box_x + 2.0 + spread,
            box_y + 4.0 + spread,
            box_w,
            box_h,
        );
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.035);
        let _ = cr.fill();
    }

    // White card on top
    cr.rectangle(box_x, box_y, box_w, box_h);
    cr.set_source_rgb(1.0, 1.0, 1.0);
    let _ = cr.fill();

    // Suppress the unused-variable warning for dark_mode in the render path.
    let _ = dark_mode;
    let _ = card;
}

// ── Export ───────────────────────────────────────────────────────────────────

/// Render design-token cards to a PNG file at 2× resolution.
pub fn export_png(
    cards: &[RenderCard],
    width: u32,
    height: u32,
    path: &std::path::Path,
) -> Result<(), String> {
    let scale = 2.0_f64;
    let surf = cairo::ImageSurface::create(
        cairo::Format::Rgb24,
        (width as f64 * scale) as i32,
        (height as f64 * scale) as i32,
    )
    .map_err(|e| format!("create surface: {e}"))?;

    let cr = cairo::Context::new(&surf).map_err(|e| format!("create context: {e}"))?;
    cr.scale(scale, scale);
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().map_err(|e| format!("paint: {e}"))?;

    render(&cr, cards, width as f64, height as f64, false, None);

    let mut file = std::fs::File::create(path).map_err(|e| format!("create file: {e}"))?;
    surf.write_to_png(&mut file)
        .map_err(|e| format!("write png: {e}"))?;
    Ok(())
}

/// Render design-token cards to an SVG file.
pub fn export_svg(
    cards: &[RenderCard],
    width: u32,
    height: u32,
    path: &std::path::Path,
) -> Result<(), String> {
    let surf = cairo::SvgSurface::new(width as f64, height as f64, Some(path))
        .map_err(|e| format!("create svg surface: {e}"))?;

    let cr = cairo::Context::new(&surf).map_err(|e| format!("create context: {e}"))?;
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.paint().map_err(|e| format!("paint: {e}"))?;

    render(&cr, cards, width as f64, height as f64, false, None);
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
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for item in items {
        // Split on non-alphanumeric runs so "Hello  World" → "hello-world" (no double dash).
        let base: String = item
            .name
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        // Fall back to the hex value (without #) when the name is all punctuation.
        let base = if base.is_empty() {
            item.hex.trim_start_matches('#').to_string()
        } else {
            base
        };

        // Deduplicate: "primary", "primary-2", "primary-3", …
        let count = seen.entry(base.clone()).or_insert(0);
        *count += 1;
        let slug = if *count == 1 {
            base
        } else {
            format!("{base}-{count}")
        };

        out.push_str(&format!("  --color-{slug}: {};\n", item.hex));
    }
    out.push('}');
    out
}

/// Export all design tokens as a W3C Design Tokens Community Group JSON file.
///
/// Each card becomes one top-level token entry:
/// - Color  → `"$type": "color"`, `"$value": "#rrggbb"`
/// - Font   → `"$type": "typography"`, `"$value": {fontFamily, fontSize, fontWeight[, lineHeight]}`
/// - Space  → `"$type": "dimension"`, `"$value": "8px"`
/// - Radius → `"$type": "borderRadius"`, `"$value": "6px"`
/// - Shadow → `"$type": "shadow"`, `"$value": "0 2px 8px …"`
pub fn to_design_tokens_json(cards: &[RenderCard]) -> String {
    let mut out = String::from("{\n");
    let slugs = unique_slugs(cards);

    for (card, slug) in cards.iter().zip(slugs.iter()) {
        match card {
            RenderCard::Color(item) => {
                out.push_str(&format!(
                    "  \"{slug}\": {{ \"$type\": \"color\", \"$value\": \"{}\" }},\n",
                    item.hex
                ));
            }
            RenderCard::Font {
                family,
                size_px,
                weight,
                line_height,
                ..
            } => {
                let size_str = format!("{size_px}px");
                let fam = family.replace('"', "\\\"");
                let lh_field = match line_height {
                    Some(lh) => format!(", \"lineHeight\": {lh}"),
                    None => String::new(),
                };
                out.push_str(&format!(
                    "  \"{slug}\": {{ \"$type\": \"typography\", \"$value\": \
                     {{ \"fontFamily\": \"{fam}\", \"fontSize\": \"{size_str}\", \
                     \"fontWeight\": {weight}{lh_field} }} }},\n"
                ));
            }
            RenderCard::Space { display, .. } => {
                out.push_str(&format!(
                    "  \"{slug}\": {{ \"$type\": \"dimension\", \"$value\": \"{display}\" }},\n"
                ));
            }
            RenderCard::Radius { display, .. } => {
                out.push_str(&format!(
                    "  \"{slug}\": {{ \"$type\": \"borderRadius\", \"$value\": \"{display}\" }},\n"
                ));
            }
            RenderCard::Shadow { css, .. } => {
                let css_esc = css.replace('"', "\\\"");
                out.push_str(&format!(
                    "  \"{slug}\": {{ \"$type\": \"shadow\", \"$value\": \"{css_esc}\" }},\n"
                ));
            }
        }
    }

    // Strip trailing comma from the last entry so the JSON is valid.
    if out.ends_with(",\n") {
        out.truncate(out.len() - 2);
        out.push('\n');
    }
    out.push('}');
    out
}

/// Export colour tokens as a GIMP palette file (`.gpl`).
///
/// Non-colour tokens (Font, Space, Radius, Shadow) are silently skipped.
pub fn to_gimp_palette(cards: &[RenderCard], palette_name: &str) -> String {
    let mut out = String::from("GIMP Palette\n");
    out.push_str(&format!("Name: {palette_name}\n"));
    out.push_str("Columns: 5\n#\n");

    for card in cards {
        let RenderCard::Color(item) = card else {
            continue;
        };
        out.push_str(&format!(
            "{:>3} {:>3} {:>3}\t{}\n",
            item.r, item.g, item.b, item.name
        ));
    }
    out
}

/// Export colour tokens as a Tailwind CSS `theme.colors` config snippet (`.js`).
///
/// Non-colour tokens are silently skipped. The output is a self-contained JS
/// module that can be merged into an existing `tailwind.config.js`.
pub fn to_tailwind_config(cards: &[RenderCard]) -> String {
    let mut out =
        String::from("/** @type {import('tailwindcss').Config} */\nmodule.exports = {\n  theme: {\n    colors: {\n");

    let color_cards: Vec<&RenderCard> = cards.iter().filter(|c| c.as_color().is_some()).collect();
    let slugs = unique_slugs(&color_cards.iter().map(|c| (*c).clone()).collect::<Vec<_>>());

    for (card, slug) in color_cards.iter().zip(slugs.iter()) {
        let Some(item) = card.as_color() else {
            continue;
        };
        out.push_str(&format!("      '{slug}': '{}',\n", item.hex));
    }

    out.push_str("    },\n  },\n};\n");
    out
}

/// Export colour tokens as an Adobe Swatch Exchange (`.ase`) binary file.
///
/// Non-colour tokens are silently skipped. Returns raw bytes ready to write to
/// a file — the format is binary so it cannot be represented as a `String`.
pub fn to_ase_palette(cards: &[RenderCard]) -> Vec<u8> {
    let colors: Vec<&SwatchItem> = cards.iter().filter_map(|c| c.as_color()).collect();

    let mut out = Vec::new();
    out.extend_from_slice(b"ASEF");
    out.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // version 1.0
    out.extend_from_slice(&(colors.len() as u32).to_be_bytes());

    for item in &colors {
        // Name as null-terminated UTF-16BE
        let name_utf16: Vec<u16> = item.name.encode_utf16().chain(std::iter::once(0)).collect();
        let name_bytes: Vec<u8> = name_utf16.iter().flat_map(|c| c.to_be_bytes()).collect();

        // block content = name_len_field(2) + name_bytes + model(4) + rgb(12) + type(2)
        let block_len = 2u32 + name_bytes.len() as u32 + 4 + 12 + 2;

        out.extend_from_slice(&[0x00, 0x01]); // block type: color entry
        out.extend_from_slice(&block_len.to_be_bytes());
        out.extend_from_slice(&(name_utf16.len() as u16).to_be_bytes());
        out.extend_from_slice(&name_bytes);
        out.extend_from_slice(b"RGB ");
        out.extend_from_slice(&(item.r as f32 / 255.0).to_be_bytes());
        out.extend_from_slice(&(item.g as f32 / 255.0).to_be_bytes());
        out.extend_from_slice(&(item.b as f32 / 255.0).to_be_bytes());
        out.extend_from_slice(&[0x00, 0x02]); // color type: normal
    }
    out
}

/// Parse a GIMP palette (`.gpl`) file into a Markdown binder document.
///
/// Each color line becomes `- **Name** — \`#rrggbb\``. Non-color lines are
/// silently ignored. Returns an empty document string on malformed input.
pub fn gpl_to_markdown(content: &str) -> String {
    let mut palette_name = "Imported Palette".to_string();
    let mut lines: Vec<String> = Vec::new();
    let mut in_colors = false;

    for line in content.lines() {
        if line.starts_with("GIMP Palette") || line.starts_with("Columns:") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("Name: ") {
            palette_name = rest.trim().to_string();
            continue;
        }
        if line.starts_with('#') {
            in_colors = true;
            continue;
        }
        if !in_colors {
            continue;
        }
        // Color line format: "  R   G   B\tName"
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() < 2 {
            continue;
        }
        let rgb: Vec<&str> = parts[0].split_whitespace().collect();
        if rgb.len() < 3 {
            continue;
        }
        let (Ok(r), Ok(g), Ok(b)) = (
            rgb[0].parse::<u8>(),
            rgb[1].parse::<u8>(),
            rgb[2].parse::<u8>(),
        ) else {
            continue;
        };
        let name = parts[1].trim();
        lines.push(format!("- **{name}** — `#{r:02x}{g:02x}{b:02x}`"));
    }

    format!("# {palette_name}\n\n{}\n", lines.join("\n"))
}

/// Parse a W3C Design Tokens JSON file into a Markdown binder document.
///
/// Supports the five token types Swatchbook knows: color, typography,
/// dimension (spacing), borderRadius, and shadow. Unknown types are ignored.
/// Returns `None` if the input is not valid JSON or has no recognized tokens.
pub fn design_tokens_json_to_markdown(content: &str) -> Option<String> {
    // Minimal recursive JSON value — avoids a serde_json dependency in the lib.
    let tokens = parse_json_object(content)?;

    let mut colors: Vec<String> = Vec::new();
    let mut fonts: Vec<String> = Vec::new();
    let mut spaces: Vec<String> = Vec::new();
    let mut radii: Vec<String> = Vec::new();
    let mut shadows: Vec<String> = Vec::new();

    for (raw_key, type_str, value_str) in &tokens {
        let name = kebab_to_title(raw_key);
        match type_str.as_str() {
            "color" => colors.push(format!("- **{name}** — `{value_str}`")),
            "typography" => fonts.push(format!("- **{name}** — `{value_str}`")),
            "dimension" => spaces.push(format!("- **{name}** — `{value_str}`")),
            "borderRadius" => radii.push(format!("- **{name}** — `radius: {value_str}`")),
            "shadow" => shadows.push(format!("- **{name}** — `shadow: {value_str}`")),
            _ => {}
        }
    }

    if colors.is_empty() && fonts.is_empty() && spaces.is_empty() && radii.is_empty() && shadows.is_empty() {
        return None;
    }

    let mut md = String::from("# Imported Design Tokens\n\n");
    let sections = [
        ("Colors", &colors),
        ("Typography", &fonts),
        ("Spacing", &spaces),
        ("Radius", &radii),
        ("Shadows", &shadows),
    ];
    for (heading, items) in &sections {
        if !items.is_empty() {
            md.push_str(&format!("## {heading}\n\n"));
            md.push_str(&items.join("\n"));
            md.push_str("\n\n");
        }
    }
    Some(md.trim_end().to_string() + "\n")
}

/// Naively extract `(key, $type, $value)` triples from a flat W3C tokens object.
///
/// This is a hand-rolled extractor that avoids a `serde_json` dependency.
/// It handles the common case where all tokens are at the top level and
/// `$value` is a simple string. Nested group objects are ignored.
fn parse_json_object(src: &str) -> Option<Vec<(String, String, String)>> {
    // Strip outer braces
    let src = src.trim();
    if !src.starts_with('{') || !src.ends_with('}') {
        return None;
    }
    let inner = &src[1..src.len() - 1];

    let mut results = Vec::new();

    // Split into "key": { ... } chunks — works for flat W3C token files.
    // We look for top-level "key" patterns, then grab the next {...} block.
    let mut chars = inner.chars().peekable();
    let mut current_key: Option<String> = None;

    while chars.peek().is_some() {
        skip_whitespace(&mut chars);
        if chars.peek() == Some(&'"') {
            let key = read_json_string(&mut chars)?;
            skip_whitespace(&mut chars);
            if chars.next() != Some(':') {
                continue;
            }
            skip_whitespace(&mut chars);
            if chars.peek() == Some(&'{') {
                let obj_str = read_json_block(&mut chars, '{', '}')?;
                if let (Some(type_v), Some(value_v)) =
                    (extract_string_field(&obj_str, "$type"), extract_string_field(&obj_str, "$value"))
                {
                    results.push((key.clone(), type_v, value_v));
                } else if extract_string_field(&obj_str, "$type").is_none() {
                    // Could be a group — skip for now
                }
                current_key = Some(key);
            } else {
                // Skip non-object values
                skip_json_value(&mut chars);
            }
        } else if chars.peek() == Some(&',') {
            chars.next();
        } else {
            chars.next();
        }
    }
    let _ = current_key; // silence unused warning
    Some(results)
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
        chars.next();
    }
}

fn read_json_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    if chars.next() != Some('"') {
        return None;
    }
    let mut s = String::new();
    let mut escaped = false;
    for c in chars.by_ref() {
        if escaped {
            s.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(s);
        } else {
            s.push(c);
        }
    }
    None
}

fn read_json_block(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    open: char,
    close: char,
) -> Option<String> {
    if chars.next() != Some(open) {
        return None;
    }
    let mut s = String::from(open);
    let mut depth = 1i32;
    let mut in_str = false;
    let mut escaped = false;
    for c in chars.by_ref() {
        if escaped {
            s.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' && in_str {
            escaped = true;
            s.push(c);
            continue;
        }
        if c == '"' {
            in_str = !in_str;
        }
        if !in_str {
            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;
                if depth == 0 {
                    s.push(c);
                    return Some(s);
                }
            }
        }
        s.push(c);
    }
    None
}

fn skip_json_value(chars: &mut std::iter::Peekable<std::str::Chars>) {
    skip_whitespace(chars);
    match chars.peek() {
        Some('"') => { let _ = read_json_string(chars); }
        Some('{') => { let _ = read_json_block(chars, '{', '}'); }
        Some('[') => { let _ = read_json_block(chars, '[', ']'); }
        _ => { while !matches!(chars.peek(), Some(',') | Some('}') | None) { chars.next(); } }
    }
}

/// Extract a plain string value for `key` from a flat JSON object string.
fn extract_string_field(obj: &str, key: &str) -> Option<String> {
    // Find `"key":` then the next `"value"`
    let needle = format!("\"{}\"", key);
    let pos = obj.find(&needle)?;
    let rest = &obj[pos + needle.len()..];
    let colon = rest.find(':')? + 1;
    let after = rest[colon..].trim_start();
    if after.starts_with('"') {
        let mut chars = after.chars().peekable();
        read_json_string(&mut chars)
    } else if after.starts_with('{') {
        // $value is an object (e.g. typography) — flatten to a font: spec
        flatten_typography_value(after)
    } else {
        None
    }
}

/// Convert a typography `$value` object into a `font:` token string.
fn flatten_typography_value(obj: &str) -> Option<String> {
    let family = extract_string_field(obj, "fontFamily").unwrap_or_else(|| "sans-serif".into());
    let size = extract_string_field(obj, "fontSize").unwrap_or_else(|| "16px".into());
    let weight_str = extract_string_field(obj, "fontWeight").unwrap_or_else(|| "400".into());
    let weight: u64 = weight_str.parse().unwrap_or(400);
    let weight_kw = weight_to_keyword(weight);
    let lh = extract_string_field(obj, "lineHeight");

    let mut spec = format!("font: {family} {weight_kw} {size}");
    if let Some(lh) = lh {
        spec.push('/');
        spec.push_str(&lh);
    }
    Some(spec)
}

fn weight_to_keyword(w: u64) -> &'static str {
    match w {
        100 => "thin",
        200 => "extralight",
        300 => "light",
        400 => "regular",
        500 => "medium",
        600 => "semibold",
        700 => "bold",
        800 => "extrabold",
        900 => "black",
        _ => "regular",
    }
}

fn kebab_to_title(s: &str) -> String {
    s.split(['-', '_'])
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build de-duplicated kebab-case slugs for a slice of cards.
///
/// Duplicate base slugs get a numeric suffix (`primary`, `primary-2`, …).
fn unique_slugs(cards: &[RenderCard]) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    cards
        .iter()
        .map(|card| {
            let base = name_to_slug(card.name(), &card.value_label());
            let count = seen.entry(base.clone()).or_insert(0);
            *count += 1;
            if *count == 1 {
                base
            } else {
                format!("{base}-{count}")
            }
        })
        .collect()
}

/// Convert a token name to a kebab-case slug, falling back to `fallback`
/// (typically the value string without `#`) when the name is all punctuation.
fn name_to_slug(name: &str, fallback: &str) -> String {
    let base: String = name
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if base.is_empty() {
        fallback.trim_start_matches('#').to_string()
    } else {
        base
    }
}

fn draw_text(
    cr: &cairo::Context,
    text: &str,
    x: f64,
    y: f64,
    max_w: f64,
    size_pt: f64,
    bold: bool,
) {
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

/// Paint a light/grey checkerboard inside `rect` — the conventional "this area
/// is transparent" backdrop. The caller is expected to have clipped to the
/// swatch shape first.
fn draw_checkerboard(cr: &cairo::Context, rect: &SwatchRect) {
    const CELL: f64 = 8.0;
    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.rectangle(rect.x, rect.y, rect.w, rect.h);
    let _ = cr.fill();

    cr.set_source_rgb(0.8, 0.8, 0.8);
    let cols = (rect.w / CELL).ceil() as usize;
    let rows = (rect.h / CELL).ceil() as usize;
    for row in 0..rows {
        for col in 0..cols {
            if (row + col) % 2 == 0 {
                continue;
            }
            cr.rectangle(
                rect.x + col as f64 * CELL,
                rect.y + row as f64 * CELL,
                CELL,
                CELL,
            );
        }
    }
    let _ = cr.fill();
}

fn rounded_rect(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    use std::f64::consts::PI;
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -0.5 * PI, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, 0.5 * PI);
    cr.arc(x + r, y + h - r, r, 0.5 * PI, PI);
    cr.arc(x + r, y + r, r, PI, 1.5 * PI);
    cr.close_path();
}
