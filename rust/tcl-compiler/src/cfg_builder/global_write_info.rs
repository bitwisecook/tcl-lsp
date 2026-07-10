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

//! Per-proc summary of global/namespace variable writes.
//!
//! Mirrors [`super::upvar_info`]'s shape and role in the pipeline, for the
//! complementary correctness gap: `upvar_info` tells a caller which of
//! *its own* frame variables a callee will alias and write; this module
//! tells a caller which *global/namespace* variables a callee will alias
//! (via `global` / `variable` / `upvar #0`) and write, so that
//! [`super::CfgBuilder::apply_upvar_invalidation`] can widen a call's
//! `defs` for those too. Without this, SCCP treats a bare global/namespace
//! name as an ordinary private local across an opaque call — unsound
//! (`proc mutate {} { global x; set x 99 }; set x 5; mutate; expr {$x+1}`
//! must not fold to `6`; tclsh yields `100`).
//!
//! ## Flow-insensitive, sound over-approximation
//!
//! Real Tcl style declares `global`/`variable` before using the name, but
//! this scan does not depend on that ordering: it first unions every
//! `global` / `variable` / `upvar #0` declaration in the whole proc body
//! (via the shared [`crate::var_observability::stmt_gen`] recognition
//! logic — reused, not re-derived) into one flag state, then checks every
//! write-target name found anywhere in the body against that *complete*
//! state. A write on a conditional branch that never executes together
//! with the declaring branch is still counted — the goal is soundness
//! (never *miss* a global write), not maximal precision.
//!
//! Dynamic (non-literal) `global`/`variable`/`upvar` targets are silently
//! skipped, mirroring [`super::upvar_info::record_upvar_call`]'s existing
//! "can't classify → don't record" policy for dynamic `upvar` sources —
//! this is an accepted, pre-existing-class residual gap, not new
//! unsoundness (a name we can't prove is global/namespace-aliased is
//! simply not added to `names`; it is never wrongly *excluded* from an
//! outer-scope alias that *is* statically classifiable).

use std::collections::{BTreeSet, HashMap};

use crate::ir::{Module, Script, Statement};
use crate::ir_helpers::nested_bodies;
use crate::naming::normalise_var_name;
use crate::var_observability::{State, stmt_gen};

/// Per-proc summary: the outer-scope (global/namespace) variable names a
/// proc's body writes while aliased via `global` / `variable` / `upvar #0`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct GlobalWriteInfo {
    /// Literal outer-scope names this proc's body writes.
    pub names: BTreeSet<String>,
}

