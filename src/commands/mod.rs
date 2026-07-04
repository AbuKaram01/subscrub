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

//! Orchestration layer: wires `cli` (input/prompts) to `core` (domain logic)
//! for each top-level command. This is the only layer allowed to call
//! `std::process::exit` — `core` reports problems via `Result`, `cli`
//! collects input, and `commands` decides what a failure means for the run.

pub mod download;
pub mod merge;

use console::style;

/// Prints a formatted error line and exits with status 1. The single place
/// that turns a `Result::Err` from validation or a dependency check into
/// the CLI's standard error format.
pub fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("\n  {}  {}\n", style("✗").red().bold(), msg);
    std::process::exit(1);
}
