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
#[cfg(test)]
use tcl_registry::StateTransition;
use tcl_registry::{CallerFrameSelection, CommandRegistry, VariableAliasTarget};

use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::normalise_var_name_braced;

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

fn mark_name(state: &mut State, name: &str, flag: EscapeFlag) {
    let name = normalise_var_name_braced(name, true);
    *state.entry(name.to_owned()).or_default() |= flag;
}

fn alias_flag(
    target: &VariableAliasTarget,
    normal: &crate::registry_invocation::NormalTransferInvocation,
) -> EscapeFlag {
    match target {
        VariableAliasTarget::Global { .. } => EscapeFlag::GLOBAL,
        VariableAliasTarget::CurrentNamespace { .. } | VariableAliasTarget::Namespace { .. } => {
            EscapeFlag::NAMESPACE
        }
        VariableAliasTarget::CallerSelectedFrame { frame, .. } => match frame {
            CallerFrameSelection::Explicit(level)
                if level.literal().is_some_and(|_| {
                    normal
                        .variable_alias_frame_level(frame)
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
    let context = standalone_observability_context(registry);
    stmt_gen_with_metadata_context(stmt, state, registry, context);
}

fn standalone_observability_context(
    registry: &CommandRegistry,
) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
    registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile)
        .map(Into::into)
}

/// Apply original alias/trace hazards under supplied availability, not labels.
/// Missing or foreign metadata grants no declaration or physical-store closure.
pub(crate) fn stmt_gen_with_metadata_context(
    stmt: &Statement,
    state: &mut State,
    registry: &CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
) {
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return;
    };
    if let Some(normal) = stmt.tokens().and_then(|tokens| {
        crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
            registry,
            Some(context),
            tokens,
        )
    }) {
        for alias in normal.variable_alias_transitions() {
            if let Some(local) = alias.local.literal() {
                mark_name(state, local, alias_flag(&alias.target, &normal));
            }
        }
    }
    if let Some(possible) = stmt.tokens().and_then(|tokens| {
        crate::registry_invocation::possible_variable_trace_transitions_with_metadata_context(
            registry,
            Some(context),
            tokens,
        )
    }) {
        for trace in possible.transitions() {
            match trace {
                tcl_registry::TraceTransition::Add { target, .. }
                | tcl_registry::TraceTransition::Remove { target, .. } => {
                    if let tcl_registry::TraceTarget::Variable(target) = target
                        && let Some(name) = target.literal()
                    {
                        mark_name(state, name, EscapeFlag::TRACED);
                    }
                }
            }
        }
    }
    // A retained receiver candidate contributes alias hazards only. It does
    // not prove an object namespace, native completion or a physical link.
    if let Some(binding) = stmt
        .tokens()
        .and_then(|tokens| tokens.source_binding.as_ref())
        && let Some(candidates) = binding.receiver_self_builtin_candidates(registry)
        && let Some(operation) = candidates.operation()
    {
        for name in binding
            .evaluated_argument_words
            .iter()
            .skip(1)
            .filter_map(|word| word.as_registry_word().literal())
        {
            if operation.accepts_variable_link_name(name) {
                mark_name(state, name, EscapeFlag::NAMESPACE);
            }
        }
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
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
}

impl VarObservability<'_> {
    fn state_at(&self, block: BlockId, stmt_idx: usize) -> State {
        let mut state = self.block_entry.get(&block).cloned().unwrap_or_default();
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for stmt in blk.statements.iter().take(stmt_idx) {
                stmt_gen_with_metadata_context(stmt, &mut state, self.registry, self.context);
            }
        }
        state
    }

    /// The escape flags of `name` when `block::stmt_idx` executes.
    #[must_use]
    pub fn flag_at(&self, block: BlockId, stmt_idx: usize, name: &str) -> EscapeFlag {
        self.state_at(block, stmt_idx)
            .get(normalise_var_name_braced(name, true))
            .copied()
            .unwrap_or_default()
    }

    /// True when `name` is aliased or traced at this point.
    #[must_use]
    pub fn is_escaping_at(&self, block: BlockId, stmt_idx: usize, name: &str) -> bool {
        !self.flag_at(block, stmt_idx, name).is_empty()
    }

    /// Original dependency names marked aliased or traced at this exact
    /// point. Physical consumers must resolve these through their retained
    /// source environment; display labels are not storage identities.
    #[must_use]
    pub fn escaping_var_names_at(
        &self,
        block: BlockId,
        stmt_idx: usize,
    ) -> std::collections::HashSet<String> {
        self.state_at(block, stmt_idx)
            .into_iter()
            .filter_map(|(name, flags)| (!flags.is_empty()).then_some(name))
            .collect()
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
                    stmt_gen_with_metadata_context(stmt, &mut state, self.registry, self.context);
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
                    stmt_gen_with_metadata_context(stmt, &mut state, self.registry, self.context);
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
///
/// `registry` resolves the alias grammar
/// ([`StateTransition::VariableCellAlias`] onto
/// [`VariableAliasTarget::Global`]), so — exactly as for
/// [`analyse_var_observability`] below — the same dialect the caller lowered
/// `ir_module` under must be passed. A mismatched registry is worse here
/// than there: a name this scan *misses* is a name the top level is told
/// does not escape, which licenses folding a value another procedure
/// reassigns through `global`. There is deliberately no default-registry
/// fallback inside this function.
#[must_use]
pub fn scan_module_global_names(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> std::collections::HashSet<String> {
    use crate::ir::{Script, Statement, for_each_statement};
    let mut names = std::collections::HashSet::new();
    let Some(context) = crate::registry_invocation::retained_source_metadata_context(
        registry,
        ir_module.source_metadata_input.as_ref(),
    ) else {
        return names;
    };
    let mut visit = |script: &Script| {
        for_each_statement(script, &mut |stmt| {
            let (Statement::Call { .. } | Statement::Barrier { .. }) = stmt else {
                return;
            };
            let Some(possible) = stmt.tokens().and_then(|tokens| {
                crate::registry_invocation::possible_variable_alias_transitions_in_context(
                    &context, tokens,
                )
            }) else {
                return;
            };
            for alias in possible.aliases() {
                if matches!(alias.target, VariableAliasTarget::Global { .. })
                    && let Some(local) = alias.local.literal()
                {
                    let name = normalise_var_name_braced(local, true);
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
    analyse_var_observability_with_metadata_context(
        cfg,
        registry,
        standalone_observability_context(registry),
    )
}

/// Alias and trace hazards under one retained metadata generation.
/// The same context is retained for point queries and whole-function replay.
/// Missing or foreign metadata supplies no unobserved-cell or execution proof.
#[must_use]
pub fn analyse_var_observability_with_metadata_context<'a>(
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
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
                    stmt_gen_with_metadata_context(stmt, &mut exit_state, registry, context);
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
        context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    fn registry() -> std::sync::Arc<CommandRegistry> {
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .clone()
    }

    fn cu(src: &str) -> CompilationUnit {
        CompilationUnit::build_for(src, &registry(), false)
    }

    #[test]
    fn retained_availability_survives_observability_point_replay() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // One original Logical body and command store, two availability inputs.
        // These source hazards certify no Native link, frame or trace table.
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "gated-global",
            surface: registry.get("dict").unwrap().surface,
            ..registry.get("global").unwrap().clone()
        });
        let registry = std::sync::Arc::new(registry);
        let compilation = CompilationUnit::build_for(
            "proc p {} {gated-global {$g}; set {$g} VALUE}",
            &registry,
            false,
        );
        let function = compilation.function("::p").unwrap();
        for (environment, available) in [("tcl8.4", false), ("tcl9.0", true)] {
            let context = tcl_registry::model::ingress::static_context_for(environment)
                .with_command_store(std::sync::Arc::clone(&registry));
            let observations = analyse_var_observability_with_metadata_context(
                &function.cfg,
                &registry,
                Some((&context).into()),
            );
            assert_eq!(
                observations.escaping_var_names().contains("$g"),
                available,
                "{environment}"
            );
            assert_eq!(
                observations
                    .flag_at(function.cfg.entry, usize::MAX, "$g")
                    .contains(EscapeFlag::GLOBAL),
                available,
                "{environment}"
            );
            assert!(!observations.escaping_var_names().contains("g"));
            let mut flags = State::new();
            for statement in function
                .cfg
                .blocks
                .values()
                .flat_map(|block| &block.statements)
            {
                stmt_gen_with_metadata_context(
                    statement,
                    &mut flags,
                    &registry,
                    Some((&context).into()),
                );
            }
            assert_eq!(flags.get("$g").is_some(), available, "{environment}");
        }
        let unavailable =
            analyse_var_observability_with_metadata_context(&function.cfg, &registry, None);
        assert!(unavailable.escaping_var_names().is_empty());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let observations = analyse_var_observability_with_metadata_context(
            &function.cfg,
            &registry,
            Some(foreign.into()),
        );
        assert!(observations.escaping_var_names().is_empty());
    }

    #[test]
    fn literal_dollar_alias_names_remain_distinct() {
        let c = cu("proc p {} {global {$g}; set {$g} VALUE; set g LOCAL}");
        let function = c.function("::p").unwrap();
        let reg = registry();
        let observations = analyse_var_observability(&function.cfg, &reg);
        assert!(observations.escaping_var_names().contains("$g"));
        assert!(!observations.escaping_var_names().contains("g"));
        let mut flags = State::new();
        mark_name(&mut flags, "$g", EscapeFlag::TRACED);
        assert_eq!(flags.get("$g"), Some(&EscapeFlag::TRACED));
        assert!(!flags.contains_key("g"));
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
        // scope. The generic receiver inventory supplies a May namespace
        // hazard without certifying one instance's physical alias.
        let c = cu("oo::class create C {method p {} {my variable x; set x 1}}");
        let fu = &c.methods["::C::p"];
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
    fn a_receiver_variable_override_does_not_inherit_builtin_alias_hazards() {
        let unit = cu(
            "oo::class create C {method variable args {return NOOP}; method p {} {my variable x; set x 1}}",
        );
        let method = &unit.methods["::C::p"];
        let registry = registry();
        let observability = analyse_var_observability(&method.cfg, &registry);
        assert!(!observability.escaping_var_names().contains("x"));
    }

    #[test]
    fn a_root_my_spelling_does_not_supply_a_receiver_alias_hazard() {
        let unit = cu("proc p {} {my variable x; set x 1}");
        let registry = registry();
        let observability =
            analyse_var_observability(&unit.function("::p").unwrap().cfg, &registry);
        assert!(!observability.escaping_var_names().contains("x"));
    }

    #[test]
    fn receiver_method_trace_is_a_may_hazard_without_normal_registration() {
        let unit = cu("oo::class create C {method m {} {trace add variable v write cb}}");
        let reg = registry();
        let method = unit.ir_module.methods.values().next().unwrap();
        let tokens = method
            .body
            .statements
            .iter()
            .find_map(Statement::tokens)
            .unwrap();
        let footprint =
            crate::registry_invocation::possible_variable_trace_transitions(&reg, None, tokens)
                .expect("retained stock trace candidate");
        assert!(footprint.unknown_residual());
        assert_eq!(footprint.transitions().count(), 1);
        assert!(
            crate::registry_invocation::normal_transfer_invocation(&reg, None, tokens).is_none()
        );
        assert!(unit.ir_module.traced_variables.contains("v"));
    }

    #[test]
    fn replaced_absolute_trace_cannot_donate_method_observer_metadata() {
        let unit = cu(
            "proc ::trace args {}; oo::class create C {method m {} {::trace add variable v write cb}}",
        );
        let reg = registry();
        let method = unit.ir_module.methods.values().next().unwrap();
        let tokens = method
            .body
            .statements
            .iter()
            .find_map(Statement::tokens)
            .unwrap();
        let footprint =
            crate::registry_invocation::possible_variable_trace_transitions(&reg, None, tokens)
                .expect("document implementation remains an opaque candidate");
        assert_eq!(footprint.transitions().count(), 0);
        assert!(footprint.unknown_residual());
        assert!(!unit.ir_module.traced_variables.contains("v"));
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
    fn exception_edge_join_retains_proved_alias_and_trace_transitions() {
        let unit = cu("proc p {} {trace add variable t write cb; global g; error boom}");
        let mut graph = unit.function("::p").unwrap().cfg.clone();
        let handler = graph.intern_block("observer_exception_handler");
        let origins: Vec<_> = graph.blocks.keys().copied().collect();
        graph.blocks.insert(
            handler,
            crate::cfg::Block::new("observer_exception_handler"),
        );
        graph
            .exception_edges
            .extend(origins.iter().map(|&origin| (origin, handler)));
        let reg = registry();
        let observations = analyse_var_observability(&graph, &reg);
        assert!(
            observations
                .flag_at(handler, 0, "t")
                .contains(EscapeFlag::TRACED)
        );
        assert!(
            observations
                .flag_at(handler, 0, "g")
                .contains(EscapeFlag::GLOBAL)
        );
    }

    #[test]
    fn try_handler_joins_trace_and_alias_state_from_exception_edge() {
        // The body may fail before or after either registry-described state
        // transition.  A handler access must therefore retain both hazards;
        // treating it as a private, untraced local could authorise an invalid
        // load/store elimination.
        let source = "proc ::p {} {\n try {\n  trace add variable t write cb\n  global g\n  error boom\n } on error {} {\n  set t 1\n  set g 2\n }\n}";
        let c = cu(source);
        let fu = c.function("::p").unwrap();
        let handler = fu
            .cfg
            .blocks
            .iter()
            .find_map(|(&id, block)| {
                block
                    .statements
                    .iter()
                    .any(|statement| {
                        let span = statement.span();
                        source
                            .get(span.start() as usize..span.end() as usize)
                            .is_some_and(|text| text.starts_with("set t 1"))
                    })
                    .then_some(id)
            })
            .expect("entered handler store retains its authored source");
        let mut exceptional_path: Vec<_> = fu
            .cfg
            .exception_edges
            .iter()
            .map(|&(_, target)| target)
            .collect();
        let mut visited = std::collections::HashSet::new();
        while let Some(block) = exceptional_path.pop() {
            if visited.insert(block) {
                exceptional_path.extend(fu.cfg.block_successors(block));
            }
        }
        assert!(
            visited.contains(&handler),
            "the entered handler must be reachable from a captured exceptional edge"
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
        let names = scan_module_global_names(&c.ir_module, &registry());
        assert!(names.contains("n"), "{names:?}");
    }

    #[test]
    fn scan_module_global_names_ignores_local_and_namespace_vars() {
        let c = cu("proc ::p {} { set x 1\nvariable v\nset v 2 }");
        let names = scan_module_global_names(&c.ir_module, &registry());
        assert!(names.is_empty(), "{names:?}");
    }

    #[test]
    fn scan_module_global_names_finds_declaration_nested_in_if() {
        // A `global` declaration buried inside a conditional body must still
        // be found — the scan is flow-insensitive (any occurrence counts).
        let c = cu("proc ::p {} { if {1} { global n\nset n 2 } }");
        let names = scan_module_global_names(&c.ir_module, &registry());
        assert!(names.contains("n"), "{names:?}");
    }

    #[test]
    fn scan_module_global_names_finds_declaration_in_method_body() {
        let c = cu("oo::class create C {\n method m {} { global n\nset n 2 }\n}");
        let names = scan_module_global_names(&c.ir_module, &registry());
        assert!(
            names.contains("n"),
            "{names:?}; methods: {:?}",
            c.ir_module
                .methods
                .iter()
                .map(|(name, method)| (
                    name,
                    method
                        .body
                        .statements
                        .iter()
                        .map(|statement| (
                            statement.span(),
                            statement.tokens().map(|tokens| (
                                tokens.source_binding.as_ref().map(|binding| (
                                    binding.variable_frame.clone(),
                                    binding
                                        .execution_targets()
                                        .map(|target| target.command.clone())
                                        .collect::<Vec<_>>(),
                                    binding.execution_is_unknown(),
                                )),
                                tokens.argv_texts.clone(),
                            ))
                        ))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn global_frame_noop_declaration_does_not_invent_a_local_alias() {
        let source = "set n 4; proc helper {} {uplevel #0 {global n; set n 17}}; helper; puts $n";
        let unit = cu(source);
        // Native global is inactive in the selected global frame. The actual
        // outward store is tracked by the canonical cell owner independently.
        assert!(scan_module_global_names(&unit.ir_module, &registry()).is_empty());
        let folds =
            crate::optimiser::optimise_raw_for_profile(source, &registry(), registry().profile());
        assert!(!folds.iter().any(|edit| edit.replacement.contains("puts 4")));
    }

    // ---- registry-threading coverage (issue #1788) ----
    //
    // `scan_module_global_names` used to resolve its alias facts against a
    // hardcoded `tcl8.6` registry. Every test below therefore has to turn on
    // a registry whose answer *differs* from `tcl8.6`'s, or it would pass
    // just as happily with the defect in place.
    //
    // No shipped dialect profile gives `global` a different alias grammar —
    // `global` carries the identical `VariableCellAlias`/`Global`
    // transition on 8.4 through 9.1, iRules, tmsh and every EDA profile — so
    // the difference has to come from the other axis the registry varies on:
    // a command a *custom* registry carries and the plain 8.6 one does not
    // (a `SpecTcl` workspace pack, an authored overlay). That is the axis the
    // issue names, and it is the axis on which a missed name is a miscompile.

    /// The `global`-shaped alias fact, declared by a command no shipped
    /// `tcl8.6` registry knows: argument 0 becomes a local alias of the
    /// same-named global cell.
    ///
    /// Authored as registry data, per the registry rule — nothing in
    /// `scan_module_global_names` knows this command's name.
    fn overlay_alias_transitions(
        arguments: tcl_registry::InvocationArguments<'_>,
    ) -> tcl_registry::StateTransitions {
        let mut transitions = tcl_registry::StateTransitions::default();
        if let Some(variable) = tcl_registry::TransitionSubject::from_argument(arguments, 0) {
            transitions.push(StateTransition::VariableCellAlias(
                tcl_registry::VariableCellAliasTransition {
                    destination: tcl_registry::VariableAliasDestination::CurrentNamespaceOrLocal,
                    local: variable.clone(),
                    target: VariableAliasTarget::Global { variable },
                    writes_value: false,
                },
            ));
        }
        transitions
    }

    /// A one-command overlay spec named `bindglobal`, available on `surface`.
    fn overlay_alias_spec(
        surface: &'static [tcl_dialect::model::SpecSurface],
    ) -> tcl_registry::CommandSpec {
        tcl_registry::CommandSpec {
            name: "bindglobal",
            surface: Some(surface),
            arity: tcl_registry::Arity::any(),
            native_compilation: Some(tcl_registry::native_compilation::NativeCompilationSpec {
                grammar: tcl_registry::native_compilation::NativeCompilationGrammar::NoHook,
                operation: tcl_registry::SemanticOperationId::Invoke,
                body: tcl_registry::native_compilation::NativeBodyCompilation::Direct,
            }),
            arg_roles: &[(0, tcl_registry::ArgRole::VarWrite)],
            assigns_variable_at: Some(0),
            state_transitions: Some(tcl_registry::StateTransitionDescriptor {
                success_resolver: None,
                resolver: Some(overlay_alias_transitions),
                ..tcl_registry::StateTransitionDescriptor::EMPTY
            }),
            ..tcl_registry::CommandSpec::DEFAULT
        }
    }

    /// `dialect`'s cached registry plus the `bindglobal` overlay, keyed so the
    /// two surfaces never share a cache generation.
    fn overlay_registry(
        dialect: &str,
        key: u64,
        surface: &'static [tcl_dialect::model::SpecSurface],
    ) -> std::sync::Arc<CommandRegistry> {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        tcl_registry::registry_for_profile_with_overlay(profile, key, move |registry| {
            registry.insert(overlay_alias_spec(surface));
        })
    }

    /// The module the tests scan: a procedure aliases the top level's `n`
    /// through the overlay command, then reassigns it. A scan that misses
    /// `n` tells the top-level SCCP build `n` never escapes, which licenses
    /// folding the final read to the stale literal `1`.
    const OVERLAY_SOURCE: &str =
        "set g 4\nproc helper {} { bindglobal g\nset g 17 }\nhelper\nputs $g\n";

    #[test]
    fn scan_uses_the_callers_registry_not_a_default_one() {
        // Availability everywhere, so only "which registry" can explain a
        // difference.
        let reg = overlay_registry(
            "tcl9.0",
            0x1788_0001,
            tcl_dialect::model::SpecSurface::ALL_TCL,
        );
        let c = CompilationUnit::build_for(OVERLAY_SOURCE, &reg, false);

        let names = scan_module_global_names(&c.ir_module, &reg);
        assert!(
            names.contains("g"),
            "the caller's registry declares `bindglobal`'s global alias, so `g` \
             must escape: {names:?}",
        );

        // Control: the registry the defect hardcoded cannot see this
        // command at all, so it answers the unsound empty set. This is the
        // difference the test turns on.
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let baseline_names = scan_module_global_names(&c.ir_module, baseline);
        assert!(
            !baseline_names.contains("g"),
            "control: a plain tcl8.6 registry must not know `bindglobal`: {baseline_names:?}",
        );
    }

    #[test]
    fn scan_follows_the_resolved_release_profile() {
        // The same authored transition, gated to Tcl 9.0+. The scan must
        // answer per the release the unit resolved to.
        let on = overlay_registry(
            "tcl9.0",
            0x1788_0002,
            tcl_dialect::model::SpecSurface::TCL90_PLUS,
        );
        let off = overlay_registry(
            "tcl8.6",
            0x1788_0002,
            tcl_dialect::model::SpecSurface::TCL90_PLUS,
        );

        let on_unit = CompilationUnit::build_for(OVERLAY_SOURCE, &on, false);
        let on_names = scan_module_global_names(&on_unit.ir_module, &on);
        assert!(
            on_names.contains("g"),
            "tcl9.0 is inside the declared surface, so `g` escapes: {on_names:?}",
        );

        let off_unit = CompilationUnit::build_for(OVERLAY_SOURCE, &off, false);
        let off_names = scan_module_global_names(&off_unit.ir_module, &off);
        assert!(
            !off_names.contains("g"),
            "tcl8.6 is outside the declared surface, so nothing aliases `g`: {off_names:?}",
        );
    }

    #[test]
    fn scan_default_tcl9_behaviour_is_unchanged() {
        // The shipped answer for plain `global` is release-invariant: the
        // threading must not move Tcl 9.0.4 (or any other profile) off it.
        let src = "set n 1\nproc ::p {} { global n\nset n 2 }\np\nputs $n";
        for dialect in [
            "tcl9.0",
            "tcl9.1",
            "tcl8.6",
            "tcl8.5",
            "tcl8.4",
            "f5-irules",
        ] {
            let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
            let c = CompilationUnit::build_for(src, reg, false);
            let names = scan_module_global_names(&c.ir_module, reg);
            assert!(names.contains("n"), "{dialect}: {names:?}");
            assert_eq!(names.len(), 1, "{dialect}: {names:?}");
        }
    }

    /// Caller 1 — `CompilationUnit`'s `ModuleWideFacts`. The top-level SCCP
    /// lattice must treat `g` as externally mutable because the *registry*
    /// says `bindglobal` aliases it, the same protection
    /// `top_level_var_touched_by_callee_global_is_overdefined` pins for the
    /// core `global` command.
    #[test]
    fn top_level_sccp_respects_a_registry_declared_alias() {
        let reg = overlay_registry(
            "tcl9.0",
            0x1788_0001,
            tcl_dialect::model::SpecSurface::ALL_TCL,
        );
        let cu = CompilationUnit::build_for(OVERLAY_SOURCE, &reg, false);
        let read = cu.top_level.ssa.blocks.iter().find_map(|(&block, body)| {
            body.statements
                .iter()
                .enumerate()
                .find_map(|(index, statement)| {
                    let Statement::Call {
                        command,
                        tokens: Some(tokens),
                        ..
                    } = &statement.statement
                    else {
                        return None;
                    };
                    if command.trim_start_matches(':') != "puts" {
                        return None;
                    }
                    crate::ssa::SsaSourceView::at_statement(&cu.top_level.ssa, block, index)
                        .read_word(tokens.words().get(1)?)
                })
        });
        if let Some(reference) = read
            && let Some(version) = reference.version
        {
            assert_ne!(
                cu.top_level.sccp.values.get(&(reference.symbol, version)),
                Some(&crate::analyses::LatticeValue::Const(
                    crate::analyses::ConstValue::Int(4)
                )),
                "the actual callee mutation must withdraw the old global value",
            );
        }
    }

    /// Caller 2 — `optimiser::propagation::run`. Its own
    /// `top_level_extra_escaping` must come from the unit's registry, or the
    /// pass proposes rewriting `puts $g` to the stale literal `puts 4`.
    #[test]
    fn propagation_does_not_fold_a_registry_declared_alias_target() {
        let reg = overlay_registry(
            "tcl9.0",
            0x1788_0001,
            tcl_dialect::model::SpecSurface::ALL_TCL,
        );
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile();
        let folds: Vec<_> =
            crate::optimiser::optimise_raw_for_profile(OVERLAY_SOURCE, &reg, Some(profile))
                .into_iter()
                .filter(|o| matches!(o.code.as_str(), "O100" | "O102"))
                .map(|o| (o.code.to_string(), o.span.start(), o.replacement))
                .collect();
        assert!(
            folds.is_empty(),
            "`g` may be reassigned through the registry-declared alias, so the \
             final read must not be forwarded: {folds:?}",
        );

        // Control: a registry without that command has no reason to hold
        // back, and does propose the (for this unit, wrong) literal — so the
        // assertion above is about the registry, not about propagation
        // happening to be silent here.
        let baseline = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let baseline_folds: Vec<_> =
            crate::optimiser::optimise_raw_for_profile("set g 4; puts $g", baseline, Some(profile))
                .into_iter()
                .filter(|o| matches!(o.code.as_str(), "O100" | "O102"))
                .collect();
        assert!(
            !baseline_folds.is_empty(),
            "control: an unmodified global value remains eligible for forwarding",
        );
    }
}
