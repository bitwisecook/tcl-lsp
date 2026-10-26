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

//! Recursive IR and expression tree helpers for the CFG builder.
//!
//! These extract variable definitions from structured IR scripts and
//! expression trees. Used at `catch`/`try` merge points to
//! conservatively invalidate variables that may have been partially
//! modified before an exception, and at condition sites to track
//! definitions produced by command substitutions.

mod materialized_footprint;
pub(crate) use materialized_footprint::{
    command_possible_footprint_with_metadata_context, command_possible_reads_with_metadata_context,
    expression_possible_writes_with_metadata_context, footprint_command_words,
    immediate_same_frame_script_values, script_value_name_ownership_with_metadata_context,
    script_value_possible_writes_with_metadata_context,
};
use std::collections::BTreeSet;

use tcl_lexer::{LexerConfig, SourceMap, TokenType};
use tcl_registry::{ArgRole, CommandRegistry, InvocationWord, InvocationWords, Traits};

use crate::depth_guard::{MAX_BRACKET_TEXT_DEPTH, MAX_EXPR_NODE_DEPTH};
use crate::expr_ast::ExprNode;
pub(crate) use crate::ir::ExecutionNamespace;
use crate::ir::{CommandTokens, Script, Statement, WordExpr, WordPart};
#[cfg(test)]
use crate::naming::normalise_var_name;
use crate::segmenter::SegmentedCommand;
use crate::var_refs::VarReferenceScanner;

/// The nested script bodies a structured-control-flow statement contains —
/// every shape a flow-*insensitive* whole-body walk needs to recurse into
/// (`If`/`For`/`While`/`Foreach`/`Catch`/`Block`/`UpFrame`/`Switch`/`Try`).
/// `pub(crate)` so both [`crate::cfg_builder::global_write_info`] (per-proc
/// global-write summaries) and [`crate::var_observability`] (module-wide
/// trace-target summaries) share one recursive-descent shape instead of each
/// re-deriving it.
///
/// Distinct from [`collect_defs_from_script`]'s per-statement match: that
/// walk also extracts *defs* at each nesting level (iteration variables,
/// catch/try result vars) as part of the same pass, so it isn't a drop-in
/// replacement for a caller that only wants "which scripts nest inside this
/// statement".
#[must_use]
pub(crate) fn nested_bodies(stmt: &Statement) -> Vec<&Script> {
    match stmt {
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut bodies: Vec<&Script> = clauses.iter().map(|c| &c.body).collect();
            if let Some(e) = else_body {
                bodies.push(e);
            }
            bodies
        }
        Statement::For {
            init, next, body, ..
        } => vec![init, next, body],
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => vec![body],
        Statement::Switch {
            arms, default_body, ..
        } => {
            let mut bodies: Vec<&Script> = arms.iter().filter_map(|a| a.body.as_ref()).collect();
            if let Some(d) = default_body {
                bodies.push(d);
            }
            bodies
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let mut bodies = vec![body];
            bodies.extend(handlers.iter().map(|h| &h.body));
            if let Some(f) = finally_body {
                bodies.push(f);
            }
            bodies
        }
        _ => Vec::new(),
    }
}

/// Nested script bodies paired with the command namespace in which Tcl runs
/// them. This is the sole nested-execution context transform for semantic IR
/// walkers: `#0` selects global, relative zero inherits, and every other
/// caller selection remains unknown.
#[must_use]
pub(crate) fn nested_execution_bodies<'a>(
    stmt: &'a Statement,
    parent: &ExecutionNamespace,
) -> Vec<(&'a Script, ExecutionNamespace)> {
    let context = match stmt {
        Statement::UpFrame {
            absolute,
            frame_shift,
            ..
        } => parent.selected_frame(*absolute, *frame_shift),
        _ => parent.clone(),
    };
    nested_bodies(stmt)
        .into_iter()
        .map(|body| (body, context.clone()))
        .collect()
}

/// Whether executing `script` in a runtime-selected current namespace can
/// resolve any command differently from the compiler's lexical fallback.
///
/// Direct literal call heads, typed-lowering dependencies, and recovered
/// literal command substitutions all enter through this single inventory.
/// Absolute heads are namespace-invariant; relative literal heads may be
/// shadowed by the runtime receiver/caller namespace. Computed heads are not a
/// lexical fallback dependency: the CFG already represents them as opaque
/// runtime dispatch and may still analyse facts on either side of that barrier.
#[must_use]
pub(crate) fn requires_runtime_command_namespace(
    script: &Script,
    registry: &CommandRegistry,
) -> bool {
    let mut pending = vec![(script, ExecutionNamespace::RuntimeSelected)];
    while let Some((script, execution_namespace)) = pending.pop() {
        if script
            .command_binding_sites
            .iter()
            .any(|site| execution_namespace.for_head(&site.binding.name).is_none())
        {
            return true;
        }
        for stmt in &script.statements {
            let specialised_binding = match stmt {
                Statement::AssignExpr {
                    command_binding, ..
                }
                | Statement::Return {
                    command_binding, ..
                } => command_binding.as_ref(),
                Statement::ExprEval {
                    command_binding, ..
                } => Some(command_binding),
                _ => None,
            };
            if specialised_binding
                .is_some_and(|binding| execution_namespace.for_head(&binding.name).is_none())
            {
                // A nested command consumed by typed lowering does not appear
                // in the expression/text surfaces below. Its binding metadata is
                // therefore the authoritative source head for the same
                // runtime-selected namespace proof as an ordinary Call.
                return true;
            }
            if let Statement::Call {
                command, tokens, ..
            }
            | Statement::Barrier {
                command, tokens, ..
            } = stmt
            {
                let needs_runtime_namespace = match tokens {
                    Some(tokens) if tokens.synthetic.is_some() => false,
                    Some(tokens) => tokens
                        .words()
                        .first()
                        .and_then(|head| {
                            crate::registry_invocation::invocation_word(head).literal()
                        })
                        .is_some_and(|head| execution_namespace.for_head(head).is_none()),
                    None => execution_namespace.for_head(command).is_none(),
                };
                if needs_runtime_namespace {
                    return true;
                }
            }
            let embedded = evaluated_command_substitutions(stmt, registry);
            if embedded.commands.iter().any(|command| {
                command
                    .first()
                    .and_then(CommandWord::literal)
                    .is_some_and(|head| execution_namespace.for_head(head).is_none())
            }) {
                return true;
            }
            pending.extend(nested_execution_bodies(stmt, &execution_namespace));
        }
    }
    false
}

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

fn collect_defs_from_script(script: &Script, defs: &mut Vec<String>, depth: u32) {
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

/// Out-variable names assigned by command substitutions appearing in
/// `condition` (an `if`/`while` condition expr).
///
/// A command that writes a variable named by one of its own arguments does
/// so *before* either branch runs, so a read of such a variable in the
/// guarded body is **not** read-before-set — the CFG records them as defs on
/// the synthetic `<cond>` statement so the def-use / W210 analysis sees the
/// write.  Which arguments those are is the registry's
/// [`ArgRole::VarWrite`] answer, never a name list here.
#[cfg(test)]
pub(crate) fn condition_command_out_vars(
    condition: &ExprNode,
    registry: &CommandRegistry,
) -> Vec<String> {
    let mut cmds = Vec::new();
    collect_expr_commands(condition, &mut cmds);
    let mut out = Vec::new();
    for cmd_text in &cmds {
        let trimmed = cmd_text.trim();
        let inner = trimmed
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .unwrap_or(trimmed);
        cmd_substitution_out_vars(&tokenise_to_words(inner, registry), registry, &mut out, 0);
    }
    out
}

/// A word usable as a variable name: a *literal* identifier (so `{script}`,
/// `[sub]`, quoted words, and — via [`CommandWord::substituted`] — the
/// run-time-computed `$ref` are all rejected).
///
/// The substitution check has to come from the token stream, not the text:
/// the word's *content* spelling drops the `$`, so `set $n 1` would otherwise
/// read as a definition of a variable called `n` and silence a genuine
/// warning about it.
#[cfg(test)]
fn is_bare_var_word(word: &CommandWord) -> bool {
    !word.expanded
        && !word.substituted
        && !word.text.is_empty()
        && word
            .text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
}

#[cfg(test)]
fn push_out_var(word: &CommandWord, out: &mut Vec<String>) {
    if is_bare_var_word(word) {
        let normalised = normalise_var_name(&word.text);
        if !normalised.is_empty() && !out.iter().any(|v| v == normalised) {
            out.push(normalised.to_owned());
        }
    }
}

/// Out-vars written by the command named by `words[0]`, appended to `out`.
///
/// Registry-first: the write positions come from
/// [`CommandRegistry::arg_indices_for_role`]`(…, `[`ArgRole::VarWrite`]`)`,
/// which is the same machinery the analyser's out-var handling and W210's
/// suppression harvest already consult.  It answers for every command the
/// registry knows — `catch` / `scan` / `gets` / `regexp`, plus
/// `set` / `append` / `lappend` / `incr` / `lset` / `regsub` / `lassign` /
/// `binary scan` / `dict update` / `array set` / … — so
/// `if {[set idx [lsearch $l foo]] > -1} {puts $idx}` does not look
/// read-before-set (tclsh 9.0.4 / 8.6.14 both print the index).
///
/// Bodies that run in the *caller's own* frame
/// ([`CommandRegistry::plain_body_arg_indices`] — `catch`'s script, and any
/// other `Plain` body) are descended into as well: their assignments are
/// (maybe) set once the substitution completes.  A `Structural` body
/// (`uplevel`, `namespace eval`, `proc`) writes somewhere else entirely and
/// is never harvested.
///
/// Destroyers ([`Traits::DESTROYS_VARIABLE`] — `unset`) are skipped: an
/// `unset` in a condition removes a variable rather than defining one, and
/// claiming it as a def would mask a genuine W213.
///
/// A **substituted command head** (`[$cmd length foo]`) is skipped for the
/// same reason: which command runs is run-time data, so nothing can be
/// claimed about what it writes.  The word's *content* spelling drops the
/// `$`, so without this check the head of `[$set length foo]` resolves as the
/// builtin `set` and the harvest invents a definition of `length` — enough to
/// swallow a genuine warning.  tclsh 9.0.4 / 8.6.14: `proc f {set} {if
/// {[$set length foo]} {puts $length}}; f string` errors `can't read
/// "length": no such variable`, so the read really is unsafe.
///
/// Suppress-only: over-collection avoids false warnings, under-collection
/// leaves the status quo — but a name harvested from the *wrong* command is
/// not over-collection, it is a wrong fact, so the skip is a correctness
/// requirement rather than a precision nicety.
#[cfg(test)]
fn cmd_substitution_out_vars(
    words: &[CommandWord],
    registry: &CommandRegistry,
    out: &mut Vec<String>,
    depth: u32,
) {
    // Native-stack safety net: `catch {catch {catch …}}` nests
    // body text inside one word, so the bracket-text cap applies. Past it,
    // stop harvesting — under-collection is the safe direction here.
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        return;
    }
    let Some((cmd_word, arg_words)) = words.split_first() else {
        return;
    };
    let Some(cmd) = cmd_word.literal() else {
        return;
    };
    // Expansion changes the argv shape. This suppress-only harvest must
    // abstain rather than assign source positions to post-expansion words.
    if arg_words.iter().any(|word| word.expanded) {
        return;
    }
    let args: Vec<&str> = arg_words.iter().map(|w| w.text.as_str()).collect();
    // A command the document's release does not have writes nothing: under
    // Tcl 8.4 `lassign` raises `invalid command name`, so its words name no
    // place. Availability is the registry's answer, asked by resolving the
    // invocation under the registry's own surface — which a profile-less
    // registry leaves blind, answering for every release.
    if registry
        .resolve_invocation(cmd, &args, registry.own_surface_query())
        .is_none()
    {
        return;
    }
    if registry
        .get(cmd)
        .is_some_and(|s| s.traits.contains(Traits::DESTROYS_VARIABLE))
    {
        return;
    }
    for idx in registry.arg_indices_for_role(cmd, &args, ArgRole::VarWrite) {
        if let Some(w) = arg_words.get(idx) {
            push_out_var(w, out);
        }
    }
    for idx in registry.plain_body_arg_indices(cmd, &args) {
        // The word's *content* spelling already has the enclosing braces
        // stripped, so it is the body script text as written.  A body reached
        // through a substitution (`catch $script`) is unknown text and
        // contributes nothing.
        if let Some(body) = arg_words.get(idx).filter(|w| !w.substituted) {
            script_text_out_vars_at(&body.text, registry, out, depth);
        }
    }
}

