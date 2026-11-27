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

//! Which sink a command is: the diagnostic code and label from the selected registry schema, and the network and cross-interpreter sinks.

use super::super::{CommandRegistry, DiagCode, DiagFamily, SinkCall, is_irules_dialect};
use super::source_reads::SourceWarningMetadata;

// Sink detection

/// Return the diagnostic code and human-readable sink label for a
/// statement that acts as a taint sink, or `None` if the statement is
/// not a sink.
///
/// Single registry-driven classification via
/// [`tcl_registry::taint::classify_taint_sinks`] — no command name is
/// matched here. The registry's most specific fact wins: a command that
/// declares an explicit output/log sink code (`puts` → `taint_output_sink
/// = "T101"`, `HTTP::respond` → `"IRULE3001"`, `log` → `taint_log_sink =
/// "IRULE3003"`) is classified as that sink, even when it also carries the
/// generic `TAINT_SINK` trait; only a command with *no* declared sink code
/// falls back to the trait-driven **T100** code-execution classification
/// (`EVALUATES_CODE` / `TAINT_SINK` — `eval`, `exec`, `uplevel`, `subst`,
/// `expr`).
///
/// `classify_taint_sinks` is asked with an empty dialect (no
/// `supports_dialect` filtering) so a sink the active profile *bans*
/// (`exec` under f5-irules — excluded by the §9 disable list, not by its
/// surface) keeps its
/// T100 classification: a banned command that appears in source anyway is
/// still a code-execution sink. The iRules-only sink codes (`IRULE…`, from
/// `taint_output_sink` / `taint_log_sink` on iRules specs) are instead
/// gated explicitly on the active document dialect via
/// [`DiagCode::family`] — the same explicit `is_irules_dialect` gate
/// [`find_setter_constraint_warnings`] uses for IRULE3101, rather than
/// relying on whether the shared registry happens to have those specs
/// loaded.
///
/// Before any of that, a command declaring
/// [`tcl_registry::CommandSpec::taint_sink_gate`] gets one more check: when
/// this call's own option flags disable its hazard (`subst -nocommands
/// -novariables`),
/// the call is not a sink at all, regardless of which code it would
/// otherwise have classified as.
pub(in super::super) fn classify_sink(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<(DiagCode, String)> {
    let canonical_command = registry.get(command).map_or(command, |spec| spec.name);
    // A call whose own option flags disable this command's hazard for
    // *this* invocation (e.g. `subst -nocommands -novariables $x`) is not a sink
    // at all — checked before any classification below.
    if let Some(spec) = registry.get(command)
        && let Some(gate) = spec.taint_sink_gate
    {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        if !gate(&arg_refs) {
            return None;
        }
    }

    let arg_refs: Vec<_> = args.iter().map(String::as_str).collect();
    if is_irules_dialect(dialect)
        && let Some((name, _)) =
            tcl_registry::taint::header_write_arguments(registry, command, &arg_refs)
    {
        let label = if name == 0 {
            canonical_command.to_owned()
        } else {
            format!("{canonical_command} {}", args[0])
        };
        return Some((DiagCode::Irule3002, label));
    }

    let subcommand = args.first().map(String::as_str);
    let info = tcl_registry::taint::classify_taint_sinks(registry, command, subcommand, None);

    if let Some(code_str) = info.output_sink.or(info.log_sink) {
        let code: DiagCode = code_str.parse().unwrap_or_else(|_| {
            panic!("registry declared unknown sink code {code_str:?} for {command}")
        });
        if code.family() == DiagFamily::IRule && !is_irules_dialect(dialect) {
            return None;
        }
        let label = if info.output_sink_is_subcommand_qualified {
            let canonical = subcommand
                .and_then(|s| {
                    registry
                        .get(command)
                        .and_then(|spec| spec.resolve_subcommand(s))
                })
                .map_or_else(|| subcommand.unwrap_or_default(), |sub| sub.name);
            format!("{canonical_command} {canonical}")
        } else {
            canonical_command.to_owned()
        };
        return Some((code, label));
    }

    if info.is_code_sink {
        return Some((DiagCode::T100, canonical_command.to_owned()));
    }

    None
}

/// Actual selected Registry descriptor and effective argument presentation
/// for a source call. Detached legacy callers retain their explicit scalar
/// path; source callers with missing selection are unavailable.
pub(in super::super) fn selected_sink_schema<'a>(
    call: &SinkCall<'a>,
) -> Option<(&'a tcl_registry::CommandSpec, String, Vec<String>)> {
    if let Some(read) = call.source_read
        && let SourceWarningMetadata::Authored(selected) = read.metadata.invocation_metadata()
    {
        selected.matches_point(read.point).then_some(())?;
        return selected.sink_schema(
            call.registry,
            call.tokens?,
            call.effective,
            call.argument_offset,
            call.args,
        );
    }
    if call.source.allows_nominal_metadata() && call.source.metadata.is_none() {
        let spec = call.registry.get(call.command)?;
        return Some((spec, spec.name.to_owned(), call.args.to_vec()));
    }
    if let Some(read) = call.source_read
        && let SourceWarningMetadata::Hosted(hosted) = read.metadata.invocation_metadata()
    {
        let command = hosted.shape().command();
        let spec = call.registry.get(command)?;
        return Some((spec, command.to_owned(), call.args.to_vec()));
    }
    let context = call.source.metadata?;
    if !context.matches_registry(call.registry) {
        return None;
    }
    let tokens = call.tokens?;
    if let Some(binding) = tokens.source_binding.as_ref()
        && binding.unknown
        && binding
            .withdrawn_world_head
            .as_deref()
            .is_some_and(|head| head.trim_start_matches("::") == call.command)
    {
        let spec = call.registry.get(call.command)?;
        return Some((spec, spec.name.to_owned(), call.args.to_vec()));
    }
    let selected = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        call.registry,
        Some(context),
        tokens,
    )?;
    if call
        .effective
        .is_some_and(|effective| effective != &selected.effective)
    {
        return None;
    }
    let args = selected
        .effective
        .argument_spellings(tokens.argv_texts.get(1..)?)?;
    let projected =
        args.get(call.argument_offset..call.argument_offset.checked_add(call.args.len())?)?;
    if projected != call.args {
        return None;
    }
    let command = selected.facts.canonical_command;
    let spec = call.registry.get(&command)?;
    Some((spec, command, args))
}

