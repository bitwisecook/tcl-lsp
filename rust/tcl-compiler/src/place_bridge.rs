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

//! Bridge from the IR/CFG to the unified [`Place`] model.
//!
//! Resolves each IR statement's *def* and *read* targets to canonical
//! [`Place`]s on demand, so the dataflow consumers (dead-store / unused /
//! read-before-set) can switch from name-string equality to the sound
//! [`overlap`] relation — without re-stamping every
//! construction site in lowering.  The per-function [`ResolveContext`] is built
//! at each CFG site from registry transitions. Consumers share the same
//! point-specific binding and lifetime state rather than rescanning declarations.

use tcl_registry::{ArgRole, CommandRegistry, Traits};

use crate::cfg::{Function, Terminator};
use crate::ir::{CommandTokens, Statement};
use crate::place::{self, Place, PlaceKind, overlap, places_read_to_form};
use crate::segmenter::segment_commands_with_offset_and_config;
use crate::ssa::structural_body_indices;
use crate::var_refs::{
    command_subst_texts_with_config, scan_var_ref_forms_braced_with_config, vars_in_expr,
};
use crate::var_resolve::ResolveContext;
#[cfg(test)]
use crate::var_resolve::resolve_place;

/// True when `name`'s `VarRead`-role argument names the *whole* array,
/// not a scalar (so a write to any element is observed) — the registry's
/// [`Traits::WHOLE_ARRAY_ARG`] (`array` / `parray`). `registry.get`
/// resolves the bare and `::`-qualified forms alike.
fn is_whole_array_command(registry: &CommandRegistry, name: &str) -> bool {
    registry
        .get(name)
        .is_some_and(|s| s.traits.contains(Traits::WHOLE_ARRAY_ARG))
}

pub use crate::variable_bindings::{
    PointResolveContexts, build_point_resolve_contexts, build_point_resolve_contexts_with_entry,
};

/// Conservative whole-function exposure projection for compatibility readers.
/// Precise accesses use `build_point_resolve_contexts` at their own CFG site.
#[must_use]
pub fn build_resolve_context(cfg: &Function, fn_qname: &str) -> ResolveContext {
    let registry = CommandRegistry::build_default();
    let points = build_point_resolve_contexts(cfg, fn_qname, &registry);
    let mut result = ResolveContext::for_function(fn_qname);
    for (&id, block) in &cfg.blocks {
        result.join(points.before_terminator(id));
        for index in 0..block.statements.len() {
            result.join(points.before_statement(id, index));
        }
    }
    result
}

/// True when arg `arg_index` of a command is a **braced literal** word
/// (`{…}`): Tcl performs no `$` / `[` substitution on it, so its de-braced IR
/// text must NOT be scanned for reads — `puts {$a(k)}` does *not* read `a(k)`.
///
/// Thin wrapper over the shared
/// [`CommandTokens::arg_is_braced_literal`] for the `Option<&CommandTokens>`
/// this module carries.
fn is_braced_literal(tokens: Option<&CommandTokens>, arg_index: usize) -> bool {
    tokens.is_some_and(|t| t.arg_is_braced_literal(arg_index))
}

/// Bound contents stores and destructions, including unset and namespace deletion.
/// This projection is separate from SSA value definitions: deletion supplies no value.
#[must_use]
pub fn statement_mutation_places(
    statement: &Statement,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    if statement.has_opaque_native_accesses() {
        return vec![place::unknown_top()];
    }
    if let Some(effects) =
        crate::dictionary_bindings::scope_marker_effects(statement, context, registry)
    {
        return effects
            .writes
            .into_iter()
            .chain(effects.destructions)
            .chain(effects.clobbers)
            .collect();
    }
    let Some(tokens) = statement.tokens() else {
        return def_places(statement, context, registry);
    };
    if tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments) {
        return Vec::new();
    }
    if matches!(
        tokens.synthetic,
        Some(crate::ir::SyntheticMarker::IterationBindings(_))
    ) {
        return def_places(statement, context, registry);
    }
    let semantic_context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    crate::registry_invocation::normal_transfer_invocation(registry, semantic_context, tokens)
        .map_or_else(
            || vec![place::unknown_top()],
            |invocation| invocation.mutation_places(context, registry),
        )
}

/// Bound stores and destructions at their authored operand-selection phase.
/// Body output operands may select a different binding on normal continuation.
#[must_use]
pub fn statement_mutation_places_with_continuation(
    statement: &Statement,
    before: &ResolveContext,
    after: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    if let Some(effects) =
        crate::dictionary_bindings::scope_marker_effects(statement, before, registry)
    {
        return effects
            .writes
            .into_iter()
            .chain(effects.destructions)
            .chain(effects.clobbers)
            .collect();
    }
    let context = match variable_operand_binding_phase(statement, registry) {
        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments => before,
        tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation => after,
        tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol => {
            return vec![place::unknown_top()];
        }
    };
    statement_mutation_places(statement, context, registry)
}

/// Physical destruction targets on a successful handler continuation.
/// Destruction supplies no contents value; unknown targets remain may clobbers.
#[must_use]
pub fn statement_destruction_places_with_continuation(
    statement: &Statement,
    before: &ResolveContext,
    after: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    if let Some(effects) =
        crate::dictionary_bindings::scope_marker_effects(statement, before, registry)
    {
        return effects.destructions;
    }
    let mut targets =
        crate::variable_bindings::namespace_destruction_places(statement, before, registry);
    let Some(tokens) = statement.tokens() else {
        return targets;
    };
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    if crate::registry_invocation::normal_transfer_invocation(registry, context, tokens)
        .is_some_and(|invocation| {
            invocation
                .variable_traits()
                .contains(Traits::DESTROYS_VARIABLE)
        })
    {
        targets.extend(statement_mutation_places_with_continuation(
            statement, before, after, registry,
        ));
    }
    targets
}

