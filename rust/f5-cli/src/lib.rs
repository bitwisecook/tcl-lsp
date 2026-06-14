//! Native Rust port of the `f5-query` BIG-IP / iRules CLI.
//!
//! Owns the `clap` command tree mirroring `tooling/f5/main.py` + the
//! `tooling/f5/verbs/*` registry (and the `irule` verb group), plus dispatch
//! into the BIG-IP engine crates (`tcl-bigip`, `tcl-bigip-query`,
//! `tcl-bigip-remote`, `tcl-bigip-pcap`).
//!
//! As with `tcl-cli`, the command surface is the parity contract; verb engines
//! land phase by phase. Until a verb is ported its handler returns a clear
//! "not yet implemented" error (exit code 2).

#![forbid(unsafe_code)]

mod cli;

use std::ffi::OsString;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

/// Parse `args` and run the selected verb, returning the process exit code.
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
            tcl_cli_support::chrome::eprint_error(format!("{err:#}"));
            ExitCode::from(2)
        }
    }
}

fn dispatch(command: &Command) -> anyhow::Result<u8> {
    // Phase 0 scaffolding: see `tcl_cli::dispatch`.
    let verb = command.verb_name();
    anyhow::bail!("`f5 {verb}` is not yet implemented in the Rust port");
}
