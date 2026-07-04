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

use std::fs;
use std::path::PathBuf;

use console::style;

use subscrub::core::{
    downloader::{
        detect_browser, download_with_retry, fetch_playlist, get_video_title, is_playlist_url,
        is_valid_youtube_url, list_available_subs, require_deno, require_yt_dlp,
        resolve_output_dir, temp_id,
    },
    parser::process_json3,
    types::{SubFormat, SubType},
    util::unique_path,
    writer::{write_srt, write_vtt},
};

use crate::cli::ui::{
    ask_format, ask_language_choices, ask_output_dir, ask_url, make_spinner, print_summary,
    show_browser,
};
use crate::cli::{
    parse_format, parse_languages, parse_sub_type, validate_download_args, DownloadArgs,
    DownloadMode, LanguagesArgs,
};
use crate::commands::fail;

// ── entry point ───────────────────────────────────────────────────────────────

pub fn run(args: DownloadArgs) {
    require_yt_dlp().unwrap_or_else(|e| fail(e));
    require_deno().unwrap_or_else(|e| fail(e));

    let mode = validate_download_args(&args).unwrap_or_else(|missing| {
        fail(format!(
            "download requires: {}",
            style(missing.join("  ·  ")).cyan()
        ))
    });

    let browser = resolve_browser(args.browser.as_ref());
    show_browser(&browser);

    let custom_output: Option<PathBuf> = args.output.as_deref().map(PathBuf::from);

    let output_dir = match mode {
        DownloadMode::Interactive if custom_output.is_none() => {
            println!();
            ask_output_dir("Downloads folder")
        }
        _ => custom_output.clone(),
    };

    let url = match mode {
        DownloadMode::Interactive => {
            println!();
            ask_url()
        }
        DownloadMode::Flags => args.url.as_deref().unwrap().trim().to_string(),
    };

    if url.is_empty() {
        fail("No URL provided.");
    }

    if !is_valid_youtube_url(&url) {
        fail(format!("Not a valid YouTube URL: {}", style(&url).dim()));
    }

    if is_playlist_url(&url) {
        run_playlist(
            &url,
            &browser,
            mode,
            args.sub_type.as_deref(),
            args.lang.as_deref(),
            args.format.as_deref(),
            &output_dir,
        );
    } else {
        run_single(
            &url,
            &browser,
            mode,
            args.sub_type.as_deref(),
            args.lang.as_deref(),
            args.format.as_deref(),
            &output_dir,
        );
    }
}

// ── list available languages ──────────────────────────────────────────────────

/// Entry point for `subscrub languages`: shows every subtitle language
/// available for `--url`, for both manual and auto-generated captions, then
/// exits without downloading anything. Meant for users who want to know
/// what's available before picking `download --lang` values.
pub fn run_list_languages(args: LanguagesArgs) {
    require_yt_dlp().unwrap_or_else(|e| fail(e));
    require_deno().unwrap_or_else(|e| fail(e));

    let url = args.url.trim().to_string();

    if url.is_empty() {
        fail("No URL provided.");
    }
    if !is_valid_youtube_url(&url) {
        fail(format!("Not a valid YouTube URL: {}", style(&url).dim()));
    }

    let browser = resolve_browser(args.browser.as_ref());
    show_browser(&browser);

    println!();
    let pb = make_spinner("Fetching available languages…".to_string());
    let manual = list_available_subs(&url, &SubType::Manual, &browser);
    let auto = list_available_subs(&url, &SubType::Auto, &browser);
    pb.finish_and_clear();

    print_language_group("Manual (community)", &manual);
    print_language_group("Auto-generated", &auto);
    println!();
}

fn print_language_group(label: &str, languages: &[String]) {
    println!();
    if languages.is_empty() {
        println!(
            "  {}  {}  {}",
            style("✗").red().bold(),
            label,
            style("none available").dim()
        );
        return;
    }
    println!(
        "  {}  {}  {} language{}",
        style("✓").green().bold(),
        label,
        style(languages.len()).cyan().bold(),
        if languages.len() == 1 { "" } else { "s" }
    );
    print_wrapped_list(languages);
}

