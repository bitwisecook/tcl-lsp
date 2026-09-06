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

//! Command namespaces (T1.5) — the command-table-as-core-service.
//!
//! The runtime's command lookup is **one** `resolve(currentNs, name) → Command`
//! function (the command-binding contract's A1/A2 — see
//! `docs/design/contracts/command-binding-and-aliasing.md`):
//!
//! 1. Parse the name: a leading or embedded `::` ⇒ **qualified** (absolute if
//!    leading `::`, else relative to `currentNs`) → look up the simple name
//!    directly in that namespace (no path/import fallback).
//! 2. **Unqualified** → **(a)** the current namespace's command table, **(b)**
//!    each namespace on its `namespace path` in order, **(c)** the global `::`,
//!    **(d)** miss (the caller raises `invalid command name`, later `unknown`).
//!
//! The tree is an arena (`Vec<Namespace>` + [`NsId`] indices) — no `Rc`/parent
//! pointers, `wasm32`-friendly. `rename`/`interp alias`/`import`/ensembles layer
//! on this one resolver (they install redirect/alias `Command`s); the binding
//! lattice (only `pristine-builtin` inlines) is the AOT side.

use std::collections::{BTreeMap, BTreeSet};

use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_syntax::naming::{ends_with_separator, qualifier_segments as split_qualifier};

use crate::frame::VarTable;
use crate::interp::Command;

/// An index into the namespace arena. The global namespace `::` is always 0.
pub type NsId = usize;

/// The global namespace `::`.
pub const GLOBAL: NsId = 0;

/// The result of a [`Namespaces::rename`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameOutcome {
    /// `old` was moved to `new`.
    Renamed,
    /// `rename old ""` removed the command.
    Deleted,
    /// `old` did not resolve to any command.
    NoSuchCommand,
    /// `old` is an alias and moving it onto `new` would close an alias loop
    /// (C's `TclPreventAliasLoop`); the table is left untouched.
    AliasLoop,
    /// `new` already names a bound command (C's `TclRenameCommand` checks the
    /// destination's hash table *before* moving `old` out of its own — so
    /// this also catches a same-slot "self-rename" like `rename foo foo`,
    /// which tclsh 9.0.4 refuses too, since the source is still occupying
    /// that slot at check time); both `old` and the occupant at `new` are
    /// left untouched (issue #1412 item 1).
    TargetExists,
}

/// One namespace's command table: the `BTreeMap` the resolver looks names up
/// in, plus the retained `TCL_STRING_KEYS` bucket order `TclTeardownNamespace`
/// snapshots and a per-slot generation.
///
/// The order owner is C's `Namespace.cmdTable` itself: its bucket array
/// quadruples at a 3:1 load factor, never shrinks, and reverses chains on
/// every rebuild, all of which a namespace's command-delete traces observe.
/// The generation distinguishes a command a delete callback replaced from the
/// token the teardown snapshot named, so the replacement waits for the next
/// pass exactly as C's `CMD_DYING` early return makes it. Generations are
/// minted by the owning [`Namespaces`], so they identify a token across the
/// whole interpreter — a retained table and a same-named recreation hold two
/// distinct `::N::q` tokens at once.
#[derive(Default)]
struct CommandTable {
    entries: BTreeMap<Vec<u8>, (u64, Command)>,
    order: TclStringHashOrder,
}

impl CommandTable {
    fn get(&self, key: &[u8]) -> Option<&Command> {
        self.entries.get(key).map(|(_, command)| command)
    }

    fn contains_key(&self, key: &[u8]) -> bool {
        self.entries.contains_key(key)
    }

    fn keys(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.entries.keys()
    }

    fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Command)> {
        self.entries
            .iter()
            .map(|(key, (_, command))| (key, command))
    }

    fn values(&self) -> impl Iterator<Item = &Command> {
        self.entries.values().map(|(_, command)| command)
    }

    fn values_mut(&mut self) -> impl Iterator<Item = &mut Command> {
        self.entries.values_mut().map(|(_, command)| command)
    }

    /// Bind `command` at `key`, returning whatever it displaced. A live key is
    /// re-created at its bucket head, as C's `TclCreateObjCommandInNs` does
    /// when it deletes the old hash entry and creates a fresh one.
    fn insert(&mut self, key: Vec<u8>, command: Command, generation: u64) -> Option<Command> {
        match self.entries.insert(key.clone(), (generation, command)) {
            Some((_, displaced)) => {
                self.order.reinsert(&key);
                Some(displaced)
            }
            None => {
                self.order.insert(&key);
                None
            }
        }
    }

    /// Delete `key`'s entry, returning its binding. The bucket array keeps its
    /// capacity (`Tcl_DeleteHashEntry` never shrinks).
    fn remove(&mut self, key: &[u8]) -> Option<Command> {
        let (_, command) = self.entries.remove(key)?;
        self.order.remove(key);
        Some(command)
    }

    /// Take a slot's binding out while leaving its hash entry in place —
    /// C's alias-loop probe (`TclRenameCommand`) reassigns `cmdPtr->hPtr` and
    /// undoes the move without ever deleting the source's entry.
    fn take_slot(&mut self, key: &[u8]) -> Option<(u64, Command)> {
        self.entries.remove(key)
    }

    /// Put a slot taken by [`Self::take_slot`] back, creating the hash entry
    /// when the probe's destination did not already have one.
    fn restore_slot(&mut self, key: Vec<u8>, slot: (u64, Command)) -> Option<(u64, Command)> {
        self.order.insert(&key);
        self.entries.insert(key, slot)
    }

    /// Create `key`'s hash entry ahead of the binding that fills it — C's
    /// `TclRenameCommand` calls `Tcl_CreateHashEntry` on the destination
    /// before it deletes the source's entry.
    fn reserve_entry(&mut self, key: &[u8]) {
        self.order.insert(key);
    }

    /// Delete a hash entry created by [`Self::reserve_entry`] whose value has
    /// been taken back — the alias-loop probe's refused destination.
    fn drop_entry(&mut self, key: &[u8]) {
        self.order.remove(key);
    }

    /// `Tcl_DeleteHashTable` + `Tcl_InitHashTable`: the table returns to Tcl's
    /// four static buckets.
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// The token generation currently bound at `key`.
    fn generation(&self, key: &[u8]) -> Option<u64> {
        self.entries.get(key).map(|(generation, _)| *generation)
    }

    /// The live `(name, generation)` slots in `Tcl_FirstHashEntry` order — the
    /// snapshot `TclTeardownNamespace` takes before deleting each token.
    fn hash_order(&self) -> Vec<(Vec<u8>, u64)> {
        self.order
            .keys()
            .into_iter()
            .filter_map(|key| Some((key.to_vec(), self.generation(key)?)))
            .collect()
    }
}

/// One namespace: its simple name, its command table, child namespaces, the
/// `namespace path` search list, and `export` patterns.
struct Namespace {
    /// Simple name (e.g. `mathfunc`); the global namespace's is empty.
    name: Vec<u8>,
    parent: Option<NsId>,
    children: BTreeMap<Vec<u8>, NsId>,
    /// The child's `TCL_STRING_KEYS` table, including retained resize history.
    child_order: TclStringHashOrder,
    commands: CommandTable,
    /// `namespace path` — namespaces searched for unqualified commands (step b).
    path: Vec<NsId>,
    /// `namespace export` patterns — gate what `import` may pull (matched with
    /// `string match` glob via the shared [`tcl_syntax::glob`]).
    exports: Vec<Vec<u8>>,
    /// Per-namespace variable table (`Namespace.varTable`). The global
    /// namespace's holds the global variables; the variable resolver
    /// ([`crate::vars`]) routes qualified / global / namespace-eval names here.
    vars: VarTable,
    /// `namespace unknown` handler (a command prefix). `None` ⇒ the namespace
    /// uses the interpreter default (the global `::unknown`); an empty handler
    /// also resets to the default.
    unknown: Option<Vec<u8>>,
    /// The name a retained token keeps reporting once its parent edge is gone.
    /// C's deferred deletion nulls `parentPtr` — freeing the name for a fresh
    /// token — but leaves `fullName` alone, so `namespace current` in the
    /// frames still holding the token is unchanged.
    retained_fqn: Option<Vec<u8>>,
}

