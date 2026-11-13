//! End-to-end CLI tests using a fake `yt-dlp` executable.
//!
//! These tests exercise the Rust orchestration (argument construction,
//! extraction parsing, folder layout, and result collection) without touching
//! the network.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

#[cfg(unix)]
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

#[cfg(unix)]
fn install_fake_yt_dlp(directory: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = directory.join("fake-yt-dlp");
    std::fs::write(&path, FAKE_YT_DLP).expect("write fake yt-dlp");
    let mut permissions = std::fs::metadata(&path)
        .expect("fake yt-dlp metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).expect("make fake yt-dlp executable");
    path
}

#[cfg(windows)]
const FAKE_YT_DLP_PS1: &str = r#"
param()
$allArgs = $args
$allStr = $allArgs -join " "

if ($allStr -match "--dump-single-json") {
    if ($allStr -match "--flat-playlist") {
        Write-Output '{"id":"PLTEST","title":"Test Playlist","entries":[{"id":"video001","title":"First Video","url":"https://www.youtube.com/watch?v=video001"},{"id":"video002","title":"Second Video"}]}'
        exit 0
    } else {
        Write-Output '{"id":"video001","title":"First Video"}'
        exit 0
    }
}

$output = ""
for ($i = 0; $i -lt $allArgs.Count; $i++) {
    if ($allArgs[$i] -eq "--output" -and ($i + 1) -lt $allArgs.Count) {
        $output = $allArgs[$i + 1]
    }
}

if (-not $output) {
    [Console]::Error.WriteLine("missing --output argument")
    exit 2
}

$dir = [System.IO.Path]::GetDirectoryName($output)
if (-not (Test-Path $dir)) {
    [System.IO.Directory]::CreateDirectory($dir) | Out-Null
}
[System.IO.File]::WriteAllText((Join-Path $dir "Video.mp4"), "fake video")
exit 0
"#;

#[cfg(windows)]
fn install_fake_yt_dlp(directory: &Path) -> PathBuf {
    let ps1_path = directory.join("fake-yt-dlp.ps1");
    std::fs::write(&ps1_path, FAKE_YT_DLP_PS1).expect("write fake yt-dlp ps1");
    let cmd_path = directory.join("fake-yt-dlp.cmd");
    let cmd_content = "@powershell -NoProfile -ExecutionPolicy Bypass -File \"%~dpn0.ps1\" %*\r\n";
    std::fs::write(&cmd_path, cmd_content).expect("write fake yt-dlp cmd");
    cmd_path
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

    let media = output.join("First Video [video001]").join("Video.mp4");
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
    assert!(
        playlist
            .join("First Video [video001]")
            .join("Video.mp4")
            .is_file()
    );
    assert!(
        playlist
            .join("Second Video [video002]")
            .join("Video.mp4")
            .is_file()
    );
}
