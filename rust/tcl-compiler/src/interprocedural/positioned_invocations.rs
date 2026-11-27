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

//! Facts from registry-resolved invocations at their original positions: aliases, visible reads and writes, callback reachability and retained substitutions.

use super::body_scan::possible_body_id;
use super::call_facts::{is_plain_proc_name, mark_upvar_alias_write, scan_unproved_invocation};
use super::call_targets::command_prefix_head;
use super::parameter_observations::note_original_command_params;
use super::substitution_calls::scan_value_substitutions;
use super::{LocalFacts, ProcArgTrait, ScanCtx};
use crate::side_effects::EffectRegion;
use std::collections::{BTreeSet, HashSet};

/// Project alias identities from the same resolved transition facts as the
/// point-binding solver. The registry owns optional-level and pair grammar;
/// this consumer only records which caller parameters and visible cells a
/// transition can reach.
fn scan_resolved_aliases(
    resolved: &crate::registry_invocation::ResolvedStatementInvocation,
    defs: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    use tcl_registry::frame_effect::FrameLevel;
    use tcl_registry::state_transition::{
        CallerFrameSelection, StateTransition, TransitionSubject, VariableAliasTarget,
    };
    let parameter = |subject: &TransitionSubject| -> Option<String> {
        let TransitionSubject::Unknown { argument_index, .. } = subject else {
            return None;
        };
        let tcl_source::words::WordExpr::Variable { spelling, .. } = resolved
            .effective
            .words
            .get(argument_index.checked_add(1)?)?
        else {
            return None;
        };
        let name = tcl_syntax::naming::whole_scalar_variable_reference_name(
            spelling,
            ctx.source_module.lexer_config,
        )?;
        ctx.params.contains(name).then(|| name.to_owned())
    };
    let mut declarations = HashSet::new();
    if let Some(transitions) = resolved.facts.state_transitions.declared() {
        for fact in transitions.facts() {
            let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if let Some(local) = alias.local.literal() {
                if !alias.writes_value {
                    declarations.insert(local.to_owned());
                }
            } else if let Some(name) = parameter(&alias.local) {
                facts
                    .param_trait_flags
                    .entry(name)
                    .or_default()
                    .insert(ProcArgTrait::VarWrite);
            }
            let (visible, caller, variable) = match &alias.target {
                VariableAliasTarget::Global { variable }
                | VariableAliasTarget::CurrentNamespace { variable }
                | VariableAliasTarget::Namespace { variable, .. } => (true, false, variable),
                VariableAliasTarget::CallerSelectedFrame { frame, variable } => {
                    let level = match frame {
                        CallerFrameSelection::DefaultCaller => Some(FrameLevel::DEFAULT),
                        CallerFrameSelection::Explicit(subject) => {
                            subject.literal().and_then(|word| {
                                resolved.dialect.map_or_else(
                                    || FrameLevel::parse(word),
                                    |dialect| FrameLevel::parse_for_dialect(word, dialect),
                                )
                            })
                        }
                    };
                    (
                        level.is_none_or(|level| {
                            matches!(level, FrameLevel::Absolute(_) | FrameLevel::Dynamic)
                        }),
                        level == Some(FrameLevel::DEFAULT),
                        variable,
                    )
                }
            };
            if visible {
                match alias.local.literal() {
                    Some(local) => {
                        facts.global_aliases.insert(local.to_owned());
                    }
                    None => facts.writes_global = true,
                }
                facts.writes_global |= alias.writes_value;
            }
            if let Some(name) = parameter(variable) {
                facts
                    .param_trait_flags
                    .entry(name.clone())
                    .or_default()
                    .insert(ProcArgTrait::VarRead);
                if caller && let Some(local) = alias.local.literal() {
                    facts.upvar_aliases.insert(local.to_owned(), name);
                }
            }
        }
    }
    // Barrier-shaped calls can retain exact native write roles even when their
    // value is dynamic. Projection must not depend on structural lowering.
    let mut writes: BTreeSet<String> = defs.iter().cloned().collect();
    writes.extend(resolved.facts.arg_roles.iter().filter_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::VarWrite)
            .then(|| {
                resolved.argument_literal(resolved.facts.argument_offset + usize::from(*index))
            })
            .flatten()
    }));
    for name in writes.iter().filter(|name| !declarations.contains(*name)) {
        mark_upvar_alias_write(name, facts);
        facts.writes_global |=
            tcl_syntax::naming::is_absolute(name) || facts.global_aliases.contains(name);
    }
}

