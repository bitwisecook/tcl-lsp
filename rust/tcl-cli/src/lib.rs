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

//! The unified `tcl` toolchain CLI.
//!
//! This crate owns the `clap` command tree (the verb registry) plus the
//! dispatch that drives each verb into the underlying Rust engine crates
//! (`tcl-compiler`, `tcl-lsp-core`, `tcl-registry`, and the `tcl-cli-*`
//! support crates).
//!
//! The command surface (verb names, aliases, flags) defines the stable CLI
//! contract surfaced by `tcl --help` and every `tcl <verb> --help`. Every
//! top-level verb dispatches to a native engine handler, so the dispatch
//! `match` is exhaustive.

#![forbid(unsafe_code)]

mod cli;
mod commands;
#[cfg(feature = "tui")]
mod tui;

/// Structured KCS help lookup for embedders (the native MCP `help` tool).
pub use commands::help::help_json;

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
    match run_on_generous_stack(cli.command) {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            tcl_cli_support::chrome::eprint_error(format!("{err:#}"));
            ExitCode::from(2)
        }
    }
}

/// Stack budget for the dedicated worker thread every verb runs on.
///
/// Matches `WORKER_STACK_SIZE` in `tcl-lsp-server`/`tcl-mcp`'s `main.rs` —
/// see the doc comment there for the full rationale. Short version: the
/// analyser's and CFG builder's depth-capped recursions
/// (`tcl_compiler::analyser::commands::MAX_BODY_DEPTH` /
/// `tcl_compiler::cfg_builder::MAX_LOWER_DEPTH`, both 256) bound the number
/// of stack frames but not how much stack those frames need, and the
/// OS-provided main-thread stack this binary would otherwise inherit is
/// outside this crate's control — 8 MiB by default on Linux, but as little
/// as 1 MiB on Windows, or whatever a container's `ulimit -s` happens to
/// be. Running every verb on an explicitly-sized thread makes the CLI's
/// crash behaviour independent of the ambient environment (issue #996).
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Run `dispatch` on a dedicated thread with [`WORKER_STACK_SIZE`] of stack.
fn run_on_generous_stack(command: Command) -> anyhow::Result<u8> {
    std::thread::Builder::new()
        .stack_size(WORKER_STACK_SIZE)
        .spawn(move || dispatch(&command))
        .expect("failed to spawn the tcl CLI worker thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
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
        Command::Dis { input, optimise } => commands::compile::run_dis(input, *optimise),
        Command::Compwasm {
            input,
            backend,
            wat_output,
        } => commands::compile::run_compwasm(input, *backend, wat_output.as_deref()),
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
            serve,
            bind,
            port,
            open,
            colour,
        } => {
            if *serve {
                commands::gui::run_serve(bind, *port, *open)
            } else {
                commands::explore::run_explore(input, show, *json, *text, *tui, colour)
            }
        }
        Command::Help {
            query,
            dialect,
            limit,
            json,
            output,
        } => commands::help::run_help(query, dialect, *limit, *json, output.as_deref()),
        Command::FindLegacy { input, json } => commands::misc::run_find_legacy(input, *json),
        Command::Minimize {
            input,
            no_rename,
            json,
        } => commands::minimize::run_minimize(input, *no_rename, *json),
        Command::Pkg { action } => commands::pkg::run(action),
        Command::Venv { action } => commands::venv::run(action),
        Command::Docker { action } => commands::docker::run(action),
    }
}
