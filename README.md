# subscrub

**A fast CLI tool to download, clean, convert, and embed YouTube subtitles.**

Download subtitles directly from YouTube, remove formatting noise and HTML artifacts, export them as **VTT** or **SRT**, and merge them into videos with proper language metadata using **ffmpeg**.

**100% local • No tracking • No ads • No cloud • Script-friendly**

---

## Features

### Download

- Download subtitles from single videos or entire playlists
    
- Manual and auto-generated subtitles shown together in interactive mode — pick across both in one step
    
- List every available language for a video with `subscrub languages`, no download required
    
- Download one or multiple languages at once
    

### Clean

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
    

### Safety

- Never overwrites an existing file — if the target name is already taken, subscrub automatically saves as `name (1)`, `name (2)`, etc.
    
- Applies to downloaded subtitles, merged folders, and single merges alike
    

### Automation

- Interactive terminal interface
    
- Fully scriptable flags mode
    
- Automatic browser detection for cookie authentication
    
- Exponential back-off retry logic for network failures
    

---

## Before / After

YouTube's auto-generated captions build up each line word by word, re-sending the growing sentence over and over until it's replaced by the next one. Watch with captions on (or open the raw subtitle file) and this is what you get:

**Before (raw YouTube captions):**

```text
Hello everyone and welcome back

Hello everyone and welcome back
to the channel

to the channel
how are you doing today
```

**After (`subscrub download`):**

```text
Hello everyone and welcome back
to the channel
how are you doing today
```

Each line appears once, in order, with no repeated text — a normal, readable subtitle file instead of a rolling transcript.

---

## Requirements

subscrub depends on the following tools:

|Dependency|Purpose|Required for|
|---|---|---|
|yt-dlp|Fetches subtitle data from YouTube|Download|
|Deno|JavaScript runtime used internally by yt-dlp to solve YouTube's signature/challenge scripts|Download|
|ffmpeg|Embeds subtitles into video files|Merge|

subscrub itself doesn't call Deno directly — it's a dependency of `yt-dlp`, which needs a real JS runtime to keep working as YouTube's anti-bot challenges evolve. Because of this, subscrub only checks for Deno when you run it in **download** mode; merge mode only requires `ffmpeg`.

### Installing yt-dlp and ffmpeg

|Platform|yt-dlp|ffmpeg|
|---|---|---|
|Debian / Ubuntu|`sudo apt install yt-dlp`|`sudo apt install ffmpeg`|
|Fedora / RHEL|`sudo dnf install yt-dlp`|`sudo dnf install ffmpeg`|
|Arch Linux|`sudo pacman -S yt-dlp`|`sudo pacman -S ffmpeg`|
|openSUSE|`sudo zypper install yt-dlp`|`sudo zypper install ffmpeg`|

**Note:** on Fedora, plain `dnf install ffmpeg` (or the `ffmpeg-free` package) is enough for subscrub — merging only stream-copies your existing video/audio and re-encodes the subtitle track, so the patented codecs behind RPM Fusion's full build aren't needed.

### Getting a newer yt-dlp

Distro-packaged `yt-dlp` can lag behind upstream, and YouTube extraction can start failing as a result. If that happens, first try:

```bash
yt-dlp -U
```

If that doesn't fix it (or your distro's package is too old to begin with), install the latest version yourself with `pipx` instead. Avoid plain `pip install yt-dlp` — most distros now block `pip` from installing outside a virtual environment ([PEP 668](https://peps.python.org/pep-0668/)); `pipx` is the supported way to install Python CLI tools like `yt-dlp` system-wide without hitting that.

|Platform|pipx|
|---|---|
|Debian / Ubuntu|`sudo apt install pipx`|
|Fedora / RHEL|`sudo dnf install pipx`|
|Arch Linux|`sudo pacman -S python-pipx`|
|openSUSE|`sudo zypper install python3-pipx`|

```bash
pipx install yt-dlp
```

Later on, to update it to the newest version:

```bash
pipx upgrade yt-dlp
```

### Installing Deno

```bash
curl -fsSL https://deno.land/install.sh | sh
```

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
subscrub download \
    --url "https://youtube.com/watch?v=..." \
    --type auto \
    --lang ar,en \
    --format srt \
    --output "/path/to/output"
```

### Playlist

```bash
subscrub download \
    --url "https://youtube.com/playlist?list=..." \
    --type auto \
    --lang ar,en \
    --format vtt \
    --output "/path/to/output"
```

### Checking available languages first

Not sure what's available before writing the `--lang` list? List every manual and auto-generated language for a video without downloading anything:

```bash
subscrub languages --url "https://youtube.com/watch?v=..."
```

### Interactive mode

Running `subscrub download` with no flags drops you into a guided session. Manual and auto-generated languages are shown together in a single list, each tagged with its type, so you can pick across both in one step — no separate "subtitle type" prompt to get stuck on if a video only has one kind:

```text
> Select languages  (type to search, Space = toggle, Enter = confirm)
  ar  ·  manual
  ar  ·  auto
  en  ·  auto
```

---

# Merge Mode

### Folder mode

```bash
subscrub merge folder \
    --videos-dir "/path/to/videos" \
    --subs-dir "/path/to/subtitles" \
    --output "/path/to/output"
```

### Single file mode

```bash
subscrub merge single \
    --video "/path/to/video.mkv" \
    --sub "/path/to/sub_ar.srt" \
    --sub "/path/to/sub_en.srt" \
    --output "/path/to/output"
```

---

# Browser Selection

By default, `subscrub download` automatically detects an installed browser (used for cookie authentication) using the following priority:

```
Firefox → Chrome → Brave → Edge → Chromium → Opera → Vivaldi
```

Merge mode doesn't need a browser at all. To override automatic detection for downloads:

```bash
subscrub download --browser brave --url "..." --type auto --lang ar --format srt
```

---

# Command Line Options

### `subscrub download`

|Flag|Short|Description|
|---|---|---|
|`--url`||YouTube video or playlist URL|
|`--type`|`-t`|`manual` or `auto` subtitles|
|`--lang`|`-l`|Comma-separated language codes (`ar,en,fr`)|
|`--format`|`-f`|`vtt` or `srt`|
|`--output`|`-o`|Output folder|
|`--browser`|`-b`|Browser used for cookie authentication|

`--url`, `--type`, `--lang`, and `--format` must be given together to skip the interactive prompts; provide none of them and subscrub will guide you through it.

### `subscrub languages`

|Flag|Short|Description|
|---|---|---|
|`--url`||YouTube video or playlist URL|
|`--browser`|`-b`|Browser used for cookie authentication|

Lists every available manual and auto-generated language for `--url` and exits — no download, no other flags needed.

### `subscrub merge folder`

|Flag|Short|Description|
|---|---|---|
|`--videos-dir`||Videos folder path|
|`--subs-dir`||Subtitles folder path|
|`--output`|`-o`|Output folder|

### `subscrub merge single`

|Flag|Short|Description|
|---|---|---|
|`--video`||Video file path|
|`--sub`||Subtitle file path (repeatable, at least one required)|
|`--output`|`-o`|Output folder — defaults to alongside the source video|

---

# Library Usage

subscrub can also be embedded into Rust applications.

```rust
use subscrub::core::{
    downloader::{detect_browser, list_available_subs, download_with_retry},
    parser::process_json3,
    util::unique_path,
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

See the [LICENSE](LICENSE) file for the full license text.
