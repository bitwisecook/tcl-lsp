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

//! Flow-sensitive variable alias + trace-observability lattice.
//!
//! Answers, at every program
//! point, *why* an access to a variable is not a private-local access:
//!
//! * **alias** — the name is bound (via `global` / `variable` /
//!   `namespace upvar` / `upvar`) to out-of-frame storage, so a write
//!   may be observed elsewhere;
//! * **observability** — the name is under a `trace`, so *every* access
//!   fires a callback and must not be elided.
//!
//! Computed as a forward data-flow lattice over the CFG ([`EscapeFlag`]
//! is a set-union lattice; `NONE` is ⊥ and the join is bitwise OR).
//! Because the marks are flow-sensitive, a `trace add variable x` or a
//! `global x` only marks accesses that *follow* it.
//!
//! Distinct from [`crate::var_escape`], which answers the *codegen*
//! slot-resolution question (`Local` vs `Frame`) and carries no
//! `TRACED` flag — this is the optimiser-soundness lattice (the
//! foundation for an applicable O104 string-build fold and flow-
//! sensitive alias/trace reasoning in memory-SSA / SCCP / GVN).  The
//! current O104 is hint-only, so this lattice has no optimiser
//! consumer yet; it is the foundation those consumers need.
//!
//! As with [`crate::command_binding`], predecessors come from terminator
//! successors only (the Rust CFG has no explicit exception edges).

use std::collections::HashMap;

use bitflags::bitflags;

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::var_scoping::{
    global_declaration_indices, upvar_local_declaration_indices, variable_declaration_indices,
};

bitflags! {
    /// Why an access to a variable is not a private-local access.  A
    /// set-union lattice: `empty()` is ⊥ and the join of two states is
    /// their bitwise OR.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct EscapeFlag: u8 {
        /// `global` / `upvar #0` — aliases the global frame.
        const GLOBAL = 1 << 0;
        /// `variable` / `namespace upvar` — aliases an enclosing namespace.
        const NAMESPACE = 1 << 1;
        /// `upvar N>=1` — aliases a *caller* frame.
        const UPVAR = 1 << 2;
        /// Under a `trace` — every access is observable.
        const TRACED = 1 << 3;
    }
}

impl EscapeFlag {
    /// True when the name is bound to any out-of-frame storage.
    #[must_use]
    pub fn aliased(self) -> bool {
        self.intersects(Self::GLOBAL | Self::NAMESPACE | Self::UPVAR)
    }

    /// True when a write reaches a global / enclosing-namespace variable.
    /// (Caller-frame `upvar` writes escape to the *caller*, not
    /// necessarily a global, so they are excluded here.)
    #[must_use]
    pub fn writes_outer_scope(self) -> bool {
        self.intersects(Self::GLOBAL | Self::NAMESPACE)
    }

    /// True when the name is under a `trace`.
    #[must_use]
    pub fn is_traced(self) -> bool {
        self.contains(Self::TRACED)
    }
}

/// A per-variable flag map; absent names default to `EscapeFlag::empty()`.
///
/// `pub(crate)` so [`crate::cfg_builder::global_write_info`] can thread the
/// same state type through its own flow-insensitive whole-body walk (it
/// reuses [`stmt_gen`] rather than re-deriving the `global`/`variable`/
/// `upvar` recognition logic).
pub(crate) type State = HashMap<String, EscapeFlag>;

/// Variable named by a `trace` command, or `None`.  Recognises both the
/// `trace add variable NAME …` (8.5+) and the 8.4 `trace variable NAME …`
/// spellings, for any operation.
fn trace_target(args: &[String]) -> Option<&str> {
    if args.len() >= 3 && args[0] == "add" && args[1] == "variable" {
        return Some(&args[2]);
    }
    if args.len() >= 2 && args[0] == "variable" {
        return Some(&args[1]);
    }
    None
}

