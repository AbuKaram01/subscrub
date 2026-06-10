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

pub mod ui;

use clap::Parser;
use console::style;

use subscrub::core::types::{SubFormat, SubType};


// ── task & merge type ─────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum Task {
    Download,
    Merge,
}

#[derive(Clone, Copy)]
pub enum MergeType {
    Folder,
    Single,
}


// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name       = "subscrub",
    about      = "Download & clean YouTube subtitles",
    long_about = "\
Run with no flags for a guided interactive session.\n\
Run with ALL required flags to skip every prompt (scripting).\n\n\
  Interactive  : subscrub\n\
  Download     : subscrub --url <URL> --type <TYPE> --lang <LANGS> --format <FORMAT>\n\
  Merge folder : subscrub --merge --videos-dir <PATH> --subs-dir <PATH>\n\
  Merge single : subscrub --merge --video <PATH> --sub <PATH> [--sub <PATH> ...]"
)]
pub struct Cli {
    /// YouTube video or playlist URL
    #[arg(long, value_name = "URL")]
    pub url: Option<String>,

    /// Subtitle type: manual or auto
    #[arg(short = 't', long = "type", value_name = "TYPE", value_parser = ["manual", "auto"])]
    pub sub_type: Option<String>,

    /// Language codes, comma-separated (e.g. ar,en,fr)
    #[arg(short, long, value_name = "LANGS")]
    pub lang: Option<String>,

    /// Output format: vtt or srt
    #[arg(short, long, value_name = "FORMAT", value_parser = ["vtt", "srt"])]
    pub format: Option<String>,

    /// Browser for cookie extraction — auto-detected if omitted
    #[arg(short, long, value_name = "BROWSER",
          value_parser = ["firefox", "chrome", "brave", "edge", "chromium", "opera", "vivaldi"])]
    pub browser: Option<String>,

    /// Merge mode: embed subtitles into videos using ffmpeg
    #[arg(long)]
    pub merge: bool,

    /// Videos folder path (merge folder mode)
    #[arg(long, value_name = "PATH")]
    pub videos_dir: Option<String>,

    /// Subtitles folder path (merge folder mode)
    #[arg(long, value_name = "PATH")]
    pub subs_dir: Option<String>,

    /// Single video file path (merge single mode)
    #[arg(long, value_name = "PATH")]
    pub video: Option<String>,

    /// Subtitle file path (merge single mode, repeatable)
    #[arg(long = "sub", value_name = "PATH")]
    pub sub: Vec<String>,
}


// ── mode detection ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum Mode {
    Interactive,
    Flags,
}

pub fn detect_download_mode(cli: &Cli) -> Mode {
    let any = cli.url.is_some()
        || cli.sub_type.is_some()
        || cli.lang.is_some()
        || cli.format.is_some();

    let all = cli.url.is_some()
        && cli.sub_type.is_some()
        && cli.lang.is_some()
        && cli.format.is_some();

    match (any, all) {
        (false, _)    => Mode::Interactive,
        (true, true)  => Mode::Flags,
        (true, false) => {
            let missing: Vec<&str> = [
                cli.url     .is_none().then_some("--url"),
                cli.sub_type.is_none().then_some("--type"),
                cli.lang    .is_none().then_some("--lang"),
                cli.format  .is_none().then_some("--format"),
            ]
            .into_iter().flatten().collect();

            eprintln!(
                "\n  {}  download mode requires: {}\n",
                style("✗").red().bold(),
                style(missing.join("  ·  ")).cyan()
            );
            std::process::exit(1);
        }
    }
}

/// Returns the mode for merge task.
/// Also validates that folder flags and single flags are not mixed.
pub fn detect_merge_mode(cli: &Cli) -> Mode {
    let has_folder = cli.videos_dir.is_some() || cli.subs_dir.is_some();
    let has_single = cli.video.is_some() || !cli.sub.is_empty();

    if has_folder && has_single {
        eprintln!(
            "\n  {}  cannot mix --videos-dir/--subs-dir with --video/--sub\n",
            style("✗").red().bold()
        );
        std::process::exit(1);
    }

    if has_folder {
        return match (cli.videos_dir.is_some(), cli.subs_dir.is_some()) {
            (true, true)  => Mode::Flags,
            (true, false) => { eprintln!("\n  {}  folder merge requires: --subs-dir\n",   style("✗").red().bold()); std::process::exit(1); }
            (false, _)    => { eprintln!("\n  {}  folder merge requires: --videos-dir\n", style("✗").red().bold()); std::process::exit(1); }
        };
    }

    if has_single {
        return match (cli.video.is_some(), !cli.sub.is_empty()) {
            (true, true)  => Mode::Flags,
            (true, false) => { eprintln!("\n  {}  single merge requires: --sub\n",   style("✗").red().bold()); std::process::exit(1); }
            (false, _)    => { eprintln!("\n  {}  single merge requires: --video\n", style("✗").red().bold()); std::process::exit(1); }
        };
    }

    Mode::Interactive
}


// ── flags parsers ─────────────────────────────────────────────────────────────

pub fn parse_sub_type(s: &str) -> SubType {
    match s { "manual" => SubType::Manual, _ => SubType::Auto }
}

pub fn parse_format(s: &str) -> SubFormat {
    match s { "vtt" => SubFormat::Vtt, _ => SubFormat::Srt }
}

pub fn parse_languages(arg: &str, languages: &[String]) -> Vec<usize> {
    let requested: Vec<&str> = arg.split(',').map(str::trim).collect();

    for req in &requested {
        if !languages.iter().any(|l| l == req) {
            eprintln!(
                "  {}  language '{}' not available — skipping.",
                style("!").yellow().bold(),
                style(req).cyan()
            );
        }
    }

    let indices: Vec<usize> = requested
        .iter()
        .filter_map(|req| languages.iter().position(|l| l == req))
        .collect();

    if indices.is_empty() {
        eprintln!("  {}  None of the requested languages are available.", style("✗").red().bold());
        std::process::exit(1);
    }

    let chosen: Vec<&str> = indices.iter().map(|&i| languages[i].as_str()).collect();
    println!("  {}  {}", style("✓").green().bold(), style(chosen.join("  ·  ")).cyan());

    indices
}
