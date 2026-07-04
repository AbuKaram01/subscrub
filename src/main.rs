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
mod commands;

use clap::Parser;

use cli::ui::{ask_task, print_banner};
use cli::{Cli, Commands, DownloadArgs, Task};

fn main() {
    let cli = Cli::parse();
    print_banner();

    match cli.command {
        Some(Commands::Download(args)) => commands::download::run(args),
        Some(Commands::Languages(args)) => commands::download::run_list_languages(args),
        Some(Commands::Merge { mode }) => commands::merge::run(mode),
        None => match ask_task() {
            Task::Download => commands::download::run(DownloadArgs::default()),
            Task::Merge => commands::merge::run(None),
        },
    }
}
