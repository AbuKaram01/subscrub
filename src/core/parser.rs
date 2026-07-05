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

use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1252};
use regex::Regex;
use serde_json::Value;
use std::fs;

use super::types::{Event, SubCue};

// ── decoding ──────────────────────────────────────────────────────────────────

pub fn try_decode(bytes: &[u8]) -> String {
    let stripped = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    if let Ok(s) = std::str::from_utf8(stripped) {
        return s.to_string();
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (decoded, _, had_errors) = UTF_16LE.decode(&bytes[2..]);
        if !had_errors {
            return decoded.into_owned();
        }
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (decoded, _, had_errors) = UTF_16BE.decode(&bytes[2..]);
        if !had_errors {
            return decoded.into_owned();
        }
    }
    let (decoded, _, _) = WINDOWS_1252.decode(bytes);
    decoded.into_owned()
}

// ── json3 parsing ─────────────────────────────────────────────────────────────

pub fn parse_json3(bytes: &[u8]) -> Vec<Event> {
    let content = try_decode(bytes);
    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  [warn] json3 parse error: {e}");
            return Vec::new();
        }
    };

    let events = match json.get("events").and_then(|v| v.as_array()) {
        Some(e) => e,
        None => return Vec::new(),
    };

    let mut out: Vec<Event> = Vec::new();

    for event in events {
        let t_start = event.get("tStartMs").and_then(|v| v.as_i64()).unwrap_or(0);

        let duration_ms = event.get("dDurationMs").and_then(|v| v.as_i64());

        let segs = match event.get("segs").and_then(|v| v.as_array()) {
            Some(s) => s,
            None => continue,
        };

        let mut event_words: Vec<(String, i64)> = Vec::new();

        for seg in segs {
            let text = match seg.get("utf8").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => continue,
            };
            if text.trim().is_empty() {
                continue;
            }

            let offset = seg.get("tOffsetMs").and_then(|v| v.as_i64()).unwrap_or(0);

            let tokens: Vec<&str> = text.split_whitespace().collect();
            let n = tokens.len() as i64;
            for (j, token) in tokens.iter().enumerate() {
                let spread = if n > 1 { (j as i64 * 300) / n } else { 0 };
                event_words.push((token.to_string(), t_start + offset + spread));
            }
        }

        if event_words.is_empty() {
            continue;
        }

        let text = event_words
            .iter()
            .map(|(t, _)| t.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let start_ms = event_words[0].1;
        let end_ms = duration_ms.map(|d| t_start + d);

        out.push(Event {
            start_ms,
            end_ms,
            text,
        });
    }

    out
}

// ── cleaning ──────────────────────────────────────────────────────────────────

fn clean_word(word: &str) -> Option<String> {
    use std::sync::OnceLock;
    static RE_HTML: OnceLock<Regex> = OnceLock::new();

    let re_html = RE_HTML.get_or_init(|| Regex::new(r"<[^>]+>").unwrap());

    let t = re_html.replace_all(word, "");
    let t = t.trim().to_string();

    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

fn ends_sentence(word: &str) -> bool {
    let last = word.trim_end().chars().last().unwrap_or(' ');
    matches!(last, '.' | '؟' | '!' | '…' | '?')
}

fn wrap_text(text: &str, _max_chars: usize) -> String {
    text.to_string()
}

// ── cue grouping ──────────────────────────────────────────────────────────────

pub fn group_into_cues(events: Vec<Event>) -> Vec<SubCue> {
    const MAX_WORDS: usize = 15;
    const MAX_DUR: i64 = 6_000;
    const MAX_CUE_DUR: i64 = 8_000;

    let mut cues: Vec<SubCue> = Vec::new();

    let mut buf_words: Vec<String> = Vec::new();
    let mut buf_start: i64 = 0;
    let mut buf_last_ms: i64 = 0;

    let flush = |buf_words: &mut Vec<String>,
                 buf_start: &mut i64,
                 buf_last_ms: &mut i64,
                 next_start_ms: i64,
                 cues: &mut Vec<SubCue>| {
        if buf_words.is_empty() {
            return;
        }
        let text = wrap_text(&buf_words.join(" "), 42);
        let end_ms = next_start_ms
            .saturating_sub(10)
            .min(*buf_start + MAX_CUE_DUR);
        cues.push(SubCue {
            index: 0,
            start_ms: *buf_start,
            end_ms,
            text,
        });
        buf_words.clear();
        *buf_last_ms = next_start_ms;
    };

    for (i, event) in events.iter().enumerate() {
        if let Some(_trusted_dur) = event.end_ms {
            flush(
                &mut buf_words,
                &mut buf_start,
                &mut buf_last_ms,
                event.start_ms,
                &mut cues,
            );

            let clean_tokens: Vec<String> = event
                .text
                .split_whitespace()
                .filter_map(clean_word)
                .collect();
            if clean_tokens.is_empty() {
                continue;
            }

            let next_start = events
                .get(i + 1)
                .map(|e| e.start_ms)
                .unwrap_or(event.start_ms + 1_500);
            let end_ms = next_start
                .saturating_sub(10)
                .min(event.start_ms + MAX_CUE_DUR);

            let text = wrap_text(&clean_tokens.join(" "), 42);
            cues.push(SubCue {
                index: 0,
                start_ms: event.start_ms,
                end_ms,
                text,
            });
            buf_last_ms = end_ms;
            continue;
        }

        for token in event.text.split_whitespace() {
            if let Some(clean) = clean_word(token) {
                if buf_words.is_empty() {
                    buf_start = event.start_ms;
                }
                buf_words.push(clean.clone());
                buf_last_ms = event.start_ms;

                let dur = event.start_ms - buf_start;
                let at_sentence = ends_sentence(&clean);
                let at_limit = buf_words.len() >= MAX_WORDS || dur >= MAX_DUR;

                if at_sentence || at_limit {
                    let next_ms = events
                        .get(i + 1)
                        .map(|e| e.start_ms)
                        .unwrap_or(buf_last_ms + 1_500);
                    flush(
                        &mut buf_words,
                        &mut buf_start,
                        &mut buf_last_ms,
                        next_ms,
                        &mut cues,
                    );
                }
            }
        }
    }

    if !buf_words.is_empty() {
        let end_ms = (buf_last_ms + 1_500).min(buf_start + MAX_CUE_DUR);
        cues.push(SubCue {
            index: 0,
            start_ms: buf_start,
            end_ms,
            text: wrap_text(&buf_words.join(" "), 42),
        });
    }

    for (i, cue) in cues.iter_mut().enumerate() {
        cue.index = i + 1;
    }

    cues
}

// ── public entry point ────────────────────────────────────────────────────────

pub fn process_json3(input_path: &str) -> Result<Vec<SubCue>, Box<dyn std::error::Error>> {
    let bytes = fs::read(input_path)?;
    let events = parse_json3(&bytes);
    if events.is_empty() {
        return Err("No events found in json3 file.".into());
    }
    Ok(group_into_cues(events))
}
