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

mod cli;

use clap::Parser;
use console::style;
use std::fs;
use std::path::{Path, PathBuf};

use subscrub::core::{
    downloader::{
        detect_browser, download_with_retry, fetch_playlist,
        get_video_title, is_playlist_url,
        list_available_subs, require_ffmpeg, require_yt_dlp,
        resolve_output_dir, temp_id,
    },
    merger::{match_videos_to_subs, merge_single, merge_video},
    parser::process_json3,
    types::{SubFormat, SubType},
    writer::{write_srt, write_vtt},
};

use cli::{
    detect_download_mode, detect_merge_mode,
    parse_format, parse_languages, parse_sub_type,
    Cli, MergeType, Mode, Task,
};
use cli::ui::{
    ask_dir, ask_file, ask_format, ask_languages, ask_merge_type,
    ask_output_dir, ask_sub_files, ask_sub_type, ask_task, ask_url,
    make_spinner, print_banner, show_browser,
};


// ── browser resolution ────────────────────────────────────────────────────────

fn resolve_browser(cli_browser: &Option<String>) -> String {
    if let Some(b) = cli_browser { return b.clone(); }
    match detect_browser() {
        Some(b) => b,
        None => {
            eprintln!("\n  {}  No supported browser found. Use --browser to specify one.\n", style("✗").red().bold());
            std::process::exit(1);
        }
    }
}


// ── shared helpers ────────────────────────────────────────────────────────────

fn print_summary(saved: usize, total: usize) {
    println!("  {}", style("─".repeat(44)).dim());
    if saved == total {
        println!("  {}  All {} file{} saved.", style("✓").green().bold(), style(total).green().bold(), if total == 1 { "" } else { "s" });
    } else {
        println!("  {}  {}/{} file{} saved.", style("▶").yellow().bold(), style(saved).green().bold(), total, if total == 1 { "" } else { "s" });
    }
    println!();
}


// ── single video download ─────────────────────────────────────────────────────

