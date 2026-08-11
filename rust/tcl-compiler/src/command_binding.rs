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

//! Flow-sensitive command-binding lattice.
//!
//! Tracks what each command
//! *name* resolves to at every program point — its original builtin, a
//! user proc, an `interp alias`, or an opaque (renamed / deleted /
//! never-defined) target — modelling `rename`, proc (re)definition, and
//! `interp alias` flow-sensitively over the CFG.  Two *different*
//! concrete bindings join to [`BindingKind::Unknown`] (⊤); a *dynamic*
//! mutation (`rename $old …`, `proc $x …`) collapses the whole state to
//! a wildcard ⊤ from that point on.
//!
//! Consumers: the W128 diagnostic ("call to a command renamed/deleted
//! earlier in this file") in `analyser`, and — via the flow-insensitive
//! whole-module summary [`scan_module_command_mutations`] — the
//! optimiser's builtin-fold trust gate.
//!
//! The Rust CFG has no explicit exception edges (catch is lowered
//! opaquely), so predecessors come from terminator successors only.
//! That can only *miss* a rebinding reaching a handler, never invent
//! one — sound for a warning.

use std::collections::{HashMap, HashSet};

use crate::cfg::BlockId;
use crate::cfg::Function as CfgFunction;
use crate::ir::Statement;
use crate::naming::is_dynamic_word;
use crate::naming::normalise_qualified_name as nqn;
use crate::var_escape::helpers::invocation_facts;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, CommandRegistry, CommandTableEffect,
    StateTransition, StateTransitionDomain, TransitionSubject,
};

/// The lattice element a command name resolves to.
///
/// Height-3 join lattice: [`BindingKind::Bottom`] (⊥) is the identity,
/// a concrete binding joined with itself is unchanged, and two
/// *different* bindings rise to [`BindingKind::Unknown`] (⊤).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// ⊥ — identity for join (no contribution yet).
    Bottom,
    /// The original core/registry command, unperturbed.
    Builtin,
    /// A user procedure (`target` = its canonical qname).
    Proc,
    /// A concrete registry-described command whose narrower identity is not
    /// relevant to this lattice.
    Command,
    /// A `TclOO`/snit/itcl class or instance command created by a
    /// registry-described definer (`target` = its canonical qname).
    /// Distinct from [`Self::Proc`] so `NAME destroy` — the universal
    /// object method — is only modelled as a deletion for names that
    /// actually denote objects.
    Class,
    /// An `interp alias` (`target` = the alias target name).
    Alias,
    /// Renamed/deleted-away or never-defined → dispatches to `unknown`.
    Opaque,
    /// ⊤ — conflicting bindings at a merge, or dynamic mutation.
    Unknown,
}

/// A command-name binding: its [`BindingKind`] plus an optional target
/// (the proc qname for [`BindingKind::Proc`], the alias target for
/// [`BindingKind::Alias`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// The kind of binding.
    pub kind: BindingKind,
    /// Target name for `Proc` / `Alias` bindings; `None` otherwise.
    pub target: Option<String>,
}

impl Binding {
    const fn of(kind: BindingKind) -> Self {
        Self { kind, target: None }
    }

    /// True when the name still denotes its original core builtin.
    #[must_use]
    pub fn is_original_builtin(&self) -> bool {
        self.kind == BindingKind::Builtin
    }

    /// True when the name denotes a concrete, foldable user proc.
    #[must_use]
    pub fn is_foldable_proc(&self) -> bool {
        self.kind == BindingKind::Proc && self.target.is_some()
    }
}

/// A sparse per-name binding map.  An absent name takes its *default*
/// binding (a pure function of the name — builtin if the registry knows
/// the bare global name, else opaque).  `wildcard` marks "every name is
/// ⊤ from here" after a dynamic mutation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct State {
    map: HashMap<String, Binding>,
    wildcard: bool,
}

/// The unperturbed binding of `qname` before any rename/proc/alias.
///
/// Only an unqualified global name the registry knows is a `Builtin`
/// (`::string` → `string`); a namespaced tail (`::ns::foo`) or an
/// unknown name is `Opaque`.
fn default_binding(qname: &str, registry: &CommandRegistry) -> Binding {
    let bare = qname.strip_prefix("::").unwrap_or(qname);
    if !bare.contains("::") && registry.get(bare).is_some() {
        Binding::of(BindingKind::Builtin)
    } else {
        Binding::of(BindingKind::Opaque)
    }
}

/// Resolve `qname`'s binding within `state`, honouring wildcard + default.
fn binding_in(state: &State, qname: &str, registry: &CommandRegistry) -> Binding {
    if state.wildcard {
        return Binding::of(BindingKind::Unknown);
    }
    state
        .map
        .get(qname)
        .cloned()
        .unwrap_or_else(|| default_binding(qname, registry))
}

/// Lattice join: ⊥ is identity, equal stays, anything else rises to ⊤.
fn join_binding(a: &Binding, b: &Binding) -> Binding {
    if a.kind == BindingKind::Bottom {
        return b.clone();
    }
    if b.kind == BindingKind::Bottom {
        return a.clone();
    }
    if a == b {
        return a.clone();
    }
    Binding::of(BindingKind::Unknown)
}

fn literal_subject(subject: &TransitionSubject) -> Option<&str> {
    match subject {
        TransitionSubject::Literal(value) => Some(value),
        TransitionSubject::Unknown { .. } => None,
    }
}

fn definition_binding(kind: CommandBindingDefinitionKind, qname: String) -> Binding {
    let kind = match kind {
        CommandBindingDefinitionKind::Command => BindingKind::Command,
        CommandBindingDefinitionKind::Procedure => BindingKind::Proc,
        CommandBindingDefinitionKind::Object => BindingKind::Class,
    };
    Binding {
        kind,
        target: Some(qname),
    }
}

