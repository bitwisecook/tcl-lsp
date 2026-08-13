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
//! As with [`crate::command_binding`], predecessors come from
//! [`CfgFunction::block_successors`].  That canonical successor view includes
//! analysis-only `try` exception edges, so handler entry conservatively joins
//! alias and trace state that may have been established before the exception.

use std::collections::HashMap;

use bitflags::bitflags;
use tcl_registry::{CallerFrameSelection, CommandRegistry, StateTransition, VariableAliasTarget};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::lowering::variable_trace_write_indices;
use crate::naming::normalise_var_name;
use crate::var_escape::helpers::{default_registry, invocation_facts};
use crate::var_scoping::my_variable_declaration_indices;

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

/// Union `flag` into `state[name]` (after normalising `name`).
fn mark(state: &mut State, args: &[String], idx: usize, flag: EscapeFlag) {
    if let Some(a) = args.get(idx) {
        let name = normalise_var_name(a);
        if !name.is_empty() {
            *state.entry(name.to_owned()).or_default() |= flag;
        }
    }
}

fn mark_name(state: &mut State, name: &str, flag: EscapeFlag) {
    let name = normalise_var_name(name);
    if !name.is_empty() {
        *state.entry(name.to_owned()).or_default() |= flag;
    }
}

fn alias_flag(
    target: &VariableAliasTarget,
    registry: &tcl_registry::CommandRegistry,
) -> EscapeFlag {
    match target {
        VariableAliasTarget::Global { .. } => EscapeFlag::GLOBAL,
        VariableAliasTarget::CurrentNamespace { .. } | VariableAliasTarget::Namespace { .. } => {
            EscapeFlag::NAMESPACE
        }
        VariableAliasTarget::CallerSelectedFrame { frame, .. } => match frame {
            CallerFrameSelection::Explicit(level)
                if level.literal().is_some_and(|level| {
                    tcl_registry::frame_effect::FrameLevel::parse_in(level, registry)
                        .is_some_and(tcl_registry::frame_effect::FrameLevel::is_global_frame)
                }) =>
            {
                EscapeFlag::GLOBAL
            }
            CallerFrameSelection::DefaultCaller | CallerFrameSelection::Explicit(_) => {
                EscapeFlag::UPVAR
            }
        },
    }
}

/// Apply `stmt`'s alias / trace declarations to `state` in place.
///
/// `pub(crate)`: reused by [`crate::cfg_builder::global_write_info`] for its
/// own flow-insensitive whole-body scan — the recognition logic for
/// `global` / `variable` / `upvar` / `trace` lives here once.
pub(crate) fn stmt_gen(stmt: &Statement, state: &mut State, registry: &CommandRegistry) {
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
        return;
    };
    // Alias / trace declarations key off the canonical command name.
    let canon = stmt.canonical_command_or_source();
    // `my variable NAME …` (TclOO) binds each instance variable into the
    // method's local scope — a namespace-style scope alias, exactly like a
    // bare `variable`, but reached through the `my` dispatch so the base
    // command word is `my` and the declared names follow a `variable`
    // subcommand word. An instance variable's intrep is externally
    // determined (the constructor / other methods can set it to anything),
    // so a use-site / merge / loop-oscillation check must treat it as
    // escaping — the same protection FP-SH-02 / FP-SH-16 give a bare
    // `variable`. Whether the head *is* the self-dispatch keyword comes from
    // the registry, not a name literal (issue #1050); `get` resolves the
    // `::`-qualified spelling itself.
    if registry.method_dispatch_keyword(canon)
        == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
    {
        for i in my_variable_declaration_indices(args) {
            mark(state, args, i, EscapeFlag::NAMESPACE);
        }
    }
    if let Some(facts) = invocation_facts(stmt, registry)
        && let Some(transitions) = facts.state_transitions.declared()
    {
        for fact in transitions.facts() {
            let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if let Some(local) = alias.local.literal() {
                mark_name(state, local, alias_flag(&alias.target, registry));
            }
        }
    }

    // Variable-trace targets, registry-driven: any subcommand carrying
    // `Traits::ESTABLISHES_VARIABLE_TRACE` (`trace add|remove|variable|
    // vdelete` — not the read-only `info`/`vinfo` forms) marks its
    // `ArgRole::VarWrite` target(s) TRACED. Mirrors
    // `crate::lowering::populate_variable_trace_facts`'s whole-module
    // fact via the same shared query, so this carries no hardcoded
    // knowledge of `trace`'s subcommand grammar.
    for i in variable_trace_write_indices(registry, canon, args) {
        mark(state, args, i, EscapeFlag::TRACED);
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
    registry: &'a CommandRegistry,
}

