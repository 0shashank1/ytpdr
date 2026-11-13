//! Core download engine.
//!
//! `yt-dlp` remains the download/extraction engine — reimplementing YouTube's
//! constantly changing formats, signature handling, and post-processing in Rust
//! would be both fragile and unnecessary. This module owns the process
//! orchestration, option translation, progress parsing, and result collection.

use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

use log::debug;
use serde_json::Value;

use crate::error::YtpdError;
use crate::filesystem::{build_playlist_folder, build_video_folder};
use crate::models::{
    DownloadConfig, DownloadOutcome, PlaylistResult, UrlType, VideoResult, classify_url,
};
use crate::ui;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "flv", "m4a", "opus", "mp3",
];
const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt", "ass", "ssa", "sub", "lrc"];
const THUMBNAIL_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

const PROGRESS_PREFIX: &str = "YTPDR_PROGRESS|";
const PROGRESS_TEMPLATE: &str = concat!(
    "YTPDR_PROGRESS|",
    "%(progress.downloaded_bytes)s|",
    "%(progress.total_bytes)s|",
    "%(progress.total_bytes_estimate)s|",
    "%(progress._speed_str)s|",
    "%(progress._eta_str)s|",
    "%(info.title)s"
);

#[derive(Debug, Clone)]
struct MediaInfo {
    id: String,
    title: String,
    url: Option<String>,
}

impl MediaInfo {
    fn watch_url(&self) -> String {
        self.url
            .clone()
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={}", self.id))
    }
}

#[derive(Debug, Clone)]
struct PlaylistMetadata {
    id: String,
    title: String,
    entries: Vec<MediaInfo>,
}

/// Download a video or playlist based on URL classification.
pub fn download(config: &DownloadConfig) -> Result<DownloadOutcome, YtpdError> {
    match classify_url(&config.url) {
        UrlType::Playlist => download_playlist(config).map(DownloadOutcome::Playlist),
        UrlType::Video => Ok(DownloadOutcome::Video(download_video(
            &config.url,
            config,
            None,
            None,
        ))),
    }
}

fn download_video(
    url: &str,
    config: &DownloadConfig,
    base_folder: Option<&Path>,
    info: Option<&MediaInfo>,
) -> VideoResult {
    let base = base_folder.unwrap_or(&config.output_dir);

    let info = match info {
        Some(info) => info.clone(),
        None => match extract_video_metadata(url, config) {
            Ok(info) => info,
            Err(error) => {
                return VideoResult::failure("unknown", url, base.to_path_buf(), error.to_string());
            }
        },
    };

    let folder = match build_video_folder(base, &info.title, &info.id) {
        Ok(folder) => folder,
        Err(error) => {
            return VideoResult::failure(
                &info.id,
                &info.title,
                base.to_path_buf(),
                error.to_string(),
            );
        }
    };

    let template = output_template(&folder);

    let pass1_error = run_download(config, url, &template, &info.title).err();
    let mut video_file = find_video_file(&folder, &config.merge_format);

    if video_file.is_none() && config.subtitles.enabled {
        let reason = pass1_error
            .unwrap_or_else(|| "video file missing after download (likely subtitle error)".into());
        ui::print_warning(&format!("{reason}\nRetrying without subtitles..."));

        let mut no_subtitles = config.clone();
        no_subtitles.subtitles.enabled = false;
        no_subtitles.embed_subs = false;

        let pass2_error = run_download(&no_subtitles, url, &template, &info.title).err();
        video_file = find_video_file(&folder, &config.merge_format);

        if video_file.is_none() {
            return VideoResult::failure(
                &info.id,
                &info.title,
                folder,
                pass2_error
                    .unwrap_or_else(|| "video file not found after retry without subtitles".into()),
            );
        }
    } else if video_file.is_none() {
        return VideoResult::failure(
            &info.id,
            &info.title,
            folder,
            pass1_error.unwrap_or_else(|| "video file not found after download".into()),
        );
    }

    let video_file = video_file.expect("video file existence checked above");

    VideoResult {
        video_id: info.id,
        title: info.title,
        subtitle_files: collect_subtitle_files(&folder),
        thumbnail: find_thumbnail(&folder),
        folder,
        filepath: video_file,
        success: true,
        error: None,
    }
}

