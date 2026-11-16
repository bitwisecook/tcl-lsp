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

use crate::frame::VarTable;
use crate::interp::{BuiltinFn, Command, OoCommandRole};

mod jim_local;
#[cfg(test)]
mod jim_table_key_tests;
mod native_namespace_name;

mod command_publication;
mod command_surface;
mod jim_objects;
mod namespace_deletion;
mod namespace_lookup;
mod native_command_nodes;
mod rename;
mod variable_tables;

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

/// The half-finished `rename` a [`Namespaces::publish_rename_destination_with_original_key`]
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

/// Actual temporary placement, independent of the original target getter.
pub(crate) struct AliasRenameProbe {
    pub(crate) destination: (NsId, Vec<u8>),
    binding: CommandBinding,
    displaced: Option<CommandBinding>,
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
/// A Jim hash slot owns its key independently of the replaceable command value.
/// Cloned command transports never acquire this native reference.
#[derive(Default)]
struct CommandSlot {
    original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    binding: Option<CommandBinding>,
}

/// An actual current root-table entry, independent of its displayed name.
#[derive(Clone)]
pub(crate) struct OriginalJimCommandInventoryKey {
    pub(crate) generation: u64,
    pub(crate) report: Vec<u8>,
    original: crate::obj::NativeObjectLifetime,
}

impl OriginalJimCommandInventoryKey {
    pub(crate) fn original(
        &self,
    ) -> Result<*mut crate::obj::TclObj, tcl_syntax::value::ValueError> {
        self.original.checked_ptr()
    }
}

/// Extracted original owners, released after the namespace borrow. Jim releases
/// the key before the value on deletion; replacement contains only the value.
pub(crate) struct RetiredCommandSlot {
    procedure: Option<
        tcl_runtime_api::native_procedure_roles::NativeProcedureReference<crate::interp::ProcDef>,
    >,
    original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    binding: Option<CommandBinding>,
}
impl RetiredCommandSlot {
    pub(crate) fn retire_checked(
        mut self,
        current: &dyn Fn() -> Result<(), tcl_runtime_api::NativeExecutionError>,
    ) -> Result<(), tcl_runtime_api::NativeExecutionError> {
        let mut first = None;
        // C's existing Proc binding is withdrawn independently of command views.
        // Jim has no extracted Proc role and keeps its key-before-value release.
        drop(self.procedure.take());
        if let Err(error) = current() {
            first.get_or_insert(error);
        }
        drop(self.original_jim_key.take());
        if let Err(error) = current() {
            first.get_or_insert(error);
        }
        drop(self.binding.take());
        if let Err(error) = current() {
            first.get_or_insert(error);
        }
        first.map_or(Ok(()), Err)
    }
}

#[derive(Default)]
struct CommandTable {
    entries: BTreeMap<Vec<u8>, CommandSlot>,
    order: TclStringHashOrder,
    retired: std::cell::RefCell<Vec<RetiredCommandSlot>>,
}

impl CommandTable {
    fn binding(&self, key: &[u8]) -> Option<&CommandBinding> {
        self.entries.get(key)?.binding.as_ref()
    }
    fn get(&self, key: &[u8]) -> Option<&Command> {
        self.binding(key).map(|binding| &binding.command)
    }
    fn contains_key(&self, key: &[u8]) -> bool {
        self.binding(key).is_some()
    }
    fn keys(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.entries
            .iter()
            .filter_map(|(key, slot)| slot.binding.as_ref().map(|_| key))
    }
    fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Command)> {
        self.entries
            .iter()
            .filter_map(|(key, slot)| Some((key, &slot.binding.as_ref()?.command)))
    }
    fn values(&self) -> impl Iterator<Item = &Command> {
        self.entries
            .values()
            .filter_map(|slot| Some(&slot.binding.as_ref()?.command))
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut Command> {
        self.entries
            .values_mut()
            .filter_map(|slot| Some(&mut slot.binding.as_mut()?.command))
    }
    fn insert(
        &mut self,
        key: Vec<u8>,
        command: Command,
        generation: u64,
        jim_table_key: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
        original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    ) -> Option<Command> {
        let jim = jim_table_key.is_some();
        let slot = self.entries.entry(key.clone()).or_default();
        // Jim's keyDup takes its own native reference at a fresh hash slot.
        // A registration/qualification temporary is a distinct source role;
        // sharing its Rust Rc would erase the temporary reference during
        // publication. Occupied slots keep the original key unchanged.
        if slot.original_jim_key.is_none() {
            slot.original_jim_key = original_jim_key
                .as_ref()
                .map(|key| std::rc::Rc::new(crate::obj::Owned::retain(key.as_ptr())));
        }
        let unused_incoming = original_jim_key;
        let displaced = slot.binding.replace(CommandBinding {
            generation,
            command,
            jim_table_key,
        });
        if jim {
            self.order.insert(&key);
        } else if displaced.is_some() {
            self.order.reinsert(&key);
        } else {
            self.order.insert(&key);
        }
        let result = displaced.map(|binding| {
            let command = binding.command.clone();
            let procedure = if jim {
                None
            } else {
                match &binding.command {
                    Command::Proc(procedure) => procedure.take_for_retirement(),
                    _ => None,
                }
            };
            self.retired.borrow_mut().push(RetiredCommandSlot {
                procedure,
                original_jim_key: None,
                binding: Some(binding),
            });
            command
        });
        if unused_incoming.is_some() {
            self.retired.borrow_mut().push(RetiredCommandSlot {
                procedure: None,
                original_jim_key: unused_incoming,
                binding: None,
            });
        }
        result
    }
    fn remove_binding(&mut self, key: &[u8]) -> Option<CommandBinding> {
        let slot = self.entries.remove(key)?;
        self.order.remove(key);
        if slot.original_jim_key.is_some() {
            let result = slot.binding.clone();
            self.retired.borrow_mut().push(RetiredCommandSlot {
                procedure: None,
                original_jim_key: slot.original_jim_key,
                binding: slot.binding,
            });
            result
        } else {
            slot.binding
        }
    }
    /// Take only the value. The actual occupied Jim key survives local pushes
    /// and replacement; C's temporary alias-loop slots retain their hash entry.
    fn take_slot(&mut self, key: &[u8]) -> Option<CommandBinding> {
        self.entries.get_mut(key)?.binding.take()
    }
    fn restore_slot(&mut self, key: Vec<u8>, binding: CommandBinding) -> Option<CommandBinding> {
        self.order.insert(&key);
        self.entries
            .entry(key)
            .or_default()
            .binding
            .replace(binding)
    }
    fn rebind_slot(&mut self, key: &[u8], command: Command) {
        if let Some(binding) = self
            .entries
            .get_mut(key)
            .and_then(|slot| slot.binding.as_mut())
        {
            binding.command = command;
        }
    }
    fn reserve_entry(&mut self, key: &[u8]) {
        self.order.insert(key);
        self.entries.entry(key.to_vec()).or_default();
    }
    fn drop_entry(&mut self, key: &[u8]) {
        let _ = self.remove_binding(key);
    }
    fn drop_entry_with_binding(&mut self, key: &[u8], binding: CommandBinding) {
        let slot = self.entries.remove(key);
        self.order.remove(key);
        self.retired.borrow_mut().push(RetiredCommandSlot {
            procedure: None,
            original_jim_key: slot.and_then(|slot| slot.original_jim_key),
            binding: Some(binding),
        });
    }
    fn clear(&mut self) {
        let entries = std::mem::take(&mut self.entries);
        for (_, slot) in entries {
            self.retired.borrow_mut().push(RetiredCommandSlot {
                procedure: None,
                original_jim_key: slot.original_jim_key,
                binding: slot.binding,
            });
        }
        self.order.clear();
    }
    fn generation(&self, key: &[u8]) -> Option<u64> {
        self.binding(key).map(|binding| binding.generation)
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
    Published(Vec<std::rc::Weak<crate::obj::Owned>>),
}
impl JimNamespaceObject {
    fn owner(&self) -> Option<std::rc::Rc<crate::obj::Owned>> {
        match self {
            Self::Initial(owner) => Some(std::rc::Rc::clone(owner)),
            Self::Published(owners) => owners.iter().find_map(std::rc::Weak::upgrade),
        }
    }
}

enum JimNamespaceName {
    Unmaterialized,
    Materialized(std::rc::Rc<[u8]>),
}

struct Namespace {
    /// Actual Jim namespace object context, separate from C tree coordinates.
    jim_namespace: Option<(JimNamespaceObject, JimNamespaceName)>,
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
    namespace: NsId,
    deletion_started: bool,
    jim_command_name: Option<crate::obj::JimCommandNameObservation>,
}

/// The original creation entry, retained across re-entrant delete callbacks.
pub(crate) struct NativeCommandCreation {
    namespace: NsId,
    original_tail: Vec<u8>,
    occupied: bool,
    occupied_jim_key: Option<std::rc::Weak<crate::obj::Owned>>,
    jim_table_key: Option<tcl_syntax::naming::NativeJimCommandTableKey>,
    original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
}

/// The namespace tree + the command resolver.
pub(crate) struct RetiredJimCommands {
    _pending_slots: Vec<RetiredCommandSlot>,
    _tables: Vec<CommandTable>,
    _nodes: BTreeMap<u64, NativeCommandNode>,
    _previous: BTreeMap<u64, CommandBinding>,
}

pub(crate) struct OriginalPublicationRevision {
    world: std::rc::Weak<()>,
    revision: u64,
    single_unlink: Option<(u64, u64)>,
}

impl OriginalPublicationRevision {
    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.revision == other.revision
            && self.world.strong_count() != 0
            && self.world.ptr_eq(&other.world)
    }

    pub(crate) fn single_unlink_since(&self, previous: &Self) -> Option<u64> {
        let (revision, generation) = self.single_unlink?;
        (previous.revision == revision
            && revision.checked_add(1) == Some(self.revision)
            && previous.world.strong_count() != 0
            && previous.world.ptr_eq(&self.world))
        .then_some(generation)
    }
}

/// Lookup metadata bound to an actually selected command generation.
pub(crate) struct CommandGuardLookup {
    world: std::rc::Weak<()>,
    current: NsId,
    holder: NsId,
    generation: u64,
    path: Vec<NsId>,
    policy: u64,
}

pub struct Namespaces {
    arena: Vec<Namespace>,
    native_namespace_names: std::cell::RefCell<
        BTreeMap<NsId, tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken>,
    >,
    native_namespace_name_dead: BTreeSet<NsId>,
    native_command_nodes: BTreeMap<u64, NativeCommandNode>,
    original_publication_world: std::rc::Rc<()>,
    original_publication_revision: Option<u64>,
    original_publication_single_unlink: Option<(u64, u64)>,
    retired_ensemble_roles: std::cell::RefCell<Vec<std::rc::Rc<crate::ensemble::EnsembleToken>>>,
    native_command_version: Option<tcl_dialect::TclVersion>,
    native_compiler_epoch: Option<u64>,
    /// Software lookup-policy mutations, separate from native compiler epochs.
    guard_lookup_environment_revision: Option<u64>,
    guard_command_lookup_policy_revision: Option<u64>,
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
    pub(crate) fn original_publication_revision(&self) -> Option<OriginalPublicationRevision> {
        Some(OriginalPublicationRevision {
            world: std::rc::Rc::downgrade(&self.original_publication_world),
            revision: self.original_publication_revision?,
            single_unlink: self.original_publication_single_unlink,
        })
    }