fn apply_binding_transition(
    transition: &CommandBindingTransition,
    state: &mut State,
    registry: &CommandRegistry,
) {
    match transition {
        CommandBindingTransition::Define { name, kind } => {
            let Some(name) = literal_subject(name) else {
                state.wildcard = true;
                return;
            };
            if name.is_empty() {
                return;
            }
            let qname = nqn(name);
            state
                .map
                .insert(qname.clone(), definition_binding(*kind, qname));
        }
        CommandBindingTransition::Move { from, to } => {
            let (Some(from), Some(to)) = (literal_subject(from), literal_subject(to)) else {
                state.wildcard = true;
                return;
            };
            let from = nqn(from);
            let moved = binding_in(state, &from, registry);
            state.map.insert(from, Binding::of(BindingKind::Opaque));
            if !to.is_empty() {
                state.map.insert(nqn(to), moved);
            }
        }
        CommandBindingTransition::Delete { interpreter, name } => {
            let affects_current = match interpreter {
                None => true,
                Some(subject) => match literal_subject(subject) {
                    Some("") => true,
                    Some(_) => false,
                    None => {
                        state.wildcard = true;
                        return;
                    }
                },
            };
            if !affects_current {
                return;
            }
            let Some(name) = literal_subject(name) else {
                state.wildcard = true;
                return;
            };
            state
                .map
                .insert(nqn(name), Binding::of(BindingKind::Opaque));
        }
        CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter: _,
            target,
        } => {
            let Some(source_interpreter) = literal_subject(source_interpreter) else {
                state.wildcard = true;
                return;
            };
            if !source_interpreter.is_empty() {
                return;
            }
            let (Some(alias), Some(target)) = (literal_subject(alias), literal_subject(target))
            else {
                state.wildcard = true;
                return;
            };
            state.map.insert(
                nqn(alias),
                Binding {
                    kind: BindingKind::Alias,
                    target: Some(nqn(target)),
                },
            );
        }
        CommandBindingTransition::Unknown { .. } => state.wildcard = true,
    }
}

/// Apply the registry's closed transition description.  `None` means the
/// invocation was unresolved or deliberately unstamped, so the compatibility
/// path below must remain conservative.
fn apply_registry_transitions(
    stmt: &Statement,
    state: &mut State,
    registry: &CommandRegistry,
) -> Option<()> {
    let facts = invocation_facts(stmt, registry)?;
    let transitions = facts.state_transitions.declared()?;
    for fact in transitions.facts() {
        match &fact.transition {
            StateTransition::CommandBinding(transition) => {
                apply_binding_transition(transition, state, registry);
            }
            StateTransition::Widen(widening)
                if widening
                    .domains
                    .contains(&StateTransitionDomain::CommandBindings) =>
            {
                state.wildcard = true;
            }
            StateTransition::Interpreter(_)
            | StateTransition::VariableCellAlias(_)
            | StateTransition::Namespace(_)
            | StateTransition::Trace(_)
            | StateTransition::ObjectDispatch(_)
            | StateTransition::Widen(_) => {}
        }
    }
    Some(())
}

/// Apply `stmt`'s command-table mutation to `state` in place.
///
/// Registry-declared [`StateTransition`] facts are authoritative for ordinary
/// definitions, renames, deletions, and aliases.  Unstamped legacy mutators
/// widen conservatively instead of being re-decoded here.  Runtime object
/// receiver calls remain a small separate path because their source head is a
/// value, not a statically registered command.
fn stmt_gen(stmt: &Statement, state: &mut State, registry: &CommandRegistry) {
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
        return;
    };
    if state.wildcard {
        return; // already maximally conservative
    }
    if apply_registry_transitions(stmt, state, registry).is_some() {
        return;
    }

    // The canonical command falls back to the source spelling.
    let cmd = stmt.canonical_command_or_source();
    let cmd_bare = cmd.strip_prefix("::").unwrap_or(cmd);

    if matches!(
        registry.command_table_effect(cmd_bare, args.first().map(String::as_str)),
        Some(
            CommandTableEffect::DefinesProcedure
                | CommandTableEffect::RenamesCommands
                | CommandTableEffect::CreatesAliases
        )
    ) {
        state.wildcard = true;
        return;
    }

    // Class lifecycle.  A registry-described definer creates a command;
    // registry-declared object-surface methods can delete it or manufacture
    // another named object command.
    if let Some(created) = definer_created_command(registry, cmd, args) {
        state.map.insert(
            nqn(&created),
            Binding {
                kind: BindingKind::Class,
                target: Some(nqn(&created)),
            },
        );
        return;
    }
    let head = nqn(cmd);
    let head_binding = binding_in(state, &head, registry);
    if head_binding.kind == BindingKind::Class {
        if args
            .first()
            .is_some_and(|w| registry.is_destructive_object_method(w))
        {
            state.map.insert(head, Binding::of(BindingKind::Opaque));
        } else if let Some(name_idx) = args.first().and_then(|method| {
            registry
                .is_manufacturer_method(method)
                .then(|| registry.uniform_manufacturer_names_instance_at(method))
                .flatten()
        }) && let Some(name) = args.get(name_idx)
            && !is_dynamic_word(name)
        {
            state.map.insert(
                nqn(name),
                Binding {
                    kind: BindingKind::Class,
                    target: head_binding.target,
                },
            );
        }
    }
}