#[cfg(test)]
fn script_text_out_vars_at(
    body: &str,
    registry: &CommandRegistry,
    out: &mut Vec<String>,
    depth: u32,
) {
    // The document's grammar, from the registry's profile — as
    // the standalone expression inventory does.
    for words in tokenise_command_words(body, LexerConfig::for_profile(registry.profile())) {
        cmd_substitution_out_vars(&words, registry, out, depth + 1);
    }
}

/// Return `true` if *expr* contains at least one command
/// substitution (`[cmd ...]`). Used by CFG lowering to decide
/// whether a branch condition needs a synthetic `<cond>` placeholder
/// for emission-time startCommand wrapping.
#[must_use]
pub fn expr_has_command(expr: &ExprNode) -> bool {
    // Public entry: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`expr_has_command_at`].
    expr_has_command_at(expr, 0)
}

fn expr_has_command_at(expr: &ExprNode, depth: u32) -> bool {
    // Native-stack safety net: past the cap, assume "yes, has a
    // command substitution" — the conservative direction, since callers use
    // this to decide whether a branch condition needs a synthetic `<cond>`
    // placeholder / startCommand wrapping; a false `true` only adds a
    // harmless wrapper, never drops a needed one.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command_at(left, depth + 1) || expr_has_command_at(right, depth + 1)
        }
        ExprNode::Unary { operand, .. } => expr_has_command_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command_at(condition, depth + 1)
                || expr_has_command_at(true_branch, depth + 1)
                || expr_has_command_at(false_branch, depth + 1)
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| expr_has_command_at(a, depth + 1)),
        // A `"…"` operand substitutes, so a `[cmd]` in it runs: O126 dropped
        // `set y [expr {"[incr x]"}]` as an unused store, losing the `incr`
        // tclsh 8.6.18 performs (#2227, found in review).
        ExprNode::String { text, .. } => crate::word_subst::quoted_operand_body(text).is_some(),
        ExprNode::Literal { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => false,
    }
}

/// Recursively collect `ExprNode::Command` text from an expression tree.
///
/// Respects short-circuit evaluation: for `&&`/`||`, the RHS is
/// always included (conservative — we do not attempt compile-time
/// constant evaluation).
pub(crate) fn collect_expr_commands(expr: &ExprNode, out: &mut Vec<String>) {
    // Entry point: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`collect_expr_commands_at`].
    collect_expr_commands_at(expr, out, 0);
}

/// Tcl source fragments owned by `stmt` whose `[...]` substitutions execute
/// while the statement is evaluated, plus whether a bounded expression walk
/// had to give up.
///
/// This is the shared statement-shape inventory for effect consumers.  It is
/// intentionally conservative: where structured IR does not retain enough
/// word quoting to prove a fragment inert, the fragment is included.  A false
/// positive only widens an effect summary; omitting an executed substitution
/// can make propagation carry a stale value across an arbitrary command.
/// Callers still lex the returned fragments with the active document's
/// [`LexerConfig`] before interpreting command words.
pub(crate) struct EvaluatedCommandSubstitutionSurfaces<'a> {
    /// Source fragments which may contain executed command substitutions.
    pub texts: Vec<&'a str>,
    /// The expression walk exceeded its shared recursion limit.
    pub opaque: bool,
    /// At least one collected surface is conditionally evaluated by Tcl
    /// short-circuit/ternary control flow or by a later `elseif` clause.
    pub conditional: bool,
}

fn push_command_substitution_text<'a>(text: &'a str, out: &mut Vec<&'a str>) {
    if text.contains('[') {
        out.push(text);
    }
}

fn collect_expr_command_surface_refs<'a>(
    expr: &'a ExprNode,
    out: &mut Vec<&'a str>,
    opaque: &mut bool,
    conditional: &mut bool,
    depth: u32,
) {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        *opaque = true;
        return;
    }
    let push_text = |text: &'a str, out: &mut Vec<&'a str>| {
        if text.contains('[') {
            out.push(text);
        }
    };
    match expr {
        ExprNode::Command { text, .. } => push_text(text, out),
        ExprNode::Raw { text } => {
            let before = out.len();
            push_text(text, out);
            if out.len() != before {
                // A raw expression has no retained evaluation tree, so the
                // collector cannot prove its substitutions unconditional.
                *conditional = true;
            }
        }
        ExprNode::Binary { op, left, right } => {
            collect_expr_command_surface_refs(left, out, opaque, conditional, depth + 1);
            let before_right = out.len();
            collect_expr_command_surface_refs(right, out, opaque, conditional, depth + 1);
            if matches!(
                op,
                crate::expr_ast::BinOp::And
                    | crate::expr_ast::BinOp::Or
                    | crate::expr_ast::BinOp::WordAnd
                    | crate::expr_ast::BinOp::WordOr
            ) && out.len() != before_right
            {
                *conditional = true;
            }
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_command_surface_refs(operand, out, opaque, conditional, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_command_surface_refs(condition, out, opaque, conditional, depth + 1);
            let before_arms = out.len();
            collect_expr_command_surface_refs(true_branch, out, opaque, conditional, depth + 1);
            collect_expr_command_surface_refs(false_branch, out, opaque, conditional, depth + 1);
            if out.len() != before_arms {
                *conditional = true;
            }
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_command_surface_refs(arg, out, opaque, conditional, depth + 1);
            }
        }
        // A word already reduced to its value. Braced, its brackets are data
        // and provably never run, so it is inert — the one case this may skip
        // without guessing. Unbraced, whatever substitution the word still owes
        // *is* executed at run time, and omitting an executed substitution is
        // the failure this inventory exists to avoid, so its text is collected
        // like any other surface.
        ExprNode::CompiledWord { text, braced } => {
            if !*braced {
                push_text(text, out);
            }
        }
        // `"…"` substitutes inside an expression and `{…}` does not, and the
        // variant carries its delimiters, so the delimiter decides. Reading
        // every string as inert hid the write in
        // `set y [expr {"[incr x]" + 0}]`: tclsh 8.6.18 prints `2` then `2`,
        // and O102 forwarded the stale `1` across it exactly as it did for a
        // braced operand before #2141 (#2118, found in review).
        ExprNode::String { text, .. } => {
            if let Some(inner) = crate::word_subst::quoted_operand_body(text) {
                out.push(inner);
            }
        }
        ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
    }
}

fn collect_control_command_substitution_surfaces<'a>(
    stmt: &'a Statement,
    texts: &mut Vec<&'a str>,
    opaque: &mut bool,
    conditional: &mut bool,
) {
    match stmt {
        Statement::If { clauses, .. } => {
            for (index, clause) in clauses.iter().enumerate() {
                let before = texts.len();
                collect_expr_command_surface_refs(&clause.condition, texts, opaque, conditional, 0);
                if index != 0 && texts.len() != before {
                    *conditional = true;
                }
            }
        }
        Statement::For { condition, .. } | Statement::While { condition, .. } => {
            collect_expr_command_surface_refs(condition, texts, opaque, conditional, 0);
        }
        Statement::Foreach { iterators, .. } => {
            for iterator in iterators.iter().filter(|iterator| !iterator.list_braced) {
                push_command_substitution_text(&iterator.list_arg, texts);
            }
        }
        Statement::Catch {
            raw_args, tokens, ..
        } => {
            for arg in unbraced_words(raw_args, tokens.as_ref()) {
                push_command_substitution_text(arg, texts);
            }
        }
        Statement::Try { raw_args, .. } => {
            // `Try` does not retain per-word quoting. Include every raw word;
            // literal-body substitutions are harmless over-approximation and
            // the recursively lowered bodies supply the exact coverage.
            for arg in raw_args {
                push_command_substitution_text(arg, texts);
            }
        }
        Statement::Switch {
            subject,
            subject_braced,
            arms,
            ..
        } => {
            if !subject_braced {
                push_command_substitution_text(subject, texts);
            }
            for arm in arms.iter().filter(|arm| !arm.pattern_braced) {
                push_command_substitution_text(&arm.pattern, texts);
            }
        }
        _ => {}
    }
}

/// Enumerate every command-substitution-bearing value/expression surface of
/// one IR statement.
///
/// Nested *script bodies* are deliberately excluded: callers already recurse
/// through [`nested_bodies`] with the correct execution namespace.  This
/// helper covers the statement's own evaluated words and expression nodes.
#[must_use]
pub(crate) fn evaluated_command_substitution_surfaces<'a>(
    stmt: &'a Statement,
    registry: &CommandRegistry,
) -> EvaluatedCommandSubstitutionSurfaces<'a> {
    let mut texts = Vec::new();
    let mut opaque = false;
    let mut conditional = false;
    match stmt {
        Statement::NativeCall { .. } => opaque = true,
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            collect_expr_command_surface_refs(expr, &mut texts, &mut opaque, &mut conditional, 0);
        }
        Statement::AssignValue { value, .. } => push_command_substitution_text(value, &mut texts),
        Statement::Incr { amount, .. } => {
            if let Some(amount) = amount {
                push_command_substitution_text(amount, &mut texts);
            }
        }
        Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } => {
            for arg in invoked_word_surfaces(
                canonical_command.as_deref().unwrap_or(command.as_str()),
                args,
                tokens.as_ref(),
                registry,
            ) {
                push_command_substitution_text(arg, &mut texts);
            }
        }
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            if !braced && let Some(value) = value {
                push_command_substitution_text(value, &mut texts);
            }
            if let Some(expr) = expr {
                collect_expr_command_surface_refs(
                    expr,
                    &mut texts,
                    &mut opaque,
                    &mut conditional,
                    0,
                );
            }
        }
        _ => collect_control_command_substitution_surfaces(
            stmt,
            &mut texts,
            &mut opaque,
            &mut conditional,
        ),
    }
    EvaluatedCommandSubstitutionSurfaces {
        texts,
        opaque,
        conditional,
    }
}