/// Union `flag` into `state[name]` (after normalising `name`).
fn mark(state: &mut State, args: &[String], idx: usize, flag: EscapeFlag) {
    if let Some(a) = args.get(idx) {
        let name = normalise_var_name(a);
        if !name.is_empty() {
            *state.entry(name.to_owned()).or_default() |= flag;
        }
    }
}

/// Apply `stmt`'s alias / trace declarations to `state` in place.
///
/// `pub(crate)`: reused by [`crate::cfg_builder::global_write_info`] for its
/// own flow-insensitive whole-body scan — the recognition logic for
/// `global` / `variable` / `upvar` / `trace` lives here once.
pub(crate) fn stmt_gen(stmt: &Statement, state: &mut State) {
    let (Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. }) = stmt
    else {
        return;
    };
    // Alias / trace declarations key off the canonical command name.
    let canon = stmt.canonical_command_or_source();
    match canon.strip_prefix("::").unwrap_or(canon) {
        "global" => {
            for i in global_declaration_indices(args) {
                mark(state, args, i, EscapeFlag::GLOBAL);
            }
        }
        "variable" => {
            for i in variable_declaration_indices(args) {
                mark(state, args, i, EscapeFlag::NAMESPACE);
            }
        }
        "trace" => {
            if let Some(t) = trace_target(args) {
                let nm = normalise_var_name(t);
                if !nm.is_empty() {
                    *state.entry(nm.to_owned()).or_default() |= EscapeFlag::TRACED;
                }
            }
        }
        _ => {}
    }

    // `upvar` / `namespace upvar` are recognised structurally on the
    // *source* command (the IR command for `namespace upvar` is
    // `namespace`).  `upvar #0` / `0` aliases the global frame; any other
    // level aliases a caller frame; `namespace upvar` aliases a namespace.
    let cmd = command.as_str();
    let is_ns_upvar = cmd == "namespace" && args.first().map(String::as_str) == Some("upvar");
    let mut upvar_flag = if is_ns_upvar {
        EscapeFlag::NAMESPACE
    } else {
        EscapeFlag::UPVAR
    };
    if cmd == "upvar" && !args.is_empty() {
        let level = args[0].trim();
        if level == "#0" || level == "0" {
            upvar_flag = EscapeFlag::GLOBAL;
        }
    }
    for i in upvar_local_declaration_indices(cmd, args) {
        mark(state, args, i, upvar_flag);
    }
}

/// Union `src` into `dst`; return `true` if `dst` changed.
fn join_into(dst: &mut State, src: &State) -> bool {
    let mut changed = false;
    for (name, &flag) in src {
        let entry = dst.entry(name.clone()).or_default();
        let merged = *entry | flag;
        if merged != *entry {
            *entry = merged;
            changed = true;
        }
    }
    changed
}

/// Result of the alias/observability analysis for one function.
pub struct VarObservability<'a> {
    block_entry: HashMap<BlockId, State>,
    ordered_blocks: Vec<BlockId>,
    cfg: &'a CfgFunction,
}

impl VarObservability<'_> {
    fn state_at(&self, block: BlockId, stmt_idx: usize) -> State {
        let mut state = self.block_entry.get(&block).cloned().unwrap_or_default();
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for stmt in blk.statements.iter().take(stmt_idx) {
                stmt_gen(stmt, &mut state);
            }
        }
        state
    }

    /// The escape flags of `name` when `block::stmt_idx` executes.
    #[must_use]
    pub fn flag_at(&self, block: BlockId, stmt_idx: usize, name: &str) -> EscapeFlag {
        self.state_at(block, stmt_idx)
            .get(normalise_var_name(name))
            .copied()
            .unwrap_or_default()
    }

    /// True when `name` is aliased or traced at this point.
    #[must_use]
    pub fn is_escaping_at(&self, block: BlockId, stmt_idx: usize, name: &str) -> bool {
        !self.flag_at(block, stmt_idx, name).is_empty()
    }

    /// True when `name` is under a `trace` at this point.
    #[must_use]
    pub fn is_traced_at(&self, block: BlockId, stmt_idx: usize, name: &str) -> bool {
        self.flag_at(block, stmt_idx, name).is_traced()
    }

    /// Whole-function union: every name that is ever aliased or traced
    /// at any point in the body.  The flow-insensitive view.
    #[must_use]
    pub fn escaping_var_names(&self) -> std::collections::HashSet<String> {
        let mut names = std::collections::HashSet::new();
        for block in &self.ordered_blocks {
            let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
            collect_escaping(&state, &mut names);
            if let Some(blk) = self.cfg.blocks.get(block) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut state);
                    collect_escaping(&state, &mut names);
                }
            }
        }
        names
    }
}