impl GlobalWriteInfo {
    /// True when the proc's body writes no outer-scope name.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// Detect, for every proc in `module`, which outer-scope (global/
/// namespace) variable names its body writes — see the module doc for the
/// two-pass, flow-insensitive algorithm. Both the qualified and short
/// forms are registered for every proc, matching
/// [`super::detect_upvar_procs`]'s convention (the caller-side lookup at
/// [`super::CfgBuilder::apply_upvar_invalidation`] resolves a `Statement::
/// Call::command` by whichever spelling appears in source).
///
/// Then closes the per-proc summaries transitively over the direct-call
/// graph (a proc that calls a global-writing proc also "writes" those
/// names, recursively) via a bounded fixpoint — monotonic set-union, so
/// recursive/mutually-recursive cycles terminate safely.
#[must_use]
pub fn detect_global_write_procs(module: &Module) -> HashMap<String, GlobalWriteInfo> {
    // `stmt_gen` needs a registry to resolve the variable-trace grammar, but
    // this scan only ever reads its `writes_outer_scope()` (GLOBAL/NAMESPACE)
    // bit, never `is_traced()` — so a dialect-specific registry is
    // unnecessary here; the default registry resolves `global`/`variable`/
    // `upvar` identically across every dialect.
    let registry = tcl_registry::CommandRegistry::build_default();
    let mut own: HashMap<String, GlobalWriteInfo> = HashMap::new();
    let mut direct_calls: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (qname, proc) in &module.procedures {
        let info = own_body_global_writes(&proc.body, &registry);
        let calls = direct_call_targets(&proc.body);
        for key in registered_keys(qname) {
            own.insert(key.clone(), info.clone());
            direct_calls.insert(key, calls.clone());
        }
    }

    // Transitive closure: monotonic union over direct-callee summaries.
    // Bounded by the number of registered names, so an adversarial call
    // graph still terminates.
    let mut changed = true;
    let mut guard = 0usize;
    while changed && guard <= own.len() {
        changed = false;
        guard += 1;
        let snapshot = own.clone();
        for (caller, callees) in &direct_calls {
            let Some(target_summary) = own.get_mut(caller) else {
                continue;
            };
            for callee in callees {
                let Some(source_summary) = snapshot.get(callee) else {
                    continue;
                };
                for name in &source_summary.names {
                    if target_summary.names.insert(name.clone()) {
                        changed = true;
                    }
                }
            }
        }
    }

    own
}

/// Both the short and fully-qualified forms of `qname`, matching
/// [`super::detect_upvar_procs`]'s registration convention.
fn registered_keys(qname: &str) -> Vec<String> {
    let mut keys = vec![qname.to_owned()];
    if let Some((_, short)) = qname.rsplit_once("::")
        && !short.is_empty()
    {
        keys.push(short.to_owned());
    }
    keys
}

/// Pass 1: union every `global` / `variable` declaration in `body` into one
/// flag state (order-independent), *and* separately collect the
/// (local-alias-name → real-outer-name) map for `upvar #0` / `upvar 0` /
/// `namespace upvar` declarations — those alias a *different* local name to
/// the outer one, so a write to the local name must be attributed to the
/// outer name, not recorded under the local spelling.  `global`/`variable`
/// are identity aliases (the declared name *is* the outer name), so they
/// stay in the flag-state path.  Pass 2: walk `body` again collecting every
/// write-target name, resolving each through the alias map first and
/// falling back to the flag-state identity check.
fn own_body_global_writes(
    body: &Script,
    registry: &tcl_registry::CommandRegistry,
) -> GlobalWriteInfo {
    let mut state = State::default();
    let mut renamed_aliases: HashMap<String, String> = HashMap::new();
    accumulate_state(body, &mut state, &mut renamed_aliases, registry);

    let mut names = BTreeSet::new();
    collect_write_targets(body, &state, &renamed_aliases, &mut names);
    GlobalWriteInfo { names }
}

fn accumulate_state(
    script: &Script,
    state: &mut State,
    renamed_aliases: &mut HashMap<String, String>,
    registry: &tcl_registry::CommandRegistry,
) {
    for stmt in &script.statements {
        stmt_gen(stmt, state, registry);
        collect_renamed_outer_alias(stmt, renamed_aliases);
        for body in nested_bodies(stmt) {
            accumulate_state(body, state, renamed_aliases, registry);
        }
    }
}

/// Record (local → outer) for a literal `upvar #0`/`upvar 0` (global) or
/// `namespace upvar` (namespace) declaration whose *source* word is also
/// literal. Reuses [`crate::var_scoping::upvar_local_declaration_indices`]
/// for the local-name index — the source word is always the immediately
/// preceding argument (the `(src, dst)` pairing both
/// [`crate::var_observability::stmt_gen`] and
/// [`super::upvar_info::upvar_pairs`] rely on).
fn collect_renamed_outer_alias(stmt: &Statement, renamed_aliases: &mut HashMap<String, String>) {
    let (Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. }) = stmt
    else {
        return;
    };
    let cmd = command.as_str();
    let is_ns_upvar = cmd == "namespace" && args.first().map(String::as_str) == Some("upvar");
    let is_global_level =
        cmd == "upvar" && !args.is_empty() && matches!(args[0].trim(), "#0" | "0");
    if !is_ns_upvar && !is_global_level {
        return;
    }
    for local_idx in crate::var_scoping::upvar_local_declaration_indices(cmd, args) {
        let Some(src_idx) = local_idx.checked_sub(1) else {
            continue;
        };
        let (Some(src), Some(dst)) = (args.get(src_idx), args.get(local_idx)) else {
            continue;
        };
        if src.starts_with('$') || dst.starts_with('$') {
            continue; // dynamic — can't classify, matches upvar_info's policy.
        }
        let src_name = normalise_var_name(src);
        let dst_name = normalise_var_name(dst);
        if !src_name.is_empty() && !dst_name.is_empty() {
            renamed_aliases.insert(dst_name.to_owned(), src_name.to_owned());
        }
    }
}