    fn invalidate_original_publications(&mut self) {
        self.original_publication_single_unlink = None;
        self.original_publication_revision = self
            .original_publication_revision
            .and_then(|revision| revision.checked_add(1));
    }

    fn invalidate_single_original_unlink(&mut self, generation: u64) {
        let previous = self.original_publication_revision;
        self.invalidate_original_publications();
        if matches!(
            self.variable_name_protocol,
            Some(tcl_syntax::naming::NativeNameProtocol::C(_))
        ) && self.original_publication_revision.is_some()
        {
            self.original_publication_single_unlink =
                previous.map(|revision| (revision, generation));
        }
    }

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
        self.invalidate_original_publications();
        RetiredJimCommands {
            _pending_slots: self.take_retired_command_slots(),
            _tables: self
                .arena
                .iter_mut()
                .map(|namespace| std::mem::take(&mut namespace.commands))
                .collect(),
            _nodes: std::mem::take(&mut self.native_command_nodes),
            _previous: std::mem::take(&mut self.jim_previous_commands),
        }
    }

    pub(crate) fn take_retired_command_slots(&self) -> Vec<RetiredCommandSlot> {
        self.arena
            .iter()
            .flat_map(|namespace| std::mem::take(&mut *namespace.commands.retired.borrow_mut()))
            .collect()
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
        self.qualified_member_report(ns, simple)
    }