fn collect_escaping(state: &State, names: &mut std::collections::HashSet<String>) {
    for (name, flag) in state {
        if !flag.is_empty() {
            names.insert(name.clone());
        }
    }
}

/// Whole-module scan: every (normalised) variable name declared via a
/// literal `global NAME …` statement anywhere in the module — the
/// top-level script, every procedure, every `TclOO` method body, and every
/// synthetic body unit (`apply` lambda / `namespace eval` body).
///
/// The per-function escaping-set computed by [`analyse_var_observability`]
/// (and consulted by [`crate::sccp::sccp`]) answers "is this name aliased
/// *within this function's own body*" — sound for an ordinary procedure,
/// whose local frame is genuinely private unless *that body itself*
/// declares `global`/`variable`/`upvar`. It is unsound for the *top-level*
/// script: top-level names already live in the global frame (there is no
/// separate local frame for them to shadow), so a name the top-level body
/// never mentions via `global` can still be reassigned mid-run by any
/// *other* procedure's own `global NAME; set NAME …` — an ordinary call,
/// with nothing textually resembling an alias, from the top level's point
/// of view.  (Reproduced against tclsh 8.6/9.0: `set n 1; proc p {} {global
/// n; set n 2}; p; puts $n` prints `2`; before this scan fed into SCCP as
/// `extra_escaping`, the optimiser proposed folding the final `puts` to the
/// stale literal `1`.)
///
/// This whole-module union is fed into the *top-level* unit's SCCP build
/// (see `CompilationUnit::build_for_with_config`) as `extra_escaping` to
/// close that gap; per-procedure/method scoping needs no such widening —
/// each already protects its own declared aliases flow-sensitively.
#[must_use]
pub fn scan_module_global_names(
    ir_module: &crate::ir::Module,
) -> std::collections::HashSet<String> {
    use crate::ir::{Script, Statement, for_each_statement};
    use crate::var_scoping::global_declaration_indices;

    let mut names = std::collections::HashSet::new();
    let mut visit = |script: &Script| {
        for_each_statement(script, &mut |stmt| {
            let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
                return;
            };
            let canon = stmt.canonical_command_or_source();
            if canon.strip_prefix("::").unwrap_or(canon) != "global" {
                return;
            }
            for i in global_declaration_indices(args) {
                if let Some(a) = args.get(i) {
                    let name = normalise_var_name(a);
                    if !name.is_empty() {
                        names.insert(name.to_owned());
                    }
                }
            }
        });
    };
    visit(&ir_module.top_level);
    for proc in ir_module.procedures.values() {
        visit(&proc.body);
    }
    for method in ir_module.methods.values() {
        visit(&method.body);
    }
    for body in ir_module.body_units.values() {
        visit(&body.body);
    }
    names
}

