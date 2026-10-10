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

//! Command namespaces — the command-table-as-core-service.
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
use tcl_core_types::OoId;
use tcl_syntax::naming::{ends_with_separator, qualifier_segments as split_qualifier};

use crate::frame::VarTable;
use crate::interp::{BuiltinFn, Command, OoCommandRole};

mod jim_local;
#[cfg(test)]
mod jim_table_key_tests;
mod native_namespace_name;

/// An index into the namespace arena. The global namespace `::` is always 0.
pub type NsId = usize;

/// The global namespace `::`.
pub const GLOBAL: NsId = 0;

static JIM_CONTEXT_PATH: tcl_core_types::ByteNamespacePath =
    tcl_core_types::ByteNamespacePath::root();

/// The result of a [`crate::interp::Interp::rename_command`].
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
    /// left untouched.
    TargetExists,
}

/// The half-finished `rename` a [`Namespaces::publish_rename_destination`]
/// leaves behind: the command stands under *both* names until
/// [`Namespaces::retire_rename_source`] drops the source's table entry.
pub(crate) struct RenamePublication {
    /// The source table slot — C's `oldHPtr`, which `TclRenameCommand` holds
    /// across the `rename` traces and deletes at the very end.
    source: (NsId, Vec<u8>),
    /// The destination's fully-qualified name: the key the command's traces,
    /// imports and TclOO registration move to, and the callbacks' second word.
    pub(crate) destination_fqn: Vec<u8>,
    /// Stable identity of the command token being moved. Rename, hide, and
    /// expose change placement without minting a new token.
    pub(crate) generation: u64,
}

/// One command token and the interpreter-unique generation that owns all of
/// its sidecars. Placements may move between namespace and hidden tables, but
/// this pair moves intact.
#[derive(Clone)]
pub(crate) struct CommandBinding {
    pub(crate) generation: u64,
    pub(crate) command: Command,
    pub(crate) jim_table_key: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
}

/// One namespace's command table: the `BTreeMap` the resolver looks names up
/// in, plus the retained `TCL_STRING_KEYS` bucket order used by namespace
/// teardown and a per-slot generation.
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
    entries: BTreeMap<Vec<u8>, CommandBinding>,
    order: TclStringHashOrder,
}

impl CommandTable {
    fn get(&self, key: &[u8]) -> Option<&Command> {
        self.entries.get(key).map(|binding| &binding.command)
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
            .map(|(key, binding)| (key, &binding.command))
    }

    fn values(&self) -> impl Iterator<Item = &Command> {
        self.entries.values().map(|binding| &binding.command)
    }

    fn values_mut(&mut self) -> impl Iterator<Item = &mut Command> {
        self.entries
            .values_mut()
            .map(|binding| &mut binding.command)
    }

    /// Bind `command` at `key`, returning whatever it displaced. A live key is
    /// re-created at its bucket head, as C's `TclCreateObjCommandInNs` does
    /// when it deletes the old hash entry and creates a fresh one.
    fn insert(
        &mut self,
        key: Vec<u8>,
        command: Command,
        generation: u64,
        jim_table_key: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    ) -> Option<Command> {
        match self.entries.insert(
            key.clone(),
            CommandBinding {
                generation,
                command,
                jim_table_key,
            },
        ) {
            Some(displaced) => {
                self.order.reinsert(&key);
                Some(displaced.command)
            }
            None => {
                self.order.insert(&key);
                None
            }
        }
    }

    /// Remove `key` while preserving the command token generation for a
    /// placement move such as hide/expose. The bucket capacity is retained.
    fn remove_binding(&mut self, key: &[u8]) -> Option<CommandBinding> {
        let binding = self.entries.remove(key)?;
        self.order.remove(key);
        Some(binding)
    }

    /// Take a slot's binding out while leaving its hash entry in place —
    /// C's alias-loop probe (`TclRenameCommand`) reassigns `cmdPtr->hPtr` and
    /// undoes the move without ever deleting the source's entry.
    fn take_slot(&mut self, key: &[u8]) -> Option<CommandBinding> {
        self.entries.remove(key)
    }

    /// Put a slot taken by [`Self::take_slot`] back, creating the hash entry
    /// when the probe's destination did not already have one.
    fn restore_slot(&mut self, key: Vec<u8>, slot: CommandBinding) -> Option<CommandBinding> {
        self.order.insert(&key);
        self.entries.insert(key, slot)
    }

    /// Swap `key`'s binding without disturbing its hash entry, so the slot
    /// keeps its generation and its bucket position. C's `TclRenameCommand`
    /// never touches the source entry until the very end, but it does re-home
    /// the one `Command` both entries point at, so the vacating name reports
    /// the destination's namespace for the callbacks' duration.
    fn rebind_slot(&mut self, key: &[u8], command: Command) {
        if let Some(binding) = self.entries.get_mut(key) {
            binding.command = command;
        }
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
        self.entries.get(key).map(|binding| binding.generation)
    }

    /// Live `(name, generation)` slots in original `Tcl_FirstHashEntry` order.
    /// The selected teardown recipe chooses one entry or the complete frontier.
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
enum JimNamespaceObject {
    Initial(std::rc::Rc<crate::obj::Owned>),
    Published(std::rc::Weak<crate::obj::Owned>),
}
impl JimNamespaceObject {
    fn owner(&self) -> Option<std::rc::Rc<crate::obj::Owned>> {
        match self {
            Self::Initial(owner) => Some(std::rc::Rc::clone(owner)),
            Self::Published(owner) => owner.upgrade(),
        }
    }
}

struct Namespace {
    /// Actual Jim canonical namespace holder, separate from C tree edges.
    jim_namespace: Option<(JimNamespaceObject, std::rc::Rc<[u8]>)>,
    /// Simple name (e.g. `mathfunc`); the global namespace's is empty.
    name: Vec<u8>,
    parent: Option<NsId>,
    /// Original constructed C address, retained independently of public edges
    /// and the diagnostic full-name spelling after namespace deletion.
    constructed_path: tcl_core_types::ByteNamespacePath,
    children: BTreeMap<Vec<u8>, NsId>,
    /// The child's `TCL_STRING_KEYS` table, including retained resize history.
    child_order: TclStringHashOrder,
    commands: CommandTable,
    /// Native command-reference invalidation, independent of compiler guards.
    command_reference_epoch: u64,
    ensemble_export_epoch: u64,
    native_resolver_epoch: Option<u64>,
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
    unknown: Option<crate::obj::Owned>,
    /// The name a retained token keeps reporting once its parent edge is gone.
    /// C's deferred deletion nulls `parentPtr` — freeing the name for a fresh
    /// token — but leaves `fullName` alone, so `namespace current` in the
    /// frames still holding the token is unchanged.
    retained_fqn: Option<Vec<u8>>,
}

impl Namespace {
    fn new(name: Vec<u8>, parent: Option<NsId>) -> Namespace {
        Namespace {
            jim_namespace: None,
            name,
            parent,
            constructed_path: tcl_core_types::ByteNamespacePath::root(),
            children: BTreeMap::new(),
            child_order: TclStringHashOrder::default(),
            commands: CommandTable::default(),
            command_reference_epoch: 0,
            ensemble_export_epoch: 0,
            native_resolver_epoch: Some(0),
            path: Vec::new(),
            exports: Vec::new(),
            vars: VarTable::default(),
            unknown: None,
            retained_fqn: None,
        }
    }
}

/// The raw compiler copied at native command publication. Callable lookup is
/// independent: imports may follow another implementation after this copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NativeCompilerRecipe {
    Absent,
    Unknown,
    Registered {
        registration: Vec<u8>,
        spec: tcl_registry::native_compilation::NativeCompilationSpec,
    },
    ProcedureNoOp,
}

/// Metadata for a native command node. No worker or procedure is retained.
struct NativeCommandNode {
    epoch: u64,
    compiler_hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    compiler_recipe: NativeCompilerRecipe,
    compiler_handler: Option<BuiltinFn>,
    placement: Option<(NsId, Vec<u8>)>,
}

/// The original creation entry, retained across re-entrant delete callbacks.
pub(crate) struct NativeCommandCreation {
    namespace: NsId,
    occupied: bool,
    jim_table_key: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
}

/// The namespace tree + the command resolver.
pub(crate) struct RetiredJimCommands {
    _tables: Vec<CommandTable>,
    _nodes: BTreeMap<u64, NativeCommandNode>,
    _previous: BTreeMap<u64, CommandBinding>,
}

pub struct Namespaces {
    arena: Vec<Namespace>,
    native_namespace_names: std::cell::RefCell<
        BTreeMap<NsId, tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken>,
    >,
    native_namespace_name_dead: BTreeSet<NsId>,
    native_command_nodes: BTreeMap<u64, NativeCommandNode>,
    retired_ensemble_roles: std::cell::RefCell<Vec<std::rc::Rc<crate::ensemble::EnsembleToken>>>,
    native_command_version: Option<tcl_dialect::TclVersion>,
    native_compiler_epoch: Option<u64>,
    jim_previous_commands: BTreeMap<u64, CommandBinding>,
    jim_previous_locations: BTreeMap<u64, (NsId, Vec<u8>)>,
    jim_upcalls: BTreeMap<u64, usize>,
    jim_procedure_epoch: u64,
    jim_retired_commands: usize,
    jim_active_commands: BTreeMap<u64, usize>,
    jim_retired_tokens: BTreeSet<u64>,
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
    retired_variable_namespaces: BTreeSet<NsId>,
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
    /// Tcl 8.x resolves an unqualified variable at **namespace scope**
    /// to the global variable when the namespace has none but the global
    /// namespace does (reads and writes both); 9.0 removed the fallback
    /// (TIP 278, `TCL_NAMESPACE_ONLY`).  Defaults to the 9.0 behaviour
    /// (`false`); an 8.x embedding flips it via
    /// [`crate::interp::Interp::set_runtime_version`].
    pub(crate) ns_var_global_fallback: bool,
    pub(crate) variable_container_model: tcl_dialect::VariableContainerModel,
    pub(crate) variable_hash_recipe: Option<tcl_core_types::NativeHashRecipe>,
    pub(crate) variable_lookup_policy: tcl_dialect::VariableLookupPolicy,
    /// Selected byte-name recipe; absence is a host capability withdrawal.
    pub(crate) variable_name_protocol: Option<tcl_syntax::naming::NativeNameProtocol>,
    pub(crate) variable_string_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    pub(crate) execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    pub(crate) variable_link_binding: tcl_dialect::VariableLinkBinding,
}

impl Default for Namespaces {
    fn default() -> Self {
        Self::new()
    }
}

