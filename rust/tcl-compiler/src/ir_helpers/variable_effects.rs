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

//! Variable writes and reads: structural defs of IR scripts, possible name writes of original statements, and the cells recovered command substitutions read or write.

use super::command_words::CommandWord;
use super::evaluated_substitutions::{
    EvaluatedCommandSubstitutions, expression_command_substitutions,
};
use super::substitution_surfaces::{
    collect_expr_command_surface_refs, evaluated_command_substitution_surfaces,
};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Script, Statement};
use crate::var_refs::VarReferenceScanner;
use std::collections::BTreeSet;
use tcl_registry::{ArgRole, CommandRegistry, InvocationWord, InvocationWords};

/// Depth cap for [`collect_defs_from_script`]'s recursion over nested
/// `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch` bodies.
/// Transitively bounded via `MAX_LOWER_NEST_DEPTH` (every `Script`
/// feeding SSA construction was built by [`crate::lowering`], which already
/// caps its own construction at 256), capped here independently for
/// defence-in-depth and consistency with every other full-tree walker in
/// this crate.
const MAX_DEFS_COLLECT_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// Possible value writes owned directly by a selected structured IR node.
/// Generic call defs are excluded: those require current Registry availability
/// and original binding/argument resolution. Names here are literal destinations,
/// independent of command substitution or a reached Native storage cell.
#[must_use]
pub(crate) fn structural_variable_write_projection(
    stmt: &Statement,
) -> tcl_registry::VariableWriteProjection {
    let mut projection = tcl_registry::VariableWriteProjection::default();
    match stmt {
        Statement::AssignConst {
            name, name_braced, ..
        }
        | Statement::AssignExpr {
            name, name_braced, ..
        }
        | Statement::AssignValue {
            name, name_braced, ..
        }
        | Statement::Incr {
            name, name_braced, ..
        } => {
            if crate::ssa::is_dynamic_write_target(name, *name_braced) {
                projection.opaque_variable_frame = true;
            } else {
                projection.literal_names.push(name.clone());
            }
        }
        Statement::Foreach { iterators, .. } => projection.literal_names.extend(
            iterators
                .iter()
                .flat_map(|iterator| iterator.vars.iter().cloned()),
        ),
        Statement::Catch {
            result_var,
            options_var,
            ..
        } => projection
            .literal_names
            .extend(result_var.iter().chain(options_var).cloned()),
        Statement::Try { handlers, .. } => projection.literal_names.extend(
            handlers
                .iter()
                .flat_map(|handler| handler.var_name.iter().chain(&handler.options_var).cloned()),
        ),
        _ => {}
    }
    projection
}

/// Literal destination names and existing callee summaries in structured IR.
/// Dynamic destinations remain unavailable; these names prove no completed
/// store, current variable cell or definite SSA version.
#[must_use]
pub fn defs_from_ir_script(script: &Script) -> Vec<String> {
    let mut defs = Vec::new();
    collect_defs_from_script(script, &mut defs, 0);
    defs
}

pub(super) fn collect_defs_from_script(script: &Script, defs: &mut Vec<String>, depth: u32) {
    if MAX_DEFS_COLLECT_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::AssignConst { .. }
            | Statement::AssignExpr { .. }
            | Statement::AssignValue { .. }
            | Statement::Incr { .. } => {
                defs.extend(structural_variable_write_projection(stmt).literal_names);
            }

            Statement::Call {
                defs: call_defs, ..
            } if !call_defs.is_empty() => {
                defs.extend_from_slice(call_defs);
            }

            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    collect_defs_from_script(&clause.body, defs, depth + 1);
                }
                if let Some(eb) = else_body {
                    collect_defs_from_script(eb, defs, depth + 1);
                }
            }

            Statement::For { body, .. } | Statement::While { body, .. } => {
                collect_defs_from_script(body, defs, depth + 1);
            }

            Statement::Foreach { body, .. } => {
                defs.extend(structural_variable_write_projection(stmt).literal_names);
                collect_defs_from_script(body, defs, depth + 1);
            }

            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                collect_defs_from_script(body, defs, depth + 1);
                if let Some(rv) = result_var {
                    defs.push(rv.clone());
                }
                if let Some(ov) = options_var {
                    defs.push(ov.clone());
                }
            }

            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                collect_defs_from_script(body, defs, depth + 1);
                for handler in handlers {
                    if let Some(vn) = &handler.var_name {
                        defs.push(vn.clone());
                    }
                    if let Some(ov) = &handler.options_var {
                        defs.push(ov.clone());
                    }
                    collect_defs_from_script(&handler.body, defs, depth + 1);
                }
                if let Some(fb) = finally_body {
                    collect_defs_from_script(fb, defs, depth + 1);
                }
            }

            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(body) = &arm.body {
                        collect_defs_from_script(body, defs, depth + 1);
                    }
                }
                if let Some(db) = default_body {
                    collect_defs_from_script(db, defs, depth + 1);
                }
            }

            _ => {}
        }
    }
}

