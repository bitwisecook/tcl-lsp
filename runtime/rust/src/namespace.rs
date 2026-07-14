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

use std::collections::BTreeMap;

use tcl_syntax::naming::{
    is_qualified as contains_qualifier, qualifier_segments as split_qualifier,
};

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
}

/// One namespace: its simple name, its command table, child namespaces, the
/// `namespace path` search list, and `export` patterns.
struct Namespace {
    /// Simple name (e.g. `mathfunc`); the global namespace's is empty.
    name: Vec<u8>,
    parent: Option<NsId>,
    children: BTreeMap<Vec<u8>, NsId>,
    commands: BTreeMap<Vec<u8>, Command>,
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
}

impl Namespace {
    fn new(name: Vec<u8>, parent: Option<NsId>) -> Namespace {
        Namespace {
            name,
            parent,
            children: BTreeMap::new(),
            commands: BTreeMap::new(),
            path: Vec::new(),
            exports: Vec::new(),
            vars: VarTable::default(),
            unknown: None,
        }
    }
}

/// The namespace tree + the command resolver.
pub struct Namespaces {
    arena: Vec<Namespace>,
}

impl Default for Namespaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespaces {
    /// A fresh tree with just the global namespace `::`.
    #[must_use]
    pub fn new() -> Namespaces {
        Namespaces {
            arena: vec![Namespace::new(Vec::new(), None)],
        }
    }

    /// Register `command` under `name` (possibly qualified), creating any
    /// intermediate namespaces. A qualified `name` is rooted at global.
    pub fn register(&mut self, name: &[u8], command: Command) {
        let segments = split_qualifier(name);
        let Some((simple, ns_parts)) = segments.split_last() else {
            return; // empty / `::` only — nothing to register
        };
        let mut ns = GLOBAL;
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        self.arena[ns].commands.insert((*simple).to_vec(), command);
    }

