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

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{InquireError, MultiSelect, Select, Text};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::{MergeType, Task};
use subscrub::core::downloader::is_valid_youtube_url;
use subscrub::core::types::{SubFormat, SubType};

// ── banner & spinner ──────────────────────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!(
        "  {}  {}",
        style("▶").red().bold(),
        style("subscrub").white().bold()
    );
    println!("  {}", style("YouTube subtitle downloader & cleaner").dim());
    println!("  {}", style("─".repeat(44)).dim());
    println!();
}

pub fn show_browser(browser: &str) {
    println!(
        "  {}  browser  {}",
        style("✓").green().bold(),
        style(browser).cyan().bold()
    );
}

pub fn make_spinner(msg: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.cyan.bold}  {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Prints the final "N/N files saved" line shared by every download/merge run.
pub fn print_summary(saved: usize, total: usize) {
    println!("  {}", style("─".repeat(44)).dim());
    if saved == total {
        println!(
            "  {}  All {} file{} saved.",
            style("✓").green().bold(),
            style(total).green().bold(),
            if total == 1 { "" } else { "s" }
        );
    } else {
        println!(
            "  {}  {}/{} file{} saved.",
            style("▶").yellow().bold(),
            style(saved).green().bold(),
            total,
            if total == 1 { "" } else { "s" }
        );
    }
    println!();
}

// ── prompt cancellation ───────────────────────────────────────────────────────
//
// Every interactive prompt goes through this so pressing Esc/Ctrl+C prints
// one clean line and exits, instead of the raw inquire panic message.

fn finish<T>(result: Result<T, InquireError>) -> T {
    match result {
        Ok(value) => value,
        Err(InquireError::OperationCanceled) | Err(InquireError::OperationInterrupted) => {
            println!();
            println!("  {}  Cancelled.", style("✗").red().bold());
            std::process::exit(130);
        }
        Err(e) => {
            eprintln!("  {}  {}", style("✗").red().bold(), e);
            std::process::exit(1);
        }
    }
}

// ── interactive prompts ───────────────────────────────────────────────────────

pub fn ask_task() -> Task {
    let choice = finish(
        Select::new(
            "What do you want to do?",
            vec!["Download subtitles", "Merge subtitles into videos"],
        )
        .prompt(),
    );

    if choice == "Download subtitles" {
        Task::Download
    } else {
        Task::Merge
    }
}

pub fn ask_merge_type() -> MergeType {
    let choice = finish(
        Select::new(
            "Merge mode",
            vec![
                "Folder  ·  match and merge entire folders",
                "Single  ·  merge one video with subtitle(s)",
            ],
        )
        .prompt(),
    );

    if choice.starts_with("Folder") {
        MergeType::Folder
    } else {
        MergeType::Single
    }
}

/// Asks for a YouTube URL and keeps prompting until a valid one is entered.
pub fn ask_url() -> String {
    loop {
        let input = finish(Text::new("YouTube URL:").prompt());
        let url = input.trim().to_string();

        if url.is_empty() {
            eprintln!(
                "  {}  URL can't be empty — try again.",
                style("✗").red().bold()
            );
            continue;
        }
        if !is_valid_youtube_url(&url) {
            eprintln!(
                "  {}  Not a valid YouTube URL — try again.",
                style("✗").red().bold()
            );
            continue;
        }
        return url;
    }
}

/// Presents every manual *and* auto-generated language together — each
/// tagged with its type — and lets the user multi-select across both in
/// one step, re-prompting until at least one is picked. Manual/auto
/// variants of the same language code sit next to each other.
pub fn ask_language_choices(manual: &[String], auto: &[String]) -> Vec<(String, SubType)> {
    let mut codes: Vec<&String> = manual.iter().chain(auto.iter()).collect();
    codes.sort();
    codes.dedup();

    let mut options: Vec<(String, SubType)> = Vec::new();
    for code in codes {
        if manual.iter().any(|l| l == code) {
            options.push((code.clone(), SubType::Manual));
        }
        if auto.iter().any(|l| l == code) {
            options.push((code.clone(), SubType::Auto));
        }
    }

    let labels: Vec<String> = options
        .iter()
        .map(|(lang, sub_type)| match sub_type {
            SubType::Manual => format!("{lang}  ·  manual"),
            SubType::Auto => format!("{lang}  ·  auto"),
        })
        .collect();

    loop {
        let chosen = finish(
            MultiSelect::new(
                "Select languages  (type to search, Space = toggle, Enter = confirm)",
                labels.clone(),
            )
            .with_vim_mode(true)
            .prompt(),
        );

        if chosen.is_empty() {
            eprintln!(
                "  {}  Select at least one language — try again.",
                style("✗").red().bold()
            );
            continue;
        }

        let selected: Vec<(String, SubType)> = chosen
            .iter()
            .filter_map(|c| {
                labels
                    .iter()
                    .position(|l| l == c)
                    .map(|i| options[i].clone())
            })
            .collect();

        println!(
            "  {}  {}",
            style("✓").green().bold(),
            style(chosen.join("  ·  ")).cyan()
        );
        return selected;
    }
}

pub fn ask_format() -> SubFormat {
    let choice =
        finish(Select::new("Output format", vec!["VTT  ·  cleaned", "SRT  ·  cleaned"]).prompt());

    if choice == "VTT  ·  cleaned" {
        SubFormat::Vtt
    } else {
        SubFormat::Srt
    }
}

/// Asks for an optional output folder. Empty input keeps the default location.
/// Keeps prompting until the folder actually exists or can be created there
/// (catches typos, bad permissions, or a path that points at an existing file).
pub fn ask_output_dir(default_hint: &str) -> Option<PathBuf> {
    let prompt = format!("Save folder  (Enter = {default_hint}):");
    loop {
        let input = finish(Text::new(&prompt).prompt());
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return None;
        }

        let path = PathBuf::from(trimmed);
        match fs::create_dir_all(&path) {
            Ok(()) => return Some(path),
            Err(e) => {
                eprintln!(
                    "  {}  Can't use that folder ({e}) — try again.",
                    style("✗").red().bold()
                );
            }
        }
    }
}

