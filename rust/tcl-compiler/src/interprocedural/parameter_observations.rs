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

//! How a body uses each parameter, read from the original source words: references, forwarding, aliases, conditions and returns.

use super::{LocalFacts, ProcArgTrait, ProcScan, ScanCtx};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::ir::ProcedureId;
use std::collections::HashSet;

pub(super) fn original_parameter_observation_owner(scan: ProcScan<'_>) -> Option<ProcedureId> {
    let select = || {
        let module = scan.source_module;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.positioned_for_module(module, scan.registry)?;
        let mut owners = module
            .original_source_procedure_bodies()
            .filter(|(_, procedure)| std::ptr::eq(*procedure, scan.proc));
        let (&id, procedure) = owners.next()?;
        if owners.next().is_some()
            || !module.procedure_ids.owns(id)
            || !retained.matches_original_procedure_summary(id, procedure)
            || module.installed_procedure_body_units.contains_key(&id)
        {
            return None;
        }
        let formals = if let Some(card) = module.original_authored_declaration_body_units.get(&id) {
            card.matches_procedure(module, procedure)
        } else if let Some(card) = module.original_native_declaration_body_units.get(&id) {
            card.matches_procedure(module, procedure)
        } else if let Some(card) = module.original_logical_declaration_body_units.get(&id) {
            card.matches_procedure(module, procedure)
        } else {
            let topology =
                crate::var_escape::original_slots::original_procedure_topology(module, procedure)?;
            if topology
                .bindings(topology.parameters().len())
                .ok()?
                .iter()
                .any(|binding| {
                    matches!(
                        binding,
                        tcl_syntax::formal_params::FormalByteArgumentBinding::CallerLink { .. }
                    )
                })
            {
                return None;
            }
            topology
                .parameters()
                .iter()
                .map(|formal| formal.name.as_slice())
                .eq(procedure.params.iter().map(String::as_bytes))
        };
        if !formals {
            return None;
        }
        if let Some(unit) = scan.unit {
            let function = unit.function_for_body(Some(id))?;
            if !std::ptr::eq(std::ptr::from_ref(&unit.ir_module), module)
                || unit.source.as_bytes() != module.source.bytes()
                || unit
                    .original_procedure_for_body(id)
                    .is_none_or(|current| !std::ptr::eq(current, procedure))
                || match (
                    function.source_metadata_input(),
                    module.source_metadata_input.as_ref(),
                ) {
                    (None, None) => false,
                    (Some(actual), Some(original)) => {
                        actual != original && actual != &original.for_nested_source()
                    }
                    _ => true,
                }
                || function.body_id != Some(id)
                || unit.original_procedure_cfg_for_body(id) != Some(&function.cfg)
                || function.source_lexer_config()
                    != module.native_lexer_config().nested().normalized()
            {
                return None;
            }
        }
        Some(id)
    };
    select()
}

/// Complete lexical incidence over genuine original words; every child keeps
/// the same immutable image and full grammar. Evaluation roles remain separate.
pub(super) fn original_word_reference_names(
    words: &[tcl_lexer::NativeWord],
) -> Option<HashSet<String>> {
    let mut pending = words.to_vec();
    let mut names = HashSet::new();
    let mut remaining = 4096usize;
    while let Some(word) = pending.pop() {
        remaining = remaining.checked_sub(1)?;
        let arena = word.executable_parts();
        for component in arena.all_parts() {
            remaining = remaining.checked_sub(1)?;
            match component.part {
                tcl_lexer::ExecutablePart::Variable { name, .. } => {
                    names.insert(std::str::from_utf8(arena.bytes(name)?).ok()?.to_owned());
                }
                tcl_lexer::ExecutablePart::Command { body } => {
                    let plan = tcl_lexer::native_script_words_in_with_purpose(
                        word.image().clone(),
                        body,
                        word.config(),
                        tcl_lexer::SourceSegmentationPurpose::NestedScript,
                    )
                    .ok()?;
                    if plan.fatal_tail.is_some() {
                        return None;
                    }
                    for command in plan.commands {
                        pending.extend(command.words);
                    }
                }
                tcl_lexer::ExecutablePart::Text(_) => {}
                tcl_lexer::ExecutablePart::ParseError(_)
                | tcl_lexer::ExecutablePart::Expression { .. } => return None,
            }
        }
    }
    Some(names)
}