/// Project normal physical writes without licensing native execution. The
/// selected handler may still carry a compiler-error or unknown completion.
pub(super) fn scan_normal_visible_writes(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        return;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return;
    };
    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        ctx.registry,
        Some(context.as_ref().into()),
        tokens,
    ) else {
        return;
    };
    facts.writes_global |= normal
        .mutation_places(&binding.variable_context, ctx.registry)
        .iter()
        .any(|place| {
            place.cell.as_ref().is_some_and(|cell| {
                matches!(
                    cell.owner,
                    crate::place::CellOwner::Namespace(_)
                        | crate::place::CellOwner::NamespaceIdentity(_)
                )
            })
        });
}

/// Project normal physical reads of namespace storage as reads of global
/// state: `set ::seen` and `[set ::seen]` read the global as `$::seen` does
/// (#2332). An unnamed or dynamic place is an unknown read.
pub(super) fn scan_normal_visible_reads(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        return;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return;
    };
    let Some(normal) = crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
        ctx.registry,
        Some(context.as_ref().into()),
        tokens,
    ) else {
        return;
    };
    // Every named operand is a local linked to the place a parameter names
    // one frame up: what the read reaches is that place, which the caller
    // sees through the link alone.
    let roles = normal.variable_roles();
    let through_links = !roles.is_empty()
        && roles.iter().all(|&(index, _)| {
            normal
                .argument_literal(index)
                .is_some_and(|name| facts.upvar_aliases.contains_key(&name))
        });
    for place in normal.read_places(&binding.variable_context, ctx.registry) {
        let owner = place.cell.as_ref().map(|cell| &cell.owner);
        if place.dynamic || place.observed || owner.is_none() {
            if through_links {
                facts.local_pure = false;
            } else {
                facts.mark_impure();
            }
            facts.effect_reads |= EffectRegion::GLOBAL_STATE | EffectRegion::UNKNOWN_STATE;
        } else if matches!(
            owner,
            Some(
                crate::place::CellOwner::Namespace(_)
                    | crate::place::CellOwner::NamespaceIdentity(_)
            )
        ) {
            facts.mark_impure();
            facts.effect_reads |= EffectRegion::GLOBAL_STATE;
        }
    }
}

/// Candidate class metadata contributes callback reachability only. It does
/// not prove the receiver's implementation, effects or method execution.
pub(super) fn scan_candidate_method_callbacks(
    receiver: &str,
    arguments: &[&str],
    words: &[tcl_registry::InvocationWord<'_>],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(method) = words.first().and_then(|word| word.literal()) else {
        return;
    };
    let Some(classes) = ctx.object_types.get(receiver) else {
        return;
    };
    for class in classes {
        for (index, _) in ctx.registry.instance_method_command_prefixes_structured(
            class,
            method,
            &arguments[1..],
            &words[1..],
        ) {
            if let Some(word) = words
                .get(index + 1)
                .and_then(|word| word.literal())
                .and_then(|word| command_prefix_head(ctx.registry, word))
                && is_plain_proc_name(&word)
                && let Some(target) = possible_body_id(&word, ctx)
            {
                facts.direct_calls.insert(target);
            }
        }
    }
}

/// Original callback source targets contribute possible call-graph edges.
/// Whole installer, builder, frame descriptor and declaration geometry retain
/// their own obligations; receiver labels never select a callback procedure.
fn scan_positioned_candidate_callbacks(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(rows) = ctx.source_input.and_then(|input| {
        ctx.identities
            .original_source_callback_targets_for_tokens(input, tokens)
    }) else {
        return;
    };
    for row in rows {
        if let Some(id) = row.target().and_then(|target| {
            target
                .original_ir_procedure_id(ctx.source_procedures)
                .or_else(|| target.original_source_ir_procedure_id(ctx.source_module, ctx.registry))
        }) && ctx.known.contains(&id)
        {
            facts.direct_calls.insert(id);
        }
    }
}

/// Successful handler entry can contribute a possible call edge without
/// withdrawing a compiler or completion obligation from its statement.
fn scan_normal_procedure_caller(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        return;
    };
    if let Some(call) =
        crate::registry_invocation::normal_user_procedure_invocation_with_metadata_context(
            ctx.registry,
            Some(context.as_ref().into()),
            tokens,
        )
        && let Some(callee) = possible_body_id(&call.target, ctx)
    {
        // A possible successful handler entry contributes a call-graph edge.
        // Compiler/completion uncertainty below still withdraws effects and purity.
        facts.direct_calls.insert(callee);
    }
}