fn collect_expr_commands_at(expr: &ExprNode, out: &mut Vec<String>, depth: u32) {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the command texts gathered so far is the safe fallback
    // (substitutions buried deeper than the cap are not collected; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match expr {
        ExprNode::Command { text, .. } => {
            out.push(text.clone());
        }
        ExprNode::Binary { left, right, .. } => {
            collect_expr_commands_at(left, out, depth + 1);
            collect_expr_commands_at(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_commands_at(operand, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_commands_at(condition, out, depth + 1);
            collect_expr_commands_at(true_branch, out, depth + 1);
            collect_expr_commands_at(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_commands_at(arg, out, depth + 1);
            }
        }
        _ => {}
    }
}

/// Tokenise source text into a flat list of words (single command).
///
/// Split under the document's grammar, taken from the dialect-selected
/// `registry`'s own profile — the same standalone command grammar,
/// so one document is tokenised one way whichever harvest asks.
#[cfg(test)]
fn tokenise_to_words(source: &str, registry: &CommandRegistry) -> Vec<CommandWord> {
    tokenise_command_words(source, LexerConfig::for_profile(registry.profile()))
        .into_iter()
        .next()
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
enum WrittenWordGeometry {
    Missing,
    Present,
}

/// One word of a tokenised command, in both spellings its consumers need.
///
/// The segmenter's reconstructed word text canonicalises a substitution to its
/// braced spelling (`$x` → `${x}`) and de-braces a braced word (`{s}` → `s`),
/// which is what a name-harvesting caller wants but which erases the
/// distinction that decides whether a word *is* a name: `set {$x} 1` names a
/// variable literally called `$x`.  Carrying the written spelling plus the two
/// quoting facts lets every consumer ask its own question of one tokenisation.
#[derive(Debug, Clone)]
pub(crate) struct CommandWord {
    /// The segmenter's reconstructed word text — `$x` → `${x}`, `{s}` → `s`.
    /// The same spelling [`crate::ir::Statement`] arguments carry, so a
    /// consumer reads one text convention whichever side it arrives from.
    pub text: String,
    /// The word's **verbatim** source spelling, its whole written span — `$x`
    /// stays `$x`; original delimiters and expansion markers remain complete.
    /// Missing written geometry is unavailable, never a static literal.
    pub raw: String,
    geometry: WrittenWordGeometry,
    /// Some constituent fragment is a `$` / `[` substitution, so the word's
    /// value is not its source text.
    pub substituted: bool,
    /// Tcl `{*}` expands this source word into zero or more argv entries.
    /// This is an argv-shape fact independent of whether the wrapped word
    /// itself substitutes (`{*}$x`) or is literal (`{*}{a b}`).
    pub expanded: bool,
    /// The word is a single brace-quoted token (`{…}`): Tcl leaves its
    /// content literal, so a `$` inside is source text, not a substitution.
    pub braced_literal: bool,
}

impl CommandWord {
    fn has_source_geometry(&self) -> bool {
        matches!(self.geometry, WrittenWordGeometry::Present)
    }

    /// Project this source word into the registry's value-conservative
    /// invocation vocabulary. Expansion takes precedence because it changes
    /// argv shape even when the wrapped value is otherwise literal.
    #[must_use]
    pub(crate) fn invocation_word(&self) -> InvocationWord<'_> {
        if !self.has_source_geometry() {
            InvocationWord::Dynamic
        } else if self.expanded {
            InvocationWord::Expanded
        } else if self.substituted {
            InvocationWord::Dynamic
        } else {
            InvocationWord::Literal(&self.text)
        }
    }

    /// The single, statically-known Tcl value contributed by this word.
    #[must_use]
    pub(crate) fn literal(&self) -> Option<&str> {
        self.invocation_word().literal()
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

/// Tokenise source text into a list of commands, each a list of words.
///
/// One splitter, one dialect: this is
/// [`crate::segmenter::segment_commands_with_offset_and_config`] mapped onto
/// the four facts the name-harvesting consumers ask for.  `config` decides
/// where the word boundaries fall — `{*}` expands only where the dialect has
/// it (never under Tcl 8.4 or iRules), and only iRules treats `}{` as a word
/// separator, so `cmd {a}{b}` is two words in Tcl and three in an iRule.  A
/// second, differently-configured tokenisation would disagree with the IR
/// about *which* argument sits in a variable-name position, which is a wrong
/// fact rather than a coarse one for a caller such as
/// [`crate::dynamic_names`]. These callers supply an extracted command or
/// materialized body, so local parsing selects `NestedScript` while command
/// words and value rules retain the complete caller configuration.
pub(crate) fn tokenise_command_words(source: &str, config: LexerConfig) -> Vec<Vec<CommandWord>> {
    let sm = SourceMap::new(source);
    crate::segmenter::segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(config),
    )
    .iter()
    .filter(|command| !command.argv.is_empty())
    .map(|command| command_words(&sm, config, command))
    .collect()
}

/// Where one recovered command sits in [`EvaluatedCommandSubstitutions`]'
/// inventories, in the order the walk met it.
#[derive(Clone, Copy)]
enum CommandSubstitutionIndex {
    /// An index into [`EvaluatedCommandSubstitutions::commands`].
    Ordinary(usize),
    /// An index into [`EvaluatedCommandSubstitutions::in_frame_commands`].
    InFrame(usize),
}

/// Registry/dialect-shaped command invocations recovered from every evaluated
/// `[...]` surface owned by one statement.
#[derive(Default)]
pub(crate) struct EvaluatedCommandSubstitutions {
    /// Commands in evaluation order within each recovered bracket script.
    pub commands: Vec<Vec<CommandWord>>,
    /// Commands reached only by descending into a brace-quoted word the
    /// callee evaluates **in this frame**: an expression — the `[incr x]` of
    /// `puts [expr {$x + [incr x]}]` — or a protected script, the body of a
    /// `catch` — the `incr x` of `[catch {incr x}]`.
    ///
    /// Kept apart from [`Self::commands`] rather than merged into it because
    /// the two answer different questions. A consumer asking *what cells does
    /// this statement write* wants both: the write is real either way (#2141).
    /// A consumer asking *which procedures does this body call* — the
    /// global-write summary's call graph — reads `commands` alone, because a
    /// recursive callee's summary is opaque and folding that opacity in here
    /// would put a frame barrier on `return [expr {[fib $n] + 1}]` that the
    /// unbraced `return [fib $n]` earns for a reason unrelated to this word.
    /// Closing that second gap means fixing the recursion summary first; the
    /// two are separate, and this split says which is which.
    pub in_frame_commands: Vec<Vec<CommandWord>>,
    /// The commands of [`Self::in_frame_commands`] reached inside a protected
    /// script — the body of a `catch` or a `try` — with the substitutions and
    /// expression words in it. Each runs only where every command before it
    /// in the script completed normally, and the error that stops the rest
    /// is absorbed, not raised: a write among them may not happen, and the
    /// place keeps what it held where the script stopped first.
    pub protected_commands: Vec<Vec<CommandWord>>,
    /// The brace-quoted words themselves — the text of each `{$x + [incr x]}`
    /// or `{incr x; puts $y}` a recovered command evaluates in this frame,
    /// which is where the variable reads of that word sit: no command word of
    /// [`Self::in_frame_commands`] holds them.
    pub in_frame_texts: Vec<String>,
    /// Indices into both inventories in the order the shared walk evaluates
    /// them. Keeping the classifications separate must not move an earlier
    /// expression's nested invocation after a later sibling substitution.
    evaluation_order: Vec<CommandSubstitutionIndex>,
    /// A malformed fragment, a recursion-limit hit, or a protected script that
    /// is run-time data (`[catch $script]`) prevented complete recovery.
    pub opaque: bool,
}

impl EvaluatedCommandSubstitutions {
    /// Record `words`, reached as `reach` says, after every command the walk
    /// met before it.
    fn record(&mut self, reach: Reach, words: Vec<CommandWord>) {
        if reach == Reach::Statement {
            self.evaluation_order
                .push(CommandSubstitutionIndex::Ordinary(self.commands.len()));
            self.commands.push(words);
            return;
        }
        if reach == Reach::Protected {
            self.protected_commands.push(words.clone());
        }
        self.evaluation_order
            .push(CommandSubstitutionIndex::InFrame(
                self.in_frame_commands.len(),
            ));
        self.in_frame_commands.push(words);
    }

    /// Every command the statement may run, whether or not it took an in-frame
    /// word to reach, in Tcl evaluation order. Variable-effect and
    /// binding-transition consumers use this view; call-graph consumers may
    /// still select the ordinary inventory without changing its classification.
    pub(crate) fn all_commands(&self) -> impl Iterator<Item = &Vec<CommandWord>> {
        self.evaluation_order.iter().map(|index| match *index {
            CommandSubstitutionIndex::Ordinary(index) => &self.commands[index],
            CommandSubstitutionIndex::InFrame(index) => &self.in_frame_commands[index],
        })
    }
}

/// Recover the commands executed by `[...]` substitutions in `stmt` using the
/// exact lexer grammar selected for its registry profile.
///
/// This is effect analysis, so failure widens through [`Self::opaque`] rather
/// than being interpreted as an empty command list.  Dynamic/substituted heads
/// remain represented by [`CommandWord::substituted`]; consumers decide which
/// state domain that unknown identity invalidates.
#[must_use]
pub(crate) fn evaluated_command_substitutions(
    stmt: &Statement,
    registry: &CommandRegistry,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_heads(stmt, registry, None)
}

/// [`evaluated_command_substitutions`] with a caller-supplied head resolver.
///
/// A caller holding the module's command bindings can answer *which command a
/// recovered head actually reaches*, which the registry alone cannot: after
/// `interp alias {} e {} expr`, `[e {$x + [incr x]}]` runs `expr` on a word
/// the raw spelling `e` has no role for, and the `[incr x]` in it was missed.
/// Callers without bindings pass `None` and get the raw spelling, as before.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_heads(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_replay(stmt, registry, heads, None)
}

/// Recover invocations while replaying each completed command before the next
/// head's expression roles are queried. Both callbacks share the caller's
/// binding state, so earlier alias transitions affect later descent.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_replay(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
) -> EvaluatedCommandSubstitutions {
    let surfaces = evaluated_command_substitution_surfaces(stmt, registry);
    command_substitutions_in_surfaces_with_replay(
        &surfaces.texts,
        surfaces.opaque,
        surfaces.conditional,
        registry,
        heads,
        observe,
    )
}

/// Recover source substitutions with the actual availability and lexer input.
/// Missing or foreign metadata is opaque; no entered script/frame is inferred.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_heads_and_metadata_context(
        stmt, registry, None, metadata, config,
    )
}

/// The same source inventory with a genuine module target/prefix resolver.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_heads_and_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    evaluated_command_substitutions_with_replay_and_metadata_context(
        stmt, registry, heads, None, metadata, config,
    )
}

/// Replay source substitutions under retained availability and lexical policy.
#[must_use]
pub(crate) fn evaluated_command_substitutions_with_replay_and_metadata_context(
    stmt: &Statement,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return EvaluatedCommandSubstitutions {
            opaque: true,
            ..EvaluatedCommandSubstitutions::default()
        };
    };
    let mut surfaces = evaluated_command_substitution_surfaces(stmt, registry);
    if let Statement::Call { args, tokens, .. } | Statement::Barrier { args, tokens, .. } = stmt {
        // Every unbraced original argv word substitutes before dispatch,
        // including a structural command's dynamic body operand.
        surfaces.texts = unbraced_words(args, tokens.as_ref())
            .filter(|arg| arg.contains('['))
            .collect();
    }
    recover_substitution_surfaces(
        &surfaces.texts,
        surfaces.opaque,
        SubstitutionWalkContext {
            config,
            registry,
            heads,
            observe,
            conditional: surfaces.conditional,
            metadata: Some(metadata),
        },
    )
}

/// The command a recovered substitution head reaches, and the words an alias
/// chain prepends ahead of the ones written at the call site.
pub(crate) struct ResolvedEmbeddedHead {
    /// The spelling whose registry descriptor answers for this call.
    pub command: String,
    /// Arguments the alias chain supplies before the written ones, which
    /// shift every written word's index in the callee's own argv.
    pub prepended: Vec<String>,
}

/// Resolves a recovered substitution's head. `None` for a spelling with no
/// single statically known registry-backed target — the raw spelling is then
/// used, exactly as when no resolver is supplied.
pub(crate) type EmbeddedHeadResolver<'a> = &'a dyn Fn(&str) -> Option<ResolvedEmbeddedHead>;
/// Observes a completed command and whether expression control may skip it.
pub(crate) type EmbeddedCommandObserver<'a> = &'a dyn Fn(&[CommandWord], bool);

#[derive(Clone, Copy)]
struct SubstitutionWalkContext<'a> {
    config: LexerConfig,
    registry: &'a CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    heads: Option<EmbeddedHeadResolver<'a>>,
    observe: Option<EmbeddedCommandObserver<'a>>,
    conditional: bool,
}

/// Where a recovered command runs, relative to the statement whose
/// substitutions the walk recovers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// A substitution the statement runs itself.
    Statement,
    /// A word a recovered command evaluates in this frame: an expression.
    InFrame,
    /// A protected script, or a word or substitution inside one.
    Protected,
}

fn walk_text(
    text: &str,
    context: &SubstitutionWalkContext<'_>,
    depth: u32,
    reach: Reach,
    out: &mut EvaluatedCommandSubstitutions,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.opaque = true;
        return;
    }
    let source_map = SourceMap::new(text);
    let Ok(tokens) = tcl_lexer::Lexer::with_config(
        text,
        tcl_lexer::SourceSegmentationPurpose::NestedScript.config(context.config),
    )
    .tokenise_all() else {
        out.opaque = true;
        return;
    };
    for token in tokens.iter().filter(|token| token.kind == TokenType::Cmd) {
        let inner = source_map.token_text(*token);
        let recovered = tokenise_command_words(inner, context.config);
        let protected = reach == Reach::Protected;
        for words in recovered {
            // Tcl substitutes command words from left to right before it
            // invokes the enclosing command. A nested `[rename …]` therefore
            // changes the binding that resolves its outer command; recording
            // the outer command first reverses that observable order.
            for word in words.iter().filter(|word| !word.braced_literal) {
                walk_text(&word.text, context, depth + 1, reach, out);
            }
            walk_in_frame_words(&words, context, (depth, protected), out);
            if let Some(observe) = context.observe {
                observe(&words, context.conditional);
            }
            out.record(reach, words);
        }
    }
}

