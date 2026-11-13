//! Command-line interface and application entry point.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use url::Url;

use crate::downloader::download;
use crate::error::YtpdError;
use crate::models::{DownloadConfig, DownloadOutcome, Quality, SubtitleConfig};
use crate::ui;

/// Download a YouTube video or playlist in high quality.
///
/// URL can be a video URL, a playlist URL, or a video-in-playlist URL.
#[derive(Debug, Parser)]
#[command(
    name = "ytpdr",
    version,
    about = "Download a YouTube video or playlist in high quality.",
    long_about = None,
    after_help = "Examples:\n  \
        ytpdr 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'\n  \
        ytpdr 'https://www.youtube.com/playlist?list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf'\n  \
        ytpdr -q medium --sub-langs en,es 'https://youtu.be/dQw4w9WgXcQ'"
)]
pub struct Cli {
    /// URL to download.
    pub url: String,

    /// Root output directory.
    #[arg(short, long, value_name = "PATH", default_value = "downloads")]
    pub output: PathBuf,

    /// Video quality preset.
    #[arg(short, long, value_enum, default_value = "best", ignore_case = true)]
    pub quality: Quality,

    /// Skip subtitle download.
    #[arg(long)]
    pub no_subs: bool,

    /// Comma-separated subtitle languages (for example: en,es,fr).
    #[arg(long, value_name = "LANGS", default_value = "en")]
    pub sub_langs: String,

    /// Skip auto-generated subtitles.
    #[arg(long)]
    pub no_auto_subs: bool,

    /// Do not embed subtitles into the media file.
    #[arg(long)]
    pub no_embed_subs: bool,

    /// Do not embed the thumbnail into the media file.
    #[arg(long)]
    pub no_thumbnail: bool,

    /// Do not embed metadata into the media file.
    #[arg(long)]
    pub no_metadata: bool,

    /// Output container format.
    #[arg(
        long = "format",
        value_name = "FORMAT",
        default_value = "mp4",
        value_parser = ["mp4", "mkv", "webm"],
        ignore_case = true
    )]
    pub merge_format: String,

    /// Download rate limit (for example: 5M).
    #[arg(long, value_name = "RATE")]
    pub rate_limit: Option<String>,

    /// Number of retries per fragment.
    #[arg(long, value_name = "N", default_value_t = 5)]
    pub retries: u32,

    /// Path to a Netscape-format cookies file (for age-gated content).
    #[arg(long, value_name = "PATH")]
    pub cookies: Option<PathBuf>,

    /// Download-archive file used to skip already-downloaded videos.
    #[arg(long, value_name = "PATH")]
    pub archive: Option<PathBuf>,

    /// Treat a playlist URL as a single video.
    #[arg(long)]
    pub playlist_as_video: bool,

    /// Enable verbose/debug logging.
    #[arg(short, long)]
    pub verbose: bool,
}

