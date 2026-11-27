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

//! Replaying stored callbacks as synthetic procedures: candidates, insertion into the original source, the replay unit and its report.

use super::callback_replay_source::{
    callback_input_label, callback_replay_source, whole_command_substitution,
};
use super::registry_colours::unique_instance_class;
use super::sink_warnings::static_word_expr_value;
use super::source_context::TaintSourceContext;
use super::unit_report::taint_warning_report_for_cu_base_with_external_variable_seeds;
use super::{
    CallbackListBuilder, CallbackReplayCandidate, CallbackReplaySource, LocalInstanceClasses,
    TaintLattice, TaintReplayAnalysis, TaintReplaySourceSnapshot, TaintWarning, TaintWarningReport,
    UnavailableTaintReplay, UnavailableTaintReplayReason, dialect_to_point,
    instance_classes_for_function,
};
use crate::ir::CommandTokens;
use crate::ir::Statement;
#[cfg(test)]
use crate::taint::CALLBACK_REPLAY_BUILDS;
#[cfg(test)]
use crate::taint::CALLBACK_REPLAY_NAME_SEARCHES;
use std::collections::{HashMap, HashSet};
use tcl_dialect::model::SurfaceQuery;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_source::words::WordExpr;

/// Maximum statically recoverable stored callbacks replayed in one batch.
///
/// A callback replay performs a normal, interprocedural compiler pass so it
/// can reuse the established SSA and proc-parameter taint transfer. Cap the
/// per-batch work rather than making one synthetic unit quadratic; ordinary
/// applications have a handful of handlers.
pub(super) const MAX_CALLBACK_TAINT_REPLAYS: usize = 32;

/// Synthetic callback parameter used only inside replay procedures. Its
/// top-level source slot is seeded directly in SSA rather than assigned by a
/// source command, so user shadows cannot affect the external-input model.
pub(super) const CALLBACK_REPLAY_INPUT_VAR_BASE: &str = "__tcl_lsp_callback_input";

pub(super) const CALLBACK_REPLAY_PROC_BASE: &str = "__tcl_lsp_callback_replay";

/// Pick synthetic names that cannot collide with source/callback variables.
///
/// Every replayed callback came from `source`, so absence from the complete
/// source text also proves absence from every static callback body we append.
/// Procedure candidates reserve one unique wrapper per bounded replay.
/// Deterministic suffixes keep diagnostics/replays reproducible.
fn fresh_callback_replay_names(source: &str) -> (String, String) {
    #[cfg(test)]
    CALLBACK_REPLAY_NAME_SEARCHES.with(|count| count.set(count.get() + 1));

    // Collecting identifier-shaped matches is linear in the source. Unlike a
    // repeated `contains` loop, it does not turn a deliberately collision-rich
    // document into dozens of full scans before callback analysis even starts.
    let mut occupied = HashSet::new();
    for base in [CALLBACK_REPLAY_INPUT_VAR_BASE, CALLBACK_REPLAY_PROC_BASE] {
        for (start, _) in source.match_indices(base) {
            let suffix = &source[start + base.len()..];
            let identifier_len = suffix
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            occupied.insert(source[start..start + base.len() + identifier_len].to_owned());
        }
    }

    // Every rejected suffix consumes a distinct occupied spelling, so the
    // next `occupied.len() + 1` candidates must contain a collision-free one.
    for suffix in 0..=occupied.len() {
        let input = if suffix == 0 {
            CALLBACK_REPLAY_INPUT_VAR_BASE.to_owned()
        } else {
            format!("{CALLBACK_REPLAY_INPUT_VAR_BASE}_{suffix}")
        };
        let proc_prefix = if suffix == 0 {
            CALLBACK_REPLAY_PROC_BASE.to_owned()
        } else {
            format!("{CALLBACK_REPLAY_PROC_BASE}_{suffix}")
        };
        let proc_collides = (0..MAX_CALLBACK_TAINT_REPLAYS)
            .any(|index| occupied.contains(&format!("{proc_prefix}_{index}")));
        if !occupied.contains(&input) && !proc_collides {
            return (input, crate::naming::rooted_member_key("::", &proc_prefix));
        }
    }
    unreachable!("a bounded synthetic-name search must find an unoccupied suffix")
}

