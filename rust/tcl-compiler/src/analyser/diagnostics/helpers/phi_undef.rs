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

//! The phi-from-undef trace: which SSA versions can be undefined, with the startup bindings and call steps it reads.

use super::existence_guards::{ExistenceGuard, block_dominated_by};
use crate::cfg::BlockId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;
use tcl_dialect::model::SurfaceQuery;

/// Read-only context threaded unchanged through [`phi_can_undef`]'s recursion:
/// the phi indices, the `unset`-killed set, the executable block / edge sets,
/// the dominating existence guards, the registry-owned startup binding, and
/// the SSA function itself.
pub(in super::super) struct PhiUndefCtx<'a> {
    /// The registry whose special-variable faces answer the startup facts,
    /// pack rows included.
    pub registry: &'a tcl_registry::CommandRegistry,
    pub phi_def: &'a PhiDefMap,
    pub phi_block: &'a PhiBlockMap,
    pub killed: &'a FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub considered: &'a HashSet<BlockId>,
    pub executable_edges: &'a HashSet<(BlockId, BlockId)>,
    pub exists_guards: &'a [ExistenceGuard],
    pub may_defs: &'a MayDefMap,
    /// Startup bindings exist only in the document's initial global frame.
    pub initial_global: bool,
    /// Locals that registry metadata says alias the interpreter's global
    /// namespace in this function (`global name`).
    pub global_aliases: &'a HashSet<String>,
    pub dialect: Option<SurfaceQuery<'a>>,
    pub ssa: &'a crate::ssa::SsaFunction,
    /// The definitions a route's outcome preserved, each with the version
    /// it read ([`crate::sccp::SccpResult::preserved`]).
    pub preserved: &'a std::collections::HashMap<crate::ssa::ValueKey, crate::ssa::Version>,
}

/// The version whose binding a read of `version` reads: through the
/// definitions a route preserved ([`through_preserved`]) and the fresh
/// versions a call to code the module cannot see gave the names live after
/// it ([`crate::ssa::SsaFunction::binding_version`]), followed until neither
/// moves it. Each step reads an earlier version, so the walk ends.
fn binding_origin(
    ssa: &crate::ssa::SsaFunction,
    preserved: &std::collections::HashMap<crate::ssa::ValueKey, crate::ssa::Version>,
    symbol: crate::ssa::Symbol,
    mut version: crate::ssa::Version,
) -> crate::ssa::Version {
    loop {
        let origin = ssa.binding_version(symbol, through_preserved(preserved, symbol, version));
        if origin == version {
            return version;
        }
        version = origin;
    }
}

/// The version a read of `version` reads through the definitions a route
/// preserved: a preserved definition is its prior version's value and
/// existence, followed until a version no outcome preserved.
fn through_preserved(
    preserved: &std::collections::HashMap<crate::ssa::ValueKey, crate::ssa::Version>,
    symbol: crate::ssa::Symbol,
    mut version: crate::ssa::Version,
) -> crate::ssa::Version {
    // A preserved definition reads an earlier version, so the chain ends;
    // the bound keeps a malformed map from looping.
    for _ in 0..=preserved.len() {
        match preserved.get(&(symbol, version)) {
            Some(&prior) if prior != version => version = prior,
            _ => break,
        }
    }
    version
}

/// Registry spelling of an authored startup key. Named namespaces retain their
/// qualification; Native cells use their independently retained namespace owner.
pub(in super::super) fn startup_var_name(name: &str) -> &str {
    // naming.diagnostics.original-resolved-variable-name-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-resolved-variable-name-advice.md
    let normalised = crate::naming::split_array_name_braced(name, true).0;
    tcl_syntax::naming::unroot_rooted_key(normalised).unwrap_or(normalised)
}

/// Whether `name` resolves to the interpreter's global binding in this
/// analysis frame.  The `global`-alias set comes from a registry trait, so a
/// new dialect spelling can opt in without a command-name branch here.
pub(in super::super) fn has_global_startup_binding(
    name: &str,
    initial_global: bool,
    global_aliases: &HashSet<String>,
) -> bool {
    if initial_global
        || tcl_syntax::naming::unroot_rooted_key(
            crate::naming::split_array_name_braced(name, true).0,
        )
        .is_some()
    {
        return true;
    }
    let startup_name = startup_var_name(name);
    global_aliases
        .iter()
        .any(|alias| startup_var_name(alias) == startup_name)
}

