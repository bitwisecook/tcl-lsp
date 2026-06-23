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

use crate::cfg::Function as CfgFunction;
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
type State = HashMap<String, EscapeFlag>;

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
fn stmt_gen(stmt: &Statement, state: &mut State) {
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
    block_entry: HashMap<String, State>,
    ordered_blocks: Vec<String>,
    cfg: &'a CfgFunction,
}

impl VarObservability<'_> {
    fn state_at(&self, block: &str, stmt_idx: usize) -> State {
        let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
        if let Some(blk) = self.cfg.blocks.get(block) {
            for stmt in blk.statements.iter().take(stmt_idx) {
                stmt_gen(stmt, &mut state);
            }
        }
        state
    }

    /// The escape flags of `name` when `block::stmt_idx` executes.
    #[must_use]
    pub fn flag_at(&self, block: &str, stmt_idx: usize, name: &str) -> EscapeFlag {
        self.state_at(block, stmt_idx)
            .get(normalise_var_name(name))
            .copied()
            .unwrap_or_default()
    }

    /// True when `name` is aliased or traced at this point.
    #[must_use]
    pub fn is_escaping_at(&self, block: &str, stmt_idx: usize, name: &str) -> bool {
        !self.flag_at(block, stmt_idx, name).is_empty()
    }

    /// True when `name` is under a `trace` at this point.
    #[must_use]
    pub fn is_traced_at(&self, block: &str, stmt_idx: usize, name: &str) -> bool {
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

/// Compute the flow-sensitive alias/observability lattice for `cfg`.
#[must_use]
pub fn analyse_var_observability(cfg: &CfgFunction) -> VarObservability<'_> {
    let mut preds: HashMap<String, Vec<String>> =
        cfg.blocks.keys().map(|n| (n.clone(), Vec::new())).collect();
    for (name, blk) in &cfg.blocks {
        if let Some(term) = &blk.terminator {
            for succ in term.successors() {
                if let Some(v) = preds.get_mut(succ) {
                    v.push(name.clone());
                }
            }
        }
    }

    let order = cfg.reverse_postorder();
    let mut block_entry: HashMap<String, State> = cfg
        .blocks
        .keys()
        .map(|n| (n.clone(), State::default()))
        .collect();
    let mut block_exit = block_entry.clone();

    // Monotonic forward fixpoint: the per-name lattice is a finite
    // bitset and the union join only rises, so RPO iteration terminates.
    let mut changed = true;
    while changed {
        changed = false;
        for name in &order {
            let mut entry = State::default();
            if let Some(ps) = preds.get(name) {
                for p in ps {
                    join_into(&mut entry, &block_exit[p]);
                }
            }
            block_entry.insert(name.clone(), entry.clone());
            let mut exit_state = entry;
            if let Some(blk) = cfg.blocks.get(name) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut exit_state);
                }
            }
            if exit_state != block_exit[name] {
                block_exit.insert(name.clone(), exit_state);
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
        let entry = fu.cfg.entry.clone();
        // x is never aliased.
        assert!(!obs.is_escaping_at(&entry, 3, "x"));
        // g is GLOBAL-aliased after the `global` declaration.
        assert!(obs.flag_at(&entry, 3, "g").contains(EscapeFlag::GLOBAL));
        assert!(obs.escaping_var_names().contains("g"));
        assert!(!obs.escaping_var_names().contains("x"));
    }

    #[test]
    fn variable_marks_namespace_alias() {
        let c = cu("proc ::p {} { variable v\nset v 1 }");
        let fu = c.function("::p").unwrap();
        let obs = analyse_var_observability(&fu.cfg);
        assert!(
            obs.flag_at(&fu.cfg.entry, 2, "v")
                .contains(EscapeFlag::NAMESPACE)
        );
        assert!(obs.flag_at(&fu.cfg.entry, 2, "v").writes_outer_scope());
    }

    #[test]
    fn trace_marks_observable() {
        let c = cu("proc ::p {} { trace add variable t write cb\nset t 1 }");
        let fu = c.function("::p").unwrap();
        let obs = analyse_var_observability(&fu.cfg);
        assert!(obs.is_traced_at(&fu.cfg.entry, 2, "t"));
        assert!(obs.escaping_var_names().contains("t"));
    }

    #[test]
    fn upvar_level_zero_is_global_other_is_caller() {
        let c0 = cu("proc ::p {} { upvar #0 g loc }");
        let f0 = c0.function("::p").unwrap();
        let o0 = analyse_var_observability(&f0.cfg);
        assert!(
            o0.flag_at(&f0.cfg.entry, 1, "loc")
                .contains(EscapeFlag::GLOBAL)
        );

        let c1 = cu("proc ::p {} { upvar 1 caller loc }");
        let f1 = c1.function("::p").unwrap();
        let o1 = analyse_var_observability(&f1.cfg);
        let f = o1.flag_at(&f1.cfg.entry, 1, "loc");
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
}