fn download_playlist(config: &DownloadConfig) -> Result<PlaylistResult, YtpdError> {
    ui::print_extracting_playlist();

    let metadata = extract_playlist_metadata(&config.url, config)?;

    if metadata.entries.is_empty() {
        return Err(YtpdError::Extraction {
            url: config.url.clone(),
            reason: "playlist is empty or could not be read".to_string(),
        });
    }

    let folder = build_playlist_folder(&config.output_dir, &metadata.title, &metadata.id)?;
    ui::print_playlist_header(&metadata.title, metadata.entries.len(), &folder);

    let mut videos = Vec::with_capacity(metadata.entries.len());

    for (index, entry) in metadata.entries.iter().enumerate() {
        ui::print_video_heading(index + 1, metadata.entries.len(), &entry.title);
        let entry_url = entry.watch_url();
        videos.push(download_video(
            &entry_url,
            config,
            Some(&folder),
            Some(entry),
        ));
    }

    Ok(PlaylistResult {
        playlist_id: metadata.id,
        title: metadata.title,
        folder,
        videos,
    })
}

fn extract_video_metadata(url: &str, config: &DownloadConfig) -> Result<MediaInfo, YtpdError> {
    let value = extract_json(url, config, false)?;
    let id = value.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(id);

    Ok(MediaInfo {
        id: id.to_string(),
        title: title.to_string(),
        url: None,
    })
}

fn extract_playlist_metadata(
    url: &str,
    config: &DownloadConfig,
) -> Result<PlaylistMetadata, YtpdError> {
    let value = extract_json(url, config, true)?;
    let id = value.get("id").and_then(Value::as_str).unwrap_or("unknown");
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(id);
    let entries = value
        .get("entries")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(media_info_from_value).collect())
        .unwrap_or_default();

    Ok(PlaylistMetadata {
        id: id.to_string(),
        title: title.to_string(),
        entries,
    })
}

fn media_info_from_value(value: &Value) -> Option<MediaInfo> {
    let id = value.get("id").and_then(Value::as_str)?;
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| !title.is_empty())
        .unwrap_or(id);
    let url = value
        .get("webpage_url")
        .and_then(Value::as_str)
        .filter(|url| url.starts_with("http"))
        .or_else(|| {
            value
                .get("url")
                .and_then(Value::as_str)
                .filter(|url| url.starts_with("http"))
        })
        .map(ToString::to_string);

    Some(MediaInfo {
        id: id.to_string(),
        title: title.to_string(),
        url,
    })
}

fn extract_json(
    url: &str,
    config: &DownloadConfig,
    flat_playlist: bool,
) -> Result<Value, YtpdError> {
    let mut command = Command::new(yt_dlp_binary());
    if let Some(ffmpeg_dir) = ffmpeg_directory() {
        command.arg("--ffmpeg-location").arg(ffmpeg_dir);
    }
    command
        .arg("--dump-single-json")
        .arg("--no-warnings")
        .arg("--no-progress");

    if flat_playlist {
        command.arg("--flat-playlist");
    }

    if let Some(cookies) = &config.cookies_file {
        command.arg("--cookies").arg(cookies);
    }

    command.arg(url);
    debug!("running metadata extraction: {command:?}");

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let reason = if stderr.is_empty() {
            format!("yt-dlp exited with {}", output.status)
        } else {
            stderr
        };
        return Err(YtpdError::Extraction {
            url: url.to_string(),
            reason,
        });
    }

    let value: Value = serde_json::from_slice(&output.stdout)?;

    if value.is_null() {
        return Err(YtpdError::Extraction {
            url: url.to_string(),
            reason: "yt-dlp returned no metadata".to_string(),
        });
    }

    Ok(value)
}