    /// The reporting parts of an already reached namespace: its Jim flat
    /// holder when it has one, otherwise its constructed C path.
    pub(crate) fn report_parts(&self, ns: NsId) -> tcl_syntax::naming::NativeNamespaceReportParts {
        if ns != GLOBAL && self.arena[ns].jim_namespace.is_some() {
            tcl_syntax::naming::NativeNamespaceReportParts::JimFlat(
                self.jim_namespace_bytes(ns)
                    .expect("Jim namespace reporting requires an already reached namespace getter")
                    .into(),
            )
        } else {
            tcl_syntax::naming::NativeNamespaceReportParts::CConstructed(
                self.arena[ns].constructed_path.clone(),
            )
        }
    }

    /// A member's qualified report under an already reached namespace.
    pub(crate) fn qualified_member_report(&self, ns: NsId, member: &[u8]) -> Vec<u8> {
        tcl_syntax::naming::native_qualified_inventory_report_bytes(&self.report_parts(ns), member)
    }

    /// Install one real command binding. Replacement deletes the displaced
    /// command token; the installed ensemble token acquires this binding's FQN.
    fn insert_bound(&mut self, ns: NsId, simple: Vec<u8>, command: Command) {
        let entry = self.native_command_creation_entry(ns, &simple);
        self.insert_bound_after_entry(entry, simple, command);
    }