fn variable_operand_binding_phase(
    statement: &Statement,
    registry: &CommandRegistry,
) -> tcl_registry::native_compilation::VariableOperandBindingPhase {
    use tcl_registry::native_compilation::VariableOperandBindingPhase;
    let Some(tokens) = statement.tokens() else {
        return VariableOperandBindingPhase::AfterArguments;
    };
    let semantic_context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    crate::registry_invocation::normal_transfer_invocation(registry, semantic_context, tokens)
        .map_or(VariableOperandBindingPhase::AfterArguments, |invocation| {
            invocation.variable_binding_phase()
        })
}

/// SSA contents transitions caused by destruction, separate from value stores.
/// The boolean marks original declaration advice: it can kill a diagnostic
/// version on that conditional interpretation, but establishes no Must effect.
#[must_use]
pub(crate) fn ssa_destruction_keys(
    statement: &Statement,
    before: &ResolveContext,
    after: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<(crate::var_resolve::VariableCellKey, bool)> {
    let actual: Vec<_> =
        statement_destruction_places_with_continuation(statement, before, after, registry)
            .iter()
            .filter_map(crate::var_resolve::canonical_binding_value_key)
            .map(|key| (key, false))
            .collect();
    if !actual.is_empty() || statement.has_opaque_native_accesses() {
        return actual;
    }
    let Some(tokens) = statement.tokens() else {
        return actual;
    };
    let Some(advice) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.declaration_operand_layout_advice(tokens))
    else {
        return actual;
    };
    let original = ResolveContext::default().in_frame(advice.frame());
    if !advice.closed_lookup()
        || original.frame_kind != before.frame_kind
        || original.namespace != before.namespace
        || original.namespace_identity != before.namespace_identity
        || !(original.global_frame() && before.global_frame()
            || original.activation.is_some() && original.activation == before.activation)
    {
        return actual;
    }
    crate::registry_invocation::declaration_invocation_flow(registry, tokens, &advice)
        .into_iter()
        .flat_map(|flow| flow.removes)
        .filter_map(|name| {
            crate::var_resolve::canonical_literal_variable_key(&name, before, registry)
        })
        .map(|key| (key, true))
        .collect()
}

/// The places *stmt* defines a value in (element-granular).
#[must_use]
pub fn def_places(
    stmt: &Statement,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    if stmt.has_opaque_native_accesses() {
        return Vec::new();
    }
    if let Some(effects) = crate::dictionary_bindings::scope_marker_effects(stmt, ctx, registry) {
        return effects.writes;
    }
    if stmt.tokens().is_some_and(|tokens| {
        tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
    }) {
        return Vec::new();
    }
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            vec![crate::var_resolve::resolve_target_access(
                name,
                *name_braced,
                ctx,
                registry,
                tcl_registry::TraceOperation::Write,
            )]
        }
        Statement::Call {
            defs,
            tokens: Some(tokens),
            ..
        } if matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        ) =>
        {
            defs.iter()
                .map(|name| {
                    crate::var_resolve::resolve_literal_access(
                        name,
                        ctx,
                        false,
                        registry,
                        tcl_registry::TraceOperation::Write,
                    )
                })
                .collect()
        }
        Statement::Call { .. } => invocation_def_places(stmt, ctx, registry),
        _ => Vec::new(),
    }
}

fn invocation_def_places(
    stmt: &Statement,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    let Some(invocation) = stmt.tokens().and_then(|tokens| {
        crate::registry_invocation::normal_transfer_invocation(registry, context, tokens)
    }) else {
        return vec![place::unknown_top()];
    };
    invocation.definition_places(ctx, registry)
}

pub(crate) fn resolved_invocation_def_places_with_output_order(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    output_order: Option<&[usize]>,
) -> Vec<Place> {
    resolved_invocation_variable_definitions(invocation, ctx, registry, output_order)
        .into_iter()
        .map(|(_, place)| place)
        .collect()
}

pub(crate) fn resolved_invocation_variable_definitions(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    output_order: Option<&[usize]>,
) -> Vec<(usize, Place)> {
    let facts = invocation.facts.as_ref();
    if facts.traits.contains(Traits::DESTROYS_VARIABLE) {
        return Vec::new();
    }

    let words: Vec<_> = (0..invocation.arguments.len())
        .map(|index| invocation.argument_word(index))
        .collect();
    let arguments: Vec<_> = words
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments = tcl_registry::InvocationArguments::structured(&arguments);
    let uncertain = crate::variable_bindings::uncertain_variable_output_addresses(
        facts,
        arguments,
        ctx,
        registry,
        output_order,
    );
    facts
        .arg_roles
        .iter()
        .filter_map(|&(index, role)| {
            let index = facts.argument_offset + usize::from(index);
            if role != ArgRole::VarWrite
                || !crate::variable_bindings::contents_write_operand(
                    facts,
                    arguments.literal_at(index),
                )
            {
                return None;
            }
            Some((
                index,
                if uncertain.contains(&index) {
                    crate::var_resolve::project_access(
                        place::unknown_top(),
                        ctx,
                        tcl_registry::TraceOperation::Write,
                    )
                } else {
                    crate::variable_bindings::variable_output_operand_access(
                        facts, arguments, index, ctx, registry,
                    )
                },
            ))
        })
        .collect()
}

