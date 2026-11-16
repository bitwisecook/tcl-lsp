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

//! `UndefSuppression`: the name-level facts that excuse a possibly undefined read, and their construction.

use super::existence_guards::collect_existence_guards;
use super::hidden_writes::{
    collect_expr_cmd_sub_writes, collect_script_concat_writes, collect_unseen_call_sites,
};
use super::phi_undef::{
    CallStepMap, PhiUndefCtx, PhiUndefMemo, UndefIndexMaps, build_phi_undef_index, call_steps,
    phi_can_undef,
};
use crate::cfg::BlockId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;
use tcl_dialect::model::SurfaceQuery;

/// Name-level suppression context for the `return`-value phi-from-undef W210
/// pass, harvested from `dict with` / `dict update` and qualified `variable`
/// declarations.
#[derive(Default)]
pub(in super::super) struct UndefSuppression {
    /// A `dict with` / `dict update` is present (enables the key-aware gate).
    has_dict_with: bool,
    /// At least one dict-with target's value shape is statically unknown.
    dict_with_any_unknown: bool,
    /// Keys provably unpacked by some known-literal dict-with target.
    pub(super) dict_with_known_keys: HashSet<String>,
    /// The dict-with target variable names themselves.
    dict_vars: HashSet<String>,
    /// Names with a concrete (version > 0) statement/phi definition.
    explicitly_defined: HashSet<String>,
    /// Local-alias tails declared by a qualified `variable ns::tail`.
    alias_tails: FxHashSet<String>,
    /// Where a command substitution buried inside an `expr` argument writes
    /// a name (`set e [expr {[catch {…} tmp] || $tmp}]` writes `tmp` during
    /// expr evaluation): per name, each `(block, statement)` that writes it.
    /// The `[…]` is opaque to SSA def tracking, so a `$tmp` read in the same
    /// expression or after it looks read-before-set; a read before it is
    /// still one. Suppress-only.
    cmd_sub_writes: FxHashMap<String, Vec<(BlockId, usize)>>,
    /// Where code the module cannot see runs: each `(block, statement)` of a
    /// marker for it ([`crate::ssa::is_unseen_call_marker`]). A name nothing in
    /// the function assigns may be one that code set, so a read of it that
    /// follows a marker is no read before it is set. Suppress-only.
    unseen_call_sites: Vec<(BlockId, usize)>,
    /// Names written by a `Traits::SCRIPT_CONCATENATES_ARGS` call whose
    /// script the lowering left as an opaque barrier — `eval set l2 hello`
    /// really does set `l2` in the caller's own frame, but its words reach
    /// the IR as barrier arguments with no def attached, so a later
    /// `puts $l2` would look read-before-set.  Name-level, suppress-only.
    script_concat_writes: FxHashSet<String>,
    /// `(name, version)` pairs killed by an `unset` — undef at their reads,
    /// so a direct read of one is read-before-set just like a version-0
    /// origin.
    pub(in super::super) killed:
        FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub(in super::super) call_steps: CallStepMap,
    /// Phi versions that can be undefined on some executable path
    /// (a one-branch `set y 1` merge, or a try-handler merge). A statement
    /// read of one is read-before-set; the def-use pass can't express this
    /// because the read targets the *phi* version, not a version-0 origin.
    pub(in super::super) can_undef:
        FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub(in super::super) preserved_undef:
        FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    /// Loop-header phi versions whose *only* undef source is the loop's entry
    /// (zero-trip) edge — the loop body assigns the variable on every back
    /// edge, so the value is defined whenever the loop ran ≥1 time. Maps each
    /// such `(name, version)` to the loop's body-block name set.
    ///
    /// A read of the version reached *after* the loop (a block outside the
    /// set) is not read-before-set: matching C Tcl — which errors only when
    /// the iterator list / condition is actually empty at runtime, not merely
    /// when it *could* be — we assume a loop that may run does run. A read
    /// *inside* the loop body (a block in the set) still fires, because a
    /// first-iteration read before the body's assignment is a genuine error.
    /// A *provably* empty loop (`foreach x {}`, or a constant-false
    /// `while`/`for` SCCP already prunes) is excluded, so it keeps firing.
    pub(in super::super) loop_entry_only_undef:
        FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), FxHashSet<String>>,
}