/// The argument indices `lookup` evaluates **as an expression in this frame**,
/// as indices into its own argv-minus-head.
///
/// The single owner of that question, shared by the two walks that ask it: the
/// variable-effect walk ([`walk_in_frame_words`]) and the call-site walk
/// ([`crate::word_subst::lifted_calls_with_registry`]). The two disagreeing
/// about which commands a statement runs is what let `return [expr {[fact …]}]`
/// hide a recursive call from the caller-evidence scan (#2118).
///
/// Two shapes reach the same expression, and both come from the registry
/// rather than from the spelling `expr`:
///
/// * a single [`tcl_registry::ArgRole::Expr`] word — `[expr {$x + 1}]`;
/// * every word of a command that
///   [concatenates its arguments into one expression](tcl_registry::Traits::EXPR_CONCATENATES_ARGS).
///   `expr 1 + {[incr x]}` joins its words into `1 + [incr x]` and then
///   parses *that*, so the braced word's `[…]` runs: tclsh 8.6.18 and
///   9.0.4 both print `3` then `2` for
///   `set x 1; puts [expr 1 + {[incr x]}]; puts $x`.
///
/// Only `Expr` is named, never `Body`: a body word runs in this frame too but
/// may bind names of its own, which a flat command list cannot represent.
///
/// Read through the document's command *surface*, not the bare catalogue: a
/// `# tcl-lsp: stub myexpr {value:expr}` declares an expression word exactly
/// as a shipped command does, and lowering honours that role, so `myexpr
/// {[id 9]}` runs the call it holds. A declaration only ever *adds* a role
/// position, so this can widen the answer and never narrow it.
pub(crate) fn in_frame_expression_arg_indices(
    lookup: &str,
    args: &[&str],
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
) -> Vec<usize> {
    let mut descend: Vec<usize> =
        surface.arg_indices_for_role(lookup, args, tcl_registry::ArgRole::Expr);
    if surface
        .commands()
        .get_for_surface(lookup, surface.commands().own_surface_query())
        .is_some_and(|spec| {
            spec.traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
        })
    {
        descend.extend(0..args.len());
    }
    descend.sort_unstable();
    descend.dedup();
    descend
}

/// Descend the brace-quoted words a recovered command evaluates **in this
/// frame**: an expression, and a protected script.
///
/// The word lexer is right to stop at `{…}` — the *command* parser
/// substitutes nothing there — but `expr` re-parses that text as an
/// expression, and a `[…]` in it is a substitution the statement really
/// runs. `puts [expr {$x + [incr x]}]` writes `x` exactly as
/// `puts [incr x]` does, and without this descent the write was invisible
/// and O102 forwarded a stale literal across it (#2141).
///
/// Which words are an expression is [`in_frame_expression_arg_indices`]' to
/// answer; this walks the brace-quoted ones among them.
///
/// A script the call's clause plan places as protected
/// ([`tcl_registry::ClauseTiming::Protected`]: the body of `catch` and of
/// `try`) runs once in this frame whatever it does, and its completion is
/// observed rather than propagated, so what it writes and reads is the
/// statement's: `set c [catch {incr x}]` writes `x`, and an `if` that reads
/// it was folded on the value `x` held before (#2231). Its commands are
/// descended, and their own protected scripts and expression words with
/// them, where the script is known text: brace-quoted, or a quoted or bare
/// word that substitutes nothing (`[catch "incr x"]`). A script that is
/// run-time data (`[catch $script]`) is unreadable, and the walk is marked
/// [`EvaluatedCommandSubstitutions::opaque`]. Every other body is left: it
/// may run zero or many times, in another frame, or bind names of its own,
/// which a flat command list cannot represent. What the walk reaches inside
/// a protected script, or inside a word of a command `in_protected` says
/// is in one, is also [`EvaluatedCommandSubstitutions::protected_commands`].
fn walk_in_frame_words(
    words: &[CommandWord],
    context: &SubstitutionWalkContext<'_>,
    (depth, in_protected): (u32, bool),
    out: &mut EvaluatedCommandSubstitutions,
) {
    if MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
        out.opaque = true;
        return;
    }
    let Some(head) = words.first().and_then(CommandWord::literal) else {
        return;
    };
    let resolved = context.heads.and_then(|resolve| resolve(head));
    if context.metadata.is_some() && context.heads.is_some() && resolved.is_none() {
        out.opaque = true;
        return;
    }
    let (lookup, prepended) = resolved.as_ref().map_or((head, [].as_slice()), |target| {
        (target.command.as_str(), target.prepended.as_slice())
    });
    // The callee's own argv: whatever an alias chain prepends, then the
    // words written here. Every role index the registry answers with is
    // an index into *that*, so a written word sits at `index - shift`.
    let shift = prepended.len();
    let mut args: Vec<&str> = prepended.iter().map(String::as_str).collect();
    args.extend(
        words
            .iter()
            .skip(1)
            .map(|word| word.literal().unwrap_or("")),
    );

    // The variable-effect walk is reached from consumers that hold only a
    // catalogue, so it asks the same owner with no declarations attached.
    let Some((indices, concatenated)) =
        source_expression_indices(words, lookup, prepended, &args, context, out)
    else {
        return;
    };
    let expression_texts = if concatenated {
        vec![args.join(" ")]
    } else {
        indices
            .iter()
            .filter_map(|&index| args.get(index).map(|arg| (*arg).to_owned()))
            .collect()
    };
    let mut conditional = context.conditional
        || (concatenated && words.iter().skip(1).any(|word| word.literal().is_none()));
    let mut opaque = false;
    for text in &expression_texts {
        let expr = if let Some(metadata) = context.metadata {
            let Some(dialect) = source_substitution_dialect(metadata) else {
                opaque = true;
                conditional = true;
                continue;
            };
            let mut parser = dialect.expression_parse_context(
                metadata
                    .source_analysis_input()
                    .map(crate::analyser::ResolvedAnalysisInput::unit_profile),
            );
            parser.lexer_grammar = context.config.grammar_over(parser.lexer_grammar);
            crate::expr_parser::parse_expr_with_syntax_context(text, &parser)
        } else {
            crate::expr_parser::parse_expr(
                text,
                context.registry.profile().map(|profile| profile.name),
            )
        };
        collect_expr_command_surface_refs(&expr, &mut Vec::new(), &mut opaque, &mut conditional, 0);
    }
    out.opaque |= opaque;
    let expression_context = SubstitutionWalkContext {
        conditional,
        ..*context
    };
    for index in indices {
        // A prepended word is a value the alias already holds, not source
        // this call substitutes.
        let Some(source_index) = index.checked_sub(shift) else {
            continue;
        };
        // An unbraced expression word already substituted at the command
        // level, so the enclosing walk has seen its `[…]` and descending
        // again would double-count the effect.
        let Some(word) = words
            .get(source_index + 1)
            .filter(|word| word.braced_literal)
        else {
            continue;
        };
        out.in_frame_texts.push(word.text.clone());
        let reach = if in_protected {
            Reach::Protected
        } else {
            Reach::InFrame
        };
        walk_text(&word.text, &expression_context, depth + 1, reach, out);
    }

    walk_protected_scripts(words, (lookup, &args, shift), context, depth, out);
}

/// Descend the scripts the call `words` runs as protected — the ones its
/// clause plan places at [`tcl_registry::ClauseTiming::Protected`], the body
/// of a `catch` or a `try` — where each is known text ([`walk_in_frame_words`]
/// says which). `callee` is the command the call reaches, its argv after any
/// words an alias prepends, and how many those are.
fn walk_protected_scripts(
    words: &[CommandWord],
    (lookup, args, shift): (&str, &[&str], usize),
    context: &SubstitutionWalkContext<'_>,
    depth: u32,
    out: &mut EvaluatedCommandSubstitutions,
) {
    let protected: Vec<usize> = context
        .registry
        .clause_plan(lookup, args)
        .map(|plan| {
            plan.clauses
                .iter()
                .filter(|clause| clause.timing == tcl_registry::ClauseTiming::Protected)
                .flat_map(|clause| clause.operands(tcl_registry::ArgRole::Body))
                .collect()
        })
        .unwrap_or_default();
    for index in protected {
        let Some(source_index) = index.checked_sub(shift) else {
            continue;
        };
        let Some(word) = words.get(source_index + 1) else {
            continue;
        };
        // The script is the text of a brace-quoted word, or of a quoted or
        // bare word that substitutes nothing and has no escape to read. A word
        // that substitutes, or is expanded, is run-time data: the script may
        // write any name, so the walk is incomplete.
        let script = if word.braced_literal {
            Some(word.text.as_str())
        } else {
            word.literal().filter(|_| !word.raw.contains('\\'))
        };
        let Some(script) = script else {
            out.opaque = true;
            continue;
        };
        out.in_frame_texts.push(script.to_owned());
        // The script's commands run in order, each after the substitutions
        // of its own words, and any of them may be skipped: an error before
        // it stops the script.
        let script_context = SubstitutionWalkContext {
            conditional: true,
            ..*context
        };
        for inner in tokenise_command_words(script, context.config) {
            for word in inner.iter().filter(|word| !word.braced_literal) {
                walk_text(
                    &word.text,
                    &script_context,
                    depth + 1,
                    Reach::Protected,
                    out,
                );
            }
            walk_in_frame_words(&inner, &script_context, (depth + 1, true), out);
            if let Some(observe) = script_context.observe {
                observe(&inner, script_context.conditional);
            }
            out.record(Reach::Protected, inner);
        }
    }
}

fn source_substitution_dialect(
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) -> Option<tcl_registry::InvocationDialect> {
    metadata
        .source_analysis_input()
        .map(|input| tcl_registry::InvocationDialect::of_profile(input.unit_profile()))
        .or_else(|| {
            metadata
                .context()
                .environment
                .point()
                .map(tcl_registry::InvocationDialect::of_point)
        })
}

fn source_expression_indices(
    words: &[CommandWord],
    lookup: &str,
    prepended: &[String],
    args: &[&str],
    context: &SubstitutionWalkContext<'_>,
    out: &mut EvaluatedCommandSubstitutions,
) -> Option<(Vec<usize>, bool)> {
    let selected = if let Some(metadata) = context.metadata {
        let mut effective: Vec<_> = prepended
            .iter()
            .map(|word| InvocationWord::Literal(word))
            .collect();
        effective.extend(words.iter().skip(1).map(CommandWord::invocation_word));
        let mut invocation_words =
            InvocationWords::structured(InvocationWord::Literal(lookup), &effective);
        if let Some(dialect) = source_substitution_dialect(metadata) {
            invocation_words = invocation_words.with_dialect(dialect);
        }
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.registry,
                Some(metadata.context()),
                invocation_words,
                metadata.context().authoring_query().realm,
            );
        let Some(invocation) = resolution.resolved() else {
            out.opaque = true;
            return None;
        };
        let (roles, complete) = invocation.argument_roles();
        out.opaque |= !complete;
        let concatenated = invocation
            .semantics
            .traits
            .contains(Traits::EXPR_CONCATENATES_ARGS);
        let mut indices: Vec<_> = roles
            .into_iter()
            .filter_map(|(index, role)| {
                (role == ArgRole::Expr)
                    .then_some(invocation.semantics.argument_offset + usize::from(index))
            })
            .collect();
        if concatenated {
            indices.extend(0..args.len());
        }
        indices.sort_unstable();
        indices.dedup();
        (indices, concatenated)
    } else {
        let surface = tcl_registry::model::DocumentCommandSurface::new(context.registry, None);
        (
            in_frame_expression_arg_indices(lookup, args, &surface),
            surface
                .commands()
                .get(lookup)
                .is_some_and(|spec| spec.traits.contains(Traits::EXPR_CONCATENATES_ARGS)),
        )
    };
    Some(selected)
}

/// The words of a statement that keeps its own raw argv, minus the
/// brace-quoted ones: Tcl substitutes nothing inside braces, so a `[…]`
/// there is text the statement does not run.
fn unbraced_words<'a>(
    args: &'a [String],
    tokens: Option<&CommandTokens>,
) -> impl Iterator<Item = &'a str> {
    let braced: Vec<bool> = (0..args.len())
        .map(|idx| tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(idx)))
        .collect();
    args.iter()
        .enumerate()
        .filter(move |(idx, _)| !braced[*idx])
        .map(|(_, arg)| arg.as_str())
}

