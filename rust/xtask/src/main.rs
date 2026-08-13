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
//! - `number-drift` — flag hand-rolled Tcl radix-prefix recognition outside
//!   the one numeral parser (`tcl_syntax::number`).

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod audit_option_dialects;
mod command_backing;
mod diag_emission;
mod diag_tables;
mod fp_sweep;
mod gen_ai;
mod gen_editor_catalogs;
mod gen_editor_settings;
mod gen_jetbrains;
mod gen_tmlanguage_keywords;
mod gen_vscode_package;
mod gen_zed_queries;
mod kcs_index_links;
mod number_drift;
mod registry_oracle;
mod resolution_drift;
mod tcltest_sweep;
mod tzdata_bundle;
mod util;
mod version;
mod workflow_sync;

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

    /// Check the WASM runtime backs every core-Tcl registry command;
    /// regenerate `docs/generated/wasm-command-backing.md`.
    #[command(name = "command-backing")]
    WasmBacking {
        /// Verify backing + report are in sync instead of rewriting; exit
        /// non-zero on a gap, a stale classification, or report drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate the `docs/generated/` code tables from the `DiagCode` catalogue.
    DiagTables {
        /// Verify the committed tables are in sync instead of rewriting them;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Verify every non-internal, non-reserved `DiagCode` has at least one
    /// real construction site under `rust/tcl-compiler/src` (issue #1317).
    #[command(name = "diag-emission-check")]
    DiagEmissionCheck,

    /// Generate the Zed/VS Code editor catalog JSON from the command registry.
    GenEditorCatalogs {
        /// Verify the committed catalogs are in sync instead of rewriting them;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate the Zed tree-sitter highlight query command lists from the
    /// command registry (`editors/zed/languages/tcl/highlights.scm`).
    GenZedQueries {
        /// Verify the committed query is in sync instead of rewriting it;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Generate the VS Code / `JetBrains` / Sublime Text `TextMate` grammars'
    /// command-name keyword lists from the command registry.
    GenTmlanguageKeywords {
        /// Verify the committed grammars are in sync instead of rewriting
        /// them; exit non-zero on drift.
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

    /// Verify the installed `.github/workflows/` copies still match their
    /// canonical sources under `rust/bigip-report-gen/python/deploy/`.
    WorkflowSync {
        /// Verify instead of reinstalling; exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Flag hand-rolled Tcl radix-prefix recognition outside the one numeral
    /// parser (`tcl_syntax::number`) — the numeric-grammar drift class.
    NumberDrift {
        /// Accepted for symmetry with the other gates (the lint always
        /// verifies; it never rewrites).
        #[arg(long)]
        check: bool,
    },

    /// Flag namespace-blind `.name ==` scans over `all_procs`/`all_classes`
    /// outside the shared resolution contract (the M1 drift class).
    ResolutionDrift {
        /// Accepted for symmetry with the other gates (the lint always
        /// verifies; it never rewrites).
        #[arg(long)]
        check: bool,
    },

    /// Compare the iRules registry with a local BIG-IP schema/man-page
    /// extract; exact source omissions fail, newer registry entries are
    /// reported separately.
    #[command(name = "registry-oracle")]
    RegistryOracle {
        /// BIG-IP extract root containing `irule-schema-split/` and `man-*`.
        #[arg(long)]
        irules_root: PathBuf,
        /// Write or verify a deterministic Markdown report.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Verify an existing report instead of writing it.
        #[arg(long)]
        check: bool,
    },

    /// Run the C tcltest suite through the VM + reference tclsh and regenerate the
    /// VM-vs-C parity scoreboard (`docs/design/runtime/rust-vm-tier-parity.md`).
    TcltestSweep {
        /// Which backend(s) to run: `vm` | `tclsh` | `both`.
        #[arg(long, default_value = "both")]
        backend: String,
        /// Sweep only this single stem and print its result (does not rewrite the
        /// committed scoreboard).
        #[arg(long)]
        stem: Option<String>,
        /// Per-file timeout in seconds (default 120).
        #[arg(long)]
        timeout: Option<u64>,
        /// Verify the committed scoreboard is in sync instead of rewriting it;
        /// exit non-zero on drift.
        #[arg(long)]
        check: bool,
    },

    /// Dump every firing of one or more diagnostic/optimisation codes across
    /// a corpus, dialect-aware, grouped by message shape — the false-positive
    /// audit harness (issue #1316; `docs/design/compiler/fp-sweep.md`).
    FpSweep {
        /// Diagnostic/optimisation code to sweep (repeatable, e.g. `--code
        /// W111 --code W112`).
        #[arg(long = "code", required = true)]
        codes: Vec<String>,
        /// Corpus directory or file to sweep (repeatable). A directory is
        /// walked recursively for Tcl/iRules source files, direct `.txt`
        /// iRules, and iRules code blocks embedded in `.rst` documents.
        #[arg(long = "corpus", required = true)]
        corpus: Vec<PathBuf>,
        /// Sample locations printed per message shape.
        #[arg(long, default_value_t = 3)]
        examples: usize,
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
        Command::WasmBacking { check } => command_backing::run(check),
        Command::DiagTables { check } => diag_tables::run(check),
        Command::DiagEmissionCheck => Ok(diag_emission::run()),
        Command::GenEditorCatalogs { check } => gen_editor_catalogs::run(check),
        Command::GenZedQueries { check } => gen_zed_queries::run(check),
        Command::GenTmlanguageKeywords { check } => gen_tmlanguage_keywords::run(check),
        Command::GenEditorSettings { check } => gen_editor_settings::run(check),
        Command::GenVscodePackage { check } => gen_vscode_package::run(check),
        Command::GenJetbrainsCatalog { check } => gen_jetbrains::run(check),
        Command::GenAiDiagnostics { check } => gen_ai::run(check),
        Command::WorkflowSync { check } => workflow_sync::run(check),
        Command::NumberDrift { check } => Ok(number_drift::run(check)),
        Command::ResolutionDrift { check } => Ok(resolution_drift::run(check)),
        Command::RegistryOracle {
            irules_root,
            output,
            check,
        } => registry_oracle::run(&irules_root, output.as_deref(), check),
        Command::TcltestSweep {
            backend,
            stem,
            timeout,
            check,
        } => tcltest_sweep::run(backend.parse()?, stem.as_deref(), timeout, check),
        Command::FpSweep {
            codes,
            corpus,
            examples,
        } => fp_sweep::run(&codes, &corpus, examples),
    }
}