impl UndefSuppression {
    /// True when a read of `name` is suppressed by an alias declaration or a
    /// `dict with` / `dict update` unpack.  Blanket variant: an unknown-shape
    /// dict suppresses every non-concrete name (the conservative
    /// "might-have-the-key" stance, used where no truth source can confirm
    /// the dict is empty — e.g. a `return` after a `dict with` on a param).
    pub(in super::super) fn suppresses(&self, name: &str) -> bool {
        self.suppresses_strict(name) || self.dict_with_blanket(name)
    }

    /// [`Self::suppresses`] for a read of the version `key`. A version in
    /// [`Self::preserved_undef`] was written by no substitution, so the
    /// condition-write suppression does not speak for it; every other one
    /// still does.
    pub(in super::super) fn suppresses_read(
        &self,
        name: &str,
        key: &(crate::var_resolve::VariableCellKey, crate::ssa::Version),
    ) -> bool {
        if self.preserved_undef.contains(key) {
            return self.suppresses_unsubstituted(name) || self.dict_with_blanket(name);
        }
        self.suppresses(name)
    }

    /// The unknown-shape `dict with` blanket of [`Self::suppresses`].
    fn dict_with_blanket(&self, name: &str) -> bool {
        self.has_dict_with && self.dict_with_any_unknown && !self.explicitly_defined.contains(name)
    }

    /// True when reading `key` at `block` is a safe *after-loop* read of a
    /// variable the loop body defines on every iteration (see
    /// [`Self::loop_entry_only_undef`]): the version is loop-entry-only-undef
    /// and `block` is outside the loop body. A read inside the loop body still
    /// fires (first-iteration undef is real).
    pub(in super::super) fn after_loop_defined(
        &self,
        key: &(crate::var_resolve::VariableCellKey, crate::ssa::Version),
        block: &str,
    ) -> bool {
        self.loop_entry_only_undef
            .get(key)
            .is_some_and(|body| !body.contains(block))
    }

    /// Like [`Self::suppresses`] but **without** the unknown-shape blanket —
    /// only alias tails, dict vars, and *provably-unpacked* keys suppress.
    /// Used on statement reads inside a `dict with` body, where an
    /// unknown-shape dict (e.g. an interprocedurally-empty literal
    /// SCCP cannot yet resolve) must still fire so a genuine missing-key read
    /// is not hidden.
    pub(in super::super) fn suppresses_strict(&self, name: &str) -> bool {
        self.suppresses_unsubstituted(name)
    }

    /// Whether an `expr` argument's command substitution writes `name` at
    /// or before the read at `index` of `block` — earlier in the block, or
    /// in a block that dominates it; `index` -1 is the block's terminator.
    pub(in super::super) fn written_by_substitution_before(
        &self,
        name: &str,
        ssa: &crate::ssa::SsaFunction,
        block: BlockId,
        index: i32,
    ) -> bool {
        self.cmd_sub_writes.get(name).is_some_and(|sites| {
            sites.iter().any(|&(site, at)| {
                if site == block {
                    usize::try_from(index).map_or(true, |index| at <= index)
                } else {
                    crate::loops::dominates(ssa, site, block)
                }
            })
        })
    }

    /// Whether code the module cannot see runs before the read at `index` of
    /// `block` — earlier in the block, or in a block that dominates it; `index`
    /// -1 is the block's terminator. A read in the statement a marker stands
    /// ahead of is after it, as the marker for a substitution's command stands
    /// ahead of its host.
    pub(in super::super) fn unseen_call_before(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: BlockId,
        index: i32,
    ) -> bool {
        self.unseen_call_sites.iter().any(|&(site, at)| {
            if site == block {
                usize::try_from(index).map_or(true, |index| at < index)
            } else {
                crate::loops::dominates(ssa, site, block)
            }
        })
    }

    /// The name-level suppressions, none of them a substitution's write.
    fn suppresses_unsubstituted(&self, name: &str) -> bool {
        if self.alias_tails.contains(name)
            || self.dict_vars.contains(name)
            || self.script_concat_writes.contains(name)
        {
            return true;
        }
        self.has_dict_with
            && !self.explicitly_defined.contains(name)
            && self.dict_with_known_keys.contains(name)
    }
}