/// The argument words of an invocation that are this statement's own
/// substitution surface.
///
/// A *structural* body — one the registry says runs in a separate definition
/// or dispatch context rather than in this frame — is excluded. The module
/// doc already says bodies are [`nested_bodies`]' job; the invocation arm of
/// [`evaluated_command_substitution_surfaces`] simply never applied it.
///
/// `proc q {} {…}` is the case that matters. Its body is lowered into a
/// procedure of its own, so reading it here attributed the body's
/// substitutions to the enclosing `proc` statement: once those carried reads
/// as well as writes, `puts [incr m]` inside the body reported `W210 'm' is
/// read before it is set` against the caller's frame, on a program tclsh
/// 9.0.4 runs cleanly (it prints `2`).
///
/// Braced words are not argument substitution surfaces, even when they
/// contain a caller-frame body. Entered body and expression reads retain their
/// separate evaluation owners; nested scripts are walked at that execution
/// boundary rather than attributed to this command's argv.
fn invoked_word_surfaces<'a>(
    lookup: &str,
    args: &'a [String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> impl Iterator<Item = &'a str> {
    let structural = crate::ssa::structural_body_indices(lookup, args, tokens, registry);
    args.iter()
        .enumerate()
        .filter(move |(idx, _)| {
            tokens.is_none_or(CommandTokens::evaluates_words)
                && !structural.contains(idx)
                && !tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(*idx))
        })
        .map(|(_, arg)| arg.as_str())
}

/// Recover commands from an already-selected collection of evaluated Tcl
/// source surfaces, resolving each recovered head through `heads` when the
/// caller can supply one.
#[must_use]
pub(crate) fn command_substitutions_in_surfaces_with_replay(
    surfaces: &[&str],
    initially_opaque: bool,
    conditional: bool,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
) -> EvaluatedCommandSubstitutions {
    let config = registry
        .profile()
        .map_or_else(LexerConfig::default, |profile| {
            LexerConfig::from_grammar(profile.grammar)
        });
    let mut out = EvaluatedCommandSubstitutions {
        opaque: initially_opaque,
        ..EvaluatedCommandSubstitutions::default()
    };
    let context = SubstitutionWalkContext {
        config,
        registry,
        metadata: None,
        heads,
        observe,
        conditional,
    };
    for text in surfaces {
        walk_text(text, &context, 0, Reach::Statement, &mut out);
    }
    out
}

fn recover_substitution_surfaces(
    surfaces: &[&str],
    initially_opaque: bool,
    context: SubstitutionWalkContext<'_>,
) -> EvaluatedCommandSubstitutions {
    let mut out = EvaluatedCommandSubstitutions {
        opaque: initially_opaque,
        ..EvaluatedCommandSubstitutions::default()
    };
    for text in surfaces {
        walk_text(text, &context, 0, Reach::Statement, &mut out);
    }
    out
}

/// Recover commands from command substitutions nested in one expression.
#[must_use]
pub(crate) fn expression_command_substitutions(
    expr: &ExprNode,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
) -> EvaluatedCommandSubstitutions {
    expression_command_substitutions_with_replay(expr, registry, heads, None)
}

#[must_use]
pub(crate) fn expression_command_substitutions_with_replay(
    expr: &ExprNode,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
) -> EvaluatedCommandSubstitutions {
    let mut texts = Vec::new();
    let mut opaque = false;
    let mut conditional = false;
    collect_expr_command_surface_refs(expr, &mut texts, &mut opaque, &mut conditional, 0);
    command_substitutions_in_surfaces_with_replay(
        &texts,
        opaque,
        conditional,
        registry,
        heads,
        observe,
    )
}

/// Conditional expression command inventory under the supplied metadata and
/// exact syntax policy. Missing/foreign input is opaque; no native replay or
/// callback/frame entry is established by this readonly inventory.
#[must_use]
pub(crate) fn expression_command_substitutions_with_replay_and_metadata_context(
    expr: &ExprNode,
    registry: &CommandRegistry,
    heads: Option<EmbeddedHeadResolver<'_>>,
    observe: Option<EmbeddedCommandObserver<'_>>,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: LexerConfig,
) -> EvaluatedCommandSubstitutions {
    let Some(metadata) = metadata.filter(|metadata| metadata.matches_registry(registry)) else {
        return EvaluatedCommandSubstitutions {
            opaque: true,
            ..Default::default()
        };
    };
    if metadata
        .source_analysis_input()
        .is_some_and(|input| input.lexer_config().normalized() != config.normalized())
    {
        return EvaluatedCommandSubstitutions {
            opaque: true,
            ..Default::default()
        };
    }
    let mut texts = Vec::new();
    let mut opaque = false;
    let mut conditional = false;
    collect_expr_command_surface_refs(expr, &mut texts, &mut opaque, &mut conditional, 0);
    recover_substitution_surfaces(
        &texts,
        opaque,
        SubstitutionWalkContext {
            config,
            registry,
            metadata: Some(metadata),
            heads,
            observe,
            conditional,
        },
    )
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

/// Map one segmented command onto its per-word [`CommandWord`] facts.
fn command_written_spans(
    sm: &SourceMap<'_>,
    config: LexerConfig,
    command: &SegmentedCommand,
) -> Option<Vec<tcl_lexer::Span>> {
    if !command.word_views_aligned() {
        return None;
    }
    let source = std::str::from_utf8(sm.source_bytes()).ok()?;
    for token in &command.all_tokens {
        let raw = source.get(token.span.as_range())?;
        raw.get(usize::from(token.content_offset)..)?;
    }
    let groups = tcl_lexer::group_commands_bytes(&command.all_tokens, sm.source_bytes(), config);
    let groups = groups
        .iter()
        .flat_map(|command| &command.words)
        .collect::<Vec<_>>();
    if groups.len() != command.argv.len() {
        return None;
    }
    groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let fragments = command.word_fragments.get(index)?;
            let first = fragments.first()?.token;
            let expected = tcl_lexer::Token {
                span: group.span,
                ..first
            };
            let actual = command.all_tokens.get(group.tokens.clone())?;
            if command.argv[index] != expected
                || actual.len() != fragments.len()
                || !actual.iter().zip(fragments).all(|(token, fragment)| {
                    *token == fragment.token
                        && fragment.text == crate::segmenter::word_piece(sm, *token)
                })
                || command.texts[index]
                    != fragments
                        .iter()
                        .map(|fragment| fragment.text.as_str())
                        .collect::<String>()
                || command.single_token_word[index] != group.is_single_token()
                || command
                    .expand_word
                    .as_ref()
                    .is_some_and(|expand| expand[index])
                    != group.expand
            {
                return None;
            }
            let span = tcl_lexer::NativeWord::written_span_from_group(
                sm.source_bytes(),
                config,
                &command.all_tokens,
                group,
            )
            .ok()?;
            source.get(span.as_range())?;
            Some(span)
        })
        .collect()
}

fn command_words(
    sm: &SourceMap<'_>,
    config: LexerConfig,
    command: &SegmentedCommand,
) -> Vec<CommandWord> {
    let written = command_written_spans(sm, config, command);
    let tokens = written
        .as_ref()
        .map(|_| CommandTokens::from_segmented(sm, config, command));
    command
        .argv
        .iter()
        .enumerate()
        .map(|(idx, token)| CommandWord {
            text: command.texts.get(idx).cloned().unwrap_or_default(),
            raw: written
                .as_ref()
                .and_then(|spans| spans.get(idx))
                .map_or_else(String::new, |span| sm.text(*span).to_owned()),
            geometry: if written.is_some() {
                WrittenWordGeometry::Present
            } else {
                WrittenWordGeometry::Missing
            },
            substituted: written.is_none()
                || tokens
                    .as_ref()
                    .and_then(|tokens| tokens.words().get(idx))
                    .is_some_and(word_substitutes),
            expanded: command
                .expand_word
                .as_ref()
                .and_then(|markers| markers.get(idx))
                .copied()
                .unwrap_or(false),
            // `{…}` and nothing welded to it — the same question
            // `CommandTokens::arg_is_braced_literal` answers for an IR
            // statement's arguments, asked here in whole-argv indexing.
            braced_literal: written.is_some()
                && token.kind == TokenType::Str
                && command.single_token_word.get(idx).copied().unwrap_or(false),
        })
        .collect()
}

