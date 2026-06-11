# subscrub

**Download, clean, and embed YouTube subtitles — from the terminal.**

`subscrub` fetches subtitles directly from YouTube in `json3` format, cleans them
(removes noise tags, music symbols, and HTML artifacts), and saves them as **VTT**
or **SRT** files. It also embeds cleaned subtitles into video files via ffmpeg,
with full language metadata so every media player displays them correctly.

No tracking. No ads. No cloud. Runs entirely on your machine.

> ⚠️ **Important Notice regarding YouTube Subtitles:**
> You must install the Dino software on your system for the tool to function correctly. Use the following command:
> ```bash
> curl -fsSL https://deno.land/install.sh | sh
> ```

---

## Features

- **Interactive mode** — guided prompts, no flags needed
- **Flags mode** — fully scriptable, zero prompts
- **Playlist support** — downloads subtitles for every video in a playlist
- **Multiple languages** — select one or more at once
- **VTT & SRT** — clean output in either format
- **Subtitle merging** — embed subtitles into videos via ffmpeg (MKV output)
  - Folder mode: match and merge entire directories
  - Single mode: merge one video with one or more subtitle files
- **Incremental merging** — add more languages to an already-merged file without breaking existing tracks
- **Smart matching** — matches videos to subtitles by YouTube ID, title, or alphabetical position
- **Auto browser detection** — finds your installed browser for cookie auth automatically
- **Browser priority** — Firefox → Chrome → Brave → Edge → Chromium → Opera → Vivaldi
- **RTL support** — Arabic, Hebrew, Persian, Urdu get correct direction styling in VTT
- **Retry logic** — exponential back-off on network failures

---

## Requirements

Tool	For installation
yt-dlp	

https://github.com/yt-dlp/yt-dlp

Or install from your package repository


ffmpeg	https://ffmpeg.org

Or install from your package repository

deno	curl -fsSL https://deno.land/install.sh | sh

Supported browsers: `firefox` · `chrome` · `brave` · `edge` · `chromium` · `opera` · `vivaldi`

---

## Installation

### From source

```bash
git clone https://github.com/AbuKaram01/subscrub
cd subscrub
cargo build --release
sudo cp target/release/subscrub /usr/local/bin/
```

### Debian / Ubuntu — `.deb`

```bash
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/subget_*.deb
```

### Fedora / RHEL / openSUSE — `.rpm`

```bash
cargo install cargo-generate-rpm
cargo build --release
cargo generate-rpm
sudo rpm -i target/generate-rpm/subscrub-*.rpm
```

---

## Usage

### Interactive mode

Run with no arguments — `subscrub` guides you through everything:

```
$ subscrub

  ▶ subscrub
     YouTube subtitle downloader & cleaner
  ────────────────────────────────────────────

  What do you want to do?
  ❯ Download subtitles
    Merge subtitles into videos
```

### Download — flags

```bash
# Single video
subscrub --url "https://youtube.com/watch?v=..." \
       --type auto \
       --lang ar,en \
       --format srt

# Playlist
subscrub --url "https://youtube.com/playlist?list=..." \
       --type auto \
       --lang ar,en \
       --format vtt
```

### Merge — flags

```bash
# Folder mode
subscrub --merge \
       --videos-dir "/path/to/videos" \
       --subs-dir "/path/to/subtitles"

# Single file mode
subscrub --merge \
       --video "/path/to/video.mkv" \
       --sub "/path/to/sub_ar.srt" \
       --sub "/path/to/sub_en.srt"
```

### Override browser

```bash
subscrub --browser brave ...
```

---

## Output

Mode	Output
Single video	`~/Downloads/{Title} - {lang}.vtt`
Playlist	`~/Downloads/{Playlist} subs/{Title} - {lang}.vtt`
Merge folder	`{videos folder} merged/{Title}.mkv`
Merge single	`{video location}/{Title}.mkv` (or `_merged.mkv` if input is already MKV)

---

## All options

Flag	Short	Description	Required in flags mode
`--url`		YouTube video or playlist URL	Download only
`--type`	`-t`	`manual` or `auto`	Download only
`--lang`	`-l`	Language codes, comma-separated (e.g. `ar,en,fr`)	Download only
`--format`	`-f`	`vtt` or `srt`	Download only
`--browser`	`-b`	Browser for cookie auth	Never (auto-detected)
`--merge`		Switch to merge mode	—
`--videos-dir`		Videos folder *(merge folder mode)*	Merge folder
`--subs-dir`		Subtitles folder *(merge folder mode)*	Merge folder
`--video`		Single video file *(merge single mode)*	Merge single
`--sub`		Subtitle file, repeatable *(merge single mode)*	Merge single

---

## Library usage

`subscrub` is also usable as a Rust library:

```rust
use subscrub::core::{
    downloader::{detect_browser, list_available_subs, download_with_retry},
    parser::process_json3,
    writer::write_vtt,
    merger::{match_videos_to_subs, merge_video, merge_single},
};
```

---

## Contributing

Contributions are welcome. Please:

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Commit your changes: `git commit -m 'Add your feature'`
4. Push and open a pull request

The codebase is split into `core/` (pure logic, no UI) and `cli/` (terminal interface),
so it should be straightforward to find where changes belong.

---

## License

Copyright (C) 2026 AbuKaram01

This program is free software: you can redistribute it and/or modify it under
the terms of the **GNU General Public License v3.0** or later.

See [LICENSE](LICENSE) for the full text.