fn run_download(
    config: &DownloadConfig,
    url: &str,
    output_template: &str,
    title: &str,
) -> Result<(), String> {
    let mut command = build_download_command(config, url, output_template);
    debug!("running download: {command:?}");

    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to start yt-dlp: {error}"))?;

    let stderr_handle = child.stderr.take().map(|mut stream| {
        thread::spawn(move || {
            let mut buffer = String::new();
            let _ = stream.read_to_string(&mut buffer);
            buffer
        })
    });

    let stdout = child
        .stdout
        .take()
        .expect("yt-dlp stdout is piped when spawned");
    let progress = ui::create_progress_bar(title);

    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else {
            break;
        };

        if let Some(payload) = line.strip_prefix(PROGRESS_PREFIX) {
            handle_progress_line(&progress, payload);
        }
    }

    let status = child
        .wait()
        .map_err(|error| format!("failed to wait for yt-dlp: {error}"))?;

    let stderr = stderr_handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default();

    progress.finish_and_clear();

    if !status.success() {
        let reason = stderr.trim();
        return Err(if reason.is_empty() {
            format!("yt-dlp exited with {status}")
        } else {
            reason.to_string()
        });
    }

    Ok(())
}

fn build_download_command(config: &DownloadConfig, url: &str, output_template: &str) -> Command {
    let mut command = Command::new(yt_dlp_binary());

    if let Some(ffmpeg_dir) = ffmpeg_directory() {
        command.arg("--ffmpeg-location").arg(ffmpeg_dir);
    }

    command
        .arg("--format")
        .arg(config.quality.format_selector())
        .arg("--output")
        .arg(output_template)
        .arg("--merge-output-format")
        .arg(&config.merge_format)
        .arg("--retries")
        .arg(config.retries.to_string())
        .arg("--fragment-retries")
        .arg(config.retries.to_string())
        .arg("--ignore-errors")
        .arg("--quiet")
        .arg("--no-warnings")
        .arg("--newline")
        .arg("--progress")
        .arg("--progress-template")
        .arg(PROGRESS_TEMPLATE);

    if config.subtitles.enabled {
        command.arg("--write-subs");

        if config.subtitles.auto_generated {
            command.arg("--write-auto-subs");
        }

        command
            .arg("--sub-langs")
            .arg(config.subtitles.languages.join(","))
            .arg("--sub-format")
            .arg(config.subtitles.formats.join("/"))
            .arg("--sleep-subtitles")
            .arg("1");
    }

    if config.embed_subs && config.subtitles.enabled {
        command.arg("--embed-subs");
    }

    if config.embed_thumbnail {
        command.arg("--write-thumbnail").arg("--embed-thumbnail");
    }

    if config.embed_metadata {
        command.arg("--embed-metadata");
    }

    command.arg("--remux-video").arg(&config.merge_format);

    if let Some(rate_limit) = &config.rate_limit {
        command.arg("--limit-rate").arg(rate_limit);
    }

    if let Some(archive) = &config.archive_file {
        command.arg("--download-archive").arg(archive);
    }

    if let Some(cookies) = &config.cookies_file {
        command.arg("--cookies").arg(cookies);
    }

    command.arg(url);
    command
}

fn handle_progress_line(progress: &indicatif::ProgressBar, payload: &str) {
    let mut fields = payload.splitn(6, '|');
    let downloaded = fields.next().and_then(parse_number);
    let total_bytes = fields.next().and_then(parse_number);
    let total_estimate = fields.next().and_then(parse_number);
    let total = total_bytes.or(total_estimate);

    ui::update_progress(progress, downloaded, total);
}

fn parse_number(value: &str) -> Option<u64> {
    let value = value.trim();

    if value.is_empty()
        || value.eq_ignore_ascii_case("na")
        || value.eq_ignore_ascii_case("none")
        || value.eq_ignore_ascii_case("nan")
    {
        return None;
    }

    let number = value.parse::<f64>().ok()?;

    if number.is_finite() && number >= 0.0 {
        Some(number.round() as u64)
    } else {
        None
    }
}

fn yt_dlp_binary() -> PathBuf {
    if let Ok(override_path) = std::env::var("YTPDR_YT_DLP") {
        return PathBuf::from(override_path);
    }

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let candidate = if cfg!(windows) {
                exe_dir.join("yt-dlp.exe")
            } else {
                exe_dir.join("yt-dlp")
            };
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    PathBuf::from("yt-dlp")
}

fn ffmpeg_directory() -> Option<PathBuf> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            let candidate = if cfg!(windows) {
                exe_dir.join("ffmpeg.exe")
            } else {
                exe_dir.join("ffmpeg")
            };
            if candidate.is_file() {
                return Some(exe_dir.to_path_buf());
            }
        }
    }
    None
}