/// The command a registry-described class definer creates, when this call
/// is a creation: `METACLASS create NAME …` / `METACLASS createWithNamespace
/// NAME …` for the `TclOo` family (gated on `IS_OO_METACLASS`, mirroring the
/// analyser's dual gate), `DEFINER NAME BODY` for snit/itcl.  `None` for
/// non-definers, `new` (auto-named), or a dynamic name.
fn definer_created_command(
    registry: &CommandRegistry,
    cmd: &str,
    args: &[String],
) -> Option<String> {
    let spec = registry.get(cmd)?;
    let grammar = spec.definition_body?;
    let name = match grammar.family {
        tcl_registry::definer::DefinerFamily::TclOo => {
            if !spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS) {
                return None;
            }
            let method = registry.exported_manufacturer_method(cmd, args.first()?)?;
            args.get(usize::from(method.names_instance_at?))?
        }
        _ => args.first()?,
    };
    (!name.is_empty() && !is_dynamic_word(name)).then(|| name.clone())
}

/// Join predecessor exit states into a block-entry state.
///
/// A name absent from a finished predecessor exit takes its **default**
/// binding, whereas a name not yet contributed to the accumulator is
/// **⊥** (identity for join) — so the merge is per-name across all
/// predecessors at once, seeded at ⊥.  One wildcard predecessor forces
/// the whole merge to wildcard.
fn merge_preds(pred_exits: &[&State], registry: &CommandRegistry) -> State {
    if pred_exits.is_empty() {
        return State::default();
    }
    if pred_exits.iter().any(|pe| pe.wildcard) {
        return State {
            map: HashMap::new(),
            wildcard: true,
        };
    }
    let mut relevant: HashSet<&String> = HashSet::new();
    for pe in pred_exits {
        relevant.extend(pe.map.keys());
    }
    let mut entry = State::default();
    for name in relevant {
        let mut acc = Binding::of(BindingKind::Bottom);
        for pe in pred_exits {
            let b = pe
                .map
                .get(name)
                .cloned()
                .unwrap_or_else(|| default_binding(name, registry));
            acc = join_binding(&acc, &b);
        }
        if acc != default_binding(name, registry) {
            entry.map.insert(name.clone(), acc);
        }
    }
    entry
}

/// Result of the command-binding analysis for one function/script.
///
/// `block_entry` holds the lattice state at each block's entry;
/// point-wise queries replay the gen of the statements before the
/// queried index.  Borrows the `cfg` and `registry` for the
/// point-wise query API.
pub struct CommandBinding<'a> {
    block_entry: HashMap<BlockId, State>,
    ordered_blocks: Vec<BlockId>,
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
}

impl CommandBinding<'_> {
    fn state_at_block(&self, block: BlockId, stmt_idx: usize) -> State {
        let mut state = self.block_entry.get(&block).cloned().unwrap_or_default();
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for stmt in blk.statements.iter().take(stmt_idx) {
                stmt_gen(stmt, &mut state, self.registry);
            }
        }
        state
    }

    /// The binding of `command_name` when `block::stmt_idx` executes.
    #[must_use]
    pub fn binding_at(&self, block: BlockId, stmt_idx: usize, command_name: &str) -> Binding {
        binding_in(
            &self.state_at_block(block, stmt_idx),
            &nqn(command_name),
            self.registry,
        )
    }

    /// True when `command_name` still denotes its core builtin here.
    #[must_use]
    pub fn is_original_builtin_at(
        &self,
        block: BlockId,
        stmt_idx: usize,
        command_name: &str,
    ) -> bool {
        self.binding_at(block, stmt_idx, command_name)
            .is_original_builtin()
    }

    /// Every command name perturbed from its default *anywhere* in the
    /// body — the flow-insensitive union over all points of names whose
    /// binding ever differs from its default.
    #[must_use]
    pub fn rebound_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();
        for block in &self.ordered_blocks {
            let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
            self.collect_rebound(&state, &mut names);
            if let Some(blk) = self.cfg.blocks.get(block) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut state, self.registry);
                    self.collect_rebound(&state, &mut names);
                }
            }
        }
        names
    }

    fn collect_rebound(&self, state: &State, names: &mut HashSet<String>) {
        for (name, binding) in &state.map {
            if *binding != default_binding(name, self.registry) {
                names.insert(name.clone());
            }
        }
    }

    /// True when some path performs a *dynamic* command-table mutation.
    #[must_use]
    pub fn has_wildcard(&self) -> bool {
        for block in &self.ordered_blocks {
            let mut state = self.block_entry.get(block).cloned().unwrap_or_default();
            if state.wildcard {
                return true;
            }
            if let Some(blk) = self.cfg.blocks.get(block) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut state, self.registry);
                    if state.wildcard {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Compute the flow-sensitive command-binding lattice for `cfg`.
///
/// `initial` seeds the entry block's state — the command bindings
/// already in force when this function begins.  The top-level analysis
/// seeds it with every module procedure (`{qname: Proc(qname)}`) so a
/// proc defined inside a `namespace eval` block is still known to be a
/// proc, while top-level `rename` / redefinition events perturb it
/// flow-sensitively.
#[must_use]
pub fn analyse_command_binding<'a>(
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
    initial: &[(String, Binding)],
) -> CommandBinding<'a> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|id| (*id, Vec::new())).collect();
    for (id, blk) in &cfg.blocks {
        if let Some(term) = &blk.terminator {
            for succ in term.successors() {
                if let Some(v) = preds.get_mut(&succ) {
                    v.push(*id);
                }
            }
        }
    }

    let order = cfg.reverse_postorder();
    let seed = State {
        map: initial.iter().cloned().collect(),
        wildcard: false,
    };

    let mut block_entry: HashMap<BlockId, State> = cfg
        .blocks
        .keys()
        .map(|id| (*id, State::default()))
        .collect();
    let mut block_exit = block_entry.clone();

    // Monotonic forward fixpoint: the per-name lattice has height 3 and
    // the join only rises, so RPO iteration terminates.
    let mut changed = true;
    while changed {
        changed = false;
        for id in &order {
            let entry = {
                let mut pred_states: Vec<&State> = preds
                    .get(id)
                    .map(|ps| ps.iter().map(|p| &block_exit[p]).collect())
                    .unwrap_or_default();
                if *id == cfg.entry {
                    pred_states.push(&seed);
                }
                merge_preds(&pred_states, registry)
            };
            block_entry.insert(*id, entry.clone());
            let mut exit_state = entry;
            if let Some(blk) = cfg.blocks.get(id) {
                for stmt in &blk.statements {
                    stmt_gen(stmt, &mut exit_state, registry);
                }
            }
            if exit_state != block_exit[id] {
                block_exit.insert(*id, exit_state);
                changed = true;
            }
        }
    }

    CommandBinding {
        block_entry,
        ordered_blocks: order,
        cfg,
        registry,
    }
}

