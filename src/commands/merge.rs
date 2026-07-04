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
use std::path::{Path, PathBuf};

use console::style;

use subscrub::core::{
    downloader::require_ffmpeg,
    merger::{match_videos_to_subs, merge_single, merge_video},
};

use crate::cli::ui::{
    ask_dir, ask_file, ask_merge_type, ask_output_dir, ask_sub_files, make_spinner, print_summary,
};
use crate::cli::{MergeCommand, MergeType};
use crate::commands::fail;

// ── entry point ───────────────────────────────────────────────────────────────

pub fn run(mode: Option<MergeCommand>) {
    require_ffmpeg().unwrap_or_else(|e| fail(e));

    match mode {
        Some(MergeCommand::Folder(args)) => {
            let videos_dir = PathBuf::from(args.videos_dir);
            let subs_dir = PathBuf::from(args.subs_dir);
            let output_dir = args.output.map(PathBuf::from);
            run_merge_folder(&videos_dir, &subs_dir, &output_dir);
        }
        Some(MergeCommand::Single(args)) => {
            let video = PathBuf::from(args.video);
            let sub_paths: Vec<PathBuf> = args.sub.into_iter().map(PathBuf::from).collect();
            let output_dir = args.output.map(PathBuf::from);
            run_merge_single(&video, &sub_paths, &output_dir);
        }
        None => run_interactive(),
    }
}

fn run_interactive() {
    println!();
    let output_dir = ask_output_dir("alongside the source files");

    println!();
    match ask_merge_type() {
        MergeType::Folder => {
            let videos_dir = ask_dir("Videos folder:");
            let subs_dir = ask_dir("Subtitles folder:");
            run_merge_folder(&videos_dir, &subs_dir, &output_dir);
        }
        MergeType::Single => {
            let video = ask_file("Video file:");
            let sub_paths = ask_sub_files();
            run_merge_single(&video, &sub_paths, &output_dir);
        }
    }
}

// ── merge folder ──────────────────────────────────────────────────────────────

fn run_merge_folder(videos_dir: &Path, subs_dir: &Path, output_dir: &Option<PathBuf>) {
    if !videos_dir.is_dir() {
        eprintln!("  {}  Videos folder not found.", style("✗").red().bold());
        return;
    }
    if !subs_dir.is_dir() {
        eprintln!("  {}  Subtitles folder not found.", style("✗").red().bold());
        return;
    }

    println!();
    let pb = make_spinner("Matching videos with subtitles…".to_string());
    let (jobs, unmatched) = match_videos_to_subs(videos_dir, subs_dir);
    pb.finish_and_clear();

    if jobs.is_empty() {
        eprintln!("  {}  No matches found.", style("✗").red().bold());
        return;
    }

    println!(
        "  {}  {} video{} matched",
        style("✓").green().bold(),
        style(jobs.len()).cyan().bold(),
        if jobs.len() == 1 { "" } else { "s" }
    );

    for p in &unmatched {
        eprintln!(
            "  {}  no subtitles for: {}",
            style("!").yellow().bold(),
            style(p.file_name().unwrap().to_string_lossy()).dim()
        );
    }

    let stage3 = jobs.iter().filter(|j| j.match_stage == 3).count();
    if stage3 > 0 {
        eprintln!(
            "  {}  {} video{} matched by position — verify manually",
            style("!").yellow().bold(),
            stage3,
            if stage3 == 1 { "" } else { "s" }
        );
    }

    let output_dir = match output_dir {
        Some(p) => p.clone(),
        None => videos_dir.parent().unwrap_or(Path::new(".")).join(format!(
            "{} merged",
            videos_dir.file_name().unwrap().to_string_lossy()
        )),
    };

    if let Err(e) = fs::create_dir_all(&output_dir) {
        eprintln!("  {}  {}", style("✗").red().bold(), e);
        return;
    }

    println!();
    println!(
        "  {}  {}",
        style("▶").dim(),
        style(output_dir.display()).bold()
    );
    println!();
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let total = jobs.len();
    let mut saved = 0usize;

    for (n, job) in jobs.iter().enumerate() {
        let stage_label = match job.match_stage {
            1 => style("id   ").green(),
            2 => style("title").cyan(),
            _ => style("pos  ").yellow(),
        };
        let pb = make_spinner(format!(
            "[{}/{}]  {}  {}",
            n + 1,
            total,
            stage_label,
            style(&job.output_name).dim()
        ));
        let result = merge_video(job, &output_dir);
        pb.finish_and_clear();

        match result {
            Ok(path) => {
                saved += 1;
                println!(
                    "  {}  [{}]  {}",
                    style("✓").green().bold(),
                    stage_label,
                    style(path.file_name().unwrap().to_string_lossy()).dim()
                );
            }
            Err(e) => eprintln!(
                "  {}  {}  {}",
                style("✗").red().bold(),
                style(&job.output_name).dim(),
                style(e.to_string()).red()
            ),
        }
    }

    println!();
    print_summary(saved, total);
}

// ── merge single ──────────────────────────────────────────────────────────────

fn run_merge_single(video_path: &Path, sub_paths: &[PathBuf], output_dir: &Option<PathBuf>) {
    if !video_path.is_file() {
        eprintln!(
            "  {}  Video file not found: {}",
            style("✗").red().bold(),
            video_path.display()
        );
        return;
    }
    for sub in sub_paths {
        if !sub.is_file() {
            eprintln!(
                "  {}  Subtitle file not found: {}",
                style("✗").red().bold(),
                sub.display()
            );
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
    println!(
        "  {}  {}",
        style("▶").dim(),
        style(video_path.file_name().unwrap().to_string_lossy()).bold()
    );
    println!("  {}", style("─".repeat(44)).dim());
    println!();

    let pb = make_spinner("Merging…".to_string());
    let result = merge_single(video_path, sub_paths, output_dir.as_deref());
    pb.finish_and_clear();

    match result {
        Ok(output) => {
            println!(
                "  {}  {}",
                style("✓").green().bold(),
                style(output.display()).dim()
            );
            println!();
            print_summary(1, 1);
        }
        Err(e) => eprintln!(
            "  {}  {}",
            style("✗").red().bold(),
            style(e.to_string()).red()
        ),
    }
}