fn retained_callback_list_builder(
    registry: &CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    parent: &CommandTokens,
    span: Span,
) -> Option<CallbackListBuilder> {
    let input = metadata.source_analysis_input()?;
    metadata.matches_registry(registry).then_some(())?;
    let parent_binding = parent.source_binding.as_ref()?;
    parent_binding.original_argument_metadata_for_input(parent, input, registry)?;
    let parent_site = parent_binding.invocation_site()?;
    let mut candidates =
        crate::word_subst::checked_original_argument_lifted_calls(parent, registry, metadata)?
            .into_iter()
            .filter(|candidate| candidate.span == span);
    let child = candidates.next()?;
    if candidates.next().is_some() {
        return None;
    }
    let tokens = child.tokens?;
    if !tokens.original_words_match_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    binding.original_argument_metadata_for_input(&tokens, input, registry)?;
    let site = binding.invocation_site()?;
    if !std::sync::Arc::ptr_eq(&parent_site.source, &site.source)
        || site.offset < span.start()
        || site.offset >= span.end()
        || binding.invocation_realm()
            != Some(tcl_dialect::model::InvocationRealm::InterpreterRuntime)
    {
        return None;
    }
    let image = binding.original_argument_source_image(&tokens)?;
    let bytes = image
        .bytes()
        .get(span.start() as usize..span.end() as usize)?;
    let source = std::str::from_utf8(bytes).ok()?.to_owned();
    if source.len() != span.end().checked_sub(span.start())? as usize
        || !source.starts_with('[')
        || !source.ends_with(']')
    {
        return None;
    }
    let invocation = crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
        registry,
        Some(metadata),
        &tokens,
    )?;
    // The retained effective vectors below start after the command head.
    // Until they carry the selected operand slice, only offset-zero result
    // contracts have an exact correspondence to that full vector.
    if !invocation
        .facts
        .traits
        .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
        || invocation.facts.argument_offset != 0
        || invocation.facts.native_result
            != Some(tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 })
    {
        return None;
    }
    Some(CallbackListBuilder { source, invocation })
}

/// Query a callback argument through either a direct command or a resolved
/// instance method. The latter is driven solely by local receiver-class facts
/// and `CommandRegistry::instance_callback_taint_inputs`; widget names and
/// method spellings never enter the analyser as special cases.
fn callback_taint_inputs_at_call(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    index: usize,
    dialect: Option<SurfaceQuery<'_>>,
    instance_classes: &LocalInstanceClasses,
    receiver: Option<(&WordExpr, tcl_lexer::LexerConfig, u32)>,
) -> &'static [tcl_registry::CallbackTaintInput] {
    let direct = registry.callback_taint_inputs(command, args, index, dialect);
    if !direct.is_empty() {
        return direct;
    }
    let Some((word, config, position)) = receiver else {
        return &[];
    };
    let Some(class) =
        unique_instance_class(Some((word, config)), Some(instance_classes), Some(position))
    else {
        return &[];
    };
    registry.instance_callback_taint_inputs(class, command, args, index, dialect)
}

/// Replay statically materialised stored callbacks with their registry-declared
/// external substitutions replaced by a directly seeded synthetic source.
///
/// The substitution descriptor is the only Tk knowledge here. The callback
/// body is then lowered and checked by the same generic CFG/SSA/interproc
/// taint pipeline as ordinary source, which covers both `set x %P; eval $x`
/// and a `[list helper %P]` prefix flowing into `proc helper {x} { eval $x }`.
/// Dynamic callback construction deliberately abstains.
// Candidate discovery, bounded materialisation, and source-span remapping are
// one safety transaction; keeping them adjacent makes the replay cap auditable.
#[allow(clippy::too_many_lines)]
#[cfg(test)]
pub(super) fn find_callback_substitution_warnings(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    baseline: &[TaintWarning],
) -> Vec<TaintWarning> {
    taint_callback_substitution_report(cu, registry, dialect, baseline).warnings
}

