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

//! T102: tainted values where a command still scans for options.

use super::super::{
    CodeFix, DiagCode, FxHashSet, HashSet, SinkCall, Span, Statement, TaintColour, TaintScan,
    TaintWarning, dialect_to_point, parse_command_substitution_with_spans_and_config,
    scan_argument_names,
};
use super::source_reads::{SourceWarningMetadata, operand_read_taint, original_sink_argument};

/// Emit T102 warnings for option injection into a command declaring a `--`
/// option terminator.
///
/// A T102 violation occurs when a tainted pure-variable-reference argument
/// is passed to a command at a position that can be misinterpreted as a
/// command option (flag), without a preceding `--` terminator to end flag
/// parsing.
///
/// Example: `regexp $pattern $string` where `$pattern` is tainted —
/// if `$pattern` starts with `-`, it will be treated as a `regexp` flag
/// rather than the pattern, producing option injection (T102).
/// Whether `arg` could expand to a string beginning with `-` and thus be
/// (mis)interpreted as a switch — a leading literal `-`, or a leading
/// substitution (`$` / `[` / `{*}` expansion) whose runtime value is
/// unknown. Any other leading literal char is a definite positional and
/// ends option scanning.
fn arg_can_be_option(arg: &str) -> bool {
    match arg.as_bytes().first() {
        None => false,
        Some(&c) => c == b'-' || c == b'$' || c == b'[' || arg.starts_with("{*}"),
    }
}

/// Return the argument indexes still within Tcl's option-scanning region.
/// Tcl scans for `-switch` args from `scan_start` until the first definite
/// positional literal (one that cannot begin with `-`) or `--`. A literal
/// `-option` that takes a value also consumes the following arg.
/// `reserved_trailing_words` excludes a command's structurally-fixed
/// trailing words (e.g. `switch`'s `string` + pattern-list) from the scan
/// entirely, regardless of their shape — see
/// [`tcl_registry::spec::CommandSpec::reserved_trailing_words`].
fn option_scan_region(
    args: &[String],
    scan_start: usize,
    options: &[tcl_registry::hover::OptionSpec],
    reserved_trailing_words: usize,
) -> HashSet<usize> {
    let mut region: HashSet<usize> = HashSet::new();
    let mut i = scan_start;
    let n = args.len().saturating_sub(reserved_trailing_words);
    while i < n {
        let arg = &args[i];
        if arg == "--" {
            region.insert(i);
            break;
        }
        if !arg_can_be_option(arg) {
            // Definite positional literal → option scanning ends here.
            break;
        }
        region.insert(i);
        if arg.starts_with('-')
            && let Some(opt) = options.iter().find(|o| o.matches(arg))
        {
            let consumed = opt.value_word_count(args, i);
            if consumed > 0 {
                i += 1 + consumed;
                continue;
            }
        }
        i += 1;
    }
    region
}

/// Resolve a tight per-argument span for an argument-anchored taint
/// diagnostic (T102 option injection, W313 destructive-file path), so the
/// highlighted range (and any fix) covers only the offending argument
/// rather than the whole statement.
///
/// - `Call` / `Barrier`: the argument's own token span, read directly off
///   `tokens.argv` (offset by 1 to skip the command-name word — `argv[0]`
///   is the command, `argv[i + 1]` is `args[i]`).
/// - `AssignValue` (`set x [cmd ...]`): the embedded command has no
///   token of its own on *this* statement — `tokens.argv[2]` is the
///   whole bracketed value word (`argv[0]` = `set`, `argv[1]` = the
///   variable name, `argv[2]` = the `[...]` value), so the argument's
///   position is re-located inside that word's own text via
///   `parse_command_substitution_with_spans` and offset by the value
///   word's span start.
///
/// Returns `None` when the token data needed isn't available (e.g.
/// synthetic/test-built IR with `tokens: None`) — callers fall back to
/// the whole-statement span in that case. A tight span is a
/// highlighting nicety, never a correctness requirement, so this never
/// panics or guesses.
pub(in super::super) fn arg_index_span(
    stmt: &Statement,
    arg_index: usize,
    config: tcl_lexer::LexerConfig,
) -> Option<Span> {
    match stmt {
        Statement::Call {
            tokens: Some(t), ..
        }
        | Statement::Barrier {
            tokens: Some(t), ..
        } => {
            let written = if t.source_binding.is_some() {
                crate::registry_invocation::effective_command_words(t)?
                    .written_argument(arg_index)?
            } else {
                arg_index
            };
            t.argv.get(written + 1).copied()
        }
        Statement::AssignValue {
            value,
            tokens: Some(t),
            ..
        } => {
            let value_span = t.argv.get(2)?;
            let (_cmd, args) = parse_command_substitution_with_spans_and_config(value, config)?;
            let (_text, arg_span) = args.get(arg_index)?;
            Some(Span::new(
                value_span.start() + arg_span.start(),
                value_span.start() + arg_span.end(),
            ))
        }
        _ => None,
    }
}