/// Compute the flow-sensitive alias/observability lattice for `cfg`.
#[must_use]
pub fn analyse_var_observability(cfg: &CfgFunction) -> VarObservability<'_> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|id| (*id, Vec::new())).collect();
    for (&id, blk) in &cfg.blocks {
        if let Some(term) = &blk.terminator {
            for succ in term.successors() {
                if let Some(v) = preds.get_mut(&succ) {
                    v.push(id);
                }
            }
        }
    }

    let order = cfg.reverse_postorder();
    let mut block_entry: HashMap<BlockId, State> = cfg
        .blocks
        .keys()
        .map(|id| (*id, State::default()))
        .collect();
    let mut block_exit = block_entry.clone();

    // Monotonic forward fixpoint: the per-name lattice is a finite
    // bitset and the union join only rises, so RPO iteration terminates.
    let mut changed = true;
    while changed {
        changed = false;
        for &id in &order {
            let mut entry = State::default();
            if let Some(ps) = preds.get(&id) {
                for p in ps {
                    join_into(&mut entry, &block_exit[p]);
                }
            }
            block_entry.insert(id, entry.clone());
            let mut exit_state = entry;
            if let Some(blk) = cfg.blocks.get(&id) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut exit_state);
                }
            }
            if exit_state != block_exit[&id] {
                block_exit.insert(id, exit_state);
                changed = true;
            }
        }
    }

    VarObservability {
        block_entry,
        ordered_blocks: order,
        cfg,
    }
}

/// Module-wide (flow-insensitive, call-order-independent) summary of every
/// `trace add variable` / `trace variable` target across the whole module —
/// top level, every proc, every method — the [`crate::command_binding::
/// ModuleCommandMutations`] pattern applied to variable traces instead of
/// command bindings.
///
/// [`analyse_var_observability`]'s flow-sensitive `TRACED` flag only
/// recognises a trace that is added *and read in the same function body*: a
/// trace installed by one proc and observed through a variable read in a
/// *different* proc — reached via a call whose relative order isn't
/// statically known — is invisible to it. That gap is real and unsound:
///
/// ```tcl
/// proc install_trace {} { trace add variable ::x write {apply {a {set ::x 99}}} }
/// set x 5
/// install_trace
/// set x 6
/// puts [expr {$x + 1}]     ;# tclsh: 100 (the trace fires on `set x 6`).
/// ```
///
/// Rather than a full interprocedural call-order fixpoint, this takes the
/// same sound over-approximation `ModuleCommandMutations` does: a name traced
/// *anywhere* in the module is treated as traced *everywhere*, for the
/// lifetime of the module (a later `trace remove` does not "untrace" it here
/// — the fact is flow-insensitive by design, so it only ever prevents an
/// unsound fold, never causes one).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ModuleVariableTraces {
    /// Literal variable names traced anywhere in the module.
    names: std::collections::BTreeSet<String>,
    /// A body traces a dynamically-computed target (`trace add variable
    /// $name write …`) — resolution of *any* name is opaque, mirroring
    /// [`crate::command_binding::ModuleCommandMutations`]'s `dynamic` flag.
    dynamic: bool,
}

impl ModuleVariableTraces {
    /// True when `name` is traced somewhere in the module (or the module
    /// traces a dynamically-computed target, which could be any name).
    #[must_use]
    pub fn is_traced(&self, name: &str) -> bool {
        self.dynamic || self.names.contains(canonical_outer_name(name))
    }
}

/// Canonicalise a variable name for module-wide outer-scope matching: strip
/// a `$`/`${…}`/array-index wrapper (via [`normalise_var_name`]), then a
/// single leading `::` — the top-level scope *is* the global namespace, so
/// `trace add variable ::x …` and a bare top-level `set x …` name the same
/// variable, and must canonicalise to the same string for [`ModuleVariableTraces::is_traced`]
/// to recognise them as one. A deeper `::ns::x` is left with its remaining
/// `::` intact: a bare local can never collide with it, and any *direct*
/// reference to `::ns::x` elsewhere is already unconditionally treated as
/// externally mutable by SCCP's own `starts_with("::")` rule, so this lookup
/// is never reached for that spelling.
fn canonical_outer_name(name: &str) -> &str {
    let n = normalise_var_name(name);
    n.strip_prefix("::").unwrap_or(n)
}

/// Scan `module` for every `trace add variable` / `trace variable` target —
/// top level, every proc, every method — recursing into nested control-flow
/// bodies via [`crate::ir_helpers::nested_bodies`].
#[must_use]
pub fn scan_module_variable_traces(module: &crate::ir::Module) -> ModuleVariableTraces {
    let mut names = std::collections::BTreeSet::new();
    let mut dynamic = false;

    let mut visit = |script: &crate::ir::Script| {
        walk_trace_targets(script, &mut names, &mut dynamic);
    };
    visit(&module.top_level);
    for proc in module.procedures.values() {
        visit(&proc.body);
    }
    for method in module.methods.values() {
        visit(&method.body);
    }

    ModuleVariableTraces { names, dynamic }
}