fn materialise_callback_replay_candidate(
    (callback, written_index): (&str, usize),
    inputs: &'static [tcl_registry::CallbackTaintInput],
    tokens: Option<&CommandTokens>,
    stmt: &Statement,
    source: TaintSourceContext<'_>,
    lexer_config: tcl_lexer::LexerConfig,
    registry: &CommandRegistry,
) -> Option<CallbackReplayCandidate> {
    let callback_word = tokens.and_then(|tokens| tokens.word_exprs.get(written_index + 1));
    let command_substitution = callback_word.and_then(whole_command_substitution);
    let source_is_command_substitution = command_substitution.is_some()
        || tokens
            .and_then(|tokens| tokens.argv_kinds.get(written_index + 1))
            .is_some_and(|kind| *kind == tcl_lexer::TokenType::Cmd);
    // The callback argument is evaluated once when registered.
    // Reduce static brace/quote/bare/backslash forms to that
    // runtime value before parsing it as the later callback
    // script. A whole command substitution remains source for
    // the specialised list-prefix builder below.
    let materialised_callback = if source_is_command_substitution {
        command_substitution.unwrap_or(callback).to_owned()
    } else if let Some(word) = callback_word {
        static_word_expr_value(
            word,
            lexer_config.escapes,
            tcl_syntax::word_rules::WordValueRules::from_config(&lexer_config),
        )?
    } else {
        callback.to_owned()
    };
    let callback_input_label = callback_input_label(&materialised_callback, inputs)?;
    let callback_span = tokens
        .and_then(|tokens| tokens.argv.get(written_index + 1).copied())
        .unwrap_or_else(|| stmt.span());
    let selected_builder = if source_is_command_substitution {
        let metadata = source.metadata_context()?;
        let tokens = tokens?;
        let builder = retained_callback_list_builder(registry, metadata, tokens, callback_span)?;
        Some(builder)
    } else {
        None
    };
    // Prove the callback can be materialised before choosing
    // synthetic names. The probe is never compiled or
    // evaluated, so it need not be collision-free; it only
    // exercises the parser/list-prefix policy. This prevents
    // dynamic registrations from consuming the replay cap or
    // making callback-free files scan their full source.
    callback_replay_source(
        &materialised_callback,
        inputs,
        source_is_command_substitution,
        "__tcl_lsp_callback_probe",
        "::__tcl_lsp_callback_probe",
        lexer_config,
        selected_builder.as_ref(),
    )?;
    Some(CallbackReplayCandidate {
        callback: materialised_callback,
        inputs,
        source_is_command_substitution,
        callback_span,
        callback_input_label,
        selected_builder,
    })
}

fn collect_statement_callback_candidates(
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    source: TaintSourceContext<'_>,
    instance_classes: &LocalInstanceClasses,
    stmt: &Statement,
    lexer_config: tcl_lexer::LexerConfig,
    candidates: &mut Vec<CallbackReplayCandidate>,
) {
    let (command, args, tokens) = match stmt {
        Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } => (
            stmt.canonical_command_or_source(),
            args.as_slice(),
            tokens.as_ref(),
        ),
        _ => return,
    };
    let resolved = crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
        registry,
        source.metadata,
        stmt,
    );
    if resolved.is_none() {
        return;
    }
    let effective = resolved.as_ref().map(|resolved| &resolved.effective);
    let command = resolved.as_ref().map_or(command, |resolved| {
        resolved.facts.canonical_command.as_str()
    });
    let presentations: Vec<Option<&str>> = resolved.as_ref().map_or_else(
        || {
            args.iter()
                .map(|argument| Some(argument.as_str()))
                .collect()
        },
        |resolved| {
            resolved
                .arguments
                .iter()
                .map(|argument| argument.as_deref())
                .collect()
        },
    );
    let arg_refs = presentations.iter().copied().collect::<Option<Vec<_>>>();
    for (index, callback) in presentations.iter().copied().enumerate() {
        let Some(callback) = callback else {
            continue;
        };
        let written_index = match &effective {
            Some(effective) => {
                let Some(index) = effective.written_argument(index) else {
                    continue;
                };
                index
            }
            None => index,
        };
        let direct = resolved.as_ref().map_or(&[] as &[_], |resolved| {
            resolved.with_argument_words(|words| {
                registry.callback_taint_inputs_words(words, index, dialect_to_point(dialect))
            })
        });
        let inputs = if direct.is_empty() {
            arg_refs.as_ref().map_or(&[] as &[_], |arguments| {
                callback_taint_inputs_at_call(
                    registry,
                    command,
                    arguments,
                    index,
                    dialect_to_point(dialect),
                    instance_classes,
                    tokens
                        .and_then(|tokens| tokens.words().first())
                        .map(|word| (word, source.config, stmt.span().start())),
                )
            })
        } else {
            direct
        };
        if inputs.is_empty() {
            continue;
        }
        if let Some(candidate) = materialise_callback_replay_candidate(
            (callback, written_index),
            inputs,
            tokens,
            stmt,
            source,
            lexer_config,
            registry,
        ) {
            candidates.push(candidate);
        }
    }
}

