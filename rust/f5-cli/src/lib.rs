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

#[allow(clippy::too_many_lines)]
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
        Command::Grep {
            pattern,
            inputs,
            regex,
            cidr,
            direction,
            max_depth,
            recurse: _,
            no_recurse,
            max_nodes,
            full,
            json,
            output,
            format,
            passphrase,
        } => commands::grep::run_grep(
            pattern,
            inputs,
            *regex,
            *cidr,
            direction,
            *max_depth,
            !*no_recurse,
            *max_nodes,
            *full,
            *json,
            &format.format,
            output.as_deref(),
            passphrase,
        ),
        Command::Validate {
            inputs,
            category,
            severity,
            format,
            output,
        } => commands::validate::run_validate(
            inputs,
            category.as_deref(),
            severity.as_deref(),
            format,
            output.as_deref(),
        ),
        Command::Rename {
            old,
            new,
            path,
            output,
            in_place,
            write,
            format,
        } => commands::rename::run_rename(
            old,
            new,
            path,
            output.as_deref(),
            *in_place,
            *write,
            format,
        ),
        Command::Redact {
            path,
            keep_ips,
            target_cidr,
            shuffle,
            seed,
            remap_private,
            map_file,
            source_cidr,
            format,
            output,
        } => commands::redact::run_redact(
            path,
            *keep_ips,
            target_cidr,
            *shuffle,
            seed,
            *remap_private,
            map_file.as_deref(),
            source_cidr,
            format,
            output.as_deref(),
        ),
        Command::Unredact {
            map_file,
            path,
            format,
            output,
        } => commands::unredact::run_unredact(map_file, path, format, output.as_deref()),
        Command::Tmsh {
            path,
            output,
            modify,
            include,
        } => commands::tmsh::run_tmsh(path, output.as_deref(), *modify, include),
        Command::Convert {
            format,
            path,
            output,
            tenant,
            application,
            report,
            passphrase,
        } => commands::convert::run_convert(
            format,
            path,
            output.as_deref(),
            tenant,
            application,
            *report,
            passphrase,
        ),
        Command::RegistryDump {
            section,
            json: _,
            output,
        } => {
            // `--output -` (the default) means stdout, mirroring the Python verb.
            let path = (output != "-").then(|| std::path::Path::new(output));
            commands::registry_dump::run_registry_dump(section, path)
        }
        Command::PcapRemap {
            map_file,
            input,
            output,
            reverse,
            on_unknown,
            schema,
            list_schemas,
        } => commands::pcap_remap::run_pcap_remap(
            map_file,
            input,
            output,
            *reverse,
            on_unknown,
            schema,
            *list_schemas,
        ),
        Command::EnrichPcapng {
            config,
            input,
            output,
            keylog,
            all,
            dry_run,
        } => commands::enrich_pcapng::run_enrich_pcapng(
            config,
            input,
            output,
            keylog.as_deref(),
            *all,
            *dry_run,
        ),
        Command::EnrichWireshark {
            config,
            output,
            force,
        } => commands::enrich_wireshark::run_enrich_wireshark(config, output, *force),
        cmd @ Command::Query { .. } => dispatch_query(cmd),
        Command::Irule { action } => commands::irule::run_irule(action),
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
/// Handle the `--help-*` actions, which short-circuit before requiring an
/// expression / inputs (mirroring argparse's immediate-exit custom actions).
///
/// Returns `Ok(Some(code))` when a help action fired (so the caller exits with
/// `code`), `Ok(None)` when no help flag was set, or an error for the deferred
/// builtins-prose surfaces (`--help-builtins` / `--help-manual`).
fn dispatch_query_help(command: &Command) -> anyhow::Result<Option<u8>> {
    let Command::Query {
        help_dsl,
        help_builtins,
        help_examples,
        help_manual,
        help_renderers,
        help_inputs,
        ..
    } = command
    else {
        unreachable!("dispatch_query_help is only called with Command::Query");
    };

    // `--help-renderers` / `--help-inputs` are fully ported (static catalogues;
    // no user-plugin scan in the Rust port).
    if *help_renderers {
        print!("{}", commands::query::help_renderers_text());
        return Ok(Some(0));
    }
    if *help_inputs {
        print!("{}", commands::query::help_inputs_text());
        return Ok(Some(0));
    }

    // `--help-dsl` prints the static grammar reference (ported byte-for-byte
    // from `dialects/f5/query/grammar.py`); `--help-examples` the cookbook
    // (from `dialects/f5/query/examples.py`).
    if *help_dsl {
        print!("{}", tcl_bigip_query::grammar::format_grammar());
        return Ok(Some(0));
    }
    if *help_examples {
        print!("{}", tcl_bigip_query::examples::format_examples());
        return Ok(Some(0));
    }

    // `--help-builtins` and `--help-manual` are deferred: both depend on the
    // per-function prose catalogue (`format_builtins`), which the Rust
    // `BuiltinSpec` intentionally omits, so byte-parity output is out of scope.
    if help_builtins.is_some() {
        anyhow::bail!("`f5 query --help-builtins` is not yet ported in the Rust port");
    }
    if *help_manual {
        anyhow::bail!(
            "`f5 query --help-manual` is not yet ported in the Rust port \
             (the builtins prose catalogue it embeds is out of scope)"
        );
    }

    Ok(None)
}

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
        enable_probes,
        ca_bundle,
        render_name,
        render_opt,
        ..
    } = command
    else {
        unreachable!("dispatch_query is only called with Command::Query");
    };

    // Help actions short-circuit before any expression / input is required,
    // mirroring argparse's custom actions that `parser.exit()` immediately.
    if let Some(code) = dispatch_query_help(command)? {
        return Ok(code);
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
        &commands::query::ProbeArgs {
            enable_probes: *enable_probes,
            ca_bundle: ca_bundle.clone(),
        },
    )
}