impl Cli {
    /// Convert parsed arguments into a validated [`DownloadConfig`].
    pub fn into_config(self) -> Result<DownloadConfig, YtpdError> {
        let Cli {
            url,
            output,
            quality,
            no_subs,
            sub_langs,
            no_auto_subs,
            no_embed_subs,
            no_thumbnail,
            no_metadata,
            merge_format,
            rate_limit,
            retries,
            cookies,
            archive,
            playlist_as_video,
            ..
        } = self;

        let url = clean_input(&url);
        let effective_url = if playlist_as_video && url.contains("list=") {
            strip_playlist_params(&url)
        } else {
            url
        };

        let cookies = cookies.map(clean_path);
        let archive = archive.map(clean_path);

        if let Some(cookies_path) = cookies.as_ref().filter(|path| !path.exists()) {
            return Err(YtpdError::Filesystem(format!(
                "cookies file not found: {}",
                cookies_path.display()
            )));
        }

        let languages: Vec<String> = sub_langs
            .split(',')
            .map(str::trim)
            .map(|s| s.trim_matches(|c| c == '\'' || c == '"').trim())
            .filter(|language| !language.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        let languages = if languages.is_empty() {
            vec!["en".to_string()]
        } else {
            languages
        };

        Ok(DownloadConfig {
            url: effective_url,
            output_dir: absolute_path(output),
            quality,
            subtitles: SubtitleConfig {
                enabled: !no_subs,
                languages,
                auto_generated: !no_auto_subs,
                ..SubtitleConfig::default()
            },
            embed_subs: !no_embed_subs,
            embed_thumbnail: !no_thumbnail,
            embed_metadata: !no_metadata,
            merge_format: merge_format.to_lowercase(),
            rate_limit,
            retries,
            cookies_file: cookies.map(absolute_path),
            archive_file: archive.map(absolute_path),
        })
    }
}

/// Run the CLI and return the process exit code.
pub fn run(cli: Cli) -> Result<ExitCode, YtpdError> {
    init_logging(cli.verbose);
    ui::print_banner();

    let config = cli.into_config()?;
    ui::print_config(&config);

    match download(&config)? {
        DownloadOutcome::Video(result) => {
            ui::print_video_result(&result);
            Ok(if result.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        DownloadOutcome::Playlist(result) => {
            ui::print_playlist_result(&result);
            Ok(if result.failed() == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
    }
}

fn init_logging(verbose: bool) {
    let default_level = if verbose { "debug" } else { "warn" };
    let environment = env_logger::Env::default().default_filter_or(default_level);
    let mut builder = env_logger::Builder::from_env(environment);
    builder.format_timestamp_secs();
    let _ = builder.try_init();
}

fn clean_input(s: &str) -> String {
    let trimmed = s.trim();
    trimmed
        .trim_matches(|c| c == '\'' || c == '"')
        .trim()
        .to_string()
}

fn clean_path(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    let trimmed = s.trim().trim_matches(|c| c == '\'' || c == '"').trim();
    if trimmed.len() != s.len() {
        PathBuf::from(trimmed)
    } else {
        path
    }
}

fn strip_playlist_params(url: &str) -> String {
    let clean = clean_input(url);
    let candidate = if clean.contains("://") {
        clean.clone()
    } else {
        format!("https://{clean}")
    };

    let Ok(mut parsed) = Url::parse(&candidate) else {
        return clean;
    };

    let retained: Vec<(String, String)> = parsed
        .query_pairs()
        .filter(|(key, _)| key.as_ref() != "list" && key.as_ref() != "index")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();

    parsed.set_query(None);

    if !retained.is_empty() {
        let mut query = parsed.query_pairs_mut();
        for (key, value) in retained {
            query.append_pair(&key, &value);
        }
    }

    parsed.to_string()
}

fn expand_tilde(path: PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str == "~" {
        if let Some(home) = home_dir() {
            return home;
        }
    } else if let Some(sub) = path_str
        .strip_prefix("~/")
        .or_else(|| path_str.strip_prefix("~\\"))
    {
        if let Some(home) = home_dir() {
            return home.join(sub);
        }
    }
    path
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

fn absolute_path(path: PathBuf) -> PathBuf {
    let path = clean_path(path);
    let path = expand_tilde(path);
    if path.is_absolute() {
        return path;
    }

    match std::env::current_dir() {
        Ok(current_dir) => current_dir.join(&path),
        Err(_) => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("valid CLI arguments")
    }

    #[test]
    fn builds_default_config() {
        let config = parse(&["ytpdr", "https://youtu.be/dQw4w9WgXcQ"])
            .into_config()
            .expect("config");

        assert_eq!(config.quality, Quality::Best);
        assert_eq!(config.merge_format, "mp4");
        assert_eq!(config.subtitles.languages, vec!["en"]);
        assert!(config.subtitles.enabled);
        assert!(config.subtitles.auto_generated);
        assert!(config.embed_subs);
        assert!(config.embed_thumbnail);
        assert!(config.embed_metadata);
        assert_eq!(config.retries, 5);
    }

    #[test]
    fn trims_subtitle_languages() {
        let config = parse(&["ytpdr", "--sub-langs", " en, es ,, fr ", "video"])
            .into_config()
            .expect("config");

        assert_eq!(config.subtitles.languages, vec!["en", "es", "fr"]);
    }

    #[test]
    fn strips_playlist_parameters() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL123&index=2";
        assert_eq!(
            strip_playlist_params(url),
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }

    #[test]
    fn playlist_as_video_updates_url() {
        let config = parse(&[
            "ytpdr",
            "--playlist-as-video",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&list=PL123",
        ])
        .into_config()
        .expect("config");

        assert_eq!(config.url, "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    }

    #[test]
    fn rejects_missing_cookies_file() {
        let cli = parse(&[
            "ytpdr",
            "--cookies",
            "/definitely/not/a/real/cookies/file.txt",
            "video",
        ]);

        assert!(cli.into_config().is_err());
    }

    #[test]
    fn accepts_case_insensitive_quality_and_format() {
        let config = parse(&["ytpdr", "-q", "BEST", "--format", "MKV", "video"])
            .into_config()
            .expect("config");

        assert_eq!(config.quality, Quality::Best);
        assert_eq!(config.merge_format, "mkv");
    }

    #[test]
    fn strips_playlist_parameters_without_scheme() {
        let url = "youtube.com/watch?v=dQw4w9WgXcQ&list=PL123&index=2";
        assert_eq!(
            strip_playlist_params(url),
            "https://youtube.com/watch?v=dQw4w9WgXcQ"
        );
    }

    #[test]
    fn rejects_unknown_container_format() {
        assert!(Cli::try_parse_from(["ytpdr", "--format", "avi", "video"]).is_err());
    }

    #[test]
    fn accepts_absolute_output_path() {
        #[cfg(windows)]
        let (raw, expected) = (
            "C:\\tmp\\ytpdr-output",
            PathBuf::from("C:\\tmp\\ytpdr-output"),
        );
        #[cfg(not(windows))]
        let (raw, expected) = ("/tmp/ytpdr-output", PathBuf::from("/tmp/ytpdr-output"));

        let config = parse(&["ytpdr", "--output", raw, "video"])
            .into_config()
            .expect("config");

        assert_eq!(config.output_dir, expected);
    }

    #[test]
    fn expands_tilde_output_path() {
        if let Some(home) = home_dir() {
            let config = parse(&["ytpdr", "--output", "~/Videos", "video"])
                .into_config()
                .expect("config");

            assert_eq!(config.output_dir, home.join("Videos"));
        }
    }

    #[test]
    fn strips_enclosing_single_and_double_quotes_from_url() {
        let config = parse(&["ytpdr", "'https://youtu.be/dQw4w9WgXcQ'"])
            .into_config()
            .expect("config");
        assert_eq!(config.url, "https://youtu.be/dQw4w9WgXcQ");

        let config2 = parse(&["ytpdr", "\"https://youtu.be/dQw4w9WgXcQ\""])
            .into_config()
            .expect("config");
        assert_eq!(config2.url, "https://youtu.be/dQw4w9WgXcQ");
    }
}