/// `dict with` / `dict update` key-aware suppression: record the dict-var
/// names and, when the dict value is a same-block literal (or an
/// interprocedurally-propagated SCCP const), the variables the registry's
/// plan binds from it on entry — each key `dict with` finds, each `dict
/// update` variable whose key the dictionary holds.  A value that resolves
/// to neither marks the dict shape unknown.
pub(super) fn harvest_dict_with_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    s: &mut UndefSuppression,
    rules: tcl_syntax::word_rules::WordValueRules,
    registry: &tcl_registry::CommandRegistry,
) {
    for &bn in considered {
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for (idx, stmt) in block.statements.iter().enumerate() {
            let Some(tokens) = stmt.tokens() else {
                continue;
            };
            let Some(advice) =
                crate::registry_invocation::declaration_dictionary_scope_advice(registry, tokens)
            else {
                continue;
            };
            let args = &advice.arguments;
            s.has_dict_with = true;
            let Some(dict_var) = args
                .get(advice.plan.dictionary_argument)
                .and_then(Option::as_deref)
            else {
                s.dict_with_any_unknown = true;
                continue;
            };
            let dvar = crate::naming::split_array_name_braced(dict_var, true)
                .0
                .to_string();
            if dvar.is_empty() {
                s.dict_with_any_unknown = true;
                continue;
            }
            s.dict_vars.insert(dvar.clone());
            let literal = dictionary_suppression_literal(fu, bn, idx, &dvar, tokens, registry);
            match literal {
                Some(v) => {
                    let elems = crate::tcl_expr_eval::split_tcl_list(&v, rules);
                    match &advice.plan.bindings {
                        tcl_registry::dictionary_scope::DictionaryScopeBindings::Pairs(pairs) => {
                            let present: HashSet<&str> =
                                elems.iter().step_by(2).map(String::as_str).collect();
                            for &(key, variable) in pairs {
                                if args
                                    .get(key)
                                    .and_then(Option::as_deref)
                                    .is_some_and(|key| present.contains(key))
                                    && let Some(variable) =
                                        args.get(variable).and_then(Option::as_deref)
                                {
                                    let variable =
                                        crate::naming::split_array_name_braced(variable, true)
                                            .0
                                            .to_string();
                                    if !variable.is_empty() {
                                        s.dict_with_known_keys.insert(variable);
                                    }
                                }
                            }
                        }
                        tcl_registry::dictionary_scope::DictionaryScopeBindings::AllKeys(path)
                            if path.is_empty() =>
                        {
                            for (i, key) in elems.into_iter().enumerate() {
                                if i % 2 == 0 {
                                    s.dict_with_known_keys.insert(key);
                                }
                            }
                        }
                        tcl_registry::dictionary_scope::DictionaryScopeBindings::AllKeys(_) => {
                            s.dict_with_any_unknown = true;
                        }
                    }
                }
                None => s.dict_with_any_unknown = true,
            }
        }
    }
}

/// Resolve only the dictionary value read by this original invocation.
fn dictionary_suppression_literal(
    fu: &crate::compilation_unit::FunctionUnit,
    bn: BlockId,
    idx: usize,
    dvar: &str,
    tokens: &crate::ir::CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    use crate::ir::Statement;

    // Resolve the dict's value to harvest its keys.  Prefer the SCCP
    // CONST of the SPECIFIC version read by this dict-with (so
    // interprocedurally-propagated literals — a caller passing `{}` —
    // are honoured), falling back to a same-block literal `set`.  A
    // known value (even empty) harvests its keys; only a value that
    // resolves to neither marks the dict shape unknown.
    let read = fu.ssa.var_symbol_at(bn, idx, dvar).and_then(|symbol| {
        fu.ssa
            .blocks
            .get(&bn)?
            .statements
            .get(idx)?
            .uses
            .get(&symbol)
            .copied()
            .map(|version| (symbol, version))
    });
    if read.is_some_and(|(symbol, _)| fu.ssa.cell_key(symbol).is_none()) {
        return None;
    }
    let mut literal = read.and_then(|(symbol, version)| {
        match fu.diagnostic_value_facts().values().get(&(symbol, version)) {
            Some(crate::analyses::LatticeValue::Const(crate::analyses::ConstValue::String(
                value,
            ))) => Some(value.clone()),
            _ => None,
        }
    });
    if literal.is_none()
        && let Some((symbol, version)) = read
    {
        // A literal fallback must define this exact reaching SSA value.
        // Equal source spellings cannot donate another alias or lifetime.
        literal = fu.ssa.blocks.get(&bn).and_then(|data| {
            data.statements[..idx].iter().rev().find_map(|statement| {
                if statement.defs.get(&symbol) != Some(&version) {
                    return None;
                }
                match &statement.statement {
                    Statement::AssignConst { value, .. } => Some(value.clone()),
                    _ => None,
                }
            })
        });
    }
    if literal.is_none()
        && let Some(binding) = tokens.source_binding.as_ref()
        && let Some(site) = binding.invocation_site()
        && let Some(report) = binding.declaration_flow_report(registry)
    {
        literal = report
            .conditional_handler_literal(site, dvar)
            .map(str::to_owned);
    }
    // A represented constant cannot survive a missing original
    // dictionary read, even if its SSA slot still has the old value.
    if let Some((symbol, _)) = read
        && crate::ssa::SsaSourceView::at_statement(&fu.ssa, bn, idx)
            .source_tokens()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(|binding| binding.invocation_variable_reads.as_ref())
            .is_some_and(|reads| {
                reads.native_reads.iter().any(|access| {
                    fu.ssa.cell_key(symbol).is_some_and(|cell| {
                        crate::var_resolve::canonical_binding_value_key(&access.place).as_ref()
                            == Some(cell)
                    }) && access
                        .variable_context
                        .closed_contents_presence(&access.place)
                        != Some(crate::var_resolve::ContentsPresence::Defined)
                })
            })
    {
        literal = None;
    }
    literal
}