/// Registry startup spelling only when the exact cell belongs to the native
/// root namespace. Authored cells retain their separate declaration-only scope
/// contract; a same-spelled named namespace never acquires root startup facts.
pub(in super::super) fn startup_cell_binding<'a>(
    cell: &'a crate::var_resolve::VariableCellKey,
    initial_global: bool,
    global_aliases: &HashSet<String>,
) -> (&'a str, bool) {
    use crate::var_resolve::VariableCellKey;
    match cell {
        VariableCellKey::Authored(name) => (
            startup_var_name(name),
            has_global_startup_binding(name, initial_global, global_aliases),
        ),
        VariableCellKey::Namespace { identity, simple } => {
            let Ok(display) = simple.try_utf8() else {
                return ("", false);
            };
            (
                display,
                match identity {
                    crate::command_binding::SourceNamespaceKey::Authored(namespace) => {
                        namespace == "::"
                    }
                    _ => identity
                        .exact_native_path()
                        .is_some_and(tcl_core_types::ByteNamespacePath::is_root),
                },
            )
        }
        VariableCellKey::Lifetime { cell, .. } | VariableCellKey::Element { cell, .. } => {
            startup_cell_binding(cell, initial_global, global_aliases)
        }
        _ => ("", false),
    }
}

/// Startup facts for one variable name, as [`phi_can_undef`] needs them.
///
/// They depend only on `name` (and the registry-owned dialect data), so they
/// are computed once per name instead of once per phi operand examined.
#[derive(Clone, Copy)]
pub(super) struct StartupFacts {
    /// The default Tcl host binds this name before user code runs.
    readable_at_startup: bool,
    /// A read after `unset` materialises the value again — registry data
    /// confines this to `tcl_precision` on Tcl 8.x.
    rematerialises_after_unset: bool,
}

impl StartupFacts {
    fn for_name(name: &crate::var_resolve::VariableCellKey, ctx: &PhiUndefCtx<'_>) -> Self {
        let (startup_name, global_binding) =
            startup_cell_binding(name, ctx.initial_global, ctx.global_aliases);
        Self {
            readable_at_startup: global_binding
                && ctx
                    .registry
                    .is_readable_at_startup(startup_name, ctx.dialect),
            rematerialises_after_unset: global_binding
                && ctx.registry.is_lazily_readable(startup_name, ctx.dialect),
        }
    }
}

/// A phi version, as the undef trace keys them: the variable's interned SSA
/// [`Symbol`](crate::ssa::Symbol) rather than its name, so the index and its
/// worklist hash and copy a pair of `u32`s.
pub(super) type VersionKey = (crate::ssa::Symbol, crate::ssa::Version);

/// Undefinition answers shared by every query in one [`PhiUndefCtx`].
/// A worklist over phi operands computes all versions without enumerating paths.
#[derive(Default)]
pub(in super::super) struct PhiUndefMemo {
    pub(super) index: Option<PhiUndefIndex>,
}

impl PhiUndefMemo {
    /// The index, built on first use. `ctx` must be the same context on every
    /// call — a memo is per-context state, not a cache across contexts.
    pub(super) fn index(&mut self, ctx: &PhiUndefCtx<'_>) -> &PhiUndefIndex {
        self.index.get_or_insert_with(|| PhiUndefIndex::build(ctx))
    }
}

/// Every version that can be undefined, plus the per-name facts the leaf
/// answers need.
pub(super) struct PhiUndefIndex {
    /// Phi versions reaching an undef origin on some executable path.
    undef: FxHashSet<VersionKey>,
    /// `unset`-killed versions and the answer each gives.
    pub(super) killed: FxHashMap<VersionKey, bool>,
    /// Startup facts per variable, for the version-0 answer.
    pub(super) facts: FxHashMap<crate::ssa::Symbol, StartupFacts>,
}

