//! Filesystem utilities: filename sanitisation, folder creation, and path
//! resolution.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use unicode_normalization::UnicodeNormalization;

use crate::error::YtpdError;

static ILLEGAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"[<>:"/\\|?*\x00-\x1f]"#).expect("valid illegal-character regex")
});

static TRAILING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\s.]+$").expect("valid trailing-character regex"));

const DEFAULT_MAX_LENGTH: usize = 200;

/// Return a filesystem-safe version of `name`.
///
/// Unicode is normalised (NFC), illegal characters are replaced with
/// underscores, trailing dots and spaces are stripped, and the result is
/// truncated to 200 characters.
pub fn sanitize_filename(name: &str) -> String {
    sanitize_filename_with_limit(name, DEFAULT_MAX_LENGTH)
}

/// Like [`sanitize_filename`], but with a custom maximum length in characters.
pub fn sanitize_filename_with_limit(name: &str, max_length: usize) -> String {
    let normalized: String = name.nfc().collect();
    let replaced = ILLEGAL_RE.replace_all(&normalized, "_");
    let without_trailing = TRAILING_RE.replace_all(&replaced, "");
    let mut sanitized = without_trailing.trim().to_string();

    if sanitized.is_empty() {
        return "untitled".to_string();
    }

    if sanitized.chars().count() > max_length {
        sanitized = sanitized.chars().take(max_length).collect();
        sanitized = sanitized.trim_end().to_string();
    }

    if sanitized.is_empty() {
        "untitled".to_string()
    } else {
        sanitized
    }
}

/// Create `path` and all its parents if necessary, returning the path.
pub fn ensure_directory(path: &Path) -> Result<PathBuf, YtpdError> {
    fs::create_dir_all(path).map_err(|error| {
        YtpdError::Filesystem(format!(
            "cannot create directory {}: {error}",
            path.display()
        ))
    })?;
    Ok(path.to_path_buf())
}

/// Create and return a per-video folder named `<base>/<safe_title> [<id>]`.
pub fn build_video_folder(
    base: &Path,
    video_title: &str,
    video_id: &str,
) -> Result<PathBuf, YtpdError> {
    let safe = sanitize_filename(video_title);
    ensure_directory(&base.join(format!("{safe} [{video_id}]")))
}

/// Create and return a per-playlist folder named `<base>/<safe_title> [<id>]`.
pub fn build_playlist_folder(
    base: &Path,
    playlist_title: &str,
    playlist_id: &str,
) -> Result<PathBuf, YtpdError> {
    let safe = sanitize_filename(playlist_title);
    ensure_directory(&base.join(format!("{safe} [{playlist_id}]")))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

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
    fn creates_video_folder_with_id() {
        let temp = tempdir().expect("tempdir");
        let folder = build_video_folder(temp.path(), "My Cool Video", "abc123").expect("folder");
        assert!(folder.exists());
        assert_eq!(folder.file_name().unwrap(), "My Cool Video [abc123]");
    }

    #[test]
    fn sanitizes_folder_title() {
        let temp = tempdir().expect("tempdir");
        let folder =
            build_video_folder(temp.path(), "Bad: Name? <Yes>", "xyz").expect("folder");
        assert!(folder.exists());
        let name = folder.file_name().unwrap().to_string_lossy();
        assert!(!name.contains('<'));
        assert!(!name.contains('>'));
    }

    #[test]
    fn creates_nested_directories() {
        let temp = tempdir().expect("tempdir");
        let nested = temp.path().join("a").join("b").join("c");
        let result = ensure_directory(&nested).expect("directory");
        assert_eq!(result, nested);
        assert!(nested.is_dir());
    }
}