/// Prints `items` wrapped across as many lines as the terminal needs,
/// instead of one giant joined line — a video can easily have 100+
/// auto-translated languages, and dumping them all on one row is
/// unreadable and wraps badly.
fn print_wrapped_list(items: &[String]) {
    const INDENT: &str = "     ";

    let cols = console::Term::stdout().size().1 as usize;
    let available = cols.saturating_sub(INDENT.len()).max(20);

    for line in wrap_items(items, available) {
        println!("{INDENT}{}", style(line).cyan());
    }
}

/// Packs `items` into lines of at most `width` characters (joined by
/// `  ·  `), fitting as many per line as will cleanly fit. Pure and
/// side-effect free so it's easy to unit test independent of a real
/// terminal.
fn wrap_items(items: &[String], width: usize) -> Vec<String> {
    const SEP: &str = "  ·  ";

    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for item in items {
        let extra_len = if current.is_empty() {
            item.len()
        } else {
            SEP.len() + item.len()
        };

        if !current.is_empty() && current.len() + extra_len > width {
            lines.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push_str(SEP);
        }
        current.push_str(item);
    }
    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_items_respects_width() {
        let items: Vec<String> = (0..157).map(|i| format!("lang-{i}")).collect();
        let lines = wrap_items(&items, 40);

        assert!(
            lines.len() > 1,
            "157 items at width 40 should need several lines"
        );
        for line in &lines {
            assert!(line.len() <= 40, "line exceeded width: {line:?}");
        }

        // every original item must still be present somewhere in the output
        let rejoined = lines.join("  ·  ");
        for item in &items {
            assert!(rejoined.contains(item.as_str()));
        }
    }

    #[test]
    fn wrap_items_single_line_when_it_fits() {
        let items = vec!["ar".to_string(), "en".to_string(), "fr".to_string()];
        let lines = wrap_items(&items, 80);
        assert_eq!(lines, vec!["ar  ·  en  ·  fr".to_string()]);
    }

    #[test]
    fn wrap_items_handles_empty_input() {
        let lines: Vec<String> = wrap_items(&[], 80);
        assert!(lines.is_empty());
    }
}

// ── browser resolution ────────────────────────────────────────────────────────

fn resolve_browser(cli_browser: Option<&String>) -> String {
    if let Some(b) = cli_browser {
        return b.clone();
    }
    match detect_browser() {
        Some(b) => b,
        None => fail("No supported browser found. Use --browser to specify one."),
    }
}

// ── language selection ────────────────────────────────────────────────────────

/// Fetches the subtitle languages available for `probe_url` and resolves
/// which `(language, subtitle-type)` pairs to download.
///
/// - Flags mode uses the single type given by `--type`, matched against
///   `--lang`.
/// - Interactive mode fetches manual *and* auto-generated languages and
///   lets the user multi-select across both in one step — so a video with
///   no manual captions still shows its auto-generated ones instead of
///   the session ending.
///
/// `note`, if given, is appended after the "available" line (used by the
/// playlist path to clarify the count is based on the first video).
/// Returns `None` when nothing at all is available for `probe_url`.
fn select_languages(
    probe_url: &str,
    browser: &str,
    mode: DownloadMode,
    sub_type_flag: Option<&str>,
    cli_lang: Option<&str>,
    note: Option<&str>,
) -> Option<Vec<(String, SubType)>> {
    let pb = make_spinner("Fetching available subtitles…".to_string());

    match mode {
        DownloadMode::Flags => {
            let sub_type = parse_sub_type(sub_type_flag.unwrap());
            let languages = list_available_subs(probe_url, &sub_type, browser);
            pb.finish_and_clear();

            if languages.is_empty() {
                return None;
            }

            print!(
                "  {}  {} language{} available",
                style("✓").green().bold(),
                style(languages.len()).cyan().bold(),
                if languages.len() == 1 { "" } else { "s" }
            );
            if let Some(n) = note {
                print!("  {}", style(n).dim());
            }
            println!();
            println!();

            let indices =
                parse_languages(cli_lang.unwrap(), &languages).unwrap_or_else(|e| fail(e));
            Some(
                indices
                    .into_iter()
                    .map(|i| (languages[i].clone(), sub_type.clone()))
                    .collect(),
            )
        }
        DownloadMode::Interactive => {
            let manual = list_available_subs(probe_url, &SubType::Manual, browser);
            let auto = list_available_subs(probe_url, &SubType::Auto, browser);
            pb.finish_and_clear();

            let total = manual.len() + auto.len();
            if total == 0 {
                return None;
            }

            print!(
                "  {}  {} language{} available",
                style("✓").green().bold(),
                style(total).cyan().bold(),
                if total == 1 { "" } else { "s" }
            );
            if let Some(n) = note {
                print!("  {}", style(n).dim());
            }
            println!(
                "  {}",
                style(format!("({} manual  ·  {} auto)", manual.len(), auto.len())).dim()
            );
            println!();

            Some(ask_language_choices(&manual, &auto))
        }
    }
}