    /// The single command resolver, `resolve(currentNs, name)` (A2). Returns a
    /// **clone** of the small command handle (a fn-pointer for `Builtin`; the
    /// target + frozen prefix for `Alias`) so the caller can dispatch without
    /// holding a borrow on the table.
    #[must_use]
    pub fn resolve(&self, current: NsId, name: &[u8]) -> Option<Command> {
        self.home_of(current, name)
            .map(|(ns, simple)| self.arena[ns].commands[&simple].clone())
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
                self.arena[ns].commands.insert(simple, command);
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
            Some((ns, simple)) => self.arena[ns].commands.remove(&simple).is_some(),
            None => false,
        }
    }

    /// `rename old new`: move the command bound to `old` to `new` (both resolved
    /// relative to `current`, absolute when `::`-led); `new == ""` deletes it.
    /// A self-rename is a no-op (remove-then-reinsert under the same key).
    ///
    /// Built-in protection lives in the `rename` builtin, not here — this is the
    /// pure table operation.
    pub fn rename(&mut self, current: NsId, old: &[u8], new: &[u8]) -> RenameOutcome {
        let Some((old_ns, old_simple)) = self.home_of(current, old) else {
            return RenameOutcome::NoSuchCommand;
        };
        // SAFETY of unwrap: `home_of` reported the binding exists.
        let cmd = self.arena[old_ns].commands.remove(&old_simple).unwrap();
        if new.is_empty() {
            return RenameOutcome::Deleted;
        }
        let absolute = new.starts_with(b"::");
        let segments = split_qualifier(new);
        let Some((simple, ns_parts)) = segments.split_last() else {
            // `new` was `::`-only — nothing nameable; put the command back.
            self.arena[old_ns].commands.insert(old_simple, cmd);
            return RenameOutcome::NoSuchCommand;
        };
        let mut ns = if absolute { GLOBAL } else { current };
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        self.arena[ns].commands.insert((*simple).to_vec(), cmd);
        RenameOutcome::Renamed
    }

    /// Every alias command's fully-qualified name across the tree (`interp
    /// aliases`). Global aliases keep their simple name (aliases are registered
    /// interpreter-wide); namespaced ones are qualified.
    #[must_use]
    pub fn alias_names(&self) -> Vec<Vec<u8>> {
        let mut found: Vec<(NsId, Vec<u8>)> = Vec::new();
        for (id, ns) in self.arena.iter().enumerate() {
            for (key, cmd) in &ns.commands {
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
        if let Some((_tail, ns_parts)) = segments.split_last() {
            for part in ns_parts {
                ns = self.ensure_child(ns, part);
            }
        }
        ns
    }

    /// Find (creating if needed) the namespace named `qualified`, rooted at
    /// `current` (absolute if it leads with `::`). For `namespace eval`.
    pub fn ensure_namespace(&mut self, current: NsId, qualified: &[u8]) -> NsId {
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = self.ensure_child(ns, part);
        }
        ns
    }

    /// Resolve `qualified` to an existing namespace, or `None`.
    #[must_use]
    pub fn find_namespace(&self, current: NsId, qualified: &[u8]) -> Option<NsId> {
        let absolute = qualified.starts_with(b"::");
        let mut ns = if absolute { GLOBAL } else { current };
        for part in split_qualifier(qualified) {
            ns = *self.arena[ns].children.get(part)?;
        }
        Some(ns)
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

    /// The simple (unqualified) name of `ns` — its last component (`::a::b` →
    /// `b`); empty for the global namespace. C's `Namespace.name`.
    #[must_use]
    pub(crate) fn simple_name(&self, ns: NsId) -> Vec<u8> {
        self.arena[ns].name.clone()
    }

    /// The fully-qualified name of `ns` (`::a::b`; global is `::`).
    #[must_use]
    pub fn qualified_name(&self, ns: NsId) -> Vec<u8> {
        let mut parts: Vec<&[u8]> = Vec::new();
        let mut cur = ns;
        while let Some(parent) = self.arena[cur].parent {
            parts.push(&self.arena[cur].name);
            cur = parent;
        }
        let mut out = Vec::new();
        if parts.is_empty() {
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

    /// The fully-qualified name a *variable* `name` resolves to from `current`
    /// (`namespace which -variable`), or `None`. Like Tcl's
    /// `Tcl_FindNamespaceVar`, this resolves `name` as a **namespace** variable
    /// (ignoring local proc links) and requires it to be declared/set in the
    /// target namespace's variable table.
    #[must_use]
    pub(crate) fn which_variable(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        let (ns, simple) = if contains_qualifier(name) {
            self.var_home(current, name)?
        } else {
            (current, name.to_vec())
        };
        self.arena.get(ns)?.vars.cell(&simple)?;
        let mut fqn = self.qualified_name(ns);
        if ns != GLOBAL {
            fqn.extend_from_slice(b"::"); // global's qualified_name is already `::`
        }
        fqn.extend_from_slice(&simple);
        Some(fqn)
    }

    /// `namespace origin` — the fully-qualified name of the *original* command
    /// `name` resolves to, following `import` chains to their source. `None` if
    /// the command doesn't resolve.
    pub fn command_origin(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        let mut fqn = self.which_command(current, name)?;
        // Follow imported commands to their source (bounded against cycles).
        for _ in 0..64 {
            match self.resolve(current, &fqn) {
                Some(Command::Imported { source }) => fqn = source,
                _ => break,
            }
        }
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
        self.arena[ns].commands.insert(name.to_vec(), command);
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
        self.arena[ns].children.values().copied().collect()
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

    /// Remove every ensemble command whose configured namespace is in `victims`,
    /// returning each removed command's fully-qualified name. An ensemble command
    /// is tied to its namespace, so deleting the namespace deletes the command —
    /// even when the command itself lives elsewhere (e.g. `::ns` in the global
    /// table for an ensemble created inside `ns`). Mirrors C's ensemble
    /// namespace-deletion hook.
    pub(crate) fn remove_ensembles_for(
        &mut self,
        victims: &std::collections::HashSet<NsId>,
    ) -> Vec<Vec<u8>> {
        // Collect first (immutable borrow), then unbind (mutable).
        let mut hits: Vec<(NsId, Vec<u8>)> = Vec::new();
        for (id, node) in self.arena.iter().enumerate() {
            for (name, cmd) in &node.commands {
                if let Command::Ensemble(cfg) = cmd {
                    if victims.contains(&cfg.ns) {
                        hits.push((id, name.clone()));
                    }
                }
            }
        }
        let mut fqns = Vec::with_capacity(hits.len());
        for (id, name) in hits {
            self.arena[id].commands.remove(&name);
            let mut fqn = self.qualified_name(id);
            if fqn != b"::" {
                fqn.extend_from_slice(b"::");
            }
            fqn.extend_from_slice(&name);
            fqns.push(fqn);
        }
        fqns
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

    /// Delete the namespace `ns` (and its subtree), unlinking it from its parent.
    /// Deleting the global namespace clears its contents but keeps the node
    /// (it has no parent to unlink from) — matching `namespace delete ::`.
    pub fn delete_namespace_by_id(&mut self, ns: NsId) {
        let victims = self.descendant_ids(ns);
        self.delete_subtree(ns);
        // Unlink from the parent so the name no longer resolves by lookup, but
        // keep the node's own `name`/`parent` intact: a call frame still active
        // in this (now dying) namespace must keep reporting its fully-qualified
        // name from `namespace current` until it pops (C keeps the dying
        // `Namespace` alive via its activation count — namespace-7.1).
        if let Some(parent) = self.arena[ns].parent {
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
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

    /// Recursively clear a namespace and its descendants: dropping the `VarTable`
    /// releases the variables' object references (`TclFreeVar`); commands and
    /// child links are dropped.
    fn delete_subtree(&mut self, ns: NsId) {
        for child in self.children(ns) {
            self.delete_subtree(child);
        }
        let n = &mut self.arena[ns];
        n.children.clear();
        n.commands.clear();
        n.path.clear();
        n.exports.clear();
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
                Command::Imported { source } => Some((k.clone(), source.clone())),
                _ => None,
            })
            .collect()
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
        let (simple, ns_parts) = segments.split_last()?;
        // Walk `ns_parts` from `base`, then require the command itself.
        let find_under = |base: NsId| -> Option<NsId> {
            let mut ns = base;
            for part in ns_parts {
                ns = *self.arena[ns].children.get(*part)?;
            }
            if self.arena[ns].commands.contains_key(*simple) {
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
        self.arena[parent].children.insert(name.to_vec(), id);
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
