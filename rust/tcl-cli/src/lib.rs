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
mod commands;
#[cfg(feature = "tui")]
mod tui;

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
            tcl_cli_support::chrome::eprint_error(format!("{err:#}"));
            ExitCode::from(2)
        }
    }
}

/// Dispatch a parsed command to its handler.
///
/// Returns the intended process exit code (0 = success, 1 = semantic failure,
/// 2 = usage/internal error) so the binary can forward it verbatim.
#[allow(clippy::too_many_lines)] // one match arm per verb; grows with the verb set
fn dispatch(command: &Command) -> anyhow::Result<u8> {
    match command {
        Command::Diag { input, diag } | Command::Lint { input, diag } => {
            commands::diag::run_diag(input, diag)
        }
        Command::Validate { input, diag } => commands::diag::run_validate(input, diag),
        Command::Diagram { input, json } => commands::diagram::run_diagram(input, *json),
        Command::Symbols { input, json } => commands::graphs::run_symbols(input, *json),
        Command::Symbolgraph { input, json } => commands::graphs::run_symbolgraph(input, *json),
        Command::Callgraph { input, json } => commands::graphs::run_callgraph(input, *json),
        Command::Dataflow { input, json } => commands::graphs::run_dataflow(input, *json),
        Command::Diff {
            left,
            right,
            left_source,
            right_source,
            dialect,
            show,
            json,
            output,
        } => commands::diff::run_diff(
            left.as_deref(),
            right.as_deref(),
            left_source.as_deref(),
            right_source.as_deref(),
            dialect,
            show,
            *json,
            output.as_deref(),
        ),
        Command::CmdInfo {
            command,
            dialect,
            json,
            output,
        } => commands::lookup::run_command_info(command, dialect, *json, output.as_deref()),
        Command::Highlight {
            input,
            format,
            colour,
        } => commands::highlight::run_highlight(input, format, colour),
        Command::Completion { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "tcl", &mut std::io::stdout());
            Ok(0)
        }
        Command::Opt {
            input,
            profile,
            disable,
            enable,
            colour,
        } => commands::transform::run_opt(input, profile, disable, enable, colour),
        Command::Format {
            input,
            indent_size,
            indent_style,
            max_line_length,
            colour,
        } => commands::transform::run_format(
            input,
            *indent_size,
            indent_style.as_deref(),
            *max_line_length,
            colour,
        ),
        Command::Minify {
            input,
            compact,
            symbol_map,
            aggressive,
            isolated,
            colour,
        } => commands::transform::run_minify(
            input,
            *compact,
            symbol_map.as_deref(),
            *aggressive,
            *isolated,
            colour,
        ),
        Command::RegistryDump {
            dialect,
            all_dialects,
            json: _,
            output,
        } => commands::registry::run_registry_dump(dialect, *all_dialects, output.as_deref()),
        Command::UnminifyError {
            symbol_map,
            error,
            error_file,
            minified,
            original,
            output,
        } => commands::transform::run_unminify_error(
            symbol_map,
            error.as_deref(),
            error_file.as_deref(),
            minified.as_deref(),
            original.as_deref(),
            output.as_deref(),
        ),
        Command::Explore {
            input,
            show,
            json,
            text,
            tui,
            colour,
        } => commands::explore::run_explore(input, show, *json, *text, *tui, colour),
        Command::Help {
            query,
            dialect,
            limit,
            json,
            output,
        } => commands::help::run_help(query, dialect, *limit, *json, output.as_deref()),
        Command::FindLegacy { input, json } => commands::misc::run_find_legacy(input, *json),
        // Verbs not yet ported fall through to a clear not-implemented error.
        other => {
            let verb = other.verb_name();
            anyhow::bail!("`tcl {verb}` is not yet implemented in the Rust port");
        }
    }
}
