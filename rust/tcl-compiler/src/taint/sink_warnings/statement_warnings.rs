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

//! Warnings for one SSA statement: sink candidates, retained invocations, expression statements and the dispatch to each check.

use super::super::substitution_taint::hosted_argument_spellings;
use super::super::{
    CommandHeadContext, CommandRegistry, CommandTokens, HashMap, HashSet, SinkCall,
    SinkWarningContext, SourceWarningRead, SsaFunction, SsaStatement, Statement, Symbol, TaintCtx,
    TaintLattice, TaintScan, TaintSinkSource, TaintWarning, ValueKey, Version,
    callback_static_word_value, parse_command_substitution_with_config,
};
use super::classification::{classify_network_interp_sinks, classify_sink, classify_source_sink};
use super::command_heads::{
    StaticCommandHead, emit_command_head_substitution_warnings, emit_dynamic_command_head_warning,
    resolve_taint_command_heads, static_invocation,
};
use super::double_encode::emit_double_encode_warnings;
use super::numeric_coercion::{
    CoercionOperand, CoercionTaint, emit_expr_coercion_warnings, emit_numeric_coercion_warnings,
    expression_read_taint,
};
use super::option_injection::emit_option_injection;
use super::pattern_operands::emit_regexp_pattern_warnings;
use super::sink_emission::emit_sink_warnings;
use super::source_reads::SourceWarningMetadata;

fn emit_original_assign_value_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    source: crate::ssa::SsaSourceView<'_>,
    context: &SinkWarningContext<'_, H>,
) {
    let stmt = &ssa_stmt.statement;
    let Some(parent) = source
        .source_tokens()
        .filter(|parent| stmt.tokens() == Some(*parent))
    else {
        return;
    };
    let Some(metadata) = context.source.metadata else {
        return;
    };
    let Some(children) = crate::word_subst::checked_original_argument_lifted_calls(
        parent,
        context.registry,
        metadata,
    ) else {
        return;
    };
    for child in children {
        if let Some(tokens) = child.tokens {
            emit_retained_invocation_warnings(&tokens, ssa_stmt, taints, warnings, source, context);
        }
    }
}

/// Emit taint warnings for a single SSA statement.
///
/// Handles three cases:
/// - `AssignExpr` / `ExprEval`: any tainted use is a T100 injection.
/// - `Call` / `Barrier` / `AssignValue` with `[cmd ...]`: classify as a
///   sink via the registry and emit T100/T101 per tainted use.
/// - `Call`: additionally emits T102 option-injection warnings.
pub(in super::super) fn emit_statement_warnings<
    S: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