/// Conservative, flow-insensitive summary of command rebindings across a
/// whole module — the input to the optimiser's builtin-fold trust gate.
///
/// A `rename` / proc redef / `interp alias` buried in a proc body only
/// takes effect when that proc is *called*, and the cross-proc call order
/// is not statically known.  Rather than a full interprocedural
/// call-effect fixpoint, this takes the sound over-approximation: any
/// core builtin some body may rebind is treated as untrusted
/// *everywhere*.  Top-level rebindings stay precise via the
/// flow-sensitive [`CommandBinding`] lattice; this whole-module union is
/// the conservative fold gate.
///
/// `Default` trusts everything (no names, not dynamic) — the
/// "no mutations observed" baseline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModuleCommandMutations {
    /// Canonical names of core builtins some body may rebind.
    names: std::collections::HashSet<String>,
    /// Canonical names that are the *source* or *target* of a `rename`, or
    /// the alias name of an `interp alias`, anywhere in the module — i.e.
    /// a name that no longer reliably denotes the proc it was declared as
    /// (or never denoted one at all). Unlike `names`, this is NOT
    /// restricted to builtins: a plain `proc NAME { … }` declaration is
    /// deliberately excluded (declaring a name as itself is the expected,
    /// trustworthy binding) — only `rename` / `interp alias` touching the
    /// name is recorded. Feeds [`Self::trusts_proc_binding`].
    rebound: std::collections::HashSet<String>,
    /// A body performs a dynamic `rename`/alias/proc (target not
    /// statically known) → resolution of *any* name is opaque.
    dynamic: bool,
}

impl ModuleCommandMutations {
    /// True when `command_name` is not clobbered by any body mutation —
    /// i.e. the optimiser may still fold it with its original builtin
    /// semantics.
    #[must_use]
    pub fn trusts(&self, command_name: &str) -> bool {
        if self.dynamic {
            return false;
        }
        !self.names.contains(&nqn(command_name))
    }

    /// The everything-is-untrusted lattice top: `trusts` /
    /// `trusts_proc_binding` answer `false` for every name. The sound
    /// stand-in when a consumer has **no whole-module view at all** (the
    /// analyser's isolated per-item body pass, issue #1132) — folding with
    /// builtin semantics is then never permitted.
    #[must_use]
    pub fn distrust_all() -> Self {
        Self {
            names: std::collections::HashSet::new(),
            rebound: std::collections::HashSet::new(),
            dynamic: true,
        }
    }

    /// A canonical, hashable snapshot of this summary — see
    /// [`CommandTrustSnapshot`].
    #[must_use]
    pub fn snapshot(&self) -> CommandTrustSnapshot {
        let mut untrusted_builtins: Vec<String> = self.names.iter().cloned().collect();
        untrusted_builtins.sort_unstable();
        let mut rebound: Vec<String> = self.rebound.iter().cloned().collect();
        rebound.sort_unstable();
        CommandTrustSnapshot {
            untrusted_builtins,
            rebound,
            dynamic: self.dynamic,
        }
    }

    /// True when `proc_name` can still be trusted to denote the module
    /// procedure it was declared as at an arbitrary later call site — i.e.
    /// its bare name was never the subject of a later `rename` (as the old
    /// name being moved away *or* the new name a different command moved
    /// onto) or `interp alias` (as the alias name) anywhere in the module.
    ///
    /// Flow-insensitive and whole-module, like [`Self::trusts`]: a
    /// rebinding buried in a proc body only takes effect when that proc
    /// runs, and the cross-proc call order isn't statically known, so any
    /// observed rebinding of the name is treated as live everywhere. This
    /// is what makes it sound to gate the optimiser's proc-call constant
    /// fold (O103) on this query — folding a call to the *original* proc's
    /// constant return would miscompile a script that later does
    /// `rename otherProc thisName` or `interp alias {} thisName {} other`.
    #[must_use]
    pub fn trusts_proc_binding(&self, proc_name: &str) -> bool {
        if self.dynamic {
            return false;
        }
        !self.rebound.contains(&nqn(proc_name))
    }
}

/// A canonical (sorted), hashable form of [`ModuleCommandMutations`], so
/// the whole-module trust fact can ride inside a memoisation key — the
/// analyser's per-item body pass carries it on each deferred body whose
/// text could fold a command substitution (issue #1132), keeping the
/// isolated fragment memo sound when a `rename` elsewhere in the file
/// appears or disappears.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandTrustSnapshot {
    untrusted_builtins: Vec<String>,
    rebound: Vec<String>,
    dynamic: bool,
}

