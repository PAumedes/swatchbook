use swatchbook::renderer::{
    to_css_variables, to_design_tokens_json, to_gimp_palette, to_tailwind_config, RenderCard,
    SwatchItem,
};

// ── helpers ───────────────────────────────────────────────────────────────────

fn color(name: &str, hex: &str, r: u8, g: u8, b: u8) -> RenderCard {
    RenderCard::Color(SwatchItem {
        name: name.to_string(),
        hex: hex.to_string(),
        r,
        g,
        b,
        a: 255,
    })
}

fn font(name: &str, family: &str, size_px: f64, weight: u16) -> RenderCard {
    RenderCard::Font {
        name: name.to_string(),
        family: family.to_string(),
        size_px,
        weight,
        line_height: None,
        display: format!("{family} {size_px}px"),
    }
}

fn space(name: &str, value_px: f64, display: &str) -> RenderCard {
    RenderCard::Space {
        name: name.to_string(),
        value_px,
        display: display.to_string(),
    }
}

fn radius(name: &str, value_px: f64, display: &str) -> RenderCard {
    RenderCard::Radius {
        name: name.to_string(),
        value_px,
        display: display.to_string(),
    }
}

fn shadow(name: &str, css: &str) -> RenderCard {
    RenderCard::Shadow {
        name: name.to_string(),
        css: css.to_string(),
    }
}

// ── to_design_tokens_json ─────────────────────────────────────────────────────

#[test]
fn json_color_token() {
    let cards = vec![color("Primary", "#3482e3", 0x34, 0x82, 0xe3)];
    let json = to_design_tokens_json(&cards);
    assert!(json.contains("\"primary\""), "got: {json}");
    assert!(json.contains("\"$type\": \"color\""), "got: {json}");
    assert!(json.contains("\"$value\": \"#3482e3\""), "got: {json}");
}

#[test]
fn json_font_token() {
    let cards = vec![RenderCard::Font {
        name: "Body".into(),
        family: "Inter".into(),
        size_px: 16.0,
        weight: 400,
        line_height: Some(1.5),
        display: "Inter 16px/1.5".into(),
    }];
    let json = to_design_tokens_json(&cards);
    assert!(json.contains("\"body\""), "got: {json}");
    assert!(json.contains("\"$type\": \"typography\""), "got: {json}");
    assert!(json.contains("\"fontFamily\": \"Inter\""), "got: {json}");
    assert!(json.contains("\"fontSize\": \"16px\""), "got: {json}");
    assert!(json.contains("\"fontWeight\": 400"), "got: {json}");
    assert!(json.contains("\"lineHeight\": 1.5"), "got: {json}");
}

#[test]
fn json_font_no_line_height() {
    let cards = vec![font("Heading", "Inter", 24.0, 700)];
    let json = to_design_tokens_json(&cards);
    assert!(!json.contains("lineHeight"), "should omit absent lineHeight: {json}");
}

#[test]
fn json_space_token() {
    let cards = vec![space("gap-md", 8.0, "8px")];
    let json = to_design_tokens_json(&cards);
    assert!(json.contains("\"gap-md\""), "got: {json}");
    assert!(json.contains("\"$type\": \"dimension\""), "got: {json}");
    assert!(json.contains("\"$value\": \"8px\""), "got: {json}");
}

#[test]
fn json_radius_token() {
    let cards = vec![radius("button", 6.0, "6px")];
    let json = to_design_tokens_json(&cards);
    assert!(json.contains("\"$type\": \"borderRadius\""), "got: {json}");
    assert!(json.contains("\"$value\": \"6px\""), "got: {json}");
}

#[test]
fn json_shadow_token() {
    let cards = vec![shadow("card", "0 2px 8px rgba(0,0,0,0.12)")];
    let json = to_design_tokens_json(&cards);
    assert!(json.contains("\"$type\": \"shadow\""), "got: {json}");
    assert!(
        json.contains("\"$value\": \"0 2px 8px rgba(0,0,0,0.12)\""),
        "got: {json}"
    );
}

#[test]
fn json_all_token_types() {
    let cards = vec![
        color("Primary", "#3482e3", 0x34, 0x82, 0xe3),
        font("Body", "Inter", 16.0, 400),
        space("Gap", 8.0, "8px"),
        radius("Btn", 6.0, "6px"),
        shadow("Card", "0 2px 8px rgba(0,0,0,0.12)"),
    ];
    let json = to_design_tokens_json(&cards);
    // Valid JSON: starts and ends correctly, no trailing comma before }
    assert!(json.starts_with('{'), "got: {json}");
    assert!(json.ends_with('}'), "got: {json}");
    assert!(!json.contains(",\n}"), "trailing comma: {json}");
}