>(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    source: crate::ssa::SsaSourceView<'_>,
    context: &SinkWarningContext<'_, H>,
    command_head_ctx: Option<CommandHeadContext<'_>>,
) {
    let ssa = source.function();
    if emit_expr_statement_warnings(
        ssa_stmt,
        taints,
        warnings,
        source,
        context.registry,
        (context.source.config, context.grammar()),
    ) {
        return;
    }

    let dialect = context.dialect;
    let stmt = &ssa_stmt.statement;

    if !context.source.allows_nominal_metadata() && matches!(stmt, Statement::AssignValue { .. }) {
        emit_original_assign_value_warnings(ssa_stmt, taints, warnings, source, context);
        return;
    }

    // Owned fallback for `AssignValue` — `parse_command_substitution`
    // returns an owned (String, Vec<String>) so we stash it in this
    // scope and borrow into the uniform `(command, call_args)` tuple
    // below. Preserves sub-command args so e.g.
    // `set _ [HTTP::header insert X-Foo $v]` still reaches the
    // IRULE3002 subcommand gate.
    let assign_parsed: Option<(String, Vec<String>)> = match stmt {
        Statement::AssignValue { value, .. } => {
            parse_taint_command_substitution(value.trim(), context.source.config)
        }
        _ => None,
    };
    // Prefer the canonical target the lowerer already resolved
    // (`stmt.canonical_command_or_source()`, populated for a resolved
    // `interp alias` or static `rename`) over the source spelling, so
    // `interp alias {} myEval {} eval; myEval $tainted` (or `rename eval
    // myEval; myEval $tainted`) is classified against `eval`'s registry
    // sink data instead of silently missing it because `"myEval"` isn't a
    // registered command. Falls back to the source name unchanged when no
    // alias/rename was resolved (the common case). `tokens` carries the
    // per-argument-word spans used to narrow a sink warning's highlight to
    // the one tainted argument (`tight_arg_span`) — only available for a
    // real `Call`/`Barrier` statement; the `AssignValue` fallback
    // re-parses a nested `[cmd …]` out of the `set` RHS text and has no
    // per-word spans for it, so it falls back to the whole-statement span.
    let (raw_command, call_args, tokens): (&str, &[String], Option<&CommandTokens>) = match stmt {
        Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } => (
            stmt.canonical_command_or_source(),
            args.as_slice(),
            tokens.as_ref(),
        ),
        Statement::AssignValue { .. } => match assign_parsed.as_ref() {
            Some((cmd, sub_args)) => (cmd.as_str(), sub_args.as_slice(), None),
            None => return,
        },
        _ => return,
    };
    let has_canonical_override = matches!(
        stmt,
        Statement::Call { command, .. } | Statement::Barrier { command, .. }
            if raw_command != command
    );
    let static_invocation = (!has_canonical_override)
        .then(|| static_invocation(tokens?, call_args, dialect))
        .flatten();
    let written_command = static_invocation
        .as_ref()
        .map_or(raw_command, |invocation| invocation.head.command.as_str());
    let call_args = static_invocation
        .as_ref()
        .and_then(|invocation| invocation.effective_args.as_deref())
        .unwrap_or(call_args);
    // Once a head expansion inserts or removes argv elements, the per-word
    // token indices no longer line up with the effective invocation, so use
    // the statement span rather than pointing a sink diagnostic at the wrong
    // source word.
    let sink_tokens = static_invocation
        .as_ref()
        .map_or(tokens, |invocation| invocation.sink_tokens(tokens));

    // Every command substitution in command position executes before Tcl can
    // dispatch the outer result. Eval's inline-body lowering can produce this
    // generic IR shape for `eval {[eval $x]}`; compound heads such as
    // `prefix[eval $x]` and `"[puts ok][eval $x]"` execute their substitutions
    // too. Classify every one against the registry. The eventual outer command
    // name remains unknown and follows the ordinary conservative path below.
    emit_command_head_substitution_warnings(
        tokens,
        ssa_stmt,
        taints,
        warnings,
        ssa,
        context,
        command_head_ctx,
    );

    if let Some(tokens) = tokens
        && tokens.source_binding.is_some()
    {
        // A proved wrapper's argument boundary is where its substituted words
        // reach the wrapper: `eval p $x s` hands the concatenated script to
        // `eval` there, while its phases and residual boundary pass nothing.
        let wrapper = tokens.evaluated_arguments_invocation();
        let tokens = wrapper.as_ref().unwrap_or(tokens);
        emit_retained_invocation_warnings(tokens, ssa_stmt, taints, warnings, source, context);
        return;
    }

    let resolved_heads =
        resolve_taint_command_heads(written_command, stmt, tokens, command_head_ctx);
    if resolved_heads.is_empty() {
        emit_dynamic_command_head_warning(
            tokens
                .and_then(|tokens| tokens.word_exprs.first())
                .map(|head| (head, context.source.config)),
            ssa_stmt,
            taints,
            warnings,
            ssa,
        );
        return;
    }
    emit_sink_candidates(
        SinkCandidates {
            heads: &resolved_heads,
            args: call_args,
            tokens: sink_tokens,
        },
        ssa_stmt,
        taints,
        warnings,
        ssa,
        context,
    );
}

