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
//! - `refcount-contract` — lint that every `pub export fn` in `runtime/zig/`
//!   has a row in the refcount contract doc.
//! - `kcs-index-links` — validate markdown links + KCS/design index coverage
//!   under `docs/`.
//! - `version` — print the setuptools-scm / hatch-vcs project version from
//!   `git describe`.
//! - `tzdata-bundle` — pack the curated tzdata `TZBL` bundle for the WASM
//!   runtime.
//! - `audit-option-dialects` — probe `OptionSpec` dialect gates against real
//!   tclsh 8.4/8.5/8.6/9.0.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod audit_option_dialects;
mod kcs_index_links;
mod refcount_contract;
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
    /// Lint that every `runtime/zig/` export has a refcount-contract row.
    RefcountContract {
        /// Exit non-zero on any missing/extra row (default: warning only).
        #[arg(long)]
        strict: bool,
    },

    /// Validate markdown links + KCS/design index coverage under `docs/`.
    KcsIndexLinks,

    /// Print the project version (`git describe` → setuptools-scm scheme).
    Version,

    /// Pack the curated tzdata `TZBL` bundle for the WASM runtime.
    TzdataBundle {
        /// Host zoneinfo directory to read `TZif` files from.
        #[arg(long, default_value = "/usr/share/zoneinfo")]
        zoneinfo: PathBuf,
        /// Path to write the packed bundle (e.g. `runtime/zig/data/tzdata.bin`).
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
}

fn main() -> anyhow::Result<ExitCode> {
    match Cli::parse().command {
        Command::RefcountContract { strict } => refcount_contract::run(strict),
        Command::KcsIndexLinks => kcs_index_links::run(),
        Command::Version => Ok(version::run()),
        Command::TzdataBundle {
            zoneinfo,
            output,
            trim_from,
            trim_to,
        } => tzdata_bundle::run(&zoneinfo, &output, trim_from, trim_to),
        Command::AuditOptionDialects => audit_option_dialects::run(),
    }
}