/// Possible call edges are independent of the later strict effect proof.
fn scan_positioned_reachability(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(binding) = &tokens.source_binding else {
        return;
    };
    scan_normal_procedure_caller(tokens, ctx, facts);
    scan_positioned_candidate_callbacks(tokens, ctx, facts);
    let source_calls = if ctx
        .source_module
        .source_metadata_input
        .as_ref()
        .is_some_and(crate::analyser::ResolvedAnalysisInput::has_logical_source_name_context)
    {
        crate::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    } else {
        crate::registry_invocation::original_authored_procedure_calls_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    };
    if let Some(calls) = source_calls {
        for call in calls {
            if call.accepts_arguments() && ctx.known.contains(&call.id()) {
                facts.direct_calls.insert(call.id());
            }
        }
    }
    if let Some(context) = ctx.metadata_context()
        && let Some((reads, writes)) =
            crate::registry_invocation::possible_normal_handler_effect_regions_with_metadata_context(
                ctx.registry,
                Some(context.as_ref().into()),
                tokens,
            )
    {
        facts.effect_reads |= reads;
        facts.effect_writes |= writes;
    }
    // Reachability depends on the target identity, independently of whether
    // a retained alias prefix has printable bytes or a composable argv shape.
    // Unknown/absent residuals still withdraw effects and purity below.
    for target in binding.execution_targets() {
        if !target.registry_backed
            && let Some((callee, _)) = ctx.source_module.source_procedure_for_target(target)
            && ctx.known.contains(&callee)
        {
            facts.direct_calls.insert(callee);
        }
    }
    // naming.compiler.original-positioned-child-body-reachability
    // docs/design/analysis/name-resolution-proofs/compiler-original-positioned-child-body-reachability.md
    // Each child retains its actual target, source offset and entered owner.
    // Re-segmenting an argument value cannot donate a nested command's roles.
    if let Some(site) = binding.invocation_site() {
        for child in ctx.identities.possible_entered_body_invocations(site) {
            for target in child.execution_targets() {
                if !target.registry_backed
                    && let Some((callee, _)) = ctx.source_module.source_procedure_for_target(target)
                    && ctx.known.contains(&callee)
                {
                    facts.direct_calls.insert(callee);
                }
            }
        }
    }
}

pub(super) fn scan_positioned_registry_facts(
    resolved: &crate::registry_invocation::ResolvedStatementInvocation,
    tokens: &crate::ir::CommandTokens,
    args: &[String],
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(context) = ctx.metadata_context() else {
        scan_unproved_invocation(args, ctx, facts);
        return;
    };
    let Some((reads, writes)) =
        crate::registry_invocation::possible_normal_handler_effect_regions_with_metadata_context(
            ctx.registry,
            Some(context.as_ref().into()),
            tokens,
        )
    else {
        scan_unproved_invocation(args, ctx, facts);
        return;
    };
    facts.effect_reads |= reads;
    facts.effect_writes |= writes;
    facts.writes_global |= writes.intersects(EffectRegion::GLOBAL_STATE);
    let traits = resolved.facts.traits;
    if traits
        .intersects(tcl_registry::Traits::EVALUATES_CODE | tcl_registry::Traits::CREATES_BARRIER)
    {
        scan_unproved_invocation(args, ctx, facts);
        return;
    }
    if !resolved.facts.evaluates_purely() || !writes.is_empty() {
        facts.mark_impure();
    }
    // Targets and source incidence retain independent original owners.
    note_original_command_params(ctx, ProcArgTrait::Passthrough, facts);
}

/// Consume positioned implementation facts before assistance projections.
pub(super) fn scan_positioned_invocation(
    stmt: &crate::ir::Statement,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) -> bool {
    use crate::ir::Statement;
    let (args, defs, tokens) = match stmt {
        Statement::Call {
            args,
            defs,
            tokens: Some(tokens),
            ..
        } => (args, defs.as_slice(), tokens),
        Statement::Barrier {
            args,
            tokens: Some(tokens),
            ..
        } => (args, &[][..], tokens),
        _ => return false,
    };
    let Some(binding) = &tokens.source_binding else {
        return false;
    };
    scan_positioned_reachability(tokens, ctx, facts);
    let Some(target) = binding.proved_execution_target() else {
        scan_unproved_invocation(args, ctx, facts);
        return true;
    };
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        scan_unproved_invocation(args, ctx, facts);
        return true;
    };
    let Some(_effective_args) = effective.argument_presentations(args) else {
        scan_unproved_invocation(args, ctx, facts);
        return true;
    };
    // The exact point binding supersedes a whole-document assistance join.
    let positioned = ScanCtx {
        identities: crate::realm::CommandBindingRealm::none(),
        positioned_substitutions: true,
        current_tokens: Some(tokens),
        ..ctx
    };
    if target.registry_backed {
        let Some(context) = ctx.metadata_context() else {
            scan_unproved_invocation(args, positioned, facts);
            return true;
        };
        let Some(resolved) =
            crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
                ctx.registry,
                Some(context.as_ref().into()),
                stmt,
            )
        else {
            scan_unproved_invocation(args, ctx, facts);
            return true;
        };
        scan_resolved_aliases(&resolved, defs, positioned, facts);
        scan_normal_visible_reads(tokens, positioned, facts);
        if let Some(presented) = resolved
            .arguments
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
        {
            scan_positioned_registry_facts(&resolved, tokens, &presented, positioned, facts);
        } else {
            // The target is proved, but a legacy textual effect/callback
            // query cannot invent bytes for a captured unmaterialised value.
            scan_unproved_invocation(args, positioned, facts);
        }
    } else if let Some(target) = possible_body_id(&target.command, ctx) {
        facts.procedure_calls.push((target, Some(args.clone())));
        facts.direct_calls.insert(target);
        note_original_command_params(positioned, ProcArgTrait::ForwardedToCallee, facts);
        for arg in args {
            scan_value_substitutions(arg, ctx, facts, 0);
        }
    } else {
        scan_unproved_invocation(args, ctx, facts);
    }
    if matches!(stmt, Statement::Barrier { .. }) {
        facts.has_barrier = true;
        facts.local_pure = false;
        facts.effect_reads |= EffectRegion::UNKNOWN_STATE;
        facts.effect_writes |= EffectRegion::UNKNOWN_STATE;
    }
    true
}