/// Whether a word's value comes from run-time data rather than its own source
/// text — the [`WordExpr`] forms that substitute.
///
/// `Template` is the compound case (`a$b`, `{a}[c]`): one substituting part is
/// enough, exactly as the whole word's value stops being its written text.
/// `Expand` wraps the word `{*}` expands and answers for that word.
fn word_substitutes(word: &WordExpr) -> bool {
    match word {
        WordExpr::Variable { .. } | WordExpr::CommandSubstitution { .. } => true,
        WordExpr::Template { parts, .. } => parts.iter().any(|part| {
            matches!(
                part,
                WordPart::Variable { .. } | WordPart::CommandSubstitution { .. }
            )
        }),
        WordExpr::Expand { word, .. } => word_substitutes(word),
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } | WordExpr::Opaque { .. } => {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::interprocedural::test_body_identity::fixture_id;

    use super::*;
    use crate::ir::{ForeachIterator, IfClause, Script};
    use tcl_lexer::Span;

    #[test]
    fn residual_body_wrapper_does_not_repeat_argument_substitutions() {
        let registry = CommandRegistry::build_default();
        let mut statement = Statement::Call {
            span: Span::new(0, 16),
            command: "puts".to_owned(),
            canonical_command: None,
            args: vec!["[set x 1]".to_owned()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(CommandTokens::marker(
                crate::ir::SyntheticMarker::EvaluatedArguments,
            )),
            foreach_groups: None,
        };
        assert_eq!(
            evaluated_command_substitution_surfaces(&statement, &registry).texts,
            ["[set x 1]"]
        );
        statement.tokens_mut().unwrap().synthetic =
            Some(crate::ir::SyntheticMarker::EvaluatedWrapper);
        assert_eq!(
            evaluated_command_substitution_surfaces(&statement, &registry).texts,
            [] as [&str; 0]
        );
    }

    #[test]
    fn defs_from_empty_script() {
        let script = Script::new();
        assert_eq!(defs_from_ir_script(&script), [] as [std::string::String; 0]);
    }

    /// `expr_has_command` and
    /// `collect_expr_commands` each recurse once per `ExprNode` level, so both
    /// need a depth cap. A tree built directly is unbounded (the
    /// Pratt parser caps its own output at 256) and empirically overflowed
    /// the native stack (SIGABRT) in the low thousands of levels on a 2 MiB
    /// thread. 3000 is past that crash range and past `MAX_EXPR_NODE_DEPTH`
    /// (256); the assertion is that each returns at all.
    #[test]
    fn deeply_nested_expr_command_walks_survive() {
        use crate::expr_ast::UnaryOp;
        let mut node = ExprNode::Command {
            text: "[x]".into(),
            start: 0,
            end: 3,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let _ = expr_has_command(&node);
        let mut cmds = Vec::new();
        collect_expr_commands(&node, &mut cmds);
    }

    #[test]
    fn defs_from_assign() {
        let script = Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 7),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["x"]);
    }

    #[test]
    fn defs_from_call_defs() {
        let script = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 10),
            command: "set".into(),
            canonical_command: None,
            args: vec!["x".into(), "1".into()],
            defs: vec!["x".into()],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["x"]);
    }

    #[test]
    fn defs_from_nested_if() {
        let script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 30),
            clauses: vec![IfClause {
                condition: ExprNode::Literal {
                    text: "1".into(),
                    start: 0,
                    end: 1,
                },
                condition_span: Span::new(3, 4),
                body: Script::from_statements(vec![Statement::AssignConst {
                    span: Span::new(6, 13),
                    name: "y".into(),
                    name_braced: false,
                    value: "2".into(),
                    value_span: None,
                }]),
                body_span: Span::new(5, 14),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["y"]);
    }

    /// `collect_defs_from_script`
    /// recurses once per nested `If`/`For`/`While`/`Foreach`/`Catch`/`Try`/
    /// `Switch` body, so it needs a depth cap of its own.
    /// Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering
    /// pass today, so this is defence-in-depth / consistency with every
    /// other full-tree walker in this crate, not a currently-reproducible
    /// crash. 2000 levels is comfortably past this new cap; the assertion
    /// is that `defs_from_ir_script` returns at all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_defs_from_ir_script() {
        const DEPTH: usize = 2000;
        let leaf = Statement::AssignConst {
            span: Span::new(0, 0),
            name: "leaf".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        };
        let mut script = Script::from_statements(vec![leaf]);
        for _ in 0..DEPTH {
            script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Literal {
                        text: "1".into(),
                        start: 0,
                        end: 1,
                    },
                    condition_span: Span::new(0, 0),
                    body: script,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }

        // 2000 levels of nesting exceeds the new 256-deep cap, so the
        // innermost `leaf` def is never reached — the assertion is that
        // this returns at all (no stack overflow), not what it returns.
        let _ = defs_from_ir_script(&script);
    }

    #[test]
    fn defs_from_foreach() {
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["k".into(), "v".into()],
                list_arg: "$d".into(),
                list_braced: false,
            }],
            body: Script::new(),
            body_span: Span::new(20, 25),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        }]);
        assert_eq!(defs_from_ir_script(&script), vec!["k", "v"]);
    }

    #[test]
    fn ir_destination_inventory_keeps_literal_data_and_refuses_computed_names() {
        let assign = |name: &str, braced| Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: braced,
            value: "VALUE".into(),
            value_span: None,
        };
        let script = Script::from_statements(vec![
            assign("$literal", true),
            assign("${literal}", true),
            assign("", true),
            assign("arr($index)", true),
            assign("$computed", false),
            Statement::Foreach {
                span: Span::new(0, 0),
                iterators: vec![ForeachIterator {
                    vars: vec![
                        "$iterator".into(),
                        "${iterator}".into(),
                        String::new(),
                        "array(key)".into(),
                    ],
                    list_arg: "ONE TWO THREE FOUR".into(),
                    list_braced: true,
                }],
                body: Script::from_statements(vec![assign("nested", false)]),
                body_span: Span::new(0, 0),
                is_lmap: false,
                raw_args: Vec::new(),
                is_dict_iteration: false,
                is_array_iteration: false,
                raw_tokens: None,
            },
        ]);
        assert_eq!(
            defs_from_ir_script(&script),
            [
                "$literal",
                "${literal}",
                "",
                "arr($index)",
                "$iterator",
                "${iterator}",
                "",
                "array(key)",
                "nested"
            ]
        );
        assert!(structural_variable_write_projection(&script.statements[4]).opaque_variable_frame);
    }

    #[test]
    fn defs_from_catch() {
        let script = Script::from_statements(vec![Statement::Catch {
            span: Span::new(0, 30),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "inner".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            }]),
            body_span: Span::new(6, 15),
            result_var: Some("result".into()),
            options_var: None,
            raw_args: vec![],
            tokens: None,
        }]);
        let d = defs_from_ir_script(&script);
        assert!(d.contains(&"inner".to_string()));
        assert!(d.contains(&"result".to_string()));
    }

    #[test]
    fn tokenise_simple_command() {
        let words = tokenise_to_words("set x 1", &CommandRegistry::build_default());
        let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, vec!["set", "x", "1"]);
        assert!(words.iter().all(|w| !w.substituted));
    }

    #[test]
    fn tokenise_keeps_both_spellings_and_the_substitution_flag() {
        let words = tokenise_to_words("set $x {a $b}", &CommandRegistry::build_default());
        let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
        let raws: Vec<&str> = words.iter().map(|w| w.raw.as_str()).collect();
        // The reconstructed text canonicalises the substitution to its braced
        // spelling (`$x` → `${x}`, the same form an IR statement's arguments
        // carry) and de-braces a braced word; raw keeps what was written. A
        // delimited word retains its complete written wrapper independently
        // of the variable/component span convention.
        assert_eq!(texts, vec!["set", "${x}", "a $b"]);
        assert_eq!(raws, vec!["set", "$x", "{a $b}"]);
        assert_eq!(
            words.iter().map(|w| w.substituted).collect::<Vec<_>>(),
            vec![false, true, false]
        );
        assert!(words[2].braced_literal, "`{{a $b}}` is a literal word");
    }

    #[test]
    fn tokenise_preserves_release_aware_argument_expansion() {
        let config_for = |dialect| {
            let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
            LexerConfig::from_grammar(registry.profile().unwrap().grammar)
        };

        let tcl90 = tokenise_command_words("cmd {*}$x {*}{a b}", config_for("tcl9.0"));
        let words = &tcl90[0];
        assert_eq!(
            words
                .iter()
                .map(CommandWord::invocation_word)
                .map(InvocationWord::kind)
                .collect::<Vec<_>>(),
            vec![
                tcl_registry::InvocationWordKind::Literal,
                tcl_registry::InvocationWordKind::Expanded,
                tcl_registry::InvocationWordKind::Expanded,
            ]
        );

        let tcl84 = tokenise_command_words("cmd {*}$x {*}{a b}", config_for("tcl8.4"));
        assert!(
            tcl84[0].iter().all(|word| !word.expanded),
            "Tcl 8.4 predates argument expansion"
        );
    }

    #[test]
    fn source_substitution_roles_and_footprints_use_retained_availability() {
        // naming.compiler.original-analysis-metadata-context

        // Conditional source roles/footprints do not prove dispatch or a Native cell.
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "gated-expr",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("expr").unwrap().clone()
        });
        registry.insert(tcl_registry::CommandSpec {
            name: "gated-write",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("set").unwrap().clone()
        });
        let registry = std::sync::Arc::new(registry);
        let source = "set result [gated-expr {[gated-write {$scalar(open} VALUE]}]";
        let module = crate::lowering::lower_to_ir(source, &registry);
        let statement = &module.top_level.statements[0];
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(std::sync::Arc::clone(&registry));
            let inventory = evaluated_command_substitutions_with_metadata_context(
                statement,
                &registry,
                Some((&context).into()),
                LexerConfig::default(),
            );
            let nested: Vec<_> = inventory
                .all_commands()
                .filter(|words| words.first().and_then(CommandWord::literal) == Some("gated-write"))
                .collect();
            assert_eq!(nested.len(), usize::from(available), "{environment}");
            if available {
                let footprint = variable_write_effects_from_commands_with_metadata_context(
                    nested,
                    &registry,
                    Some((&context).into()),
                );
                assert_eq!(footprint.names, ["$scalar(open"]);
                assert!(!footprint.opaque);
            } else {
                assert!(inventory.opaque);
                let words = tokenise_command_words(
                    "gated-write {$scalar(open} VALUE",
                    LexerConfig::default(),
                );
                let footprint = variable_write_effects_from_commands_with_metadata_context(
                    &words,
                    &registry,
                    Some((&context).into()),
                );
                assert!(footprint.names.is_empty());
                assert!(footprint.opaque);
            }
        }
        assert!(
            evaluated_command_substitutions_with_metadata_context(
                statement,
                &registry,
                None,
                LexerConfig::default()
            )
            .opaque
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
        assert!(
            evaluated_command_substitutions_with_metadata_context(
                statement,
                &registry,
                Some(foreign.into()),
                LexerConfig::default()
            )
            .opaque
        );
    }

    #[test]
    fn source_substitution_descent_retains_config_and_target_refusal() {
        // naming.compiler.original-analysis-metadata-context

        let registry = std::sync::Arc::new(CommandRegistry::build_default());
        let context = tcl_registry::model::ingress::static_context_for("tcl9.0")
            .with_command_store(std::sync::Arc::clone(&registry));
        let module = crate::lowering::lower_to_ir("set result [list {*}{a b}]", &registry);
        let statement = &module.top_level.statements[0];
        for (environment, expanded) in [("tcl8.4", false), ("tcl9.0", true)] {
            let config = LexerConfig::from_grammar(
                tcl_registry::model::ingress::static_context_for(environment)
                    .commands()
                    .profile()
                    .unwrap()
                    .grammar,
            );
            let inventory = evaluated_command_substitutions_with_metadata_context(
                statement,
                &registry,
                Some((&context).into()),
                config,
            );
            assert_eq!(
                inventory.all_commands().flatten().any(|word| word.expanded),
                expanded
            );
        }
        let module = crate::lowering::lower_to_ir("set result [expr {[incr hidden]}]", &registry);
        let refuse = |_head: &str| None;
        let inventory = evaluated_command_substitutions_with_heads_and_metadata_context(
            &module.top_level.statements[0],
            &registry,
            Some(&refuse),
            Some((&context).into()),
            LexerConfig::default(),
        );
        assert!(inventory.opaque);
        assert!(
            !inventory
                .all_commands()
                .any(|words| words.first().and_then(CommandWord::literal) == Some("incr"))
        );
    }

    #[test]
    fn variable_write_projection_marks_expanded_set_opaque() {
        let registry = CommandRegistry::build_default();
        let commands = tokenise_command_words("set {*}{x 99}", LexerConfig::default());
        let effects = variable_write_effects_from_commands(&commands, &registry);
        assert_eq!(effects.names, [] as [std::string::String; 0]);
        assert!(effects.opaque);
    }

    #[test]
    fn variable_write_projection_keeps_all_trailing_regexp_targets() {
        let registry = CommandRegistry::build_default();
        let commands = tokenise_command_words(
            "regexp {(a)(b)} ab whole first second",
            LexerConfig::default(),
        );
        let effects = variable_write_effects_from_commands(&commands, &registry);
        assert_eq!(effects.names, ["whole", "first", "second"]);
        assert!(!effects.opaque);
    }

    #[test]
    fn collect_expr_command_nodes() {
        let expr = ExprNode::Command {
            text: "[set x 1]".into(),
            start: 0,
            end: 9,
        };
        let mut cmds = Vec::new();
        collect_expr_commands(&expr, &mut cmds);
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0], "[set x 1]");
    }

    #[test]
    fn compiled_word_surfaces_only_execute_when_unbraced() {
        let registry = CommandRegistry::build_default();
        let commands = |braced| {
            expression_command_substitutions(
                &ExprNode::CompiledWord {
                    text: "prefix[set x 1]".into(),
                    braced,
                },
                &registry,
                None,
            )
            .commands
        };

        assert_eq!(commands(false)[0][0].text, "set");
        assert_eq!(commands(true).len(), 0);
    }

    /// The commands of `surface` a statement's substitution recovers, under
    /// the 8.6 registry.
    fn recovered(surface: &str) -> EvaluatedCommandSubstitutions {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        command_substitutions_in_surfaces_with_replay(
            &[surface],
            false,
            false,
            registry,
            None,
            None,
        )
    }

    /// The head spelling of each command, sorted.
    fn heads<'a>(commands: impl IntoIterator<Item = &'a Vec<CommandWord>>) -> Vec<&'a str> {
        let mut heads: Vec<&str> = commands
            .into_iter()
            .filter_map(|words| words.first())
            .map(|word| word.text.as_str())
            .collect();
        heads.sort_unstable();
        heads
    }

    /// The script a `catch` protects, and the body of a `try`, run once in the
    /// frame the substitution is written in, so their commands are the
    /// statement's: `[catch {incr x}]` writes `x`, and so does `[catch "incr
    /// x"]`, whose word substitutes nothing. Nothing else is descended by a
    /// command's name — a handler, a `finally`, an `if` arm, a loop body, an
    /// `eval` — and a script that is run-time data is not known text: the walk
    /// is incomplete.
    #[test]
    fn a_protected_script_is_descended_and_no_other_body_is() {
        let one = recovered("[catch {incr x}]");
        assert_eq!(heads(&one.commands), ["catch"]);
        assert_eq!(heads(&one.in_frame_commands), ["incr"]);
        assert_eq!(one.in_frame_texts, ["incr x"]);

        let two = recovered("[catch {catch {incr x}}]");
        assert_eq!(heads(&two.in_frame_commands), ["catch", "incr"]);
        assert_eq!(two.in_frame_texts, ["catch {incr x}", "incr x"]);

        let in_expr = recovered("[catch {expr {[incr x] + 1}}]");
        assert_eq!(heads(&in_expr.in_frame_commands), ["expr", "incr"]);

        let in_substitution = recovered("[catch {set y [incr x]}]");
        assert_eq!(heads(&in_substitution.in_frame_commands), ["incr", "set"]);

        let body_only = recovered("[try {incr x} on error {} {incr y} finally {incr z}]");
        assert_eq!(heads(&body_only.in_frame_commands), ["incr"]);
        assert_eq!(body_only.in_frame_texts, ["incr x"]);

        let quoted = recovered("[catch \"incr x; set y 2\"]");
        assert_eq!(heads(&quoted.in_frame_commands), ["incr", "set"]);
        assert_eq!(quoted.in_frame_texts, ["incr x; set y 2"]);
        assert!(!quoted.opaque);
        let nested_quoted = recovered("[catch \"catch {incr x}\"]");
        assert_eq!(heads(&nested_quoted.in_frame_commands), ["catch", "incr"]);

        for other in [
            "[if {1} {incr x}]",
            "[eval {incr x}]",
            "[foreach v {1} {incr x}]",
            "[while {1} {incr x}]",
        ] {
            let embedded = recovered(other);
            assert!(
                !heads(&embedded.in_frame_commands).contains(&"incr")
                    && embedded
                        .in_frame_texts
                        .iter()
                        .all(|text| !text.contains("incr"))
                    && !embedded.opaque,
                "{other}"
            );
        }
        for computed in [
            "[catch $script]",
            "[catch [list incr x]]",
            "[catch \"incr $name\"]",
            "[catch \"incr x\\n\"]",
            "[catch {*}$script]",
            "[try $script on error {} {}]",
        ] {
            let embedded = recovered(computed);
            assert!(
                embedded.opaque
                    && !heads(&embedded.in_frame_commands).contains(&"incr")
                    && embedded
                        .in_frame_texts
                        .iter()
                        .all(|text| !text.contains("incr")),
                "{computed}"
            );
        }
        assert!(!recovered("[catch {incr x}]").opaque);
    }

    #[test]
    fn complete_substitution_inventory_preserves_expression_sibling_order() {
        let registry = CommandRegistry::build_default();
        let embedded = command_substitutions_in_surfaces_with_replay(
            &["[expr {[rename aaa {}; rename eval aaa; set _ 0]}] [aaa {set x 6}]"],
            false,
            false,
            &registry,
            None,
            None,
        );
        let heads = |commands: Vec<&Vec<CommandWord>>| {
            commands
                .into_iter()
                .map(|words| words[0].text.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            heads(embedded.all_commands().collect()),
            ["rename", "rename", "set", "expr", "aaa"]
        );
        assert_eq!(heads(embedded.commands.iter().collect()), ["expr", "aaa"]);
        assert_eq!(
            heads(embedded.in_frame_commands.iter().collect()),
            ["rename", "rename", "set"]
        );
        assert!(!embedded.opaque);
    }

    #[test]
    fn standalone_expression_write_names_preserve_literal_operand_data() {
        let registry = CommandRegistry::build_default();
        for (source, expected) in [
            ("[set {$literal} 1]", "$literal"),
            ("[set {} 1]", ""),
            ("[catch {set {$literal} 1}]", "$literal"),
        ] {
            let expression = ExprNode::Command {
                text: source.to_owned(),
                start: 0,
                end: u32::try_from(source.len()).unwrap(),
            };
            let names = defs_from_expr(&expression, &registry);
            assert!(
                names.iter().any(|name| name == expected),
                "{source}: {names:?}"
            );
            assert!(!names.iter().any(|name| name == "literal"));
        }
        let expression = ExprNode::Command {
            text: "[set $selector 1]".to_owned(),
            start: 0,
            end: 17,
        };
        assert!(
            !defs_from_expr(&expression, &registry)
                .iter()
                .any(|name| name == "selector")
        );
    }

    #[test]
    fn selected_extra_store_and_registered_trace_effects_remain_opaque() {
        use tcl_registry::world_effect::{
            EffectAccess, EffectAccessMode, EffectFootprint, InterpreterScope, NamespaceScope,
            SubjectScope, WorldEffectDescriptor, WorldStateDomain,
        };
        fn extra_store(_: tcl_registry::InvocationArguments<'_>) -> EffectFootprint {
            let mut effects = EffectFootprint::default();
            effects.add_access(EffectAccess::new(
                WorldStateDomain::VariableStore,
                EffectAccessMode::Write,
                InterpreterScope::Current,
                NamespaceScope::named("::other"),
                SubjectScope::named("foreign"),
            ));
            effects
        }
        fn coalesced_store(_: tcl_registry::InvocationArguments<'_>) -> EffectFootprint {
            let mut effects = EffectFootprint::default();
            effects.add_access(EffectAccess::variable_store_wildcard(
                EffectAccessMode::Write,
            ));
            effects
        }
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let original = tcl_registry::model::ingress::context_for_profile(profile);
        for resolver in [
            Some(extra_store as tcl_registry::world_effect::WorldEffectResolver),
            Some(coalesced_store),
            None,
        ] {
            let mut registry = original.commands().project_for_profile(profile);
            let mut setter = registry.get("set").unwrap().clone();
            setter.world_effects = Some(WorldEffectDescriptor {
                resolver,
                ..WorldEffectDescriptor::VARIABLE_WRITE
            });
            if resolver.is_none() {
                setter.arg_roles = &[];
                setter.arg_role_resolver = None;
                setter.arg_role_count_resolver = None;
                setter.arg_role_layout_resolver = None;
                setter.assigns_variable_at = None;
                setter.forms = &[];
                setter.arg_role_resolver_roles = &[];
            }
            registry.insert(setter);
            let context =
                std::sync::Arc::new(original.with_command_store(std::sync::Arc::new(registry)));
            let registry = context.commands();
            let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                context.clone(),
                config,
            );
            let module = crate::lowering::lower_to_ir_with(
                crate::lowering::Lowerer::with_config(registry, config)
                    .for_bytecode_backend()
                    .with_resolved_analysis_input(input),
                "set counter 0",
            );
            let statement = &module.top_level.statements[0];
            let tokens = module
                .top_level
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let writes = original_statement_possible_name_writes(statement, tokens, registry);
            assert!(
                writes.opaque,
                "additional store effect cannot borrow baseline role coverage"
            );
        }
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, original.clone(), config);
        let module = crate::lowering::lower_to_ir_with(
            crate::lowering::Lowerer::with_config(original.commands(), config)
                .for_bytecode_backend()
                .with_resolved_analysis_input(input),
            "trace add variable counter write cb; set counter 0",
        );
        let statement = module.top_level.statements.last().unwrap();
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let variables = &tokens.source_binding.as_ref().unwrap().variable_context;
        assert!(
            variables.dynamic_traces
                || !variables.traced.is_empty()
                || !variables.trace_registrations.is_empty()
                || !variables.trace_registration_receivers.is_empty()
                || !variables.possible_trace_registrations.is_empty()
        );
        assert!(
            original_statement_possible_name_writes(statement, tokens, original.commands()).opaque
        );
    }

    #[test]
    fn native_original_scalar_effects_keep_complete_names_and_refuse_changed_words() {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let registry = context.commands();
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(native)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .for_bytecode_backend()
            .with_resolved_analysis_input(input);
        lowerer.set_source_analysis_options(entry.options());
        let module = crate::lowering::lower_to_ir_with(lowerer, "set counter 0");
        assert!(module.source_entry.native_entry.is_some());
        let statement = &module.top_level.statements[0];
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let complete = original_invocation_possible_name_writes(tokens, registry)
            .expect("actual complete Native metadata/candidate/effect owner");
        assert!(!complete.opaque);
        assert!(complete.names.iter().any(|name| name == "counter"));
        let mut writes = std::collections::BTreeSet::new();
        assert!(!crate::ssa::nested_writes(
            &module.top_level,
            registry,
            &mut writes
        ));
        assert!(writes.contains("counter"));
        let mut changed = tokens.clone();
        changed.argv_texts[1].push_str(" changed");
        assert!(original_invocation_possible_name_writes(&changed, registry).is_none());
    }

    #[test]
    fn original_write_name_candidates_retain_unknown_effects_and_source_refusals() {
        let registry = CommandRegistry::build_default();
        let module =
            crate::lowering::lower_to_ir("proc p {} {set observed [set {$literal} 1]}", &registry);
        let (_, procedure) = module.original_source_procedure_bodies().next().unwrap();
        let statement = &procedure.body.statements[0];
        let tokens = procedure
            .body
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let names = original_statement_possible_name_writes(statement, tokens, &registry);
        assert!(names.opaque);
        assert!(names.names.iter().any(|name| name == "observed"));
        assert!(names.names.iter().any(|name| name == "$literal"));
        assert!(!names.names.iter().any(|name| name == "literal"));
        let mut changed = tokens.clone();
        changed.argv_texts[1].push_str(" changed");
        let refused = original_statement_possible_name_writes(statement, &changed, &registry);
        assert!(refused.opaque && refused.names.is_empty());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let refused = original_statement_possible_name_writes(statement, tokens, foreign);
        assert!(refused.opaque && refused.names.is_empty());
    }

    #[test]
    fn standalone_typed_write_coverage_preserves_primitive_data_and_nested_control() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let assign = |name: &str| Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: true,
            value: "VALUE".into(),
            value_span: None,
        };
        let script = Script::from_statements(vec![
            assign("$literal"),
            assign(""),
            Statement::If {
                span: Span::new(0, 0),
                clauses: vec![crate::ir::IfClause {
                    condition: crate::expr_parser::parse_expr("$counter < 3", None),
                    condition_span: Span::new(0, 0),
                    condition_base: None,
                    body: Script::from_statements(vec![assign("nested")]),
                    body_span: Span::new(0, 0),
                }],
                else_body: None,
                else_span: None,
            },
        ]);
        let mut names = BTreeSet::new();
        assert!(!crate::ssa::nested_writes(&script, registry, &mut names));
        assert_eq!(
            names,
            BTreeSet::from([String::new(), "$literal".into(), "nested".into()])
        );
    }

    #[test]
    fn standalone_typed_write_coverage_keeps_calls_and_present_source_terminal() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let assign = Statement::AssignConst {
            span: Span::new(0, 0),
            name: "counter".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        };
        let standalone = Script::from_statements(vec![assign.clone()]);
        assert!(standalone_typed_statement_writes_are_covered(
            &standalone,
            &standalone.statements[0],
            registry
        ));
        for condition in ["[set unexpected 1]", "sin(0)", "$arr([set key 1])"] {
            let script = Script::from_statements(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![crate::ir::IfClause {
                    condition: crate::expr_parser::parse_expr(condition, None),
                    condition_span: Span::new(0, 0),
                    condition_base: None,
                    body: Script::new(),
                    body_span: Span::new(0, 0),
                }],
                else_body: None,
                else_span: None,
            }]);
            assert!(
                crate::ssa::nested_writes(&script, registry, &mut BTreeSet::new()),
                "{condition}"
            );
        }
        let call = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 0),
            command: "set".into(),
            canonical_command: None,
            args: vec!["counter".into(), "0".into()],
            defs: vec!["counter".into()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }]);
        assert!(crate::ssa::nested_writes(
            &call,
            registry,
            &mut BTreeSet::new()
        ));
        let module = crate::lowering::lower_to_ir("set counter 0", registry);
        let mut source = module.top_level;
        assert!(source.executed_source.is_some());
        source.statements = vec![assign];
        source.command_binding_sites = crate::ir::CommandBindingSites::default();
        assert!(!standalone_typed_statement_writes_are_covered(
            &source,
            &source.statements[0],
            registry
        ));
        assert!(crate::ssa::nested_writes(
            &source,
            registry,
            &mut BTreeSet::new()
        ));
    }

    #[test]
    fn defs_from_expr_set_command() {
        let reg = CommandRegistry::build_default();
        let expr = ExprNode::Command {
            text: "[set x 1]".into(),
            start: 0,
            end: 9,
        };
        let d = defs_from_expr(&expr, &reg);
        assert!(
            d.contains(&"x".to_string()),
            "should find set's VarWrite; got {d:?}"
        );
    }

    /// Out-vars a condition-embedded `[cmd …]` contributes, as
    /// [`condition_command_out_vars`] sees them.
    fn cond_out_vars(cmd_text: &str) -> Vec<String> {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let expr = ExprNode::Command {
            text: format!("[{cmd_text}]"),
            start: 0,
            end: 0,
        };
        condition_command_out_vars(&expr, registry)
    }

    #[test]
    fn expanded_words_do_not_manufacture_condition_out_vars() {
        // Tcl 9.0.4 expands the list into extra `catch` arguments, reports
        // wrong-arity, and creates neither `x` nor `result`. Source positions
        // before expansion therefore cannot be used as registry role indices.
        assert_eq!(
            cond_out_vars("catch {*}{set x 1} result"),
            [] as [std::string::String; 0]
        );
    }

    /// The registry's `ArgRole::VarWrite` query must answer for every command
    /// a `catch` / `scan` / `gets` / `regexp` name list would cover.
    #[test]
    fn registry_query_covers_the_deleted_hardcoded_four() {
        for (cmd_text, expected) in [
            ("catch {error x} msg opts", vec!["msg", "opts"]),
            ("scan $t {%d %d} a b", vec!["a", "b"]),
            ("gets $fp line", vec!["line"]),
            ("regexp {(a)(b)} $s m p q", vec!["m", "p", "q"]),
            // The switch-skipping the hand-rolled `regexp` loop did by hand
            // is the registry resolver's job now.
            ("regexp -nocase -- $re $s m", vec!["m"]),
        ] {
            let got = cond_out_vars(cmd_text);
            for name in &expected {
                assert!(
                    got.iter().any(|v| v == name),
                    "`{cmd_text}` must still yield `{name}`; got {got:?}"
                );
            }
        }
    }

    /// …and it exceeds that list: commands the hardcoded four never covered,
    /// each verified against tclsh 9.0.4 / 8.6.14 first.
    #[test]
    fn registry_query_exceeds_the_deleted_hardcoded_four() {
        for (cmd_text, expected) in [
            // `proc bar {lst} {if {[set idx [lsearch $lst foo]] > -1} {puts $idx}}`
            // `bar {a foo b}` → 1
            ("set idx [lsearch $lst foo]", vec!["idx"]),
            // `proc loopy {} {set i 0; while {[incr i] < 3} {puts $i}}` → 1 2
            ("incr i", vec!["i"]),
            // `proc lass {lst} {lassign $lst a b; puts "$a-$b"}`
            // `lass {1 2}` → 1-2
            ("lassign $lst a b", vec!["a", "b"]),
            // `proc binscan {} {binary scan "AB" H2H2 hi lo; puts "$hi $lo"}`
            // → 41 42
            ("binary scan $d H2H2 hi lo", vec!["hi", "lo"]),
            // `regsub` writes its result variable.
            ("regsub $re $s $r out", vec!["out"]),
        ] {
            let got = cond_out_vars(cmd_text);
            for name in &expected {
                assert!(
                    got.iter().any(|v| v == name),
                    "`{cmd_text}` must yield `{name}`; got {got:?}"
                );
            }
        }
    }

    /// A `Plain` body (`catch`'s script) runs in the caller's own frame, so
    /// its writes are harvested; a destroyer is not a definition.
    #[test]
    fn condition_out_vars_descends_plain_bodies_but_skips_destroyers() {
        let got = cond_out_vars("catch {set inner 1} msg");
        assert!(got.iter().any(|v| v == "inner"), "got {got:?}");
        assert!(got.iter().any(|v| v == "msg"), "got {got:?}");

        // tclsh 9.0.4 / 8.6.14:
        // `proc t {} {if {[catch {unset gone}]} {puts $gone}}`
        // `catch {t} err` → 1, err → `can't read "gone": no such variable`
        let got = cond_out_vars("catch {unset gone}");
        assert!(
            !got.iter().any(|v| v == "gone"),
            "`unset` destroys rather than defines; got {got:?}"
        );
    }

    /// A dynamic target contributes no *name* — `push_out_var` only accepts a
    /// bare identifier, so `set $n 1` in a condition stays silent rather than
    /// inventing a def called `n`.
    #[test]
    fn condition_out_vars_ignores_a_dynamic_target() {
        let got = cond_out_vars("set $n 1");
        assert_eq!(got.len(), 0, "got {got:?}");
    }

    /// A command word that is itself a substitution names
    /// an unknown command, so nothing may be harvested from it — not even when
    /// its *content* spelling collides with a builtin.
    ///
    /// tclsh 9.0.4 / 8.6.14: `proc f {set} {if {[$set length foo]} {puts
    /// $length}}; f string` runs `string length foo` and then errors
    /// `can't read "length": no such variable`.
    #[test]
    fn condition_out_vars_ignores_a_substituted_command_head() {
        for cmd_text in ["$set length foo", "$catch {error x} msg", "[pick] $fp line"] {
            let got = cond_out_vars(cmd_text);
            assert_eq!(
                got.len(),
                0,
                "`{cmd_text}` has a computed head; nothing is knowable about \
what it writes, got {got:?}"
            );
        }
    }

    /// TP control for the pair above: the same words under a *literal* head
    /// keep harvesting, so the guard keys on substitution rather than on the
    /// spelling.
    #[test]
    fn condition_out_vars_still_harvests_a_literal_head() {
        assert!(
            cond_out_vars("set length foo")
                .iter()
                .any(|v| v == "length")
        );
        assert!(
            cond_out_vars("catch {error x} msg")
                .iter()
                .any(|v| v == "msg")
        );
        assert!(cond_out_vars("gets $fp line").iter().any(|v| v == "line"));
    }

    fn assert_absolute_method_ir(module: &crate::ir::Module) {
        assert!(
            matches!(
                module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::exact"))
                    .expect("authored fixture body")
                    .body
                    .statements
                    .first(),
                Some(Statement::Return { expr: Some(_), .. })
            ),
            "absolute nested expr must retain typed return IR: {:#?}",
            module
                .methods
                .get(&fixture_id(&module.methods, "::C::exact"))
                .expect("authored fixture body")
                .body
        );
        assert!(
            matches!(
                module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::exact_info"))
                    .expect("authored fixture body")
                    .body
                    .statements
                    .first(),
                Some(Statement::If { .. })
            ),
            "absolute if must retain typed control IR: {:#?}",
            module
                .methods
                .get(&fixture_id(&module.methods, "::C::exact_info"))
                .expect("authored fixture body")
                .body
        );
        assert!(
            matches!(
                module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::exact_regex"))
                    .expect("authored fixture body")
                    .body
                    .statements
                    .first(),
                Some(Statement::AssignConst { .. })
            ),
            "absolute set must retain typed assignment IR: {:#?}",
            module
                .methods
                .get(&fixture_id(&module.methods, "::C::exact_regex"))
                .expect("authored fixture body")
                .body
        );
    }

    #[test]
    fn runtime_selected_method_namespace_inventory_requires_absolute_heads() {
        let registry = tcl_registry::default_registry();
        let module = crate::lowering::lower_to_ir(
            "interp alias {} e {} expr\n\
             oo::class create C {\n\
                 method helper {} {::return 42}\n\
                 method m {} {::set unused [my helper]; ::puts done}\n\
                 method dynamic {cmd} {$cmd value}\n\
                 method global_body {} {::uplevel #0 {set x 1}}\n\
                 method exact {p} {::return [::expr {$p + 1}]}\n\
                 method exact_info {p} {::if {[::info exists p]} {::puts yes}}\n\
                 method exact_regex {s} {::set re {a+}; ::regexp $re $s}\n\
                 method nested_alias {p} {::set x [e {$p + 1}]}\n\
             }",
            registry,
        );
        assert!(!requires_runtime_command_namespace(
            &module
                .methods
                .get(&fixture_id(&module.methods, "::C::helper"))
                .expect("authored fixture body")
                .body,
            registry
        ));
        assert!(
            requires_runtime_command_namespace(
                &module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::m"))
                    .expect("authored fixture body")
                    .body,
                registry
            ),
            "the receiver namespace can shadow TclOO's injected `my` command"
        );
        assert!(
            !requires_runtime_command_namespace(
                &module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::dynamic"))
                    .expect("authored fixture body")
                    .body,
                registry
            ),
            "a computed head is already an opaque CFG barrier, not a lexical fallback dependency"
        );
        assert!(
            !requires_runtime_command_namespace(
                &module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::global_body"))
                    .expect("authored fixture body")
                    .body,
                registry
            ),
            "uplevel #0 selects the exact global namespace for its nested body"
        );
        for name in ["::C::exact", "::C::exact_info", "::C::exact_regex"] {
            let body = &module
                .methods
                .get(&fixture_id(&module.methods, name))
                .expect("authored fixture body")
                .body;
            assert!(
                !requires_runtime_command_namespace(body, registry),
                "absolute-only method {name} must remain analysable: {body:#?}"
            );
        }
        assert!(
            requires_runtime_command_namespace(
                &module
                    .methods
                    .get(&fixture_id(&module.methods, "::C::nested_alias"))
                    .expect("authored fixture body")
                    .body,
                registry
            ),
            "a relative nested binding consumed by typed lowering can be shadowed by the object namespace"
        );
        assert_absolute_method_ir(&module);
    }
}

