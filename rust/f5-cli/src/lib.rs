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
mod commands;

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
    match command {
        Command::Diff {
            before,
            after,
            json,
            output,
        } => commands::diff::run_diff(before, after, *json, output.as_deref()),
        Command::Explain {
            kind,
            target,
            inputs,
            json,
            output,
        } => commands::explain::run_explain(kind, target, inputs, *json, output.as_deref()),
        Command::Split {
            input,
            output,
            format,
        } => commands::split::run_split(input, output, format),
        Command::Merge {
            paths,
            format,
            output,
        } => commands::merge::run_merge(paths, format, output.as_deref()),
        Command::Extract {
            ucs,
            include_extras,
            format,
            passphrase,
            output,
        } => commands::extract::run_extract(
            ucs,
            *include_extras,
            format,
            passphrase,
            output.as_deref(),
        ),
        Command::Stats {
            inputs,
            top,
            json,
            output,
            passphrase,
        } => commands::stats::run_stats(inputs, *top, *json, output.as_deref(), passphrase),
        Command::Cleanup {
            inputs,
            keep,
            no_keep_common,
            json,
            output,
            passphrase,
        } => commands::cleanup::run_cleanup(
            inputs,
            keep,
            *no_keep_common,
            *json,
            output.as_deref(),
            passphrase,
        ),
        Command::Graph {
            inputs,
            format,
            seed,
            reverse,
            max_depth,
            output,
            passphrase,
        } => commands::graph::run_graph(
            inputs,
            format,
            seed,
            *reverse,
            *max_depth,
            output.as_deref(),
            passphrase,
        ),
        cmd @ Command::Query { .. } => dispatch_query(cmd),
        Command::Completion { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "f5-query", &mut std::io::stdout());
            Ok(0)
        }
        // Verbs not yet ported (the BIG-IP engines land in later phases) fall
        // through to a clear not-implemented error.
        other => {
            let verb = other.verb_name();
            anyhow::bail!("`f5 {verb}` is not yet implemented in the Rust port");
        }
    }
}

/// Route `Command::Query` to the read-only query verb, mapping the output-mode
/// flags and rejecting the deferred help actions.
fn dispatch_query(command: &Command) -> anyhow::Result<u8> {
    let Command::Query {
        expression,
        inputs,
        name,
        input_json,
        input_jsonl,
        input_csv,
        input_f5log,
        input,
        merge,
        write,
        in_place,
        scf,
        raw,
        paths_only,
        json,
        table,
        table_lineart,
        strict,
        render_name,
        render_opt,
        help_dsl,
        help_builtins,
        help_examples,
        help_renderers,
        help_inputs,
        ..
    } = command
    else {
        unreachable!("dispatch_query is only called with Command::Query");
    };

    // `--help-renderers` is fully ported (the registry catalogue is static).
    if *help_renderers {
        print!("{}", commands::query::help_renderers_text());
        return Ok(0);
    }

    // `--help-inputs` is fully ported (the built-in format catalogue is
    // static — no user-plugin scan in the Rust port).
    if *help_inputs {
        print!("{}", commands::query::help_inputs_text());
        return Ok(0);
    }

    // The remaining help actions are deferred in the read-only port.
    if *help_dsl || help_builtins.is_some() || *help_examples {
        anyhow::bail!("`f5 query` help actions are not yet ported in the Rust port");
    }

    // `--render NAME` re-uses the same output dispatch path as the built-in
    // modes: `output::render` falls through to the renderer registry on an
    // unknown mode, so we swap the output mode for the requested name and
    // thread the parsed `--render-opt` map through. Mirrors `_run_query`.
    let mode = if let Some(name) = render_name {
        name.clone()
    } else {
        commands::query::OutputModeFlags {
            scf: *scf,
            raw: *raw,
            paths_only: *paths_only,
            json: *json,
            table: *table,
            table_lineart: *table_lineart,
        }
        .resolve()
        .to_owned()
    };

    let render_opts = if render_name.is_some() {
        match commands::query::parse_render_opts(render_opt) {
            Ok(opts) => opts,
            Err(e) => {
                eprintln!("error: {e}");
                return Ok(2);
            }
        }
    } else {
        std::collections::BTreeMap::new()
    };

    commands::query::run_query_verb(
        expression.as_deref(),
        inputs,
        name,
        &commands::query::InputArgs {
            input_json,
            input_jsonl,
            input_csv,
            input_f5log,
            input,
        },
        &mode,
        &render_opts,
        commands::query::QueryFlags {
            merge: *merge,
            write: *write,
            in_place: *in_place,
            strict: *strict,
        },
    )
}