impl Namespace {
    fn new(name: Vec<u8>, parent: Option<NsId>) -> Namespace {
        Namespace {
            name,
            parent,
            children: BTreeMap::new(),
            child_order: TclStringHashOrder::default(),
            commands: CommandTable::default(),
            path: Vec::new(),
            exports: Vec::new(),
            vars: VarTable::default(),
            unknown: None,
            retained_fqn: None,
        }
    }
}

/// The namespace tree + the command resolver.
pub struct Namespaces {
    arena: Vec<Namespace>,
    /// Call frames currently running in each arena slot (C's
    /// `Namespace.activationCount`). `Tcl_PushCallFrame` counts every frame —
    /// proc, `apply`, TclOO method, `namespace eval`/`inscope` — and
    /// `Tcl_PopCallFrame` runs the deletion a non-zero count deferred.
    activations: Vec<u32>,
    /// Namespace-token identities permanently invalidated by deletion. Arena
    /// slots are stable and never reused, so a namespace recreated with the
    /// same name receives a distinct live identity while retained activations
    /// keep naming their deleted token.
    dead: BTreeSet<NsId>,
    /// The next command-token generation. One counter for the interpreter, so
    /// a token's identity is unique across namespaces and across a table that
    /// was thrown away and recreated under the same name.
    next_command_generation: u64,
    /// Tokens deleted while a frame was still running in them: C's
    /// `activationCount > 0` branch of `Tcl_DeleteNamespace` marks `NS_DYING`,
    /// unlinks the parent edge and returns, leaving the commands, variables and
    /// children in place for the frames that still hold the token.
    deferred: BTreeSet<NsId>,
    /// Namespace nodes detached during command-delete callbacks. They are not
    /// visible to `namespace exists`, but command definition/resolution still
    /// reaches their command tables until the deletion sweep finishes, just as
    /// C keeps a dying Namespace alive through its activation/token refs.
    dying_children: BTreeMap<(NsId, Vec<u8>), NsId>,
    dying: BTreeSet<NsId>,
    /// M11: Tcl 8.x resolves an unqualified variable at **namespace scope**
    /// to the global variable when the namespace has none but the global
    /// namespace does (reads and writes both); 9.0 removed the fallback
    /// (TIP 278, `TCL_NAMESPACE_ONLY`).  Defaults to the 9.0 behaviour
    /// (`false`); an 8.x embedding flips it via
    /// [`crate::interp::Interp::set_runtime_version`].
    pub(crate) ns_var_global_fallback: bool,
}

impl Default for Namespaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespaces {
    fn mark_command_deleted(command: &Command) {
        if let Command::Ensemble(token) = command {
            token.mark_deleted();
        }
    }

    /// The identity a freshly bound command token carries.
    fn mint_command_generation(&mut self) -> u64 {
        self.next_command_generation += 1;
        self.next_command_generation
    }

    fn command_fqn(&self, ns: NsId, simple: &[u8]) -> Vec<u8> {
        let mut fqn = self.qualified_name(ns);
        if fqn != b"::" {
            fqn.extend_from_slice(b"::");
        }
        fqn.extend_from_slice(simple);
        fqn
    }

    /// Install one real command binding. Replacement deletes the displaced
    /// command token; the installed ensemble token acquires this binding's FQN.
    fn insert_bound(&mut self, ns: NsId, simple: Vec<u8>, command: Command) {
        // C redefines a command by deleting the old hash entry and creating a
        // new one, so the name moves to its bucket head
        // (`TclCreateObjCommandInNs`).
        if let Some(displaced) = self.arena[ns].commands.remove(&simple) {
            Self::mark_command_deleted(&displaced);
        }
        if let Command::Ensemble(token) = &command {
            token.rename(self.command_fqn(ns, &simple));
        }
        let generation = self.mint_command_generation();
        self.arena[ns].commands.insert(simple, command, generation);
    }

    /// A fresh tree with just the global namespace `::`.
    #[must_use]
    pub fn new() -> Namespaces {
        Namespaces {
            ns_var_global_fallback: false,
            arena: vec![Namespace::new(Vec::new(), None)],
            activations: vec![0],
            next_command_generation: 0,
            dead: BTreeSet::new(),
            deferred: BTreeSet::new(),
            dying_children: BTreeMap::new(),
            dying: BTreeSet::new(),
        }
    }

    /// Register `command` under `name` (possibly qualified), creating any
    /// intermediate namespaces. A qualified `name` is rooted at global.
    pub fn register(&mut self, name: &[u8], command: Command) {
        let _ = self.register_at(name, command);
    }

    /// [`register`](Self::register) reporting the `(namespace, simple name)` it
    /// bound the command at — `None` for a name with no tail to bind (empty /
    /// `::` only, where nothing is registered). The alias-loop gate needs the
    /// binding site so it can walk the chain from it and unbind again.
    pub(crate) fn register_at(&mut self, name: &[u8], command: Command) -> Option<(NsId, Vec<u8>)> {
        let segments = split_qualifier(name);
        let (simple, ns_parts) = segments.split_last()?;
        let mut ns = GLOBAL;
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        let simple = (*simple).to_vec();
        self.insert_bound(ns, simple.clone(), command);
        Some((ns, simple))
    }

    /// The command bound directly at `(ns, name)` — the exact table slot, with
    /// no resolution walk (the alias-chain gate addresses bindings it has
    /// already located).
    pub(crate) fn command_in(&self, ns: NsId, name: &[u8]) -> Option<Command> {
        self.arena[ns].commands.get(name).cloned()
    }

    /// Remove the binding at `(ns, name)`, returning it — the rollback for a
    /// refused alias definition.
    pub(crate) fn unbind_in(&mut self, ns: NsId, name: &[u8]) -> Option<Command> {
        self.arena[ns].commands.remove(name)
    }

    /// The single command resolver, `resolve(currentNs, name)` (A2). Returns a
    /// **clone** of the small command handle (a fn-pointer for `Builtin`; the
    /// target + frozen prefix for `Alias`) so the caller can dispatch without
    /// holding a borrow on the table.
    #[must_use]
    pub fn resolve(&self, current: NsId, name: &[u8]) -> Option<Command> {
        self.home_of(current, name).and_then(|(ns, simple)| {
            // `home_of` reported this exact slot holds a binding.
            self.arena[ns].commands.get(&simple).cloned()
        })
    }

    /// Rebind an existing command in place at the namespace where it actually
    /// resolves (the full resolution order, incl. `namespace path`) — the
    /// `namespace ensemble configure` set form. Unlike [`bind`](Self::bind),
    /// which always targets `current`, this updates the command `resolve` would
    /// hit, so reconfiguring an ensemble reached via `namespace path` mutates the
    /// real binding rather than shadowing it. Returns `false` if `name` does not
    /// resolve (the caller has already verified it does).
    pub fn rebind_resolved(&mut self, current: NsId, name: &[u8], command: Command) -> bool {
        match self.home_of(current, name) {
            Some((ns, simple)) => {
                self.insert_bound(ns, simple, command);
                true
            }
            None => false,
        }
    }