#[cfg(test)]
mod original_recovered_source_purpose_tests {

    use super::*;

    #[test]
    fn original_recovered_word_keeps_bom_and_supplied_input_through_nested_inventory() {
        // naming.compiler.original-analysis-metadata-context

        // Conditional lexical inventory; no original child execution or Normal handler grant.
        let mut prepared = crate::analyser::Analyser::new();
        let base = prepared.prepare_analysis_input("tcl9.1").unwrap();
        let config = LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            strict_quoting: true,
            ..base.lexer_config()
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            base.analyser_profile(),
            base.unit_profile(),
            base.context_registry(),
            config,
        );
        let context = input.context_registry();
        let registry = context.commands();
        let text = "\u{feff}#prefix ${a{b}";
        let words = tokenise_command_words(text, config);
        assert_eq!(words.len(), 1);
        assert_eq!(words[0][0].literal(), Some("\u{feff}#prefix"));
        assert_eq!(words[0][1].raw, "${a{b}");
        assert!(words[0][1].substituted);
        assert!(tokenise_command_words("#prefix ${a{b}", config).is_empty());
        let source = "\u{feff}puts \u{feff}#prefix[list ${a{b}]";
        let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
            .with_resolved_analysis_input(input.clone());
        lowerer.lower(source);
        let module = lowerer.finish_module(source);
        let statement = &module.top_level.statements[0];
        let metadata = crate::registry_invocation::InvocationMetadataContext::for_analysis_input(
            registry, &input,
        )
        .unwrap();
        let inventory = evaluated_command_substitutions_with_metadata_context(
            statement,
            registry,
            Some(metadata),
            config,
        );
        assert_eq!(inventory.commands.len(), 1);
        assert_eq!(inventory.commands[0][0].literal(), Some("list"));
        assert_eq!(inventory.commands[0][1].raw, "${a{b}");
        assert!(!inventory.opaque);
        assert!(
            evaluated_command_substitutions_with_metadata_context(
                statement, registry, None, config
            )
            .opaque
        );
        assert!(
            evaluated_command_substitutions_with_metadata_context(
                statement,
                registry,
                Some(metadata),
                config.nested()
            )
            .opaque
        );
        let foreign = tcl_registry::model::ingress::static_context_for("tcl8.4");
        assert!(
            evaluated_command_substitutions_with_metadata_context(
                statement,
                registry,
                Some(foreign.into()),
                config
            )
            .opaque
        );
        assert_eq!(module.source_metadata_input.as_ref(), Some(&input));
        assert_eq!(module.lexer_config, config);
    }
}

