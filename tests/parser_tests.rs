use swatchbook::parser::parse;
use swatchbook::token::{extract_color, ColorValue};

#[test]
fn extract_hex6() {
    assert_eq!(extract_color("#E53935"), Some(ColorValue::Hex(0xE5, 0x39, 0x35)));
}

#[test]
fn extract_hex3() {
    assert_eq!(extract_color("#f00"), Some(ColorValue::Hex(0xff, 0x00, 0x00)));
}

#[test]
fn extract_rgb() {
    assert_eq!(extract_color("rgb(255, 0, 128)"), Some(ColorValue::Rgb(255, 0, 128)));
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
    assert_eq!(ColorValue::Hex(0x3A, 0x82, 0xE3).to_hex_string(), "#3a82e3");
    assert_eq!(ColorValue::Named("blue".into()).to_hex_string(), "#0000ff");
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
    assert_eq!(s.swatches[0].color, ColorValue::Hex(0xE5, 0x39, 0x35));
    assert_eq!(s.swatches[1].color, ColorValue::Hex(0x34, 0x82, 0xE3));
}

#[test]
fn two_sections() {
    let md = "# Palette\n\n- Red — `#ff0000`\n\n## Typography\n\n- Body — `rgb(30, 30, 30)`\n";
    let doc = parse(md);
    assert_eq!(doc.sections.len(), 2);
    assert_eq!(doc.sections[1].swatches[0].color, ColorValue::Rgb(30, 30, 30));
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
    assert_eq!(doc.sections[0].swatches[0].color, ColorValue::Hex(0x00, 0xff, 0x00));
}

#[test]
fn item_without_color_is_ignored() {
    let md = "# P\n\n- No colour here\n- Red `#ff0000`\n";
    let doc = parse(md);
    assert_eq!(doc.sections[0].swatches.len(), 1);
    assert_eq!(doc.sections[0].swatches[0].name, "Red");
}