#[derive(Clone, Copy)]
struct SinkCandidates<'a> {
    heads: &'a [StaticCommandHead],
    pub(in super::super) args: &'a [String],
    pub(in super::super) tokens: Option<&'a CommandTokens>,
}

fn emit_sink_candidates<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    candidates: SinkCandidates<'_>,
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
    context: &SinkWarningContext<'_, H>,
) {
    if !context.source.allows_nominal_metadata() && context.source.metadata.is_none() {
        return;
    }
    let (resolved_heads, call_args, sink_tokens) =
        (candidates.heads, candidates.args, candidates.tokens);
    let (registry, dialect, shadowed_builtins) =
        (context.registry, context.dialect, context.shadowed_builtins);
    if resolved_heads.len() <= 1 {
        if let Some(head) = resolved_heads.first() {
            let resolved_args = head.effective_args(call_args);
            emit_resolved_statement_warnings(
                SinkCall {
                    source: context.source,
                    command: &head.command,
                    args: resolved_args.as_deref().unwrap_or(call_args),
                    tokens: (!head.affects_args()).then_some(sink_tokens).flatten(),
                    registry,
                    effective: None,
                    source_read: None,
                    argument_offset: 0,
                },
                ssa_stmt,
                &ssa_stmt.uses,
                taints,
                dialect,
                shadowed_builtins,
                warnings,
                ssa,
            );
        }
        return;
    }

    // A finite command-head join can contain equivalent source spellings
    // (`eval` / `::eval`). Keep one diagnostic for the semantic invocation,
    // while still evaluating every candidate independently so namespace
    // shadowing cannot hide an explicitly-global sink arm.
    let start = warnings.len();
    let mut candidates = Vec::new();
    for head in resolved_heads {
        let resolved_args = head.effective_args(call_args);
        emit_resolved_statement_warnings(
            SinkCall {
                source: context.source,
                command: &head.command,
                args: resolved_args.as_deref().unwrap_or(call_args),
                tokens: (!head.affects_args()).then_some(sink_tokens).flatten(),
                registry,
                effective: None,
                source_read: None,
                argument_offset: 0,
            },
            ssa_stmt,
            &ssa_stmt.uses,
            taints,
            dialect,
            shadowed_builtins,
            &mut candidates,
            ssa,
        );
    }
    for warning in candidates {
        if !warnings[start..].contains(&warning) {
            warnings.push(warning);
        }
    }
}

type RetainedSinkCandidate = (
    String,
    crate::registry_invocation::EffectiveCommandWords,
    Option<SourceWarningMetadata>,
);

fn retained_sink_candidates<H>(
    tokens: &CommandTokens,
    context: &SinkWarningContext<'_, H>,
) -> (Vec<RetainedSinkCandidate>, bool) {
    if !context.source.has_authored_body() {
        return ordinary_retained_sink_candidates(tokens, context);
    }
    let Some(selected) = context.source.authored_body_sink(tokens, context.registry) else {
        return (Vec::new(), false);
    };
    (
        vec![(
            selected.command().to_owned(),
            selected.effective().clone(),
            Some(SourceWarningMetadata::Authored(Box::new(selected))),
        )],
        false,
    )
}

