// subscrub - Download & clean YouTube subtitles
// Copyright (C) 2026  AbuKaram01
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use glob::glob;
use regex::Regex;
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::types::{Playlist, PlaylistVideo, SubType};

// ── browser detection ─────────────────────────────────────────────────────────

const BROWSER_PRIORITY: &[(&str, &[&str])] = &[
    ("firefox", &["firefox"]),
    (
        "chrome",
        &["google-chrome", "google-chrome-stable", "chrome"],
    ),
    ("brave", &["brave-browser", "brave"]),
    ("edge", &["microsoft-edge", "msedge"]),
    ("chromium", &["chromium-browser", "chromium"]),
    ("opera", &["opera"]),
    ("vivaldi", &["vivaldi"]),
];

fn is_installed(exe: &str) -> bool {
    #[cfg(unix)]
    {
        Command::new("which")
            .arg(exe)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        Command::new("where")
            .arg(exe)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

pub fn require_yt_dlp() {
    if is_installed("yt-dlp") {
        return;
    }
    eprintln!();
    eprintln!("  \x1b[1;31m✗\x1b[0m  yt-dlp not found");
    eprintln!();
    eprintln!("     subscrub requires yt-dlp to download subtitles.");
    eprintln!("     Install it with one of:");
    eprintln!();
    eprintln!("       Debian/Ubuntu : sudo apt install yt-dlp");
    eprintln!("       Fedora        : sudo dnf install yt-dlp");
    eprintln!("       Arch          : sudo pacman -S yt-dlp");
    eprintln!("       pip           : pip install yt-dlp");
    eprintln!("       GitHub        : https://github.com/yt-dlp/yt-dlp");
    eprintln!();
    std::process::exit(1);
}

pub fn require_ffmpeg() {
    if is_installed("ffmpeg") {
        return;
    }
    eprintln!();
    eprintln!("  \x1b[1;31m✗\x1b[0m  ffmpeg not found");
    eprintln!();
    eprintln!("     subscrub requires ffmpeg to embed subtitles into videos.");
    eprintln!("     Install it with one of:");
    eprintln!();
    eprintln!("       Debian/Ubuntu : sudo apt install ffmpeg");
    eprintln!("       Fedora        : sudo dnf install ffmpeg");
    eprintln!("       Arch          : sudo pacman -S ffmpeg");
    eprintln!("       GitHub        : https://ffmpeg.org/download.html");
    eprintln!();
    std::process::exit(1);
}

pub fn detect_browser() -> Option<String> {
    for (browser_name, executables) in BROWSER_PRIORITY {
        if executables.iter().any(|exe| is_installed(exe)) {
            return Some(browser_name.to_string());
        }
    }
    None
}

// ── paths ─────────────────────────────────────────────────────────────────────

pub fn get_downloads_dir() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|_| PathBuf::from("."))
    })
}

/// Returns the user-chosen output directory (creating it if needed),
/// or falls back to the default Downloads folder.
pub fn resolve_output_dir(custom: &Option<PathBuf>) -> PathBuf {
    match custom {
        Some(p) => {
            let _ = std::fs::create_dir_all(p);
            p.clone()
        }
        None => get_downloads_dir(),
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

pub fn temp_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("{:016x}", d.as_nanos()))
        .unwrap_or_else(|_| "0".to_string())
}

fn clean_filename(raw: &str) -> String {
    let re = Regex::new(r#"[\\/*?:"<>|]"#).unwrap();
    let clean = re.replace_all(raw, "").trim().to_string();
    if clean.is_empty() {
        "untitled".to_string()
    } else {
        clean
    }
}

fn build_yt_dlp(base_args: &[&str], browser: Option<&str>, url: &str) -> Command {
    let mut cmd = Command::new("yt-dlp");
    cmd.args(base_args);
    if let Some(b) = browser {
        cmd.args(["--cookies-from-browser", b]);
    }
    // "--" marks the end of options, so `url` can never be misread as a yt-dlp flag.
    cmd.arg("--");
    cmd.arg(url);
    cmd.stderr(Stdio::null());
    cmd
}

fn opt_browser(browser: &str) -> Option<&str> {
    if browser.is_empty() {
        None
    } else {
        Some(browser)
    }
}

// ── video title ───────────────────────────────────────────────────────────────

fn try_get_title(url: &str, browser: Option<&str>) -> Option<String> {
    let output = build_yt_dlp(
        &[
            "--get-title",
            "--no-check-certificates",
            "--sleep-requests",
            "2",
        ],
        browser,
        url,
    )
    .output()
    .ok()?;

    if !output.status.success() {
        return None;
    }
    let title = clean_filename(String::from_utf8_lossy(&output.stdout).trim());
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

pub fn get_video_title(url: &str, browser: &str) -> String {
    if let Some(t) = try_get_title(url, opt_browser(browser)) {
        return t;
    }

    eprintln!("  [warn] Could not fetch title; using 'subtitles'.");
    "subtitles".to_string()
}

// ── URL validation ────────────────────────────────────────────────────────────

/// Returns true if `url` looks like a YouTube video or playlist URL.
pub fn is_valid_youtube_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && (lower.contains("youtube.com") || lower.contains("youtu.be"))
}

// ── playlist ──────────────────────────────────────────────────────────────────

pub fn is_playlist_url(url: &str) -> bool {
    url.contains("list=") || url.contains("/playlist")
}

fn try_fetch_playlist(
    url: &str,
    browser: Option<&str>,
) -> Result<Playlist, Box<dyn std::error::Error>> {
    let output = build_yt_dlp(
        &["--flat-playlist", "-J", "--no-check-certificates"],
        browser,
        url,
    )
    .output()?;

    if !output.status.success() {
        return Err("yt-dlp failed to fetch playlist info".into());
    }

    let json: Value = serde_json::from_slice(&output.stdout)?;

    let title = clean_filename(
        json.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("playlist"),
    );

    let videos: Vec<PlaylistVideo> = json
        .get("entries")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|entry| {
            let raw_url = entry
                .get("webpage_url")
                .or_else(|| entry.get("url"))
                .and_then(|v| v.as_str())?;

            let video_url = if raw_url.starts_with("http") {
                raw_url.to_string()
            } else {
                format!("https://www.youtube.com/watch?v={raw_url}")
            };

            let video_title = clean_filename(
                entry
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("video"),
            );

            Some(PlaylistVideo {
                url: video_url,
                title: video_title,
            })
        })
        .collect();

    Ok(Playlist { title, videos })
}