/// Resolve definitions at their authored variable operand-selection phase.
/// A literal write can also re-establish a root lifetime lost at an incoming join.
#[must_use]
pub fn def_places_with_continuation(
    stmt: &Statement,
    before: &ResolveContext,
    after: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    if let Some(effects) = crate::dictionary_bindings::scope_marker_effects(stmt, before, registry)
    {
        return effects.writes;
    }
    if variable_operand_binding_phase(stmt, registry)
        == tcl_registry::native_compilation::VariableOperandBindingPhase::NormalContinuation
    {
        return def_places(stmt, after, registry);
    }
    let after_places = def_places(stmt, after, registry);
    def_places(stmt, before, registry)
        .into_iter()
        .map(|place| {
            if place
                .cell
                .as_ref()
                .is_some_and(|cell| cell.generation == crate::place::CellGeneration::Unknown)
                && let Some(continued) = after_places.iter().find(|continued| {
                    continued.ns == place.ns
                        && continued.name == place.name
                        && continued.index == place.index
                        && continued
                            .cell
                            .as_ref()
                            .zip(place.cell.as_ref())
                            .is_some_and(|(left, right)| left.owner == right.owner)
                })
            {
                return continued.clone();
            }
            place
        })
        .collect()
}

/// Resolve the reads of every command in `script_text` — VAR_READ-role name
/// args (`array get arr` reads whole array `arr`; `info exists x` reads `x`)
/// plus a recursive descent of each arg word.
fn command_reads(
    script_text: &str,
    ctx: &ResolveContext,
    out: &mut Vec<Place>,
    registry: &CommandRegistry,
) {
    // The registry carries the environment's profile, so the embedded script
    // is segmented under the document's own grammar.
    for cmd in segment_commands_with_offset_and_config(
        script_text,
        0,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    ) {
        let name = cmd.name();
        if name.is_empty() {
            continue;
        }
        let args = cmd.args();
        let whole = is_whole_array_command(registry, name);
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        for i in registry.arg_indices_for_role(name, &arg_strs, ArgRole::VarRead) {
            if let Some(a) = args.get(i) {
                if !a.starts_with('$') && !a.starts_with('[') {
                    out.push(crate::var_resolve::resolve_access(
                        a,
                        ctx,
                        whole,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    ));
                } else {
                    // Dynamic VAR_READ name (`array get $arr_name`): the read
                    // target is computed → UNKNOWN (overlaps everything) so a
                    // write to any array is not falsely flagged dead.
                    out.push(place::unknown_top());
                }
            }
        }
        word_reads(name, ctx, out, registry);
        for arg in args {
            word_reads(arg, ctx, out, registry);
        }
    }
}

/// Resolve reads in a single word: `$`-refs (precise, with array index) at the
/// top level, and any `[...]` command substitution recursively.
fn word_reads(text: &str, ctx: &ResolveContext, out: &mut Vec<Place>, registry: &CommandRegistry) {
    if text.is_empty() {
        return;
    }
    // The registry carries the environment's profile, so the `$`/`[…]`
    // boundaries this word is read at are the document's own.
    let config = ctx.invocation_dialect.map_or_else(
        || tcl_lexer::LexerConfig::for_profile(registry.profile()),
        |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    );
    if text.contains('$') {
        for (reference, braced) in scan_var_ref_forms_braced_with_config(text, config) {
            let access = if braced {
                crate::var_resolve::resolve_literal_access(
                    &reference,
                    ctx,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                )
            } else {
                crate::var_resolve::resolve_access(
                    &reference,
                    ctx,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                )
            };
            out.push(access);
        }
    }
    for inner in command_subst_texts_with_config(text, config) {
        command_reads(&inner, ctx, out, registry);
    }
}

fn invocation_reads(
    stmt: &Statement,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
    out: &mut Vec<Place>,
) {
    let (Statement::Call {
        command,
        args,
        tokens,
        ..
    }
    | Statement::Barrier {
        command,
        args,
        tokens,
        ..
    }) = stmt
    else {
        return;
    };
    if tokens
        .as_ref()
        .is_some_and(|tokens| !tokens.evaluates_words())
    {
        return;
    }
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    let invocation =
        crate::registry_invocation::resolved_statement_invocation(registry, context, stmt);
    let facts = invocation
        .as_ref()
        .map(|invocation| invocation.facts.as_ref());
    let effective = invocation.as_ref().map(|invocation| &invocation.effective);
    let arguments_only = tokens.as_ref().is_some_and(|tokens| {
        tokens.synthetic == Some(crate::ir::SyntheticMarker::EvaluatedArguments)
    });
    let mut evaluated = std::collections::HashSet::new();
    if !arguments_only && let (Some(facts), Some(effective)) = (facts, effective) {
        let whole = facts.traits.contains(Traits::WHOLE_ARRAY_ARG);
        for &(index, role) in &facts.arg_roles {
            let index = facts.argument_offset + usize::from(index);
            match role {
                ArgRole::VarRead => {
                    let value = invocation
                        .as_ref()
                        .and_then(|invocation| invocation.argument_literal(index));
                    out.push(value.as_deref().map_or_else(place::unknown_top, |name| {
                        crate::var_resolve::resolve_literal_access(
                            name,
                            ctx,
                            whole,
                            registry,
                            tcl_registry::TraceOperation::Read,
                        )
                    }));
                }
                ArgRole::Body | ArgRole::Expr => {
                    if let Some(written) = effective.written_argument(index) {
                        evaluated.insert(written);
                    }
                }
                _ => {}
            }
        }
    }
    let lookup = facts.map_or(command.as_str(), |facts| facts.canonical_command.as_str());
    let structural = structural_body_indices(lookup, args, tokens.as_ref(), registry);
    let expanded = tokens
        .as_ref()
        .is_some_and(|tokens| tokens.evaluated_body().is_some());
    for (index, argument) in args.iter().enumerate() {
        if structural.contains(&index) {
            continue;
        }
        if !is_braced_literal(tokens.as_ref(), index) || (evaluated.contains(&index) && !expanded) {
            word_reads(argument, ctx, out, registry);
        }
    }
    word_reads(command, ctx, out, registry);
}

