//! `cargo xtask` — the native task runner that replaces the Python
//! `scripts/` toolchain (rewrite track **API-PYO3**, the `scripts`→`xtask`
//! migration).
//!
//! The end goal of the rewrite is **zero** Python executed by any in-repo
//! entry point, build and release scripts included. This crate is where
//! those scripts land as Rust subcommands, ported one at a time and kept
//! byte-compatible with the Python they replace so the Makefile / CI can
//! switch over incrementally (the Python script stays as the fallback for
//! one release cycle, per the rollout rule, then retires).
//!
//! Run a task with `cargo xtask <command>` (the workspace `.cargo/config.toml`
//! aliases `xtask` to `run --package xtask --`).
//!
//! Ported so far:
//!
//! - `refcount-contract` — port of `scripts/check/refcount_contract.py`: lint
//!   that every `pub export fn` in `runtime/zig/` has a row in the refcount
//!   contract doc.
//! - `kcs-index-links` — port of `scripts/check/kcs_index_links.py`: validate
//!   markdown links + KCS/design index coverage under `docs/`.
//! - `version` — port of `scripts/print_version.py`: print the
//!   setuptools-scm / hatch-vcs project version from `git describe`.
//! - `tzdata-bundle` — port of `scripts/build/tzdata_bundle.py`: pack the
//!   curated tzdata `TZBL` bundle for the WASM runtime.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
    ///
    /// Port of `scripts/check/refcount_contract.py`.
    RefcountContract {
        /// Exit non-zero on any missing/extra row (default: warning only).
        #[arg(long)]
        strict: bool,
    },

    /// Validate markdown links + KCS/design index coverage under `docs/`.
    ///
    /// Port of `scripts/check/kcs_index_links.py`.
    KcsIndexLinks,

    /// Print the project version (`git describe` → setuptools-scm scheme).
    ///
    /// Port of `scripts/print_version.py`.
    Version,

    /// Pack the curated tzdata `TZBL` bundle for the WASM runtime.
    ///
    /// Port of `scripts/build/tzdata_bundle.py`.
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
    }
}