#[test]
fn json_deduplicates_names() {
    let cards = vec![
        color("Primary", "#ff0000", 255, 0, 0),
        color("Primary", "#00ff00", 0, 255, 0),
    ];
    let json = to_design_tokens_json(&cards);
    assert!(json.contains("\"primary\""), "got: {json}");
    assert!(json.contains("\"primary-2\""), "got: {json}");
}

#[test]
fn json_empty_is_empty_object() {
    let json = to_design_tokens_json(&[]);
    // Valid JSON — whitespace between braces is acceptable.
    assert!(json.starts_with('{') && json.ends_with('}'), "got: {json}");
    assert!(!json.contains('"'), "should have no keys: {json}");
}

// ── to_gimp_palette ───────────────────────────────────────────────────────────

#[test]
fn gimp_palette_header() {
    let cards = vec![color("Red", "#ff0000", 255, 0, 0)];
    let gpl = to_gimp_palette(&cards, "Test Palette");
    assert!(gpl.starts_with("GIMP Palette\n"), "got: {gpl}");
    assert!(gpl.contains("Name: Test Palette\n"), "got: {gpl}");
    assert!(gpl.contains("Columns: 5\n#\n"), "got: {gpl}");
}

#[test]
fn gimp_palette_color_line() {
    let cards = vec![color("Primary", "#3482e3", 0x34, 0x82, 0xe3)];
    let gpl = to_gimp_palette(&cards, "Tokens");
    // Line: " 52 130 227\tPrimary\n"
    assert!(
        gpl.contains("52") && gpl.contains("130") && gpl.contains("227"),
        "got: {gpl}"
    );
    assert!(gpl.contains("Primary"), "got: {gpl}");
}

#[test]
fn gimp_palette_skips_non_color_tokens() {
    let cards = vec![
        color("Red", "#ff0000", 255, 0, 0),
        font("Body", "Inter", 16.0, 400),
        space("Gap", 8.0, "8px"),
    ];
    let gpl = to_gimp_palette(&cards, "Tokens");
    // Only the color line should appear in the body (after the header)
    let body: String = gpl.lines().skip(4).collect::<Vec<_>>().join("\n");
    assert!(body.contains("Red"), "got body: {body}");
    assert!(!body.contains("Body"), "font should be skipped: {body}");
    assert!(!body.contains("Gap"), "space should be skipped: {body}");
}

// ── to_tailwind_config ────────────────────────────────────────────────────────

#[test]
fn tailwind_config_structure() {
    let cards = vec![color("Primary", "#3482e3", 0x34, 0x82, 0xe3)];
    let tw = to_tailwind_config(&cards);
    assert!(tw.contains("module.exports"), "got: {tw}");
    assert!(tw.contains("theme:"), "got: {tw}");
    assert!(tw.contains("colors:"), "got: {tw}");
    assert!(tw.contains("'primary': '#3482e3'"), "got: {tw}");
}

#[test]
fn tailwind_config_skips_non_color() {
    let cards = vec![
        color("Primary", "#3482e3", 0x34, 0x82, 0xe3),
        font("Body", "Inter", 16.0, 400),
        space("Gap", 8.0, "8px"),
    ];
    let tw = to_tailwind_config(&cards);
    assert!(tw.contains("primary"), "got: {tw}");
    assert!(!tw.contains("body"), "font should be skipped: {tw}");
    assert!(!tw.contains("gap"), "space should be skipped: {tw}");
}

#[test]
fn tailwind_config_deduplicates() {
    let cards = vec![
        color("Brand", "#ff0000", 255, 0, 0),
        color("Brand", "#00ff00", 0, 255, 0),
    ];
    let tw = to_tailwind_config(&cards);
    assert!(tw.contains("'brand'"), "got: {tw}");
    assert!(tw.contains("'brand-2'"), "got: {tw}");
}

// ── to_css_variables (regression — ensure it still works) ─────────────────────

#[test]
fn css_variables_still_works_with_swatch_item() {
    use swatchbook::renderer::SwatchItem;
    let items = vec![SwatchItem {
        name: "Primary".into(),
        hex: "#3482e3".into(),
        r: 0x34,
        g: 0x82,
        b: 0xe3,
        a: 255,
    }];
    let css = to_css_variables(&items);
    assert!(css.contains("--color-primary: #3482e3;"), "got: {css}");
}