/// Release-, dialect-, and registry-shaped inputs used while reconstructing
/// read-before-set suppression from detached expression and list text.
#[derive(Clone, Copy)]
pub(in super::super) struct UndefSuppressionSemantics<'a> {
    pub dialect: Option<SurfaceQuery<'a>>,
    pub rules: tcl_syntax::word_rules::WordValueRules,
    pub lexer_config: tcl_lexer::LexerConfig,
    pub source: &'a str,
    pub analysis: &'a crate::analyser::AnalysisResult,
    pub context: &'a tcl_registry::model::ContextRegistry,
    /// The module's procedures, whose transfer summaries say what a call to
    /// one does to the places it names.
    pub module: Option<&'a crate::interprocedural::ModuleProcedures<'a>>,
}

fn collect_explicit_definitions(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    s: &mut UndefSuppression,
) {
    if s.has_dict_with {
        for &bn in considered {
            let Some(sb) = fu.ssa.blocks.get(&bn) else {
                continue;
            };
            for st in &sb.statements {
                for (&n, v) in &st.defs {
                    if *v > 0 {
                        s.explicitly_defined.insert(fu.ssa.var_name(n).to_owned());
                    }
                }
            }
            for phi in &sb.phis {
                if phi.version > 0 {
                    s.explicitly_defined
                        .insert(fu.ssa.var_name(phi.name).to_owned());
                }
            }
        }
    }
}