fn collect_write_targets(
    script: &Script,
    state: &State,
    renamed_aliases: &HashMap<String, String>,
    names: &mut BTreeSet<String>,
) {
    for stmt in &script.statements {
        for target in own_write_targets(stmt) {
            let target = normalise_var_name(&target);
            if let Some(outer) = renamed_aliases.get(target) {
                names.insert(outer.clone());
            } else if state.get(target).is_some_and(|f| f.writes_outer_scope()) {
                names.insert(target.to_owned());
            }
        }
        for body in nested_bodies(stmt) {
            collect_write_targets(body, state, renamed_aliases, names);
        }
    }
}

/// The variable name(s) `stmt` itself directly writes — its own `name`
/// field for the direct-assignment statement kinds, `Call::defs` for a
/// direct call to a registry-known var-mutating builtin (`incr x`,
/// `lappend x …`, …; the registry-driven mechanism already used
/// throughout the compiler — see `AGENTS.md`'s "registry is the source of
/// truth"), plus any embedded builtin var-mutator command substitution in
/// the statement's own value/argument text (`set y [incr x]`), reusing
/// [`super::CfgBuilder::builtin_write_defs_from_text`] — the same helper
/// [`super::CfgBuilder::apply_upvar_invalidation`] already uses for this
/// exact embedded-substitution shape.
///
/// Excludes `global` / `variable` / `upvar` / `namespace upvar` / `trace`
/// themselves: the registry may record their declared name in `Call::defs`
/// (they do create a local binding), but declaring an alias does not write
/// the *value* of the variable it aliases — only a subsequent assignment
/// through that alias does.
fn own_write_targets(stmt: &Statement) -> Vec<String> {
    let mut out = Vec::new();
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => out.push(name.clone()),
        Statement::Call {
            command,
            args,
            defs,
            ..
        } => {
            let cmd = command.trim_start_matches("::");
            let is_ns_upvar =
                cmd == "namespace" && args.first().map(String::as_str) == Some("upvar");
            if !is_ns_upvar && !matches!(cmd, "global" | "variable" | "upvar" | "trace") {
                out.extend(defs.iter().cloned());
            }
        }
        _ => {}
    }
    for text in stmt_embedded_texts(stmt) {
        out.extend(super::CfgBuilder::builtin_write_defs_from_text(text));
    }
    out
}

/// Value/argument text fields that may embed a `[cmd …]` command
/// substitution whose head is a builtin var-mutator — the same fields
/// [`super::CfgBuilder::apply_upvar_invalidation`]'s embedded-substitution
/// scan inspects.
fn stmt_embedded_texts(stmt: &Statement) -> Vec<&str> {
    match stmt {
        Statement::AssignValue { value, .. } if value.contains('[') => vec![value.as_str()],
        Statement::Call { args, .. } => args
            .iter()
            .filter(|a| a.contains('['))
            .map(String::as_str)
            .collect(),
        _ => Vec::new(),
    }
}

/// Direct-call targets in `body` — the command name of every
/// `Statement::Call`, recursed into every nested body. Used to build the
/// call graph for the transitive-closure step; resolution against
/// `module.procedures` (qualified vs. short name) happens at the lookup
/// site in [`detect_global_write_procs`].
fn direct_call_targets(body: &Script) -> BTreeSet<String> {
    let mut calls = BTreeSet::new();
    collect_direct_calls(body, &mut calls);
    calls
}