    /// The canonical fully-qualified name a (relative/absolute) command `name`
    /// resolves to, following the full resolution order — or `None` if no such
    /// command exists. Used to key command/execution traces so they address the
    /// same binding `resolve` (and `rename`/`delete`) hit.
    #[must_use]
    pub fn resolve_fqn(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        self.home_of(current, name).map(|(ns, simple)| {
            let q = self.qualified_name(ns);
            let mut fqn = if q == b"::" { Vec::new() } else { q };
            fqn.extend_from_slice(b"::");
            fqn.extend_from_slice(&simple);
            fqn
        })
    }

    /// Sorted command names in namespace `ns` (`info commands`).
    #[must_use]
    pub fn command_names(&self, ns: NsId) -> Vec<&[u8]> {
        self.arena[ns].commands.keys().map(Vec::as_slice).collect()
    }

    /// Remove the command bound to `name` (the `rename old ""` / alias-clear
    /// path); returns whether it existed. Honours the full resolution order so
    /// `delete` retires the same binding `resolve` would have hit.
    pub fn delete(&mut self, current: NsId, name: &[u8]) -> bool {
        match self.home_of(current, name) {
            Some((ns, simple)) => self.arena[ns]
                .commands
                .remove(&simple)
                .is_some_and(|command| {
                    Self::mark_command_deleted(&command);
                    true
                }),
            None => false,
        }
    }

    /// Remove a command binding without deleting its token. Used only for
    /// visibility moves such as `interp hide`; the token remains alive.
    pub fn take(&mut self, current: NsId, name: &[u8]) -> Option<Command> {
        let (ns, simple) = self.home_of(current, name)?;
        self.arena[ns].commands.remove(&simple)
    }

    /// `rename old new`: move the command bound to `old` to `new` (both resolved
    /// relative to `current`, absolute when `::`-led); `new == ""` deletes it.
    /// Occupancy protection is the caller's job (see
    /// [`Self::destination_occupant_fqn`]) — a script-visible "already
    /// exists" refusal needs release-gate context (a TclOO root this release
    /// hides is not really taken) that this table-only layer does not have,
    /// so this unconditionally overwrites whatever is at the destination,
    /// same as [`Self::insert_bound`].
    ///
    /// A command moved across namespaces is re-homed: a `Command::Proc`'s
    /// `ns`/`fqn` are updated to the destination so `namespace current`
    /// inside its body reports the new namespace, mirroring C's
    /// `cmdPtr->nsPtr` reassignment.
    ///
    /// Built-in protection lives in the `rename` builtin, not here — this is the
    /// pure table operation.
    pub fn rename(&mut self, current: NsId, old: &[u8], new: &[u8]) -> RenameOutcome {
        let Some((old_ns, old_simple)) = self.home_of(current, old) else {
            return RenameOutcome::NoSuchCommand;
        };
        if new.is_empty() {
            // SAFETY of unwrap: `home_of` reported the binding exists.
            let cmd = self.arena[old_ns].commands.remove(&old_simple).unwrap();
            Self::mark_command_deleted(&cmd);
            return RenameOutcome::Deleted;
        }
        let Some((ns, simple)) = self.destination_of(current, new) else {
            // Unreachable for a non-empty, non-separator-terminated name;
            // nothing has moved yet, so there is nothing to put back.
            return RenameOutcome::NoSuchCommand;
        };
        // C's `TclRenameCommand` creates the destination hash entry *before*
        // deleting the source's, so the transient extra entry can trigger a
        // rebuild a delete-first order would not.
        self.arena[ns].commands.reserve_entry(&simple);
        // SAFETY of unwrap: `home_of` reported the binding exists.
        let cmd = self.arena[old_ns].commands.remove(&old_simple).unwrap();
        let cmd = Self::rehome_proc(cmd, ns, &self.command_fqn(ns, &simple));
        self.insert_bound(ns, simple, cmd);
        RenameOutcome::Renamed
    }

    /// The fully-qualified name of whatever is currently bound at the
    /// destination `rename old new` would write to, or `None` when it is
    /// free — read-only (beyond creating intermediate namespaces, same as
    /// C's `TCL_CREATE_NS_IF_UNKNOWN`, which `rename` re-resolves
    /// idempotently) so the `rename` builtin can decide occupancy (folding
    /// in release-gate context this layer does not have — see
    /// [`crate::interp::Interp::is_gate_hidden_object_root`]) *before* firing
    /// a rename trace or moving anything. `old`'s own binding still counts as
    /// occupying its slot here (it is not removed until the real `rename`
    /// call), so a same-slot self-rename (`rename foo foo`) reads as
    /// occupied too, matching tclsh 9.0.4.
    pub(crate) fn destination_occupant_fqn(
        &mut self,
        current: NsId,
        new: &[u8],
    ) -> Option<Vec<u8>> {
        if new.is_empty() {
            return None;
        }
        let (ns, simple) = self.destination_of(current, new)?;
        self.arena[ns]
            .commands
            .contains_key(&simple)
            .then(|| self.command_fqn(ns, &simple))
    }

    /// Re-home a `Command::Proc` moved by `rename` to its new binding site.
    /// Every other `Command` variant carries no namespace of its own and
    /// passes through unchanged.
    fn rehome_proc(command: Command, ns: NsId, fqn: &[u8]) -> Command {
        match command {
            Command::Proc(def) if def.ns != ns => {
                let mut def = (*def).clone();
                def.ns = ns;
                def.fqn = fqn.to_vec();
                Command::Proc(std::rc::Rc::new(def))
            }
            other => other,
        }
    }

    /// The `(namespace, simple name)` a written destination name binds — the
    /// split C's `TclGetNamespaceForQualName(…, TCL_CREATE_NS_IF_UNKNOWN)`
    /// performs for `rename`'s new name, creating any intermediate namespaces
    /// (C creates them even when the rename is later refused). A trailing
    /// separator run names the empty-string `{}` command in the full qualifier
    /// chain (`rename foo x::` binds `::x::`, `rename bar ::` the global `{}` —
    /// tclsh 8.6/9.0-pinned, #934), matching `command_home_ns` / `home_of`.
    fn destination_of(&mut self, current: NsId, new: &[u8]) -> Option<(NsId, Vec<u8>)> {
        let absolute = new.starts_with(b"::");
        let segments = split_qualifier(new);
        let (simple, ns_parts): (&[u8], &[&[u8]]) = if ends_with_separator(new) {
            (b"", &segments[..])
        } else {
            let (simple, ns_parts) = segments.split_last()?;
            (*simple, ns_parts)
        };
        let mut ns = if absolute { GLOBAL } else { current };
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        Some((ns, simple.to_vec()))
    }

    /// C's `TclPreventAliasLoop` (`tclInterp.c`) on the alias bound at
    /// `(ns, simple)`: follow the chain — each hop resolves the alias's stored
    /// target name **anchored at the global namespace**, exactly as dispatch
    /// does — and report whether it comes back to the alias we started from.
    /// An unresolvable target ends the chain (legal: aliases late-bind), and so
    /// does a target that is not itself an alias.
    ///
    /// Every alias already in the table passed this same gate when it was
    /// defined or renamed, so the chain holds no pre-existing cycle; the
    /// visited list bounds the walk regardless, so no table state can spin it.
    pub(crate) fn alias_chain_loops(&self, ns: NsId, simple: &[u8]) -> bool {
        let start = (ns, simple.to_vec());
        let mut hop = start.clone();
        let mut seen: Vec<(NsId, Vec<u8>)> = Vec::new();
        loop {
            let Some(Command::Alias { target, .. }) = self.command_in(hop.0, &hop.1) else {
                return false;
            };
            let Some(next) = self.home_of(GLOBAL, &target) else {
                return false;
            };
            if next == start || seen.contains(&next) {
                return true;
            }
            seen.push(next.clone());
            hop = next;
        }
    }