/// Possible writes from an independently supplied standalone expression.
/// These names establish no completed store, native invocation or body owner.
#[must_use]
pub fn defs_from_expr(expr: &ExprNode, registry: &CommandRegistry) -> Vec<String> {
    let inventory = expression_command_substitutions(expr, registry, None);
    variable_write_effects_from_commands(inventory.all_commands(), registry).names
}

/// Primitive effects of independently supplied typed IR, without source lookup.
/// Present source evidence remains terminal; this supplies no dispatch, cell or value proof.
pub(crate) fn standalone_typed_statement_writes_are_covered(
    script: &Script,
    statement: &Statement,
    registry: &CommandRegistry,
) -> bool {
    if script.executed_source.is_some()
        || script.namespace_context.is_some()
        || script.command_binding_sites.iter().next().is_some()
        || !script.implicit_math_invocations.is_empty()
        || !script.expression_preparations.is_empty()
        || script
            .procedure_binding_requirements
            .iter()
            .next()
            .is_some()
        || script.native_compilation_failure.is_some()
        || script.native_compilation_admission.is_some()
        || statement.tokens().is_some()
    {
        return false;
    }
    let mut expressions = Vec::new();
    match statement {
        Statement::AssignConst { .. } | Statement::Incr { .. } | Statement::Switch { .. } => {}
        Statement::AssignExpr {
            expr,
            command_binding: None,
            ..
        } => expressions.push(expr),
        Statement::If { clauses, .. } => {
            expressions.extend(clauses.iter().map(|clause| &clause.condition));
        }
        Statement::For { condition, .. } | Statement::While { condition, .. } => {
            expressions.push(condition);
        }
        _ => return false,
    }
    let surfaces = evaluated_command_substitution_surfaces(statement, registry);
    if surfaces.opaque || !surfaces.texts.is_empty() {
        return false;
    }
    let mut pending: Vec<_> = expressions.into_iter().map(|expr| (expr, 0_u32)).collect();
    while let Some((expr, depth)) = pending.pop() {
        if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
            return false;
        }
        match expr {
            ExprNode::Command { .. } | ExprNode::Call { .. } | ExprNode::Raw { .. } => {
                return false;
            }
            ExprNode::Var { text, .. } if text.contains('[') => return false,
            ExprNode::Binary { left, right, .. } => {
                pending.extend([(left.as_ref(), depth + 1), (right.as_ref(), depth + 1)]);
            }
            ExprNode::Unary { operand, .. } => pending.push((operand, depth + 1)),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([
                (condition.as_ref(), depth + 1),
                (true_branch.as_ref(), depth + 1),
                (false_branch.as_ref(), depth + 1),
            ]),
            ExprNode::Literal { .. }
            | ExprNode::String { .. }
            | ExprNode::CompiledWord { .. }
            | ExprNode::Var { .. } => {}
        }
    }
    true
}