pub(in super::super) fn build_undef_suppression(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    initial_global: bool,
    global_aliases: &HashSet<String>,
    semantics: UndefSuppressionSemantics<'_>,
) -> UndefSuppression {
    let UndefSuppressionSemantics {
        dialect,
        rules,
        lexer_config,
        module,
        ..
    } = semantics;
    let registry = semantics.context.commands();
    let commands = registry;
    let call_steps = call_steps(fu, considered, module, commands);
    let UndefIndexMaps {
        phi_def,
        phi_block,
        killed,
        may_defs,
    } = build_phi_undef_index(fu, considered, registry, &call_steps);
    let exists_guards = collect_existence_guards(fu, registry, lexer_config);
    // Phi versions that can reach an undef origin on some executable path —
    // a statement read of one is read-before-set. The per-use existence
    // guard + suppression set still apply in the emitter loop.
    let undef_ctx = PhiUndefCtx {
        registry: commands,
        phi_def: &phi_def,
        phi_block: &phi_block,
        killed: &killed,
        may_defs: &may_defs,
        considered,
        executable_edges: fu.diagnostic_value_facts().executable_edges(),
        exists_guards: &exists_guards,
        initial_global,
        global_aliases,
        dialect,
        ssa: &fu.ssa,
        preserved: &fu.sccp.preserved,
    };
    let mut can_undef: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)> =
        FxHashSet::default();
    // One memo for the whole sweep and the loop-entry fixpoint below: both run
    // against `undef_ctx` unchanged, so an answer found for one key stands for
    // every other query (issue #2021 — without it the sweep re-walks every
    // path through the phi graph).
    let mut memo = PhiUndefMemo::default();
    for key in phi_def.keys().chain(may_defs.keys()) {
        if phi_can_undef(&key.0, key.1, &undef_ctx, &mut memo) {
            can_undef.insert(key.clone());
        }
    }
    // A definition its statement left untouched — a `regexp` that did not
    // match, a `scan` whose input ran out, any declared `Preserve` — holds
    // its prior version, so it is undefined exactly when that version can
    // be: a read of it is then a read before set.
    let mut preserved_undef: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)> =
        FxHashSet::default();
    for &(symbol, version) in fu.sccp.preserved.keys() {
        let Some(name) = fu.ssa.cell_key(symbol) else {
            continue;
        };
        if phi_can_undef(name, version, &undef_ctx, &mut memo) {
            can_undef.insert((name.to_owned(), version));
            preserved_undef.insert((name.to_owned(), version));
        }
    }
    for (block, markers) in &fu.ssa.value_clobbers {
        if !considered.contains(block) {
            continue;
        }
        for versions in markers.values() {
            for (&symbol, &(_, fresh)) in versions {
                let Some(name) = fu.ssa.cell_key(symbol) else {
                    continue;
                };
                if phi_can_undef(name, fresh, &undef_ctx, &mut memo) {
                    can_undef.insert((name.clone(), fresh));
                }
            }
        }
    }
    let loop_entry_only_undef =
        build_loop_entry_only_undef(fu, &can_undef, &undef_ctx, rules, &mut memo);
    let mut s = UndefSuppression {
        cmd_sub_writes: collect_expr_cmd_sub_writes(fu, considered, semantics),
        script_concat_writes: collect_script_concat_writes(fu, considered, semantics),
        unseen_call_sites: collect_unseen_call_sites(fu, considered, initial_global, semantics),
        killed,
        call_steps,
        can_undef,
        preserved_undef,
        loop_entry_only_undef,
        ..Default::default()
    };
    harvest_dict_with_suppression(fu, considered, &mut s, rules, registry);

    // Names with a concrete (version > 0) statement or phi definition — a
    // dict-with scope never suppresses these (they are genuinely set).
    collect_explicit_definitions(fu, considered, &mut s);

    s.alias_tails = collect_qualified_variable_alias_tails(fu, considered, semantics.context);
    s
}