fn run_single(
    url: &str, sub_type: &SubType, browser: &str,
    mode: Mode, cli_lang: Option<&str>, cli_fmt: Option<&str>,
    output_dir: &Option<PathBuf>,
) {
    println!();
    let pb = make_spinner("Fetching available subtitles…".to_string());
    let languages = list_available_subs(url, sub_type, browser);
    pb.finish_and_clear();

    if languages.is_empty() {
        eprintln!("  {}  No subtitles found for this video.", style("✗").red().bold());
        return;
    }

    println!("  {}  {} language{} available", style("✓").green().bold(), style(languages.len()).cyan().bold(), if languages.len() == 1 { "" } else { "s" });

    println!();
    let chosen = match mode {
        Mode::Interactive => ask_languages(&languages),
        Mode::Flags       => parse_languages(cli_lang.unwrap(), &languages),
    };

    println!();
    let format = match mode {
        Mode::Interactive => ask_format(),
        Mode::Flags       => parse_format(cli_fmt.unwrap()),
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
    let total     = chosen.len();
    let mut saved = 0usize;

    for (n, idx) in chosen.iter().enumerate() {
        let lang        = &languages[*idx];
        let temp_prefix = format!("temp_subs_{}_{}", lang, temp_id());
        let pb = make_spinner(format!("[{}/{}]  {}  Downloading…", n + 1, total, style(lang).cyan().bold()));
        let mut temp_files: Vec<String> = Vec::new();

        let result: Result<String, Box<dyn std::error::Error>> = (|| {
            let json3_path = download_with_retry(url, lang, sub_type, &temp_prefix, browser, 3)?;
            temp_files.push(json3_path.clone());
            pb.set_message(format!("[{}/{}]  {}  Processing…", n + 1, total, style(lang).cyan().bold()));
            let cues     = process_json3(&json3_path)?;
            let filename = match format { SubFormat::Vtt => format!("{title} - {lang}.vtt"), SubFormat::Srt => format!("{title} - {lang}.srt") };
            let out_path = downloads.join(&filename);
            fs::write(&out_path, match format { SubFormat::Vtt => write_vtt(&cues, lang), SubFormat::Srt => write_srt(&cues) }.as_bytes())?;
            Ok(out_path.to_string_lossy().into_owned())
        })();

        for p in &temp_files { let _ = fs::remove_file(p); }
        pb.finish_and_clear();
        match &result {
            Ok(p)  => { saved += 1; println!("  {}  {}  {}", style("✓").green().bold(), style(lang).cyan().bold(), style(p).dim()); }
            Err(e) => eprintln!("  {}  {}  {}", style("✗").red().bold(), style(lang).cyan().bold(), style(e.to_string()).red()),
        }
    }

    println!();
    print_summary(saved, total);
}


// ── playlist download ─────────────────────────────────────────────────────────

fn run_playlist(
    url: &str, sub_type: &SubType, browser: &str,
    mode: Mode, cli_lang: Option<&str>, cli_fmt: Option<&str>,
    output_dir: &Option<PathBuf>,
) {
    println!();
    let pb = make_spinner("Fetching playlist info…".to_string());
    let playlist = match fetch_playlist(url, browser) {
        Ok(p)  => { pb.finish_and_clear(); p }
        Err(e) => { pb.finish_and_clear(); eprintln!("  {}  {}", style("✗").red().bold(), e); return; }
    };

    println!("  {}  {}  {}", style("▶").dim(), style(&playlist.title).bold(), style(format!("({} videos)", playlist.videos.len())).dim());

    if playlist.videos.is_empty() { eprintln!("  {}  Playlist is empty.", style("✗").red().bold()); return; }

    println!();
    let pb = make_spinner("Fetching available subtitles…".to_string());
    let languages = list_available_subs(&playlist.videos[0].url, sub_type, browser);
    pb.finish_and_clear();

    if languages.is_empty() { eprintln!("  {}  No subtitles found on the first video.", style("✗").red().bold()); return; }

    println!("  {}  {} language{} available  {}", style("✓").green().bold(), style(languages.len()).cyan().bold(), if languages.len() == 1 { "" } else { "s" }, style("(based on first video)").dim());

    println!();
    let chosen = match mode { Mode::Interactive => ask_languages(&languages), Mode::Flags => parse_languages(cli_lang.unwrap(), &languages) };

    println!();
    let format = match mode { Mode::Interactive => ask_format(), Mode::Flags => parse_format(cli_fmt.unwrap()) };

    let folder_path = resolve_output_dir(output_dir).join(format!("{} subs", playlist.title));
    if let Err(e) = fs::create_dir_all(&folder_path) { eprintln!("  {}  {}", style("✗").red().bold(), e); return; }

    println!();
    println!("  {}  {}", style("▶").dim(), style(folder_path.display()).bold());
    println!();
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let total = playlist.videos.len() * chosen.len();
    let mut saved = 0usize;
    let mut n     = 0usize;

    for video in &playlist.videos {
        for idx in &chosen {
            n += 1;
            let lang        = &languages[*idx];
            let temp_prefix = format!("temp_subs_{}_{}", lang, temp_id());
            let pb = make_spinner(format!("[{n}/{total}]  {}  {}", style(lang).cyan().bold(), style(&video.title).dim()));
            let mut temp_files: Vec<String> = Vec::new();

            let result: Result<String, Box<dyn std::error::Error>> = (|| {
                let json3_path = download_with_retry(&video.url, lang, sub_type, &temp_prefix, browser, 3)?;
                temp_files.push(json3_path.clone());
                pb.set_message(format!("[{n}/{total}]  {}  Processing…", style(lang).cyan().bold()));
                let cues     = process_json3(&json3_path)?;
                let filename = match format { SubFormat::Vtt => format!("{} - {}.vtt", video.title, lang), SubFormat::Srt => format!("{} - {}.srt", video.title, lang) };
                let out_path = folder_path.join(&filename);
                fs::write(&out_path, match format { SubFormat::Vtt => write_vtt(&cues, lang), SubFormat::Srt => write_srt(&cues) }.as_bytes())?;
                Ok(filename)
            })();

            for p in &temp_files { let _ = fs::remove_file(p); }
            pb.finish_and_clear();
            match &result {
                Ok(f)  => { saved += 1; println!("  {}  {}  {}", style("✓").green().bold(), style(lang).cyan().bold(), style(f).dim()); }
                Err(e) => eprintln!("  {}  {}  {}", style("✗").red().bold(), style(lang).cyan().bold(), style(e.to_string()).red()),
            }
        }
    }

    println!();
    print_summary(saved, total);
}


// ── merge folder ──────────────────────────────────────────────────────────────

fn run_merge_folder(videos_dir: &Path, subs_dir: &Path, output_dir: &Option<PathBuf>) {
    if !videos_dir.is_dir() { eprintln!("  {}  Videos folder not found.", style("✗").red().bold()); return; }
    if !subs_dir.is_dir()   { eprintln!("  {}  Subtitles folder not found.", style("✗").red().bold()); return; }

    println!();
    let pb = make_spinner("Matching videos with subtitles…".to_string());
    let (jobs, unmatched) = match_videos_to_subs(videos_dir, subs_dir);
    pb.finish_and_clear();

    if jobs.is_empty() { eprintln!("  {}  No matches found.", style("✗").red().bold()); return; }

    println!("  {}  {} video{} matched", style("✓").green().bold(), style(jobs.len()).cyan().bold(), if jobs.len() == 1 { "" } else { "s" });

    for p in &unmatched {
        eprintln!("  {}  no subtitles for: {}", style("!").yellow().bold(), style(p.file_name().unwrap().to_string_lossy()).dim());
    }

    let stage3 = jobs.iter().filter(|j| j.match_stage == 3).count();
    if stage3 > 0 {
        eprintln!("  {}  {} video{} matched by position — verify manually", style("!").yellow().bold(), stage3, if stage3 == 1 { "" } else { "s" });
    }

    let output_dir = match output_dir {
        Some(p) => p.clone(),
        None    => videos_dir.parent().unwrap_or(Path::new("."))
            .join(format!("{} merged", videos_dir.file_name().unwrap().to_string_lossy())),
    };

    if let Err(e) = fs::create_dir_all(&output_dir) { eprintln!("  {}  {}", style("✗").red().bold(), e); return; }

    println!();
    println!("  {}  {}", style("▶").dim(), style(output_dir.display()).bold());
    println!();
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let total     = jobs.len();
    let mut saved = 0usize;

    for (n, job) in jobs.iter().enumerate() {
        let stage_label = match job.match_stage { 1 => style("id   ").green(), 2 => style("title").cyan(), _ => style("pos  ").yellow() };
        let pb = make_spinner(format!("[{}/{}]  {}  {}", n + 1, total, stage_label, style(&job.output_name).dim()));
        let result = merge_video(job, &output_dir);
        pb.finish_and_clear();

        match result {
            Ok(_)  => { saved += 1; println!("  {}  [{}]  {}", style("✓").green().bold(), stage_label, style(format!("{}.mkv", job.output_name)).dim()); }
            Err(e) => eprintln!("  {}  {}  {}", style("✗").red().bold(), style(&job.output_name).dim(), style(e.to_string()).red()),
        }
    }

    println!();
    print_summary(saved, total);
}


// ── merge single ──────────────────────────────────────────────────────────────

fn run_merge_single(video_path: &Path, sub_paths: &[PathBuf], output_dir: &Option<PathBuf>) {
    if !video_path.is_file() {
        eprintln!("  {}  Video file not found: {}", style("✗").red().bold(), video_path.display());
        return;
    }
    for sub in sub_paths {
        if !sub.is_file() {
            eprintln!("  {}  Subtitle file not found: {}", style("✗").red().bold(), sub.display());
            return;
        }
    }

    if let Some(dir) = output_dir {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("  {}  {}", style("✗").red().bold(), e);
            return;
        }
    }

    println!();
    println!("  {}  {}", style("▶").dim(), style(video_path.file_name().unwrap().to_string_lossy()).bold());
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let pb = make_spinner("Merging…".to_string());
    let result = merge_single(video_path, sub_paths, output_dir.as_deref());
    pb.finish_and_clear();

    match result {
        Ok(output) => {
            println!("  {}  {}", style("✓").green().bold(), style(output.display()).dim());
            println!();
            print_summary(1, 1);
        }
        Err(e) => eprintln!("  {}  {}", style("✗").red().bold(), style(e.to_string()).red()),
    }
}


// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    print_banner();

    let custom_output: Option<PathBuf> = cli.output.as_deref().map(PathBuf::from);

    // ── determine task ────────────────────────────────────────────────────────
    let task = if cli.merge {
        Task::Merge
    } else if cli.url.is_some() || cli.sub_type.is_some() || cli.lang.is_some() || cli.format.is_some() {
        Task::Download
    } else {
        ask_task()
    };

    match task {

        // ── download ──────────────────────────────────────────────────────────
        Task::Download => {
            require_yt_dlp();
            let mode    = detect_download_mode(&cli);
            let browser = resolve_browser(&cli.browser);
            show_browser(&browser);

            let output_dir = match mode {
                Mode::Interactive if custom_output.is_none() => {
                    println!();
                    ask_output_dir("Downloads folder")
                }
                _ => custom_output.clone(),
            };

            let url = match mode {
                Mode::Interactive => { println!(); ask_url() }
                Mode::Flags       => cli.url.as_deref().unwrap().trim().to_string(),
            };

            if url.is_empty() { eprintln!("  {}  No URL provided.", style("✗").red().bold()); std::process::exit(1); }

            println!();
            let sub_type = match mode {
                Mode::Interactive => ask_sub_type(),
                Mode::Flags       => parse_sub_type(cli.sub_type.as_deref().unwrap()),
            };

            if is_playlist_url(&url) {
                run_playlist(&url, &sub_type, &browser, mode, cli.lang.as_deref(), cli.format.as_deref(), &output_dir);
            } else {
                run_single(&url, &sub_type, &browser, mode, cli.lang.as_deref(), cli.format.as_deref(), &output_dir);
            }
        }

        // ── merge ─────────────────────────────────────────────────────────────
        Task::Merge => {
            require_ffmpeg();
            let mode = detect_merge_mode(&cli);

            let is_single_flags = cli.video.is_some() || !cli.sub.is_empty();

            let output_dir = match mode {
                Mode::Interactive if custom_output.is_none() => {
                    println!();
                    ask_output_dir("alongside the source files")
                }
                _ => custom_output.clone(),
            };

            match mode {
                Mode::Flags => {
                    if is_single_flags {
                        let video    = PathBuf::from(cli.video.as_deref().unwrap());
                        let sub_paths: Vec<PathBuf> = cli.sub.iter().map(PathBuf::from).collect();
                        run_merge_single(&video, &sub_paths, &output_dir);
                    } else {
                        let videos_dir = PathBuf::from(cli.videos_dir.as_deref().unwrap());
                        let subs_dir   = PathBuf::from(cli.subs_dir.as_deref().unwrap());
                        run_merge_folder(&videos_dir, &subs_dir, &output_dir);
                    }
                }

                Mode::Interactive => {
                    println!();
                    match ask_merge_type() {
                        MergeType::Folder => {
                            let videos_dir = ask_dir("Videos folder:");
                            let subs_dir   = ask_dir("Subtitles folder:");
                            run_merge_folder(&videos_dir, &subs_dir, &output_dir);
                        }
                        MergeType::Single => {
                            let video     = ask_file("Video file:");
                            let sub_paths = ask_sub_files();
                            run_merge_single(&video, &sub_paths, &output_dir);
                        }
                    }
                }
            }
        }
    }
}