/// Possible name writes in one genuine original statement and its substitutions.
/// Unavailable owners and incomplete source coverage remain explicit.
pub(crate) fn original_statement_possible_name_writes(
    statement: &Statement,
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> VariableWriteEffects {
    let mut out = VariableWriteEffects::default();
    let Some(parent) = crate::registry_invocation::original_source_name_roles(registry, tokens)
    else {
        out.opaque = true;
        return out;
    };
    if let Some(effects) = original_invocation_possible_name_writes(tokens, registry) {
        out = effects;
    } else {
        retain_original_write_roles(&parent, &mut out);
    }
    let structural = structural_variable_write_projection(statement);
    out.opaque |= structural.opaque_variable_frame
        || structural
            .literal_names
            .iter()
            .any(|name| !out.names.contains(name));
    if !crate::var_escape::helpers::statement_needs_original_substitution_inventory(statement) {
        return out;
    }
    let Some(children) =
        crate::var_escape::helpers::original_substitution_invocations(tokens, registry)
    else {
        out.opaque = true;
        return out;
    };
    out.opaque |= children.unknown_residual;
    for child in children {
        let Some(names) = child.invocation else {
            out.opaque = true;
            continue;
        };
        if let Some(effects) = child.name_writes {
            out.opaque |= effects.opaque;
            out.names.extend(effects.names);
            out.read_names.extend(effects.read_names);
        } else {
            retain_original_write_roles(&names, &mut out);
        }
    }
    out
}

/// Complete selected effect projection beside the retained MAY name candidates.
/// This returns no store values, definite SSA definitions or entered-frame proof.
pub(crate) fn original_invocation_possible_name_writes(
    tokens: &CommandTokens,
    registry: &CommandRegistry,
) -> Option<VariableWriteEffects> {
    use crate::registry_invocation::EffectiveInvocationWord;
    use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding
        .original_source_metadata_for_advice(tokens, registry)
        .ok()??;
    let candidates =
        crate::registry_invocation::possible_variable_name_operands_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )?;
    let invocation = crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
        registry,
        Some(metadata),
        tokens,
    )?;
    let realm = binding.invocation_realm()?;
    let (projection, additional_writes) =
        invocation.with_metadata_schema(registry, metadata, realm, |schema| {
            (schema.descriptor_key() == invocation.facts.descriptor_key()).then(|| {
                (
                    CommandRegistry::variable_write_projection_for_selected_source(schema),
                    schema
                        .semantics
                        .world_effects
                        .has_uncovered_variable_role_writes(
                            schema.words.arguments(),
                            candidates
                                .phased_operands()
                                .any(|(role, _, _, _)| role == ArgRole::VarWrite)
                                || candidates
                                    .phased_unresolved_roles()
                                    .any(|(role, _)| role == ArgRole::VarWrite),
                        ),
                )
            })
        })?;
    let mut out = VariableWriteEffects {
        names: projection.literal_names,
        exact_read_names: projection.read_before_write_names.clone(),
        read_names: projection.read_before_write_names,
        opaque: projection.opaque_variable_frame
            || candidates.unknown_residual()
            || additional_writes,
    };
    let callback = invocation.facts.effects.callback();
    let variables = &binding.variable_context;
    out.opaque |= match callback.kinds {
        tcl_registry::world_effect::CallbackKinds::NONE => false,
        tcl_registry::world_effect::CallbackKinds::TRACE => {
            variables.dynamic_traces
                || !variables.traced.is_empty()
                || !variables.untracked_traces.is_empty()
                || !variables.possible_trace_registrations.is_empty()
                || !variables.trace_registrations.is_empty()
                || !variables.trace_registration_receivers.is_empty()
        }
        _ => true,
    };
    // State changes outside the selected stores can alter later alias/observer behaviour.
    out.opaque |= invocation.facts.effects.accesses().iter().any(|access| {
        access.domain != WorldStateDomain::VariableStore && access.mode != EffectAccessMode::Read
    });
    for (role, word, _, _) in candidates.phased_operands() {
        if role != ArgRole::VarWrite {
            continue;
        }
        match word {
            EffectiveInvocationWord::Literal(name) if !name.contains('\0') => {
                if !out.names.contains(name) {
                    out.names.push(name.clone());
                }
            }
            EffectiveInvocationWord::ArrayElementName { root } if !root.contains('\0') => {
                if !out.names.contains(root) {
                    out.names.push(root.clone());
                }
            }
            _ => out.opaque = true,
        }
    }
    out.opaque |= candidates
        .phased_unresolved_roles()
        .any(|(role, _)| role == ArgRole::VarWrite);
    Some(out)
}