pub(super) fn collect_callback_replay_candidates(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    lexer_config: tcl_lexer::LexerConfig,
    candidates: &mut Vec<CallbackReplayCandidate>,
) {
    for fu in cu.all_body_function_units() {
        let source = TaintSourceContext::for_unit_function(registry, cu, fu);
        let instance_classes = instance_classes_for_function(
            &fu.cfg,
            registry,
            cu.interproc.as_ref(),
            !std::ptr::eq(fu, std::ptr::from_ref(&cu.top_level)),
            source,
        );
        for block in fu.cfg.blocks.values() {
            for stmt in &block.statements {
                collect_statement_callback_candidates(
                    registry,
                    dialect,
                    source,
                    &instance_classes,
                    stmt,
                    lexer_config,
                    candidates,
                );
            }
        }
    }
}

fn retain_callback_replay_report(
    cu: &crate::compilation_unit::CompilationUnit,
    baseline: &[TaintWarning],
    input_var: &str,
    replay_spans: &[(Span, Span, String)],
    insertions: &CallbackReplayInsertions,
    replay_report: TaintWarningReport,
    out: &mut TaintWarningReport,
) {
    let was_analysed = matches!(&replay_report.replays, TaintReplayAnalysis::Analysed { .. });
    let mut unavailable = Vec::new();
    for residual in replay_report.replays.unavailable_replays() {
        unavailable.extend(original_callback_replay_residuals(
            residual,
            cu,
            replay_spans,
            insertions,
        ));
    }
    if !was_analysed && unavailable.is_empty() {
        unavailable.extend(replay_spans.iter().map(|(_, registration, _)| {
            UnavailableTaintReplay {
                span: *registration,
                reason: UnavailableTaintReplayReason::SourceMetadata,
                source: Some(TaintReplaySourceSnapshot::for_module(&cu.ir_module)),
            }
        }));
    }
    out.replays.merge(if was_analysed {
        TaintReplayAnalysis::Analysed { unavailable }
    } else {
        TaintReplayAnalysis::Unavailable { unavailable }
    });
    for mut warning in replay_report.warnings {
        let in_replay = replay_spans.iter().any(|(span, _, _)| {
            warning.span.start() >= span.start() && warning.span.start() < span.end()
        });
        if !in_replay && insertions.shifts(warning.span) {
            let Some(original) = insertions.original_span(warning.span) else {
                continue;
            };
            warning.span = original;
            // Source words and fixes were taken from the injected text.
            warning.source_sink = None;
            warning.fixes.retain_mut(|fix| {
                insertions
                    .original_span(fix.span)
                    .map(|span| fix.span = span)
                    .is_some()
            });
        }
        // A prefix can call a local procedure. Its sink then lies in the
        // original procedure body even though the synthetic invocation
        // taints its parameter. Keep that new source span intact. Only a
        // sink in an appended callback script is retargeted.
        if baseline.iter().any(|existing| {
            existing.span == warning.span
                && existing.variable == warning.variable
                && existing.sink_command == warning.sink_command
                && existing.code == warning.code
        }) {
            continue;
        }
        if let Some((_, registration, callback_input_label)) =
            replay_spans.iter().find(|(span, _, _)| {
                warning.span.start() >= span.start() && warning.span.start() < span.end()
            })
        {
            if warning.variable == input_var {
                let synthetic_name = format!("Tainted variable ${input_var}");
                warning.variable.clone_from(callback_input_label);
                warning.message =
                    warning
                        .message
                        .replacen(&synthetic_name, callback_input_label, 1);
            }
            warning.span = *registration;
            // A future sink-specific fix emitted for replay text must
            // never point at the appended synthetic source.
            warning.replacement = None;
            warning.fixes.clear();
        }
        out.warnings.push(warning);
    }
}

pub(super) type CallbackReplaySpans = Vec<(Span, Span, String)>;

