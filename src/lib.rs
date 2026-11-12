//! YTPD-R — a high-quality YouTube video and playlist downloader.
//!
//! The crate is split into a small number of focused modules:
//!
//! * [`cli`] — argument parsing and application entry point.
//! * [`downloader`] — orchestration around the external `yt-dlp` engine.
//! * [`filesystem`] — filename sanitisation and per-item folder creation.
//! * [`models`] — immutable value objects describing configuration and results.
//! * [`ui`] — terminal presentation and progress reporting.
//! * [`error`] — the crate-wide error type.

pub mod cli;
pub mod downloader;
pub mod error;
pub mod filesystem;
pub mod models;
pub mod ui;

pub use error::YtpdError;
pub use models::{
    DownloadConfig, DownloadOutcome, PlaylistResult, Quality, SubtitleConfig, UrlType, VideoResult,
    classify_url,
};
