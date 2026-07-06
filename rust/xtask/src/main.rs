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

//! `cargo xtask` — the native task runner for the workspace's build and check
//! tasks.
//!
//! Each subcommand is kept byte-compatible with the `scripts/` tool it
//! replaces so the Makefile / CI can switch over incrementally; the legacy
//! script stays as the fallback for one release cycle, then retires.
//!
//! Run a task with `cargo xtask <command>` (the workspace `.cargo/config.toml`
//! aliases `xtask` to `run --package xtask --`).
//!
//! Subcommands:
//!
//! - `kcs-index-links` — validate markdown links + KCS/design index coverage
//!   under `docs/`.
//! - `version` — print the setuptools-scm / hatch-vcs project version from
//!   `git describe`.
//! - `tzdata-bundle` — pack the curated tzdata `TZBL` bundle for the WASM
//!   runtime.
//! - `audit-option-dialects` — probe `OptionSpec` dialect gates against real
//!   tclsh 8.4/8.5/8.6/9.0.
//! - `diag-tables` — generate the `docs/generated/` code tables from the
//!   `DiagCode` catalogue (`--check` to verify instead of write).
//! - `gen-editor-catalogs` — generate the Zed/VS Code command & iRules-event
//!   catalog JSON from the registry (`--check` to verify instead of write).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod audit_option_dialects;
mod diag_tables;
mod gen_ai;
mod gen_editor_catalogs;
mod gen_editor_settings;
mod gen_jetbrains;
mod gen_vscode_package;
mod kcs_index_links;
mod tzdata_bundle;
mod util;
mod version;

/// Native build/check tasks for the tcl-lsp workspace.
#[derive(Parser)]
#[command(name = "xtask", about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate markdown links + KCS/design index coverage under `docs/`.
    KcsIndexLinks,

    /// Print the project version (`git describe` → setuptools-scm scheme).
    Version,

    /// Pack the curated tzdata `TZBL` bundle for the WASM runtime.
    TzdataBundle {
        /// Host zoneinfo directory to read `TZif` files from.
        #[arg(long, default_value = "/usr/share/zoneinfo")]
        zoneinfo: PathBuf,
        /// Path to write the packed bundle (e.g. `runtime/rust/data/tzdata.bin`).
        #[arg(long)]
        output: PathBuf,
        /// Drop `TZif` transitions strictly before this Unix epoch second.
        #[arg(long, value_name = "EPOCH")]
        trim_from: Option<i64>,
        /// Drop `TZif` transitions strictly after this Unix epoch second.
        #[arg(long, value_name = "EPOCH")]
        trim_to: Option<i64>,
    },

    /// Probe `OptionSpec` dialect gates against real tclsh 8.4/8.5/8.6/9.0.
    AuditOptionDialects,

    /// Generate the `docs/generated/` code tables from the `DiagCode` catalogue.
    DiagTables {
        /// Verify the committed tables are in sync instead of rewriting them;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate the Zed/VS Code editor catalog JSON from the command registry.
    GenEditorCatalogs {
        /// Verify the committed catalogs are in sync instead of rewriting them;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate the VS Code diagnostic catalogue (`diagnosticCatalog.ts`).
    GenEditorSettings {
        /// Verify the committed catalogue is in sync instead of rewriting it;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Regenerate the `tclLsp.*` sections of the VS Code `package.json`.
    GenVscodePackage {
        /// Verify instead of rewriting; exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate the `JetBrains` `DiagnosticCatalog.kt` from the `DiagCode` catalogue.
    GenJetbrainsCatalog {
        /// Verify instead of rewriting; exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate `ai/shared/diagnostics.json` from the `DiagCode` catalogue.
    GenAiDiagnostics {
        /// Verify instead of rewriting; exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> anyhow::Result<ExitCode> {
    match Cli::parse().command {
        Command::KcsIndexLinks => kcs_index_links::run(),
        Command::Version => Ok(version::run()),
        Command::TzdataBundle {
            zoneinfo,
            output,
            trim_from,
            trim_to,
        } => tzdata_bundle::run(&zoneinfo, &output, trim_from, trim_to),
        Command::AuditOptionDialects => audit_option_dialects::run(),
        Command::DiagTables { check } => diag_tables::run(check),
        Command::GenEditorCatalogs { check } => gen_editor_catalogs::run(check),
        Command::GenEditorSettings { check } => gen_editor_settings::run(check),
        Command::GenVscodePackage { check } => gen_vscode_package::run(check),
        Command::GenJetbrainsCatalog { check } => gen_jetbrains::run(check),
        Command::GenAiDiagnostics { check } => gen_ai::run(check),
    }
}