/// Classify a source call only through the actual selected invocation and
/// its effective argument vector. The standalone scalar helper remains for
/// explicitly detached callers and tests.
pub(super) fn classify_source_sink(
    call: &SinkCall<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<(DiagCode, String)> {
    let (_, command, args) = selected_sink_schema(call)?;
    classify_sink(call.registry, &command, &args, dialect)
}

/// Registry-driven SSRF / cross-interpreter sinks (T104 / T105),
/// returned in addition to the primary [`classify_sink`] match so a
/// single statement can trip multiple categories:
///
/// * **T104** — the command's spec carries `taint_network_sink_args`
///   (a network-address argument → SSRF risk; e.g. `socket`).
/// * **T105** — the first argument names a subcommand in the spec's
///   `taint_interp_eval_subcommands` (cross-interpreter code execution;
///   e.g. `interp eval` / `interp invokehidden`).
pub(in super::super) fn classify_network_interp_sinks(
    registry: &CommandRegistry,
    command: &str,
    args: &[String],
) -> Vec<(DiagCode, String)> {
    let mut out = Vec::new();
    let Some(spec) = registry.get(command) else {
        return out;
    };
    if spec.taint_network_sink_args.is_some() {
        out.push((DiagCode::T104, spec.name.to_owned()));
    }
    // Resolve the subcommand prefix-aware (`interp ev` ⇒ `eval`) before testing
    // membership, so a legal abbreviation does not dodge the cross-interpreter
    // eval classification.
    if let Some(sub) = args.first() {
        let canonical = spec
            .resolve_subcommand(sub)
            .map_or(sub.as_str(), |s| s.name);
        if spec.taint_interp_eval_subcommands.contains(&canonical) {
            out.push((DiagCode::T105, format!("{} {canonical}", spec.name)));
        }
    }
    out
}
