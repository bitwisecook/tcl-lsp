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

//! The `f5-query` BIG-IP / iRules CLI.
//!
//! Owns the `clap` command tree (the verb registry and the `irule` verb
//! group), plus dispatch into the BIG-IP engine crates (`tcl-bigip`,
//! `tcl-bigip-query`, `tcl-bigip-remote`, `tcl-bigip-pcap`).
//!
//! A verb whose engine is not yet implemented returns a clear
//! "not yet implemented" error (exit code 2).

#![forbid(unsafe_code)]

mod cli;
mod commands;
pub mod f5mku;

/// Explain-flow computation for embedders (the native MCP `explain_flow` tool).
pub use commands::explain_flow::{ExplainFlowOptions, explain_flow_value};

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
/// Matches `WORKER_STACK_SIZE` in `tcl-lsp-server`/`tcl-mcp`/`tcl-cli`'s
/// entry points — see `tcl-lsp-server/src/main.rs`'s doc comment for the
/// full rationale. Short version: `irule minify` (`commands::irule`) runs
/// `tcl_lsp_core::minify`, which calls straight into
/// `tcl_compiler::analyser::Analyser::analyse` on caller-supplied Tcl
/// source — the same depth-capped-but-stack-hungry recursion chain that
/// crashed `tcl-lsp-server` in issue #996. The OS-provided main-thread
/// stack this binary would otherwise inherit is outside this crate's
/// control (8 MiB by default on Linux, far less guaranteed elsewhere), so
/// every verb runs on an explicitly-sized thread instead.
const WORKER_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Run `dispatch` on a dedicated thread with [`WORKER_STACK_SIZE`] of stack.
fn run_on_generous_stack(command: Command) -> anyhow::Result<u8> {
    std::thread::Builder::new()
        .stack_size(WORKER_STACK_SIZE)
        .spawn(move || dispatch(&command))
        .expect("failed to spawn the f5-query CLI worker thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

// One flat arm per top-level verb; splitting this dispatch would obscure it.
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
        Command::EncryptSecrets {
            path,
            output,
            key,
            salt,
            passphrase,
            format,
        } => commands::secrets::run_secrets(
            commands::secrets::Mode::Encrypt,
            path,
            key,
            salt.as_deref(),
            passphrase,
            format,
            output.as_deref(),
        ),
        Command::DecryptSecrets {
            path,
            output,
            key,
            passphrase,
            format,
        } => commands::secrets::run_secrets(
            commands::secrets::Mode::Decrypt,
            path,
            key,
            None,
            passphrase,
            format,
            output.as_deref(),
        ),
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
            // `--output -` (the default) means stdout.
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
        } => Ok(commands::pcap_remap::run_pcap_remap(
            map_file,
            input,
            output,
            *reverse,
            on_unknown,
            schema,
            *list_schemas,
        )),
        Command::ExplainFlow {
            pcap,
            paths,
            tshark,
            keylog,
            tshark_filter,
            simulate,
            no_event_bodies,
            max_event_lines,
            json,
            output,
        } => commands::explain_flow::run_explain_flow(
            pcap,
            paths,
            *tshark,
            keylog.as_deref(),
            tshark_filter.as_deref(),
            *simulate,
            *no_event_bodies,
            *max_event_lines,
            *json,
            output.as_deref(),
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
        Command::Fetch {
            host,
            user,
            password,
            port,
            ssh_port,
            transport,
            fmt,
            insecure,
            timeout,
            output,
            no_prompt,
            print_path,
        } => Ok(commands::fetch::run_fetch(&commands::fetch::FetchArgs {
            host: host.as_deref(),
            user: user.as_deref(),
            password: password.as_deref(),
            port: *port,
            ssh_port: *ssh_port,
            transport,
            fmt,
            insecure: *insecure,
            timeout: *timeout,
            output: output.as_deref(),
            no_prompt: *no_prompt,
            print_path: *print_path,
        })),
        Command::Push {
            kind,
            payload,
            host,
            user,
            password,
            port,
            no_prompt,
            insecure,
            create,
            dry_run,
            timeout,
        } => Ok(commands::push::run_push(&commands::push::PushArgs {
            kind,
            payload,
            host: host.as_deref(),
            user: user.as_deref(),
            password: password.as_deref(),
            port: *port,
            no_prompt: *no_prompt,
            insecure: *insecure,
            create: *create,
            dry_run: *dry_run,
            timeout: *timeout,
        })),
        Command::Pull {
            kind,
            full_path,
            host,
            user,
            password,
            port,
            no_prompt,
            insecure,
            json,
            timeout,
            format,
        } => Ok(commands::pull::run_pull(&commands::pull::PullArgs {
            kind,
            full_path,
            host: host.as_deref(),
            user: user.as_deref(),
            password: password.as_deref(),
            port: *port,
            no_prompt: *no_prompt,
            insecure: *insecure,
            json: *json,
            timeout: *timeout,
            format: &format.format,
            transaction: format.transaction,
        })),
        cmd @ Command::Query { .. } => dispatch_query(cmd),
        Command::Irule { action } => commands::irule::run_irule(action),
        Command::Completion { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            clap_complete::generate(*shell, &mut cmd, "f5-query", &mut std::io::stdout());
            Ok(0)
        }
    }
}

