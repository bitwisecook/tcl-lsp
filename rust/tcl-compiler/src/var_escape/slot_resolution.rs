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

//! Compile-time
//! slot resolution for proc-locals.
//!
//! For every proc whose body satisfies a small set of safety
//! checks the pass assigns indices ``0..LOCALS_ARRAY_CAP-1`` to
//! its scalar literal locals.  The codegen consumer (the WASM
//! emitter on the runtime side) swaps the name-keyed
//! ``tcl_local_set`` / ``tcl_local_get`` calls for the indexed
//! accessors.
//!
//! Eligibility — every check must pass for the proc to receive a
//! slot mapping at all:
//!
//! * ``not summary.dynamic_barrier()`` — pessimistic procs already
//!   use the WASM-local + sync path; the indexed slots stay unused.
//! * ``not summary.has_fallback()`` and ``not summary.has_call_-
//!   fallback()`` — any path that reaches the interpreter would
//!   need a name-keyed bucket on the frame; an indexed slot
//!   wouldn't satisfy ``var_resolve``'s linear scan.
//! * ``upvar_source_names`` empty and not ``unbounded_upvar_-
//!   source()`` — a callee's ``upvar`` reaches into the caller
//!   frame's hash store; redirecting to indexed storage would
//!   break the alias.
//! * No ``upvar`` / ``global`` / ``variable`` / ``info exists`` /
//!   ``info vars`` / ``info locals`` / ``trace add variable`` /
//!   ``trace remove variable`` / ``trace info variable`` against
//!   any local in the body.
//! * No ``eval`` / ``uplevel`` / ``apply`` whose body could name
//!   the local at runtime.
//! * No dynamic name access (``set $varname value``).

use std::collections::{BTreeMap, HashSet};

use tcl_registry::{ArgRole, Traits};

use crate::ir::{Module, Script, Statement};
use crate::var_escape::types::ProcEscapeSummary;

/// Cap on the indexed-locals array.  Must match the runtime side's
/// locals-array capacity; bumping the runtime side should bump this
/// constant too.
pub const LOCALS_ARRAY_CAP: usize = 16;

/// Scalar destination data and its independent original Normal correspondence.
/// Missing source evidence cannot borrow the standalone IR compatibility purpose.
fn typed_local_slot_name<'a>(
    script: &Script,
    statement: &'a Statement,
) -> Result<Option<&'a str>, ()> {
    let name = match statement {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => name.as_str(),
        _ => return Ok(None),
    };
    let tokens = script.retained_source_tokens_for_statement(statement);
    if let Some(tokens) = tokens {
        let binding = tokens.source_binding.as_ref().ok_or(())?;
        let registry = binding.retained_registry_for_tokens(tokens).ok_or(())?;
        let metadata = binding
            .original_source_metadata_for_advice(tokens, registry)
            .map_err(|_| ())?;
        let normal =
            crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
            .ok_or(())?;
        let selected =
            crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
            .or_else(|| {
                crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                    registry, metadata, tokens,
                )
            })
            .ok_or(())?;
        if normal.descriptor_key() != Some(selected.facts.descriptor_key())
            || normal.argument_offset() != selected.facts.argument_offset
        {
            return Err(());
        }
        let argument = selected.facts.argument_offset;
        let word = selected.effective.words.get(argument + 1).ok_or(())?;
        if !matches!(
            selected.effective.origins.get(argument + 1),
            Some(crate::registry_invocation::InvocationWordOrigin::Written(_))
        ) || crate::registry_invocation::compiled_local_name_word(word)
            != crate::registry_invocation::CompiledLocalNameWord::Direct
            || name.contains('\0')
        {
            return Err(());
        }
        let (original, unknown) =
            crate::registry_invocation::original_assignment_name_operand(registry, tokens, name)
                .ok_or(())?;
        if unknown
            || normal.argument_literal(normal.argument_offset()).as_deref()
                != Some(original.as_str())
            || binding.variable_context.frame_kind != crate::var_resolve::VariableFrameKind::Local
        {
            return Err(());
        }
        let policy = binding
            .variable_context
            .invocation_dialect
            .and_then(|dialect| dialect.variable_lookup_policy)
            .ok_or(())?;
        if tcl_syntax::naming::split_element_ref(name).is_some()
            || !tcl_syntax::naming::source_variable_name_is_frame_local(name.as_bytes(), policy)
        {
            return Ok(None);
        }
        return Ok(Some(name));
    }
    if script.executed_source.is_some()
        || script.command_binding_sites.iter().next().is_some()
        || statement.tokens().is_some()
    {
        return Err(());
    }
    if crate::ir_helpers::structural_variable_write_projection(statement).opaque_variable_frame {
        return Err(());
    }
    // Unattributed IR retains only the common, unambiguous scalar shape.
    if name.is_empty() || tcl_syntax::naming::is_dynamic_word(name) {
        return Err(());
    }
    if name.contains('(') || tcl_syntax::naming::is_qualified(name.as_bytes()) {
        return Ok(None);
    }
    Ok(Some(name))
}

