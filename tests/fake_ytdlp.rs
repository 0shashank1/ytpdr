//! End-to-end CLI tests using a fake `yt-dlp` executable.
//!
//! These tests exercise the Rust orchestration (argument construction,
//! extraction parsing, folder layout, and result collection) without touching
//! the network.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

const FAKE_YT_DLP: &str = r#"#!/bin/sh
set -eu

output=""
previous=""
for argument in "$@"; do
    if [ "$previous" = "--output" ]; then
        output="$argument"
    fi
    previous="$argument"
done

case " $* " in
    *" --dump-single-json "*)
        case " $* " in
            *" --flat-playlist "*)
                cat <<'JSON'
{"id":"PLTEST","title":"Test Playlist","entries":[
{"id":"video001","title":"First Video","url":"https://www.youtube.com/watch?v=video001"},
{"id":"video002","title":"Second Video"}]}
JSON
                ;;
            *)
                printf '%s\n' '{"id":"video001","title":"First Video"}'
                ;;
        esac
        ;;
    *)
        if [ -z "$output" ]; then
            echo "missing --output argument" >&2
            exit 2
        fi
        directory=$(dirname "$output")
        mkdir -p "$directory"
        printf 'fake video' > "$directory/Video.mp4"
        ;;
esac
"#;

fn install_fake_yt_dlp(directory: &Path) -> PathBuf {
    let path = directory.join("fake-yt-dlp");
    std::fs::write(&path, FAKE_YT_DLP).expect("write fake yt-dlp");
    let mut permissions = std::fs::metadata(&path)
        .expect("fake yt-dlp metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("make fake yt-dlp executable");
    path
}

fn ytpdr_command(fake_yt_dlp: &Path, output: &Path, url: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ytpdr"));
    command
        .env("YTPDR_YT_DLP", fake_yt_dlp)
        .arg("--no-subs")
        .arg("--no-thumbnail")
        .arg("--no-metadata")
        .arg("--output")
        .arg(output)
        .arg(url);
    command
}

#[test]
fn downloads_a_single_video_into_its_own_folder() {
    let temp = tempdir().expect("tempdir");
    let fake = install_fake_yt_dlp(temp.path());
    let output = temp.path().join("downloads");

    let result = ytpdr_command(&fake, &output, "https://www.youtube.com/watch?v=video001")
        .output()
        .expect("run ytpdr");

    assert!(
        result.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let media = output
        .join("First Video [video001]")
        .join("Video.mp4");
    assert!(media.is_file(), "expected {}", media.display());
}

#[test]
fn downloads_a_playlist_into_video_subfolders() {
    let temp = tempdir().expect("tempdir");
    let fake = install_fake_yt_dlp(temp.path());
    let output = temp.path().join("downloads");

    let result = ytpdr_command(
        &fake,
        &output,
        "https://www.youtube.com/playlist?list=PLTEST",
    )
    .output()
    .expect("run ytpdr");

    assert!(
        result.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let playlist = output.join("Test Playlist [PLTEST]");
    assert!(playlist.is_dir(), "expected {}", playlist.display());
    assert!(playlist
        .join("First Video [video001]")
        .join("Video.mp4")
        .is_file());
    assert!(playlist
        .join("Second Video [video002]")
        .join("Video.mp4")
        .is_file());
}