/// Where synthetic replay text was inserted into the original source: the
/// original offset it precedes and its length in the injected source.
#[derive(Debug, Default)]
pub(super) struct CallbackReplayInsertions(pub(super) Vec<(u32, u32)>);

impl CallbackReplayInsertions {
    /// Original offset of an injected offset outside every insertion.
    fn original_offset(&self, injected: u32, inclusive_end: bool) -> Option<u32> {
        let mut shift = 0u32;
        for &(at, len) in &self.0 {
            let start = at + shift;
            if injected < start || (inclusive_end && injected == start) {
                break;
            }
            if injected < start + len {
                return None;
            }
            shift += len;
        }
        Some(injected - shift)
    }

    pub(super) fn original_span(&self, span: Span) -> Option<Span> {
        let start = self.original_offset(span.start(), false)?;
        let end = if span.end() > span.start() {
            self.original_offset(span.end(), true)?
        } else {
            start
        };
        Some(Span::new(start, end.max(start)))
    }

    pub(super) fn shifts(&self, span: Span) -> bool {
        self.0.first().is_some_and(|&(at, _)| span.start() >= at)
    }
}

/// Insert each callback replay immediately before the top-level statement
/// that registers it. The replay is then analysed in the command world of
/// its registration point: a later dynamic registration or redefinition
/// cannot cloud its definition, and earlier user definitions still resolve.
pub(super) fn insert_callback_replays(
    source: &str,
    top_level: &[crate::ir::Statement],
    replays: &[(CallbackReplaySource, Span, String)],
) -> (String, CallbackReplaySpans, CallbackReplayInsertions) {
    let source_end = u32::try_from(source.len()).unwrap_or(u32::MAX);
    let anchor = |callback: Span| {
        top_level
            .iter()
            .map(crate::ir::Statement::span)
            .find(|span| span.start() <= callback.start() && callback.start() < span.end())
            .map_or(source_end, Span::start)
            .min(source_end)
    };
    let mut ordered: Vec<(u32, &(CallbackReplaySource, Span, String))> = replays
        .iter()
        .map(|replay| (anchor(replay.1), replay))
        .collect();
    ordered.sort_by_key(|(at, _)| *at);

    let mut injected = String::with_capacity(
        source.len()
            + replays
                .iter()
                .map(|(source, _, _)| source.definition.len() + 2)
                .sum::<usize>(),
    );
    let mut replay_spans: CallbackReplaySpans = Vec::with_capacity(replays.len());
    let mut insertions = CallbackReplayInsertions::default();
    let mut copied = 0usize;
    // Retained procedure entries share the registration world. Their
    // arbitrary effects belong to their own future executions, so one
    // replay never mutates another replay's incoming command table.
    for (at, (replay, callback_span, callback_input_label)) in ordered {
        let at_usize = at as usize;
        injected.push_str(&source[copied..at_usize]);
        copied = at_usize;
        let inserted_from = injected.len();
        if at == source_end && !injected.is_empty() {
            injected.push('\n');
        }
        let start = u32::try_from(injected.len()).unwrap_or(u32::MAX);
        injected.push_str(&replay.definition);
        let end = u32::try_from(injected.len()).unwrap_or(u32::MAX);
        if at != source_end {
            injected.push('\n');
        }
        let len = u32::try_from(injected.len() - inserted_from).unwrap_or(u32::MAX);
        insertions.0.push((at, len));
        replay_spans.push((
            Span::new(start, end),
            *callback_span,
            callback_input_label.clone(),
        ));
    }
    injected.push_str(&source[copied..]);

    (injected, replay_spans, insertions)
}

/// Build the interprocedural unit that analyses injected callback replays
/// with the original document's source analysis input.
pub(super) fn callback_replay_unit(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    injected: &str,
) -> Option<crate::compilation_unit::CompilationUnit> {
    let input =
        crate::registry_invocation::InvocationMetadataContext::for_module(registry, &cu.ir_module)
            .and_then(
                crate::registry_invocation::InvocationMetadataContext::source_analysis_input,
            )?;
    Some(
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            injected,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: cu.ir_module.lexer_config,
                dialect,
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
        .with_interprocedural(registry, dialect),
    )
}