pub fn fetch_playlist(url: &str, browser: &str) -> Result<Playlist, Box<dyn std::error::Error>> {
    try_fetch_playlist(url, opt_browser(browser))
}

// ── language listing ──────────────────────────────────────────────────────────

fn query_sub_langs(url: &str, sub_type: &SubType, browser: Option<&str>) -> Vec<String> {
    let output = build_yt_dlp(
        &["-j", "--ignore-errors", "--no-check-certificates"],
        browser,
        url,
    )
    .output();

    let stdout = match output {
        Ok(o) if !o.stdout.is_empty() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let json: Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  [warn] JSON parse error: {e}");
            return Vec::new();
        }
    };

    let field = match sub_type {
        SubType::Manual => "subtitles",
        SubType::Auto => "automatic_captions",
    };

    let map = match json.get(field).and_then(|v| v.as_object()) {
        Some(m) => m,
        None => return Vec::new(),
    };

    let mut langs: Vec<String> = map
        .iter()
        .filter(|(_, formats)| {
            formats.as_array().is_some_and(|arr| {
                arr.iter()
                    .any(|f| f.get("ext").and_then(|e| e.as_str()) == Some("json3"))
            })
        })
        .map(|(k, _)| k.clone())
        .collect();

    langs.sort();
    langs
}

pub fn list_available_subs(url: &str, sub_type: &SubType, browser: &str) -> Vec<String> {
    query_sub_langs(url, sub_type, opt_browser(browser))
}

// ── downloading ───────────────────────────────────────────────────────────────

fn run_download_json3(
    url: &str,
    language: &str,
    sub_type: &SubType,
    temp_prefix: &str,
    browser: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let sub_flag = match sub_type {
        SubType::Manual => "--write-subs",
        SubType::Auto => "--write-auto-subs",
    };
    let output_template = format!("{temp_prefix}.%(ext)s");

    let base: &[&str] = &[
        "--skip-download",
        "--no-warnings",
        sub_flag,
        "--sub-langs",
        language,
        "--sub-format",
        "json3",
        "-o",
        &output_template,
        "--no-check-certificates",
        "--sleep-requests",
        "3",
        "--extractor-retries",
        "5",
        "--retry-sleep",
        "exp=1:30",
    ];

    let status = build_yt_dlp(base, browser, url).status()?;

    if !status.success() {
        return Err("yt-dlp exited with a non-zero status".into());
    }

    let mut found: Vec<String> = Vec::new();
    for path in glob(&format!("{temp_prefix}*.json3"))?.flatten() {
        found.push(path.to_string_lossy().into_owned());
    }

    found
        .into_iter()
        .next()
        .ok_or_else(|| "No json3 file was downloaded.".into())
}

fn download_json3(
    url: &str,
    language: &str,
    sub_type: &SubType,
    temp_prefix: &str,
    browser: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    run_download_json3(url, language, sub_type, temp_prefix, opt_browser(browser))
}

pub fn download_with_retry(
    url: &str,
    language: &str,
    sub_type: &SubType,
    temp_prefix: &str,
    browser: &str,
    max_retries: u32,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut last_err = String::new();

    for attempt in 0..=max_retries {
        if attempt > 0 {
            let secs = 2u64.pow(attempt);
            eprintln!("  [retry] attempt {attempt}/{max_retries} — waiting {secs}s …");
            std::thread::sleep(Duration::from_secs(secs));
        }
        match download_json3(url, language, sub_type, temp_prefix, browser) {
            Ok(path) => return Ok(path),
            Err(e) => last_err = e.to_string(),
        }
    }

    Err(last_err.into())
}
