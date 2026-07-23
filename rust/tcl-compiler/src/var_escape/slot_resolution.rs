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
use std::sync::OnceLock;

use tcl_registry::{CommandRegistry, Traits};

use crate::ir::{Module, Script, Statement};
use crate::var_escape::types::ProcEscapeSummary;

/// Cap on the indexed-locals array.  Must match the runtime side's
/// locals-array capacity; bumping the runtime side should bump this
/// constant too.
pub const LOCALS_ARRAY_CAP: usize = 16;

/// Commands whose call shape forces the entire proc out of slot
/// resolution.  ``upvar`` / ``global`` / ``variable`` install
/// aliases on the frame's hash bucket; ``info exists`` /
/// ``info vars`` / ``info locals`` walk the bucket;
/// ``trace add variable`` registers against the bucket.
/// Memoised set of commands that alias / read / write variables through
/// the frame's hash bucket by name (`upvar` / `global` / `variable` /
/// `lassign` / `lset` / `regexp` / `regsub` / `scan` / `binary` / `vwait`
/// / `tkwait`), sourced from the registry's [`Traits::FRAME_HASH_BUILTIN`]
/// trait.
fn frame_hash_builtins() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .commands_with_trait(Traits::FRAME_HASH_BUILTIN)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Memoised set of `info` subcommands that introspect by variable name
/// (`info exists|vars|locals|args|default`), sourced from the registry's
/// [`Traits::INTROSPECTS_BY_NAME`] subcommand trait.
fn info_introspecting_subcmds() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .subcommands_with_trait("info", Traits::INTROSPECTS_BY_NAME)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Memoised set of `trace` subcommands that target a variable by name
/// (`trace add|remove|info|variable|vdelete|vinfo`), sourced from the
/// registry's [`Traits::TARGETS_VARIABLE_BY_NAME`] subcommand trait.
fn trace_name_targeting() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .subcommands_with_trait("trace", Traits::TARGETS_VARIABLE_BY_NAME)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Memoised set of commands carrying [`Traits::DYNAMIC_EVAL_BODY`] —
/// those whose body is executed by the interpreter, opening a
/// name-resolution channel back into the local frame (`eval` / `uplevel`
/// / `apply` / `source` / `namespace` / `interp`). Sourced from the
/// registry (the single source of truth); cached once because the set is
/// dialect-agnostic core Tcl.
fn dynamic_eval_set() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        CommandRegistry::build_default()
            .commands_with_trait(Traits::DYNAMIC_EVAL_BODY)
            .into_iter()
            .map(str::to_owned)
            .collect()
    })
}

/// Strip a leading `::` from a command name for canonical lookup.
fn normalise_cmd(cmd: &str) -> &str {
    cmd.strip_prefix("::").unwrap_or(cmd)
}