/// Build the [`UndefSuppression::loop_entry_only_undef`] map: loop-header phi
/// versions whose sole undef origin is the loop's zero-trip entry edge.
///
/// For each natural loop (built over the SCCP-executable subgraph, so a
/// provably-dead loop body never forms a loop) whose header carries a phi in
/// `can_undef`, the phi qualifies when *every* back-edge (in-loop
/// predecessor) operand is itself defined — i.e. the loop body assigns the
/// variable on each iteration and the only way the phi is undef is by skipping
/// the loop entirely. A provably-empty `foreach` (all iterator lists are
/// empty literals) is excluded: its body never runs, so tclsh always errors,
/// and the read must keep firing.
fn build_loop_entry_only_undef(
    fu: &crate::compilation_unit::FunctionUnit,
    can_undef: &FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    ctx: &PhiUndefCtx<'_>,
    rules: tcl_syntax::word_rules::WordValueRules,
    memo: &mut PhiUndefMemo,
) -> FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), FxHashSet<String>> {
    let mut out: FxHashMap<
        (crate::var_resolve::VariableCellKey, crate::ssa::Version),
        FxHashSet<String>,
    > = FxHashMap::default();
    if can_undef.is_empty() {
        return out;
    }
    let forest = crate::loops::build_loop_forest(&fu.cfg, &fu.ssa, ctx.considered);
    // Fixpoint over the loop forest so *nested* accumulators converge: an inner
    // loop whose body defines the variable is itself loop-entry-only-undef, so
    // for the enclosing loop its exit operand counts as defined (we assume both
    // loops run). Innermost loops are marked first; a pass that marks nothing
    // new terminates. Bounded by the forest size (each pass marks ≥1 phi or
    // stops), so at most `loops.len()` passes.
    loop {
        let mut changed = false;
        for natural in &forest.loops {
            let Some(header_id) = fu.cfg.block_id(&natural.header) else {
                continue;
            };
            // A provably-empty `foreach` runs zero times: its body-assigned
            // variables are never set, so tclsh always errors — keep firing.
            if foreach_header_provably_empty(fu, header_id, rules) {
                continue;
            }
            let body_blocks: FxHashSet<String> = natural.blocks.iter().cloned().collect();
            let Some(ssa_header) = fu.ssa.blocks.get(&header_id) else {
                continue;
            };
            for phi in &ssa_header.phis {
                let Some(name) = fu.ssa.cell_key(phi.name).cloned() else {
                    continue;
                };
                let key = (name.clone(), phi.version);
                if out.contains_key(&key) || !can_undef.contains(&key) {
                    continue;
                }
                // The phi is "entry-only undef" when no *back-edge* operand can
                // be undef: a body predecessor that still merges an unset value
                // is a conditional-def-in-body case (or a break/continue before
                // the set) that tclsh can still leave unset — those keep firing.
                // An operand already proven loop-entry-only-undef (a nested
                // loop's result) counts as defined here.
                let entry_only = phi.incoming.iter().all(|(&pred, &ver_in)| {
                    // Only in-loop (back-edge) predecessors gate the verdict;
                    // the pre-header entry edge carries the zero-trip undef we
                    // assume away. Non-executable predecessors never read.
                    if !ctx.considered.contains(&pred) {
                        return true;
                    }
                    if !body_blocks.contains(fu.cfg.block_name(pred)) {
                        return true;
                    }
                    let binding = fu.ssa.binding_version(phi.name, ver_in);
                    if out.contains_key(&(name.clone(), binding)) {
                        return true;
                    }
                    !phi_can_undef(&name, ver_in, ctx, memo)
                });
                if entry_only {
                    out.insert(key, body_blocks.clone());
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    // A call to code the module cannot see changes value facts without
    // introducing a new binding.
    // Carry the established after-loop binding proof through their fresh value
    // versions, just as `phi_can_undef` follows that same binding lineage.
    for (block, markers) in &fu.ssa.value_clobbers {
        if !ctx.considered.contains(block) {
            continue;
        }
        for versions in markers.values() {
            for (&symbol, &(_, fresh)) in versions {
                let binding = fu.ssa.binding_version(symbol, fresh);
                let Some(name) = fu.ssa.cell_key(symbol).cloned() else {
                    continue;
                };
                if let Some(body) = out.get(&(name.clone(), binding)).cloned() {
                    out.insert((name, fresh), body);
                }
            }
        }
    }
    out
}

/// True when `header_id` is the header of a `foreach` / `lmap` / `dict for`
/// whose iterator lists are *all* statically-empty literals (so the loop body
/// provably never runs). The synthetic iterator-binding node placed at the
/// loop header records each iterator list's text in `args`; an argument splits
/// to zero elements only for an empty literal (a `$`/`[` substitution splits
/// to a single opaque element, a non-empty literal to ≥1 element).
fn foreach_header_provably_empty(
    fu: &crate::compilation_unit::FunctionUnit,
    header_id: BlockId,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> bool {
    use crate::ir::Statement;
    let Some(block) = fu.cfg.blocks.get(&header_id) else {
        return false;
    };
    block.statements.iter().any(|stmt| {
        matches!(
            stmt,
            Statement::Call { foreach_groups: Some(_), args, .. }
                if !args.is_empty()
                    && args
                        .iter()
                        .all(|a| crate::tcl_expr_eval::split_tcl_list(a, rules).is_empty())
        )
    })
}

/// Local-alias tail names in the retained source declaration for a
/// qualified namespace-variable operand. The actual context admits the
/// command metadata; this name-level suppression establishes no executed
/// namespace link, cell identity, frame or stored contents.
pub(super) fn collect_qualified_variable_alias_tails(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    context: &tcl_registry::model::ContextRegistry,
) -> FxHashSet<String> {
    let mut tails = FxHashSet::default();
    for statement in considered
        .iter()
        .filter_map(|id| fu.cfg.blocks.get(id))
        .flat_map(|block| &block.statements)
    {
        let Some(invocation) = crate::registry_invocation::resolved_statement_invocation_in_context(
            context, statement,
        ) else {
            continue;
        };
        let Some(transitions) = invocation.facts.state_transitions.declared() else {
            continue;
        };
        for fact in transitions.facts() {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            let tcl_registry::VariableAliasTarget::CurrentNamespace { variable } = &alias.target
            else {
                continue;
            };
            if variable
                .literal()
                .is_some_and(|name| tcl_syntax::naming::is_qualified(name.as_bytes()))
                && let Some(local) = alias.local.literal()
            {
                tails.insert(local.to_owned());
            }
        }
    }
    tails
}
