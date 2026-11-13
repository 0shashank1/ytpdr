//! Integration tests for URL classification and quality presets.

use ytpdr::models::{Quality, UrlType, classify_url};

#[test]
fn classifies_standard_video_url() {
    assert_eq!(
        classify_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
        UrlType::Video
    );
}

#[test]
fn classifies_short_video_url() {
    assert_eq!(classify_url("https://youtu.be/dQw4w9WgXcQ"), UrlType::Video);
}

#[test]
fn classifies_embed_video_url() {
    assert_eq!(
        classify_url("https://www.youtube.com/embed/dQw4w9WgXcQ"),
        UrlType::Video
    );
}

#[test]
fn classifies_shorts_url() {
    assert_eq!(
        classify_url("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
        UrlType::Video
    );
}

#[test]
fn classifies_playlist_url() {
    assert_eq!(
        classify_url("https://www.youtube.com/playlist?list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"),
        UrlType::Playlist
    );
}

#[test]
fn classifies_video_in_playlist_as_playlist() {
    assert_eq!(
        classify_url(
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"
        ),
        UrlType::Playlist
    );
}

#[test]
fn classifies_url_without_protocol() {
    assert_eq!(
        classify_url("youtube.com/watch?v=dQw4w9WgXcQ"),
        UrlType::Video
    );
}

#[test]
fn classifies_quoted_url() {
    assert_eq!(
        classify_url("'https://www.youtube.com/watch?v=dQw4w9WgXcQ'"),
        UrlType::Video
    );
    assert_eq!(
        classify_url(
            "\"https://www.youtube.com/playlist?list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf\""
        ),
        UrlType::Playlist
    );
}

#[test]
fn best_preset_uses_bestvideo() {
    assert!(Quality::Best.format_selector().contains("bestvideo"));
}

#[test]
fn audio_preset_is_audio_only() {
    let selector = Quality::Audio.format_selector();
    assert!(selector.contains("bestaudio"));
    assert!(!selector.contains("bestvideo"));
}

#[test]
fn medium_preset_limits_height() {
    assert!(Quality::Medium.format_selector().contains("720"));
}
