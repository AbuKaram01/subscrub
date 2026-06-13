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
use inquire::{Select, MultiSelect, Text};
use std::path::PathBuf;
use std::time::Duration;

use subscrub::core::types::{SubFormat, SubType};
use super::{MergeType, Task};


// ── banner & spinner ──────────────────────────────────────────────────────────

pub fn print_banner() {
    println!();
    println!("  {}  {}", style("▶").red().bold(), style("subscrub").white().bold());
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


// ── interactive prompts ───────────────────────────────────────────────────────

pub fn ask_task() -> Task {
    let choice = Select::new(
        "What do you want to do?",
        vec!["Download subtitles", "Merge subtitles into videos"],
    )
    .prompt()
    .expect("Selection failed");

    if choice == "Download subtitles" { Task::Download } else { Task::Merge }
}

pub fn ask_merge_type() -> MergeType {
    let choice = Select::new(
        "Merge mode",
        vec![
            "Folder  ·  match and merge entire folders",
            "Single  ·  merge one video with subtitle(s)",
        ],
    )
    .prompt()
    .expect("Selection failed");

    if choice.starts_with("Folder") { MergeType::Folder } else { MergeType::Single }
}

pub fn ask_url() -> String {
    Text::new("YouTube URL:")
        .prompt()
        .expect("Input failed")
        .trim()
        .to_string()
}

pub fn ask_sub_type() -> SubType {
    let choice = Select::new(
        "Subtitle type",
        vec!["Manual (community)", "Auto-generated"],
    )
    .prompt()
    .expect("Selection failed");

    if choice == "Manual (community)" { SubType::Manual } else { SubType::Auto }
}

pub fn ask_languages(languages: &[String]) -> Vec<usize> {
    let chosen = MultiSelect::new(
        "Select languages  (type to search, Space = toggle, Enter = confirm)",
        languages.to_vec(),
    )
    .with_vim_mode(true)
    .prompt()
    .unwrap_or_default();

    if chosen.is_empty() {
        eprintln!("  {}  No languages selected — exiting.", style("✗").red().bold());
        std::process::exit(1);
    }

    let indices: Vec<usize> = chosen
        .iter()
        .filter_map(|c| languages.iter().position(|l| l == c))
        .collect();

    println!("  {}  {}", style("✓").green().bold(), style(chosen.join("  ·  ")).cyan());
    indices
}

pub fn ask_format() -> SubFormat {
    let choice = Select::new(
        "Output format",
        vec!["VTT  ·  cleaned", "SRT  ·  cleaned"],
    )
    .prompt()
    .expect("Selection failed");

    if choice == "VTT  ·  cleaned" { SubFormat::Vtt } else { SubFormat::Srt }
}

/// Asks for an optional output folder. Empty input keeps the default location.
pub fn ask_output_dir(default_hint: &str) -> Option<PathBuf> {
    let prompt  = format!("Save folder  (Enter = {default_hint}):");
    let input   = Text::new(&prompt).prompt().expect("Input failed");
    let trimmed = input.trim();

    if trimmed.is_empty() { return None; }
    Some(PathBuf::from(trimmed))
}

/// Asks for a folder path and keeps prompting until a valid directory is entered.
pub fn ask_dir(prompt: &str) -> PathBuf {
    loop {
        let input = Text::new(prompt).prompt().expect("Input failed");
        let path  = PathBuf::from(input.trim());
        if path.is_dir() { return path; }
        eprintln!("  {}  Path not found or is not a folder — try again.", style("✗").red().bold());
    }
}

/// Asks for a file path and keeps prompting until a valid file is entered.
pub fn ask_file(prompt: &str) -> PathBuf {
    loop {
        let input = Text::new(prompt).prompt().expect("Input failed");
        let path  = PathBuf::from(input.trim());
        if path.is_file() { return path; }
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

        let input = Text::new(&prompt).prompt().expect("Input failed");
        let trimmed = input.trim();

        if trimmed.is_empty() {
            if files.is_empty() {
                eprintln!("  {}  Add at least one subtitle file.", style("!").yellow().bold());
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

    println!("  {}  {}", style("✓").green().bold(), style(names.join("  ·  ")).cyan());
    files
}