// ── single video download ─────────────────────────────────────────────────────

fn run_single(
    url: &str,
    browser: &str,
    mode: DownloadMode,
    sub_type_flag: Option<&str>,
    cli_lang: Option<&str>,
    cli_fmt: Option<&str>,
    output_dir: &Option<PathBuf>,
) {
    println!();
    let chosen = match select_languages(url, browser, mode, sub_type_flag, cli_lang, None) {
        Some(c) => c,
        None => {
            eprintln!(
                "  {}  No subtitles found for this video.",
                style("✗").red().bold()
            );
            return;
        }
    };

    println!();
    let format = match mode {
        DownloadMode::Interactive => ask_format(),
        DownloadMode::Flags => parse_format(cli_fmt.unwrap()),
    };

    println!();
    let pb = make_spinner("Fetching video title…".to_string());
    let title = get_video_title(url, browser);
    pb.finish_and_clear();
    println!("  {}  {}", style("▶").dim(), style(&title).bold());

    println!();
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let downloads = resolve_output_dir(output_dir);
    let total = chosen.len();
    let mut saved = 0usize;

    for (n, (lang, sub_type)) in chosen.iter().enumerate() {
        let temp_prefix = format!("temp_subs_{}_{}", lang, temp_id());
        let pb = make_spinner(format!(
            "[{}/{}]  {}  Downloading…",
            n + 1,
            total,
            style(lang).cyan().bold()
        ));
        let mut temp_files: Vec<String> = Vec::new();

        let result: Result<String, Box<dyn std::error::Error>> = (|| {
            let json3_path = download_with_retry(url, lang, sub_type, &temp_prefix, browser, 3)?;
            temp_files.push(json3_path.clone());
            pb.set_message(format!(
                "[{}/{}]  {}  Processing…",
                n + 1,
                total,
                style(lang).cyan().bold()
            ));
            let cues = process_json3(&json3_path)?;
            let filename = match format {
                SubFormat::Vtt => format!("{title} - {lang}.vtt"),
                SubFormat::Srt => format!("{title} - {lang}.srt"),
            };
            let out_path = unique_path(&downloads.join(&filename));
            fs::write(
                &out_path,
                match format {
                    SubFormat::Vtt => write_vtt(&cues, lang),
                    SubFormat::Srt => write_srt(&cues),
                }
                .as_bytes(),
            )?;
            Ok(out_path.to_string_lossy().into_owned())
        })();

        for p in &temp_files {
            let _ = fs::remove_file(p);
        }
        pb.finish_and_clear();
        match &result {
            Ok(p) => {
                saved += 1;
                println!(
                    "  {}  {}  {}",
                    style("✓").green().bold(),
                    style(lang).cyan().bold(),
                    style(p).dim()
                );
            }
            Err(e) => eprintln!(
                "  {}  {}  {}",
                style("✗").red().bold(),
                style(lang).cyan().bold(),
                style(e.to_string()).red()
            ),
        }
    }

    println!();
    print_summary(saved, total);
}

