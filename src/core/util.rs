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

//! Small, pure filesystem-naming helpers shared by the downloader and the
//! merger. Kept in one place so a change to the rules (e.g. which
//! characters are unsafe, or how collisions are resolved) only has to
//! happen once.

use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Removes filesystem-unsafe characters from a string for use as a filename.
/// Falls back to `"untitled"` if nothing usable remains.
pub fn clean_filename(s: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r#"[\\/*?:"<>|]"#).unwrap());
    let clean = re.replace_all(s.trim(), "").trim().to_string();
    if clean.is_empty() {
        "untitled".to_string()
    } else {
        clean
    }
}

/// Returns `path` unchanged if nothing exists there yet. Otherwise appends
/// " (1)", " (2)", … right before the extension until a free name is found,
/// so downloads and merges never silently overwrite an existing file.
pub fn unique_path(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|e| e.to_str());

    let mut n = 1u32;
    loop {
        let candidate_name = match ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_filename_strips_unsafe_characters() {
        assert_eq!(clean_filename("a/b\\c:d*e?f\"g<h>i|j"), "abcdefghij");
    }

    #[test]
    fn clean_filename_falls_back_when_empty() {
        assert_eq!(clean_filename("???"), "untitled");
    }

    #[test]
    fn unique_path_keeps_free_names_untouched() {
        let dir = std::env::temp_dir().join(format!("subscrub_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("clip.srt");
        assert_eq!(unique_path(&target), target);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn unique_path_increments_on_collision() {
        let dir =
            std::env::temp_dir().join(format!("subscrub_test_collision_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("clip.srt");
        std::fs::write(&target, b"first").unwrap();
        std::fs::write(dir.join("clip (1).srt"), b"second").unwrap();

        let resolved = unique_path(&target);
        assert_eq!(resolved, dir.join("clip (2).srt"));

        std::fs::remove_dir_all(&dir).ok();
    }
}