/// Select option positions before any warning is emitted. Actual source
/// callers require the same retained effective vector as the sink candidate;
/// missing metadata does not reopen the explicit standalone compatibility API.
pub(in super::super) fn option_injection_arguments(
    call: &SinkCall<'_>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<(String, Vec<usize>)> {
    if let Some(selected) =
        call.source_read
            .and_then(|source| match source.metadata.invocation_metadata() {
                SourceWarningMetadata::Native(selected) => Some(selected),
                _ => None,
            })
    {
        let context = call.source.metadata?;
        let input = context.source_analysis_input()?;
        let tokens = call.tokens?;
        if !context.matches_registry(call.registry)
            || call.argument_offset != 0
            || call.command != selected.words.command()
            || call.effective != Some(&selected.effective)
            || selected
                .effective
                .argument_spellings(tokens.argv_texts.get(1..)?)?
                .as_slice()
                != call.args
        {
            return None;
        }
        let options = selected
            .words
            .with_source_schema(input.borrowed_context_registry(), |schema| {
                schema.authored_source_possible_option_arguments()
            })??;
        let mut label = selected.words.command().to_owned();
        for selector in options.subcommands {
            label.push(' ');
            label.push_str(selector);
        }
        return Some((label, options.arguments));
    }
    if let Some(context) = call.source.metadata {
        let tokens = call.tokens?;
        let (selected, options) =
            crate::registry_invocation::conditional_option_arguments_with_metadata_context(
                call.registry,
                context,
                tokens,
            )?;
        if call.argument_offset != 0
            || call.effective != Some(&selected.effective)
            || selected
                .effective
                .argument_spellings(tokens.argv_texts.get(1..)?)?
                .as_slice()
                != call.args
        {
            return None;
        }
        let mut label = selected.facts.canonical_command;
        for selector in options.subcommands {
            label.push(' ');
            label.push_str(selector);
        }
        return Some((label, options.arguments));
    }
    if !call.source.allows_nominal_metadata() {
        return None;
    }
    let arguments = call.args.iter().map(String::as_str).collect::<Vec<_>>();
    let profile = call.registry.resolve_option_terminator(
        call.command,
        &arguments,
        dialect_to_point(dialect),
    )?;
    let command = call
        .registry
        .get(call.command)
        .map_or(call.command, |spec| spec.name);
    let label = profile
        .subcommand
        .map_or_else(|| command.to_owned(), |sub| format!("{command} {sub}"));
    let mut region = option_scan_region(
        call.args,
        profile.scan_start,
        profile.options,
        profile.reserved_trailing_words,
    )
    .into_iter()
    .collect::<Vec<_>>();
    region.sort_unstable();
    Some((label, region))
}

/// Emit `T102` (option injection) for tainted variables that sit in an
/// option-scanning position of a command declaring a `--` terminator.
///
/// Actual source advice consumes selected Registry option topology and the
/// same retained effective argument vector. Explicit standalone callers use
/// their scalar compatibility contract. Warnings need independent source-read
/// taint and mitigation facts and grant no execution or Normal completion.
pub(super) fn emit_option_injection<S: std::hash::BuildHasher>(
    sink_call: &SinkCall<'_>,
    env: &TaintScan<'_, S>,
    span: Span,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    warnings: &mut Vec<TaintWarning>,
    stmt: &Statement,
) {
    let Some((cmd_label, ordered)) = option_injection_arguments(sink_call, dialect) else {
        return;
    };
    let args = sink_call.args;
    let mut emitted: FxHashSet<String> = FxHashSet::default();
    for i in ordered {
        let Some(arg) = args.get(i) else { continue };
        let mut names: Vec<String> = scan_argument_names(env, arg);
        names.sort_unstable();
        names.dedup();
        for var in names {
            if emitted.contains(&var) {
                continue;
            }
            if env.ssa.var_symbol(&var).is_some_and(|symbol| {
                env.quoted_uses
                    .is_some_and(|quoted| quoted.contains(&symbol))
            }) {
                continue;
            }
            let Some(t) = operand_read_taint(env, &var, original_sink_argument(sink_call, i))
            else {
                continue;
            };
            if !t.is_tainted() {
                continue;
            }
            // Suppress when a mitigating colour proves the value cannot
            // start with `-` (PATH_PREFIXED / NON_DASH_PREFIXED /
            // IP_ADDRESS / PORT / FQDN) — the T102_SAFE set.
            if t.colours.intersects(TaintColour::T102_SAFE) {
                continue;
            }
            // Anchor the diagnostic (and its fix) at the offending
            // argument itself when the source tokens are available;
            // fall back to the whole statement's span otherwise (no
            // fix in that case — a fabricated span could misplace the
            // insertion, e.g. before the command name rather than the
            // argument).
            let (warn_span, fixes) = match arg_index_span(stmt, i, sink_call.source.config) {
                Some(tight) => (
                    tight,
                    vec![CodeFix {
                        span: Span::new(tight.start(), tight.start()),
                        new_text: "-- ".to_owned(),
                        description: "Insert '--' option terminator".to_owned(),
                        // T102: `--` stops a tainted value being read as an option — the
                        // fix, and a change for a call that meant it as one.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                ),
                None => (span, Vec::new()),
            };
            warnings.push(TaintWarning {
                source_sink: None,
                span: warn_span,
                variable: var.clone(),
                sink_command: cmd_label.clone(),
                code: DiagCode::T102,
                message: format!(
                    "Tainted variable ${var} in option position of '{cmd_label}' \
                     without '--' terminator; risk of option injection"
                ),
                replacement: None,
                fixes,
            });
            emitted.insert(var);
        }
    }
}