// ── playlist download ─────────────────────────────────────────────────────────

fn run_playlist(
    url: &str,
    browser: &str,
    mode: DownloadMode,
    sub_type_flag: Option<&str>,
    cli_lang: Option<&str>,
    cli_fmt: Option<&str>,
    output_dir: &Option<PathBuf>,
) {
    println!();
    let pb = make_spinner("Fetching playlist info…".to_string());
    let playlist = match fetch_playlist(url, browser) {
        Ok(p) => {
            pb.finish_and_clear();
            p
        }
        Err(e) => {
            pb.finish_and_clear();
            eprintln!("  {}  {}", style("✗").red().bold(), e);
            return;
        }
    };

    println!(
        "  {}  {}  {}",
        style("▶").dim(),
        style(&playlist.title).bold(),
        style(format!("({} videos)", playlist.videos.len())).dim()
    );

    if playlist.videos.is_empty() {
        eprintln!("  {}  Playlist is empty.", style("✗").red().bold());
        return;
    }

    println!();
    let chosen = match select_languages(
        &playlist.videos[0].url,
        browser,
        mode,
        sub_type_flag,
        cli_lang,
        Some("(based on first video)"),
    ) {
        Some(c) => c,
        None => {
            eprintln!(
                "  {}  No subtitles found on the first video.",
                style("✗").red().bold()
            );
            return;
        }
    };

    println!();
    let format = match mode {
        DownloadMode::Interactive => ask_format(),
        DownloadMode::Flags => parse_format(cli_fmt.unwrap()),
    };

    let folder_path = resolve_output_dir(output_dir).join(format!("{} subs", playlist.title));
    if let Err(e) = fs::create_dir_all(&folder_path) {
        eprintln!("  {}  {}", style("✗").red().bold(), e);
        return;
    }

    println!();
    println!(
        "  {}  {}",
        style("▶").dim(),
        style(folder_path.display()).bold()
    );
    println!();
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let total = playlist.videos.len() * chosen.len();
    let mut saved = 0usize;
    let mut n = 0usize;

    for video in &playlist.videos {
        for (lang, sub_type) in &chosen {
            n += 1;
            let temp_prefix = format!("temp_subs_{}_{}", lang, temp_id());
            let pb = make_spinner(format!(
                "[{n}/{total}]  {}  {}",
                style(lang).cyan().bold(),
                style(&video.title).dim()
            ));
            let mut temp_files: Vec<String> = Vec::new();

            let result: Result<String, Box<dyn std::error::Error>> = (|| {
                let json3_path =
                    download_with_retry(&video.url, lang, sub_type, &temp_prefix, browser, 3)?;
                temp_files.push(json3_path.clone());
                pb.set_message(format!(
                    "[{n}/{total}]  {}  Processing…",
                    style(lang).cyan().bold()
                ));
                let cues = process_json3(&json3_path)?;
                let filename = match format {
                    SubFormat::Vtt => format!("{} - {}.vtt", video.title, lang),
                    SubFormat::Srt => format!("{} - {}.srt", video.title, lang),
                };
                let out_path = unique_path(&folder_path.join(&filename));
                fs::write(
                    &out_path,
                    match format {
                        SubFormat::Vtt => write_vtt(&cues, lang),
                        SubFormat::Srt => write_srt(&cues),
                    }
                    .as_bytes(),
                )?;
                Ok(out_path.file_name().unwrap().to_string_lossy().into_owned())
            })();

            for p in &temp_files {
                let _ = fs::remove_file(p);
            }
            pb.finish_and_clear();
            match &result {
                Ok(f) => {
                    saved += 1;
                    println!(
                        "  {}  {}  {}",
                        style("✓").green().bold(),
                        style(lang).cyan().bold(),
                        style(f).dim()
                    );
                }
                Err(e) => eprintln!(
                    "  {}  {}  {}",
                    style("✗").red().bold(),
                    style(lang).cyan().bold(),
                    style(e.to_string()).red()
                ),
            }
        }
    }

    println!();
    print_summary(saved, total);
}