fn output_template(folder: &Path) -> String {
    let escaped = folder.to_string_lossy().replace('%', "%%");
    format!("{escaped}/%(title)s.%(ext)s")
}

fn collect_subtitle_files(folder: &Path) -> Vec<PathBuf> {
    files_with_extensions(folder, SUBTITLE_EXTENSIONS)
}

fn find_thumbnail(folder: &Path) -> Option<PathBuf> {
    files_with_extensions(folder, THUMBNAIL_EXTENSIONS)
        .into_iter()
        .next()
}

fn find_video_file(folder: &Path, merge_format: &str) -> Option<PathBuf> {
    files_with_extensions(folder, &[merge_format])
        .into_iter()
        .next()
        .or_else(|| {
            files_with_extensions(folder, VIDEO_EXTENSIONS)
                .into_iter()
                .next()
        })
}

fn files_with_extensions(folder: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };

    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_extension(path, extensions))
        .collect();

    files.sort();
    files
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|candidate| extension.eq_ignore_ascii_case(candidate))
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn parses_integer_and_float_numbers() {
        assert_eq!(parse_number("123"), Some(123));
        assert_eq!(parse_number("123.6"), Some(124));
        assert_eq!(parse_number(" 42 "), Some(42));
    }

    #[test]
    fn rejects_missing_or_invalid_numbers() {
        assert_eq!(parse_number("NA"), None);
        assert_eq!(parse_number("none"), None);
        assert_eq!(parse_number("nan"), None);
        assert_eq!(parse_number("-1"), None);
        assert_eq!(parse_number(""), None);
    }

    #[test]
    fn escapes_percent_in_output_folder() {
        assert_eq!(
            output_template(Path::new("/tmp/100% complete")),
            "/tmp/100%% complete/%(title)s.%(ext)s"
        );
    }

    #[test]
    fn builds_media_info_from_flat_entry() {
        let value = json!({
            "id": "abc123",
            "title": "A video",
            "url": "https://www.youtube.com/watch?v=abc123"
        });
        let info = media_info_from_value(&value).expect("media info");

        assert_eq!(info.id, "abc123");
        assert_eq!(info.title, "A video");
        assert_eq!(info.watch_url(), "https://www.youtube.com/watch?v=abc123");
    }

    #[test]
    fn falls_back_to_constructed_watch_url() {
        let value = json!({"id": "abc123", "title": "A video"});
        let info = media_info_from_value(&value).expect("media info");

        assert_eq!(info.watch_url(), "https://www.youtube.com/watch?v=abc123");
    }

    #[test]
    fn ignores_entries_without_ids() {
        assert!(media_info_from_value(&json!({"title": "No id"})).is_none());
        assert!(media_info_from_value(&Value::Null).is_none());
    }

    #[test]
    fn prefers_requested_container_format() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("video.webm"), b"webm").expect("webm");
        fs::write(temp.path().join("video.mp4"), b"mp4").expect("mp4");

        let found = find_video_file(temp.path(), "mp4").expect("video");
        assert_eq!(found.file_name().unwrap(), "video.mp4");
    }

    #[test]
    fn falls_back_to_any_video_extension() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("video.webm"), b"webm").expect("webm");

        let found = find_video_file(temp.path(), "mkv").expect("video");
        assert_eq!(found.file_name().unwrap(), "video.webm");
    }

    #[test]
    fn discovers_subtitles_and_thumbnails() {
        let temp = tempdir().expect("tempdir");
        fs::write(temp.path().join("video.en.vtt"), b"vtt").expect("vtt");
        fs::write(temp.path().join("video.en.srt"), b"srt").expect("srt");
        fs::write(temp.path().join("video.jpg"), b"jpg").expect("jpg");

        let subtitles = collect_subtitle_files(temp.path());
        assert_eq!(subtitles.len(), 2);
        assert_eq!(subtitles[0].file_name().unwrap(), "video.en.srt");
        assert_eq!(
            find_thumbnail(temp.path()).unwrap().file_name().unwrap(),
            "video.jpg"
        );
    }
}