#[cfg(test)]
mod original_written_source_tests {

    use super::*;

    #[test]
    fn original_recovered_written_words_keep_full_source_and_component_extents_separate() {
        // naming.compiler.original-analysis-metadata-context

        let config = LexerConfig {
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..LexerConfig::default()
        };
        let source = "list {} {é $x} \"[set é]$x\" prefix-${é} ${a{b} {*}[set é]";
        let words = tokenise_command_words(source, config);
        assert_eq!(words.len(), 1);
        assert_eq!(
            words[0]
                .iter()
                .map(|word| word.raw.as_str())
                .collect::<Vec<_>>(),
            vec![
                "list",
                "{}",
                "{é $x}",
                "\"[set é]$x\"",
                "prefix-${é}",
                "${a{b}",
                "{*}[set é]"
            ]
        );
        assert!(words[0].iter().all(CommandWord::has_source_geometry));
        let map = SourceMap::new(source);
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config.nested())
                .remove(0);
        let original = CommandTokens::from_segmented(&map, config, &command);
        assert_eq!(map.text(original.words()[5].source().span), "${a{b");
        assert!(words[0][5].substituted);
        assert!(words[0][6].expanded);
    }

    #[test]
    fn original_written_word_inventory_refuses_changed_cst_vectors_and_source_geometry() {
        // naming.source.original-configured-script-completeness

        let config = LexerConfig::default();
        let source = "prefix; list {é $x}";
        let map = SourceMap::new(source);
        let command =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config.nested())
                .remove(1);
        assert_eq!(command_words(&map, config, &command)[1].raw, "{é $x}");
        let mut changed = command.clone();
        changed.word_fragments[1][0].token.span = tcl_lexer::Span::new(0, 1);
        assert!(
            command_words(&map, config, &changed)
                .iter()
                .all(|word| word.literal().is_none())
        );
        changed = command.clone();
        changed.texts[1] = "FOREIGN".to_owned();
        assert!(
            command_words(&map, config, &changed)
                .iter()
                .all(|word| word.literal().is_none())
        );
        let missing = SourceMap::new("x");
        assert!(command_written_spans(&missing, config, &command).is_none());
        assert!(
            command_words(&missing, config, &command)
                .iter()
                .all(|word| word.literal().is_none())
        );
        changed = command.clone();
        changed.all_tokens[0].content_offset = u8::MAX;
        assert!(
            command_words(&map, config, &changed)
                .iter()
                .all(|word| word.literal().is_none())
        );
        assert_eq!(
            command_words(&map, config, &command)[0].literal(),
            Some("list")
        );
    }
}
