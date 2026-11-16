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

//! The calls a body makes to known procedures: argument-taint flows, resolved callees and command-substitution callees.

use super::ProcTaintSummary;
use super::summary_inference::return_ctx;
use crate::cfg::Terminator;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::ProcedureId;
use crate::ssa::{Symbol, ValueKey};
use crate::taint::{LocalInstanceClasses, TaintCtx, TaintLattice, TaintSourceContext, word_taint};
use std::collections::{HashMap, HashSet};
use tcl_lexer::SourceMap;
use tcl_registry::CommandRegistry;

// Call-flow resolution (entry-taint worklist)

struct CallFlowInputs<'a> {
    pub(super) ctx: TaintCtx<'a>,
    pub(super) taints: &'a HashMap<ValueKey, TaintLattice>,
    pub(super) source: TaintSourceContext<'a>,
    module: &'a crate::ir::Module,
    registry: &'a CommandRegistry,
    known: &'a HashSet<crate::ir::ProcedureId>,
    summaries: &'a HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
}

fn collect_call_flows_for_tokens(
    inputs: &CallFlowInputs<'_>,
    tokens: &crate::ir::CommandTokens,
    uses: &HashMap<Symbol, u32>,
    position: u32,
    flows: &mut Vec<(crate::ir::ProcedureId, Vec<TaintLattice>)>,
) {
    let mut invocations = vec![tokens.clone()];
    let lifted = inputs.source.metadata_context().and_then(|metadata| {
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            inputs.source.lexer_config(),
            inputs.registry,
            metadata,
        )
    });
    if let Some(lifted) = lifted {
        invocations.extend(lifted.into_iter().filter_map(|call| call.tokens));
    }
    for invocation in invocations {
        let Some(head) = original_head(&invocation) else {
            continue;
        };
        let Some(calls) =
            crate::registry_invocation::original_source_procedure_calls_for_word_for_module(
                &invocation,
                &head,
                inputs.module,
                inputs.registry,
            )
        else {
            continue;
        };
        for call in calls {
            if !inputs.known.contains(&call.id()) {
                continue;
            }
            let Some(summary) = inputs.summaries.get(&call.id()) else {
                continue;
            };
            let Some(count) = call.argument_count() else {
                flows.push((
                    call.id(),
                    vec![TaintLattice::tainted(); summary.params.len()],
                ));
                continue;
            };
            if !summary
                .arity
                .accepts(u16::try_from(count).unwrap_or(u16::MAX))
            {
                continue;
            }
            let effective = call.effective_arguments();
            let arguments = effective
                .words
                .iter()
                .skip(1)
                .enumerate()
                .map(|(argument, _word)| match effective.origins.get(argument + 1) {
                    Some(crate::registry_invocation::InvocationWordOrigin::Written(index)) => {
                        let Some(original) = invocation.words().get(*index) else {
                            return TaintLattice::tainted();
                        };
                        if matches!(
                            original,
                            crate::ir::WordExpr::Literal { .. }
                                | crate::ir::WordExpr::BracedLiteral { .. }
                        ) {
                            return TaintLattice::clean();
                        }
                        let argument_context = TaintCtx {
                            source_tokens: Some(&invocation),
                            source_word: Some(original),
                            word_offset: Some(original.source().span.start()),
                            ..inputs.ctx.at(position)
                        };
                        word_taint(
                            &original.legacy_text(),
                            uses,
                            inputs.taints,
                            argument_context,
                        )
                    }
                    Some(crate::registry_invocation::InvocationWordOrigin::BindingPrefix(index)) => {
                        match effective.binding_prefix.get(*index) {
                            Some(
                                crate::registry_invocation::EffectiveInvocationWord::Literal(_)
                                | crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(_),
                            ) => TaintLattice::clean(),
                            _ => TaintLattice::tainted(),
                        }
                    }
                    _ => TaintLattice::tainted(),
                })
                .collect();
            flows.push((call.id(), arguments));
        }
    }
}

