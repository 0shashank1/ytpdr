//! Integration tests for filename sanitisation and folder creation.

use tempfile::tempdir;
use ytpdr::filesystem::{build_video_folder, sanitize_filename, sanitize_filename_with_limit};

#[test]
fn removes_illegal_characters() {
    assert_eq!(
        sanitize_filename("hello<>:\"/\\|?*world"),
        "hello_________world"
    );
}

#[test]
fn strips_trailing_dots_and_spaces() {
    assert_eq!(sanitize_filename("hello...   "), "hello");
}

#[test]
fn truncates_long_names() {
    let result = sanitize_filename_with_limit(&"a".repeat(300), 200);
    assert!(result.chars().count() <= 200);
}

#[test]
fn empty_string_becomes_untitled() {
    assert_eq!(sanitize_filename(""), "untitled");
}

#[test]
fn preserves_unicode() {
    assert_eq!(sanitize_filename("日本語テスト"), "日本語テスト");
}

#[test]
fn creates_folder_with_video_id() {
    let temp = tempdir().expect("tempdir");
    let folder = build_video_folder(temp.path(), "My Cool Video", "abc123").expect("folder");

    assert!(folder.exists());
    assert_eq!(folder.file_name().unwrap(), "My Cool Video [abc123]");
}

#[test]
fn sanitizes_title_when_creating_folder() {
    let temp = tempdir().expect("tempdir");
    let folder = build_video_folder(temp.path(), "Bad: Name? <Yes>", "xyz").expect("folder");

    assert!(folder.exists());
    let name = folder.file_name().unwrap().to_string_lossy();
    assert!(!name.contains('<'));
    assert!(!name.contains('>'));
}