    fn insert_bound_after_entry(
        &mut self,
        mut entry: NativeCommandCreation,
        simple: Vec<u8>,
        command: Command,
    ) -> u64 {
        let ns = entry.namespace;
        // C redefines a command by deleting the old hash entry and creating a
        // new one, so the name moves to its bucket head
        // (`TclCreateObjCommandInNs`).
        if let Some(displaced) = self.take_native_binding_for_replacement(ns, &simple) {
            Self::mark_command_deleted(&displaced.command, &self.retired_ensemble_roles);
            if self
                .variable_name_protocol
                .is_some_and(|protocol| protocol.is_jim084())
            {
                self.arena[ns]
                    .commands
                    .retired
                    .borrow_mut()
                    .push(RetiredCommandSlot {
                        procedure: None,
                        original_jim_key: None,
                        binding: Some(displaced),
                    });
            }
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
                namespace: ns,
                deletion_started: false,
                jim_command_name: None,
            },
        );
        let jim_table_key = entry.jim_table_key.clone();
        let original_jim_key = entry.original_jim_key.take();
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
        self.arena[ns].commands.insert(
            simple,
            command,
            generation,
            jim_table_key,
            original_jim_key,
        );
        generation
    }

    /// Publish an existing command token at a new placement. Visibility and
    /// rename moves retain the generation because they move Tcl's `Command *`;
    /// only a new definition goes through [`Self::insert_bound`].
    fn insert_moved_binding(
        &mut self,
        ns: NsId,
        simple: Vec<u8>,
        binding: CommandBinding,
        original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    ) {
        self.invalidate_original_publications();
        if let Some(displaced) = self.remove_native_binding(ns, &simple) {
            Self::mark_command_deleted(&displaced.command, &self.retired_ensemble_roles);
        }
        if let Command::Ensemble(token) = &binding.command {
            token.rename(self.command_fqn(ns, &simple));
        }
        let node = self
            .native_command_nodes
            .get_mut(&binding.generation)
            .expect("moved binding retains its native command node");
        node.namespace = ns;
        node.placement = Some((ns, simple.clone()));
        self.arena[ns].commands.insert(
            simple,
            binding.command,
            binding.generation,
            binding.jim_table_key,
            original_jim_key,
        );
    }

    /// A fresh tree with just the global namespace `::`.
    #[must_use]
    pub fn new() -> Namespaces {
        Namespaces {
            native_namespace_names: std::cell::RefCell::new(BTreeMap::new()),
            native_namespace_name_dead: BTreeSet::new(),
            native_command_nodes: BTreeMap::new(),
            original_publication_world: std::rc::Rc::new(()),
            original_publication_revision: Some(0),
            original_publication_single_unlink: None,
            retired_ensemble_roles: std::cell::RefCell::new(Vec::new()),
            native_command_version: Some(tcl_dialect::TclVersion::V9_0),
            native_compiler_epoch: Some(0),
            guard_lookup_environment_revision: Some(0),
            guard_command_lookup_policy_revision: Some(0),
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
                    JimNamespaceName::Materialized(std::rc::Rc::from(&b""[..])),
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
        let slot = tcl_syntax::naming::support_command_registration_slot(name)?;
        let mut ns = GLOBAL;
        for part in slot.namespace.as_segments() {
            ns = self.ensure_child(ns, part.as_bytes());
        }
        let simple = slot.simple.as_bytes().to_vec();
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

    /// Report the selected command's full name. Its generation remains the identity.
    #[must_use]
    pub fn resolve_fqn(&self, current: NsId, name: &[u8]) -> Option<Vec<u8>> {
        self.home_of(current, name)
            .map(|(namespace, simple)| self.command_fqn(namespace, &simple))
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
        self.invalidate_original_publications();
        self.arena[ns].commands.remove_binding(&simple)
    }

    #[cfg(test)]
    fn restore(&mut self, name: &[u8], binding: CommandBinding) -> bool {
        let Some(slot) = tcl_syntax::naming::support_command_registration_slot(name) else {
            return false;
        };
        let mut namespace = GLOBAL;
        for part in slot.namespace.as_segments() {
            namespace = self.ensure_child(namespace, part.as_bytes());
        }
        self.restore_binding_at_checked(
            Some((namespace, slot.simple.as_bytes().to_vec())),
            binding,
            None,
        )
        .is_ok()
    }

    /// Select the exposed C table's global slot from the original operand.
    pub(crate) fn exposed_command_publication_at(
        &self,
        original: &[u8],
    ) -> Option<(NsId, Vec<u8>)> {
        let slot = self
            .variable_name_protocol?
            .exposed_command_publication_slot(original)
            .ok()?;
        Some((GLOBAL, slot.simple.as_bytes().to_vec()))
    }

    /// Restore the original token in its independently selected physical slot.
    /// Unpublished owners retire outside the caller's table borrow.
    pub(crate) fn restore_binding_at_checked(
        &mut self,
        slot: Option<(NsId, Vec<u8>)>,
        binding: CommandBinding,
        original_jim_key: Option<std::rc::Rc<crate::obj::Owned>>,
    ) -> Result<(), RetiredCommandSlot> {
        let Some((namespace, simple)) = slot else {
            return Err(RetiredCommandSlot {
                procedure: None,
                original_jim_key,
                binding: Some(binding),
            });
        };
        let hook = self
            .native_compiler_hook(binding.generation)
            .expect("restored command retains its native node");
        self.note_native_compiler_mutation(
            None,
            tcl_registry::native_procedure::NativeCompilerCacheMutation::CommandToken { hook },
        );
        self.insert_moved_binding(namespace, simple, binding, original_jim_key);
        self.invalidate_native_command_lookup(namespace);
        Ok(())
    }

    pub(crate) fn guard_lookup_environment_revision(&self) -> Option<u64> {
        self.guard_lookup_environment_revision
    }

    #[cfg(test)]
    pub(crate) fn exhaust_guard_lookup_environment_revision_for_test(&mut self) {
        self.guard_lookup_environment_revision = Some(u64::MAX);
        self.note_guard_lookup_environment_mutation();
    }

    pub(crate) fn guard_command_lookup_policy_revision(&self) -> Option<u64> {
        self.guard_command_lookup_policy_revision
    }

    fn note_guard_command_lookup_policy_mutation(&mut self) {
        self.note_guard_lookup_environment_mutation();
        self.guard_command_lookup_policy_revision = self
            .guard_command_lookup_policy_revision
            .and_then(|revision| revision.checked_add(1));
    }

    /// Record the actual live publication after the caller's single lookup.
    pub(crate) fn command_guard_lookup(
        &self,
        current: NsId,
        generation: u64,
    ) -> Option<CommandGuardLookup> {
        self.guard_lookup_environment_revision?;
        let policy = self.guard_command_lookup_policy_revision?;
        let reference = self.arena.get(current)?;
        let node = self.native_command_nodes.get(&generation)?;
        let (holder, key) = node.placement.as_ref()?;
        if !self.namespace_is_live(current)
            || self.arena.get(*holder).is_none()
            || !self.namespace_is_live(*holder)
            || !self.original_publication_is_current(*holder, key, generation)
            || reference
                .path
                .iter()
                .any(|entry| self.arena.get(*entry).is_none() || !self.namespace_is_live(*entry))
        {
            return None;
        }
        Some(CommandGuardLookup {
            world: std::rc::Rc::downgrade(&self.original_publication_world),
            current,
            holder: *holder,
            generation,
            path: reference.path.clone(),
            policy,
        })
    }

    /// Compare only metadata; do not repeat name conversion or resolution.
    pub(crate) fn command_guard_lookup_is_current(
        &self,
        original: &CommandGuardLookup,
        current: NsId,
        generation: u64,
    ) -> bool {
        if original.current != current
            || original.generation != generation
            || original.world.strong_count() == 0
            || !original
                .world
                .ptr_eq(&std::rc::Rc::downgrade(&self.original_publication_world))
            || self.guard_lookup_environment_revision.is_none()
            || self.guard_command_lookup_policy_revision != Some(original.policy)
            || !self.namespace_is_live(current)
            || !self.namespace_is_live(original.holder)
        {
            return false;
        }
        let Some(reference) = self.arena.get(current) else {
            return false;
        };
        let Some((holder, key)) = self
            .native_command_nodes
            .get(&generation)
            .and_then(|node| node.placement.as_ref())
        else {
            return false;
        };
        *holder == original.holder
            && self.arena.get(*holder).is_some()
            && self.original_publication_is_current(*holder, key, generation)
            && reference.path == original.path
            && reference
                .path
                .iter()
                .all(|entry| self.arena.get(*entry).is_some() && self.namespace_is_live(*entry))
    }

    fn note_guard_lookup_environment_mutation(&mut self) {
        self.guard_lookup_environment_revision = self
            .guard_lookup_environment_revision
            .and_then(|revision| revision.checked_add(1));
    }

    // helpers

    /// Select the first existing command slot through shared original-name
    /// lookup bases, retaining this arena's namespace tokens and dying children.
    pub(crate) fn home_of(&self, current: NsId, name: &[u8]) -> Option<(NsId, Vec<u8>)> {
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
        for base in selected
            .lookup_bases(current, &self.arena[current].path)
            .ok()?
        {
            let namespace = match base {
                tcl_syntax::naming::CommandLookupBase::Context(namespace) => namespace,
                tcl_syntax::naming::CommandLookupBase::Root => GLOBAL,
            };
            if let Some(namespace) = find_under(namespace) {
                return Some((namespace, simple.to_vec()));
            }
        }
        None
    }

    fn ensure_child(&mut self, parent: NsId, name: &[u8]) -> NsId {
        if let Some(&id) = self.arena[parent].children.get(name) {
            return id;
        }
        // Existing child selection above is observation; only this new child
        // changes the shared namespace lookup environment.
        self.note_guard_lookup_environment_mutation();
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
mod tests;

#[cfg(test)]
mod original_variable_geometry_tests;

#[cfg(test)]
mod native_jim_context_tests;
