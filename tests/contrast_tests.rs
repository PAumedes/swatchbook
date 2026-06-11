use swatchbook::token::{contrast_ratio, WcagLevel};

#[test]
fn black_on_white_is_maximum() {
    // The largest possible contrast ratio is 21:1.
    let ratio = contrast_ratio((0, 0, 0), (255, 255, 255));
    assert!((ratio - 21.0).abs() < 0.01, "got {ratio}");
}

#[test]
fn identical_colors_have_ratio_one() {
    let ratio = contrast_ratio((100, 100, 100), (100, 100, 100));
    assert!((ratio - 1.0).abs() < 0.001, "got {ratio}");
}

#[test]
fn ratio_is_symmetric() {
    let a = contrast_ratio((34, 130, 227), (255, 255, 255));
    let b = contrast_ratio((255, 255, 255), (34, 130, 227));
    assert!((a - b).abs() < 0.0001);
}

#[test]
fn wcag_levels_map_to_thresholds() {
    assert_eq!(WcagLevel::for_ratio(21.0), WcagLevel::Aaa);
    assert_eq!(WcagLevel::for_ratio(7.0), WcagLevel::Aaa);
    assert_eq!(WcagLevel::for_ratio(4.5), WcagLevel::Aa);
    assert_eq!(WcagLevel::for_ratio(4.49), WcagLevel::Fail);
    assert_eq!(WcagLevel::for_ratio(1.0), WcagLevel::Fail);
}

#[test]
fn wcag_labels() {
    assert_eq!(WcagLevel::Aaa.label(), "AAA");
    assert_eq!(WcagLevel::Aa.label(), "AA");
    assert_eq!(WcagLevel::Fail.label(), "Fail");
}