/// Resolve the (callee, arg-taints) flows a function makes to known procs.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_call_flows(
    fu: &FunctionUnit,
    taints: &HashMap<ValueKey, TaintLattice>,
    instance_classes: &LocalInstanceClasses,
    registry: &CommandRegistry,
    interproc: Option<&crate::interprocedural::InterproceduralAnalysis>,
    source: TaintSourceContext<'_>,
    known: &HashSet<crate::ir::ProcedureId>,
    summaries: &HashMap<crate::ir::ProcedureId, ProcTaintSummary>,
) -> Vec<(crate::ir::ProcedureId, Vec<TaintLattice>)> {
    let ctx = return_ctx(
        fu,
        registry,
        interproc,
        source,
        known,
        summaries,
        instance_classes,
    );
    let mut flows = Vec::new();
    let Some(module) = source.module() else {
        return flows;
    };
    let inputs = CallFlowInputs {
        ctx,
        taints,
        source,
        module,
        registry,
        known,
        summaries,
    };
    for bn in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(bn) else {
            continue;
        };
        let Some(ssa_block) = fu.ssa.blocks.get(bn) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            let Some(ssa_statement) = ssa_block.statements.get(index) else {
                continue;
            };
            let Some(tokens) = fu.cfg.source_input_tokens_at(*bn, index) else {
                continue;
            };
            collect_call_flows_for_tokens(
                &inputs,
                tokens,
                &ssa_statement.uses,
                statement.span().start(),
                &mut flows,
            );
        }
        if let Some(Terminator::Return {
            tokens: Some(tokens),
            span,
            ..
        }) = &block.terminator
        {
            collect_call_flows_for_tokens(
                &inputs,
                tokens,
                &ssa_block.exit_versions,
                span.map_or(0, tcl_lexer::Span::start),
                &mut flows,
            );
        }
    }
    flows
}

pub(super) fn original_head(tokens: &crate::ir::CommandTokens) -> Option<String> {
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let words = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    let head = tcl_syntax::word_rules::original_static_word_source_bytes(words.first()?)?;
    String::from_utf8(head).ok()
}

/// The known-proc callees a function references — every call target
/// [`infer_proc_summary`] resolves while propagating taints, scanned from the
/// same executable CFG blocks (`Statement::Call` + command-substitution
/// `AssignValue`) as [`resolve_call_flows`].
///
/// Unlike [`crate::interprocedural::ProcSummary::direct_calls`] — which misses a
/// call buried in a nested command substitution under a dynamic command (e.g.
/// `symbolNodeOf` in `[$t get [symbolNodeOf …] …]`) — this captures exactly the
/// callee summaries the inference reads.  The LSP summary-cascade memo keys on
/// this complete set so its reconstructed dependency context matches the
/// whole-module solve (and the debug fixpoint guard) instead of seeding a missed
/// callee clean and under-tainting the result.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn resolved_callees(
    cu: &CompilationUnit,
    body_id: Option<crate::ir::ProcedureId>,
    known: &HashSet<crate::ir::ProcedureId>,
    registry: &CommandRegistry,
) -> Vec<crate::ir::ProcedureId> {
    let fu = cu.function_for_body(body_id);
    let Some(fu) = fu else {
        return known.iter().copied().collect();
    };
    let source = TaintSourceContext::for_unit_may(registry, cu, fu);
    let mut out = std::collections::BTreeSet::new();
    for block_id in &fu.sccp.executable_blocks {
        let Some(block) = fu.cfg.blocks.get(block_id) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            let invokes_head = matches!(
                statement,
                crate::ir::Statement::Call { .. }
                    | crate::ir::Statement::Barrier { .. }
                    | crate::ir::Statement::NativeCall { .. }
            );
            let Some(tokens) = fu.cfg.source_input_tokens_at(*block_id, index) else {
                if invokes_head {
                    out.extend(known.iter().copied());
                }
                continue;
            };
            if invokes_head {
                if let Some(head) = original_head(tokens) {
                    if let Some(dependencies) = crate::registry_invocation::original_procedure_may_dependencies_for_word_for_module(tokens, &head, &cu.ir_module, registry) {
                    out.extend(dependencies.bodies.into_iter().filter(|id| known.contains(id)));
                    if dependencies.unknown_alternatives { out.extend(known.iter().copied()); }
                } else {
                    out.extend(known.iter().copied());
                }
                } else {
                    out.extend(known.iter().copied());
                }
            }
            let lifted = source.metadata_context().and_then(|metadata| {
                crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                    tokens,
                    source.lexer_config(),
                    registry,
                    metadata,
                )
            });
            if let Some(lifted) = lifted {
                for call in lifted {
                    let Some(child) = call.tokens else {
                        out.extend(known.iter().copied());
                        continue;
                    };
                    let selected = original_head(&child)
                        .and_then(|head| crate::registry_invocation::original_procedure_may_dependencies_for_word_for_module(&child, &head, &cu.ir_module, registry));
                    if let Some(dependencies) = selected {
                        out.extend(
                            dependencies
                                .bodies
                                .into_iter()
                                .filter(|id| known.contains(id)),
                        );
                        if dependencies.unknown_alternatives {
                            out.extend(known.iter().copied());
                        }
                    } else {
                        out.extend(known.iter().copied());
                    }
                }
            } else if tokens
                .words()
                .iter()
                .any(|word| word.legacy_text().contains('['))
            {
                out.extend(known.iter().copied());
            }
        }
        if let Some(Terminator::Return {
            value: Some(value), ..
        }) = &block.terminator
            && value.contains('[')
        {
            out.extend(known.iter().copied());
        }
    }
    out.into_iter().collect()
}

