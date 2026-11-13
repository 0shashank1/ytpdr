# YTPDR — YouTube Playlist Downloader

A high-quality YouTube video and playlist downloader with subtitle support, written
in Rust. Each video gets its own folder. Playlists get a parent folder containing
all video folders.

The application uses [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) as its download
engine and `ffmpeg` for muxing, subtitle embedding, thumbnail embedding, and
metadata. Rust owns the CLI, configuration, validation, filesystem layout,
progress reporting, and result presentation.

## Features

- **High-quality downloads** — best video + audio merged into MP4 by default
- **Subtitle support** — downloads available subtitles (including auto-generated)
  and embeds them
- **Per-video folders** — every video goes into its own clean folder
- **Playlist support** — playlist → parent folder → video sub-folders
- **Embedded metadata** — thumbnails and metadata are embedded into the file
- **Terminal UI** — progress bars, colour output, and playlist summary tables
- **Download archive** — skip already-downloaded videos on re-runs
- **Cookie support** — access age-gated or members-only content
- **Two-pass resilience** — retries without subtitles if a subtitle request
  would otherwise prevent the media file from being saved

## Requirements

- **Rust 1.85+**
- **yt-dlp** available on `PATH`
- **ffmpeg** available on `PATH`

### Linux & macOS

```bash
# Fedora
sudo dnf install ffmpeg
python3 -m pip install --user yt-dlp

# Ubuntu / Debian
sudo apt install ffmpeg
sudo apt install yt-dlp

# macOS
brew install ffmpeg yt-dlp
```

### Windows

```powershell
# Using winget (recommended)
winget install --id Gyan.FFmpeg -e
winget install --id yt-dlp.yt-dlp -e

# Or using Chocolatey
choco install ffmpeg yt-dlp

# Or using Scoop
scoop install ffmpeg yt-dlp
```

> [!NOTE]
> On Windows, restart your terminal after installing `ffmpeg` and `yt-dlp` so that the updated `PATH` is loaded into your shell session.

## Installation

### Automatic (Windows PowerShell)

Run the included PowerShell installer which checks prerequisites, installs `ytpdr`, and configures your `PATH`:

```powershell
cd ytpdr
.\install.ps1
```

### Manual (All Platforms)

```bash
cd ytpdr
cargo install --path .
```

> [!TIP]
> Ensure Cargo's bin directory is in your `PATH`:
> - **Linux / macOS**: `~/.cargo/bin`
> - **Windows**: `%USERPROFILE%\.cargo\bin` (e.g. `C:\Users\<username>\.cargo\bin`)

For development:

```bash
cargo build
cargo test
```

## Usage

### Download a single video

```bash
ytpdr 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'
```

### Download a playlist

```bash
ytpdr 'https://www.youtube.com/playlist?list=PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf'
```

### Custom output directory

```bash
ytpdr -o ~/Videos/YouTube 'https://youtu.be/dQw4w9WgXcQ'
```

### Quality presets

```bash
ytpdr -q best   <url>    # Best available (default)
ytpdr -q high   <url>    # Up to 1440p
ytpdr -q medium <url>    # Up to 720p
ytpdr -q low    <url>    # Up to 480p
ytpdr -q audio  <url>    # Audio only
```

### Subtitle options

```bash
# Download English and Spanish subtitles
ytpdr --sub-langs en,es <url>

# Skip subtitles entirely
ytpdr --no-subs <url>

# Don't include auto-generated subtitles
ytpdr --no-auto-subs <url>

# Keep subtitles as sidecar files instead of embedding them
ytpdr --no-embed-subs <url>
```

### Advanced options

```bash
# Rate limit to 5 MB/s
ytpdr --rate-limit 5M <url>

# Use cookies for age-gated content
ytpdr --cookies cookies.txt <url>

# Skip already-downloaded videos (great for re-running playlists)
ytpdr --archive archive.txt <url>

# Output as MKV instead of MP4
ytpdr --format mkv <url>

# Treat a playlist URL as a single video
ytpdr --playlist-as-video 'https://www.youtube.com/watch?v=xxx&list=yyy'
```

## Output Structure

### Single video

```text
downloads/
└── My Video Title [dQw4w9WgXcQ]/
    ├── My Video Title.mp4
    ├── My Video Title.en.srt
    └── My Video Title.en.vtt
```

### Playlist

```text
downloads/
└── My Playlist Name [PLrAXtmE...]/
    ├── Video 1 Title [id1]/
    │   ├── Video 1 Title.mp4
    │   └── Video 1 Title.en.srt
    ├── Video 2 Title [id2]/
    │   ├── Video 2 Title.mp4
    │   └── Video 2 Title.en.srt
    └── ...
```

## All CLI Options

```text
Usage: ytpdr [OPTIONS] <URL>

Arguments:
  <URL>  URL to download

Options:
  -o, --output <PATH>       Root output directory [default: downloads]
  -q, --quality <QUALITY>   Video quality preset [default: best]
                            [possible values: best, high, medium, low, audio]
      --no-subs             Skip subtitle download
      --sub-langs <LANGS>   Comma-separated subtitle languages [default: en]
      --no-auto-subs        Skip auto-generated subtitles
      --no-embed-subs       Do not embed subtitles into the media file
      --no-thumbnail        Do not embed the thumbnail into the media file
      --no-metadata         Do not embed metadata into the media file
      --format <FORMAT>     Output container format [default: mp4]
                            [possible values: mp4, mkv, webm]
      --rate-limit <RATE>   Download rate limit (for example: 5M)
      --retries <N>         Number of retries per fragment [default: 5]
      --cookies <PATH>      Path to a Netscape-format cookies file
      --archive <PATH>      Download-archive file
      --playlist-as-video   Treat a playlist URL as a single video
  -v, --verbose             Enable verbose/debug logging
  -h, --help                Print help
  -V, --version             Print version
```

## Architecture

| Module | Responsibility |
| --- | --- |
| `cli` | Argument parsing, validation, and application entry point |
| `models` | Immutable domain types and URL classification |
| `downloader` | `yt-dlp` process orchestration, progress parsing, file discovery |
| `filesystem` | Filename sanitisation and per-item folder creation |
| `ui` | Terminal presentation and progress bars |
| `error` | Crate-wide error type |

The binary is intentionally thin: `main` parses arguments, delegates to
`cli::run`, and maps errors to process exit codes. Most logic lives in the
library so it can be tested directly.

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

## Environment

`YTPDR_YT_DLP` can override the `yt-dlp` executable name or path.

## License

MIT


#Download from installer in windows

https://drive.google.com/file/d/1Ztnu6gZoRmamvH_iitpa8wf38eXcqkQU/view?usp=drive_link

#usage docs
https://drive.google.com/drive/folders/1Mw6pv1GS_B23o6srqV1LcT29Ddts7dyZ
