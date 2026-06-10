use swatchbook::document::Document;

#[test]
fn new_document_is_untitled_unmodified() {
    let doc = Document::new();
    assert_eq!(doc.title(), "Untitled");
    assert!(!doc.is_modified);
}

#[test]
fn window_title_without_modification() {
    let doc = Document::new();
    assert_eq!(doc.window_title(), "Untitled — Swatchbook");
}

#[test]
fn window_title_with_modification() {
    let mut doc = Document::new();
    doc.is_modified = true;
    assert_eq!(doc.window_title(), "• Untitled — Swatchbook");
}

#[test]
fn save_to_roundtrip() {
    let dir = std::env::temp_dir().join("swatchbook_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.md");

    let mut doc = Document::new();
    doc.content = "# Hello\n\n- `#ff0000`\n".to_string();
    doc.save_to(path.clone()).unwrap();

    let loaded = Document::from_file(&path).unwrap();
    assert_eq!(loaded.content, doc.content);
    assert_eq!(loaded.title(), "test.md");
    assert!(!loaded.is_modified);

    std::fs::remove_file(path).ok();
}

#[test]
fn save_without_path_returns_error() {
    let mut doc = Document::new();
    doc.content = "hello".to_string();
    assert!(doc.save().is_err());
}

#[test]
fn autosave_path_contains_swatchbook() {
    let p = Document::autosave_path();
    assert!(p.to_string_lossy().contains("swatchbook"));
    assert!(p.to_string_lossy().ends_with("autosave.md"));
}
