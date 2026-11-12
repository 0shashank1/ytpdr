//! Terminal presentation helpers and progress reporting.

use std::time::Duration;

use comfy_table::{Cell, Table};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

use crate::models::{DownloadConfig, PlaylistResult, VideoResult};

const MAX_TITLE_WIDTH: usize = 50;
const MAX_RESULT_TITLE_WIDTH: usize = 60;

/// Print the application banner.
pub fn print_banner() {
    println!();
    println!("{}", style("  ytpdr").bold().blue());
    println!(
        "{}",
        style("  YouTube Playlist Downloader").blue().bright()
    );
    println!(
        "{}",
        style(format!("  v{}", env!("CARGO_PKG_VERSION"))).dim()
    );
    println!();
}

/// Print the effective download configuration.
pub fn print_config(config: &DownloadConfig) {
    println!(
        "{}  {}",
        style("Quality:").dim(),
        config.quality.as_str().to_uppercase()
    );
    println!("{}   {}", style("Format:").dim(), config.merge_format);
    println!("{}   {}", style("Output:").dim(), config.output_dir.display());

    let subtitles = if config.subtitles.enabled {
        format!("enabled ({})", config.subtitles.languages.join(", "))
    } else {
        "disabled".to_string()
    };
    println!("{}     {}", style("Subs:").dim(), subtitles);
    println!();
}

/// Print a warning message.
pub fn print_warning(message: &str) {
    println!("{}", style(format!("Warning: {message}")).yellow());
}

/// Print a per-video heading inside a playlist.
pub fn print_video_heading(index: usize, total: usize, title: &str) {
    println!(
        "{} {}",
        style(format!("({index}/{total})")).bold().cyan(),
        truncate(title, MAX_RESULT_TITLE_WIDTH)
    );
}

/// Print the playlist summary header.
pub fn print_playlist_header(title: &str, count: usize, folder: &std::path::Path) {
    println!();
    println!("{} {}", style("Playlist:").bold().green(), title);
    println!("{} {count}", style("Videos:").dim());
    println!("{} {}", style("Saving to:").dim(), folder.display());
    println!();
}

/// Print a short status line before metadata extraction.
pub fn print_extracting_playlist() {
    println!("{}", style("Extracting playlist metadata...").blue());
}

/// Create a progress bar for an active download.
pub fn create_progress_bar(description: &str) -> ProgressBar {
    let bar = ProgressBar::new(0);
    let template = ProgressStyle::with_template(
        "{spinner:.green} {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({per_sec}, {eta})",
    )
    .expect("valid progress template")
    .progress_chars("=>-");

    bar.set_style(template);
    bar.set_message(truncate(description, MAX_TITLE_WIDTH));
    bar.enable_steady_tick(Duration::from_millis(100));
    bar
}

/// Update a progress bar from parsed `yt-dlp` progress values.
pub fn update_progress(bar: &ProgressBar, downloaded: Option<u64>, total: Option<u64>) {
    if let Some(total) = total.filter(|value| *value > 0) {
        if bar.length() != Some(total) {
            bar.set_length(total);
        }
    }

    if let Some(downloaded) = downloaded {
        if downloaded < bar.position() {
            bar.set_position(0);
        }
        bar.set_position(downloaded);
    }
}

/// Print a summary for a single video download.
pub fn print_video_result(result: &VideoResult) {
    println!();

    if result.success {
        println!(
            "{} {}",
            style("Download complete").bold().green(),
            style(&result.title).bold()
        );
        println!("  {} {}", style("Folder:").dim(), result.folder.display());
        println!("  {} {}", style("File:").dim(), result.filepath.display());

        if !result.subtitle_files.is_empty() {
            let names = result
                .subtitle_files
                .iter()
                .map(|path| path.file_name().unwrap_or_default().to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ");
            println!("  {} {}", style("Subtitles:").dim(), names);
        }

        if let Some(thumbnail) = &result.thumbnail {
            println!(
                "  {} {}",
                style("Thumbnail:").dim(),
                thumbnail.file_name().unwrap_or_default().to_string_lossy()
            );
        }
    } else {
        println!(
            "{} {}",
            style("Download failed").bold().red(),
            style(&result.title).bold()
        );
        println!(
            "  {} {}",
            style("Error:").dim(),
            result.error.as_deref().unwrap_or("unknown error")
        );
    }
}

/// Print a summary table for a playlist download.
pub fn print_playlist_result(result: &PlaylistResult) {
    println!();
    println!(
        "{} {}",
        style("Playlist download summary:").bold().blue(),
        result.title
    );
    println!("{} {}", style("Folder:").dim(), result.folder.display());
    println!();

    let mut table = Table::new();
    table.set_header(vec!["#", "Title", "Status", "Subtitles"]);

    for (index, video) in result.videos.iter().enumerate() {
        let status = if video.success {
            "OK".to_string()
        } else {
            let reason = video.error.as_deref().unwrap_or("unknown error");
            format!("FAIL: {}", truncate(reason, 30))
        };
        table.add_row(vec![
            Cell::new(index + 1),
            Cell::new(truncate(&video.title, MAX_RESULT_TITLE_WIDTH)),
            Cell::new(status),
            Cell::new(video.subtitle_files.len()),
        ]);
    }

    println!("{table}");
    println!(
        "{} succeeded, {} failed ({} total)",
        style(result.succeeded()).green(),
        style(result.failed()).red(),
        result.videos.len()
    );
    println!();
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated: String = value.chars().take(max_chars.saturating_sub(3)).collect();
    truncated.push_str("...");
    truncated
}