fn original_command_parameter_names(ctx: ScanCtx<'_>) -> Option<HashSet<String>> {
    let (_, words) = crate::registry_invocation::original_command_words_for_module(
        ctx.registry,
        ctx.source_module,
        ctx.current_tokens?,
    )?;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    let mut names = original_word_reference_names(&words)?;
    names.retain(|name| ctx.params.contains(name));
    Some(names)
}

pub(super) fn note_original_command_references(ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    if let Some(names) = original_command_parameter_names(ctx) {
        facts.param_references.extend(names);
    } else {
        facts.param_observations_incomplete = true;
    }
    let Some(roles) = ctx.current_tokens.and_then(|tokens| {
        crate::registry_invocation::original_source_name_roles(ctx.registry, tokens)
    }) else {
        facts.param_observations_incomplete = true;
        return;
    };
    if !roles.roles_complete() {
        facts.param_observations_incomplete = true;
    }
    for &(argument, role) in roles.roles() {
        if role != tcl_registry::ArgRole::VarRead
            && !(role == tcl_registry::ArgRole::VarWrite
                && roles
                    .facts()
                    .traits
                    .contains(tcl_registry::Traits::READS_BEFORE_WRITE))
        {
            continue;
        }
        let Some(name) = roles.original_variable_name_operand(argument) else {
            facts.param_observations_incomplete = true;
            continue;
        };
        if ctx.params.contains(name.as_str()) {
            facts.param_references.insert(name.as_str().to_owned());
        } else if tcl_syntax::naming::is_qualified(name.as_str().as_bytes())
            || tcl_syntax::naming::split_element_ref(name.as_str()).is_some()
        {
            facts.param_observations_incomplete = true;
        }
    }
}

pub(super) fn note_original_command_params(
    ctx: ScanCtx<'_>,
    kind: ProcArgTrait,
    facts: &mut LocalFacts,
) {
    let Some(names) = original_command_parameter_names(ctx) else {
        facts.param_observations_incomplete = true;
        return;
    };
    facts.param_references.extend(names.iter().cloned());
    for name in names {
        facts
            .param_trait_flags
            .entry(name)
            .or_default()
            .insert(kind);
    }
}

/// Conditional source forwarding reports do not close the later Native
/// invocation/effect scan or its unknown observer/lookup alternatives.
pub(super) fn note_original_authored_body_call_parameters(
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    if ctx.parameter_body.is_none() {
        return;
    }
    let Some(calls) = ctx.current_tokens.and_then(|tokens| {
        crate::registry_invocation::original_authored_body_procedure_calls_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    }) else {
        return;
    };
    // Source correspondence leaves entry, lookup and observers unproved.
    facts.param_observations_incomplete |= !calls.obligations().is_empty();
    if !calls.calls().is_empty()
        && calls
            .calls()
            .iter()
            .all(|call| call.accepts_arguments() && ctx.known.contains(&call.id()))
    {
        note_original_command_params(ctx, ProcArgTrait::ForwardedToCallee, facts);
    }
}