impl PhiUndefIndex {
    /// Mark every phi version that can be undefined.
    ///
    /// A phi is undef when any of its reachable, non-existence-guarded
    /// incomings is undef, an incoming being undef when it is the version-0
    /// origin (and the host does not bind the name at startup), an
    /// `unset`-killed version (that a read does not materialise again), or
    /// itself an undef phi. Nothing else in that rule depends on how a version
    /// was reached, so it is plain reachability over the phi-operand graph:
    /// this walks the *reverse* graph from the undef origins, which answers
    /// every version in one pass — the same answers a per-query forward DFS
    /// gives, without re-deriving them once per path.
    ///
    /// Cycles need no special case here, and that keeps the old walk's
    /// "a back-edge is not undef" rule exactly. That walk's cut only stopped
    /// it re-entering a version already open on the *current* path, never
    /// stopping it reaching one by another route, and a query started with an
    /// empty path — so it still explored every version reachable from the
    /// query, and answered `true` for exactly the queries that reach an undef
    /// origin. A loop-header phi is undef only when an origin genuinely
    /// reaches it: going round the loop offers nothing the entry edge did not,
    /// so a phi whose only route to an origin is through itself stays
    /// unmarked, just as the cut answered "not undef" on the back-edge.
    fn build(ctx: &PhiUndefCtx<'_>) -> Self {
        let mut facts: FxHashMap<crate::ssa::Symbol, StartupFacts> = FxHashMap::default();
        let mut killed: FxHashMap<VersionKey, bool> = FxHashMap::default();
        for (name, version) in ctx.killed {
            let Some(symbol) = ctx.ssa.cell_symbol(name) else {
                continue;
            };
            let name_facts = *facts
                .entry(symbol)
                .or_insert_with(|| StartupFacts::for_name(name, ctx));
            // A Tcl read trace is not an eager startup fact: `unset` removes
            // the current value, but a later read materialises it again.
            // Eager bindings such as argv remain genuine W210 reads after
            // `unset`.
            killed.insert((symbol, *version), !name_facts.rematerialises_after_unset);
        }

        let mut walk = UndefWalk::default();
        for (key, phi) in ctx.phi_def {
            let (name, version) = (&key.0, key.1);
            let symbol = phi.name;
            let node = (symbol, version);
            let name_facts = *facts
                .entry(symbol)
                .or_insert_with(|| StartupFacts::for_name(name, ctx));
            if killed.contains_key(&node) {
                // Killed wins over the phi: the version is decided by the kill,
                // and its incomings never come into it.
                continue;
            }
            // The block this phi lives in — the destination of each incoming
            // edge.
            let this_block = ctx.phi_block.get(key).copied();
            for (&pred, &incoming) in &phi.incoming {
                if !ctx.considered.contains(&pred) {
                    continue;
                }
                // A phi has one operand per predecessor *edge*; an operand
                // arriving on a non-executable edge (SCCP proved the edge dead
                // — e.g. the `cond → exit` edge of `while 1`, which a `break`
                // makes the loop's only real exit) can never actually be read,
                // so its version-0 origin must not count as a possible undef.
                // This filter is only applied when SCCP edge info is available
                // (a non-empty set).
                if let Some(block) = this_block
                    && !ctx.executable_edges.is_empty()
                    && !ctx.executable_edges.contains(&(pred, block))
                {
                    continue;
                }
                let operand = (symbol, incoming);
                // A later deletion supersedes the condition's existence fact.
                if !killed.contains_key(&operand)
                    && ctx
                        .exists_guards
                        .iter()
                        .any(|(gv, gblk)| gv == name && block_dominated_by(ctx.ssa, pred, *gblk))
                {
                    continue;
                }
                // A definition a route preserved, and the fresh version a call
                // to code the module cannot see leaves, read the binding of an
                // earlier version.
                let incoming = binding_origin(ctx.ssa, ctx.preserved, symbol, incoming);
                walk.take(
                    node,
                    (symbol, incoming),
                    &killed,
                    name_facts.readable_at_startup,
                );
            }
        }
        // A name an opaque `switch`'s arm may write is a phi with one
        // operand: the version the statement read, which it holds when no arm
        // runs.
        for (key, &(block, prior)) in ctx.may_defs {
            let Some(symbol) = ctx.ssa.cell_symbol(&key.0) else {
                continue;
            };
            let node = (symbol, key.1);
            if killed.contains_key(&node)
                || ctx.exists_guards.iter().any(|(cell, guard)| {
                    cell == &key.0 && block_dominated_by(ctx.ssa, block, *guard)
                })
            {
                continue;
            }
            let name_facts = *facts
                .entry(symbol)
                .or_insert_with(|| StartupFacts::for_name(&key.0, ctx));
            let prior = through_preserved(ctx.preserved, symbol, prior);
            walk.take(
                node,
                (symbol, prior),
                &killed,
                name_facts.readable_at_startup,
            );
        }
        let undef = walk.finish();
        Self {
            undef,
            killed,
            facts,
        }
    }
}

