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

use super::types::SubCue;

// ── timestamp helpers ─────────────────────────────────────────────────────────

fn ms_to_vtt_timestamp(ms: i64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{h:02}:{m:02}:{s:02}.{millis:03}")
}

fn ms_to_srt_timestamp(ms: i64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{h:02}:{m:02}:{s:02},{millis:03}")
}

// ── RTL detection ─────────────────────────────────────────────────────────────

pub fn is_rtl_lang(lang: &str) -> bool {
    let l = lang.to_lowercase();
    l.starts_with("ar") || l.starts_with("he") || l.starts_with("fa") || l.starts_with("ur")
}

// ── writers ───────────────────────────────────────────────────────────────────

pub fn write_vtt(cues: &[SubCue], lang: &str) -> String {
    let mut out = String::from("WEBVTT\n");
    if is_rtl_lang(lang) {
        out.push_str("\nSTYLE\n::cue {\n  direction: rtl;\n  unicode-bidi: embed;\n}\n");
    }
    out.push('\n');
    for cue in cues {
        out.push_str(&cue.index.to_string());
        out.push('\n');
        out.push_str(&ms_to_vtt_timestamp(cue.start_ms));
        out.push_str(" --> ");
        out.push_str(&ms_to_vtt_timestamp(cue.end_ms));
        out.push('\n');
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }
    out
}

pub fn write_srt(cues: &[SubCue]) -> String {
    let mut out = String::new();
    for cue in cues {
        out.push_str(&cue.index.to_string());
        out.push('\n');
        out.push_str(&ms_to_srt_timestamp(cue.start_ms));
        out.push_str(" --> ");
        out.push_str(&ms_to_srt_timestamp(cue.end_ms));
        out.push('\n');
        out.push_str(&cue.text);
        out.push_str("\n\n");
    }
    out
}