/// The places *stmt* reads (sound over-approximation).
///
/// Covers `$`-substitution reads (element-granular for arrays), VAR_READ-role
/// name args at every nesting level (`[array get arr]` reads the whole array),
/// and the name/index variables read to *form* a dynamic target/ref (`set $X`
/// reads `X`; `$arr($i)` reads `i`) via [`places_read_to_form`].
#[must_use]
pub fn read_places(
    stmt: &Statement,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    if stmt.has_opaque_native_accesses() {
        return vec![place::unknown_top()];
    }
    if let Some(effects) = crate::dictionary_bindings::scope_marker_effects(stmt, ctx, registry) {
        return effects.reads;
    }
    let mut out: Vec<Place> = Vec::new();
    let grammar = registry
        .profile()
        .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar);

    match stmt {
        Statement::AssignValue { value, .. } => word_reads(value, ctx, &mut out, registry),
        Statement::Incr {
            name,
            name_braced,
            amount,
            ..
        } => {
            // `incr x` reads its own target; the amount may read more.
            out.push(crate::var_resolve::resolve_target_access(
                name,
                *name_braced,
                ctx,
                registry,
                tcl_registry::TraceOperation::Read,
            ));
            if let Some(a) = amount {
                word_reads(a, ctx, &mut out, registry);
            }
        }
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            for nm in vars_in_expr(expr, grammar) {
                out.push(crate::var_resolve::resolve_access(
                    &nm,
                    ctx,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                ));
            }
            for cmd_text in expr.command_texts() {
                word_reads(&cmd_text, ctx, &mut out, registry);
            }
        }
        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                word_reads(v, ctx, &mut out, registry);
            }
            if let Some(e) = expr {
                for nm in vars_in_expr(e, grammar) {
                    out.push(crate::var_resolve::resolve_access(
                        &nm,
                        ctx,
                        false,
                        registry,
                        tcl_registry::TraceOperation::Read,
                    ));
                }
            }
        }
        Statement::Call { .. } | Statement::Barrier { .. } => {
            invocation_reads(stmt, ctx, registry, &mut out);
        }
        Statement::Block { body, .. } => {
            // A `Block` body (inlined passthrough / `eval {…}` brace-literal)
            // runs in the enclosing scope — recover its reads so a `$x` inside
            // `eval {puts $x}` isn't seen as dead.  (`namespace eval ns {…}`
            // lowers to a `Barrier`, not a `Block`, so it is not recursed.)
            for inner in &body.statements {
                out.extend(read_places(inner, ctx, registry));
            }
        }
        _ => {}
    }

    // The vars read to *form* any dynamic def target (`set $X` / `set ${tok}(k)`
    // / `set a($i)`) AND any dynamic read ref (`$arr($i)` reads `i`) — the
    // genuine name/index reads the dead-store/unused checks need.
    let mut formed: Vec<Place> = Vec::new();
    for dp in def_places(stmt, ctx, registry) {
        formed.extend(places_read_to_form(&dp));
    }
    for rp in &out {
        formed.extend(places_read_to_form(rp));
    }
    out.extend(formed);

    out
}

/// Read dependencies from the actual substitution contexts retained at a CFG point.
/// Named-variable roles use the invocation continuation after argument evaluation.
/// Compatibility scanners apply only when no source projection exists.
#[must_use]
pub fn read_places_at(
    stmt: &Statement,
    block: crate::cfg::BlockId,
    index: usize,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let context = points.before_statement(block, index);
    if stmt.has_opaque_native_accesses() {
        return vec![place::unknown_top()];
    }
    if let Some(effects) = crate::dictionary_bindings::scope_marker_effects(stmt, context, registry)
    {
        return effects.reads;
    }
    let Some(tokens) = points.source_tokens_at(block, index) else {
        return read_places(stmt, context, registry);
    };
    source_read_places(
        tokens,
        points.source_reads_at(block, index),
        context,
        registry,
    )
}

/// Exact lexical read dependencies before a represented terminator.
#[must_use]
pub fn terminator_read_places_at(
    term: &Terminator,
    block: crate::cfg::BlockId,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let context = points.before_terminator(block);
    let Some(tokens) = points.source_tokens_at(block, usize::MAX) else {
        return terminator_read_places(term, context, registry);
    };
    source_read_places(
        tokens,
        points.source_reads_at(block, usize::MAX),
        context,
        registry,
    )
}