/// Source-only alias/formal incidence follows actual represented sibling
/// commands; runtime entry/lookup/observer uncertainty remains explicit.
pub(super) fn note_original_authored_alias_parameters(ctx: ScanCtx<'_>, facts: &mut LocalFacts) {
    let command = ctx.current_tokens.and_then(|tokens| {
        crate::registry_invocation::original_authored_parameter_source_command_for_module(
            tokens,
            ctx.source_module,
            ctx.registry,
        )
    });
    let Some(command) = command.filter(|command| Some(command.body()) == ctx.parameter_body) else {
        facts.source_parameter_aliases.clear();
        return;
    };
    // The source recipe supplies no entered body, actual lookup or observer
    // closure, even when the original symbolic argument layout is complete.
    if !command.aliases().is_empty() || !facts.source_parameter_aliases.is_empty() {
        facts.param_observations_incomplete |= command.has_unproved_applicability();
    }
    facts
        .source_parameter_aliases
        .retain(|_, alias| alias.belongs_to(&command));
    let mut declarations = HashSet::new();
    for alias in command.aliases() {
        if let Some(parameter) = command
            .alias_target_formal(alias)
            .filter(|parameter| ctx.params.contains(*parameter))
        {
            facts.param_references.insert(parameter.to_owned());
            facts
                .param_trait_flags
                .entry(parameter.to_owned())
                .or_default()
                .insert(ProcArgTrait::VarRead);
        }
        let Some(local) = command.literal(alias.words.local) else {
            facts.source_parameter_aliases.clear();
            continue;
        };
        if !command.local_scalar(local) || alias.local.literal() != Some(local) {
            facts.source_parameter_aliases.clear();
            continue;
        }
        declarations.insert(local.to_owned());
        facts.source_parameter_aliases.remove(local);
        if let Some(observation) = command.caller_alias(alias) {
            facts
                .source_parameter_aliases
                .insert(local.to_owned(), observation);
        }
    }
    for &(argument, role) in command.roles() {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let Some(local) = command.literal(argument) else {
            facts.source_parameter_aliases.clear();
            continue;
        };
        if declarations.contains(local) {
            continue;
        }
        if let Some(alias) = facts.source_parameter_aliases.get(local) {
            facts
                .param_trait_flags
                .entry(alias.parameter().to_owned())
                .or_default()
                .insert(ProcArgTrait::VarWrite);
        }
    }
    if !command.continues() {
        facts.source_parameter_aliases.clear();
    }
}

pub(super) fn original_parameter_expression_config(
    expression: &crate::expr_ast::ExprNode,
    base: Option<u32>,
    ctx: ScanCtx<'_>,
) -> Option<tcl_lexer::LexerConfig> {
    let tokens = ctx.current_tokens?;
    let (config, words) = crate::registry_invocation::original_command_words_for_module(
        ctx.registry,
        ctx.source_module,
        tokens,
    )?;
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_authored_source_body_name_role_owner()
        && !binding.has_original_source_layout_owner()
    {
        let id = ctx.parameter_body?;
        let module = ctx.source_module;
        let procedure = module.body_units.get(&id)?;
        let card = module.original_authored_declaration_body_units.get(&id)?;
        (module.procedure_ids.owns(id) && card.matches_procedure(module, procedure))
            .then_some(())?;
        let mut matches = false;
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            use crate::ir::Statement;
            let original = |node: &crate::expr_ast::ExprNode, original_base: Option<u32>| {
                std::ptr::eq(node, expression) && original_base == base
            };
            matches |= match statement {
                Statement::AssignExpr {
                    expr, expr_base, ..
                }
                | Statement::ExprEval {
                    expr, expr_base, ..
                }
                | Statement::Return {
                    expr: Some(expr),
                    expr_base,
                    ..
                } => original(expr, *expr_base),
                Statement::If { clauses, .. } => clauses
                    .iter()
                    .any(|clause| original(&clause.condition, clause.condition_base)),
                Statement::While {
                    condition,
                    condition_base,
                    ..
                }
                | Statement::For {
                    condition,
                    condition_base,
                    ..
                } => original(condition, *condition_base),
                _ => false,
            };
        });
        matches.then_some(())?;
        // The unchanged retained declaration Script owns this exact lowered AST.
        // The original complete operand owns affine source content separately.
        let scope =
            binding.original_authored_source_body_scope_for_name_roles(tokens, ctx.registry)?;
        scope.matches_module(ctx.source_module).then_some(())?;
        let base = base?;
        let mut operands = words.iter().filter(|word| {
            word.content_span().is_ok_and(|span| {
                span.start() == base
                    && tcl_syntax::word_rules::original_static_word_unicode_value(word).is_some_and(
                        |value| word.image().bytes().get(span.as_range()) == Some(value.as_slice()),
                    )
            })
        });
        operands.next()?;
        operands.next().is_none().then_some(config)
    } else {
        let (original, _) = crate::registry_invocation::original_lowered_expression_syntax(
            expression, base, tokens,
        )?;
        (original == config).then_some(config)
    }
}

