//! Crate-wide error type.

use thiserror::Error;

/// Any recoverable error produced by the crate.
#[derive(Debug, Error)]
pub enum YtpdError {
    /// The supplied URL could not be interpreted as a YouTube URL.
    #[error("Invalid YouTube URL: {0}")]
    InvalidUrl(String),

    /// A download operation failed.
    #[error("Failed to download '{title}': {reason}")]
    Download { title: String, reason: String },

    /// Metadata extraction failed.
    #[error("Failed to extract info from {url}: {reason}")]
    Extraction { url: String, reason: String },

    /// A filesystem operation failed.
    #[error("Filesystem error: {0}")]
    Filesystem(String),

    /// An I/O operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON document could not be parsed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
