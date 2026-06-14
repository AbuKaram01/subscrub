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

use std::path::PathBuf;

/// A raw timed segment parsed directly from the json3 source.
#[derive(Debug, Clone)]
pub struct Event {
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub text: String,
}

/// A cleaned, timed subtitle cue ready to be written to any format.
#[derive(Debug, Clone)]
pub struct SubCue {
    pub index: usize,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

/// Output subtitle format.
#[derive(Debug, Clone, PartialEq)]
pub enum SubFormat {
    Vtt,
    Srt,
}

/// Source subtitle type on YouTube.
#[derive(Debug, Clone, PartialEq)]
pub enum SubType {
    Manual,
    Auto,
}

/// A single video entry inside a playlist.
#[derive(Debug, Clone)]
pub struct PlaylistVideo {
    pub url: String,
    pub title: String,
}

/// A YouTube playlist with its title and video list.
#[derive(Debug, Clone)]
pub struct Playlist {
    pub title: String,
    pub videos: Vec<PlaylistVideo>,
}

/// A single subtitle file paired with its language code.
#[derive(Debug, Clone)]
pub struct SubEntry {
    pub path: PathBuf,
    pub lang: String,
}

/// A video file matched with its subtitle(s), ready to be merged.
#[derive(Debug, Clone)]
pub struct MergeJob {
    pub video_path: PathBuf,
    pub subs: Vec<SubEntry>,
    pub output_name: String,
    /// 1 = ID match, 2 = title match, 3 = position match (fallback)
    pub match_stage: u8,
}