impl VarObservability<'_> {
    fn state_at(&self, block: BlockId, stmt_idx: usize) -> State {
        let mut state = self.block_entry.get(&block).cloned().unwrap_or_default();
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for stmt in blk.statements.iter().take(stmt_idx) {
                stmt_gen(stmt, &mut state, self.registry);
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
                    stmt_gen(stmt, &mut state, self.registry);
                    collect_escaping(&state, &mut names);
                }
            }
        }
        names
    }

    /// Whole-function union: every name that is ever under a `trace` at any
    /// point in the body — [`Self::escaping_var_names`] narrowed to the
    /// `TRACED` flag alone (excluding a plain `global` / `variable` /
    /// `upvar` alias with no trace). The flow-insensitive view: a trace
    /// added partway through the function is treated as covering the whole
    /// function, the same conservative widening [`Self::escaping_var_names`]
    /// already applies for SCCP.
    #[must_use]
    pub fn traced_var_names(&self) -> std::collections::HashSet<String> {
        let mut names = std::collections::HashSet::new();
        for block in &self.ordered_blocks {
            let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
            collect_traced(&state, &mut names);
            if let Some(blk) = self.cfg.blocks.get(block) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut state, self.registry);
                    collect_traced(&state, &mut names);
                }
            }
        }
        names
    }
}