thread_local! {
    static TRACE_STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn trace_step() {
    TRACE_STEPS.with(|steps| steps.set(steps.get().saturating_add(1)));
}

/// Work units the phi-from-undef walk has taken on this thread: each operand
/// edge examined and each undef version propagated. The walk is a reachability
/// over the phi graph, so this grows with the graph, not with its paths
/// (#2021); a scaling check reads it before and after an analysis.
#[must_use]
pub fn phi_undef_trace_steps() -> u64 {
    TRACE_STEPS.with(std::cell::Cell::get)
}

/// The reachability walk behind [`PhiUndefIndex::build`]: which versions can
/// reach an undef origin, over the operand edges of the phis and of the
/// opaque `switch` may-definitions.
#[derive(Default)]
struct UndefWalk {
    undef: FxHashSet<VersionKey>,
    worklist: Vec<VersionKey>,
    /// Reverse operand edges: an undef version makes every version that takes
    /// it as an operand undef too.
    users: FxHashMap<VersionKey, Vec<crate::ssa::Version>>,
}

impl UndefWalk {
    /// `node` takes `operand`: undef when the operand is an undef origin or
    /// already known undef, and recorded as a user of it otherwise.
    fn take(
        &mut self,
        node: VersionKey,
        operand: VersionKey,
        killed: &FxHashMap<VersionKey, bool>,
        readable_at_startup: bool,
    ) {
        trace_step();
        let origin = if let Some(&answer) = killed.get(&operand) {
            answer
        } else if operand.1 == 0 {
            // A version-zero incoming normally is the undef origin. The
            // default Tcl host, however, binds a registry-declared subset
            // before user code, and a conditional write would otherwise make
            // a merge with the startup version look undefined.
            // Procedure-local frames never set `initial_global`.
            !readable_at_startup
        } else if self.undef.contains(&operand) {
            true
        } else {
            // Another phi (or a concrete definition, which is never undef
            // and so never enters `undef`).
            self.users.entry(operand).or_default().push(node.1);
            return;
        };
        if origin && self.undef.insert(node) {
            self.worklist.push(node);
        }
    }

    /// Follow every undef version to the versions that take it.
    fn finish(mut self) -> FxHashSet<VersionKey> {
        while let Some(node) = self.worklist.pop() {
            let Some(users) = self.users.get(&node) else {
                continue;
            };
            for &user in users {
                trace_step();
                let up = (node.0, user);
                if self.undef.insert(up) {
                    self.worklist.push(up);
                }
            }
        }
        self.undef
    }
}

/// Phi-from-undef trace.  A use's SSA version > 0 normally proves a prior
/// definition reached it, but a phi result whose reachable incomings
/// include an undefined (version-0) or `unset`-killed origin only reaches
/// on a subset of paths — the others read an unset variable.  Returns
/// true when `(name, version)` can be undefined on some reachable path.
///
/// Version 0 is
/// the undef origin; an `unset`-killed version is undef; a non-phi
/// (concrete) definition is never undef; a phi is undef if any of its
/// reachable, non-existence-guarded incomings is undef.  Cycles
/// (loop-header phis) conservatively resolve to *not* undef on the cycle.
///
/// `memo` holds the [`PhiUndefIndex`] that answers this, built on the first
/// query and reused by every later one; it must only be shared between
/// queries whose [`PhiUndefCtx`] is the same.
pub(in super::super) fn phi_can_undef(
    name: &crate::var_resolve::VariableCellKey,
    version: crate::ssa::Version,
    ctx: &PhiUndefCtx<'_>,
    memo: &mut PhiUndefMemo,
) -> bool {
    let Some(symbol) = ctx.ssa.cell_symbol(name) else {
        // No SSA symbol means the function never defines the name, so it has
        // neither a phi nor a kill: only the startup answer can apply.
        return version == 0 && !StartupFacts::for_name(name, ctx).readable_at_startup;
    };
    let version = binding_origin(ctx.ssa, ctx.preserved, symbol, version);
    let index = memo.index(ctx);
    if let Some(&answer) = index.killed.get(&(symbol, version)) {
        return answer;
    }
    if version == 0 {
        return !index
            .facts
            .get(&symbol)
            .copied()
            .unwrap_or_else(|| StartupFacts::for_name(name, ctx))
            .readable_at_startup;
    }
    index.undef.contains(&(symbol, version))
}

/// `(name, version) → Phi` index used by [`phi_can_undef`].
pub(in super::super) type PhiDefMap =
    FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), crate::ssa::Phi>;

