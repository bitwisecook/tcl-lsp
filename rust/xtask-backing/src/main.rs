// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `cargo xtask-backing` — the `command-backing` gate, kept out of `xtask` so
//! the other gates build no engine (the workspace `.cargo/config.toml`
//! aliases `xtask-backing` to `run --package xtask-backing --`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod command_backing;

#[derive(Parser)]
#[command(name = "xtask-backing", about = "The WASM command-backing gate")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check the WASM runtime backs every core-Tcl registry command;
    /// regenerate `docs/generated/wasm-command-backing.md`.
    #[command(name = "command-backing")]
    WasmBacking {
        /// Verify backing + report are in sync instead of rewriting; exit
        /// non-zero on a gap, a stale classification, or report drift.
        #[arg(long)]
        check: bool,
    },
}

/// The repository root: this crate lives at `<root>/rust/xtask-backing`.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("xtask-backing crate lives at <root>/rust/xtask-backing")
        .to_path_buf()
}

fn main() -> anyhow::Result<ExitCode> {
    tcl_registry::install_shipped().expect("the shipped surface set installs");
    match Cli::parse().command {
        Command::WasmBacking { check } => command_backing::run(check),
    }
}
