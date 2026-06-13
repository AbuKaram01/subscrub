# subscrub

**A fast CLI tool to download, clean, convert, and embed YouTube subtitles.**

Download subtitles directly from YouTube, remove formatting noise and HTML artifacts, export them as **VTT** or **SRT**, and merge them into videos with proper language metadata using **ffmpeg**.

**100% local • No tracking • No ads • No cloud • Script-friendly**

---

## Features

### Download

- Download subtitles from single videos or entire playlists
    
- Support for manual and auto-generated subtitles
    
- Download one or multiple languages at once
    

### Clean

- Remove music symbols and decorative characters
    
- Strip HTML artifacts and formatting noise
    
- Produce clean, readable subtitle files
    

### Export

- Export to **VTT** or **SRT**
    
- RTL-aware VTT styling for Arabic, Hebrew, Persian, and Urdu
    

### Merge

- Embed subtitles into videos using **ffmpeg**
    
- Folder mode: merge complete directories
    
- Single mode: merge one video with multiple subtitle files
    
- Incremental merging without replacing existing subtitle tracks
    
- Smart matching by YouTube ID, title, or alphabetical order
    

### Automation

- Interactive terminal interface
    
- Fully scriptable flags mode
    
- Automatic browser detection for cookie authentication
    
- Exponential back-off retry logic for network failures
    

---

## Requirements

subscrub depends on the following tools:

|Dependency|Purpose|
|---|---|
|yt-dlp|Access YouTube subtitle data|
|ffmpeg|Merge subtitles into videos|
|Deno|Runtime required by subscrub|

Install Deno:

```bash
curl -fsSL https://deno.land/install.sh | sh
```

Install `yt-dlp` and `ffmpeg` using your distribution's package manager.

---

## Installation

### Build from source

```bash
git clone https://github.com/AbuKaram01/subscrub
cd subscrub
cargo build --release
sudo cp target/release/subscrub /usr/local/bin/
```

### Debian / Ubuntu (.deb)

```bash
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/subscrub_*.deb
```

### Fedora / RHEL / openSUSE (.rpm)

```bash
cargo install cargo-generate-rpm
cargo build --release
cargo generate-rpm
sudo rpm -i target/generate-rpm/subscrub-*.rpm
```

---

# Quick Start

Run without any arguments to launch interactive mode:

```bash
subscrub
```

Example:

```text
▶ subscrub
   YouTube subtitle downloader & cleaner
────────────────────────────────────────────

What do you want to do?

❯ Download subtitles
  Merge subtitles into videos
```

---

# Download Mode

### Single video

```bash
subscrub \
    --url "https://youtube.com/watch?v=..." \
    --type auto \
    --lang ar,en \
    --format srt \
    --output "/path/to/output"
```

### Playlist

```bash
subscrub \
    --url "https://youtube.com/playlist?list=..." \
    --type auto \
    --lang ar,en \
    --format vtt \
    --output "/path/to/output"
```

---

# Merge Mode

### Folder mode

```bash
subscrub \
    --merge \
    --videos-dir "/path/to/videos" \
    --subs-dir "/path/to/subtitles"
    --output "/path/to/output"
```

### Single file mode

```bash
subscrub \
    --merge \
    --video "/path/to/video.mkv" \
    --sub "/path/to/sub_ar.srt" \
    --sub "/path/to/sub_en.srt"
    --output "/path/to/output"
```

---

# Browser Selection

By default, subscrub automatically detects an installed browser using the following priority:

```
Firefox → Chrome → Brave → Edge → Chromium → Opera → Vivaldi
```

To override automatic detection:

```bash
subscrub --browser brave
```

---

# Command Line Options

|Flag|Short|Description|
|---|---|---|
|`--url`||YouTube video or playlist URL|
|`--type`|`-t`|`manual` or `auto` subtitles|
|`--lang`|`-l`|Comma-separated language codes (`ar,en,fr`)|
|`--format`|`-f`|`vtt` or `srt`|
|`--browser`|`-b`|Browser used for cookie authentication|
|`--output`|`-o`|Output directory|
|`--merge`||Enable merge mode|
|`--videos-dir`||Videos directory (folder mode)|
|`--subs-dir`||Subtitle directory (folder mode)|
|`--video`||Single video file|
|`--sub`||Subtitle file (repeatable)|

---

# Library Usage

subscrub can also be embedded into Rust applications.

```rust
use subscrub::core::{
    downloader::{detect_browser, list_available_subs, download_with_retry},
    parser::process_json3,
    writer::write_vtt,
    merger::{match_videos_to_subs, merge_video, merge_single},
};
```

---

# Why subscrub?

- Runs entirely on your machine
    
- No tracking or telemetry
    
- No cloud services
    
- No ads
    
- Supports scripting and automation
    
- Built with Rust for speed and reliability
    

---

# Contributing

Contributions are welcome.

1. Fork the repository
    
2. Create a feature branch
    

```bash
git checkout -b feature/your-feature
```

3. Commit your changes
    

```bash
git commit -m "Add your feature"
```

4. Push your branch and open a Pull Request
    

---

# License

Copyright (C) 2026 AbuKaram01

This program is free software: you can redistribute it and/or modify it under the terms of the **GNU General Public License v3.0** or any later version.

See the **LICENSE** file for the full license text.
