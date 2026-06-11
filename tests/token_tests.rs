use swatchbook::token::{extract_token, ColorValue, DesignToken};

// ── Color pass-through ────────────────────────────────────────────────────────

#[test]
fn token_color_hex_passes_through() {
    assert_eq!(
        extract_token("#ff0000"),
        Some(DesignToken::Color(ColorValue::Rgba(255, 0, 0, 255)))
    );
}

#[test]
fn token_color_named_passes_through() {
    let t = extract_token("red").unwrap();
    let Some(c) = t.as_color() else { panic!("expected Color") };
    assert_eq!(c.to_rgb(), (255, 0, 0));
}

// ── Font tokens ───────────────────────────────────────────────────────────────

#[test]
fn token_font_basic() {
    let t = extract_token("font: Inter 16px").unwrap();
    let DesignToken::Font {
        family, size_px, weight, ..
    } = t
    else {
        panic!("expected Font")
    };
    assert_eq!(family, "Inter");
    assert!((size_px - 16.0).abs() < 0.001);
    assert_eq!(weight, 400);
}

#[test]
fn token_font_bold_weight() {
    let t = extract_token("font: Inter Bold 24px").unwrap();
    let DesignToken::Font { weight, size_px, .. } = t else {
        panic!("expected Font")
    };
    assert_eq!(weight, 700);
    assert!((size_px - 24.0).abs() < 0.001);
}

#[test]
fn token_font_numeric_weight() {
    let t = extract_token("font: Inter 600 18px").unwrap();
    let DesignToken::Font { weight, .. } = t else {
        panic!("expected Font")
    };
    assert_eq!(weight, 600);
}

#[test]
fn token_font_line_height_unitless() {
    let t = extract_token("font: Inter 16px/1.5").unwrap();
    let DesignToken::Font { line_height, .. } = t else {
        panic!("expected Font")
    };
    assert!((line_height.unwrap() - 1.5).abs() < 0.001);
}

#[test]
fn token_font_quoted_family() {
    let t = extract_token("font: \"JetBrains Mono\" 14px").unwrap();
    let DesignToken::Font { family, size_px, .. } = t else {
        panic!("expected Font")
    };
    assert_eq!(family, "JetBrains Mono");
    assert!((size_px - 14.0).abs() < 0.001);
}

#[test]
fn token_font_no_family_defaults_to_sans_serif() {
    let t = extract_token("font: 16px").unwrap();
    let DesignToken::Font { family, .. } = t else {
        panic!("expected Font")
    };
    assert_eq!(family, "sans-serif");
}

#[test]
fn token_font_missing_size_returns_none() {
    assert!(extract_token("font: Inter Bold").is_none());
}

// ── Space tokens ──────────────────────────────────────────────────────────────

#[test]
fn token_space_px() {
    let t = extract_token("8px").unwrap();
    let DesignToken::Space { value_px, display } = t else {
        panic!("expected Space")
    };
    assert!((value_px - 8.0).abs() < 0.001);
    assert_eq!(display, "8px");
}

#[test]
fn token_space_rem() {
    let t = extract_token("0.5rem").unwrap();
    let DesignToken::Space { value_px, .. } = t else {
        panic!("expected Space")
    };
    assert!((value_px - 8.0).abs() < 0.001); // 0.5 × 16 = 8
}

#[test]
fn token_space_em() {
    let t = extract_token("2em").unwrap();
    let DesignToken::Space { value_px, .. } = t else {
        panic!("expected Space")
    };
    assert!((value_px - 32.0).abs() < 0.001); // 2 × 16 = 32
}

// ── Radius tokens ─────────────────────────────────────────────────────────────

#[test]
fn token_radius_px() {
    let t = extract_token("radius: 6px").unwrap();
    let DesignToken::Radius { value_px, display } = t else {
        panic!("expected Radius")
    };
    assert!((value_px - 6.0).abs() < 0.001);
    assert_eq!(display, "6px");
}

#[test]
fn token_radius_rem() {
    let t = extract_token("radius: 0.5rem").unwrap();
    let DesignToken::Radius { value_px, .. } = t else {
        panic!("expected Radius")
    };
    assert!((value_px - 8.0).abs() < 0.001);
}

#[test]
fn token_radius_empty_returns_none() {
    assert!(extract_token("radius:").is_none());
}

// ── Shadow tokens ─────────────────────────────────────────────────────────────

#[test]
fn token_shadow_basic() {
    let t = extract_token("shadow: 0 2px 8px rgba(0,0,0,0.15)").unwrap();
    let DesignToken::Shadow(css) = t else {
        panic!("expected Shadow")
    };
    assert_eq!(css, "0 2px 8px rgba(0,0,0,0.15)");
}

#[test]
fn token_shadow_empty_returns_none() {
    assert!(extract_token("shadow:").is_none());
    assert!(extract_token("shadow: ").is_none());
}

// ── Unknown / none ────────────────────────────────────────────────────────────

#[test]
fn token_unknown_text_returns_none() {
    assert!(extract_token("some random text").is_none());
    assert!(extract_token("42").is_none()); // bare number without unit
    assert!(extract_token("").is_none());
}

// ── fallback_name ─────────────────────────────────────────────────────────────

#[test]
fn fallback_name_color() {
    let t = DesignToken::Color(ColorValue::Rgba(255, 0, 0, 255));
    assert_eq!(t.fallback_name(), "#ff0000");
}

#[test]
fn fallback_name_space() {
    let t = DesignToken::Space {
        value_px: 8.0,
        display: "8px".into(),
    };
    assert_eq!(t.fallback_name(), "8px");
}
