//! Native Rust port of the unified `tcl` toolchain CLI.
//!
//! This crate owns the `clap` command tree that mirrors the Python
//! `tooling/tcl/main.py` verb registry, plus the dispatch that drives each
//! verb into the underlying Rust engine crates (`tcl-compiler`,
//! `tcl-lsp-core`, `tcl-registry`, and the new `tcl-cli-*` support crates).
//!
//! The command surface (verb names, aliases, flags) is the parity contract
//! with the Python CLI: `tcl --help` and every `tcl <verb> --help` are diffed
//! against the argparse output during the port. Verb *behaviour* is filled in
//! phase by phase; until a verb is ported its handler returns a clear
//! "not yet implemented" error (exit code 2), matching the Python error path.

#![forbid(unsafe_code)]

mod cli;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

/// Parse `args` and run the selected verb, returning the process exit code.
///
/// `clap` handles `--help`/`--version`/parse errors by printing and exiting
/// directly, so this only returns for a successfully parsed command.
#[must_use]
pub fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    match dispatch(&cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

/// Dispatch a parsed command to its handler.
///
/// Returns the intended process exit code (0 = success, 1 = semantic failure,
/// 2 = usage/internal error) so the binary can forward it verbatim.
fn dispatch(command: &Command) -> anyhow::Result<u8> {
    // Phase 0 scaffolding: the command tree is complete and diffable against
    // the Python CLI, but verb engines are wired up in later phases. Each arm
    // will be replaced with a real handler as that verb is ported.
    let verb = command.verb_name();
    anyhow::bail!("`tcl {verb}` is not yet implemented in the Rust port");
}