fn ordinary_retained_sink_candidates<H>(
    tokens: &CommandTokens,
    context: &SinkWarningContext<'_, H>,
) -> (Vec<RetainedSinkCandidate>, bool) {
    let mut candidates = Vec::new();
    if let Some(invocation) =
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            context.registry,
            context.source.metadata,
            tokens,
        )
    {
        candidates.push((
            invocation.facts.canonical_command,
            invocation.effective,
            None,
        ));
    } else if let Some(assistance) =
        crate::registry_invocation::registry_invocation_assistance_with_metadata_context(
            context.registry,
            context.source.metadata,
            tokens,
        )
    {
        candidates.extend(
            assistance
                .candidates
                .into_iter()
                .map(|shape| (shape.command, shape.effective, None)),
        );
    }
    let native_source = tokens.source_binding.as_ref().is_some_and(
        super::super::super::command_binding::SourceInvocationBinding::has_original_native_source_command,
    );
    // Present Native source ownership is terminal, independently of reached argv.
    if candidates.is_empty() && native_source {
        if let (Some(module), Some(body), Some(metadata)) = (
            context.source.module(),
            context.source.body_id(),
            context.source.metadata,
        ) && let Some(selected) = crate::registry_invocation::native_source_taint_invocation(
            context.registry, metadata, module, body, tokens,
        ) {
            candidates.push((
                selected.words.command().to_owned(),
                selected.effective.clone(),
                Some(SourceWarningMetadata::Native(Box::new(selected))),
            ));
        }
    } else if candidates.is_empty()
        && context.source.module().is_some()
        && let Some(assistance) =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                context.registry,
                context.source.metadata,
                tokens,
            )
    {
        candidates.extend(
            assistance
                .candidates
                .into_iter()
                .map(|shape| (shape.command, shape.effective, None)),
        );
    }
    if let Some(hosted) =
        crate::registry_invocation::hosted_source_taint_invocation(context.registry, tokens)
            .filter(|_| context.source.metadata.is_some() && !native_source)
    {
        candidates.push((
            hosted.metadata.shape().command().to_owned(),
            hosted.effective,
            Some(SourceWarningMetadata::Hosted(Box::new(hosted.metadata))),
        ));
    }
    // The explicit current Standalone CU may carry possible source syntax,
    // not an executed target. Native/hosted or missing ownership is terminal.
    if !native_source
        && context.source.allows_nominal_metadata()
        && context.source.module().is_some()
        && let Some(selected) = crate::registry_invocation::AuthoredSourceTaintInvocation::capture(
            tokens,
            context.registry,
        )
    {
        let command = selected.command().to_owned();
        let effective = selected.effective().clone();
        candidates.retain(|(held_command, held_effective, _)| {
            held_command != &command || held_effective != &effective
        });
        candidates.push((
            command,
            effective,
            Some(SourceWarningMetadata::Authored(Box::new(selected))),
        ));
    }
    (candidates, native_source)
}

/// Diagnostics may inspect a known possible sink without upgrading its shape
/// into an implementation or store proof. Exact runtime/native facts take
/// precedence; explicit unknown or absent lookup never falls back to spelling.
fn emit_retained_invocation_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    tokens: &CommandTokens,
    statement: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    source: crate::ssa::SsaSourceView<'_>,
    context: &SinkWarningContext<'_, H>,
) {
    if !context.source.allows_nominal_metadata() && context.source.metadata.is_none() {
        return;
    }
    let ssa = source.function();
    let Some(args) = tokens.argv_texts.get(1..) else {
        return;
    };
    let (candidates, _) = retained_sink_candidates(tokens, context);
    for (command, effective, source_metadata) in candidates {
        let original_reads = (!context.source.allows_nominal_metadata())
            .then(|| {
                crate::registry_invocation::original_source_taint_read_operands(
                    context.registry,
                    context.source.metadata?,
                    source,
                    tokens,
                )
            })
            .flatten();
        // The selected Native/hosted candidate remains the invocation owner.
        // Original read geometry supplements it instead of being skipped when
        // that stronger selected candidate was already present.
        let source_metadata = match (source_metadata, original_reads) {
            (Some(invocation), Some(reads)) => Some(SourceWarningMetadata::WithOriginalReads {
                invocation: Box::new(invocation),
                reads: Box::new(reads),
            }),
            (None, Some(reads)) => Some(SourceWarningMetadata::Original(Box::new(reads))),
            (invocation, None) => invocation,
        };
        if !context.source.allows_nominal_metadata() && source_metadata.is_none() {
            continue;
        }
        let Some(args) = effective.argument_spellings(args) else {
            continue;
        };
        let mut candidate_warnings = Vec::new();
        emit_resolved_statement_warnings(
            SinkCall {
                source: context.source,
                command: &command,
                args: &args,
                tokens: Some(tokens),
                registry: context.registry,
                effective: Some(&effective),
                source_read: source_metadata.as_ref().map(|metadata| SourceWarningRead {
                    metadata,
                    point: source,
                }),
                argument_offset: 0,
            },
            statement,
            &statement.uses,
            taints,
            context.dialect,
            context.shadowed_builtins,
            &mut candidate_warnings,
            ssa,
        );
        for warning in candidate_warnings {
            if !warnings.contains(&warning) {
                warnings.push(warning);
            }
        }
    }
}