impl Namespaces {
    pub(crate) fn take_jim_frame_owners(
        &mut self,
    ) -> (Vec<std::rc::Rc<crate::obj::Owned>>, Vec<VarTable>) {
        let mut namespaces = Vec::new();
        let mut variables = Vec::new();
        for namespace in &mut self.arena {
            if let Some((JimNamespaceObject::Initial(object), _)) = namespace.jim_namespace.take() {
                namespaces.push(object);
            }
            variables.push(std::mem::take(&mut namespace.vars));
        }
        (namespaces, variables)
    }

    pub(crate) fn take_jim_command_owners(&mut self) -> RetiredJimCommands {
        RetiredJimCommands {
            _tables: self
                .arena
                .iter_mut()
                .map(|namespace| std::mem::take(&mut namespace.commands))
                .collect(),
            _nodes: std::mem::take(&mut self.native_command_nodes),
            _previous: std::mem::take(&mut self.jim_previous_commands),
        }
    }

    pub(crate) fn retire_jim_procedure_epoch(&mut self) {
        self.advance_jim_procedure_epoch();
    }

    /// Retain command-binding transports for C interpreter retirement.
    /// Native roles are withdrawn after the namespace owner is released.
    pub(crate) fn c_procedure_bindings_for_retirement(
        &self,
    ) -> Option<Vec<crate::interp::NativeProcedureCommand>> {
        self.native_command_version?;
        Some(
            self.arena
                .iter()
                .flat_map(|namespace| namespace.commands.values())
                .filter_map(|command| match command {
                    Command::Proc(procedure) => Some(procedure.clone()),
                    _ => None,
                })
                .collect(),
        )
    }

    pub(crate) fn native_command_generations(&self) -> Vec<u64> {
        self.native_command_nodes.keys().copied().collect()
    }

    /// Actual compiler attachment on a retained raw token, independent of
    /// origin redirects and current handler semantics.
    pub(crate) fn native_compiler_hook(
        &self,
        generation: u64,
    ) -> Option<tcl_runtime_api::native_compilation::NativeCompilerHookPresence> {
        self.native_command_nodes
            .get(&generation)
            .map(|node| node.compiler_hook)
    }