/// True when *value* is a Tcl value with substitutions.
fn is_dynamic_value(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if value.starts_with('$') || value.starts_with('[') {
        return true;
    }
    value.contains('$') || value.contains('[')
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
/// `while`/`foreach`/`catch`/`try`/`switch`/`Block`/`UpFrame` bodies —
/// issue #996. Transitively bounded today via `MAX_LOWER_NEST_DEPTH`
/// (every `Script` this walk sees was built by `crate::lowering`, which
/// already caps its own construction at 256), capped here independently
/// for defence-in-depth and consistency with every other full-tree walker
/// in this crate.
const MAX_SLOT_WALK_DEPTH: u32 = 256;

/// Walk every statement reachable from *script*, recursing
/// through compound bodies.  Depth-first source order so a
/// slot-assignment loop sees names in the same order codegen
/// emits them. `depth` is the nesting level of `script` — see
/// [`MAX_SLOT_WALK_DEPTH`].
fn walk_statements<'a>(script: &'a Script, out: &mut Vec<&'a Statement>, depth: u32) {
    if depth > MAX_SLOT_WALK_DEPTH {
        return;
    }
    for stmt in &script.statements {
        out.push(stmt);
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
fn stmt_disables_slots(stmt: &Statement, ineligible_names: &mut HashSet<String>) -> bool {
    match stmt {
        Statement::Barrier { .. } => true,
        // Dynamic-name writes via the `Assign*` / `Incr` variants
        // would target any local at runtime; we can't know which
        // one and indexed storage can't satisfy the by-name lookup
        // the runtime would perform.  Disable the whole proc.
        // The IR lowering can preserve dynamic names on these
        // assignment variants directly, so an empty or dynamic
        // name here is enough to bail.
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. }
            if name.is_empty() || is_dynamic_value(name) =>
        {
            true
        }
        Statement::Call { command, args, .. } => {
            if is_dynamic_value(command) {
                return true;
            }
            let cmd = normalise_cmd(command);
            if dynamic_eval_set().contains(cmd) {
                return true;
            }
            if cmd == "set" && !args.is_empty() && is_dynamic_value(&args[0]) {
                return true;
            }
            if cmd == "info" && !args.is_empty() {
                let sub = args[0].as_str();
                if info_introspecting_subcmds().contains(sub) {
                    if args.len() >= 2 {
                        let target = &args[1];
                        if is_dynamic_value(target) {
                            return true;
                        }
                        ineligible_names.insert(target.clone());
                    } else {
                        return true;
                    }
                }
                if sub == "level" || sub == "frame" {
                    return true;
                }
            }
            if cmd == "trace" && !args.is_empty() {
                let sub = args[0].as_str();
                if trace_name_targeting().contains(sub) {
                    if (sub == "add" || sub == "remove" || sub == "info") && args.len() >= 3 {
                        if args[1] == "variable" {
                            let target = &args[2];
                            if is_dynamic_value(target) {
                                return true;
                            }
                            ineligible_names.insert(target.clone());
                        }
                    } else if (sub == "variable" || sub == "vdelete" || sub == "vinfo")
                        && args.len() >= 2
                    {
                        let target = &args[1];
                        if is_dynamic_value(target) {
                            return true;
                        }
                        ineligible_names.insert(target.clone());
                    }
                }
            }
            if frame_hash_builtins().contains(cmd) {
                return true;
            }
            false
        }
        _ => false,
    }
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

    let mut all_stmts: Vec<&Statement> = Vec::new();
    walk_statements(script, &mut all_stmts, 0);

    for stmt in &all_stmts {
        if stmt_disables_slots(stmt, &mut ineligible) {
            return BTreeMap::new();
        }
        // Record assign target names in source order.
        let target: Option<&str> = match stmt {
            Statement::AssignConst { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::Incr { name, .. } => Some(name.as_str()),
            _ => None,
        };
        if let Some(name) = target {
            // Skip names that include array-element notation —
            // those route through the array directory regardless.
            if !name.is_empty()
                && !seen.contains(name)
                && !name.contains('(')
                && !name.starts_with("::")
            {
                ordered_names.push(name.to_string());
                seen.insert(name.to_string());
            }
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
    summaries: &std::collections::HashMap<String, ProcEscapeSummary, S>,
    ir_module: Option<&Module>,
) -> std::collections::HashMap<String, ProcEscapeSummary>
where
    S: std::hash::BuildHasher,
{
    let Some(module) = ir_module else {
        return summaries
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    };
    let mut out: std::collections::HashMap<String, ProcEscapeSummary> =
        std::collections::HashMap::with_capacity(summaries.len());
    for (qname, summary) in summaries {
        if let Some(proc) = module.procedures.get(qname) {
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
        Script {
            statements: Vec::new(),
        }
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
        Script { statements: stmts }
    }

    /// Regression coverage for issue #996: `walk_statements` recurses once
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
        // $varname v`` doesn't get re-routed through an ``IRCall``.
        // Without this guard the slot map could include locals
        // that the dynamic write may target at runtime.
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
    fn populate_local_slots_handles_none_module() {
        use std::collections::HashMap;
        let mut summaries: HashMap<String, ProcEscapeSummary> = HashMap::new();
        summaries.insert("::foo".into(), ProcEscapeSummary::default());
        let result = populate_local_slots(&summaries, None);
        assert_eq!(result.len(), 1);
        assert!(result["::foo"].local_slots.is_empty());
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
