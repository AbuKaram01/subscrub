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

use clap::{Args, Parser, Subcommand};
use console::style;

use subscrub::core::types::{SubFormat, SubType};

// ── task & merge-type (only needed for the bare `subscrub` interactive entry) ──

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
    name = "subscrub",
    version,
    about = "Download & clean YouTube subtitles",
    long_about = "\
Run with no arguments for a guided interactive session.\n\
Run a subcommand with ALL required flags to skip every prompt (scripting).\n\n\
  Interactive     : subscrub\n\
  Download        : subscrub download --url <URL> --type <TYPE> --lang <LANGS> --format <FORMAT>\n\
  List languages  : subscrub languages --url <URL>\n\
  Merge folder    : subscrub merge folder --videos-dir <PATH> --subs-dir <PATH>\n\
  Merge single    : subscrub merge single --video <PATH> --sub <PATH> [--sub <PATH> ...]"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Download subtitles from a video or playlist
    Download(DownloadArgs),
    /// List available subtitle languages (manual + auto) for a URL
    Languages(LanguagesArgs),
    /// Merge subtitles into videos using ffmpeg
    Merge {
        #[command(subcommand)]
        mode: Option<MergeCommand>,
    },
}

#[derive(Args, Default)]
pub struct DownloadArgs {
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

    /// Output folder — where downloaded subtitles are saved
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<String>,

    /// Browser for cookie extraction — auto-detected if omitted
    #[arg(short, long, value_name = "BROWSER",
          value_parser = ["firefox", "chrome", "brave", "edge", "chromium", "opera", "vivaldi"])]
    pub browser: Option<String>,
}

#[derive(Args)]
pub struct LanguagesArgs {
    /// YouTube video or playlist URL
    #[arg(long, value_name = "URL")]
    pub url: String,

    /// Browser for cookie extraction — auto-detected if omitted
    #[arg(short, long, value_name = "BROWSER",
          value_parser = ["firefox", "chrome", "brave", "edge", "chromium", "opera", "vivaldi"])]
    pub browser: Option<String>,
}

#[derive(Subcommand)]
pub enum MergeCommand {
    /// Match and merge an entire folder of videos with a folder of subtitles
    Folder(FolderMergeArgs),
    /// Merge one video with one or more subtitle files
    Single(SingleMergeArgs),
}

#[derive(Args)]
pub struct FolderMergeArgs {
    /// Videos folder path
    #[arg(long, value_name = "PATH")]
    pub videos_dir: String,

    /// Subtitles folder path
    #[arg(long, value_name = "PATH")]
    pub subs_dir: String,

    /// Output folder — where merged videos are saved
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<String>,
}

#[derive(Args)]
pub struct SingleMergeArgs {
    /// Video file path
    #[arg(long, value_name = "PATH")]
    pub video: String,

    /// Subtitle file path (repeatable, at least one required)
    #[arg(long = "sub", value_name = "PATH", required = true)]
    pub sub: Vec<String>,

    /// Output folder — defaults to alongside the source video
    #[arg(short = 'o', long, value_name = "PATH")]
    pub output: Option<String>,
}

// ── download flag validation ──────────────────────────────────────────────────
//
// clap's derive API can't express "either none of these flags, or all of
// them" directly, so this stays hand-rolled — but unlike the old version it
// just reports the outcome. It never prints or exits; the `commands` layer
// decides what to do with the result.

#[derive(Clone, Copy, Debug)]
pub enum DownloadMode {
    Interactive,
    Flags,
}

/// Decides whether a `download` invocation should run interactively or
/// straight from flags. Returns the list of missing flag names if the user
/// supplied some but not all of the required ones.
pub fn validate_download_args(args: &DownloadArgs) -> Result<DownloadMode, Vec<&'static str>> {
    let any = args.url.is_some()
        || args.sub_type.is_some()
        || args.lang.is_some()
        || args.format.is_some();

    let all = args.url.is_some()
        && args.sub_type.is_some()
        && args.lang.is_some()
        && args.format.is_some();

    match (any, all) {
        (false, _) => Ok(DownloadMode::Interactive),
        (true, true) => Ok(DownloadMode::Flags),
        (true, false) => {
            let missing: Vec<&'static str> = [
                args.url.is_none().then_some("--url"),
                args.sub_type.is_none().then_some("--type"),
                args.lang.is_none().then_some("--lang"),
                args.format.is_none().then_some("--format"),
            ]
            .into_iter()
            .flatten()
            .collect();
            Err(missing)
        }
    }
}

// ── flag parsers (pure — no printing, no exiting) ─────────────────────────────

pub fn parse_sub_type(s: &str) -> SubType {
    match s {
        "manual" => SubType::Manual,
        _ => SubType::Auto,
    }
}

pub fn parse_format(s: &str) -> SubFormat {
    match s {
        "vtt" => SubFormat::Vtt,
        _ => SubFormat::Srt,
    }
}

/// Resolves comma-separated requested language codes against the languages
/// actually available. Returns the matched indices, or an error message if
/// none of the requested languages exist.
pub fn parse_languages(arg: &str, languages: &[String]) -> Result<Vec<usize>, String> {
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
        return Err("None of the requested languages are available.".to_string());
    }

    let chosen: Vec<&str> = indices.iter().map(|&i| languages[i].as_str()).collect();
    println!(
        "  {}  {}",
        style("✓").green().bold(),
        style(chosen.join("  ·  ")).cyan()
    );

    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(url: bool, sub_type: bool, lang: bool, format: bool) -> DownloadArgs {
        DownloadArgs {
            url: url.then(|| "https://youtube.com/watch?v=x".to_string()),
            sub_type: sub_type.then(|| "auto".to_string()),
            lang: lang.then(|| "en".to_string()),
            format: format.then(|| "srt".to_string()),
            output: None,
            browser: None,
        }
    }

    #[test]
    fn no_flags_means_interactive() {
        assert!(matches!(
            validate_download_args(&args(false, false, false, false)),
            Ok(DownloadMode::Interactive)
        ));
    }

    #[test]
    fn all_flags_means_flags_mode() {
        assert!(matches!(
            validate_download_args(&args(true, true, true, true)),
            Ok(DownloadMode::Flags)
        ));
    }

    #[test]
    fn partial_flags_report_exactly_whats_missing() {
        let missing = validate_download_args(&args(true, false, true, false)).unwrap_err();
        assert_eq!(missing, vec!["--type", "--format"]);
    }
}