    /// C's `TclPreventAliasLoop` on the *rename* path (`TclRenameCommand` moves
    /// the command, checks, and puts it back on a hit): would moving the command
    /// bound to `old` onto `new` close an alias loop? The chain can only close
    /// on the alias once it is visible at its destination, so the move is made
    /// tentatively here and undone again — including any command it displaced —
    /// leaving the caller to perform the real rename when this returns `false`.
    pub(crate) fn rename_creates_alias_loop(
        &mut self,
        current: NsId,
        old: &[u8],
        new: &[u8],
    ) -> bool {
        if new.is_empty() {
            return false; // a delete cannot close a loop
        }
        let Some((old_ns, old_simple)) = self.home_of(current, old) else {
            return false;
        };
        if !matches!(
            self.command_in(old_ns, &old_simple),
            Some(Command::Alias { .. })
        ) {
            return false; // renaming a non-alias is always allowed
        }
        let Some((dest_ns, dest_simple)) = self.destination_of(current, new) else {
            return false;
        };
        if (dest_ns, dest_simple.as_slice()) == (old_ns, old_simple.as_slice()) {
            return false; // a self-rename moves nothing
        }
        // C creates the destination's hash entry for the probe and deletes it
        // again on a refusal, and never touches the source's entry at all
        // (`TclRenameCommand` deletes `oldHPtr` only once the check passes).
        // The tentative move therefore carries the slot values only.
        let Some(moving) = self.arena[old_ns].commands.take_slot(&old_simple) else {
            return false;
        };
        let displaced = self.arena[dest_ns]
            .commands
            .restore_slot(dest_simple.clone(), moving);
        let loops = self.alias_chain_loops(dest_ns, &dest_simple);
        let moved = self.arena[dest_ns].commands.take_slot(&dest_simple);
        match displaced {
            Some(displaced) => {
                self.arena[dest_ns]
                    .commands
                    .restore_slot(dest_simple, displaced);
            }
            None => self.arena[dest_ns].commands.drop_entry(&dest_simple),
        }
        if let Some(moved) = moved {
            self.arena[old_ns].commands.restore_slot(old_simple, moved);
        }
        loops
    }

    /// Rewrite every [`Command::Imported`] redirect whose source is `old_fqn`
    /// to point at `new_fqn`. Even an ensemble import keeps this by-name shadow
    /// alongside its retained token: if a later replacement retires the token,
    /// dispatch and `namespace origin` fall back through the source's *latest*
    /// binding. Rename is cold-path, so the full-tree scan is fine.
    pub fn retarget_imports(&mut self, old_fqn: &[u8], new_fqn: &[u8]) {
        for ns in &mut self.arena {
            for cmd in ns.commands.values_mut() {
                if let Command::Imported { source, .. } = cmd {
                    if source.as_slice() == old_fqn {
                        *source = new_fqn.to_vec();
                    }
                }
            }
        }
    }

    /// Attach imports of `source_fqn` (and imports retaining `old`, when this is
    /// an ensemble-to-ensemble replacement) to `new`. This changes alias
    /// metadata only; the displaced token itself remains retired and immutable.
    pub(crate) fn retarget_imports_to_ensemble(
        &mut self,
        source_fqn: &[u8],
        old: Option<&std::rc::Rc<crate::ensemble::EnsembleToken>>,
        new: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) {
        for ns in &mut self.arena {
            for command in ns.commands.values_mut() {
                let Command::Imported {
                    source, ensemble, ..
                } = command
                else {
                    continue;
                };
                let retains_old = old.is_some_and(|old| {
                    ensemble
                        .as_ref()
                        .is_some_and(|token| std::rc::Rc::ptr_eq(token, old))
                });
                if source.as_slice() == source_fqn || retains_old {
                    *source = source_fqn.to_vec();
                    *ensemble = Some(std::rc::Rc::clone(new));
                }
            }
        }
    }

    /// Every alias command's fully-qualified name across the tree (`interp
    /// aliases`). Global aliases keep their simple name (aliases are registered
    /// interpreter-wide); namespaced ones are qualified.
    #[must_use]
    pub fn alias_names(&self) -> Vec<Vec<u8>> {
        let mut found: Vec<(NsId, Vec<u8>)> = Vec::new();
        for (id, ns) in self.arena.iter().enumerate() {
            for (key, cmd) in ns.commands.iter() {
                // Both single-interp aliases and cross-interp (child→parent)
                // aliases are reported by `interp aliases` / `$child aliases`.
                if matches!(cmd, Command::Alias { .. } | Command::ParentAlias { .. }) {
                    found.push((id, key.clone()));
                }
            }
        }
        found
            .into_iter()
            .map(|(id, key)| {
                if id == GLOBAL {
                    key
                } else {
                    let mut q = self.qualified_name(id);
                    q.extend_from_slice(b"::");
                    q.extend_from_slice(&key);
                    q
                }
            })
            .collect()
    }

    /// Set namespace `ns`'s `namespace path` to the given namespaces.
    pub fn set_path(&mut self, ns: NsId, path: Vec<NsId>) {
        self.arena[ns].path = path;
    }

    /// The namespace a (possibly qualified) **command** `name` lives in, creating
    /// any intermediate namespaces — i.e. everything before the simple tail
    /// (`::a::b::foo` → `::a::b`; `foo` → `current`). For `proc`/`define_proc`,
    /// which needs the proc's home ns id (its run-time current namespace).
    pub(crate) fn command_home_ns(&mut self, current: NsId, name: &[u8]) -> NsId {
        let absolute = name.starts_with(b"::");
        let segments = split_qualifier(name);
        let mut ns = if absolute { GLOBAL } else { current };
        // A written name ending in a separator run names the empty-string
        // `{}` command inside its FULL qualifier chain — every segment is a
        // namespace part, none is the tail (`proc x:: {} {}` defines
        // `::x::`, tclsh 8.6/9.0-pinned, #934) — mirroring `home_of`'s
        // resolution split so definition and dispatch agree.
        let ns_parts: &[&[u8]] = if ends_with_separator(name) || name.is_empty() {
            &segments[..]
        } else {
            segments
                .split_last()
                .map_or(&[][..], |(_tail, ns_parts)| ns_parts)
        };
        // Command definition may target an *exact* detached namespace token
        // during its delete callback (`proc ::N::q ...`). Namespace creation
        // is different: a longer missing path below an original dying
        // descendant (`namespace eval ::N::C::X ...`) must build an entirely
        // fresh visible `N::C::X` tree. Resolve the whole qualifier first and
        // retain a dying token only when every qualifier segment already names
        // that exact token; otherwise creation below uses visible edges only.
        let mut existing = ns;
        let mut complete = true;
        for part in ns_parts {
            let next = self.arena[existing]
                .children
                .get(*part)
                .copied()
                .or_else(|| self.dying_children.get(&(existing, part.to_vec())).copied());
            let Some(next) = next else {
                complete = false;
                break;
            };
            existing = next;
        }
        if complete {
            return existing;
        }
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        ns
    }