/// `(name, version) → defining block` index, so [`phi_can_undef`] can test
/// each incoming `(pred, phi_block)` edge against the SCCP-executable edge set.
pub(in super::super) type PhiBlockMap =
    FxHashMap<(crate::var_resolve::VariableCellKey, crate::ssa::Version), BlockId>;

/// The versions an opaque `switch` may define — a name one of its arms
/// writes — each with the version the statement read and the block it sits
/// in. Such a version holds its prior one when no arm runs, so it is
/// undefined exactly when that one can be: a phi with one operand.
pub(in super::super) type MayDefMap = FxHashMap<
    (crate::var_resolve::VariableCellKey, crate::ssa::Version),
    (BlockId, crate::ssa::Version),
>;

/// The step each definition of a call to a procedure of the module takes
/// from the callee's transfer summary, by `(name, version)`, with the block
/// the call sits in and the version it found
/// ([`crate::value_transfer::summary_steps`]).
pub(in super::super) type CallStepMap = FxHashMap<
    (crate::var_resolve::VariableCellKey, crate::ssa::Version),
    (
        crate::value_transfer::ExistenceStep,
        BlockId,
        crate::ssa::Version,
    ),
>;

/// The [`CallStepMap`] of `fu`'s calls in `considered` blocks: empty where
/// no module's procedures are in hand.
pub(in super::super) fn call_steps(
    fu: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    module: Option<&crate::interprocedural::ModuleProcedures<'_>>,
    registry: &tcl_registry::CommandRegistry,
) -> CallStepMap {
    let mut steps = CallStepMap::default();
    let Some(module) = module else {
        return steps;
    };
    if module
        .invocation_metadata_context_for_function(fu, registry)
        .is_none()
    {
        return steps;
    }
    let config = fu.source_lexer_config();
    for &bn in considered {
        let Some(block) = fu.ssa.blocks.get(&bn) else {
            continue;
        };
        for (index, statement) in block.statements.iter().enumerate() {
            let Some(tokens) = statement.statement.tokens() else {
                continue;
            };
            let Some(metadata) =
                module.original_invocation_metadata_for_function(fu, tokens, registry)
            else {
                continue;
            };
            if tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_invocation_source_purpose(tokens, metadata, registry)
            }) != Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
            {
                continue;
            }
            for (place, step) in crate::value_transfer::summary_steps(
                module,
                &fu.name,
                &statement.statement,
                &config,
            ) {
                let Some(symbol) = fu.ssa.var_symbol(&place) else {
                    continue;
                };
                if let Some(&version) = statement.defs.get(&symbol)
                    && let Some(cell) = fu.ssa.cell_key(symbol)
                {
                    let prior = crate::sccp::prior_version(block, index, symbol);
                    steps.insert((cell.clone(), version), (step, bn, prior));
                }
            }
        }
    }
    steps
}