fn substitution_body_span(base: usize, length: usize) -> Option<tcl_lexer::Span> {
    let end = base
        .checked_add(length)
        .and_then(|end| u32::try_from(end).ok())?;
    let start = u32::try_from(base).ok()?;
    Some(tcl_lexer::Span::new(start, end))
}

fn original_substitution_body<'a>(
    cu: &'a CompilationUnit,
    body_id: ProcedureId,
    registry: &CommandRegistry,
) -> Option<(
    &'a str,
    usize,
    &'a crate::command_binding::SourceCommandBindings,
)> {
    let module = &cu.ir_module;
    let procedure = cu.original_procedure_for_body(body_id)?;
    let retained = module.retained_source_bindings.as_deref()?;
    if !retained.matches_original_procedure_summary(body_id, procedure)
        || !module
            .original_source_procedure_bodies()
            .any(|(id, current)| *id == body_id && std::ptr::eq(current, procedure))
        || module.source.try_text().ok()? != cu.source
    {
        return None;
    }
    let bindings = retained.positioned_for_module(module, registry)?;
    let body = procedure.body_source.as_deref()?;
    let base = usize::try_from(procedure.body_offset).ok()?;
    if cu.source.get(base..base.checked_add(body.len())?) != Some(body) {
        return None;
    }
    Some((body, base, bindings))
}

/// Memo dependencies of original substitutions in one Module-owned body.
/// Unresolved or incomplete source retains every candidate; this selects no call edge.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn command_subst_callees(
    cu: &CompilationUnit,
    body_id: ProcedureId,
    known: &HashSet<ProcedureId>,
    registry: &CommandRegistry,
) -> Vec<ProcedureId> {
    let module = &cu.ir_module;
    let mut dependencies = std::collections::BTreeSet::new();
    let original = original_substitution_body(cu, body_id, registry);
    let Some((body, base, bindings)) = original else {
        return known.iter().copied().collect();
    };
    let config = module.lexer_config.nested();
    let profile = module
        .source_metadata_input
        .as_ref()
        .map(crate::analyser::ResolvedAnalysisInput::analyser_profile)
        .or(module.dialect_profile);
    let Some(span) = substitution_body_span(base, body.len()) else {
        return known.iter().copied().collect();
    };
    let mut pending = vec![(span, false, 0)];
    let mut seen = HashSet::new();
    while let Some((span, substitution, depth)) = pending.pop() {
        if !seen.insert((span, substitution)) {
            continue;
        }
        if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            dependencies.extend(known.iter().copied());
            continue;
        }
        let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
            module.source.clone(),
            span,
            config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        ) else {
            dependencies.extend(known.iter().copied());
            continue;
        };
        if plan.fatal_tail.is_some() {
            dependencies.extend(known.iter().copied());
        }
        for command in plan.commands {
            let Some(inner) = cu.source.get(command.span.as_range()) else {
                dependencies.extend(known.iter().copied());
                continue;
            };
            let offset = command.span.start();
            let commands =
                crate::segmenter::segment_commands_with_offset_and_config(inner, offset, config);
            let [segment] = commands.as_slice() else {
                dependencies.extend(known.iter().copied());
                continue;
            };
            if segment.is_partial {
                dependencies.extend(known.iter().copied());
                continue;
            }
            let map = SourceMap::new(inner).with_base(offset, 0, 0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, segment);
            bindings.stamp_original_tokens(&mut tokens);
            if substitution {
                let ids = original_head(&tokens).and_then(|head| {
                    crate::registry_invocation::original_procedure_may_dependencies_for_word_for_module(
                        &tokens, &head, module, registry,
                    )
                });
                if let Some(selected) = ids {
                    dependencies
                        .extend(selected.bodies.into_iter().filter(|id| known.contains(id)));
                    if selected.unknown_alternatives {
                        dependencies.extend(known.iter().copied());
                    }
                } else {
                    dependencies.extend(known.iter().copied());
                }
            }
            if let Some(regions) = crate::word_subst::original_source_dependency_regions(
                &tokens,
                &command.words,
                registry,
                profile,
            ) {
                if regions.unknown {
                    dependencies.extend(known.iter().copied());
                }
                pending.extend(
                    regions
                        .substitutions
                        .into_iter()
                        .map(|span| (span, true, depth + 1)),
                );
                pending.extend(
                    regions
                        .scripts
                        .into_iter()
                        .map(|span| (span, substitution, depth + 1)),
                );
            } else {
                dependencies.extend(known.iter().copied());
            }
        }
    }
    dependencies.into_iter().collect()
}