impl CommandTrustSnapshot {
    /// Rebuild the queryable summary this snapshot was taken from.
    #[must_use]
    pub fn to_mutations(&self) -> ModuleCommandMutations {
        ModuleCommandMutations {
            names: self.untrusted_builtins.iter().cloned().collect(),
            rebound: self.rebound.iter().cloned().collect(),
            dynamic: self.dynamic,
        }
    }
}

/// Collect tampered-with *core builtins* (default `Builtin` but observed
/// otherwise) plus the wildcard flag from `state` into the accumulators.
/// A freshly-defined user proc (default `Opaque` → `Proc`) is deliberately
/// excluded: it doesn't untrust any builtin.
fn collect_tampered_builtins(
    state: &State,
    registry: &CommandRegistry,
    names: &mut std::collections::HashSet<String>,
    dynamic: &mut bool,
) {
    if state.wildcard {
        *dynamic = true;
    }
    for (name, binding) in &state.map {
        let default = default_binding(name, registry);
        if *binding != default && default.kind == BindingKind::Builtin {
            names.insert(name.clone());
        }
    }
}

/// Record the proc-binding-relevant names touched by a `rename` or `interp
/// alias` statement: the *source* name (vacated — no longer denotes what it
/// used to) and, for a static target, the *destination* name (now denoting
/// whatever the source used to, not its own original declaration, if it
/// even had one). A `proc` (re)declaration is deliberately NOT recorded —
/// declaring `NAME` as itself is the expected, trustworthy binding; only
/// *moving* a binding onto a name, or *vacating* a name that used to denote
/// a proc, breaks the "this bare name still denotes the proc it was
/// declared as" invariant [`ModuleCommandMutations::trusts_proc_binding`]
/// needs. Independent of the [`State`] lattice — a direct syntactic scan,
/// since (unlike the builtin-only trust gate) there is no meaningful
/// "default" binding to diff a proc name against: a plain declaration
/// *also* changes the name's binding away from its textual default, so
/// diffing against `default_binding` cannot distinguish "declared itself"
/// from "rebound to something else".
fn collect_proc_rebindings(
    stmt: &Statement,
    namespace: &str,
    registry: &CommandRegistry,
    rebound: &mut std::collections::HashSet<String>,
    dynamic: &mut bool,
) {
    let (Statement::Call { args, .. } | Statement::Barrier { args, .. }) = stmt else {
        return;
    };
    if let Some(facts) = invocation_facts(stmt, registry)
        && let Some(transitions) = facts.state_transitions.declared()
    {
        for fact in transitions.facts() {
            match &fact.transition {
                StateTransition::CommandBinding(CommandBindingTransition::Move { from, to }) => {
                    let (Some(from), Some(to)) = (literal_subject(from), literal_subject(to))
                    else {
                        *dynamic = true;
                        return;
                    };
                    insert_rebound_candidates(from, namespace, rebound);
                    if !to.is_empty() {
                        insert_rebound_candidates(to, namespace, rebound);
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Delete {
                    interpreter,
                    name,
                }) => {
                    let affects_current = match interpreter {
                        None => true,
                        Some(subject) => match literal_subject(subject) {
                            Some("") => true,
                            Some(_) => false,
                            None => {
                                *dynamic = true;
                                return;
                            }
                        },
                    };
                    if affects_current {
                        let Some(name) = literal_subject(name) else {
                            *dynamic = true;
                            return;
                        };
                        insert_rebound_candidates(name, namespace, rebound);
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Alias {
                    source_interpreter,
                    alias,
                    ..
                }) => {
                    let Some(source_interpreter) = literal_subject(source_interpreter) else {
                        *dynamic = true;
                        return;
                    };
                    if source_interpreter.is_empty() {
                        let Some(alias) = literal_subject(alias) else {
                            *dynamic = true;
                            return;
                        };
                        insert_rebound_candidates(alias, namespace, rebound);
                    }
                }
                StateTransition::CommandBinding(CommandBindingTransition::Unknown { .. }) => {
                    *dynamic = true;
                    return;
                }
                StateTransition::Widen(widening)
                    if widening
                        .domains
                        .contains(&StateTransitionDomain::CommandBindings) =>
                {
                    *dynamic = true;
                    return;
                }
                StateTransition::CommandBinding(CommandBindingTransition::Define { .. })
                | StateTransition::Interpreter(_)
                | StateTransition::VariableCellAlias(_)
                | StateTransition::Namespace(_)
                | StateTransition::Trace(_)
                | StateTransition::ObjectDispatch(_)
                | StateTransition::Widen(_) => {}
            }
        }
        return;
    }

    let cmd = stmt.canonical_command_or_source();
    let cmd_bare = cmd.strip_prefix("::").unwrap_or(cmd);
    match registry.command_table_effect(cmd_bare, args.first().map(String::as_str)) {
        Some(CommandTableEffect::RenamesCommands | CommandTableEffect::CreatesAliases) => {
            // An unstamped command-table mutation is not safe to decode in a
            // consumer. The registry transition descriptor must be enriched;
            // until then, distrust every binding.
            *dynamic = true;
        }
        // A `proc` declaration is deliberately NOT recorded here — see
        // the doc comment above.
        Some(CommandTableEffect::DefinesProcedure) | None => {}
    }
}

/// Record every name a bare `rename` / `interp alias` argument could
/// resolve to when it runs inside `namespace` — Tcl resolves an
/// unqualified command name against the *current* namespace at the point
/// the `rename`/`interp alias` executes, not the global namespace (a
/// `proc ::ns::doit {} { rename triple double }` renames `::ns::triple`
/// to `::ns::double`, not `::triple`/`::double` — confirmed against
/// tclsh 9.0.4). This scan is flow-insensitive and doesn't know whether a
/// same-named command already exists in `namespace` at that point, so it
/// conservatively records BOTH the namespace-relative and the
/// global-rooted candidate for a bare name — the same sound
/// over-approximation [`collect_tampered_builtins`] already applies.
/// A name that already contains `::` resolves unambiguously (rooted at
/// `::`, matching the optimiser's own `resolve_proc_qname` simplified
/// qualification rule), so only one candidate is recorded for it.
fn insert_rebound_candidates(
    name: &str,
    namespace: &str,
    rebound: &mut std::collections::HashSet<String>,
) {
    if name.contains("::") || namespace == "::" {
        rebound.insert(nqn(name));
        return;
    }
    rebound.insert(nqn(&format!("{namespace}::{name}")));
    rebound.insert(nqn(name));
}

/// The mutable rebinding-tracking state [`walk_body_calls`] threads
/// through its recursive descent, grouped into one struct (rather than
/// three separate `&mut` parameters) so adding the depth-cap parameter
/// below doesn't push the function over clippy's `too_many_arguments`
/// threshold.
struct RebindState<'a> {
    names: &'a mut std::collections::HashSet<String>,
    rebound: &'a mut std::collections::HashSet<String>,
    dynamic: &'a mut bool,
}

/// Apply the gen of every `Call` / `Barrier` in `script` (recursing into
/// nested structured bodies, in source order) to `state`, collecting
/// after *each* mutation — so a builtin renamed away and later restored
/// (`rename string ms; …; rename ms string`) is still recorded as
/// tampered within that window. `depth` is the nesting level of `script`
/// — reuses [`crate::optimiser::MAX_OPTIMISER_WALK_DEPTH`] (this walker
/// isn't itself part of the optimiser module, but shares the same
/// `Script`/`Statement`-tree-depth semantics as every walker guarded by
/// that constant, so a second identically-valued constant would only add
/// drift risk).
fn walk_body_calls(
    script: &crate::ir::Script,
    state: &mut State,
    registry: &CommandRegistry,
    namespace: &str,
    rebind: &mut RebindState<'_>,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for stmt in &script.statements {
        match stmt {
            Statement::Call { .. } | Statement::Barrier { .. } => {
                collect_proc_rebindings(stmt, namespace, registry, rebind.rebound, rebind.dynamic);
                stmt_gen(stmt, state, registry);
                collect_tampered_builtins(state, registry, rebind.names, rebind.dynamic);
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for c in clauses {
                    walk_body_calls(&c.body, state, registry, namespace, rebind, depth + 1);
                }
                if let Some(b) = else_body {
                    walk_body_calls(b, state, registry, namespace, rebind, depth + 1);
                }
            }
            Statement::For {
                init, next, body, ..
            } => {
                walk_body_calls(init, state, registry, namespace, rebind, depth + 1);
                walk_body_calls(next, state, registry, namespace, rebind, depth + 1);
                walk_body_calls(body, state, registry, namespace, rebind, depth + 1);
            }
            Statement::While { body, .. }
            | Statement::Catch { body, .. }
            | Statement::Foreach { body, .. } => {
                walk_body_calls(body, state, registry, namespace, rebind, depth + 1);
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                walk_body_calls(body, state, registry, namespace, rebind, depth + 1);
                for h in handlers {
                    walk_body_calls(&h.body, state, registry, namespace, rebind, depth + 1);
                }
                if let Some(fb) = finally_body {
                    walk_body_calls(fb, state, registry, namespace, rebind, depth + 1);
                }
            }
            Statement::Switch {
                arms, default_body, ..
            } => {
                for a in arms {
                    if let Some(b) = &a.body {
                        walk_body_calls(b, state, registry, namespace, rebind, depth + 1);
                    }
                }
                if let Some(b) = default_body {
                    walk_body_calls(b, state, registry, namespace, rebind, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Summarise command-table mutations across the whole module — a
/// CFG-free recursive IR walk over the top-level script *and* every proc
/// / method body, so it can run before per-function CFGs are built.
///
/// Tampered-with core builtins and rebound names generally are reported
/// (see [`collect_tampered_builtins`]).  The result feeds both the
/// optimiser's builtin-fold trust gate ([`ModuleCommandMutations::trusts`])
/// and its proc-call fold trust gate
/// ([`ModuleCommandMutations::trusts_proc_binding`]).
#[must_use]
pub fn scan_module_command_mutations(
    ir_module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> ModuleCommandMutations {
    let mut names = std::collections::HashSet::new();
    let mut rebound = std::collections::HashSet::new();
    let mut dynamic = false;

    let mut visit = |script: &crate::ir::Script, namespace: &str| {
        let mut state = State::default();
        let mut rebind = RebindState {
            names: &mut names,
            rebound: &mut rebound,
            dynamic: &mut dynamic,
        };
        walk_body_calls(script, &mut state, registry, namespace, &mut rebind, 0);
    };

    visit(&ir_module.top_level, "::");
    for (qname, proc) in &ir_module.procedures {
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(qname);
        visit(&proc.body, &namespace);
    }
    for (mqname, method) in &ir_module.methods {
        let namespace = crate::optimiser::helpers::naming::namespace_from_qualified(mqname);
        visit(&method.body, &namespace);
    }

    ModuleCommandMutations {
        names,
        rebound,
        dynamic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compilation_unit::CompilationUnit;

    fn analyse(src: &str) -> (CompilationUnit, CommandRegistry) {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(src, &reg, false);
        (cu, reg)
    }

    #[test]
    fn unperturbed_builtin_is_builtin_no_rebound() {
        let (cu, reg) = analyse("string toupper a");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert!(cb.is_original_builtin_at(fu.cfg.entry, 0, "string"));
        assert!(cb.rebound_names().is_empty());
        assert!(!cb.has_wildcard());
    }

    #[test]
    fn class_destroy_makes_the_class_command_opaque() {
        // `Animal destroy` deletes the class command: the binding is Class
        // before the destroy and Opaque after, so a later `Animal new`
        // draws W128.  Definer creation and the destructive method are
        // both registry data (definition_body / oo::object's `destroy`).
        let (cu, reg) = analyse(
            "oo::class create Animal {}
Animal new
Animal destroy
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 1, "Animal").kind, BindingKind::Class);
        assert_eq!(
            cb.binding_at(entry, 3, "Animal").kind,
            BindingKind::Opaque,
            "the class command is deleted after `Animal destroy`"
        );
        assert!(cb.rebound_names().contains("::Animal"));
    }

    #[test]
    fn instance_destroy_makes_the_instance_command_opaque() {
        // `Animal create fido` binds the instance command; `fido destroy`
        // deletes it; the class itself stays bound.
        let (cu, reg) = analyse(
            "oo::class create Animal {}
Animal create fido
fido destroy
fido bark
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 3, "fido").kind, BindingKind::Opaque);
        assert_eq!(cb.binding_at(entry, 4, "Animal").kind, BindingKind::Class);
    }

    #[test]
    fn destroy_as_ordinary_argument_is_not_a_deletion() {
        // A proc named `destroy` taking a class name as an ARGUMENT must
        // not delete anything: the head is the proc, not the class.
        let (cu, reg) = analyse(
            "oo::class create Animal {}
proc destroy {x} { puts $x }
destroy Animal
Animal new",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(
            cb.binding_at(entry, 3, "Animal").kind,
            BindingKind::Class,
            "the class survives an unrelated `destroy` call"
        );
    }

    #[test]
    fn snit_type_creation_binds_a_class() {
        let (cu, reg) = analyse(
            "snit::type Dog {}
Dog destroy
Dog create d",
        );
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 1, "Dog").kind, BindingKind::Class);
        assert_eq!(cb.binding_at(entry, 2, "Dog").kind, BindingKind::Opaque);
    }

    #[test]
    fn rename_deletion_makes_old_name_opaque_flow_sensitively() {
        // `string` is its builtin before the rename, opaque after.
        let (cu, reg) = analyse("string toupper a\nrename string {}\nstring toupper b");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        assert!(cb.is_original_builtin_at(entry, 0, "string"));
        assert_eq!(
            cb.binding_at(entry, 2, "string").kind,
            BindingKind::Opaque,
            "string is renamed away before stmt 2"
        );
        assert!(cb.rebound_names().contains("::string"));
    }

    #[test]
    fn rename_redirect_moves_binding_to_new_name() {
        let (cu, reg) = analyse("rename string mystr\nmystr toupper b");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        // After the rename: old `string` is opaque, `mystr` inherits the
        // builtin binding `string` denoted.
        assert_eq!(cb.binding_at(entry, 1, "string").kind, BindingKind::Opaque);
        assert_eq!(cb.binding_at(entry, 1, "mystr").kind, BindingKind::Builtin);
    }

    #[test]
    fn proc_redefinition_binds_name_to_proc() {
        let (cu, reg) = analyse("proc string {x} { return $x }\nstring foo");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        let entry = fu.cfg.entry;
        let b = cb.binding_at(entry, 1, "string");
        assert_eq!(b.kind, BindingKind::Proc);
        assert_eq!(b.target.as_deref(), Some("::string"));
        assert!(!cb.is_original_builtin_at(entry, 1, "string"));
        assert!(cb.rebound_names().contains("::string"));
    }

    #[test]
    fn dynamic_rename_collapses_to_wildcard() {
        let (cu, reg) = analyse("set x foo\nrename $x bar\nstring toupper a");
        let fu = cu.function("::top").unwrap();
        let cb = analyse_command_binding(&fu.cfg, &reg, &[]);
        assert!(cb.has_wildcard(), "dynamic rename sets the wildcard");
        // Under the wildcard everything resolves to Unknown (⊤), never a
        // concrete binding — so no spurious W128 can fire.
        let entry = fu.cfg.entry;
        assert_eq!(cb.binding_at(entry, 2, "string").kind, BindingKind::Unknown);
    }

    #[test]
    fn seed_marks_module_procs_as_proc() {
        // The W128 seed: a name seeded as PROC resolves to Proc at entry.
        let (cu, reg) = analyse("nonbuiltin a b");
        let fu = cu.function("::top").unwrap();
        let seed = vec![(
            "::myproc".to_owned(),
            Binding {
                kind: BindingKind::Proc,
                target: Some("::myproc".to_owned()),
            },
        )];
        let cb = analyse_command_binding(&fu.cfg, &reg, &seed);
        assert_eq!(
            cb.binding_at(fu.cfg.entry, 0, "myproc").kind,
            BindingKind::Proc
        );
    }

    #[test]
    fn module_mutations_distrust_rebound_builtins_only() {
        let reg = CommandRegistry::build_default();
        // A builtin renamed inside a proc body is distrusted everywhere
        // (over-approximation); a fresh user proc untrusts nothing.
        let cu = CompilationUnit::build_for(
            "proc clobber {} { rename string {} }\nproc myproc {} { return 1 }",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts("string"), "string is rebound in a proc body");
        assert!(m.trusts("lappend"), "an untouched builtin stays trusted");
        assert!(m.trusts("myproc"), "a fresh user proc untrusts nothing");

        // A dynamic mutation distrusts every name.
        let cu2 = CompilationUnit::build_for("set x foo\nrename $x bar", &reg, false);
        let m2 = scan_module_command_mutations(&cu2.ir_module, &reg);
        assert!(!m2.trusts("string") && !m2.trusts("lappend"));
    }

    /// Regression coverage for issue #996: `walk_body_calls` recurses once
    /// per nested `if`/`for`/`while`/`foreach`/`catch`/`try`/`switch`
    /// body, with no depth cap of its own before this fix. Transitively
    /// bounded to `MAX_LOWER_NEST_DEPTH` (256) by the lowering pass today,
    /// so this is defence-in-depth / consistency with every other
    /// full-tree walker in this crate, not a currently-reproducible
    /// crash. 1000 levels of source nesting is comfortably past this new
    /// cap; the assertion is that `scan_module_command_mutations` returns
    /// at all, not what it returns. Spawns its own big-stack thread since
    /// the lexer/CST/segmenter stages upstream of the lowering cap still
    /// walk the full un-truncated source nesting before that cap trims
    /// it — same rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_walk_body_calls() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = "proc clobber {} {\n".to_owned();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("rename string {}\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        src.push_str("}\n");
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let reg = CommandRegistry::build_default();
                let cu = CompilationUnit::build_for(&src, &reg, false);
                let _ = scan_module_command_mutations(&cu.ir_module, &reg);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn trusts_proc_binding_true_for_untouched_proc() {
        // TP control: a proc never named by any `rename` / `interp alias`
        // is trusted.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for("proc myproc {} { return 1 }", &reg, false);
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts_proc_binding("myproc"));
        assert!(m.trusts_proc_binding("::myproc"));
    }

    #[test]
    fn trusts_proc_binding_false_for_rename_source_and_target() {
        // FP guard: `rename triple double` perturbs BOTH names — `triple`
        // (vacated, no longer denotes what it did) and `double` (now
        // denotes `triple`'s body, not `double`'s own declaration).
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple double\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("double"), "rename target");
        assert!(!m.trusts_proc_binding("triple"), "rename source");
    }

    #[test]
    fn trusts_proc_binding_false_for_interp_alias_name() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc answer {} { return 42 }\nproc other {} { return 99 }\ninterp alias {} answer {} other\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(!m.trusts_proc_binding("answer"));
        // The alias *target* itself is untouched — still trusted.
        assert!(m.trusts_proc_binding("other"));
    }

    #[test]
    fn trusts_proc_binding_unaffected_by_unrelated_rename() {
        // TN control: renaming a DIFFERENT proc must not untrust this one —
        // `trusts_proc_binding` is per-name, unlike the whole-module
        // `dynamic` wildcard.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc double {n} { expr {$n * 2} }\nproc triple {n} { expr {$n * 3} }\nrename triple somethingElse\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(m.trusts_proc_binding("double"));
    }

    #[test]
    fn trusts_proc_binding_false_for_namespace_relative_rename() {
        // FP guard (reported in code review): a bare `rename` argument
        // inside a namespaced proc resolves relative to THAT proc's own
        // namespace, not the global namespace — `rename triple double`
        // inside `proc ::ns::doit` renames `::ns::triple` onto
        // `::ns::double`. An earlier version always rooted the bare names
        // globally (`::triple`/`::double`), so it never distrusted the
        // actually-affected namespaced names.
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "namespace eval ::ns {\n    proc double {n} { expr {$n * 2} }\n    proc triple {n} { expr {$n * 3} }\n}\nproc ::ns::doit {} { rename triple double }\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            !m.trusts_proc_binding("::ns::double"),
            "namespace-relative rename target"
        );
        assert!(
            !m.trusts_proc_binding("::ns::triple"),
            "namespace-relative rename source"
        );
        // The scan is flow-insensitive (it can't know whether `double`/
        // `triple` already existed as GLOBAL commands at the point
        // `::ns::doit` runs), so it conservatively distrusts the
        // global-rooted candidate too — a deliberate, sound
        // over-approximation (a missed fold, never a wrong one),
        // mirroring `collect_tampered_builtins`'s existing philosophy.
        assert!(
            !m.trusts_proc_binding("::double"),
            "global-rooted candidate"
        );
        assert!(
            !m.trusts_proc_binding("::triple"),
            "global-rooted candidate"
        );
    }

    // Regression: `proc max {...}` must not be distrusted. `max`/`min` read
    // like `tcl::mathop` operator words, but real Tcl never registered them
    // there (verified against tclsh 8.6/9.0 — `info commands
    // ::tcl::mathop::*` never lists them); they exist only as unrelated
    // `expr` math functions. A now-fixed registry bug once carried bare
    // `max`/`min` `CommandSpec` entries as if they were `tcl::mathop`
    // members, which made `default_binding` treat them as pre-existing
    // builtins — so a completely ordinary `proc max {...}` looked like it
    // was "renaming a builtin", silently blocking O103 from folding calls to
    // it (caught by `tests/optimiser.rs::interprocedural_constant_folding`).
    #[test]
    fn module_mutations_do_not_distrust_proc_named_like_mathop_word() {
        let reg = CommandRegistry::build_default();
        let cu = CompilationUnit::build_for(
            "proc max {a b} {\n    if {$a > $b} { return $a } else { return $b }\n}\nset v [max 3 7]\n",
            &reg,
            false,
        );
        let m = scan_module_command_mutations(&cu.ir_module, &reg);
        assert!(
            m.trusts("max"),
            "a plain proc sharing a name with an (incorrectly bare-registered) \
             tcl::mathop-lookalike must not be distrusted"
        );
    }
}