/// Handle the `--help-*` actions, which short-circuit before requiring an
/// expression / inputs.
///
/// Returns `Ok(Some(code))` when a help action fired (so the caller exits with
/// `code`), `Ok(None)` when no help flag was set, or an error for the
/// unimplemented builtins-prose surfaces (`--help-builtins` / `--help-manual`).
fn dispatch_query_help(command: &Command) -> Option<u8> {
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

    // `--help-renderers` / `--help-inputs` print static catalogues (no
    // user-plugin scan).
    if *help_renderers {
        print!("{}", commands::query::help_renderers_text());
        return Some(0);
    }
    if *help_inputs {
        print!("{}", commands::query::help_inputs_text());
        return Some(0);
    }

    // `--help-dsl` prints the static grammar reference; `--help-examples`
    // the cookbook.
    if *help_dsl {
        print!("{}", tcl_bigip_query::grammar::format_grammar());
        return Some(0);
    }
    if *help_examples {
        print!("{}", tcl_bigip_query::examples::format_examples());
        return Some(0);
    }

    // `--help-builtins [NAME]` and `--help-manual`: `BuiltinSpec` carries no
    // doc prose, so the catalogue is generated from the registry metadata
    // (name / category / arity / flags). `--help-manual` composes grammar +
    // builtins + cookbook.
    if let Some(name) = help_builtins {
        print!(
            "{}",
            tcl_bigip_query::builtins::format_catalogue(name.as_deref())
        );
        return Some(0);
    }
    if *help_manual {
        print!("{}", tcl_bigip_query::manual::format_manual());
        return Some(0);
    }

    None
}

fn dispatch_query(command: &Command) -> anyhow::Result<u8> {
    let Command::Query {
        expression,
        inputs,
        from_file,
        name,
        partition,
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
        format,
        ..
    } = command
    else {
        unreachable!("dispatch_query is only called with Command::Query");
    };

    // Help actions short-circuit before any expression / input is required.
    if let Some(code) = dispatch_query_help(command) {
        return Ok(code);
    }

    // Resolve the query expression, honouring `-f/--from-file`. When
    // `--from-file` is set, the positional `expression` is actually the first
    // input file (the positional slot is filled before `inputs`), so promote
    // it into `inputs`.
    let mut inputs = inputs.clone();
    let resolved_expression: Option<String> = if let Some(from_file) = from_file {
        if let Some(expr) = expression {
            inputs.insert(0, std::path::PathBuf::from(expr));
        }
        match std::fs::read_to_string(from_file) {
            Ok(text) => Some(text),
            Err(e) => {
                // Print the read error, then fall through with `None` so the
                // verb prints the "no query expression" message.
                eprintln!("error: {e}");
                None
            }
        }
    } else {
        expression.clone()
    };

    // `--render NAME` re-uses the same output dispatch path as the built-in
    // modes: `output::render` falls through to the renderer registry on an
    // unknown mode, so we swap the output mode for the requested name and
    // thread the parsed `--render-opt` map through.
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
        resolved_expression.as_deref(),
        &inputs,
        name,
        partition,
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
        format,
        &commands::query::ProbeArgs {
            enable_probes: *enable_probes,
            ca_bundle: ca_bundle.clone(),
        },
    )
}