fn source_read_places(
    tokens: &CommandTokens,
    accesses: &[crate::command_binding::SourceVariableAccess],
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let mut out: Vec<_> = accesses
        .iter()
        .map(|access| access.place_in_context(&access.variable_context, registry))
        .collect();
    out.extend(invocation_execution_read_places(tokens, registry));
    named_invocation_reads(tokens, context, registry, &mut out);
    let grammar = context.invocation_dialect.map_or_else(
        || tcl_lexer::LexerConfig::for_profile(registry.profile()),
        |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    );
    for nested in crate::word_subst::lifted_calls(Some(tokens), grammar) {
        if let Some(tokens) = nested.tokens {
            out.extend(invocation_execution_read_places(&tokens, registry));
            named_invocation_reads(&tokens, context, registry, &mut out);
        }
    }
    out
}

/// Physical May reads performed after this invocation's argv evaluation.
/// Missing source inventory and unenumerated execution preserve an unknown read;
/// synthetic phase boundaries never execute the original invocation again.
#[must_use]
pub fn invocation_execution_read_places(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Vec<Place> {
    invocation_execution_read_contexts(tokens, registry)
        .into_iter()
        .map(|(place, _)| place)
        .collect()
}

/// May reads performed by reached argument-substitution invocations.
/// These depend on retained nested dispatch proofs, independently of editable
/// lexical substitution references and the outer invocation's own execution.
#[must_use]
pub fn invocation_argument_execution_read_places(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let grammar = invocation_read_grammar(tokens, registry);
    crate::word_subst::lifted_calls(Some(tokens), grammar)
        .into_iter()
        .filter_map(|call| call.tokens)
        .flat_map(|nested| invocation_execution_read_places(&nested, registry))
        .collect()
}

pub(crate) fn invocation_read_grammar(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> tcl_lexer::LexerConfig {
    tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .map_or_else(
            || tcl_lexer::LexerConfig::for_profile(registry.profile()),
            |dialect| tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
        )
}

/// Execution dependencies paired with the physical world selected at each read.
/// These are May dependencies and never editable argv substitution references.
#[must_use]
pub fn invocation_execution_read_contexts<'a>(
    tokens: &'a CommandTokens,
    registry: &CommandRegistry,
) -> Vec<(Place, &'a ResolveContext)> {
    if tokens.synthetic.is_some() {
        return Vec::new();
    }
    let Some(binding) = &tokens.source_binding else {
        return Vec::new();
    };
    let Some(reads) = &binding.invocation_variable_reads else {
        return vec![(place::unknown_top(), binding.variable_context.as_ref())];
    };
    let mut result: Vec<_> = reads
        .substitutions
        .iter()
        .map(|access| {
            (
                access.place_in_context(&access.variable_context, registry),
                access.variable_context.as_ref(),
            )
        })
        .chain(
            reads
                .native_reads
                .iter()
                .map(|access| (access.place.clone(), access.variable_context.as_ref())),
        )
        .collect();
    if reads.residual == crate::command_binding::SourceVariableReadResidual::Unknown {
        result.push((place::unknown_top(), binding.variable_context.as_ref()));
    }
    result
}

fn named_invocation_reads(
    tokens: &CommandTokens,
    fallback: &ResolveContext,
    registry: &CommandRegistry,
    out: &mut Vec<Place>,
) {
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    let Some(invocation) =
        crate::registry_invocation::normal_transfer_invocation(registry, context, tokens)
    else {
        return;
    };
    let selected = tokens
        .source_binding
        .as_ref()
        .map_or(fallback, |binding| binding.variable_context.as_ref());
    for place in invocation.read_places(selected, registry) {
        if !out.contains(&place) {
            out.push(place);
        }
    }
}

/// The places a block *terminator* reads — an `if` / `while` / `for` branch
/// condition or a `return` value/expr.  Covers the direct `$`-refs plus any
/// variables hidden in a command substitution within the condition.
#[must_use]
pub fn terminator_read_places(
    term: &Terminator,
    ctx: &ResolveContext,
    registry: &CommandRegistry,
) -> Vec<Place> {
    let mut out: Vec<Place> = Vec::new();
    let grammar = registry
        .profile()
        .map_or_else(tcl_dialect::LexerGrammar::default, |p| p.grammar);
    match term {
        Terminator::Branch { condition, .. } => {
            for nm in vars_in_expr(condition, grammar) {
                out.push(crate::var_resolve::resolve_access(
                    &nm,
                    ctx,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                ));
            }
            for cmd_text in condition.command_texts() {
                word_reads(&cmd_text, ctx, &mut out, registry);
            }
        }
        Terminator::Return {
            value,
            expr,
            braced,
            ..
        } => {
            // A braced `return {…}` is a literal — its `$`-refs are not reads.
            if !braced {
                if let Some(v) = value {
                    word_reads(v, ctx, &mut out, registry);
                }
                if let Some(e) = expr {
                    for nm in vars_in_expr(e, grammar) {
                        out.push(crate::var_resolve::resolve_access(
                            &nm,
                            ctx,
                            false,
                            registry,
                            tcl_registry::TraceOperation::Read,
                        ));
                    }
                    for cmd_text in e.command_texts() {
                        word_reads(&cmd_text, ctx, &mut out, registry);
                    }
                }
            }
        }
        Terminator::Goto { .. } | Terminator::Complete { .. } => {}
    }
    out
}

/// True when two places denote the **exact same** literal-key array element or
/// dict path (same kind, namespace, base name, and statically-known index /
/// key path).  Dynamic indices never compare equal — they keep the
/// conservative behaviour.
fn same_literal_element(a: &Place, b: &Place) -> bool {
    use crate::place::IndexKind;
    if a.kind != b.kind || a.ns != b.ns || a.name != b.name || a.cell != b.cell {
        return false;
    }
    match a.kind {
        PlaceKind::ArrayElem => matches!(
            (&a.index, &b.index),
            (Some(ai), Some(bi))
                if ai.kind == IndexKind::Literal
                    && bi.kind == IndexKind::Literal
                    && ai.value == bi.value
        ),
        PlaceKind::DictPath => {
            a.index == b.index
                && !a.keys.is_empty()
                && a.keys.len() == b.keys.len()
                && a.keys.iter().zip(&b.keys).all(|(ai, bi)| {
                    ai.kind == IndexKind::Literal
                        && bi.kind == IndexKind::Literal
                        && ai.value == bi.value
                })
        }
        _ => false,
    }
}

/// True when a literal-key `ArrayElem` / `DictPath` write at
/// `block.statements[def_idx]` is overwritten by a *later* write in the same
/// block to the EXACT same place, with no intervening read of that specific
/// element.  Such a store is dead regardless of any later-version read of the
/// element — the must-alias kill overrides the over-approximating
/// element-observed suppression.
fn must_alias_killed_in_block(
    block: &crate::cfg::Block,
    def_idx: usize,
    def_place: &Place,
    block_id: crate::cfg::BlockId,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> bool {
    use crate::place::IndexKind;
    let def_is_literal = match def_place.kind {
        PlaceKind::ArrayElem => def_place
            .index
            .as_ref()
            .is_some_and(|i| i.kind == IndexKind::Literal),
        PlaceKind::DictPath => {
            !def_place.keys.is_empty()
                && def_place.keys.iter().all(|i| i.kind == IndexKind::Literal)
        }
        _ => false,
    };
    if !def_is_literal {
        return false;
    }
    for (index, stmt) in block.statements.iter().enumerate().skip(def_idx + 1) {
        let ctx = points.before_statement(block_id, index);
        // An intervening read of the same element cancels the kill.
        if read_places_at(stmt, block_id, index, points, registry)
            .iter()
            .any(|rp| same_literal_element(rp, def_place))
        {
            return false;
        }
        // A later write to the same element is a must-alias kill.
        if def_places(stmt, ctx, registry)
            .iter()
            .any(|dp| same_literal_element(dp, def_place))
        {
            return true;
        }
    }
    false
}

/// `(block_name, statement_index)` of every array-element / dict-path
/// assignment in *cfg* whose def [`Place`] is **observed** by some read place
/// in the function — the element writes a *name-level* dead-store / unused
/// analysis mis-folds (SSA collapses `a(k)` / `a(j)` / `$a` to the base `a`)
/// but that a read in fact keeps live.
///
/// Shared by the analyser (W220) and the optimiser (O109): both flag an
/// overwritten/unused SSA def at name granularity, so both consult this to
/// suppress the array-element false positives.  The suppression is
/// element-granular only — scalar defs (which never fold) keep their precise
/// name-level verdict.  A whole-array / UNKNOWN / upvar-alias read overlaps
/// every element (so `array get a` keeps each `a(k)` write live); this
/// full-scan `overlap` matches `main`'s bucketed `_make_element_observed`,
/// where `UNKNOWN` / `UPVAR_ALIAS` reads are wildcards (here `overlap` returns
/// `true` on those edges).  Empty (no cost) when *cfg* has no array-element
/// assignment.
#[must_use]
pub fn element_writes_observed_by_reads(
    cfg: &Function,
    fn_qname: &str,
    registry: &CommandRegistry,
) -> std::collections::HashSet<(String, i32)> {
    let points = build_point_resolve_contexts(cfg, fn_qname, registry);
    element_writes_observed_with_contexts(cfg, &points, registry)
}

/// Project element observations from the same proved bindings used by scalar and cell SSA.
#[must_use]
pub fn element_writes_observed_with_contexts(
    cfg: &Function,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> std::collections::HashSet<(String, i32)> {
    use std::collections::HashSet;

    let is_array_assign = |stmt: &Statement| -> bool {
        // `incr a(k)` also defines an array element and folds to the base `a`
        // under name-level SSA, so it participates in the same FP — and
        // `def_places` already resolves it.  (The analyser's W220 never flags
        // an `Incr`, but the optimiser's O109 reaches it, so include it.)
        matches!(
            stmt,
            Statement::AssignConst { name, .. }
                | Statement::AssignValue { name, .. }
                | Statement::AssignExpr { name, .. }
                | Statement::Incr { name, .. }
                if name.contains('(')
        )
    };

    let mut out = HashSet::new();
    // Cheap pre-check: only array-element assignments can be mis-folded.
    if !cfg
        .blocks
        .values()
        .any(|b| b.statements.iter().any(&is_array_assign))
    {
        return out;
    }

    // All read places in the function, collected once.
    let mut reads: Vec<Place> = Vec::new();
    for (&id, block) in &cfg.blocks {
        for (index, stmt) in block.statements.iter().enumerate() {
            reads.extend(read_places(
                stmt,
                points.before_statement(id, index),
                registry,
            ));
        }
        if let Some(term) = &block.terminator {
            reads.extend(terminator_read_places(
                term,
                points.before_terminator(id),
                registry,
            ));
        }
    }

    for (&id, block) in &cfg.blocks {
        for (idx, stmt) in block.statements.iter().enumerate() {
            if !is_array_assign(stmt) {
                continue;
            }
            // An element write is suppressed only when a read *observes* it AND
            // it is not overwritten by a later same-element write first: a
            // must-alias kill (a later write to the exact same literal key with
            // no intervening read of it) makes this store dead regardless of any
            // later-version read.
            let suppress = def_places(stmt, points.before_statement(id, idx), registry)
                .iter()
                .any(|d| {
                    matches!(d.kind, PlaceKind::ArrayElem | PlaceKind::DictPath)
                        && reads.iter().any(|r| overlap(d, r))
                        && !must_alias_killed_in_block(block, idx, d, id, points, registry)
                });
            if suppress && let Ok(i) = i32::try_from(idx) {
                out.insert((block.name.clone(), i));
            }
        }
    }
    out
}

/// Whether a reachable later access may consume a bound store before its
/// contents are replaced. Read addresses and observer effects come from the
/// point owner; a must replacement ends exposure on that normal path.
#[must_use]
pub fn write_observed_by_unknown_access(
    cfg: &Function,
    block: crate::cfg::BlockId,
    statement_index: usize,
    written: &Place,
    points: &PointResolveContexts,
    registry: &CommandRegistry,
) -> bool {
    let mut pending = vec![(block, statement_index.saturating_add(1))];
    let mut visited = std::collections::HashSet::new();
    'paths: while let Some((block, start)) = pending.pop() {
        if !visited.insert((block, start)) {
            continue;
        }
        let Some(contents) = cfg.blocks.get(&block) else {
            continue;
        };
        for (index, statement) in contents.statements.iter().enumerate().skip(start) {
            if read_places_at(statement, block, index, points, registry)
                .iter()
                .any(|read| overlap(read, written))
            {
                return true;
            }
            if points.after_statement(block, index).dynamic_bindings
                && crate::memory_ssa::is_clobber(
                    statement,
                    registry,
                    registry
                        .profile()
                        .map(tcl_registry::model::semantic::SemanticContext::for_profile),
                )
            {
                return true;
            }
            if def_places_with_continuation(
                statement,
                points.before_statement(block, index),
                points.after_statement(block, index),
                registry,
            )
            .iter()
            .any(|replacement| must_replace_contents(replacement, written))
            {
                pending.extend(
                    cfg.exception_edges
                        .iter()
                        .filter_map(|&(from, to)| (from == block).then_some((to, 0))),
                );
                continue 'paths;
            }
        }
        if contents.terminator.as_ref().is_some_and(|terminator| {
            terminator_read_places_at(terminator, block, points, registry)
                .iter()
                .any(|read| overlap(read, written))
        }) {
            return true;
        }
        pending.extend(
            cfg.block_successors(block)
                .into_iter()
                .map(|successor| (successor, 0)),
        );
    }
    false
}

fn must_replace_contents(replacement: &Place, original: &Place) -> bool {
    !replacement.dynamic
        && !replacement.observed
        && replacement.kind == original.kind
        && matches!(replacement.kind, PlaceKind::Scalar | PlaceKind::ArrayElem)
        && replacement.cell.as_ref().is_some_and(|cell| {
            cell.generation != crate::place::CellGeneration::Unknown
                && original.cell.as_ref() == Some(cell)
        })
        && crate::var_resolve::canonical_binding_value_key(replacement)
            == crate::var_resolve::canonical_binding_value_key(original)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use crate::place::{Index, LOCAL_NS, PlaceKind, array_elem, overlap, scalar};

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn replacement_ends_store_exposure_before_later_opaque_reads() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc f {} {set x OLD; set x NEW; unknown_reader}";
        let cu = CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let function = cu.function("::f").unwrap();
        let points = function.ssa.point_contexts.as_ref().unwrap();
        let stores: Vec<_> = function
            .cfg
            .blocks
            .iter()
            .flat_map(|(&block, body)| {
                body.statements
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, statement)| {
                        def_places(statement, points.before_statement(block, index), registry)
                            .iter()
                            .any(|place| place.name == "x")
                            .then_some((block, index, statement))
                    })
            })
            .collect();
        assert_eq!(stores.len(), 2);
        for (ordinal, &(block, index, statement)) in stores.iter().enumerate() {
            let written = def_places(statement, points.before_statement(block, index), registry);
            assert_eq!(written.len(), 1);
            assert_eq!(
                write_observed_by_unknown_access(
                    &function.cfg,
                    block,
                    index,
                    &written[0],
                    points,
                    registry,
                ),
                ordinal == 1,
                "store {ordinal}"
            );
        }
    }

    fn normal_call(source: &str, registry: &CommandRegistry) -> Statement {
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let segment = segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        Statement::Call {
            span: segment.span,
            command: segment.name().to_owned(),
            canonical_command: None,
            args: segment.args().to_vec(),
            defs: vec!["result".to_owned()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens),
            foreach_groups: None,
        }
    }

    #[test]
    fn body_output_selects_its_retargeted_normal_binding() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut before = ResolveContext::for_function("::p");
        before.invocation_dialect = registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile);
        let mut after = before.clone();
        after.known_namespaces.insert("::static".to_owned());
        after.define_literal("::static::captured", "BOOM", registry);
        let target = crate::var_resolve::resolve_literal_access(
            "::static::captured",
            &after,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        after
            .alias_bindings
            .insert("result".to_owned(), target.clone());
        let catch = normal_call("catch {error BOOM} result", registry);
        assert_eq!(
            def_places_with_continuation(&catch, &before, &after, registry),
            vec![target.clone()]
        );
        assert!(
            statement_mutation_places_with_continuation(&catch, &before, &after, registry)
                .contains(&target)
        );
        let set = normal_call("set result VALUE", registry);
        let defs = def_places_with_continuation(&set, &before, &after, registry);
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].ns, LOCAL_NS);
        assert_eq!(defs[0].name, "result");
        assert_ne!(defs[0].cell, target.cell);
    }

    /// Collect every def-place and read-place across a proc's CFG.
    fn places_of(src: &str, qname: &str) -> (Vec<Place>, Vec<Place>) {
        let r = registry();
        let cu = CompilationUnit::build_for(src, &r, false);
        let fu = cu.function(qname).expect("proc not found");
        let points = build_point_resolve_contexts(&fu.cfg, &fu.name, &r);
        let mut defs = Vec::new();
        let mut reads = Vec::new();
        for (&id, block) in &fu.cfg.blocks {
            for (index, stmt) in block.statements.iter().enumerate() {
                let ctx = points.before_statement(id, index);
                defs.extend(def_places(stmt, ctx, &r));
                reads.extend(read_places(stmt, ctx, &r));
            }
        }
        for place in defs.iter_mut().chain(&mut reads) {
            place.cell = None;
        }
        (defs, reads)
    }

    #[test]
    fn array_element_def_and_read_are_element_granular() {
        // Element-granular defs/reads: `set a(k) 1` defs a(k); `puts $a(k)`
        // reads a(k); `set a(j) 2` defs a(j), which NO read observes.  Dead-store
        // analysis suppresses the W220 on a(k) (a read overlaps it) but keeps a(j)'s.
        let (defs, reads) = places_of("proc f {} { set a(k) 1; set a(j) 2; puts $a(k) }", "::f");
        let ak = array_elem("a", Index::literal("k"), LOCAL_NS, false);
        let aj = array_elem("a", Index::literal("j"), LOCAL_NS, false);
        assert!(defs.contains(&ak), "expected a(k) def, got {defs:?}");
        assert!(defs.contains(&aj), "expected a(j) def, got {defs:?}");
        // a read observes a(k) …
        assert!(
            reads.iter().any(|r| overlap(r, &ak)),
            "a(k) should be read by `puts $a(k)`, reads={reads:?}",
        );
        // … but nothing observes a(j).
        assert!(
            !reads.iter().any(|r| overlap(r, &aj)),
            "a(j) must not be observed by any read, reads={reads:?}",
        );
    }

    #[test]
    fn scalar_def_and_read() {
        let (defs, reads) = places_of("proc f {} { set x 1; puts $x }", "::f");
        let x = scalar("x", LOCAL_NS, false);
        assert!(defs.contains(&x));
        assert!(reads.iter().any(|r| overlap(r, &x)));
    }

    #[test]
    fn var_read_role_arg_resolves_whole_array() {
        // `array get arr` reads the *whole* array arr → overlaps every element.
        let (_defs, reads) = places_of("proc f {} { set out [array get arr] }", "::f");
        let arr_k = array_elem("arr", Index::literal("k"), LOCAL_NS, false);
        assert!(
            reads.iter().any(|r| overlap(r, &arr_k)),
            "array get arr should read the whole array, reads={reads:?}",
        );
    }

    #[test]
    fn build_context_collects_declarations() {
        let r = registry();
        let cu = CompilationUnit::build_for(
            "proc f {} { global g; variable v; upvar 1 caller y; set x 1 }",
            &r,
            false,
        );
        let fu = cu.function("::f").unwrap();
        let points = build_point_resolve_contexts(&fu.cfg, &fu.name, &r);
        let ctx = points.before_terminator(fu.cfg.entry);
        assert!(ctx.alias_bindings.contains_key("g"));
        assert!(ctx.alias_bindings.contains_key("v"));
        assert!(ctx.alias_bindings.contains_key("y"));
        // `y` resolves to an upvar alias, not a plain local scalar.
        let p = resolve_place("y", ctx, false, &r);
        assert_eq!(p.kind, PlaceKind::UpvarAlias);
    }

    #[test]
    fn build_context_tolerates_qualified_upvar_head() {
        // A fully-qualified `::upvar` head must still register the alias —
        // `collect_upvar_aliases` strips the leading `::` so the bare-form
        // grammar matches (otherwise `upvar_aliases` would be incomplete).
        let r = registry();
        let cu = CompilationUnit::build_for("proc f {} { ::upvar 1 caller y; set x 1 }", &r, false);
        let fu = cu.function("::f").unwrap();
        let points = build_point_resolve_contexts(&fu.cfg, &fu.name, &r);
        let ctx = points.before_terminator(fu.cfg.entry);
        assert!(ctx.alias_bindings.contains_key("y"));
        assert_eq!(
            resolve_place("y", ctx, false, &r).kind,
            PlaceKind::UpvarAlias
        );
    }

    #[test]
    fn dynamic_index_read_records_the_index_var() {
        // `set a($i) 1` defs a($i) (dynamic) and reads `i` (to form the index).
        let (defs, reads) = places_of("proc f {} { set i 0; set a($i) 1 }", "::f");
        assert!(
            defs.iter()
                .any(|d| d.kind == PlaceKind::ArrayElem && d.name == "a"),
            "defs={defs:?}",
        );
        assert!(
            reads
                .iter()
                .any(|r| overlap(r, &scalar("i", LOCAL_NS, false))),
            "the dynamic index read of `i` must be recovered, reads={reads:?}",
        );
    }
}