/// Emit the direct expression-injection case and report whether this statement
/// was expression-shaped. Keeping it separate leaves call classification
/// focused on registry-backed command invocations.
fn emit_expr_statement_warnings<S: std::hash::BuildHasher>(
    ssa_stmt: &SsaStatement,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    warnings: &mut Vec<TaintWarning>,
    source: crate::ssa::SsaSourceView<'_>,
    registry: &CommandRegistry,
    (config, grammar): (tcl_lexer::LexerConfig, tcl_dialect::LexerGrammar),
) -> bool {
    let stmt = &ssa_stmt.statement;
    let (Statement::AssignExpr {
        expr, expr_base, ..
    }
    | Statement::ExprEval {
        expr, expr_base, ..
    }) = stmt
    else {
        return false;
    };
    let resolve = |hit: &CoercionOperand| {
        if source.function().point_contexts.is_some() {
            return expression_read_taint(
                source,
                hit.variable.as_ref()?,
                *expr_base,
                registry,
                taints,
            );
        }
        let sym = source.symbol(&hit.name)?;
        let version = *ssa_stmt.uses.get(&sym)?;
        Some(CoercionTaint::represented(sym, version, taints))
    };
    emit_expr_coercion_warnings(
        expr,
        *expr_base,
        stmt.span(),
        resolve,
        warnings,
        (config, grammar),
    );
    true
}