/// The indices [`phi_can_undef`] answers from: phi operands, the block each
/// phi sits in, the `unset`-killed versions, and the may-definitions of the
/// opaque `switch` statements.
pub(in super::super) struct UndefIndexMaps {
    pub phi_def: PhiDefMap,
    pub phi_block: PhiBlockMap,
    pub killed: FxHashSet<(crate::var_resolve::VariableCellKey, crate::ssa::Version)>,
    pub may_defs: MayDefMap,
}

/// Build the `(name, version) → Phi` index, the `(name, version) → block`
/// index, and the set of `unset`-killed versions for [`phi_can_undef`],
/// restricted to `considered` (executable) blocks.
pub(in super::super) fn build_phi_undef_index(
    function: &crate::compilation_unit::FunctionUnit,
    considered: &HashSet<BlockId>,
    registry: &tcl_registry::CommandRegistry,
    steps: &CallStepMap,
) -> UndefIndexMaps {
    use tcl_registry::value_transfer::{BindingKind, Existence};
    let ssa = &function.ssa;
    let mut phi_def: PhiDefMap = FxHashMap::default();
    let mut phi_block: PhiBlockMap = FxHashMap::default();
    let mut killed = FxHashSet::default();
    let mut may_defs = MayDefMap::default();
    for &bn in considered {
        let Some(sblock) = ssa.blocks.get(&bn) else {
            continue;
        };
        for phi in &sblock.phis {
            let Some(phi_name) = ssa.cell_key(phi.name).cloned() else {
                continue;
            };
            phi_def.insert((phi_name.clone(), phi.version), phi.clone());
            phi_block.insert((phi_name, phi.version), bn);
        }
        for statement in &sblock.statements {
            if crate::ssa::has_arm_may_defs(&statement.statement) {
                let refreshed =
                    crate::ssa::refreshed_bases(&statement.statement, function, registry);
                for symbol in &statement.may_defs {
                    if refreshed.iter().any(|base| base == ssa.var_name(*symbol)) {
                        continue;
                    }
                    if let (Some(&version), Some(&prior)) =
                        (statement.defs.get(symbol), statement.uses.get(symbol))
                        && let Some(cell) = ssa.cell_key(*symbol)
                    {
                        may_defs.insert((cell.clone(), version), (bn, prior));
                    }
                }
            }
            for (&symbol, &version) in &statement.defs {
                if statement.destruction_defs.contains(&symbol)
                    && let Some(cell) = ssa.cell_key(symbol)
                {
                    killed.insert((cell.clone(), version));
                }
            }
        }
    }
    // A call to a procedure of the module is the assignment its summary
    // states for each place it names: a step that leaves the place unset
    // whatever it held kills it, one that sets it is a definition, and any
    // other — a may-bind, a preserve, or a may-unset, which a summary also
    // states out of its own caution — leaves the place unset where it was
    // before the call, so it reads the version before the call.
    for (key, &(step, block, prior)) in steps {
        let after_bound = step.apply(Existence::Bound(BindingKind::Either));
        let after_unbound = step.apply(Existence::Unbound);
        if after_bound == Existence::Unbound && after_unbound == Existence::Unbound {
            killed.insert(key.clone());
        } else if matches!(after_unbound, Existence::Unbound | Existence::MayBound) {
            may_defs.insert(key.clone(), (block, prior));
        }
    }
    for (block, markers) in &ssa.value_clobbers {
        if !considered.contains(block) {
            continue;
        }
        for versions in markers.values() {
            for (&symbol, &(_, fresh)) in versions {
                let origin = ssa.binding_version(symbol, fresh);
                let Some(name) = ssa.cell_key(symbol).cloned() else {
                    continue;
                };
                if killed.contains(&(name.clone(), origin)) {
                    killed.insert((name, fresh));
                }
            }
        }
    }
    UndefIndexMaps {
        phi_def,
        phi_block,
        killed,
        may_defs,
    }
}