pub(super) fn note_original_return_parameter(
    value: Option<&crate::ir::WordExpr>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    if facts.param_observations_incomplete {
        return;
    }
    let name = || {
        let value = value?;
        let tokens = ctx.current_tokens?;
        let (_, words) = crate::registry_invocation::original_command_words_for_module(
            ctx.registry,
            ctx.source_module,
            tokens,
        )?;
        let mut indices = tokens
            .words()
            .iter()
            .enumerate()
            .filter(|(_, word)| *word == value);
        let (index, _) = indices.next()?;
        if index == 0 || indices.next().is_some() {
            return None;
        }
        let name = tcl_syntax::word_rules::original_scalar_variable_with_prefix(
            words.get(index)?,
            &[b""],
        )?;
        let name = String::from_utf8(name).ok()?;
        ctx.params.contains(&name).then_some(name)
    };
    if let Some(name) = name() {
        // Source trait advice stays independent of classify_return's stronger
        // Native value/return-fold premises.
        facts.param_references.insert(name.clone());
        facts
            .param_trait_flags
            .entry(name)
            .or_default()
            .insert(ProcArgTrait::Passthrough);
    }
}

pub(super) fn note_original_expression_params(
    expression: &crate::expr_ast::ExprNode,
    base: Option<u32>,
    ctx: ScanCtx<'_>,
    facts: &mut LocalFacts,
) {
    let Some(config) = original_parameter_expression_config(expression, base, ctx) else {
        facts.param_observations_incomplete = true;
        return;
    };
    note_params_in_expr(expression, ctx.params, facts, config, 0);
}

/// Visit each `ExprNode::Var` in `node` and mark matching
/// parameters as `UsedInCondition` — the expression is inside an
/// `if` / `while` / `for` condition (or a standalone `ExprEval`
/// treated analogously).
pub(super) fn note_params_in_expr(
    node: &crate::expr_ast::ExprNode,
    params: &HashSet<String>,
    facts: &mut LocalFacts,
    config: tcl_lexer::LexerConfig,
    depth: u32,
) {
    use crate::expr_ast::ExprNode;
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        facts.param_observations_incomplete = true;
        return;
    }
    match node {
        ExprNode::Var { .. } => {
            let Ok(Some(reference)) = node.variable_reference(config) else {
                facts.param_observations_incomplete = true;
                return;
            };
            let Ok(name) = std::str::from_utf8(reference.name) else {
                facts.param_observations_incomplete = true;
                return;
            };
            if params.contains(name) {
                facts.param_references.insert(name.to_owned());
                facts
                    .param_trait_flags
                    .entry(name.to_owned())
                    .or_default()
                    .insert(ProcArgTrait::UsedInCondition);
            }
            // Index substitutions have their own original word geometry.
            if reference.index.is_some() {
                facts.param_observations_incomplete = true;
            }
        }
        ExprNode::Binary { left, right, .. } => {
            note_params_in_expr(left, params, facts, config, depth + 1);
            note_params_in_expr(right, params, facts, config, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            note_params_in_expr(operand, params, facts, config, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            note_params_in_expr(condition, params, facts, config, depth + 1);
            note_params_in_expr(true_branch, params, facts, config, depth + 1);
            note_params_in_expr(false_branch, params, facts, config, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for a in args {
                note_params_in_expr(a, params, facts, config, depth + 1);
            }
        }
        ExprNode::Literal { .. } => {}
        ExprNode::Command { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Raw { .. } => {
            facts.param_observations_incomplete = true;
        }
    }
}