/// Pre-screen on the escape summary alone.  Cheap path — bails
/// before walking the IR for procs that the analysis already
/// flagged as needing a runtime frame.
fn proc_is_slot_eligible(summary: &ProcEscapeSummary) -> bool {
    if summary.dynamic_barrier() {
        return false;
    }
    if summary.has_fallback() || summary.has_call_fallback() {
        return false;
    }
    if !summary.upvar_source_names.is_empty() || summary.unbounded_upvar_source() {
        return false;
    }
    true
}

/// Depth cap for [`walk_statements`]'s recursion over nested `if`/`for`/
/// `while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame` bodies.
/// Transitively bounded via `MAX_LOWER_NEST_DEPTH`
/// (every `Script` this walk sees was built by `crate::lowering`, which
/// already caps its own construction at 256), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker
/// in this crate.
const MAX_SLOT_WALK_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// Walk every statement reachable from *script*, recursing
/// through compound bodies.  Depth-first source order so a
/// slot-assignment loop sees names in the same order codegen
/// emits them. `depth` is the nesting level of `script` — see
/// [`MAX_SLOT_WALK_DEPTH`].
fn walk_statements<'a>(script: &'a Script, out: &mut Vec<(&'a Script, &'a Statement)>, depth: u32) {
    if MAX_SLOT_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        out.push((script, stmt));
        match stmt {
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    walk_statements(&clause.body, out, depth + 1);
                }
                if let Some(b) = else_body {
                    walk_statements(b, out, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_statements(init, out, depth + 1);
                walk_statements(next, out, depth + 1);
                walk_statements(body, out, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Foreach { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Block { body, .. }
            | Statement::UpFrame { body, .. } => walk_statements(body, out, depth + 1),
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_statements(body, out, depth + 1);
                for h in handlers {
                    walk_statements(&h.body, out, depth + 1);
                }
                if let Some(fb) = finally_body {
                    walk_statements(fb, out, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for arm in arms {
                    if let Some(b) = &arm.body {
                        walk_statements(b, out, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    walk_statements(b, out, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Inspect *stmt* for patterns that force a particular local —
/// or every local — to the hash store.  Updates *`ineligible_names`*
/// in place with any literal-named locals the statement targets via
/// a by-name builtin or introspection sub.
///
/// Returns ``true`` when the statement's shape forces the entire
/// proc to skip slot resolution.
fn stmt_disables_slots(
    script: &Script,
    stmt: &Statement,
    ineligible_names: &mut HashSet<String>,
) -> bool {
    match stmt {
        Statement::Barrier { .. } => true,
        Statement::AssignConst { .. }
        | Statement::AssignExpr { .. }
        | Statement::AssignValue { .. }
        | Statement::Incr { .. } => typed_local_slot_name(script, stmt).is_err(),
        Statement::Call { .. } => call_disables_slots(script, stmt, ineligible_names),
        _ => false,
    }
}

/// The verdict on one call, from the registry's declarations for the
/// invocation: `true` takes the whole proc out of slot resolution; otherwise
/// the literal locals its by-name words target join *`ineligible_names`*.
fn call_disables_slots(
    script: &Script,
    statement: &Statement,
    ineligible_names: &mut HashSet<String>,
) -> bool {
    let Some(tokens) = script.retained_source_tokens_for_statement(statement) else {
        return true;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return true;
    };
    let Some(registry) = binding.retained_registry_for_tokens(tokens) else {
        return true;
    };
    if binding.variable_context.frame_kind != crate::var_resolve::VariableFrameKind::Local {
        return true;
    }
    let Ok(metadata) = binding.original_source_metadata_for_advice(tokens, registry) else {
        return true;
    };
    let Some(normal) =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )
    else {
        return true;
    };
    if !normal.is_frameless_runtime() {
        return true;
    }
    let Some(invocation) =
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            registry, metadata, tokens,
        )
        .or_else(|| {
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        })
    else {
        return true;
    };
    let facts = &invocation.facts;
    if !facts.arg_roles_complete
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || facts.traits.intersects(
            Traits::DYNAMIC_EVAL_BODY
                | Traits::FRAME_HASH_BUILTIN
                | Traits::CURRENT_FRAME_INTROSPECTION,
        )
    {
        return true;
    }
    let Some(realm) = binding.invocation_realm() else {
        return true;
    };
    let writes_value_word = invocation.with_argument_words(|words| {
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                metadata.map(crate::registry_invocation::InvocationMetadataContext::context),
                words,
                realm,
            )
            .resolved()?;
        (selected.descriptor_key() == facts.descriptor_key())
            .then(|| selected.semantics.value.writes_value_word())
    });
    let Some(writes_value_word) = writes_value_word else {
        return true;
    };
    let mut has_variable_operand = false;
    for &(index, role) in &facts.arg_roles {
        if !matches!(role, ArgRole::VarRead | ArgRole::VarWrite) {
            continue;
        }
        has_variable_operand = true;
        let Some(name) = invocation.argument_literal(facts.argument_offset + usize::from(index))
        else {
            return true;
        };
        if !writes_value_word
            && facts
                .traits
                .intersects(Traits::INTROSPECTS_BY_NAME | Traits::TARGETS_VARIABLE_BY_NAME)
        {
            ineligible_names.insert(name);
        }
    }
    facts.traits.contains(Traits::TARGETS_VARIABLE_BY_NAME)
        && !has_variable_operand
        && !facts
            .arg_roles
            .iter()
            .any(|(_, role)| *role == ArgRole::CommandName)
}

/// Return a `{local_name: slot_index}` mapping for *script*'s
/// proc-locals that pass the slot-resolution eligibility check.
///
/// *params* is the proc's parameter list; eligible parameters get
/// the lowest-numbered slots so the prologue's param-binding
/// routes through the indexed setter.  Locals introduced by the
/// body claim slots in registration order.
///
/// Returns an empty map when the proc isn't slot-resolvable at
/// all (dynamic eval, `has_fallback`, etc.).  Caller folds the
/// result into the summary via
/// [`ProcEscapeSummary::with_local_slots`].
#[must_use]
pub fn assign_local_slots(
    script: &Script,
    summary: &ProcEscapeSummary,
    params: &[String],
) -> BTreeMap<String, u32> {
    if !proc_is_slot_eligible(summary) {
        return BTreeMap::new();
    }

    let mut ineligible: HashSet<String> = HashSet::new();
    let mut ordered_names: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Reserve slots for parameters first — codegen's prologue
    // binds them via ``tcl_local_set`` per Tcl semantics, and we
    // want ``info args`` consumers to see them in declaration
    // order.
    for pname in params {
        if !pname.is_empty() && !seen.contains(pname) {
            ordered_names.push(pname.clone());
            seen.insert(pname.clone());
        }
    }

    let mut all_stmts: Vec<(&Script, &Statement)> = Vec::new();
    walk_statements(script, &mut all_stmts, 0);

    for (owner, stmt) in &all_stmts {
        if stmt_disables_slots(owner, stmt, &mut ineligible) {
            return BTreeMap::new();
        }
        if let Ok(Some(name)) = typed_local_slot_name(owner, stmt)
            && seen.insert(name.to_owned())
        {
            ordered_names.push(name.to_owned());
        }
    }

    // Filter out names that any ineligibility hit caught.  Cap at
    // LOCALS_ARRAY_CAP — names beyond that fall back to the hash
    // store automatically.
    let mut slot_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut idx: u32 = 0;
    for name in ordered_names {
        if ineligible.contains(&name) {
            continue;
        }
        if (idx as usize) >= LOCALS_ARRAY_CAP {
            break;
        }
        slot_map.insert(name, idx);
        idx += 1;
    }
    slot_map
}

/// Walk every proc in *`ir_module`*, run [`assign_local_slots`],
/// and fold the result into each summary via
/// [`ProcEscapeSummary::with_local_slots`].
///
/// Pure transformation — never mutates the input map.  Procs that
/// aren't slot-eligible round-trip unchanged.
pub fn populate_local_slots<S>(
    summaries: &std::collections::HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary, S>,
    ir_module: Option<&Module>,
) -> std::collections::HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary>
where
    S: std::hash::BuildHasher,
{
    let Some(module) = ir_module else {
        return summaries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    };
    let mut out: std::collections::HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary> =
        std::collections::HashMap::with_capacity(summaries.len());
    for (qname, summary) in summaries {
        if let tcl_core_types::BodyFrameId::Body(id) = qname
            && let Some(proc) = module
                .procedures
                .get(id)
                .or_else(|| module.body_lowering(id))
        {
            let slots = assign_local_slots(&proc.body, summary, &proc.params);
            if slots.is_empty() {
                out.insert(qname.clone(), summary.clone());
            } else {
                out.insert(qname.clone(), summary.with_local_slots(slots));
            }
        } else {
            out.insert(qname.clone(), summary.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Script;
    use crate::var_escape::types::EscapeFlags;
    use tcl_lexer::Span;

    fn empty_script() -> Script {
        Script::new()
    }

    fn assign_const(name: &str, value: &str) -> Statement {
        Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.to_string(),
            value: value.to_string(),
            name_braced: false,
            value_span: None,
        }
    }

    fn call(command: &str, args: &[&str]) -> Statement {
        Statement::Call {
            span: Span::new(0, 0),
            command: command.to_string(),
            canonical_command: None,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    fn script_with(stmts: Vec<Statement>) -> Script {
        Script::from_statements(stmts)
    }

    /// `walk_statements` recurses once
    /// per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`/
    /// `Block`/`UpFrame` body, with no depth cap of its own before this
    /// fix. Transitively bounded to `MAX_LOWER_NEST_DEPTH` (256) by the
    /// lowering pass today, so this is defence-in-depth / consistency with
    /// every other full-tree walker in this crate, not a
    /// currently-reproducible crash. 2000 levels is comfortably past this
    /// new cap; the assertion is that `assign_local_slots` returns at
    /// all, not what it returns.
    #[test]
    fn deeply_nested_if_survives_walk_statements() {
        use crate::expr_ast::ExprNode;
        use crate::ir::IfClause;

        const DEPTH: usize = 2000;
        let mut body = script_with(vec![assign_const("leaf", "1")]);
        for _ in 0..DEPTH {
            body = script_with(vec![Statement::If {
                span: Span::new(0, 0),
                clauses: vec![IfClause {
                    condition: ExprNode::Raw { text: "1".into() },
                    condition_span: Span::new(0, 0),
                    body,
                    body_span: Span::new(0, 0),
                    condition_base: None,
                }],
                else_body: None,
                else_span: None,
            }]);
        }
        let s = ProcEscapeSummary::default();
        let _ = assign_local_slots(&body, &s, &[]);
    }

    #[test]
    fn empty_proc_returns_empty_map() {
        let s = ProcEscapeSummary::default();
        let slots = assign_local_slots(&empty_script(), &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn pessimistic_proc_skips_slot_assignment() {
        let s = ProcEscapeSummary {
            flags: EscapeFlags::DYNAMIC_BARRIER,
            ..Default::default()
        };
        let body = script_with(vec![assign_const("x", "1")]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn fallback_proc_skips_slot_assignment() {
        let s = ProcEscapeSummary {
            flags: EscapeFlags::HAS_FALLBACK,
            ..Default::default()
        };
        let body = script_with(vec![assign_const("x", "1")]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn upvar_source_names_skip_slot_assignment() {
        let s = ProcEscapeSummary {
            upvar_source_names: ["caller_var".into()].into_iter().collect(),
            ..Default::default()
        };
        let body = script_with(vec![assign_const("x", "1")]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn params_get_lowest_slots_in_order() {
        let s = ProcEscapeSummary::default();
        let body = empty_script();
        let params = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let slots = assign_local_slots(&body, &s, &params);
        assert_eq!(slots.get("a"), Some(&0));
        assert_eq!(slots.get("b"), Some(&1));
        assert_eq!(slots.get("c"), Some(&2));
    }

    #[test]
    fn body_locals_assigned_after_params() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![assign_const("x", "1"), assign_const("y", "2")]);
        let params = vec!["p".to_string()];
        let slots = assign_local_slots(&body, &s, &params);
        assert_eq!(slots.get("p"), Some(&0));
        assert_eq!(slots.get("x"), Some(&1));
        assert_eq!(slots.get("y"), Some(&2));
    }

    #[test]
    fn array_element_names_are_skipped() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            assign_const("arr(key)", "v"),
            assign_const("y", "2"),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert_eq!(slots.get("x"), Some(&0));
        assert!(!slots.contains_key("arr(key)"));
        assert_eq!(slots.get("y"), Some(&1));
    }

    #[test]
    fn qualified_names_are_skipped() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            assign_const("::ns::g", "v"),
            assign_const("y", "2"),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert_eq!(slots.get("x"), Some(&0));
        assert!(!slots.contains_key("::ns::g"));
        assert_eq!(slots.get("y"), Some(&1));
    }

    #[test]
    fn upvar_call_disables_whole_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            call("upvar", &["1", "caller", "alias"]),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn eval_command_disables_whole_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![assign_const("x", "1"), call("eval", &["body"])]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn slot_metadata_uses_registry_rooted_selection_and_refuses_unknown_heads() {
        for head in [
            "upvar",
            "::upvar",
            "eval",
            "::eval",
            "::::upvar",
            ":upvar",
            "upvar::",
            "upvar:::member",
            "missing",
            "",
        ] {
            let body = script_with(vec![
                assign_const("x", "1"),
                call(head, &["1", "source", "target"]),
            ]);
            assert!(assign_local_slots(&body, &ProcEscapeSummary::default(), &[]).is_empty());
        }
        for head in ["list", "::list"] {
            let body = script_with(vec![assign_const("x", "1"), call(head, &["value"])]);
            assert_eq!(
                assign_local_slots(&body, &ProcEscapeSummary::default(), &[]).get("x"),
                Some(&0)
            );
        }
    }

    #[test]
    fn dynamic_set_disables_whole_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            call("set", &["$varname", "v"]),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn info_exists_marks_target_ineligible_but_keeps_others() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            assign_const("y", "2"),
            call("info", &["exists", "x"]),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        // ``x`` was named by ``info exists`` so loses its slot;
        // ``y`` is unaffected and gets index 0 (registration order
        // after filtering).
        assert!(!slots.contains_key("x"));
        assert_eq!(slots.get("y"), Some(&0));
    }

    #[test]
    fn info_exists_dynamic_target_disables_whole_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            call("info", &["exists", "$dyn"]),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn info_level_or_frame_disables_whole_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![assign_const("x", "1"), call("info", &["level"])]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());

        let body2 = script_with(vec![assign_const("x", "1"), call("info", &["frame"])]);
        let slots2 = assign_local_slots(&body2, &s, &[]);
        assert!(slots2.is_empty());
    }

    #[test]
    fn trace_add_variable_marks_target_ineligible() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            assign_const("y", "2"),
            call("trace", &["add", "variable", "x", "write", "cb"]),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(!slots.contains_key("x"));
        assert_eq!(slots.get("y"), Some(&0));
    }

    /// The trace target is the word the registry gives the `VarWrite` role,
    /// so an abbreviated type resolves as `trace` does; a command trace names
    /// no local; a trace form that places no variable takes the proc out.
    #[test]
    fn trace_targets_are_the_registry_roles() {
        let s = ProcEscapeSummary::default();
        let with = |call_stmt: Statement| {
            let body = script_with(vec![
                assign_const("x", "1"),
                assign_const("y", "2"),
                call_stmt,
            ]);
            assign_local_slots(&body, &s, &[])
        };
        let slots = with(call("trace", &["add", "var", "x", "write", "cb"]));
        assert!(!slots.contains_key("x"));
        assert_eq!(slots.get("y"), Some(&0));
        let slots = with(call("trace", &["add", "command", "x", "rename", "cb"]));
        assert_eq!(slots.len(), 2);
        assert!(with(call("trace", &["vinfo", "x"])).is_empty());
        assert!(with(call("trace", &["info", "variable", "x"])).is_empty());
    }

    /// Every invocation that observes the current frame without naming a
    /// variable takes the proc out: `info level` and `info frame` as before,
    /// and `info coroutine` and `info errorstack` beside them.
    #[test]
    fn current_frame_introspection_disables_whole_proc() {
        let s = ProcEscapeSummary::default();
        for sub in ["coroutine", "errorstack"] {
            let body = script_with(vec![assign_const("x", "1"), call("info", &[sub])]);
            assert!(assign_local_slots(&body, &s, &[]).is_empty(), "info {sub}");
        }
    }

    #[test]
    fn trace_add_variable_dynamic_target_disables_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            call("trace", &["add", "variable", "$dyn", "write", "cb"]),
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn cap_at_locals_array_cap() {
        let s = ProcEscapeSummary::default();
        let stmts: Vec<Statement> = (0..(LOCALS_ARRAY_CAP + 5))
            .map(|i| assign_const(&format!("v{i}"), "1"))
            .collect();
        let body = script_with(stmts);
        let slots = assign_local_slots(&body, &s, &[]);
        assert_eq!(slots.len(), LOCALS_ARRAY_CAP);
        // First N must be in the map.
        for i in 0..LOCALS_ARRAY_CAP {
            assert!(slots.contains_key(&format!("v{i}")));
        }
        // Beyond cap is not in the map.
        for i in LOCALS_ARRAY_CAP..(LOCALS_ARRAY_CAP + 5) {
            assert!(!slots.contains_key(&format!("v{i}")));
        }
    }

    #[test]
    fn dynamic_assign_value_name_disables_whole_proc() {
        // Lowering may produce ``Statement::AssignValue { name:
        // "$varname", .. }`` directly when the source ``set
        // $varname v`` doesn't get re-routed through a
        // ``Statement::Call``.  Without this guard the slot map
        // could include locals that the dynamic write may target
        // at runtime.
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            Statement::AssignValue {
                span: Span::new(0, 0),
                name: "$varname".into(),
                value: "v".into(),
                name_braced: false,
                value_needs_backsubst: false,
                tokens: None,
            },
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn dynamic_incr_name_disables_whole_proc() {
        // Same shape via the ``Incr`` variant.
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![
            assign_const("x", "1"),
            Statement::Incr {
                span: Span::new(0, 0),
                name: "$varname".into(),
                name_braced: false,
                amount: None,
                amount_braced: false,
                safe_on_uninit: false,
            },
        ]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn empty_assign_name_disables_whole_proc() {
        // Defensive: an empty name (lowering pathology) is also
        // dynamic-by-effect — disable.
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![assign_const("", "1")]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn dynamic_command_word_disables_proc() {
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![assign_const("x", "1"), call("$cmd", &["arg"])]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn double_colon_prefixed_command_normalises_to_bare_for_match() {
        // ``::upvar`` should be recognised as the upvar command,
        // same as bare ``upvar``.
        let s = ProcEscapeSummary::default();
        let body = script_with(vec![call("::upvar", &["1", "src", "dst"])]);
        let slots = assign_local_slots(&body, &s, &[]);
        assert!(slots.is_empty());
    }

    #[test]
    fn actual_bytecode_frameless_handler_roles_preserve_other_local_slots() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let profile = registry.profile().unwrap();
        for head in ["list", "::list"] {
            let source = format!("proc p {{}} {{set x 1; {head} value}}");
            let module = crate::lowering::lower_to_ir_for_bytecode_with_dialect(
                &source,
                registry,
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                Some(profile),
            );
            let procedure = module.procedures.values().next().unwrap();
            let statement = procedure.body.statements.last().unwrap();
            let tokens = procedure
                .body
                .retained_source_tokens_for_statement(statement)
                .unwrap();
            let binding = tokens.source_binding.as_ref().unwrap();
            let metadata = binding
                .original_source_metadata_for_advice(tokens, registry)
                .unwrap();
            let normal =
                crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                    registry, metadata, tokens,
                )
                .expect("actual selected bytecode handler");
            assert!(normal.is_frameless_runtime());
            assert_eq!(
                assign_local_slots(&procedure.body, &ProcEscapeSummary::default(), &[]).get("x"),
                Some(&0),
            );
        }
    }

    #[test]
    fn original_typed_slot_names_preserve_literal_data_under_actual_lookup_policy() {
        for (label, qualified_local) in [("tcl8.6", false), ("jim", true)] {
            let context = tcl_registry::model::ingress::static_context_for(label);
            let registry = context.commands();
            let profile = registry.profile().unwrap();
            let source = "namespace eval N {}; proc p {} {set {$literal} 1; set {[literal]} 2; set {} 3; set N::x 4}";
            let module = crate::lowering::lower_to_ir_for_bytecode_with_dialect(
                source,
                registry,
                tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                Some(profile),
            );
            let procedure = module
                .procedures
                .values()
                .find(|procedure| procedure.name == "p" || procedure.name == "::p")
                .unwrap();
            let slots = assign_local_slots(&procedure.body, &ProcEscapeSummary::default(), &[]);
            assert_eq!(slots.get("$literal"), Some(&0), "{label}: {slots:?}");
            assert_eq!(slots.get("[literal]"), Some(&1), "{label}: {slots:?}");
            assert_eq!(slots.get(""), Some(&2), "{label}: {slots:?}");
            assert_eq!(
                slots.contains_key("N::x"),
                qualified_local,
                "{label}: {slots:?}"
            );
        }
    }

    #[test]
    fn typed_slot_data_refuses_changed_or_unowned_original_names() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let profile = registry.profile().unwrap();
        let module = crate::lowering::lower_to_ir_for_bytecode_with_dialect(
            "proc p {} {set {$literal} 1}",
            registry,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            Some(profile),
        );
        let procedure = module.procedures.values().next().unwrap();
        assert_eq!(
            assign_local_slots(&procedure.body, &ProcEscapeSummary::default(), &[]).get("$literal"),
            Some(&0)
        );
        let mut changed = procedure.body.clone();
        let statement = changed.statements.first_mut().unwrap();
        match statement {
            Statement::AssignConst { name, .. } => *name = "relabelled".to_owned(),
            _ => panic!("actual typed scalar assignment"),
        }
        assert!(assign_local_slots(&changed, &ProcEscapeSummary::default(), &[]).is_empty());
        let mut missing = procedure.body.clone();
        missing.command_binding_sites = Default::default();
        assert!(assign_local_slots(&missing, &ProcEscapeSummary::default(), &[]).is_empty());
        let mut unowned = assign_const("$literal", "1");
        if let Statement::AssignConst { name_braced, .. } = &mut unowned {
            *name_braced = true;
        }
        assert!(
            assign_local_slots(
                &script_with(vec![unowned]),
                &ProcEscapeSummary::default(),
                &[]
            )
            .is_empty()
        );
    }

    #[test]
    fn unowned_builtin_spelling_cannot_donate_physical_slot_eligibility() {
        let body = script_with(vec![
            assign_const("x", "1"),
            call("string", &["length", "value"]),
        ]);
        assert!(assign_local_slots(&body, &ProcEscapeSummary::default(), &[]).is_empty());
    }

    #[test]
    fn original_global_handler_does_not_supply_local_slot_frame() {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        let profile = registry.profile().unwrap();
        let module = crate::lowering::lower_to_ir_for_bytecode_with_dialect(
            "list value",
            registry,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
            Some(profile),
        );
        let statement = &module.top_level.statements[0];
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let binding = tokens.source_binding.as_ref().unwrap();
        assert_eq!(
            binding.variable_context.frame_kind,
            crate::var_resolve::VariableFrameKind::Global
        );
        let metadata = binding
            .original_source_metadata_for_advice(tokens, registry)
            .unwrap();
        let normal =
            crate::registry_invocation::normal_representation_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
            .unwrap();
        assert!(normal.is_frameless_runtime());
        assert!(call_disables_slots(
            &module.top_level,
            statement,
            &mut HashSet::new()
        ));
    }

    #[test]
    fn authored_source_name_roles_cannot_donate_normal_slot_eligibility() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir("puts value", &registry);
        assert!(module.source_entry.native_entry.is_none());
        assert!(matches!(
            &module.top_level.statements[0],
            Statement::Call { .. }
        ));
        let mut ineligible = HashSet::new();
        assert!(call_disables_slots(
            &module.top_level,
            &module.top_level.statements[0],
            &mut ineligible
        ));
    }

    #[test]
    fn populate_local_slots_handles_none_module() {
        use std::collections::HashMap;
        let id = tcl_core_types::BodyFrameId::Body(
            tcl_core_types::ProcedureIdIssuer::default().allocate(),
        );
        let mut summaries = HashMap::new();
        summaries.insert(id, ProcEscapeSummary::default());
        let result = populate_local_slots(&summaries, None);
        assert_eq!(result.len(), 1);
        assert!(result[&id].local_slots.is_empty());
    }

    #[test]
    fn with_local_slots_preserves_other_fields() {
        let mut tags = std::collections::HashMap::new();
        tags.insert("a".to_string(), crate::var_escape::types::EscapeTag::Frame);
        let s = ProcEscapeSummary {
            tags,
            frame_needed: true,
            ..Default::default()
        };
        let mut slots: BTreeMap<String, u32> = BTreeMap::new();
        slots.insert("x".into(), 0);
        let new = s.with_local_slots(slots);
        assert_eq!(
            new.tags.get("a"),
            Some(&crate::var_escape::types::EscapeTag::Frame)
        );
        assert!(new.frame_needed);
        assert_eq!(new.local_slots.get("x"), Some(&0));
    }
}
