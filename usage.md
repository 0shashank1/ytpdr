
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
