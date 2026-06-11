use swatchbook::renderer::{content_height, layout, to_css_variables, SwatchItem};

// ── layout() ────────────────────────────────────────────────────────────────

#[test]
fn empty_items_produce_no_rects() {
    assert!(layout(0, 800.0).is_empty());
}

#[test]
fn five_items_fit_one_row() {
    let rects = layout(5, 800.0);
    assert_eq!(rects.len(), 5);
    // All in the same row → same y
    let y0 = rects[0].y;
    assert!(rects.iter().all(|r| r.y == y0));
}

#[test]
fn six_items_wrap_to_two_rows() {
    let rects = layout(6, 800.0);
    assert_eq!(rects.len(), 6);
    assert_ne!(rects[0].y, rects[5].y);
}

#[test]
fn swatch_widths_are_equal() {
    let rects = layout(3, 600.0);
    let w0 = rects[0].w;
    assert!(rects.iter().all(|r| (r.w - w0).abs() < 0.001));
}

#[test]
fn single_item_uses_full_width_minus_padding() {
    let rects = layout(1, 100.0);
    // PADDING=16 on each side → w = 100 - 32 = 68
    assert!((rects[0].w - 68.0).abs() < 0.001);
}

#[test]
fn max_five_columns() {
    // 10 items → 2 rows of 5
    let rects = layout(10, 1000.0);
    assert_eq!(rects.len(), 10);
    // Items 0..4 should all share y with rects[0]
    let row0_y = rects[0].y;
    for r in &rects[0..5] {
        assert_eq!(r.y, row0_y);
    }
    // Items 5..9 should be in a different row
    assert_ne!(rects[5].y, row0_y);
}

// ── content_height() ────────────────────────────────────────────────────────

#[test]
fn content_height_zero_for_empty() {
    assert_eq!(content_height(0, 800.0), 0.0);
}

#[test]
fn content_height_positive_for_one_item() {
    assert!(content_height(1, 800.0) > 0.0);
}

#[test]
fn content_height_grows_when_row_wraps() {
    let h_one_row = content_height(5, 800.0);
    let h_two_rows = content_height(6, 800.0);
    assert!(h_two_rows > h_one_row, "two rows must be taller than one");
}

#[test]
fn content_height_consistent_with_layout_last_rect() {
    // content_height must be >= the bottom of the last swatch rect
    let rects = layout(7, 600.0);
    let last = rects.last().unwrap();
    let h = content_height(7, 600.0);
    assert!(h > last.y + last.h, "height must clear the last swatch");
}

// ── to_css_variables() ──────────────────────────────────────────────────────

fn item(name: &str, hex: &str, r: u8, g: u8, b: u8) -> SwatchItem {
    SwatchItem { name: name.to_string(), hex: hex.to_string(), r, g, b, a: 255 }
}

#[test]
fn css_variables_basic() {
    let items = vec![item("Primary", "#3482e3", 0x34, 0x82, 0xe3)];
    let css = to_css_variables(&items);
    assert!(css.contains("--color-primary: #3482e3;"), "got: {css}");
    assert!(css.starts_with(":root {"));
    assert!(css.ends_with('}'));
}

#[test]
fn css_variables_collapses_spaces_to_single_dash() {
    let items = vec![item("Hello  World", "#ffffff", 255, 255, 255)];
    let css = to_css_variables(&items);
    assert!(css.contains("--color-hello-world:"), "got: {css}");
    assert!(!css.contains("--color-hello--world:"), "must not have double dash: {css}");
}

#[test]
fn css_variables_empty_slug_falls_back_to_hex() {
    // A name made of only punctuation produces an empty slug.
    let items = vec![item("---", "#aabbcc", 0xaa, 0xbb, 0xcc)];
    let css = to_css_variables(&items);
    assert!(css.contains("--color-aabbcc:"), "got: {css}");
}

#[test]
fn css_variables_deduplicates_names() {
    let items = vec![
        item("Primary", "#ff0000", 255, 0, 0),
        item("Primary", "#00ff00", 0, 255, 0),
        item("Primary", "#0000ff", 0, 0, 255),
    ];
    let css = to_css_variables(&items);
    assert!(css.contains("--color-primary:"),   "first occurrence, got: {css}");
    assert!(css.contains("--color-primary-2:"), "second occurrence, got: {css}");
    assert!(css.contains("--color-primary-3:"), "third occurrence, got: {css}");
}

#[test]
fn css_variables_trims_leading_trailing_separators() {
    let items = vec![item("  Trim Me  ", "#123456", 0x12, 0x34, 0x56)];
    let css = to_css_variables(&items);
    assert!(css.contains("--color-trim-me:"), "got: {css}");
}
