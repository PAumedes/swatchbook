use swatchbook::renderer::layout;

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