fn collect_traced(state: &State, names: &mut std::collections::HashSet<String>) {
    for (name, flag) in state {
        if flag.is_traced() {
            names.insert(name.clone());
        }
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
    let mut names = std::collections::HashSet::new();
    let registry = default_registry();
    let mut visit = |script: &Script| {
        for_each_statement(script, &mut |stmt| {
            let (Statement::Call { .. } | Statement::Barrier { .. }) = stmt else {
                return;
            };
            let Some(facts) = invocation_facts(stmt, registry) else {
                return;
            };
            let Some(transitions) = facts.state_transitions.declared() else {
                return;
            };
            for fact in transitions.facts() {
                let StateTransition::VariableCellAlias(alias) = &fact.transition else {
                    continue;
                };
                if matches!(alias.target, VariableAliasTarget::Global { .. })
                    && let Some(local) = alias.local.literal()
                {
                    let name = normalise_var_name(local);
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
///
/// `registry` resolves the variable-trace grammar (`Traits::
/// ESTABLISHES_VARIABLE_TRACE` + `ArgRole::VarWrite`), so the same
/// dialect the caller lowered `cfg` under must be passed — a mismatched
/// registry could silently miss (or misidentify) a trace-target
/// position.
#[must_use]
pub fn analyse_var_observability<'a>(
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
) -> VarObservability<'a> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|id| (*id, Vec::new())).collect();
    for &id in cfg.blocks.keys() {
        for succ in cfg.block_successors(id) {
            if let Some(v) = preds.get_mut(&succ) {
                v.push(id);
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
                    stmt_gen(stmt, &mut exit_state, registry);
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
        registry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn cu(src: &str) -> CompilationUnit {
        CompilationUnit::build_for(src, &registry(), false)
    }

    #[test]
    fn global_marks_following_accesses_only() {
        // `set x 1` (private) then `global g` → only g is flagged, and
        // only from the `global` onward.
        let c = cu("proc ::p {} { set x 1\nglobal g\nset g 2 }");
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
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
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(
            obs.flag_at(fu.cfg.entry, 2, "v")
                .contains(EscapeFlag::NAMESPACE)
        );
        assert!(obs.flag_at(fu.cfg.entry, 2, "v").writes_outer_scope());
    }

    #[test]
    fn my_variable_marks_namespace_alias() {
        // `my variable x` (TclOO) binds an instance variable into the method
        // scope — a namespace-style escape, exactly like a bare `variable`.
        let c = cu("proc ::p {} { my variable x\nset x 1 }");
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(
            obs.flag_at(fu.cfg.entry, 2, "x")
                .contains(EscapeFlag::NAMESPACE),
            "my variable x should mark x as a namespace alias"
        );
        assert!(obs.escaping_var_names().contains("x"));
    }

    #[test]
    fn trace_marks_observable() {
        let c = cu("proc ::p {} { trace add variable t write cb\nset t 1 }");
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(obs.is_traced_at(fu.cfg.entry, 2, "t"));
        assert!(obs.escaping_var_names().contains("t"));
        assert!(obs.traced_var_names().contains("t"));
    }

    #[test]
    fn try_handler_joins_trace_and_alias_state_from_exception_edge() {
        // The body may fail before or after either registry-described state
        // transition.  A handler access must therefore retain both hazards;
        // treating it as a private, untraced local could authorise an invalid
        // load/store elimination.
        let c = cu(
            "proc ::p {} {\n try {\n  trace add variable t write cb\n  global g\n } on error {} {\n  set t 1\n  set g 2\n }\n}",
        );
        let fu = c.function("::p").unwrap();
        let handler = fu
            .cfg
            .blocks
            .iter()
            .find_map(|(&id, block)| block.name.starts_with("try_handler").then_some(id))
            .expect("try handler block");
        assert!(
            fu.cfg
                .exception_edges
                .iter()
                .any(|&(_, target)| target == handler),
            "the test must exercise an analysis-only exception edge"
        );

        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(
            obs.flag_at(handler, 0, "t").contains(EscapeFlag::TRACED),
            "handler access must remain trace-observable"
        );
        assert!(
            obs.flag_at(handler, 0, "g").contains(EscapeFlag::GLOBAL),
            "handler access must retain the global alias"
        );
    }

    /// [`VarObservability::traced_var_names`] narrows `escaping_var_names`
    /// to the `TRACED` flag alone — a plain alias with no trace (`global g`)
    /// must not appear in it, even though it does appear in the broader
    /// `escaping_var_names` set.
    #[test]
    fn traced_var_names_excludes_untraced_aliases() {
        let c = cu("proc ::p {} { global g\nset g 1\ntrace add variable t write cb\nset t 1 }");
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(obs.escaping_var_names().contains("g"));
        assert!(obs.escaping_var_names().contains("t"));
        assert!(
            !obs.traced_var_names().contains("g"),
            "an untraced global alias must not appear in traced_var_names"
        );
        assert!(obs.traced_var_names().contains("t"));
    }

    #[test]
    fn legacy_trace_variable_form_marks_observable() {
        // The deprecated `trace variable name ops command` spelling (8.4-8.6
        // only) must mark the target the same as the modern `trace add
        // variable` form — no per-form gap in the registry-driven query.
        let c = cu("proc ::p {} { trace variable t w cb\nset t 1 }");
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(obs.is_traced_at(fu.cfg.entry, 2, "t"));
        assert!(obs.escaping_var_names().contains("t"));
    }

    #[test]
    fn trace_through_interp_alias_marks_observable() {
        // `interp alias {} tracer {} trace` means `tracer add variable ...`
        // is really a `trace add variable ...` call — `stmt_gen` must key
        // off the canonical (alias-resolved) command name when locating the
        // trace-target argument, not the source-surface `tracer` spelling.
        let c = cu(
            "interp alias {} tracer {} trace\nproc ::p {} { tracer add variable t write cb\nset t 1 }",
        );
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(obs.is_traced_at(fu.cfg.entry, 2, "t"));
        assert!(obs.escaping_var_names().contains("t"));
    }

    #[test]
    fn trace_add_execution_does_not_mark_a_variable() {
        // `trace add execution` targets a *command* name, not a variable —
        // must not spuriously flag its target as a TRACED variable.
        let c = cu("proc ::p {} { trace add execution foo enter cb\nset foo 1 }");
        let fu = c.function("::p").unwrap();
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(!obs.is_traced_at(fu.cfg.entry, 2, "foo"));
    }

    #[test]
    fn upvar_level_zero_is_global_other_is_caller() {
        let reg = registry();
        let c0 = cu("proc ::p {} { upvar #0 g loc }");
        let f0 = c0.function("::p").unwrap();
        let o0 = analyse_var_observability(&f0.cfg, &reg);
        assert!(
            o0.flag_at(f0.cfg.entry, 1, "loc")
                .contains(EscapeFlag::GLOBAL)
        );

        let c1 = cu("proc ::p {} { upvar 1 caller loc }");
        let f1 = c1.function("::p").unwrap();
        let o1 = analyse_var_observability(&f1.cfg, &reg);
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
        let reg = registry();
        let obs = analyse_var_observability(&fu.cfg, &reg);
        assert!(obs.escaping_var_names().is_empty());
        assert!(EscapeFlag::empty().is_empty());
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

    #[test]
    fn scan_module_global_names_finds_declaration_inside_static_uplevel_body() {
        // FN guard (P1, code review): a `global` declaration hidden inside a
        // static-body `uplevel #0 { ... }` lowers to `Statement::UpFrame`,
        // not a plain nested block — `for_each_statement` must still descend
        // into it. Confirmed against tclsh 8.6: `set g 4; proc helper {}
        // { uplevel #0 { global g; set g 17 } }; helper; puts $g` prints
        // `17`, so missing this name here would let SCCP/O102 fold the
        // final read to the stale literal `4`.
        let c = cu("proc ::helper {} { uplevel #0 { global n\nset n 2 } }");
        let names = scan_module_global_names(&c.ir_module);
        assert!(names.contains("n"), "{names:?}");
    }
}