/// Asks for a folder path and keeps prompting until a valid directory is entered.
pub fn ask_dir(prompt: &str) -> PathBuf {
    loop {
        let input = finish(Text::new(prompt).prompt());
        let path = PathBuf::from(input.trim());
        if path.is_dir() {
            return path;
        }
        eprintln!(
            "  {}  Path not found or is not a folder — try again.",
            style("✗").red().bold()
        );
    }
}

/// Asks for a file path and keeps prompting until a valid file is entered.
pub fn ask_file(prompt: &str) -> PathBuf {
    loop {
        let input = finish(Text::new(prompt).prompt());
        let path = PathBuf::from(input.trim());
        if path.is_file() {
            return path;
        }
        eprintln!("  {}  File not found — try again.", style("✗").red().bold());
    }
}

/// Asks for subtitle files one by one until the user presses Enter on an empty line.
pub fn ask_sub_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    loop {
        let prompt = if files.is_empty() {
            "Subtitle file 1:".to_string()
        } else {
            format!("Subtitle file {}  (Enter to finish):", files.len() + 1)
        };

        let input = finish(Text::new(&prompt).prompt());
        let trimmed = input.trim();

        if trimmed.is_empty() {
            if files.is_empty() {
                eprintln!(
                    "  {}  Add at least one subtitle file.",
                    style("!").yellow().bold()
                );
                continue;
            }
            break;
        }

        let path = PathBuf::from(trimmed);
        if path.is_file() {
            files.push(path);
        } else {
            eprintln!("  {}  File not found — try again.", style("✗").red().bold());
        }
    }

    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();

    println!(
        "  {}  {}",
        style("✓").green().bold(),
        style(names.join("  ·  ")).cyan()
    );
    files
}