fn collect_direct_calls(script: &Script, calls: &mut BTreeSet<String>) {
    for stmt in &script.statements {
        if let Statement::Call { command, .. } = stmt {
            calls.insert(command.clone());
        }
        for body in nested_bodies(stmt) {
            collect_direct_calls(body, calls);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn module(src: &str) -> Module {
        lower_to_ir(src, &CommandRegistry::build_default())
    }

    #[test]
    fn empty_module_has_no_writes() {
        let m = module("");
        assert!(detect_global_write_procs(&m).is_empty());
    }

    #[test]
    fn proc_with_no_global_write_is_empty() {
        let m = module("proc ::p {} { set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().is_empty());
    }

    #[test]
    fn direct_global_write_recorded() {
        // The `mutate` repro: global x; set x 99.
        let m = module("proc ::mutate {} { global x; set x 99 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::mutate").unwrap().names.contains("x"));
        // Registered under both the qualified and short forms.
        assert!(info.get("mutate").unwrap().names.contains("x"));
    }

    #[test]
    fn variable_namespace_write_recorded() {
        let m = module("proc ::ns::p {} { variable v; set v 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::ns::p").unwrap().names.contains("v"));
    }

    #[test]
    fn upvar_hash_zero_counts_as_global() {
        let m = module("proc ::p {} { upvar #0 g local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("g"));
        // Not the local alias name — recording "local" would be a no-op at
        // the caller (no such caller-frame variable), missing the real "g".
        assert!(!info.get("::p").unwrap().names.contains("local"));
    }

    #[test]
    fn namespace_upvar_rename_resolves_to_outer_name() {
        let m = module("proc ::ns::p {} { namespace upvar ::ns nsvar localvar\nset localvar 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::ns::p").unwrap().names.contains("nsvar"));
        assert!(!info.get("::ns::p").unwrap().names.contains("localvar"));
    }

    #[test]
    fn caller_frame_upvar_is_not_a_global_write() {
        // `upvar 1` aliases the *caller* frame, not global/namespace —
        // must not be treated as an outer-scope write.
        let m = module("proc ::p {} { upvar 1 caller_x local; set local 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().is_empty());
    }

    #[test]
    fn read_only_global_is_not_a_write() {
        let m = module("proc ::p {} { global x; puts $x }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().is_empty());
    }

    #[test]
    fn write_on_conditional_branch_still_counted() {
        // Sound over-approximation: flow-insensitive, so a write inside
        // an `if` still counts even though it may not always execute.
        let m = module("proc ::p {} { global x\nif {$c} { set x 1 } }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn declaration_after_write_still_counted() {
        // Flow-insensitive union: order within the body doesn't matter.
        let m = module("proc ::p {} { set x 1\nglobal x }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn embedded_incr_in_command_substitution_recorded() {
        let m = module("proc ::p {} { global x\nset y [incr x] }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn transitive_closure_through_direct_call() {
        // `outer` calls `mutate`, which writes global x — `outer`'s
        // summary must include `x` too (transitive).
        let m = module("proc ::mutate {} { global x; set x 99 }\nproc ::outer {} { mutate }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::outer").unwrap().names.contains("x"));
    }

    #[test]
    fn recursive_proc_terminates_and_includes_write() {
        let m = module(
            "proc ::p {n} { if {$n == 0} { return }\n global x\n incr x\n p [expr {$n-1}] }",
        );
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
    }

    #[test]
    fn mutually_recursive_procs_share_the_write() {
        let m = module("proc ::a {} { global x; set x 1; b }\nproc ::b {} { a }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::a").unwrap().names.contains("x"));
        assert!(info.get("::b").unwrap().names.contains("x"));
    }

    #[test]
    fn unrelated_global_write_is_not_confused_with_other_procs() {
        let m =
            module("proc ::mutate_y {} { global y; set y 1 }\nproc ::p {} { global x; set x 1 }");
        let info = detect_global_write_procs(&m);
        assert!(info.get("::p").unwrap().names.contains("x"));
        assert!(!info.get("::p").unwrap().names.contains("y"));
    }
}