/// Parse one command substitution for registry-backed sink classification,
/// reducing a statically valued command word with the document's escape
/// grammar. The shared shape parser retains the original argument spellings;
/// the segmenter supplies the missing Tcl value of heads such as `e\166al`,
/// `{eval}`, and `"eval"`. Dynamic or compound heads remain unchanged and are
/// handled conservatively by command-head provenance.
pub(in super::super) fn parse_taint_command_substitution(
    text: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<(String, Vec<String>)> {
    let config = config.nested();
    let (written_head, args) = parse_command_substitution_with_config(text, config)?;
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let commands = crate::segmenter::segment_commands_with_offset_and_config(inner, 0, config);
    let [command] = commands.as_slice() else {
        return None;
    };
    if command.is_partial || !command.word_views_aligned() {
        return None;
    }
    let semantic_head = command
        .texts
        .first()
        .zip(command.word_fragments.first())
        .and_then(|(word, fragments)| {
            callback_static_word_value(
                word,
                fragments,
                config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
            )
        })
        .unwrap_or(written_head);
    Some((semantic_head, args))
}

#[allow(clippy::too_many_arguments)]
fn emit_resolved_statement_warnings<S: std::hash::BuildHasher, H: std::hash::BuildHasher>(
    sink_call: SinkCall<'_>,
    ssa_stmt: &SsaStatement,
    uses: &HashMap<Symbol, Version>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    shadowed_builtins: &HashSet<String, H>,
    warnings: &mut Vec<TaintWarning>,
    ssa: &SsaFunction,
) {
    let (command, call_args, tokens, registry) = (
        sink_call.command,
        sink_call.args,
        sink_call.tokens,
        sink_call.registry,
    );
    let first_warning = warnings.len();
    let stmt = &ssa_stmt.statement;
    let span = stmt.span();

    // A namespace-scoped user proc shadows this bare/canonical name (e.g.
    // `proc ::myns::puts {...}` shadowing the real `puts` for calls made
    // from `::myns`) — every check below keys off the registry, which would
    // otherwise misclassify a call to the user's own (possibly entirely
    // safe) proc as the builtin sink/source. Whatever the shadowing proc
    // itself does is covered separately by interprocedural taint analysis.
    if shadowed_builtins.contains(command)
        && tokens
            .and_then(|tokens| tokens.source_binding.as_ref())
            .is_none()
    {
        return;
    }

    // Emission order per statement:
    // T103 (regexp pattern) → T106 (double-encode) → the sink loop
    // (T100/output/log, then T102, then T104/T105).

    let env = TaintScan {
        uses,
        taints,
        ssa,
        quoted_uses: Some(&ssa_stmt.quoted_uses),
        source_read: sink_call.source_read,
        source: sink_call.source,
    };

    // T103: tainted data in a regexp/regsub pattern position.
    emit_regexp_pattern_warnings(&env, span, registry, warnings, tokens, sink_call.source);

    // T106: re-encoding an already-encoded tainted value. A minimal ctx (no
    // interprocedural summaries) is enough — the double-encode colour is added
    // by the encoder command's own registry classification, which `word_taint`
    // reads directly.
    let dbl_ctx = TaintCtx {
        registry,
        ssa,
        interproc: None,
        known_procs: None,
        dialect,
        source: sink_call.source,
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };
    // A handler's encoder condition reads its written spellings.
    let condition_args = sink_call
        .source_read
        .filter(|read| {
            matches!(
                read.metadata.invocation_metadata(),
                SourceWarningMetadata::Hosted(_)
            )
        })
        .and(sink_call.effective)
        .map(|effective| hosted_argument_spellings(effective, call_args));
    emit_double_encode_warnings(
        command,
        call_args,
        uses,
        taints,
        span,
        warnings,
        dbl_ctx,
        &ssa_stmt.quoted_uses,
        condition_args.as_deref(),
    );

    // Primary sink classification (T100 code-exec / T101 + iRules output / log).
    let classified = if sink_call.tokens.is_some() {
        classify_source_sink(&sink_call, dialect)
    } else if sink_call.source.allows_nominal_metadata() {
        classify_sink(registry, command, call_args, dialect)
    } else {
        None
    };
    if let Some((code, sink_label)) = classified {
        emit_sink_warnings(&env, span, code, &sink_label, &sink_call, warnings);
    }

    // T100 numeric coercion outside `expr`: the words this call reads as
    // numbers (`switch -integer`'s subject).
    emit_numeric_coercion_warnings(&sink_call, &env, span, dialect, warnings);

    // T102: option injection — any statement shape that resolves to a
    // command invocation (bare `Call`, a `Barrier`, or a `set x [cmd ...]`
    // `AssignValue`), after the primary sink (T100/output/log). Uses the
    // same `sink_call` (command, call_args, registry) as the check above
    // so a `set matched [regexp $pattern $s]` capture is scanned exactly
    // like the bare `regexp $pattern $s` call it wraps.
    emit_option_injection(&sink_call, &env, span, dialect, warnings, stmt);

    // Additional registry-driven SSRF / cross-interp sinks (T104 / T105),
    // which can co-occur with the primary classification.
    for (code, sink_label) in classify_network_interp_sinks(registry, command, call_args) {
        emit_sink_warnings(&env, span, code, &sink_label, &sink_call, warnings);
    }
    // These warnings were emitted for this exact selected source invocation;
    // retain its issuer before any display projection or whole-document lift.
    let sink = tokens
        .and_then(|tokens| TaintSinkSource::for_original_invocation(tokens, sink_call.source));
    for warning in &mut warnings[first_warning..] {
        warning.source_sink.clone_from(&sink);
    }
}
