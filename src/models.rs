//! Domain models used throughout the application.

use std::path::PathBuf;
use std::sync::LazyLock;

use clap::ValueEnum;
use regex::Regex;

/// Classification of a YouTube URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrlType {
    /// A single video.
    Video,
    /// A playlist, including a video URL that also carries a `list` parameter.
    Playlist,
}

static PLAYLIST_PATTERNS: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [
        Regex::new(r"(?i)(?:https?://)?(?:www\.)?youtube\.com/playlist\?list=([A-Za-z0-9_-]+)")
            .expect("valid playlist regex"),
        Regex::new(r"(?i)(?:https?://)?(?:www\.)?youtube\.com/.*[?&]list=([A-Za-z0-9_-]+)")
            .expect("valid playlist regex"),
    ]
});

static VIDEO_PATTERNS: LazyLock<[Regex; 4]> = LazyLock::new(|| {
    [
        Regex::new(r"(?i)(?:https?://)?(?:www\.)?youtube\.com/watch\?v=([A-Za-z0-9_-]{11})")
            .expect("valid video regex"),
        Regex::new(r"(?i)(?:https?://)?(?:www\.)?youtu\.be/([A-Za-z0-9_-]{11})")
            .expect("valid video regex"),
        Regex::new(r"(?i)(?:https?://)?(?:www\.)?youtube\.com/embed/([A-Za-z0-9_-]{11})")
            .expect("valid video regex"),
        Regex::new(r"(?i)(?:https?://)?(?:www\.)?youtube\.com/shorts/([A-Za-z0-9_-]{11})")
            .expect("valid video regex"),
    ]
});

/// Determine whether `url` points to a playlist or a single video.
///
/// Playlist URLs take precedence: a `watch?v=...&list=...` URL is treated as a
/// playlist. Unknown URLs fall back to [`UrlType::Video`] so that `yt-dlp` can
/// make the final decision.
pub fn classify_url(url: &str) -> UrlType {
    let clean = url.trim().trim_matches(|c| c == '\'' || c == '"').trim();
    if PLAYLIST_PATTERNS
        .iter()
        .any(|pattern| pattern.is_match(clean))
    {
        return UrlType::Playlist;
    }

    if VIDEO_PATTERNS.iter().any(|pattern| pattern.is_match(clean)) {
        return UrlType::Video;
    }

    UrlType::Video
}

/// Download quality presets mapped to `yt-dlp` format selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Quality {
    /// Best available video and audio.
    Best,
    /// Up to 1440p.
    High,
    /// Up to 720p.
    Medium,
    /// Up to 480p.
    Low,
    /// Audio only.
    Audio,
}

impl Quality {
    /// The `yt-dlp` format selector for this preset.
    pub const fn format_selector(self) -> &'static str {
        match self {
            Self::Best => "bestvideo[ext=mp4]+bestaudio[ext=m4a]/bestvideo+bestaudio/best",
            Self::High => {
                "bestvideo[height<=1440][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=1440]+bestaudio/best"
            }
            Self::Medium => {
                "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=720]+bestaudio/best"
            }
            Self::Low => {
                "bestvideo[height<=480][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=480]+bestaudio/best"
            }
            Self::Audio => "bestaudio[ext=m4a]/bestaudio/best",
        }
    }

    /// Lower-case preset name, matching the CLI value.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Best => "best",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Audio => "audio",
        }
    }
}

/// Controls subtitle downloading behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleConfig {
    /// Whether subtitles should be downloaded at all.
    pub enabled: bool,
    /// Language codes to request.
    pub languages: Vec<String>,
    /// Whether automatically generated subtitles are acceptable.
    pub auto_generated: bool,
    /// Preferred subtitle formats, in priority order.
    pub formats: Vec<String>,
}

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            languages: vec!["en".to_string()],
            auto_generated: true,
            formats: vec!["srt".to_string(), "vtt".to_string()],
        }
    }
}

/// Complete specification for a download job.
#[derive(Debug, Clone)]
pub struct DownloadConfig {
    /// The URL to download.
    pub url: String,
    /// Root output directory.
    pub output_dir: PathBuf,
    /// Quality preset.
    pub quality: Quality,
    /// Subtitle configuration.
    pub subtitles: SubtitleConfig,
    /// Whether to embed downloaded subtitles into the media file.
    pub embed_subs: bool,
    /// Whether to embed the thumbnail into the media file.
    pub embed_thumbnail: bool,
    /// Whether to embed metadata into the media file.
    pub embed_metadata: bool,
    /// Output container format (`mp4`, `mkv`, or `webm`).
    pub merge_format: String,
    /// Optional download rate limit as understood by `yt-dlp` (for example `5M`).
    pub rate_limit: Option<String>,
    /// Number of retries per fragment.
    pub retries: u32,
    /// Optional Netscape-format cookies file.
    pub cookies_file: Option<PathBuf>,
    /// Optional download-archive file.
    pub archive_file: Option<PathBuf>,
}

/// Outcome for a single video download.
#[derive(Debug, Clone)]
pub struct VideoResult {
    /// YouTube video identifier.
    pub video_id: String,
    /// Video title.
    pub title: String,
    /// Folder containing the downloaded files.
    pub folder: PathBuf,
    /// Downloaded media file.
    pub filepath: PathBuf,
    /// Subtitle sidecar files found in the folder.
    pub subtitle_files: Vec<PathBuf>,
    /// Thumbnail file, if one was written.
    pub thumbnail: Option<PathBuf>,
    /// Whether the download completed successfully.
    pub success: bool,
    /// Failure reason, when `success` is `false`.
    pub error: Option<String>,
}

impl VideoResult {
    /// Build a failed result for a video whose media file was not produced.
    pub(crate) fn failure(
        video_id: impl Into<String>,
        title: impl Into<String>,
        folder: PathBuf,
        error: impl Into<String>,
    ) -> Self {
        Self {
            video_id: video_id.into(),
            title: title.into(),
            filepath: folder.join("unknown"),
            folder,
            subtitle_files: Vec::new(),
            thumbnail: None,
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Outcome for a playlist download.
#[derive(Debug, Clone)]
pub struct PlaylistResult {
    /// YouTube playlist identifier.
    pub playlist_id: String,
    /// Playlist title.
    pub title: String,
    /// Folder containing the video sub-folders.
    pub folder: PathBuf,
    /// Per-video outcomes.
    pub videos: Vec<VideoResult>,
}

impl PlaylistResult {
    /// Number of successful video downloads.
    pub fn succeeded(&self) -> usize {
        self.videos.iter().filter(|video| video.success).count()
    }

    /// Number of failed video downloads.
    pub fn failed(&self) -> usize {
        self.videos.iter().filter(|video| !video.success).count()
    }
}

/// Result of a download job: either a single video or a playlist.
#[derive(Debug, Clone)]
pub enum DownloadOutcome {
    /// A single video was requested.
    Video(VideoResult),
    /// A playlist was requested.
    Playlist(PlaylistResult),
}
