use swatchbook::parser::parse;
use swatchbook::token::{extract_color, ColorValue};

#[test]
fn extract_hex6() {
    assert_eq!(
        extract_color("#E53935"),
        Some(ColorValue::Rgba(0xE5, 0x39, 0x35, 255))
    );
}

#[test]
fn extract_hex3() {
    assert_eq!(
        extract_color("#f00"),
        Some(ColorValue::Rgba(0xff, 0x00, 0x00, 255))
    );
}

#[test]
fn extract_hex8_carries_alpha() {
    // #rrggbbaa — the trailing pair is the alpha channel.
    assert_eq!(
        extract_color("#3482e380"),
        Some(ColorValue::Rgba(0x34, 0x82, 0xe3, 0x80))
    );
}

#[test]
fn extract_hex4_shorthand_alpha() {
    // #rgba — each nibble doubled, so `8` → `0x88`.
    assert_eq!(
        extract_color("#f008"),
        Some(ColorValue::Rgba(0xff, 0x00, 0x00, 0x88))
    );
}

#[test]
fn extract_rgb() {
    assert_eq!(
        extract_color("rgb(255, 0, 128)"),
        Some(ColorValue::Rgba(255, 0, 128, 255))
    );
}

#[test]
fn extract_rgba_float_alpha() {
    // CSS alpha is a 0..1 float; 0.5 → 128 (rounded).
    assert_eq!(
        extract_color("rgba(255, 0, 0, 0.5)"),
        Some(ColorValue::Rgba(255, 0, 0, 128))
    );
}

#[test]
fn extract_hsl_primary_red() {
    // hsl(0, 100%, 50%) is pure red.
    assert_eq!(
        extract_color("hsl(0, 100%, 50%)"),
        Some(ColorValue::Rgba(255, 0, 0, 255))
    );
}

#[test]
fn extract_hsl_grey_when_unsaturated() {
    // 0% saturation collapses to a grey at the given lightness.
    assert_eq!(
        extract_color("hsl(210, 0%, 50%)"),
        Some(ColorValue::Rgba(128, 128, 128, 255))
    );
}

#[test]
fn extract_transparent_keyword() {
    assert_eq!(
        extract_color("transparent"),
        Some(ColorValue::Rgba(0, 0, 0, 0))
    );
}

#[test]
fn extract_extended_named_color() {
    // rebeccapurple is part of the full 148-colour set, not the old 16.
    assert_eq!(
        extract_color("rebeccapurple").map(|c| c.to_rgb()),
        Some((102, 51, 153))
    );
}

#[test]
fn extract_named_red() {
    let c = extract_color("red").unwrap();
    assert_eq!(c.to_rgb(), (255, 0, 0));
}

#[test]
fn extract_unknown_returns_none() {
    assert_eq!(extract_color("notacolor"), None);
    assert_eq!(extract_color("#gg0000"), None);
    assert_eq!(extract_color("#12345"), None);
}

#[test]
fn to_hex_string_normalises() {
    assert_eq!(
        ColorValue::Rgba(0x3A, 0x82, 0xE3, 255).to_hex_string(),
        "#3a82e3"
    );
    assert_eq!(ColorValue::Named("blue".into()).to_hex_string(), "#0000ff");
}

#[test]
fn to_hex_string_includes_alpha_when_translucent() {
    assert_eq!(
        ColorValue::Rgba(0x34, 0x82, 0xe3, 0x80).to_hex_string(),
        "#3482e380"
    );
}

#[test]
fn empty_document() {
    let doc = parse("");
    assert!(doc.sections.is_empty());
    assert_eq!(doc.all_swatches().count(), 0);
}

#[test]
fn single_section_two_swatches() {
    let md = "# Palette\n\n- **Red** — `#E53935`\n- **Blue** — `#3482E3`\n";
    let doc = parse(md);
    assert_eq!(doc.sections.len(), 1);
    let s = &doc.sections[0];
    assert_eq!(s.heading, "Palette");
    assert_eq!(s.swatches.len(), 2);
    assert_eq!(s.swatches[0].color, ColorValue::Rgba(0xE5, 0x39, 0x35, 255));
    assert_eq!(s.swatches[1].color, ColorValue::Rgba(0x34, 0x82, 0xE3, 255));
}

#[test]
fn two_sections() {
    let md = "# Palette\n\n- Red — `#ff0000`\n\n## Typography\n\n- Body — `rgb(30, 30, 30)`\n";
    let doc = parse(md);
    assert_eq!(doc.sections.len(), 2);
    assert_eq!(
        doc.sections[1].swatches[0].color,
        ColorValue::Rgba(30, 30, 30, 255)
    );
}

#[test]
fn all_swatches_flattens() {
    let md = "# A\n\n- `#ff0000`\n\n# B\n\n- `#00ff00`\n- `#0000ff`\n";
    let doc = parse(md);
    assert_eq!(doc.all_swatches().count(), 3);
}

// ── clean_label / name extraction ────────────────────────────────────────────

#[test]
fn label_strips_bold_markers() {
    let md = "# P\n\n- **Primary** — `#ff0000`\n";
    let doc = parse(md);
    assert_eq!(doc.sections[0].swatches[0].name, "Primary");
}

#[test]
fn label_strips_trailing_dash_and_em_dash() {
    // Trailing " —" or " -" should be removed from the name.
    let md = "# P\n\n- Background — `#ffffff`\n";
    let doc = parse(md);
    assert_eq!(doc.sections[0].swatches[0].name, "Background");
}

#[test]
fn label_strips_italic_markers() {
    let md = "# P\n\n- *Accent* `#aabbcc`\n";
    let doc = parse(md);
    assert_eq!(doc.sections[0].swatches[0].name, "Accent");
}

#[test]
fn item_with_no_text_falls_back_to_normalised_hex() {
    // A bare colour token with no label text → name is the canonical hex.
    let md = "# P\n\n- `#E53935`\n";
    let doc = parse(md);
    let swatch = &doc.sections[0].swatches[0];
    // raw was E53935 uppercase; normalised hex is lowercase
    assert_eq!(swatch.name, "#e53935");
}

#[test]
fn last_color_token_wins_in_item() {
    // Only the final colour code in a list item is used.
    let md = "# P\n\n- See `#ff0000` or `#00ff00`\n";
    let doc = parse(md);
    assert_eq!(
        doc.sections[0].swatches[0].color,
        ColorValue::Rgba(0x00, 0xff, 0x00, 255)
    );
}

#[test]
fn item_without_color_is_ignored() {
    let md = "# P\n\n- No colour here\n- Red `#ff0000`\n";
    let doc = parse(md);
    assert_eq!(doc.sections[0].swatches.len(), 1);
    assert_eq!(doc.sections[0].swatches[0].name, "Red");
}