pub(super) fn taint_callback_substitution_report(
    cu: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    baseline: &[TaintWarning],
) -> TaintWarningReport {
    let mut candidates = Vec::new();
    // Most files have no statically replayable command-substitution callback.
    let lexer_config = cu.ir_module.lexer_config;

    collect_callback_replay_candidates(cu, registry, dialect, lexer_config, &mut candidates);

    if candidates.is_empty() {
        return TaintWarningReport::analysed(Vec::new(), Vec::new());
    }

    let (input_var, proc_prefix) = fresh_callback_replay_names(&cu.source);
    let mut out = TaintWarningReport::analysed(Vec::new(), Vec::new());
    // All recoverable callbacks are replayed in bounded batches. The per-batch
    // ranges retain source mapping while bounding the amount of synthetic
    // callback code in each interprocedural solve. Every statically recoverable
    // callback is processed; batching is a work-unit bound, not a soundness cap.
    for candidates in candidates.chunks(MAX_CALLBACK_TAINT_REPLAYS) {
        let mut replays: Vec<(CallbackReplaySource, Span, String)> =
            Vec::with_capacity(candidates.len());
        for (index, candidate) in candidates.iter().enumerate() {
            let proc_name = format!("{proc_prefix}_{index}");
            let replay = callback_replay_source(
                &candidate.callback,
                candidate.inputs,
                candidate.source_is_command_substitution,
                &input_var,
                &proc_name,
                lexer_config,
                candidate.selected_builder.as_ref(),
            )
            .expect("callback replay preflight proved this script is static");
            replays.push((
                replay,
                candidate.callback_span,
                candidate.callback_input_label.clone(),
            ));
        }

        let (injected, replay_spans, insertions) =
            insert_callback_replays(&cu.source, &cu.ir_module.top_level.statements, &replays);

        #[cfg(test)]
        CALLBACK_REPLAY_BUILDS.with(|count| count.set(count.get() + 1));
        let Some(replay_cu) = callback_replay_unit(cu, registry, dialect, &injected) else {
            let residuals = replay_spans
                .iter()
                .map(|(_, registration, _)| UnavailableTaintReplay {
                    span: *registration,
                    reason: UnavailableTaintReplayReason::SourceMetadata,
                    source: Some(TaintReplaySourceSnapshot::for_module(&cu.ir_module)),
                })
                .collect();
            out.replays.merge(TaintReplayAnalysis::Unavailable {
                unavailable: residuals,
            });
            continue;
        };
        // `input_var` was chosen absent from the complete user source, so
        // seeding both scopes cannot influence the original program.
        let replay_seed = HashMap::from([
            (
                crate::naming::rooted_member_key("::", &input_var),
                TaintLattice::tainted(),
            ),
            (input_var.clone(), TaintLattice::tainted()),
        ]);
        let replay_report = taint_warning_report_for_cu_base_with_external_variable_seeds(
            &replay_cu,
            registry,
            dialect,
            Some(&replay_seed),
        );
        retain_callback_replay_report(
            cu,
            baseline,
            &input_var,
            &replay_spans,
            &insertions,
            replay_report,
            &mut out,
        );
    }
    out
}

pub(super) fn original_callback_replay_residuals(
    residual: &UnavailableTaintReplay,
    cu: &crate::compilation_unit::CompilationUnit,
    replay_spans: &[(Span, Span, String)],
    insertions: &CallbackReplayInsertions,
) -> Vec<UnavailableTaintReplay> {
    let original_end = u32::try_from(cu.source.len()).unwrap_or(u32::MAX);
    let retained = |span| UnavailableTaintReplay {
        span,
        reason: residual.reason,
        source: Some(TaintReplaySourceSnapshot::for_module(&cu.ir_module)),
    };
    if residual.span.start() <= residual.span.end()
        && let Some(original) = insertions.original_span(residual.span)
        && original.end() <= original_end
    {
        return vec![retained(original)];
    }
    if let Some((_, registration, _)) = replay_spans.iter().find(|(synthetic, _, _)| {
        synthetic.start() <= residual.span.start()
            && residual.span.start() <= residual.span.end()
            && residual.span.end() <= synthetic.end()
    }) {
        return vec![retained(*registration)];
    }
    if !replay_spans.is_empty() {
        return replay_spans
            .iter()
            .map(|(_, registration, _)| retained(*registration))
            .collect();
    }
    vec![UnavailableTaintReplay {
        source: None,
        ..residual.clone()
    }]
}