fn walk_trace_targets(
    script: &crate::ir::Script,
    names: &mut std::collections::BTreeSet<String>,
    dynamic: &mut bool,
) {
    for stmt in &script.statements {
        record_trace_target(stmt, names, dynamic);
        for body in crate::ir_helpers::nested_bodies(stmt) {
            walk_trace_targets(body, names, dynamic);
        }
    }
}

/// Record `stmt`'s trace target (if any) into `names`, or set `dynamic` when
/// the target is not a literal name. Mirrors [`stmt_gen`]'s `"trace"` arm,
/// but module-wide rather than keyed into a per-block flag state.
fn record_trace_target(
    stmt: &Statement,
    names: &mut std::collections::BTreeSet<String>,
    dynamic: &mut bool,
) {
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
        return;
    };
    let canon = stmt.canonical_command_or_source();
    if canon.strip_prefix("::").unwrap_or(canon) != "trace" {
        return;
    }
    let Some(target) = trace_target(args) else {
        return;
    };
    if target.starts_with('$') {
        *dynamic = true;
        return;
    }
    let name = canonical_outer_name(target);
    if name.is_empty() {
        *dynamic = true;
        return;
    }
    names.insert(name.to_owned());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn cu(src: &str) -> CompilationUnit {
        CompilationUnit::build_for(src, &CommandRegistry::build_default(), false)
    }

    #[test]
    fn global_marks_following_accesses_only() {
        // `set x 1` (private) then `global g` → only g is flagged, and
        // only from the `global` onward.
        let c = cu("proc ::p {} { set x 1\nglobal g\nset g 2 }");
        let fu = c.function("::p").unwrap();
        let obs = analyse_var_observability(&fu.cfg);
        let entry = fu.cfg.entry;
        // x is never aliased.
        assert!(!obs.is_escaping_at(entry, 3, "x"));
        // g is GLOBAL-aliased after the `global` declaration.
        assert!(obs.flag_at(entry, 3, "g").contains(EscapeFlag::GLOBAL));
        assert!(obs.escaping_var_names().contains("g"));
        assert!(!obs.escaping_var_names().contains("x"));
    }

    #[test]
    fn variable_marks_namespace_alias() {
        let c = cu("proc ::p {} { variable v\nset v 1 }");
        let fu = c.function("::p").unwrap();
        let obs = analyse_var_observability(&fu.cfg);
        assert!(
            obs.flag_at(fu.cfg.entry, 2, "v")
                .contains(EscapeFlag::NAMESPACE)
        );
        assert!(obs.flag_at(fu.cfg.entry, 2, "v").writes_outer_scope());
    }

    #[test]
    fn trace_marks_observable() {
        let c = cu("proc ::p {} { trace add variable t write cb\nset t 1 }");
        let fu = c.function("::p").unwrap();
        let obs = analyse_var_observability(&fu.cfg);
        assert!(obs.is_traced_at(fu.cfg.entry, 2, "t"));
        assert!(obs.escaping_var_names().contains("t"));
    }

    #[test]
    fn upvar_level_zero_is_global_other_is_caller() {
        let c0 = cu("proc ::p {} { upvar #0 g loc }");
        let f0 = c0.function("::p").unwrap();
        let o0 = analyse_var_observability(&f0.cfg);
        assert!(
            o0.flag_at(f0.cfg.entry, 1, "loc")
                .contains(EscapeFlag::GLOBAL)
        );

        let c1 = cu("proc ::p {} { upvar 1 caller loc }");
        let f1 = c1.function("::p").unwrap();
        let o1 = analyse_var_observability(&f1.cfg);
        let f = o1.flag_at(f1.cfg.entry, 1, "loc");
        assert!(f.contains(EscapeFlag::UPVAR));
        assert!(f.aliased());
        assert!(
            !f.writes_outer_scope(),
            "caller-frame upvar is not outer-scope"
        );
    }

    #[test]
    fn private_local_has_no_flags() {
        let c = cu("proc ::p {} { set x 1\nset y $x }");
        let fu = c.function("::p").unwrap();
        let obs = analyse_var_observability(&fu.cfg);
        assert!(obs.escaping_var_names().is_empty());
        assert!(EscapeFlag::empty().is_empty());
    }

    fn module(src: &str) -> crate::ir::Module {
        crate::lowering::lower_to_ir(src, &CommandRegistry::build_default())
    }

    #[test]
    fn empty_module_traces_nothing() {
        let m = module("");
        assert_eq!(
            scan_module_variable_traces(&m),
            ModuleVariableTraces::default()
        );
    }

    #[test]
    fn top_level_trace_recorded() {
        let m = module("trace add variable ::x write cb");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("::x"));
        assert!(t.is_traced("x"));
        assert!(!t.is_traced("y"));
    }

    #[test]
    fn trace_inside_helper_proc_recorded_module_wide() {
        // The F4b repro: the trace is installed by a *different* proc than
        // the one reading the variable — a module-wide, not per-function,
        // fact.
        let m = module("proc install_trace {} { trace add variable ::x write cb }\nset x 5");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("x"));
    }

    #[test]
    fn legacy_8_4_trace_variable_spelling_recorded() {
        let m = module("proc p {} { trace variable v w cb }");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("v"));
    }

    #[test]
    fn trace_inside_nested_control_flow_recorded() {
        let m = module("proc p {} { if {$c} { trace add variable v write cb } }");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("v"));
    }

    #[test]
    fn unrelated_name_not_traced() {
        let m = module("proc p {} { trace add variable v write cb }");
        let t = scan_module_variable_traces(&m);
        assert!(!t.is_traced("other"));
    }

    #[test]
    fn dynamic_trace_target_forces_every_name_traced() {
        let m = module("proc p {name} { trace add variable $name write cb }");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("anything"));
        assert!(t.is_traced("x"));
    }

    #[test]
    fn trace_removed_later_still_counted_flow_insensitively() {
        // By design (module doc): the fact is flow-insensitive, so a later
        // `trace remove` does not "untrace" the name — this only ever
        // prevents an unsound fold, never causes one.
        let m =
            module("proc p {} { trace add variable v write cb\ntrace remove variable v write cb }");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("v"));
    }

    #[test]
    fn method_body_trace_recorded() {
        let m = module("oo::class create C {\n method m {} { trace add variable v write cb }\n}");
        let t = scan_module_variable_traces(&m);
        assert!(t.is_traced("v"));
    }

    #[test]
    fn scan_module_global_names_finds_proc_body_global() {
        let c = cu("proc ::p {} { global n\nset n 2 }");
        let names = scan_module_global_names(&c.ir_module);
        assert!(names.contains("n"), "{names:?}");
    }

    #[test]
    fn scan_module_global_names_ignores_local_and_namespace_vars() {
        let c = cu("proc ::p {} { set x 1\nvariable v\nset v 2 }");
        let names = scan_module_global_names(&c.ir_module);
        assert!(names.is_empty(), "{names:?}");
    }

    #[test]
    fn scan_module_global_names_finds_declaration_nested_in_if() {
        // A `global` declaration buried inside a conditional body must still
        // be found — the scan is flow-insensitive (any occurrence counts).
        let c = cu("proc ::p {} { if {1} { global n\nset n 2 } }");
        let names = scan_module_global_names(&c.ir_module);
        assert!(names.contains("n"), "{names:?}");
    }

    #[test]
    fn scan_module_global_names_finds_declaration_in_method_body() {
        let c = cu("oo::class create C {\n method m {} { global n\nset n 2 }\n}");
        let names = scan_module_global_names(&c.ir_module);
        assert!(names.contains("n"), "{names:?}");
    }
}