pub(super) fn scan_retained_substitutions<'a>(
    tokens: Option<&crate::ir::CommandTokens>,
    ctx: ScanCtx<'a>,
    facts: &mut LocalFacts,
) -> ScanCtx<'a> {
    use crate::ir::Statement;
    if let Some(tokens) = tokens.filter(|tokens| tokens.source_binding.is_some()) {
        let positioned = ScanCtx {
            positioned_substitutions: true,
            ..ctx
        };
        for lifted in crate::word_subst::lifted_calls_with_surface(
            Some(tokens),
            tcl_lexer::LexerConfig::for_profile(ctx.dialect),
            &ctx.surface(),
        ) {
            let nested = Statement::Call {
                span: lifted.span,
                command: lifted.command,
                canonical_command: None,
                args: lifted.args,
                defs: Vec::new(),
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: lifted.tokens,
                foreach_groups: None,
            };
            if !scan_positioned_invocation(&nested, positioned, facts) {
                // Missing retained provenance cannot authorise a lexical query.
                if let Statement::Call { args, .. } = &nested {
                    scan_unproved_invocation(args, positioned, facts);
                }
            }
        }
        positioned
    } else {
        ctx
    }
}

pub(super) fn scan_original_variable_read_effects(
    tokens: &crate::ir::CommandTokens,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    use crate::place::CellOwner;
    let unknown_read = |facts: &mut LocalFacts| {
        facts.mark_impure();
        facts.effect_reads |= EffectRegion::GLOBAL_STATE | EffectRegion::UNKNOWN_STATE;
    };
    let Some(input) = ctx.source_input else {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    };
    let Some((binding, site, config)) = tokens.source_binding.as_ref().and_then(|binding| {
        Some((
            binding,
            binding.invocation_site()?,
            binding.original_lexer_config_for_tokens(tokens)?,
        ))
    }) else {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    };
    let Some(image) = ctx.identities.original_source_image() else {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    };
    let original = ctx.identities.invocation_at_source("", site.offset);
    let corresponding = ctx.identities.matches_resolved_analysis_input(input)
        && input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            == ctx.registry.snapshot().semantic_key()
        && ctx
            .identities
            .matches_original_source_image(image, input.lexer_config())
        && site.source.source_image() == image
        && config == input.lexer_config()
        && binding
            .original_invocation_metadata_for_input(tokens, input, ctx.registry)
            .is_some()
        && original.invocation_site() == Some(site)
        && original.original_lexer_config_for_tokens(tokens) == Some(config)
        && original
            .original_recorded_command()
            .is_some_and(|(_, original)| original.words() == tokens.words());
    if !corresponding {
        if !tokens.variable_accesses.is_empty() {
            unknown_read(facts);
        }
        return;
    }
    for access in ctx
        .identities
        .source_bindings_ref()
        .variable_accesses_during_expression_invocation(site.offset)
    {
        if access.context_alternatives().is_empty()
            || access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
        {
            unknown_read(facts);
        }
        for context in access.context_alternatives() {
            let place = access.place_in_context(context, ctx.registry);
            if place.dynamic || place.observed {
                unknown_read(facts);
                continue;
            }
            match place.cell.as_ref().map(|cell| &cell.owner) {
                Some(CellOwner::CurrentFrame) => {}
                Some(CellOwner::Activation(owner))
                    if context.activation.as_ref() == Some(owner) => {}
                Some(CellOwner::Namespace(_) | CellOwner::NamespaceIdentity(_)) => {
                    facts.mark_impure();
                    facts.effect_reads |= EffectRegion::GLOBAL_STATE;
                }
                _ => unknown_read(facts),
            }
        }
    }
}