fn retain_original_write_roles(
    source: &crate::registry_invocation::SourceNameRoleInventory,
    out: &mut VariableWriteEffects,
) {
    // A name-role owner does not cover additional store, observer or alias effects.
    out.opaque = true;
    for &(argument, role) in source.roles() {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let Some(name) = source.original_variable_name_operand(argument) else {
            out.opaque = true;
            continue;
        };
        if !out.names.iter().any(|candidate| candidate == name.as_str()) {
            out.names.push(name.as_str().to_owned());
        }
    }
}

/// Source-aware projection of variable-cell writes from an invocation.
/// `opaque` means the command can write a variable but Tcl source expansion
/// or substitution prevents naming the target safely.
#[derive(Debug, Default)]
pub(crate) struct VariableWriteEffects {
    pub names: Vec<String>,
    /// The subset of [`Self::names`] the writing command reads before it
    /// writes (`[incr n]`, `[append s x]`). The store feeding one of these is
    /// *observed*, not overwritten, so a consumer that records the write must
    /// record this read with it or the feeding store looks dead (#2050).
    pub read_names: Vec<String>,
    /// [`Self::read_names`] as the commands spell them: a literal element
    /// read keeps its element, where `read_names` names its array.
    pub exact_read_names: Vec<String>,
    pub opaque: bool,
}

/// Source-aware projection of variable-cell reads from an invocation.
/// `opaque` means the command reads a variable whose name substitution
/// prevents naming.
#[derive(Debug, Default)]
pub(crate) struct VariableReadEffects {
    pub names: Vec<String>,
    pub opaque: bool,
}

/// Project the variable cells recursively recovered command substitutions read
/// **by name** — an [`ArgRole::VarRead`](tcl_registry::ArgRole::VarRead) word
/// such as `info exists n` or `array size a`, and a destroyer's target
/// (`unset x`), whose existence the unbind reads.
///
/// The read half of [`variable_write_effects_from_commands`], and needed for
/// the same reason: a store observed only by an existence query looked unread,
/// so `proc p {} {set x 1; if {[info exists x]} {puts yes}}` had `set x 1`
/// removed and stopped printing `yes` (#2132), and one observed only by a
/// nested `[unset x]` was removed until the unset raised.
#[must_use]
pub(crate) fn variable_read_effects_from_commands<'a>(
    commands: impl IntoIterator<Item = &'a Vec<CommandWord>>,
    registry: &CommandRegistry,
) -> VariableReadEffects {
    let mut out = VariableReadEffects::default();
    for words in commands {
        let Some(head) = words.first() else {
            continue;
        };
        let args: Vec<InvocationWord<'_>> = words
            .iter()
            .skip(1)
            .map(CommandWord::invocation_word)
            .collect();
        let projection = registry
            .variable_read_projection(InvocationWords::structured(head.invocation_word(), &args));
        out.opaque |= projection.opaque_variable_frame;
        for name in projection.literal_names {
            if !out.names.contains(&name) {
                out.names.push(name);
            }
        }
    }
    out
}

/// Project variable writes from recursively recovered command substitutions.
#[must_use]
pub(crate) fn variable_write_effects_from_commands<'a>(
    commands: impl IntoIterator<Item = &'a Vec<CommandWord>>,
    registry: &CommandRegistry,
) -> VariableWriteEffects {
    let mut out = VariableWriteEffects::default();
    for words in commands {
        let Some(head) = words.first() else {
            continue;
        };
        let args: Vec<InvocationWord<'_>> = words
            .iter()
            .skip(1)
            .map(CommandWord::invocation_word)
            .collect();
        let projection = registry.variable_write_projection_for_source_words(
            InvocationWords::structured(head.invocation_word(), &args),
        );
        out.opaque |= projection.opaque_variable_frame;
        for name in projection.read_before_write_names {
            if !out.read_names.contains(&name) {
                out.read_names.push(name);
            }
        }
        // A conditional writer (`regexp`, `scan`, `binary scan`) leaves each
        // target it does not reach as it was, so the store feeding one is
        // observed, not overwritten: the nested twin of the SSA's rule for
        // the same commands as statements. `set v d; if {[regexp {(x)} $s
        // -> v]} {…}; puts $v` prints `d` when nothing matches, and without
        // the read `set v d` was deleted as a dead store (#2051).
        if conditionally_writes(words, registry) {
            for name in &projection.literal_names {
                if !out.read_names.contains(name) {
                    out.read_names.push(name.clone());
                }
            }
        }
        for name in projection.literal_names {
            if !out.names.contains(&name) {
                out.names.push(name);
            }
        }
    }
    out
}