    /// Install an actual compiler attachment. C's procedure NoOp installation
    /// does not itself advance the interpreter compiler epoch. Existing imports
    /// retain the attachment they copied when their own token was created.
    #[cfg(test)]
    pub(crate) fn set_native_compiler_hook(
        &mut self,
        generation: u64,
        hook: tcl_runtime_api::native_compilation::NativeCompilerHookPresence,
    ) {
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.compiler_hook = hook;
            node.compiler_handler = None;
            node.compiler_recipe = match hook {
                tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent => {
                    NativeCompilerRecipe::Absent
                }
                _ => NativeCompilerRecipe::Unknown,
            };
        }
    }

    pub(crate) fn native_compiler_recipe(&self, generation: u64) -> Option<NativeCompilerRecipe> {
        self.native_command_nodes
            .get(&generation)
            .map(|node| node.compiler_recipe.clone())
    }

    pub(crate) fn set_native_compiler_recipe(
        &mut self,
        generation: u64,
        recipe: NativeCompilerRecipe,
    ) {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.compiler_hook = match recipe {
                NativeCompilerRecipe::Absent => Hook::Absent,
                NativeCompilerRecipe::Unknown => Hook::Unknown,
                NativeCompilerRecipe::Registered { .. } | NativeCompilerRecipe::ProcedureNoOp => {
                    Hook::Present
                }
            };
            node.compiler_recipe = recipe;
            node.compiler_handler = None;
        }
    }
    pub(crate) fn native_compiler_handler(&self, generation: u64) -> Option<BuiltinFn> {
        self.native_command_nodes.get(&generation)?.compiler_handler
    }
    pub(crate) fn set_native_compiler_handler(
        &mut self,
        generation: u64,
        handler: Option<BuiltinFn>,
    ) {
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.compiler_handler = handler;
        }
    }

    /// Apply the shared native invalidation policy at its actual mutation door.
    /// Namespace resolver epochs and the interpreter compiler epoch are separate.
    pub(crate) fn note_native_compiler_mutation(
        &mut self,
        namespace: Option<NsId>,
        mutation: tcl_registry::native_procedure::NativeCompilerCacheMutation,
    ) {
        let Some(version) = self.native_command_version else {
            return;
        };
        let invalidated = tcl_registry::native_procedure::native_compiler_cache_invalidated(
            tcl_registry::InvocationDialect::for_version(version),
            mutation,
        );
        let epoch = match namespace {
            Some(namespace) => &mut self.arena[namespace].native_resolver_epoch,
            None => &mut self.native_compiler_epoch,
        };
        match invalidated {
            Some(false) => {}
            Some(true) => {
                *epoch = epoch.map(|epoch| {
                    epoch
                        .checked_add(1)
                        .expect("native compiler epoch exhausted")
                });
            }
            None => *epoch = None,
        }
    }

    pub(crate) fn native_compiler_cache_epochs(&self, namespace: NsId) -> Option<(u64, u64)> {
        self.native_command_version?;
        if !self.namespace_is_live(namespace)
            && self.activations.get(namespace).copied().unwrap_or(0) == 0
            && !self.dying.contains(&namespace)
            && !self.deferred.contains(&namespace)
        {
            return None;
        }
        Some((
            self.native_compiler_epoch?,
            self.arena.get(namespace)?.native_resolver_epoch?,
        ))
    }

    /// Select independently authenticated native command-cache semantics.
    pub(crate) fn set_native_command_version(&mut self, version: Option<tcl_dialect::TclVersion>) {
        if self.native_command_version != version {
            for node in &mut self.arena {
                node.command_reference_epoch = node
                    .command_reference_epoch
                    .checked_add(1)
                    .expect("native command reference epoch exhausted");
            }
            for node in self.native_command_nodes.values_mut() {
                node.epoch = node
                    .epoch
                    .checked_add(1)
                    .expect("native command epoch exhausted");
            }
            self.native_command_version = version;
        }
    }

    fn increment_command_reference(&mut self, ns: NsId) {
        self.arena[ns].command_reference_epoch = self.arena[ns]
            .command_reference_epoch
            .checked_add(1)
            .expect("native command reference epoch exhausted");
    }

    /// TclInvalidateNsPath: each actual reverse-path entry invalidates its creator.
    fn invalidate_native_path_dependents(&mut self, target: NsId) {
        if self
            .native_command_version
            .is_none_or(|version| version == tcl_dialect::TclVersion::V8_4)
        {
            return;
        }
        for ns in 0..self.arena.len() {
            let count = self.arena[ns]
                .path
                .iter()
                .filter(|entry| **entry == target)
                .count();
            for _ in 0..count {
                self.increment_command_reference(ns);
            }
        }
    }

    /// TclInvalidateNsCmdLookup is distinct from reverse-path invalidation.
    pub(crate) fn ensemble_export_epoch(&self, namespace: NsId) -> u64 {
        self.arena[namespace].ensemble_export_epoch
    }

    pub(crate) fn advance_ensemble_export_epoch(&mut self, namespace: NsId) {
        self.arena[namespace].ensemble_export_epoch = self.arena[namespace]
            .ensemble_export_epoch
            .checked_add(1)
            .expect("native ensemble export epoch exhausted");
    }

    pub(crate) fn invalidate_native_command_lookup(&mut self, ns: NsId) {
        if !self.arena[ns].exports.is_empty() {
            self.advance_ensemble_export_epoch(ns);
        }

        if self
            .native_command_version
            .is_some_and(|version| version != tcl_dialect::TclVersion::V8_4)
            && !self.arena[ns].path.is_empty()
        {
            self.increment_command_reference(ns);
        }
    }

    /// TclResetShadowedCmdRefs walks physical parents and global child trails.
    fn reset_native_shadowed_references(&mut self, namespace: NsId, simple: &[u8]) {
        if self.native_command_version.is_none() {
            return;
        }
        let mut trail: Vec<NsId> = Vec::new();
        let mut current = Some(namespace);
        while let Some(ns) = current.filter(|ns| *ns != GLOBAL) {
            let mut shadow = Some(GLOBAL);
            for &child in trail.iter().rev() {
                shadow = shadow.and_then(|parent| {
                    self.arena[parent]
                        .children
                        .get(&self.arena[child].name)
                        .copied()
                });
            }
            if let Some(global) =
                shadow.filter(|global| self.arena[*global].commands.contains_key(simple))
            {
                let generation = self.arena[global]
                    .commands
                    .generation(simple)
                    .expect("shadowed command retains its token");
                let hook = self
                    .native_compiler_hook(generation)
                    .expect("shadowed command retains its native node");
                self.note_native_compiler_mutation(Some(ns),
                    tcl_registry::native_procedure::NativeCompilerCacheMutation::NamespaceCommandShadow { hook });
                self.increment_command_reference(ns);
                self.invalidate_native_path_dependents(ns);
            }
            trail.push(ns);
            current = self.arena[ns].parent;
        }
    }

    /// Capture occupancy and C8.5's early ObjCommand path invalidation.
    pub(crate) fn native_command_creation_entry(
        &mut self,
        ns: NsId,
        simple: &[u8],
    ) -> NativeCommandCreation {
        self.native_command_creation_entry_with_jim_key(ns, simple, None)
    }

    pub(crate) fn native_command_creation_entry_with_jim_key(
        &mut self,
        ns: NsId,
        simple: &[u8],
        incoming: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    ) -> NativeCommandCreation {
        if self.native_command_version == Some(tcl_dialect::TclVersion::V8_5) {
            self.invalidate_native_path_dependents(ns);
        }
        let occupied = self.arena[ns].commands.entries.get(simple);
        let jim_table_key = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
            .and_then(|protocol| {
                let incoming = incoming.or_else(|| {
                    tcl_syntax::naming::NativeJimCommandTableKey::from_comparison_key(
                        protocol, simple,
                    )
                })?;
                assert_eq!(
                    incoming.comparison_bytes(),
                    simple,
                    "actual Jim table comparison slot"
                );
                Some(incoming.retain_for_replacement(
                    occupied.and_then(|binding| binding.jim_table_key.as_ref()),
                ))
            });
        NativeCommandCreation {
            namespace: ns,
            occupied: occupied.is_some(),
            jim_table_key,
        }
    }

    fn note_native_command_created(&mut self, entry: NativeCommandCreation, simple: &[u8]) {
        let ns = entry.namespace;
        if !entry.occupied {
            self.invalidate_native_command_lookup(ns);
            if self.native_command_version != Some(tcl_dialect::TclVersion::V8_5) {
                self.invalidate_native_path_dependents(ns);
            }
        }
        self.reset_native_shadowed_references(ns, simple);
    }

    fn increment_native_command_epoch(&mut self, generation: u64) {
        if let Some(hook) = self.native_compiler_hook(generation) {
            self.note_native_compiler_mutation(
                None,
                tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken { hook },
            );
        }
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.epoch = node
                .epoch
                .checked_add(1)
                .expect("native command epoch exhausted");
        }
    }

    fn remove_native_binding(&mut self, ns: NsId, simple: &[u8]) -> Option<CommandBinding> {
        let binding = self.arena[ns].commands.remove_binding(simple)?;
        self.increment_native_command_epoch(binding.generation);
        self.native_command_nodes
            .get_mut(&binding.generation)
            .expect("bound command retains its native command node")
            .placement = None;
        self.invalidate_native_command_lookup(ns);
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            if let Command::Proc(procedure) = &binding.command {
                procedure.retire();
            }
        }
        self.note_jim_command_retirement(binding.generation);
        Some(binding)
    }

    /// Hide retains the command node, but invalidates its cached references.
    pub(crate) fn note_native_command_hidden(&mut self, generation: u64) {
        self.increment_native_command_epoch(generation);
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            let placement = node.placement.take();
            if let Some((ns, _)) = placement {
                self.invalidate_native_command_lookup(ns);
            }
        }
    }

    /// Retire a hidden node without deriving its identity from the hidden name.
    pub(crate) fn retire_native_command_node(&mut self, generation: u64) {
        self.increment_native_command_epoch(generation);
        if let Some(node) = self.native_command_nodes.get_mut(&generation) {
            node.placement = None;
        }
    }

    /// Native Jim epoch; independent of compiler guards and C command epochs.
    pub(crate) const fn jim_procedure_epoch(&self) -> u64 {
        self.jim_procedure_epoch
    }
    fn advance_jim_procedure_epoch(&mut self) {
        self.jim_procedure_epoch = self
            .jim_procedure_epoch
            .checked_add(1)
            .expect("Jim procedure epoch exhausted");
        self.jim_retired_commands = 0;
        self.jim_retired_tokens.clear();
    }
    fn note_jim_command_retirement(&mut self, token: u64) {
        if self.variable_name_protocol != Some(tcl_syntax::naming::NativeNameProtocol::Jim084)
            || self.jim_active_commands.contains_key(&token)
            || self
                .jim_previous_commands
                .values()
                .any(|node| node.generation == token)
            || !self.jim_retired_tokens.insert(token)
        {
            return;
        }
        self.jim_previous_locations.remove(&token);
        self.release_jim_previous(token);
        self.jim_retired_commands += 1;
        if self.jim_retired_commands >= 1000 {
            self.advance_jim_procedure_epoch();
        }
    }
    pub(crate) fn enter_jim_command(&mut self, token: u64) {
        *self.jim_active_commands.entry(token).or_default() += 1;
    }
    pub(crate) fn leave_jim_command(&mut self, token: u64) {
        let count = self
            .jim_active_commands
            .get_mut(&token)
            .expect("active Jim command lease");
        *count -= 1;
        if *count == 0 {
            self.jim_active_commands.remove(&token);
            if self.native_command_at_node(token).is_none() {
                self.note_jim_command_retirement(token);
            }
        }
    }

    /// Current independently mutable command-reference context.
    pub(crate) fn native_command_reference(
        &self,
        ns: NsId,
    ) -> tcl_runtime_api::native_command_name::NativeCommandNameReference {
        tcl_runtime_api::native_command_name::NativeCommandNameReference {
            namespace_token: ns as u64,
            command_reference_epoch: self.arena[ns].command_reference_epoch,
        }
    }

    /// Observe an original node without retaining its callable implementation.
    pub(crate) fn native_command_target(
        &self,
        generation: u64,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameTarget> {
        let node = self.native_command_nodes.get(&generation)?;
        let (ns, simple) = node.placement.as_ref()?;
        (self.arena[*ns].commands.generation(simple) == Some(generation)).then_some(
            tcl_runtime_api::native_command_name::NativeCommandNameTarget {
                token: generation,
                implementation_generation: generation,
                command_epoch: node.epoch,
                namespace_token: *ns as u64,
                namespace_dying: self.dead.contains(ns) || self.dying.contains(ns),
            },
        )
    }

    /// Actual current command holder/tail for one still-live command generation.
    pub(crate) fn native_command_slot_at_node(&self, generation: u64) -> Option<(NsId, Vec<u8>)> {
        let node = self.native_command_nodes.get(&generation)?;
        let (namespace, simple) = node.placement.as_ref()?;
        (self.arena[*namespace].commands.generation(simple) == Some(generation))
            .then(|| (*namespace, simple.clone()))
    }

    /// Borrow a live native command's actual simple table key without display parsing.
    pub(crate) fn native_command_simple_at_node(&self, generation: u64) -> Option<&[u8]> {
        let node = self.native_command_nodes.get(&generation)?;
        let (namespace, simple) = node.placement.as_ref()?;
        (self.arena[*namespace].commands.generation(simple) == Some(generation))
            .then_some(simple.as_slice())
    }

    /// Clone a live worker only after its actual node placement is checked.
    pub(crate) fn native_command_at_node(&self, generation: u64) -> Option<(Command, Vec<u8>)> {
        if let Some((ns, name)) = self.jim_previous_locations.get(&generation) {
            if let Some(binding) = self
                .jim_previous_commands
                .values()
                .find(|binding| binding.generation == generation)
            {
                return Some((binding.command.clone(), self.command_fqn(*ns, name)));
            }
        }
        let node = self.native_command_nodes.get(&generation)?;
        let (ns, simple) = node.placement.as_ref()?;
        (self.arena[*ns].commands.generation(simple) == Some(generation)).then(|| {
            (
                self.arena[*ns]
                    .commands
                    .get(simple)
                    .expect("matching generation has a command")
                    .clone(),
                self.command_fqn(*ns, simple),
            )
        })
    }

    /// Capture the actual selected registration and release-specific reference context.
    pub(crate) fn native_command_name_cache(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        current: NsId,
        original: &[u8],
        protocol: tcl_registry::native_command_literal::NativeCommandNameProtocol,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameCache> {
        let (ns, simple) = self.home_of(current, original)?;
        let generation = self.arena[ns].commands.generation(&simple)?;
        let node = self.native_command_nodes.get(&generation)?;
        Some(
            tcl_runtime_api::native_command_name::NativeCommandNameCache {
                interpreter,
                version: protocol.version(),
                slot: tcl_core_types::NativeByteCommandSlot {
                    namespace: self.arena[ns].constructed_path.clone(),
                    simple: tcl_core_types::NameBytes::from(simple),
                },
                namespace_token: ns as u64,
                token: generation,
                implementation_generation: generation,
                command_epoch: node.epoch,
                reference: protocol.lookup_reference(
                    original.starts_with(b"::"),
                    self.native_command_reference(current),
                    self.native_command_reference(GLOBAL),
                ),
            },
        )
    }

    /// Prime an already selected node without reparsing its reported full name.
    pub(crate) fn native_command_name_cache_from_binding(
        &self,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        binding: &tcl_runtime_api::native_compilation::NativeCompilationBinding,
        protocol: tcl_registry::native_command_literal::NativeCommandNameProtocol,
    ) -> Option<tcl_runtime_api::native_command_name::NativeCommandNameCache> {
        let node = self.native_command_nodes.get(&binding.token)?;
        let (namespace, simple) = node.placement.as_ref()?;
        if *namespace as u64 != binding.namespace_token
            || self.arena[*namespace].commands.generation(simple) != Some(binding.token)
            || binding.implementation_generation != binding.token
            || self.arena[*namespace].constructed_path != binding.slot.namespace
            || simple.as_slice() != binding.slot.simple.as_bytes()
        {
            return None;
        }
        Some(
            tcl_runtime_api::native_command_name::NativeCommandNameCache {
                interpreter,
                version: protocol.version(),
                slot: binding.slot.clone(),
                namespace_token: binding.namespace_token,
                token: binding.token,
                implementation_generation: binding.implementation_generation,
                command_epoch: node.epoch,
                reference: None,
            },
        )
    }

    fn mark_command_deleted(
        command: &Command,
        retired: &std::cell::RefCell<Vec<std::rc::Rc<crate::ensemble::EnsembleToken>>>,
    ) {
        if let Command::Ensemble(token) = command {
            token.mark_deleted_deferred();
            retired.borrow_mut().push(std::rc::Rc::clone(token));
        } else if let Command::ChildInterp(command) = command {
            command.retire();
        }
    }

    pub(crate) fn take_retired_ensemble_roles(
        &self,
    ) -> Vec<std::rc::Rc<crate::ensemble::EnsembleToken>> {
        self.retired_ensemble_roles.borrow_mut().drain(..).collect()
    }

    /// The identity a freshly bound command token carries.
    fn mint_command_generation(&mut self) -> u64 {
        self.next_command_generation = self
            .next_command_generation
            .checked_add(1)
            .expect("native command identity exhausted");
        self.next_command_generation
    }

    fn command_fqn(&self, ns: NsId, simple: &[u8]) -> Vec<u8> {
        if !self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            return tcl_syntax::naming::native_command_full_name_bytes(
                &tcl_core_types::NativeByteCommandSlot {
                    namespace: self.arena[ns].constructed_path.clone(),
                    simple: simple.into(),
                },
            );
        }
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
        let entry = self.native_command_creation_entry(ns, &simple);
        self.insert_bound_after_entry(entry, simple, command);
    }

    fn insert_bound_after_entry(
        &mut self,
        entry: NativeCommandCreation,
        simple: Vec<u8>,
        command: Command,
    ) {
        let ns = entry.namespace;
        // C redefines a command by deleting the old hash entry and creating a
        // new one, so the name moves to its bucket head
        // (`TclCreateObjCommandInNs`).
        if let Some(displaced) = self.remove_native_binding(ns, &simple) {
            Self::mark_command_deleted(&displaced.command, &self.retired_ensemble_roles);
        }
        if let Command::Ensemble(token) = &command {
            token.rename(self.command_fqn(ns, &simple));
        }
        let compiler_hook = match &command {
            Command::Imported {
                source_generation, ..
            } => self.native_compiler_hook(*source_generation).unwrap_or(
                tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Unknown,
            ),
            _ => tcl_runtime_api::native_compilation::NativeCompilerHookPresence::Absent,
        };
        let compiler_recipe = match &command {
            Command::Imported {
                source_generation, ..
            } => self
                .native_compiler_recipe(*source_generation)
                .unwrap_or(NativeCompilerRecipe::Unknown),
            _ => NativeCompilerRecipe::Absent,
        };
        let compiler_handler = match &command {
            Command::Imported {
                source_generation, ..
            } => self.native_compiler_handler(*source_generation),
            _ => None,
        };
        let generation = self.mint_command_generation();
        self.native_command_nodes.insert(
            generation,
            NativeCommandNode {
                epoch: 0,
                compiler_hook,
                compiler_recipe,
                compiler_handler,
                placement: Some((ns, simple.clone())),
            },
        );
        let jim_table_key = entry.jim_table_key.clone();
        self.note_native_command_created(entry, &simple);
        if self.variable_name_protocol == Some(tcl_syntax::naming::NativeNameProtocol::Jim084)
            && matches!(&command, Command::Proc(_))
        {
            let recipe = tcl_syntax::native_jim_lookup::NativeJimLookupProtocol::jim084();
            if recipe
                .procedure_shadow_tail(&simple)
                .is_some_and(|tail| self.arena[GLOBAL].commands.contains_key(tail))
            {
                self.advance_jim_procedure_epoch();
            }
        }
        self.arena[ns]
            .commands
            .insert(simple, command, generation, jim_table_key);
    }

    /// Publish an existing command token at a new placement. Visibility and
    /// rename moves retain the generation because they move Tcl's `Command *`;
    /// only a new definition goes through [`Self::insert_bound`].
    fn insert_moved_binding(&mut self, ns: NsId, simple: Vec<u8>, binding: CommandBinding) {
        if let Some(displaced) = self.remove_native_binding(ns, &simple) {
            Self::mark_command_deleted(&displaced.command, &self.retired_ensemble_roles);
        }
        if let Command::Ensemble(token) = &binding.command {
            token.rename(self.command_fqn(ns, &simple));
        }
        self.native_command_nodes
            .get_mut(&binding.generation)
            .expect("moved binding retains its native command node")
            .placement = Some((ns, simple.clone()));
        self.arena[ns].commands.insert(
            simple,
            binding.command,
            binding.generation,
            binding.jim_table_key,
        );
    }

    /// A fresh tree with just the global namespace `::`.
    #[must_use]
    pub fn new() -> Namespaces {
        Namespaces {
            native_namespace_names: std::cell::RefCell::new(BTreeMap::new()),
            native_namespace_name_dead: BTreeSet::new(),
            native_command_nodes: BTreeMap::new(),
            retired_ensemble_roles: std::cell::RefCell::new(Vec::new()),
            native_command_version: Some(tcl_dialect::TclVersion::V9_0),
            native_compiler_epoch: Some(0),
            jim_previous_commands: BTreeMap::new(),
            jim_previous_locations: BTreeMap::new(),
            jim_upcalls: BTreeMap::new(),
            jim_procedure_epoch: 1,
            jim_retired_commands: 0,
            jim_active_commands: BTreeMap::new(),
            jim_retired_tokens: BTreeSet::new(),
            ns_var_global_fallback: false,
            variable_container_model: tcl_dialect::VariableContainerModel::DistinctArray,
            variable_hash_recipe: None,
            variable_lookup_policy: tcl_dialect::VariableLookupPolicy::Tcl,
            execution_name_policy: None,
            variable_string_protocol: Some(tcl_syntax::native_string::NativeStringProtocol::C(
                tcl_dialect::TclVersion::V9_0,
            )),
            variable_name_protocol: Some(tcl_syntax::naming::NativeNameProtocol::for_tcl_version(
                tcl_dialect::TclVersion::V9_0,
            )),
            variable_link_binding: tcl_dialect::VariableLinkBinding::StableCell,
            arena: vec![{
                let mut root = Namespace::new(Vec::new(), None);
                root.jim_namespace = Some((
                    JimNamespaceObject::Initial(std::rc::Rc::new(crate::obj::Owned::fresh(
                        crate::obj::new_string_bytes(b""),
                    ))),
                    std::rc::Rc::from(&b""[..]),
                ));
                root
            }],
            activations: vec![0],
            next_command_generation: 0,
            dead: BTreeSet::new(),
            retired_variable_namespaces: BTreeSet::new(),
            deferred: BTreeSet::new(),
            dying_children: BTreeMap::new(),
            dying: BTreeSet::new(),
        }
    }

    /// Retain the actual canonical Jim object for a new activation context.
    /// Jim helper contexts are counted objects, not C namespace-tree nodes.
    /// Equal names do not replace the supplied holder with another object.
    pub(crate) fn retain_jim_namespace(
        &mut self,
        holder: crate::obj::Owned,
        bytes: std::rc::Rc<[u8]>,
    ) -> NsId {
        let token = self.arena.len();
        let mut namespace = Namespace::new(Vec::new(), None);
        namespace.jim_namespace =
            Some((JimNamespaceObject::Initial(std::rc::Rc::new(holder)), bytes));
        self.arena.push(namespace);
        self.activations.push(0);
        token
    }

    /// Adopt the original empty object as the genuine Jim top-frame namespace.
    pub(crate) fn adopt_jim_root_namespace(&mut self, holder: crate::obj::Owned) {
        self.arena[GLOBAL].jim_namespace = Some((
            JimNamespaceObject::Initial(std::rc::Rc::new(holder)),
            std::rc::Rc::from(&b""[..]),
        ));
    }

    /// The counted canonical name selected from the actual retained Jim object.
    pub(crate) fn jim_namespace_bytes(&self, token: NsId) -> Option<&[u8]> {
        self.arena
            .get(token)?
            .jim_namespace
            .as_ref()
            .map(|(_, bytes)| bytes.as_ref())
    }

    /// Retain the actual original Jim namespace object, without display reparsing.
    pub(crate) fn jim_namespace_object(&self, token: NsId) -> Option<crate::obj::Owned> {
        self.arena
            .get(token)?
            .jim_namespace
            .as_ref()
            .and_then(|(holder, _)| holder.owner())
            .map(|holder| crate::obj::Owned::retain(holder.as_ptr()))
    }

    /// Transfer a newly allocated native namespace reference to its actual
    /// frame or procedure. Published indexing retains only a weak lifetime.
    pub(crate) fn take_jim_namespace_owner(
        &mut self,
        token: NsId,
    ) -> Option<std::rc::Rc<crate::obj::Owned>> {
        let (slot, _) = self.arena.get_mut(token)?.jim_namespace.as_mut()?;
        let owner = slot.owner()?;
        match slot {
            JimNamespaceObject::Initial(_) => {
                *slot = JimNamespaceObject::Published(std::rc::Rc::downgrade(&owner));
                Some(owner)
            }
            JimNamespaceObject::Published(_) => {
                Some(std::rc::Rc::new(crate::obj::Owned::retain(owner.as_ptr())))
            }
        }
    }

    fn jim_context(&self, token: NsId) -> Option<tcl_syntax::naming::NativeNameContext<'_>> {
        // Jim uses only the retained object. The constructed path is not an address.
        Some(tcl_syntax::naming::NativeNameContext::with_jim_namespace(
            &JIM_CONTEXT_PATH,
            self.jim_namespace_bytes(token)?,
        ))
    }

    /// Original Jim table spelling selected before comparison-key normalisation.
    pub(crate) fn jim_command_table_key_at(
        &self,
        current: NsId,
        original: &[u8],
        purpose: tcl_syntax::naming::NativeNamePurpose,
    ) -> Option<tcl_syntax::naming::NativeJimCommandTableKey> {
        use tcl_syntax::naming::{NativeJimCommandTableKey, NativeNamePurpose};
        let protocol = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())?;
        let context = self.jim_context(current)?;
        let selected = match purpose {
            NativeNamePurpose::CommandPublication => {
                protocol.command_publication_input(context, original)
            }
            NativeNamePurpose::AliasPublication => {
                protocol.alias_publication_input(context, original)
            }
            NativeNamePurpose::RenameDestination => {
                protocol.rename_destination_input(context, original)
            }
            _ => return None,
        }
        .ok()?;
        NativeJimCommandTableKey::from_projection(&selected)
    }

    /// Original report units of an actual table entry, with its slot kept separate.
    pub(crate) fn command_table_report_name<'a>(&'a self, ns: NsId, simple: &'a [u8]) -> &'a [u8] {
        self.arena[ns]
            .commands
            .entries
            .get(simple)
            .and_then(|binding| binding.jim_table_key.as_ref())
            .filter(|_| {
                self.variable_name_protocol
                    .is_some_and(|protocol| protocol.is_jim084())
            })
            .map_or(
                simple,
                tcl_syntax::naming::NativeJimCommandTableKey::report_bytes,
            )
    }

    /// Exact publication slot selected for the active native name recipe.
    pub(crate) fn command_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        if let Some(protocol) = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
        {
            let selected = protocol
                .command_publication_input(self.jim_context(current)?, original)
                .ok()?;
            return Some((GLOBAL, selected.jim_flat_key()?.to_vec()));
        }
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// C command registration uses global unqualified names and current qualified names.
    pub(crate) fn command_c_api_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_c_api_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// TclOO publication retains its actual selected holder and counted tail.
    pub(crate) fn oo_object_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .oo_object_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// Create an object's owned namespace from an already selected command slot.
    pub(crate) fn ensure_command_owned_namespace_in_slot(
        &mut self,
        namespace: NsId,
        simple: &[u8],
    ) -> NsId {
        self.ensure_child(namespace, simple)
    }

    /// Select the alias purpose for both local and child interpreter publication.
    pub(crate) fn alias_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let current = if protocol.is_jim084() {
            GLOBAL
        } else {
            current
        };
        let path = self.native_context_path(current)?;
        let selected = protocol
            .alias_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    fn command_projection_base(
        &self,
        current: NsId,
        selected: &tcl_syntax::naming::NativeCommandSlotProjection,
    ) -> NsId {
        match selected.namespace_route() {
            tcl_syntax::naming::NativeCommandNamespaceRoute::Root => GLOBAL,
            tcl_syntax::naming::NativeCommandNamespaceRoute::Context => current,
            tcl_syntax::naming::NativeCommandNamespaceRoute::ContextParent => {
                self.parent(current).unwrap_or(current)
            }
        }
    }

    pub(crate) fn command_projection_holder(
        &self,
        current: NsId,
        selected: &tcl_syntax::naming::NativeCommandSlotProjection,
    ) -> Option<NsId> {
        let mut token = self.command_projection_base(current, selected);
        for segment in selected.qualifiers() {
            token = self
                .arena
                .get(token)?
                .children
                .get(segment.as_bytes())
                .copied()
                .or_else(|| {
                    self.dying_children
                        .get(&(token, segment.as_bytes().to_vec()))
                        .copied()
                })?;
        }
        Some(token)
    }

    fn materialise_command_publication_slot(
        &mut self,
        current: NsId,
        selected: &tcl_syntax::naming::NativeCommandSlotProjection,
    ) -> (NsId, Vec<u8>) {
        // Preserve an exact existing holder, including a synchronously dying
        // token. A missing longer qualifier chain creates visible children
        // from the operation's selected anchor.
        let namespace = self
            .command_projection_holder(current, selected)
            .unwrap_or_else(|| {
                let mut token = self.command_projection_base(current, selected);
                for segment in selected.qualifiers() {
                    token = self.ensure_child(token, segment.as_bytes());
                }
                token
            });
        (namespace, selected.slot().simple.as_bytes().to_vec())
    }

    /// Materialise a namespace ensemble's selected registration holder.
    /// Original default-parent and explicit-current contexts remain distinct.
    pub(crate) fn ensemble_publication_at(
        &mut self,
        current: NsId,
        original: Option<&[u8]>,
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let parent = self
            .parent(current)
            .and_then(|parent| self.native_context_path(parent));
        let selected = protocol
            .ensemble_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
                parent.as_ref(),
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
    }

    /// Jim's procedure namespace derives from the counted flat publication key.
    pub(crate) fn jim_procedure_context(&mut self, selected_key: &[u8]) -> Option<NsId> {
        let protocol = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())?;
        let bytes = protocol.jim_procedure_namespace(selected_key).ok()?;
        if bytes.is_empty() {
            return Some(GLOBAL);
        }
        let holder = crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes));
        Some(self.retain_jim_namespace(holder, std::rc::Rc::from(bytes)))
    }

    /// Resolve a procedure's original holder before any declaration resources
    /// are accessed. The selected lookup slot retains the original holder.
    pub(crate) fn procedure_lookup_at(
        &self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_lookup_projection(tcl_syntax::naming::NativeNameContext::new(&path), original)
            .ok()?;
        let namespace = self.command_projection_holder(current, &selected)?;
        Some((namespace, selected.slot().simple.as_bytes().to_vec()))
    }

    /// Procedure publication uses the release-selected registration slot;
    /// general builtin C API publication has an independent entry.
    pub(crate) fn procedure_publication_at(
        &mut self,
        current: NsId,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let protocol = self.variable_name_protocol?;
        if protocol.is_jim084() {
            return self.command_publication_at(current, original);
        }
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_publication_projection(
                tcl_syntax::naming::NativeNameContext::new(&path),
                original,
            )
            .ok()?;
        Some(self.materialise_command_publication_slot(current, &selected))
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
        if self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            let (namespace, key) = self.command_publication_at(GLOBAL, name)?;
            let report = self.jim_command_table_key_at(
                GLOBAL,
                name,
                tcl_syntax::naming::NativeNamePurpose::AliasPublication,
            );
            let entry = self.native_command_creation_entry_with_jim_key(namespace, &key, report);
            self.insert_bound_after_entry(entry, key.clone(), command);
            return Some((namespace, key));
        }
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

    /// The command generations owned by `owner`, optionally restricted to one
    /// public/private dispatcher role.
    pub(crate) fn oo_command_role_locations(
        &self,
        owner: OoId,
        role: Option<OoCommandRole>,
    ) -> Vec<(Vec<u8>, u64)> {
        let mut hits = Vec::new();
        for (ns, node) in self.arena.iter().enumerate() {
            for (name, command) in node.commands.iter() {
                if command.oo_binding().is_some_and(|(id, candidate)| {
                    id == owner && role.is_none_or(|expected| expected == candidate)
                }) {
                    let generation = node
                        .commands
                        .generation(name)
                        .expect("an iterated command has a generation");
                    hits.push((self.command_fqn(ns, name), generation));
                }
            }
        }
        hits
    }

    /// Remove namespace-table commands carrying `owner` in one of `roles`,
    /// returning their last fully-qualified locations. Hidden commands are
    /// held by `Interp` and are retired by the same caller after this half.
    pub(crate) fn remove_oo_command_roles(
        &mut self,
        owner: OoId,
        roles: &[OoCommandRole],
    ) -> Vec<(Vec<u8>, u64)> {
        let mut hits = Vec::new();
        for (ns, node) in self.arena.iter().enumerate() {
            for (name, command) in node.commands.iter() {
                if command
                    .oo_binding()
                    .is_some_and(|(id, role)| id == owner && roles.contains(&role))
                {
                    let generation = node
                        .commands
                        .generation(name)
                        .expect("an iterated command has a generation");
                    hits.push((ns, name.clone(), generation));
                }
            }
        }
        let mut removed = Vec::with_capacity(hits.len());
        for (ns, name, generation) in hits {
            self.remove_native_binding(ns, &name);
            removed.push((self.command_fqn(ns, &name), generation));
        }
        removed
    }

    /// Remove the binding at `(ns, name)`, returning it — the rollback for a
    /// refused alias definition.
    pub(crate) fn unbind_in(&mut self, ns: NsId, name: &[u8]) -> Option<Command> {
        self.remove_native_binding(ns, name)
            .map(|binding| binding.command)
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
            Some((ns, simple)) => self
                .remove_native_binding(ns, &simple)
                .is_some_and(|binding| {
                    Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
                    true
                }),
            None => false,
        }
    }

    /// Remove a command binding without deleting its token. Used only for
    /// visibility moves such as `interp hide`; the token remains alive.
    pub(crate) fn take(&mut self, current: NsId, name: &[u8]) -> Option<CommandBinding> {
        let (ns, simple) = self.home_of(current, name)?;
        self.arena[ns].commands.remove_binding(&simple)
    }

    /// Restore a command token taken for a visibility move under `name`,
    /// preserving its generation while creating any destination namespaces.
    pub(crate) fn restore(&mut self, name: &[u8], binding: CommandBinding) -> bool {
        let segments = split_qualifier(name);
        let Some((simple, ns_parts)) = segments.split_last() else {
            return false;
        };
        let mut ns = GLOBAL;
        for part in ns_parts {
            ns = self.ensure_child(ns, part);
        }
        let hook = self
            .native_compiler_hook(binding.generation)
            .expect("restored command retains its native node");
        self.note_native_compiler_mutation(
            None,
            tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken { hook },
        );
        self.insert_moved_binding(ns, (*simple).to_vec(), binding);
        self.invalidate_native_command_lookup(ns);
        true
    }

    /// Publish the destination half of `rename old new` — both names resolved
    /// relative to `current`, absolute when `::`-led — and leave the source's
    /// table entry standing. C's `TclRenameCommand` creates the destination
    /// hash entry, fires the `rename` traces, and only then deletes the source
    /// one, with *both* entries referencing the one `Command`; the caller
    /// closes that window with [`Self::retire_rename_source`]. `None` when
    /// `old` names no command (or `new` has no tail to bind).
    ///
    /// A command moved across namespaces is re-homed: a `Command::Proc`'s
    /// `ns`/`fqn` are updated to the destination so `namespace current` inside
    /// its body reports the new namespace, mirroring C's `cmdPtr->nsPtr`
    /// reassignment — and because C re-homes the shared `Command` before the
    /// traces run, the vacating name reports the *destination's* namespace for
    /// the callbacks' duration too (tclsh 8.6/9.0-pinned).
    ///
    /// Occupancy protection is the caller's job (see
    /// [`Self::destination_occupant_fqn`]) — a script-visible "already exists"
    /// refusal needs release-gate context (a TclOO root this release hides is
    /// not really taken) that this table-only layer does not have, so this
    /// unconditionally overwrites whatever is at the destination, same as
    /// [`Self::insert_bound`].
    ///
    /// Built-in protection lives in the `rename` builtin, not here — this is
    /// the pure table operation, as is the `rename old ""` deletion
    /// ([`Self::delete`]).
    pub(crate) fn publish_rename_destination(
        &mut self,
        current: NsId,
        old: &[u8],
        new: &[u8],
    ) -> Option<RenamePublication> {
        let (old_ns, old_simple) = self.home_of(current, old)?;
        // Unreachable for a non-empty, non-separator-terminated name; nothing
        // has moved yet, so there is nothing to put back.
        let (ns, simple) = self.destination_of(current, new)?;
        // C's `TclRenameCommand` creates the destination hash entry *before*
        // deleting the source's, so the transient extra entry can trigger a
        // rebuild a delete-first order would not.
        self.arena[ns].commands.reserve_entry(&simple);
        // SAFETY of expect: `home_of` reported the binding exists.
        let source_generation = self.arena[old_ns]
            .commands
            .generation(&old_simple)
            .expect("home_of reported a binding at the rename source");
        let cmd = self.arena[old_ns]
            .commands
            .get(&old_simple)
            .cloned()
            .expect("home_of reported a binding at the rename source");
        let destination_fqn = self.command_fqn(ns, &simple);
        let cmd = self.rehome_command(cmd, ns, &simple, &destination_fqn);
        // One command under two names, as C has it: the source slot takes the
        // re-homed binding too, and an ensemble token is shared outright.
        self.arena[old_ns]
            .commands
            .rebind_slot(&old_simple, cmd.clone());
        let jim_table_key = self.jim_command_table_key_at(
            current,
            new,
            tcl_syntax::naming::NativeNamePurpose::RenameDestination,
        );
        self.insert_moved_binding(
            ns,
            simple,
            CommandBinding {
                generation: source_generation,
                command: cmd,
                jim_table_key,
            },
        );
        self.reset_native_shadowed_references(
            ns,
            &self.native_command_nodes[&source_generation]
                .placement
                .as_ref()
                .expect("published rename destination")
                .1
                .clone(),
        );
        self.invalidate_native_command_lookup(old_ns);
        self.invalidate_native_command_lookup(ns);
        Some(RenamePublication {
            source: (old_ns, old_simple),
            destination_fqn,
            generation: source_generation,
        })
    }

    /// Delete the source entry [`Self::publish_rename_destination`] left
    /// standing — C's `Tcl_DeleteHashEntry(oldHPtr)` at the tail of
    /// `TclRenameCommand`. A plain table removal: no `delete` trace fires and
    /// no token dies, because the command lives on under its new name. A
    /// callback may have removed or rebound the slot itself, so whatever is
    /// there goes and an already-vacated entry is not an error.
    pub(crate) fn retire_rename_source(&mut self, publication: &RenamePublication) {
        let (ns, simple) = &publication.source;
        if let Some(removed) = self.arena[*ns].commands.remove_binding(simple) {
            if removed.generation != publication.generation {
                self.retire_native_command_node(removed.generation);
            }
        }
        self.increment_native_command_epoch(publication.generation);
        if self.variable_name_protocol == Some(tcl_syntax::naming::NativeNameProtocol::Jim084) {
            self.advance_jim_procedure_epoch();
        }
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

    /// Re-point a command that carries its own binding identity at the site a
    /// `rename` moved it to — C's `cmdPtr->nsPtr` reassignment, which happens
    /// to the one shared `Command` and so is visible through *both* names for
    /// the `rename` traces' duration.
    ///
    /// A `Command::Proc` carries its home namespace and FQN, so `namespace
    /// current` inside its body reports the new namespace. A
    /// `Command::OoObject` carries a stable identity, so it deliberately passes
    /// through unchanged; `OoState` updates only its Tcl-facing name projection.
    /// Every other variant carries no site of its own and passes through unchanged.
    fn rehome_command(&mut self, command: Command, ns: NsId, simple: &[u8], fqn: &[u8]) -> Command {
        if let Command::Proc(definition) = &command {
            let declaration = definition.declaration();
            let mut location = declaration.location();
            location.qualified_name = fqn.to_vec();
            if let Some(protocol) = self
                .variable_name_protocol
                .filter(|protocol| protocol.is_jim084())
            {
                if let Some(bytes) = protocol
                    .jim_procedure_relocation_namespace(simple)
                    .expect("selected Jim relocation recipe")
                {
                    let holder = crate::obj::Owned::fresh(crate::obj::new_string_bytes(bytes));
                    let namespace = self.retain_jim_namespace(holder, std::rc::Rc::from(bytes));
                    location.namespace = namespace;
                    location.jim_namespace = self.take_jim_namespace_owner(namespace);
                }
            } else {
                location.namespace = ns;
            }
            declaration.relocate(location);
        }
        command
    }

    /// The `(namespace, simple name)` a written destination name binds — the
    /// split C's `TclGetNamespaceForQualName(…, TCL_CREATE_NS_IF_UNKNOWN)`
    /// performs for `rename`'s new name, creating any intermediate namespaces
    /// (C creates them even when the rename is later refused). A trailing
    /// separator run names the empty-string `{}` command in the full qualifier
    /// chain (`rename foo x::` binds `::x::`, `rename bar ::` the global `{}` —
    /// tclsh 8.6/9.0-pinned), matching `command_home_ns` / `home_of`.
    fn destination_of(&mut self, current: NsId, new: &[u8]) -> Option<(NsId, Vec<u8>)> {
        if let Some(protocol) = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
        {
            let selected = protocol
                .rename_destination_input(self.jim_context(current)?, new)
                .ok()?;
            return Some((GLOBAL, selected.jim_flat_key()?.to_vec()));
        }
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
        tcl_syntax::naming::alias_chain_loops((ns, simple.to_vec()), |hop| {
            let Some(Command::Alias { target, .. }) = self.command_in(hop.0, &hop.1) else {
                return Ok::<_, std::convert::Infallible>(None);
            };
            Ok(self.home_of(GLOBAL, &target))
        })
        .unwrap_or_else(|error| match error {})
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
        if new.is_empty()
            || self
                .variable_name_protocol
                .is_some_and(|protocol| protocol.is_jim084())
        {
            return false;
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

    /// Rewrite redirects retaining one exact source generation. The FQN is a
    /// projection only: retained and recreated namespace tokens may expose the
    /// same spelling simultaneously, so it cannot identify the imports to move.
    pub fn retarget_imports(&mut self, source_generation: u64, new_fqn: &[u8]) {
        for ns in &mut self.arena {
            for cmd in ns.commands.values_mut() {
                if let Command::Imported {
                    source,
                    source_generation: candidate,
                    ..
                } = cmd
                {
                    if *candidate == source_generation {
                        *source = new_fqn.to_vec();
                    }
                }
            }
        }
    }

    /// Attach by-name fallback redirects to the fresh token that replaced
    /// their now-retired source generation. A simultaneously retained token
    /// with the same FQN remains live and is deliberately left alone.
    pub(crate) fn reattach_missing_imports(
        &mut self,
        source_fqn: &[u8],
        new_generation: u64,
        live_generations: &std::collections::HashSet<u64>,
    ) {
        for ns in &mut self.arena {
            for command in ns.commands.values_mut() {
                let Command::Imported {
                    source,
                    source_generation,
                    ..
                } = command
                else {
                    continue;
                };
                if source == source_fqn && !live_generations.contains(source_generation) {
                    *source_generation = new_generation;
                }
            }
        }
    }

    /// Attach imports of one exact source generation to its ensemble token.
    /// This changes alias metadata only; same-FQN imports of another retained
    /// namespace generation remain untouched.
    pub(crate) fn retarget_imports_to_ensemble(
        &mut self,
        source_generation: u64,
        new: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) {
        for ns in &mut self.arena {
            for command in ns.commands.values_mut() {
                let Command::Imported {
                    source_generation: candidate,
                    ensemble,
                    ..
                } = command
                else {
                    continue;
                };
                if *candidate == source_generation {
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
        if self
            .native_command_version
            .is_some_and(|version| version != tcl_dialect::TclVersion::V8_4)
        {
            self.note_native_compiler_mutation(
                Some(ns),
                tcl_registry::native_procedure::NativeCompilerCacheMutation::NamespacePath,
            );
            self.increment_command_reference(ns);
        }
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
        // `::x::`, tclsh 8.6/9.0-pinned) — mirroring `home_of`'s
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

    /// Create the namespace owned by a command at `name` under the exact
    /// namespace token that receives the command binding. During synchronous
    /// namespace teardown that may be a detached dying token; TclOO objects
    /// created by a delete trace must join that generation and its fixed-point
    /// sweep, not recreate the visible qualifier chain.
    pub(crate) fn ensure_command_owned_namespace(&mut self, current: NsId, name: &[u8]) -> NsId {
        let home = self.command_home_ns(current, name);
        let tail = tcl_syntax::naming::written_command_tail(name);
        if tail.is_empty() {
            return home;
        }
        self.ensure_child(home, tail)
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

    /// Whether namespace teardown has destroyed this token's variable cells.
    pub(crate) fn namespace_variables_are_deleted(&self, ns: NsId) -> bool {
        self.retired_variable_namespaces.contains(&ns)
    }

    // activations and deferred teardown (C's `activationCount`)

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
    pub(crate) fn defer_namespace(&mut self, ns: NsId) -> Option<crate::obj::Owned> {
        self.namespace_name_begin_deletion(ns);
        self.dead.insert(ns);
        self.deferred.insert(ns);
        // C frees `unknownHandlerPtr` before it looks at the activation count.
        let retired = self.arena[ns].unknown.take();
        if let Some(parent) = self.arena[ns].parent {
            let fqn = self.qualified_name(ns);
            let name = self.arena[ns].name.clone();
            self.arena[parent].children.remove(&name);
            self.arena[parent].child_order.remove(&name);
            // C nulls `parentPtr`: the spelling is free for a wholly separate
            // token straight away, and this one answers from its own name.
            self.arena[ns].parent = None;
            self.namespace_name_detach_parent(ns);
            self.arena[ns].retained_fqn = Some(fqn);
        }
        retired
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

    // per-namespace variable tables (the variable resolver's storage)

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
        let protocol = self.variable_name_protocol?;
        let input = protocol.variable_root_input(name);
        let name = input.selected();
        if protocol.is_jim084() {
            let namespace = self.jim_namespace_bytes(current)?;
            let mut prefix = b"::".to_vec();
            prefix.extend_from_slice(namespace);
            return Some((
                GLOBAL,
                tcl_syntax::naming::jim_global_variable_key_bytes(&prefix, name),
            ));
        }
        // Qualification is selected from the CString prefix; an ignored
        // separator after zero cannot reclassify the full counted simple key.
        if input.qualification() == tcl_syntax::naming::NativeNameQualification::Unqualified {
            return Some((current, name.to_vec()));
        }
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
            .map(|(k, _)| self.command_table_report_name(ns, k).to_vec())
            .collect()
    }

    /// Namespace `ns`'s variable table (read).
    #[must_use]
    pub(crate) fn var_table(&self, ns: NsId) -> &VarTable {
        self.arena[ns]
            .vars
            .set_hash_recipe(self.variable_hash_recipe);
        self.arena[ns]
            .vars
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        &self.arena[ns].vars
    }

    /// Namespace `ns`'s variable table (mutable).
    pub(crate) fn var_table_mut(&mut self, ns: NsId) -> &mut VarTable {
        self.arena[ns]
            .vars
            .set_hash_recipe(self.variable_hash_recipe);
        self.arena[ns]
            .vars
            .set_container_model(self.variable_container_model, self.variable_string_protocol);
        &mut self.arena[ns].vars
    }

    /// The fully-qualified name of `ns` (`::a::b`; global is `::`).
    #[must_use]
    pub fn qualified_name(&self, ns: NsId) -> Vec<u8> {
        if self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            if let Some(namespace) = self.jim_namespace_bytes(ns) {
                let mut rooted = b"::".to_vec();
                rooted.extend_from_slice(namespace);
                return rooted;
            }
        }
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

    /// Return the original constructed address of an actual arena token.
    /// Public liveness and physical command/variable tables remain separate;
    /// detached frames keep their own address without reparsing a display.
    pub(crate) fn native_context_path(
        &self,
        ns: NsId,
    ) -> Option<tcl_core_types::ByteNamespacePath> {
        if self
            .variable_name_protocol
            .is_some_and(|protocol| protocol.is_jim084())
        {
            let original = self.jim_namespace_bytes(ns)?;
            return Some(if original.is_empty() {
                tcl_core_types::ByteNamespacePath::root()
            } else {
                tcl_core_types::ByteNamespacePath::from_segments([original])
            });
        }
        self.arena
            .get(ns)
            .map(|namespace| namespace.constructed_path.clone())
    }

    // the `namespace` command surface

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
            self.advance_ensemble_export_epoch(ns);
        }
    }

    /// Drop all of `ns`'s export patterns (`namespace export -clear`).
    pub fn clear_exports(&mut self, ns: NsId) {
        if !self.arena[ns].exports.is_empty() {
            self.advance_ensemble_export_epoch(ns);
        }
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

    /// Consume the original creation entry after its callbacks have returned.
    pub(crate) fn bind_after_native_creation_entry(
        &mut self,
        entry: NativeCommandCreation,
        name: &[u8],
        command: Command,
    ) {
        self.insert_bound_after_entry(entry, name.to_vec(), command);
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

    /// Probe the actual child table without parsing the original member key.
    pub(crate) fn child_token(&self, parent: NsId, member: &[u8]) -> Option<NsId> {
        self.arena.get(parent)?.children.get(member).copied()
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

    /// Resolve an interpreter-unique command generation in any visible or
    /// retained namespace table, including its current Tcl-facing location.
    pub(crate) fn command_by_generation(&self, generation: u64) -> Option<(Vec<u8>, Command)> {
        if let Some((command, name)) = self.native_command_at_node(generation) {
            return Some((name, command));
        }
        self.arena.iter().enumerate().find_map(|(ns, node)| {
            node.commands.entries.iter().find_map(|(name, binding)| {
                (binding.generation == generation)
                    .then(|| (self.command_fqn(ns, name), binding.command.clone()))
            })
        })
    }

    /// Every command generation currently resident in a visible or retained
    /// namespace table.
    pub(crate) fn command_generations(&self) -> std::collections::HashSet<u64> {
        self.arena
            .iter()
            .flat_map(|node| node.commands.entries.values())
            .map(|binding| binding.generation)
            .collect()
    }

    /// The fully-qualified name of the binding at `(ns, name)`.
    pub(crate) fn command_fqn_at(&self, ns: NsId, name: &[u8]) -> Vec<u8> {
        self.command_fqn(ns, name)
    }

    /// `ns`'s `namespace unknown` handler, if one is set (an empty/`None` handler
    /// means "use the interpreter default `::unknown`").
    #[must_use]
    pub(crate) fn unknown_handler(&self, ns: NsId) -> Option<*mut crate::obj::TclObj> {
        self.arena[ns]
            .unknown
            .as_ref()
            .map(crate::obj::Owned::as_ptr)
    }

    /// Transfer the original handler root; release the retired root after the
    /// namespace borrow ends so native free hooks may re-enter the interpreter.
    pub(crate) fn set_unknown_handler(
        &mut self,
        ns: NsId,
        handler: Option<crate::obj::Owned>,
    ) -> Option<crate::obj::Owned> {
        std::mem::replace(&mut self.arena[ns].unknown, handler)
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
    ) -> Option<(Vec<u8>, u64)> {
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
        let binding = self.remove_native_binding(ns, &name)?;
        Some((self.command_fqn(ns, &name), binding.generation))
    }

    /// Locate one ensemble command by stable identity, including the exact
    /// command generation that owns its trace sidecars.
    pub(crate) fn ensemble_identity_location(
        &self,
        identity: &std::rc::Rc<crate::ensemble::EnsembleToken>,
    ) -> Option<(Vec<u8>, u64)> {
        self.arena.iter().enumerate().find_map(|(ns, node)| {
            node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Ensemble(current)
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| {
                    (
                        self.command_fqn(ns, name),
                        node.commands
                            .generation(name)
                            .expect("iterated command has a generation"),
                    )
                })
            })
        })
    }

    /// Imported aliases whose immediate source generation is in `origins` or
    /// whose retained ensemble identity is in `tokens`. Each result includes the
    /// import binding's stable identity so callers can fire delete traces while
    /// it is still visible, then remove only that original command after any
    /// reentrant replacement performed by the callback.
    pub(crate) fn imports_for_origins(
        &self,
        origins: &std::collections::HashSet<u64>,
        tokens: &[std::rc::Rc<crate::ensemble::EnsembleToken>],
    ) -> Vec<(Vec<u8>, std::rc::Rc<crate::interp::ImportToken>)> {
        let mut hits = Vec::new();
        for (id, node) in self.arena.iter().enumerate() {
            for (name, command) in node.commands.iter() {
                let Command::Imported {
                    source_generation,
                    ensemble,
                    identity,
                    ..
                } = command
                else {
                    continue;
                };
                let retains_token = ensemble.as_ref().is_some_and(|imported| {
                    tokens
                        .iter()
                        .any(|victim| std::rc::Rc::ptr_eq(imported, victim))
                });
                if origins.contains(source_generation) || retains_token {
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
    ) -> Option<(Vec<u8>, u64)> {
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
        let binding = self.remove_native_binding(ns, &name)?;
        Some((self.command_fqn(ns, &name), binding.generation))
    }

    /// Locate one imported command by stable identity, including the exact
    /// command generation that owns its trace sidecars.
    pub(crate) fn import_identity_location(
        &self,
        identity: &std::rc::Rc<crate::interp::ImportToken>,
    ) -> Option<(Vec<u8>, u64)> {
        self.arena.iter().enumerate().find_map(|(ns, node)| {
            node.commands.iter().find_map(|(name, command)| {
                matches!(
                    command,
                    Command::Imported { identity: current, .. }
                        if std::rc::Rc::ptr_eq(current, identity)
                )
                .then(|| {
                    (
                        self.command_fqn(ns, name),
                        node.commands
                            .generation(name)
                            .expect("iterated command has a generation"),
                    )
                })
            })
        })
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

    /// Mark a synchronous token dying while its original variable table and
    /// parent edge remain available to the variable deletion callbacks.
    pub(crate) fn begin_namespace_variable_teardown(&mut self, ns: NsId) {
        self.namespace_name_begin_deletion(ns);
        self.dying.insert(ns);
        if ns != GLOBAL {
            self.dead.insert(ns);
        }
    }

    /// Detach the exact token after its variable callbacks. Descendants remain
    /// live through its retained edge until recursive teardown reaches them.
    pub(crate) fn begin_namespace_teardown(&mut self, ns: NsId) {
        self.begin_namespace_variable_teardown(ns);
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
        for victim in victims {
            self.namespace_name_finish_deletion(victim);
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

    /// Fully-qualified locations and exact command generations in retained
    /// arena nodes. Teardown uses these to discard only the sidecars of the
    /// bindings it is about to clear.
    pub(crate) fn command_locations_in_ids(&self, ids: &[NsId]) -> Vec<(Vec<u8>, u64)> {
        let mut locations = Vec::new();
        for &id in ids {
            locations.extend(
                self.arena[id]
                    .commands
                    .hash_order()
                    .into_iter()
                    .map(|(name, generation)| (self.command_fqn(id, &name), generation)),
            );
        }
        locations
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
    pub(crate) fn clear_namespace_ids(&mut self, ids: &[NsId]) -> Vec<crate::obj::Owned> {
        let mut retired = Vec::new();
        for &id in ids.iter().rev() {
            if id != GLOBAL {
                self.retired_variable_namespaces.insert(id);
            }
            let commands = self.arena[id].commands.hash_order();
            for (simple, _) in commands {
                if let Some(binding) = self.remove_native_binding(id, &simple) {
                    Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
                }
            }
            let n = &mut self.arena[id];
            n.children.clear();
            n.child_order.clear();
            for command in n.commands.values() {
                Self::mark_command_deleted(command, &self.retired_ensemble_roles);
            }
            n.commands.clear();
            n.path.clear();
            n.exports.clear();
            n.vars = VarTable::default();
            retired.extend(n.unknown.take());
        }
        retired
    }

    /// Clear one dying namespace's own variable table, retaining paths and
    /// child links through the ordinary command deletion callbacks. The
    /// recursive delete callbacks still to run. The command table is emptied
    /// one token at a time afterwards, in hash order; the final fixed-point
    /// sweep clears the retained metadata and links.
    pub(crate) fn clear_namespace_token(&mut self, ns: NsId) {
        if ns != GLOBAL {
            self.retired_variable_namespaces.insert(ns);
        }
        self.arena[ns].vars = VarTable::default();
    }

    /// Unlink native paths after this token's ordinary commands are deleted.
    pub(crate) fn finish_native_namespace_command_path(&mut self, ns: NsId) {
        self.arena[ns].path.clear();
        self.invalidate_native_path_dependents(ns);
        for node in &mut self.arena {
            node.path.retain(|entry| *entry != ns);
        }
    }

    /// Recursively clear a namespace and its descendants: dropping the `VarTable`
    /// releases the variables' object references (`TclFreeVar`); commands and
    /// child links are dropped.
    fn delete_subtree(&mut self, ns: NsId) {
        if ns != GLOBAL {
            self.retired_variable_namespaces.insert(ns);
        }
        for child in self.children(ns) {
            self.delete_subtree(child);
        }
        let commands = self.arena[ns].commands.hash_order();
        for (simple, _) in commands {
            if let Some(binding) = self.remove_native_binding(ns, &simple) {
                Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
            }
        }
        let n = &mut self.arena[ns];
        n.children.clear();
        n.child_order.clear();
        for command in n.commands.values() {
            Self::mark_command_deleted(command, &self.retired_ensemble_roles);
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

    /// Remove the simple-named command `name` directly from `ns` (no resolution
    /// walk); returns whether it existed. For `namespace forget`.
    pub fn remove_in(&mut self, ns: NsId, name: &[u8]) -> bool {
        self.remove_native_binding(ns, name).is_some_and(|binding| {
            Self::mark_command_deleted(&binding.command, &self.retired_ensemble_roles);
            true
        })
    }

    // helpers

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
        if let Some(protocol) = self
            .variable_name_protocol
            .filter(|protocol| protocol.is_jim084())
        {
            let keys = protocol
                .jim_command_lookup_keys(self.jim_context(current)?, name)
                .ok()?;
            return keys.into_iter().find_map(|key| {
                self.arena[GLOBAL]
                    .commands
                    .contains_key(key.as_bytes())
                    .then(|| (GLOBAL, key.as_bytes().to_vec()))
            });
        }
        let protocol = self.variable_name_protocol?;
        let path = self.native_context_path(current)?;
        let selected = protocol
            .command_lookup_projection(tcl_syntax::naming::NativeNameContext::new(&path), name)
            .ok()?;
        let simple = selected.slot().simple.as_bytes();
        let ns_parts = selected.qualifiers();
        // Walk `ns_parts` from `base`, then require the command itself.
        let find_under = |base: NsId| -> Option<NsId> {
            let mut ns = base;
            for part in ns_parts {
                ns = self.arena[ns]
                    .children
                    .get(part.as_bytes())
                    .copied()
                    .or_else(|| {
                        self.dying_children
                            .get(&(ns, part.as_bytes().to_vec()))
                            .copied()
                    })?;
            }
            if self.arena[ns].commands.contains_key(simple) {
                Some(ns)
            } else {
                None
            }
        };
        if selected.namespace_route() == tcl_syntax::naming::NativeCommandNamespaceRoute::Root {
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
        let mut path = self.arena[parent].constructed_path.clone().into_segments();
        path.push(tcl_core_types::NameBytes::from(name));
        let mut namespace = Namespace::new(name.to_vec(), Some(parent));
        namespace.constructed_path = tcl_core_types::ByteNamespacePath::from_segments(path);
        self.arena.push(namespace);
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
    fn compiler_and_resolver_epochs_match_original_native_mutation_rows() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        let cases = [
            (
                tcl_dialect::TclVersion::V8_4,
                include_str!("../tests/data/native_compiler_epochs/8.4.20.tsv"),
            ),
            (
                tcl_dialect::TclVersion::V8_5,
                include_str!("../tests/data/native_compiler_epochs/8.5.19.tsv"),
            ),
            (
                tcl_dialect::TclVersion::V8_6,
                include_str!("../tests/data/native_compiler_epochs/8.6.18.tsv"),
            ),
            (
                tcl_dialect::TclVersion::V9_0,
                include_str!("../tests/data/native_compiler_epochs/9.0.4.tsv"),
            ),
            (
                tcl_dialect::TclVersion::V9_1,
                include_str!("../tests/data/native_compiler_epochs/9.1.0.tsv"),
            ),
        ];
        for (version, rows) in cases {
            let mut namespaces = Namespaces::new();
            namespaces.set_native_command_version(Some(version));
            namespaces.bind(GLOBAL, b"set", cmd());
            let stock = namespaces.resolve_generation(GLOBAL, b"set").unwrap();
            namespaces.set_native_compiler_hook(stock, Hook::Present);
            let baseline = rows
                .lines()
                .nth(1)
                .unwrap()
                .split('\t')
                .nth(2)
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let mut local = None;
            let mut hidden = None;
            for row in rows.lines().skip(1) {
                let fields: Vec<_> = row.split('\t').collect();
                let event = fields[0];
                match event {
                    "initial" => {}
                    "namespace_create" | "namespace_recreate" => {
                        local = Some(namespaces.ensure_namespace(GLOBAL, b"N"));
                    }
                    "plain_create" => namespaces.bind(GLOBAL, b"p", cmd()),
                    "plain_rename" | "compiled_rename" | "compiled_restore" => {
                        let (from, to) = match event {
                            "plain_rename" => (b"p".as_slice(), b"q".as_slice()),
                            "compiled_rename" => (b"set".as_slice(), b"stamp_saved_set".as_slice()),
                            _ => (b"stamp_saved_set".as_slice(), b"set".as_slice()),
                        };
                        let publication = namespaces
                            .publish_rename_destination(GLOBAL, from, to)
                            .unwrap();
                        namespaces.retire_rename_source(&publication);
                    }
                    "plain_delete" => assert!(namespaces.delete(GLOBAL, b"q")),
                    "compiled_shadow" => namespaces.bind(local.unwrap(), b"set", cmd()),
                    "shadow_delete" => assert!(namespaces.delete(local.unwrap(), b"set")),
                    "replacement_plain" => namespaces.bind(GLOBAL, b"set", cmd()),
                    "replacement_delete" => assert!(namespaces.delete(GLOBAL, b"set")),
                    "compiled_hide" => {
                        // The hidden table owns this exact token between operations.
                        let binding = namespaces.take(GLOBAL, b"set").unwrap();
                        namespaces.note_native_command_hidden(binding.generation);
                        assert_eq!(binding.generation, stock);
                        hidden = Some(binding);
                    }
                    "compiled_expose" => {
                        assert!(namespaces.restore(b"set", hidden.take().unwrap()));
                    }
                    "namespace_path" | "same_path" => {
                        let target = namespaces.ensure_namespace(GLOBAL, b"M");
                        namespaces.set_path(local.unwrap(), vec![target]);
                    }
                    "clear_path" => namespaces.set_path(local.unwrap(), Vec::new()),
                    "namespace_delete" => {
                        assert!(namespaces.delete_namespace(GLOBAL, b"N"));
                        local = None;
                    }
                    _ => panic!("unhandled original native event {event}"),
                }
                assert_eq!(fields[1], "0", "{version:?} {event}: native completion");
                let expected_compiler = fields[2].parse::<u64>().unwrap() - baseline;
                assert_eq!(
                    namespaces.native_compiler_cache_epochs(GLOBAL),
                    Some((expected_compiler, fields[3].parse().unwrap())),
                    "{version:?} {event}: global stamp"
                );
                match local {
                    Some(local) => assert_eq!(
                        namespaces.native_compiler_cache_epochs(local),
                        Some((expected_compiler, fields[4].parse().unwrap())),
                        "{version:?} {event}: original namespace stamp"
                    ),
                    None => assert_eq!(fields[4], "-1", "{version:?} {event}"),
                }
            }
        }
    }

    #[test]
    fn imported_raw_compiler_recipe_is_independent_of_future_origin_header() {
        let mut namespaces = Namespaces::new();
        namespaces.bind(GLOBAL, b"origin", cmd());
        let origin = namespaces.resolve_generation(GLOBAL, b"origin").unwrap();
        namespaces.set_native_compiler_recipe(origin, NativeCompilerRecipe::ProcedureNoOp);
        namespaces.bind(
            GLOBAL,
            b"copied",
            Command::Imported {
                source: b"::origin".to_vec(),
                source_generation: origin,
                ensemble: None,
                identity: std::rc::Rc::new(crate::interp::ImportToken),
            },
        );
        let imported = namespaces.resolve_generation(GLOBAL, b"copied").unwrap();
        namespaces.set_native_compiler_recipe(origin, NativeCompilerRecipe::Absent);
        assert_eq!(
            namespaces.native_compiler_recipe(imported),
            Some(NativeCompilerRecipe::ProcedureNoOp)
        );
        namespaces.set_native_compiler_recipe(origin, NativeCompilerRecipe::Unknown);
        assert_eq!(
            namespaces.native_compiler_recipe(imported),
            Some(NativeCompilerRecipe::ProcedureNoOp)
        );
    }

    #[test]
    fn imported_raw_compiler_attachment_is_copied_once() {
        use tcl_runtime_api::native_compilation::NativeCompilerHookPresence as Hook;
        let mut namespaces = Namespaces::new();
        namespaces.bind(GLOBAL, b"origin", cmd());
        let origin = namespaces.resolve_generation(GLOBAL, b"origin").unwrap();
        namespaces.bind(
            GLOBAL,
            b"old",
            Command::Imported {
                source: b"::origin".to_vec(),
                source_generation: origin,
                ensemble: None,
                identity: std::rc::Rc::new(crate::interp::ImportToken),
            },
        );
        let old = namespaces.resolve_generation(GLOBAL, b"old").unwrap();
        namespaces.set_native_compiler_hook(origin, Hook::Present);
        assert_eq!(namespaces.native_compiler_hook(old), Some(Hook::Absent));
        namespaces.bind(
            GLOBAL,
            b"new",
            Command::Imported {
                source: b"::origin".to_vec(),
                source_generation: origin,
                ensemble: None,
                identity: std::rc::Rc::new(crate::interp::ImportToken),
            },
        );
        let new = namespaces.resolve_generation(GLOBAL, b"new").unwrap();
        assert_eq!(namespaces.native_compiler_hook(new), Some(Hook::Present));
        assert!(namespaces.delete(GLOBAL, b"old"));
        assert_eq!(
            namespaces.native_compiler_cache_epochs(GLOBAL),
            Some((0, 0))
        );
        assert!(namespaces.delete(GLOBAL, b"new"));
        assert_eq!(
            namespaces.native_compiler_cache_epochs(GLOBAL),
            Some((1, 0))
        );
    }

    #[test]
    fn command_reference_epochs_follow_shadow_trails_and_reverse_path_entries() {
        let mut namespaces = Namespaces::new();
        let local = namespaces.ensure_namespace(GLOBAL, b"local");
        let watcher = namespaces.ensure_namespace(GLOBAL, b"watcher");
        namespaces.set_path(watcher, vec![local, local]);
        namespaces.bind(GLOBAL, b"p", cmd());
        let watcher_before = namespaces
            .native_command_reference(watcher)
            .command_reference_epoch;
        let local_before = namespaces
            .native_command_reference(local)
            .command_reference_epoch;
        namespaces.bind(local, b"p", cmd());
        // Fresh publication invalidates two path entries, then shadowing the
        // global p invalidates the local reference and the same two entries.
        assert_eq!(
            namespaces
                .native_command_reference(watcher)
                .command_reference_epoch,
            watcher_before + 4
        );
        assert_eq!(
            namespaces
                .native_command_reference(local)
                .command_reference_epoch,
            local_before + 1
        );
        let global_child = namespaces.ensure_namespace(GLOBAL, b"child");
        let local_child = namespaces.ensure_namespace(local, b"child");
        namespaces.bind(global_child, b"q", cmd());
        let before = namespaces
            .native_command_reference(local)
            .command_reference_epoch;
        namespaces.bind(local_child, b"q", cmd());
        assert_eq!(
            namespaces
                .native_command_reference(local)
                .command_reference_epoch,
            before + 1
        );
        assert_eq!(
            namespaces
                .native_command_reference(local_child)
                .command_reference_epoch,
            0
        );
    }

    #[test]
    fn c85_obj_creation_invalidates_reverse_paths_before_delete_callback_window() {
        let mut namespaces = Namespaces::new();
        namespaces.set_native_command_version(Some(tcl_dialect::TclVersion::V8_5));
        let source = namespaces.ensure_namespace(GLOBAL, b"source");
        let watcher = namespaces.ensure_namespace(GLOBAL, b"watcher");
        namespaces.set_path(watcher, vec![source, source]);
        namespaces.bind(source, b"p", cmd());
        let before = namespaces
            .native_command_reference(watcher)
            .command_reference_epoch;
        let entry = namespaces.native_command_creation_entry(source, b"p");
        assert_eq!(
            namespaces
                .native_command_reference(watcher)
                .command_reference_epoch,
            before + 2
        );
        namespaces.bind_after_native_creation_entry(entry, b"p", cmd());
        assert_eq!(
            namespaces
                .native_command_reference(watcher)
                .command_reference_epoch,
            before + 2
        );
        namespaces.set_native_command_version(Some(tcl_dialect::TclVersion::V8_4));
        let before = namespaces
            .native_command_reference(watcher)
            .command_reference_epoch;
        namespaces.bind(source, b"q", cmd());
        assert_eq!(
            namespaces
                .native_command_reference(watcher)
                .command_reference_epoch,
            before
        );
    }

    #[test]
    fn constructed_path_and_lookup_stay_on_detached_original_token() {
        let mut namespaces = Namespaces::new();
        let original = namespaces.ensure_namespace(GLOBAL, b"a:");
        let child = namespaces.ensure_child(original, b"x\xff");
        namespaces.bind(original, b"p", cmd());
        let path = namespaces.native_context_path(original).unwrap();
        assert_eq!(
            path.as_segments(),
            &[tcl_core_types::NameBytes::from(b"a:".as_slice())]
        );
        namespaces.activation_enter(original);
        drop(namespaces.defer_namespace(original));
        let replacement = namespaces.ensure_namespace(GLOBAL, b"a:");
        assert_ne!(replacement, original);
        assert_eq!(namespaces.native_context_path(original), Some(path.clone()));
        assert_eq!(namespaces.native_context_path(replacement), Some(path));
        assert_eq!(
            namespaces.native_context_path(child).unwrap().as_segments(),
            &[
                tcl_core_types::NameBytes::from(b"a:".as_slice()),
                tcl_core_types::NameBytes::from(b"x\xff".as_slice()),
            ]
        );
        assert!(namespaces.resolve(original, b"p").is_some());
        assert!(namespaces.resolve(replacement, b"p").is_none());
        assert!(namespaces.activation_leave(original));
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

#[cfg(test)]
mod original_variable_geometry_tests {
    use super::*;

    #[test]
    fn namespace_variable_roots_keep_qualification_before_zero_and_counted_simple_keys() {
        // Native proof: naming.tcloo.explicit-variable-link-counted-target
        // docs/design/analysis/name-resolution-proofs/explicit-variable-link-counted-target.md
        for version in tcl_dialect::TclVersion::ALL {
            let mut namespaces = Namespaces::new();
            namespaces.variable_name_protocol =
                Some(tcl_syntax::naming::NativeNameProtocol::C(version));
            let current = namespaces.ensure_namespace(GLOBAL, b"::N");
            let key = if version == tcl_dialect::TclVersion::V8_4 {
                b"k".as_slice()
            } else {
                b"k\0::Q".as_slice()
            };
            assert_eq!(
                namespaces.var_home(current, b"k\0::Q"),
                Some((current, key.to_vec()))
            );
            assert_eq!(
                namespaces.var_home(current, b"::N::k\0::Q"),
                Some((current, b"k".to_vec()))
            );
            assert_eq!(namespaces.var_home(current, b"missing::k"), None);
        }
    }
}