    /// Find (creating if needed) the namespace named `qualified`, rooted at
    /// `current` (absolute if it leads with `::`). For `namespace eval`.
    pub fn ensure_namespace(&mut self, current: NsId, qualified: &[u8]) -> NsId {
        // A live child remains a public namespace while an already-dying
        // parent token is being torn down. Resolve through the retained dying
        // edge before creating; a path whose *final* token is dying instead
        // falls through and builds a fresh visible tree.
        if let Some(existing) = self.find_namespace(current, qualified) {
            if self.namespace_is_live(existing) {
                return existing;
            }
        }
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.ensure_child(ns, part);
        }
        ns
    }

    /// Resolve `qualified` to a live namespace, or `None`. A retained dying
    /// edge may be traversed to reach a child whose own token is still live,
    /// but a dying/dead final token is never returned as a public namespace.
    #[must_use]
    pub fn find_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId> {
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.arena[ns]
                .children
                .get(part)
                .copied()
                .or_else(|| self.dying_children.get(&(ns, part.to_vec())).copied())?;
        }
        self.namespace_is_live(ns).then_some(ns)
    }

    /// Whether `ns` still denotes a public namespace token. A dying arena node
    /// remains command-addressable during delete callbacks, but namespace
    /// introspection must reject even an empty relative name resolved from its
    /// retained current-namespace handle.
    #[must_use]
    pub(crate) fn namespace_is_live(&self, ns: NsId) -> bool {
        !self.dead.contains(&ns) && !self.dying.contains(&ns)
    }

    // -- activations and deferred teardown (C's `activationCount`) -----------

    /// Count the activation `Tcl_PushCallFrame` adds when a call frame starts
    /// running in `ns`.
    pub(crate) fn activation_enter(&mut self, ns: NsId) {
        self.activations[ns] += 1;
    }

    /// Drop the activation a popped frame held, reporting whether that was the
    /// last one holding a deferred token — the caller then runs the teardown
    /// `Tcl_PopCallFrame` re-enters `Tcl_DeleteNamespace` for.
    pub(crate) fn activation_leave(&mut self, ns: NsId) -> bool {
        let count = &mut self.activations[ns];
        *count = count.saturating_sub(1);
        *count == 0 && self.deferred.remove(&ns)
    }

    /// Whether a call frame is still running in `ns`. The global namespace is
    /// never deferred (C compares against `nsPtr == globalNsPtr`), so its
    /// permanent frame does not count.
    #[must_use]
    pub(crate) fn namespace_is_active(&self, ns: NsId) -> bool {
        ns != GLOBAL && self.activations[ns] > 0
    }

    /// Retain a deleted token for the frames still running in it: the public
    /// parent edge goes immediately, so `namespace exists` and every absolute
    /// name stop resolving, but the command table, variables, children and
    /// exports stay exactly as they are until the last activation pops. No
    /// `dying_children` edge is recorded — unlike the synchronous window, the
    /// name is free for a fresh token straight away (C nulls `parentPtr`).
    pub(crate) fn defer_namespace(&mut self, ns: NsId) {
        self.dead.insert(ns);
        self.deferred.insert(ns);
        // C frees `unknownHandlerPtr` before it looks at the activation count.
        self.arena[ns].unknown = None;
        if let Some(parent) = self.arena[ns].parent {
            let fqn = self.qualified_name(ns);
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
            // C nulls `parentPtr`: the spelling is free for a wholly separate
            // token straight away, and this one answers from its own name.
            self.arena[ns].parent = None;
            self.arena[ns].retained_fqn = Some(fqn);
        }
    }

    /// Whether `ns` is a retained token, or lies inside one. A deferred token
    /// keeps its whole subtree, so the enclosing teardown must step over it.
    #[must_use]
    pub(crate) fn under_deferred_token(&self, ns: NsId) -> bool {
        let mut cur = Some(ns);
        while let Some(id) = cur {
            if self.deferred.contains(&id) {
                return true;
            }
            cur = self.arena[id].parent;
        }
        false
    }

    // -- per-namespace variable tables (the variable resolver's storage) ------

    /// For a **qualified** variable name, the `(namespace, simple tail)` it
    /// addresses, or `None` if that namespace doesn't exist. Absolute when
    /// `::`-led, else relative to `current` (`tclVar.c` /
    /// `namespace-tree.md` §5.3). Callers guard with `is_qualified` first,
    /// so `None` means *namespace missing*.
    ///
    /// Deliberately **not** the command rule ([`Self::home_of`]): variable
    /// resolution has no existence-checked fall-through — a qualified write
    /// creates the variable in the first namespace the qualifier resolves
    /// to — so this commits at namespace level.
    #[must_use]
    pub(crate) fn var_home(&self, current: NsId, name: &[u8]) -> Option<(NsId, Vec<u8>)> {
        let absolute = name.starts_with(b"::");
        let segments = split_qualifier(name);
        // C: a trailing `::` names the `{}` (empty) variable in the qualified
        // namespace — every segment is then a namespace component (the simple
        // name being `""`), unlike the usual "last segment is the var" split.
        let (simple, ns_parts): (Vec<u8>, &[&[u8]]) =
            if tcl_syntax::naming::ends_with_separator(name) {
                (Vec::new(), &segments[..])
            } else {
                let (s, parts) = segments.split_last()?;
                ((*s).to_vec(), parts)
            };
        let mut ns = if absolute { GLOBAL } else { current };
        for part in ns_parts {
            ns = *self.arena[ns].children.get(*part)?;
        }
        Some((ns, simple))
    }

    /// Sorted variable names in namespace `ns` (`info vars`/`globals`).
    #[must_use]
    pub(crate) fn var_names(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .vars
            .names()
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// `const` scalar names in `ns` (`info consts`).
    pub(crate) fn const_names(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .vars
            .const_names()
            .into_iter()
            .map(<[u8]>::to_vec)
            .collect()
    }

    /// Sorted names of the commands in `ns` that are procs (`info procs`).
    #[must_use]
    pub(crate) fn proc_names(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .commands
            .iter()
            .filter(|(_, c)| matches!(c, Command::Proc(_)))
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Namespace `ns`'s variable table (read).
    #[must_use]
    pub(crate) fn var_table(&self, ns: NsId) -> &VarTable {
        &self.arena[ns].vars
    }

    /// Namespace `ns`'s variable table (mutable).
    pub(crate) fn var_table_mut(&mut self, ns: NsId) -> &mut VarTable {
        &mut self.arena[ns].vars
    }

    /// The fully-qualified name of `ns` (`::a::b`; global is `::`).
    #[must_use]
    pub fn qualified_name(&self, ns: NsId) -> Vec<u8> {
        let mut parts: Vec<&[u8]> = Vec::new();
        let mut cur = ns;
        let mut out = loop {
            if let Some(fqn) = self.arena[cur].retained_fqn.as_deref() {
                break fqn.to_vec();
            }
            let Some(parent) = self.arena[cur].parent else {
                break Vec::new();
            };
            parts.push(&self.arena[cur].name);
            cur = parent;
        };
        if out.is_empty() && parts.is_empty() {
            return b"::".to_vec();
        }
        for part in parts.iter().rev() {
            out.extend_from_slice(b"::");
            out.extend_from_slice(part);
        }
        out
    }

    // -- the `namespace` command surface --------------------------------------

    /// The fully-qualified name a command `name` resolves to from `current`
    /// (`namespace which -command`), or `None` if it doesn't resolve.
    #[must_use]
    pub fn which_command(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        let (ns, simple) = self.home_of(current, name)?;
        let mut fqn = self.qualified_name(ns);
        if ns != GLOBAL {
            fqn.extend_from_slice(b"::"); // global's qualified_name is already `::`
        }
        fqn.extend_from_slice(&simple);
        Some(fqn)
    }

    /// `namespace export` — append a pattern (deduplicated). `-clear` first is the
    /// caller's job via [`clear_exports`](Self::clear_exports).
    pub fn export(&mut self, ns: NsId, pattern: &[u8]) {
        if !self.arena[ns].exports.iter().any(|p| p == pattern) {
            self.arena[ns].exports.push(pattern.to_vec());
        }
    }

    /// Drop all of `ns`'s export patterns (`namespace export -clear`).
    pub fn clear_exports(&mut self, ns: NsId) {
        self.arena[ns].exports.clear();
    }

    /// `ns`'s export patterns (`namespace export` with no args).
    #[must_use]
    pub fn exports(&self, ns: NsId) -> &[Vec<u8>] {
        &self.arena[ns].exports
    }

    /// The sorted command names in `ns` that match its export patterns — the
    /// default subcommand set of an ensemble over `ns` (`namespace ensemble`).
    #[must_use]
    pub fn exported_commands(&self, ns: NsId) -> Vec<Vec<u8>> {
        self.arena[ns]
            .commands
            .keys()
            .filter(|k| self.is_exported(ns, k))
            .cloned()
            .collect()
    }

    /// Does `name` match any of `ns`'s export patterns (`string match` glob)?
    #[must_use]
    pub fn is_exported(&self, ns: NsId, name: &[u8]) -> bool {
        let Ok(name_s) = core::str::from_utf8(name) else {
            return false;
        };
        self.arena[ns].exports.iter().any(|pat| {
            core::str::from_utf8(pat).is_ok_and(|p| tcl_syntax::glob::string_match(p, name_s))
        })
    }

    /// Bind `command` under simple `name` directly in namespace `ns` (no
    /// qualified parsing — `import` inserting a redirect into the importing ns).
    pub fn bind(&mut self, ns: NsId, name: &[u8], command: Command) {
        self.insert_bound(ns, name.to_vec(), command);
    }

    /// `ns`'s `namespace path` (the resolver's step-b list).
    #[must_use]
    pub fn path(&self, ns: NsId) -> &[NsId] {
        &self.arena[ns].path
    }

    /// `ns`'s parent (`namespace parent`); `None` only for the global root.
    #[must_use]
    pub fn parent(&self, ns: NsId) -> Option<NsId> {
        self.arena[ns].parent
    }

    /// `ns`'s direct child namespaces (`namespace children`).
    #[must_use]
    pub fn children(&self, ns: NsId) -> Vec<NsId> {
        let mut children: Vec<NsId> = self.arena[ns].children.values().copied().collect();
        children.sort_unstable();
        children
    }

    /// `ns`'s live children in Tcl string-hash enumeration order.
    #[must_use]
    pub fn children_hash_order(&self, ns: NsId) -> Vec<NsId> {
        self.arena[ns]
            .child_order
            .keys()
            .into_iter()
            .filter_map(|name| self.arena[ns].children.get(name).copied())
            .collect()
    }

    /// `ns`'s live command slots as `(simple name, generation)` in Tcl
    /// string-hash enumeration order — the snapshot `TclTeardownNamespace`
    /// takes of `cmdTable` before deleting each token.
    pub(crate) fn command_hash_order(&self, ns: NsId) -> Vec<(Vec<u8>, u64)> {
        self.arena[ns].commands.hash_order()
    }

    /// The token generation currently bound at `(ns, name)`. A teardown
    /// snapshot compares it before firing: a different generation means a
    /// delete callback replaced the token, so the replacement belongs to the
    /// next pass (C's `Tcl_DeleteCommandFromToken` returns early on
    /// `CMD_DYING`).
    pub(crate) fn command_generation(&self, ns: NsId, name: &[u8]) -> Option<u64> {
        self.arena[ns].commands.generation(name)
    }

    /// The generation of the command token `name` resolves to from `current`
    /// — the token identity a command trace is registered against, and the one
    /// a deletion frees the trace list of.
    pub(crate) fn resolve_generation(&self, current: NsId, name: &[u8]) -> Option<u64> {
        let (ns, simple) = self.home_of(current, name)?;
        self.arena[ns].commands.generation(&simple)
    }

    /// The fully-qualified name of the binding at `(ns, name)`.
    pub(crate) fn command_fqn_at(&self, ns: NsId, name: &[u8]) -> Vec<u8> {
        self.command_fqn(ns, name)
    }

    /// `ns`'s `namespace unknown` handler, if one is set (an empty/`None` handler
    /// means "use the interpreter default `::unknown`").
    #[must_use]
    pub(crate) fn unknown_handler(&self, ns: NsId) -> Option<&[u8]> {
        self.arena[ns].unknown.as_deref()
    }

    /// Set `ns`'s `namespace unknown` handler (an empty handler resets to default).
    pub(crate) fn set_unknown_handler(&mut self, ns: NsId, handler: &[u8]) {
        self.arena[ns].unknown = if handler.is_empty() {
            None
        } else {
            Some(handler.to_vec())
        };
    }

    /// Find every ensemble command whose configured namespace is in `victims`,
    /// returning each command's fully-qualified name and stable token.
    /// An ensemble command is tied to its namespace, so deleting the namespace
    /// deletes the command — even when the command itself lives elsewhere (e.g.
    /// `::ns` in the global table for an ensemble created inside `ns`). Mirrors
    /// C's ensemble namespace-deletion hook.
    pub(crate) fn ensembles_for(
        &self,
        victims: &std::collections::HashSet<NsId>,
    ) -> Vec<(Vec<u8>, std::rc::Rc<crate::ensemble::EnsembleToken>)> {
        let mut hits = Vec::new();
        for (id, node) in self.arena.iter().enumerate() {
            for (name, cmd) in node.commands.iter() {
                if let Command::Ensemble(token) = cmd {
                    if victims.contains(&token.config().ns) {
                        hits.push((self.command_fqn(id, name), std::rc::Rc::clone(token)));
                    }
                }
            }
        }
        hits
    }

    /// Remove one ensemble command by stable token identity wherever a delete
    /// callback may have renamed it, returning that binding's live FQN. A
    /// replacement at the old name carries a different token and survives.
    pub(crate) fn remove_ensemble_identity(
        &mut self,
        identity: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<Vec<u8>> {
        let mut found = None;
        for (ns, node) in self.arena.iter().enumerate() {
            if let Some(name) = node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Ensemble(current)
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| name.clone())
            }) {
                found = Some((ns, name));
                break;
            }
        }
        let (ns, name) = found?;
        self.arena[ns].commands.remove(&name);
        Some(self.command_fqn(ns, &name))
    }

    /// Imported aliases whose immediate source name is in `origins` or whose
    /// retained ensemble identity is in `tokens`. Each result includes the
    /// import binding's stable identity so callers can fire delete traces while
    /// it is still visible, then remove only that original command after any
    /// reentrant replacement performed by the callback.
    pub(crate) fn imports_for_origins(
        &self,
        origins: &std::collections::HashSet<Vec<u8>>,
        tokens: &[std::rc::Rc<crate::ensemble::EnsembleToken>],
    ) -> Vec<(Vec<u8>, std::rc::Rc<crate::interp::ImportToken>)> {
        let mut hits = Vec::new();
        for (id, node) in self.arena.iter().enumerate() {
            for (name, command) in node.commands.iter() {
                let Command::Imported {
                    source,
                    ensemble,
                    identity,
                } = command
                else {
                    continue;
                };
                let retains_token = ensemble.as_ref().is_some_and(|imported| {
                    tokens
                        .iter()
                        .any(|victim| std::rc::Rc::ptr_eq(imported, victim))
                });
                if origins.contains(source) || retains_token {
                    hits.push((self.command_fqn(id, name), std::rc::Rc::clone(identity)));
                }
            }
        }
        hits
    }

    /// Remove the imported command identified by `identity` wherever a
    /// delete-trace callback may have renamed it, returning that binding's live
    /// FQN. A replacement has a fresh identity and therefore survives.
    pub(crate) fn remove_import_identity(
        &mut self,
        identity: &std::rc::Rc<crate::interp::ImportToken>,
    ) -> Option<Vec<u8>> {
        let mut found = None;
        for (ns, node) in self.arena.iter().enumerate() {
            if let Some(name) = node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Imported { identity: current, .. }
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| name.clone())
            }) {
                found = Some((ns, name));
                break;
            }
        }
        let (ns, name) = found?;
        self.arena[ns].commands.remove(&name);
        Some(self.command_fqn(ns, &name))
    }

    /// `namespace delete name` — delete the namespace `qualified` resolves to
    /// (relative to `current`), with its child namespaces, commands, and
    /// variables. Returns `false` if it does not exist. The arena slot is
    /// tombstoned (contents cleared, unlinked from its parent) rather than
    /// removed, so the `NsId` indices of other namespaces stay valid.
    pub fn delete_namespace(&mut self, current: NsId, qualified: &[u8]) -> bool {
        let Some(ns) = self.find_namespace(current, qualified) else {
            return false;
        };
        self.delete_namespace_by_id(ns);
        true
    }

    /// Mark and detach one exact namespace token before its ordinary command
    /// callbacks. Descendants remain live and are reached through the retained
    /// parent edge until recursive teardown reaches each token in turn.
    pub(crate) fn begin_namespace_teardown(&mut self, ns: NsId) {
        self.dying.insert(ns);
        if ns != GLOBAL {
            self.dead.insert(ns);
        }
        if let Some(parent) = self.arena[ns].parent {
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
            self.dying_children.insert((parent, name), ns);
        }
    }

    /// Drop the temporary lookup edges after the final post-callback sweep.
    pub(crate) fn finish_namespace_teardown(&mut self, victims: &[NsId]) {
        let victim_set: BTreeSet<NsId> = victims.iter().copied().collect();
        self.dying.retain(|id| !victim_set.contains(id));
        self.dying_children.retain(|_, id| !victim_set.contains(id));
    }

    /// If `qualified` names a detached dying namespace, return its retained
    /// arena identity. Public lookup may traverse the same retained edge to a
    /// still-live child, but checks the final token's liveness; this helper
    /// specifically requires the final token itself to be dying.
    pub(crate) fn dying_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId> {
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.arena[ns]
                .children
                .get(part)
                .copied()
                .or_else(|| self.dying_children.get(&(ns, part.to_vec())).copied())?;
        }
        self.dying.contains(&ns).then_some(ns)
    }

    /// Delete the namespace `ns` (and its subtree), unlinking it from its parent.
    /// Deleting the global namespace clears its contents but keeps the node
    /// (it has no parent to unlink from) — matching `namespace delete ::`.
    pub fn delete_namespace_by_id(&mut self, ns: NsId) {
        let victims = self.descendant_ids(ns);
        // The global namespace is a permanent interpreter root: deleting it
        // clears its contents but does not invalidate its token. Every other
        // arena identity remains tombstoned after the temporary dying lookup
        // edges are dropped, including when the same spelling is recreated.
        self.dead
            .extend(victims.iter().copied().filter(|id| *id != GLOBAL));
        self.delete_subtree(ns);
        // Unlink from the parent so the name no longer resolves by lookup, but
        // keep the node's own `name`/`parent` intact: a call frame still active
        // in this (now dying) namespace must keep reporting its fully-qualified
        // name from `namespace current` until it pops (C keeps the dying
        // `Namespace` alive via its activation count — namespace-7.1).
        if let Some(parent) = self.arena[ns].parent {
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
        }
        // Drop the deleted namespaces from every other namespace's `namespace
        // path` — a dangling id would otherwise resolve to the global `::`
        // (`TclResetNamespaceParameters` / path fixup in `tclNamesp.c`).
        for node in &mut self.arena {
            if !node.path.is_empty() {
                node.path.retain(|p| !victims.contains(p));
            }
        }
    }

    /// `ns` and all of its descendant namespace ids (for destroying every OO
    /// object whose instance namespace lies in a namespace being deleted).
    #[must_use]
    pub fn descendant_ids(&self, ns: NsId) -> Vec<NsId> {
        let mut out = vec![ns];
        let mut i = 0;
        while i < out.len() {
            for child in self.children(out[i]) {
                out.push(child);
            }
            i += 1;
        }
        out
    }

    /// Fully-qualified command bindings in an explicit set of retained arena
    /// nodes, including tokens already detached from their former parent during
    /// teardown.
    #[must_use]
    pub(crate) fn command_fqns_in_ids(&self, ids: &[NsId]) -> Vec<Vec<u8>> {
        self.command_slots_in_ids(ids)
            .into_iter()
            .map(|(id, tail)| self.command_fqn(id, &tail))
            .collect()
    }

    /// The same bindings as `(namespace, simple name)` pairs. A retained token
    /// and a same-named recreation share every fully-qualified name they hold,
    /// so a teardown that must fire exactly one of them addresses the slot.
    pub(crate) fn command_slots_in_ids(&self, ids: &[NsId]) -> Vec<(NsId, Vec<u8>)> {
        let mut slots = Vec::new();
        for &id in ids {
            slots.extend(
                self.arena[id]
                    .commands
                    .hash_order()
                    .into_iter()
                    .map(|(name, _)| (id, name)),
            );
        }
        slots
    }

    /// Clear callback-created state from explicit dying arena nodes. The
    /// caller has already fired command traces and removed dependent imports.
    pub(crate) fn clear_namespace_ids(&mut self, ids: &[NsId]) {
        for &id in ids.iter().rev() {
            let n = &mut self.arena[id];
            n.children.clear();
            n.child_order.clear();
            for command in n.commands.values() {
                Self::mark_command_deleted(command);
            }
            n.commands.clear();
            n.path.clear();
            n.exports.clear();
            n.vars = VarTable::default();
            n.unknown = None;
        }
    }

    /// Clear one dying namespace's own variable table and drop it from every
    /// `namespace path`, while retaining its child links and metadata for the
    /// recursive delete callbacks still to run. The command table is emptied
    /// one token at a time afterwards, in hash order; the final fixed-point
    /// sweep clears the retained metadata and links.
    pub(crate) fn clear_namespace_token(&mut self, ns: NsId) {
        self.arena[ns].vars = VarTable::default();
        for node in &mut self.arena {
            node.path.retain(|entry| *entry != ns);
        }
    }

    /// Recursively clear a namespace and its descendants: dropping the `VarTable`
    /// releases the variables' object references (`TclFreeVar`); commands and
    /// child links are dropped.
    fn delete_subtree(&mut self, ns: NsId) {
        for child in self.children(ns) {
            self.delete_subtree(child);
        }
        let n = &mut self.arena[ns];
        n.children.clear();
        n.child_order.clear();
        for command in n.commands.values() {
            Self::mark_command_deleted(command);
        }
        n.commands.clear();
        // Keep namespace metadata alive through delete callbacks. The node is
        // detached, so introspection cannot see it, but command-token operations
        // such as importing a newly-created command still consult its export
        // patterns/path until the final dying-namespace sweep clears them.
        n.vars = VarTable::default(); // drop → release variable refcounts
    }

    /// The `(simple_name, source_fqn)` of every imported redirect in `ns`
    /// (`namespace forget` walks these).
    #[must_use]
    pub fn imported_in(&self, ns: NsId) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.arena[ns]
            .commands
            .iter()
            .filter_map(|(k, c)| match c {
                Command::Imported {
                    source, ensemble, ..
                } => Some((
                    k.clone(),
                    ensemble
                        .as_ref()
                        .filter(|token| !token.is_deleted())
                        .map_or_else(|| source.clone(), |token| token.name()),
                )),
                _ => None,
            })
            .collect()
    }

    /// Whether the immediate-source chain beginning at `source_fqn` reaches
    /// `needle_fqn`. `namespace import -force` consults this before replacing
    /// the destination binding: installing that edge would otherwise close an
    /// ImportRef cycle. Normal construction keeps the graph acyclic; the
    /// visited set makes the invariant check finite for malformed legacy state.
    pub(crate) fn import_chain_contains(&self, source_fqn: &[u8], needle_fqn: &[u8]) -> bool {
        let mut current = source_fqn.to_vec();
        let mut visited = BTreeSet::new();
        while visited.insert(current.clone()) {
            if current == needle_fqn {
                return true;
            }
            let Some(Command::Imported {
                source, ensemble, ..
            }) = self.resolve(GLOBAL, &current)
            else {
                return false;
            };
            current = ensemble
                .filter(|token| !token.is_deleted())
                .map_or(source, |token| token.name());
        }
        false
    }

    /// Remove the simple-named command `name` directly from `ns` (no resolution
    /// walk); returns whether it existed. For `namespace forget`.
    pub fn remove_in(&mut self, ns: NsId, name: &[u8]) -> bool {
        self.arena[ns].commands.remove(name).is_some()
    }

    // -- helpers --------------------------------------------------------------

    /// Locate the namespace + simple name that *holds* the binding `name`
    /// resolves to, following C Tcl's full command-resolution order
    /// (`Tcl_FindCommand`, `generic/tclNamesp.c`). The shared core of
    /// `resolve`/`delete`/`rename`.
    ///
    /// Structural mirror of the canonical
    /// [`tcl_syntax::naming::resolve_command_with`] rule (this table is a
    /// namespace tree, not a flat string map, so the loop walks base
    /// namespaces instead of joining candidate strings — conformance is
    /// pinned by the shared vector suite,
    /// `rust/tcl-syntax/tests/data/command_resolution_vectors.txt`):
    ///
    /// * An absolute name resolves from the global namespace only.
    /// * Any relative name — bare (`helper`) *or* qualifier-carrying
    ///   (`inner::p`) — tries the current namespace, then each
    ///   `namespace path` entry in order, then global, dispatching the
    ///   first base under which the **command exists**.  A qualifier
    ///   namespace merely existing does not commit resolution: `inner::p`
    ///   from `::outer` reaches `::inner::p` even when the namespace
    ///   `::outer::inner` exists but holds no `p` (tclsh 8.6/9.0
    ///   confirmed).
    fn home_of(&self, current: NsId, name: &[u8]) -> Option<(NsId, Vec<u8>)> {
        let segments = split_qualifier(name);
        // A name ending in a separator run — or consisting only of colons, or
        // empty — names the empty-string `{}` command in the qualified
        // namespace; `qualifier_segments` drops that empty tail, so restore
        // it (#934: with `proc {} {} {}` defined, `::` and `:::` both
        // dispatch it, tclsh 8.6/9.0-pinned).
        let (simple, ns_parts): (&[u8], &[&[u8]]) = if ends_with_separator(name) || name.is_empty()
        {
            (b"", &segments[..])
        } else {
            let (simple, ns_parts) = segments.split_last()?;
            (*simple, ns_parts)
        };
        // Walk `ns_parts` from `base`, then require the command itself.
        let find_under = |base: NsId| -> Option<NsId> {
            let mut ns = base;
            for part in ns_parts {
                ns = self.arena[ns]
                    .children
                    .get(*part)
                    .copied()
                    .or_else(|| self.dying_children.get(&(ns, part.to_vec())).copied())?;
            }
            if self.arena[ns].commands.contains_key(simple) {
                Some(ns)
            } else {
                None
            }
        };
        if name.starts_with(b"::") {
            return find_under(GLOBAL).map(|ns| (ns, simple.to_vec()));
        }
        if let Some(ns) = find_under(current) {
            return Some((ns, simple.to_vec()));
        }
        for &p in &self.arena[current].path {
            if let Some(ns) = find_under(p) {
                return Some((ns, simple.to_vec()));
            }
        }
        if current != GLOBAL {
            if let Some(ns) = find_under(GLOBAL) {
                return Some((ns, simple.to_vec()));
            }
        }
        None
    }

    fn ensure_child(&mut self, parent: NsId, name: &[u8]) -> NsId {
        if let Some(&id) = self.arena[parent].children.get(name) {
            return id;
        }
        let id = self.arena.len();
        self.arena.push(Namespace::new(name.to_vec(), Some(parent)));
        self.activations.push(0);
        self.arena[parent].children.insert(name.to_vec(), id);
        self.arena[parent].child_order.insert(name);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interp::Code;

    fn dummy(_i: &mut crate::interp::Interp, _a: &[*mut crate::obj::TclObj]) -> Code {
        Code::Ok
    }
    fn cmd() -> Command {
        Command::Builtin(dummy)
    }
    fn is_some(c: Option<Command>) -> bool {
        c.is_some()
    }

    #[test]
    fn unqualified_resolves_in_global() {
        let mut ns = Namespaces::new();
        ns.register(b"set", cmd());
        assert!(is_some(ns.resolve(GLOBAL, b"set")));
        assert!(!is_some(ns.resolve(GLOBAL, b"nope")));
    }

    #[test]
    fn qualified_registration_and_resolution() {
        let mut ns = Namespaces::new();
        ns.register(b"::tcl::mathfunc::sin", cmd());
        ns.register(b"set", cmd());
        let mf = ns.find_namespace(GLOBAL, b"::tcl::mathfunc").unwrap();
        // qualified lookup hits the namespace directly
        assert!(is_some(ns.resolve(GLOBAL, b"::tcl::mathfunc::sin")));
        // `sin` is NOT visible unqualified from global
        assert!(!is_some(ns.resolve(GLOBAL, b"sin")));
        // an absolute reference to a global command works
        assert!(is_some(ns.resolve(mf, b"::set")));
        assert_eq!(ns.qualified_name(mf), b"::tcl::mathfunc");
    }

    #[test]
    fn namespace_path_fallback() {
        let mut ns = Namespaces::new();
        ns.register(b"::tcl::mathop::+", cmd());
        let mathop = ns.find_namespace(GLOBAL, b"::tcl::mathop").unwrap();
        let foo = ns.ensure_namespace(GLOBAL, b"::foo");
        // bare `+` is not resolvable from ::foo …
        assert!(!is_some(ns.resolve(foo, b"+")));
        // … until ::tcl::mathop is on ::foo's namespace path.
        ns.set_path(foo, vec![mathop]);
        assert!(is_some(ns.resolve(foo, b"+")));
    }
}