/// Conditional embedded write footprint under exact supplied availability.
/// Missing or foreign metadata is opaque, independently of lexical word shape.
#[must_use]
#[cfg(test)]
pub(crate) fn variable_write_effects_from_commands_with_metadata_context<'a>(
    commands: impl IntoIterator<Item = &'a Vec<CommandWord>>,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) -> VariableWriteEffects {
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return VariableWriteEffects {
            opaque: true,
            ..VariableWriteEffects::default()
        };
    };
    let mut out = VariableWriteEffects::default();
    for words in commands {
        let Some(head) = words.first() else {
            continue;
        };
        let args: Vec<InvocationWord<'_>> = words
            .iter()
            .skip(1)
            .map(CommandWord::invocation_word)
            .collect();
        let projection = registry.variable_write_projection_in_resolved_context(
            context.context(),
            InvocationWords::structured(head.invocation_word(), &args),
            context.context().authoring_query().realm,
        );
        out.opaque |= projection.opaque_variable_frame;
        for name in projection.read_before_write_names {
            if !out.read_names.contains(&name) {
                out.read_names.push(name);
            }
        }
        for name in projection.literal_names {
            if !out.names.contains(&name) {
                out.names.push(name);
            }
        }
    }
    out
}

/// Whether the recovered command `words` names a conditional writer: the
/// registry's `CONDITIONAL_VARIABLE_WRITE` trait on the invocation (a
/// subcommand's included, `binary scan`'s), resolved over its literal words.
fn conditionally_writes(words: &[CommandWord], registry: &CommandRegistry) -> bool {
    let Some(head) = words.first().and_then(CommandWord::literal) else {
        return false;
    };
    let args: Vec<&str> = words[1..]
        .iter()
        .map(|word| word.literal().unwrap_or_default())
        .collect();
    registry
        .invocation_traits(head, &args, registry.own_surface_query())
        .contains(tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE)
}

/// Every variable `stmt` may read by substitution, wherever the read sits: in
/// its own words or expression, in the commands its `[…]` substitutions run,
/// and in the expression words those commands evaluate in this frame
/// (`embedded`, recovered from the same statement).
///
/// The statement's own uses cover its words. The commands its substitutions
/// run and the expression words they evaluate are scanned as text, because the
/// lowering keeps no word structure for them (an expression's
/// `ExprNode::Command` is one opaque spelling, an `incr` amount is a string).
/// The effect call that carries an embedded command's write needs the whole
/// set: a name that write defines is read by the words before, between and
/// after it.
#[must_use]
pub(crate) fn statement_substituted_reads(
    stmt: &Statement,
    embedded: &EvaluatedCommandSubstitutions,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> BTreeSet<String> {
    let mut reads: BTreeSet<String> = crate::ssa::uses_of(stmt, scanner, registry)
        .into_iter()
        .collect();
    let surfaces = evaluated_command_substitution_surfaces(stmt, registry);
    for text in surfaces
        .texts
        .iter()
        .copied()
        .chain(embedded.in_frame_texts.iter().map(String::as_str))
    {
        reads.extend(scanner.scan_word(text, registry));
    }
    reads
}

/// [`statement_substituted_reads`] for a branch condition: its own variables,
/// the commands its substitutions run, and the expression words they evaluate.
#[must_use]
pub(crate) fn condition_substituted_reads(
    condition: &ExprNode,
    embedded: &EvaluatedCommandSubstitutions,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> BTreeSet<String> {
    let mut reads: BTreeSet<String> = condition
        .vars_element_qualified_with_config(scanner.lexer_config())
        .into_iter()
        .collect();
    let mut texts: Vec<&str> = Vec::new();
    let (mut opaque, mut conditional) = (false, false);
    collect_expr_command_surface_refs(condition, &mut texts, &mut opaque, &mut conditional, 0);
    for text in texts
        .into_iter()
        .chain(embedded.in_frame_texts.iter().map(String::as_str))
    {
        reads.extend(scanner.scan_word(text, registry));
    }
    reads
}
