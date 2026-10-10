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

//! Owned native namespace and command-address storage.
//!
//! Publication services share the interpreter's actual tables through weak
//! ownership. A prepared address retains a namespace incarnation; consuming it
//! never repeats written-name resolution. Pending native changes participate in
//! lookup before the embedding registrar commits command implementations.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use tcl_cmd_core::namespace::TclStringHashOrder;
use tcl_core_types::{ByteNamespacePath, NameBytes, NsId, ROOT_NS};
use tcl_syntax::naming::{
    NameProjectionUnavailable, NativeCommandNamespaceRoute, NativeNameContext, NativeNameProtocol,
};

use super::{CommandIdentityIndex, CommandSlot, NamespaceDeferral};

/// Effects that require an executing VM when retiring a command binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeDeletionEffects {
    /// Native delete procedures are owned by the C bridge and run there.
    NoGuestExecution,
    /// Script observers or dependent binding retirement require synchronous VM entry.
    GuestExecutionRequired,
}

/// An interpreter's authoritative native namespace and command-address tables.
pub(crate) struct NativeNameWorld {
    pub(super) execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    owner: u64,
    interpreter: u64,
    live: bool,
    pub(super) command_identity: CommandIdentityIndex,
    pub(super) ns_arena: Vec<ByteNamespacePath>,
    pub(super) ns_intern: HashMap<ByteNamespacePath, NsId>,
    pub(super) ns_parents: Vec<Option<NsId>>,
    pub(super) ns_children: HashMap<(NsId, NameBytes), NsId>,
    pub(super) namespaces: HashSet<ByteNamespacePath>,
    pub(super) dying_namespaces: HashSet<NsId>,
    pub(super) dead_namespaces: HashSet<NsId>,
    pub(super) ns_deferral: NamespaceDeferral,
    pub(super) ns_child_order: HashMap<NsId, TclStringHashOrder>,
    pub(super) ns_command_order: HashMap<NsId, TclStringHashOrder>,
    pub(super) ns_paths: HashMap<NsId, Vec<NsId>>,
    /// Native referencing-namespace epochs, independent of compiler/resolver guards.
    pub(super) command_reference_epochs: HashMap<NsId, u64>,
    /// Command-node epochs survive placement changes without retaining implementations.
    pub(super) command_name_epochs: HashMap<u64, u64>,
    /// Nodes whose deleteProc phase advanced the epoch before final unlink.
    pub(super) command_name_retiring: HashSet<u64>,
    /// Delete-trace completion has published this token's compiler mutation.
    pub(super) command_compiler_deletion_published: HashSet<u64>,
    /// Exact Jim namespace objects, independent of constructed analytical paths.
    pub(super) jim_namespace_objects: HashMap<NsId, NameBytes>,
    /// Actual Jim top-frame owner of the interpreter's original empty object.
    pub(super) jim_root_namespace_object: Option<crate::Value>,
    /// Native Jim procedure epoch; neither compiler nor C lookup epochs supply it.
    pub(super) jim_procedure_epoch: u64,
    pub(super) jim_local_depth: usize,
    /// Actual table owners; object caches carry only token/epoch plus original namespace.
    pub(super) jim_command_nodes: HashMap<u64, Rc<super::native_jim_lookup::JimCommandNode>>,
    /// Original key spelling belongs to the table entry, independently of its command generation.
    pub(super) jim_command_table_keys:
        HashMap<String, tcl_syntax::naming::NativeJimCommandTableKey>,
    /// Retired active nodes remain discoverable only through their original descriptor.
    pub(super) jim_command_receipts: HashMap<u64, Weak<super::native_jim_lookup::JimCommandNode>>,
    pub(super) jim_retired_commands: usize,
    pub(super) jim_retired_tokens: HashSet<u64>,
    /// Actual namespace-cache state, independent of public namespace membership.
    pub(super) namespace_name_tokens:
        HashMap<NsId, tcl_runtime_api::native_namespace_name::NativeNamespaceNameToken>,
    deletion_effects: HashMap<CommandSlot, (u64, NativeDeletionEffects)>,
    pending: HashMap<CommandSlot, PublicationOverlay>,
    next_revision: u64,
}

#[derive(Clone, Copy, Debug)]
struct PublicationOverlay {
    revision: u64,
    present: bool,
}

impl NativeNameWorld {
    pub(super) fn new(owner: u64, interpreter: u64) -> Self {
        let root = ByteNamespacePath::root();
        Self {
            execution_name_policy: None,
            owner,
            interpreter,
            live: true,
            command_identity: CommandIdentityIndex::default(),
            ns_arena: vec![root.clone()],
            ns_intern: HashMap::from([(root.clone(), ROOT_NS)]),
            ns_parents: vec![None],
            ns_children: HashMap::new(),
            namespaces: HashSet::from([root]),
            dying_namespaces: HashSet::new(),
            dead_namespaces: HashSet::new(),
            ns_deferral: NamespaceDeferral::default(),
            ns_child_order: super::root_hash_order(),
            ns_command_order: super::root_hash_order(),
            ns_paths: HashMap::new(),
            command_reference_epochs: HashMap::new(),
            command_name_epochs: HashMap::new(),
            command_name_retiring: HashSet::new(),
            command_compiler_deletion_published: HashSet::new(),
            jim_namespace_objects: HashMap::from([(ROOT_NS, NameBytes::default())]),
            jim_root_namespace_object: None,
            jim_procedure_epoch: 1,
            jim_local_depth: 0,
            jim_command_nodes: HashMap::new(),
            jim_command_table_keys: HashMap::new(),
            jim_command_receipts: HashMap::new(),
            jim_retired_commands: 0,
            jim_retired_tokens: HashSet::new(),
            namespace_name_tokens: HashMap::new(),
            deletion_effects: HashMap::new(),
            pending: HashMap::new(),
            next_revision: 0,
        }
    }

    /// Invalidate capabilities before interpreter storage is retired.
    pub(super) fn retire(&mut self) {
        self.live = false;
        self.pending.clear();
        for token in self.namespace_name_tokens.values() {
            token.mark_dead();
        }
        self.jim_root_namespace_object = None;
        for node in self.jim_command_nodes.values() {
            node.unpublish();
        }
        self.jim_command_nodes.clear();
        self.jim_command_table_keys.clear();
        self.jim_command_receipts.clear();
    }

    /// Record effects for one actual command-token generation.
    pub(super) fn note_deletion_effects(
        &mut self,
        slot: CommandSlot,
        generation: u64,
        effects: NativeDeletionEffects,
    ) {
        self.deletion_effects.insert(slot, (generation, effects));
    }

    /// Retire metadata only for the generation that actually disappeared.
    #[cfg(test)]
    pub(super) fn remove_deletion_effects(&mut self, slot: &CommandSlot, generation: u64) {
        if self
            .deletion_effects
            .get(slot)
            .is_some_and(|(current, _)| *current == generation)
        {
            self.deletion_effects.remove(slot);
        }
    }

    /// Pending native publication is authoritative over committed placement.
    pub(super) fn observed_slot_presence(&self, slot: &CommandSlot) -> bool {
        self.pending.get(slot).map_or_else(
            || self.generation_at(slot).is_some(),
            |pending| pending.present,
        )
    }

    pub(super) const fn is_live(&self) -> bool {
        self.live
    }

    pub(super) fn pending_publications_are_committed(&self) -> bool {
        self.pending.is_empty()
    }

    /// Return a native pending disposition without consulting display spellings.
    pub(super) fn pending_slot_presence(&self, slot: &CommandSlot) -> Option<bool> {
        self.pending.get(slot).map(|pending| pending.present)
    }

    fn namespace_available(&self, namespace: NsId) -> bool {
        self.ns_arena.get(namespace.0 as usize).is_some()
            && (!self.dead_namespaces.contains(&namespace)
                || self.ns_deferral.owners.contains_key(&namespace)
                || self.dying_namespaces.contains(&namespace))
    }

    fn namespace_context(
        &self,
        namespace: NsId,
        protocol: NativeNameProtocol,
    ) -> Result<NativeNameContext<'_>, NativePublicationError> {
        if !self.namespace_available(namespace) {
            return Err(NativePublicationError::NamespaceExpired);
        }
        let path = &self.ns_arena[namespace.0 as usize];
        if protocol.is_jim084() {
            self.jim_namespace_objects
                .get(&namespace)
                .map(|object| NativeNameContext::with_jim_namespace(path, object.as_bytes()))
                .ok_or(NativePublicationError::Projection(
                    NameProjectionUnavailable::MissingJimNamespaceObject,
                ))
        } else {
            Ok(NativeNameContext::new(path))
        }
    }

    /// Follow the actual retained or live parent edge; same-spelled tokens stay distinct.
    pub(super) fn namespace_child_token(&self, parent: NsId, simple: &[u8]) -> Option<NsId> {
        if let Some(root) = self.ns_deferral.owners.get(&parent)
            && let Some(record) = self.ns_deferral.retained.get(root)
        {
            return record.subtree.values().find_map(|candidate| {
                (self.ns_parents.get(candidate.0 as usize).copied().flatten() == Some(parent)
                    && self
                        .ns_arena
                        .get(candidate.0 as usize)
                        .and_then(|path| path.last())
                        .is_some_and(|tail| tail.as_bytes() == simple))
                .then_some(*candidate)
            });
        }
        self.ns_children
            .get(&(parent, NameBytes::from(simple)))
            .copied()
            .or_else(|| {
                self.dying_namespaces.iter().find_map(|candidate| {
                    (self.ns_parents.get(candidate.0 as usize).copied().flatten() == Some(parent)
                        && self
                            .ns_arena
                            .get(candidate.0 as usize)
                            .and_then(|path| path.last())
                            .is_some_and(|tail| tail.as_bytes() == simple))
                    .then_some(*candidate)
                })
            })
    }

    fn create_child(
        &mut self,
        parent: NsId,
        simple: &NameBytes,
    ) -> Result<NsId, NativePublicationError> {
        if let Some(existing) = self.namespace_child_token(parent, simple.as_bytes()) {
            return Ok(existing);
        }
        if !self.namespace_available(parent) {
            return Err(NativePublicationError::NamespaceExpired);
        }
        let id = NsId(
            u32::try_from(self.ns_arena.len())
                .map_err(|_| NativePublicationError::NamespaceIdExhausted)?,
        );
        let path = self.ns_arena[parent.0 as usize].with_child(simple.clone());
        self.ns_arena.push(path.clone());
        self.ns_parents.push(Some(parent));
        if let Some(root) = self.ns_deferral.owners.get(&parent).copied() {
            let record = self
                .ns_deferral
                .retained
                .get_mut(&root)
                .ok_or(NativePublicationError::NamespaceExpired)?;
            record.subtree.insert(path, id);
            record
                .child_orders
                .entry(parent)
                .or_default()
                .insert(simple.as_bytes());
            self.ns_deferral.owners.insert(id, root);
        } else {
            self.ns_intern.insert(path.clone(), id);
            self.namespaces.insert(path);
            self.ns_children.insert((parent, simple.clone()), id);
            self.ns_child_order
                .entry(parent)
                .or_default()
                .insert(simple.as_bytes());
        }
        Ok(id)
    }

    fn holder_from_qualifiers(
        &mut self,
        base: NsId,
        qualifiers: &[NameBytes],
        create: bool,
    ) -> Result<Option<NsId>, NativePublicationError> {
        let mut selected = base;
        for tail in qualifiers {
            match self.namespace_child_token(selected, tail.as_bytes()) {
                Some(child) => selected = child,
                None if create => selected = self.create_child(selected, tail)?,
                None => return Ok(None),
            }
        }
        Ok(Some(selected))
    }

    /// Actual parent metadata after active deletion detaches a retained root.
    /// Descendants retain their physical parent inside the retained subtree.
    pub(super) fn physical_namespace_parent(&self, namespace: NsId) -> Option<NsId> {
        if self.ns_deferral.retained.contains_key(&namespace) {
            None
        } else {
            self.ns_parents.get(namespace.0 as usize).copied().flatten()
        }
    }

    fn selected_create_slot(
        &mut self,
        context: NsId,
        protocol: NativeNameProtocol,
        original: &[u8],
    ) -> Result<CommandSlot, NativePublicationError> {
        let native_context = self.namespace_context(context, protocol)?;
        let projected = protocol.command_c_api_publication_projection(native_context, original)?;
        let base = match projected.namespace_route() {
            NativeCommandNamespaceRoute::Root => ROOT_NS,
            NativeCommandNamespaceRoute::Context => context,
            NativeCommandNamespaceRoute::ContextParent => {
                self.physical_namespace_parent(context).unwrap_or(context)
            }
        };
        let namespace = self
            .holder_from_qualifiers(base, projected.qualifiers(), true)?
            .ok_or(NativePublicationError::NamespaceExpired)?;
        Ok(CommandSlot {
            namespace,
            simple: projected.into_slot().simple,
        })
    }

    fn lookup_slot(
        &mut self,
        context: NsId,
        protocol: NativeNameProtocol,
        original: &[u8],
    ) -> Result<Option<CommandSlot>, NativePublicationError> {
        if protocol.is_jim084() {
            let keys = protocol
                .jim_command_lookup_keys(self.namespace_context(context, protocol)?, original)?;
            return Ok(keys.into_iter().find_map(|simple| {
                let slot = CommandSlot {
                    namespace: ROOT_NS,
                    simple,
                };
                self.observed_slot_presence(&slot).then_some(slot)
            }));
        }
        let projected = protocol
            .command_lookup_projection(self.namespace_context(context, protocol)?, original)?;
        let absolute = projected.namespace_route() == NativeCommandNamespaceRoute::Root;
        let mut bases = if absolute {
            vec![ROOT_NS]
        } else {
            vec![context]
        };
        if !absolute
            && protocol
                .tcl_version()
                .is_some_and(tcl_dialect::TclVersion::has_namespace_path)
        {
            let paths = self
                .ns_deferral
                .owners
                .get(&context)
                .and_then(|root| self.ns_deferral.retained.get(root))
                .and_then(|record| record.paths.get(&context))
                .or_else(|| self.ns_paths.get(&context));
            if let Some(paths) = paths {
                bases.extend(paths.iter().copied());
            }
        }
        if !absolute && !bases.contains(&ROOT_NS) {
            bases.push(ROOT_NS);
        }
        for base in bases {
            if !self.namespace_available(base) {
                continue;
            }
            let projected = protocol
                .command_lookup_projection(self.namespace_context(base, protocol)?, original)?;
            let anchor = match projected.namespace_route() {
                NativeCommandNamespaceRoute::Root => ROOT_NS,
                NativeCommandNamespaceRoute::Context => base,
                NativeCommandNamespaceRoute::ContextParent => {
                    self.physical_namespace_parent(base).unwrap_or(base)
                }
            };
            let Some(namespace) =
                self.holder_from_qualifiers(anchor, projected.qualifiers(), false)?
            else {
                continue;
            };
            let slot = CommandSlot {
                namespace,
                simple: projected.into_slot().simple,
            };
            if self.observed_slot_presence(&slot) {
                return Ok(Some(slot));
            }
        }
        Ok(None)
    }

    fn generation_at(&self, slot: &CommandSlot) -> Option<u64> {
        let key = self.command_identity.by_slot.get(slot)?;
        self.command_identity
            .generations
            .get(key)
            .copied()
            .or_else(|| {
                self.ns_deferral
                    .owners
                    .get(&slot.namespace)
                    .and_then(|root| self.ns_deferral.retained.get(root))
                    .and_then(|record| record.generations.get(key))
                    .copied()
            })
    }

    fn preflight_retirement(&self, slot: &CommandSlot) -> Result<(), NativePublicationError> {
        if self.pending.contains_key(slot) {
            // Native callbacks run in the bridge; a tombstone has already retired its binding.
            return Ok(());
        }
        // Rename may reserve an address before publishing a command token.
        // An address index entry alone does not prove a binding is present.
        let Some(generation) = self.generation_at(slot) else {
            return Ok(());
        };
        let effects = self
            .deletion_effects
            .get(slot)
            .filter(|(classified, _)| *classified == generation)
            .map(|(_, effects)| *effects)
            .ok_or(NativePublicationError::UnclassifiedDeletionEffects)?;
        match effects {
            NativeDeletionEffects::NoGuestExecution => Ok(()),
            NativeDeletionEffects::GuestExecutionRequired => {
                Err(NativePublicationError::GuestDeletionExecutionRequired)
            }
        }
    }
}

impl Drop for NativeNameWorld {
    fn drop(&mut self) {
        self.retire();
    }
}

/// Native operation whose address is selected before delete callbacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativePublicationPurpose {
    CreateCCommand,
    DeleteCCommand,
}

/// Reporting key for the selected address; it is not an authority receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativePublicationKey {
    pub owner: u64,
    pub interpreter: u64,
    /// An actual `NsId`, or `u64::MAX` for a proved absent lookup.
    pub namespace: u64,
    pub simple: NameBytes,
}

#[derive(Clone)]
enum PublicationAddress {
    Slot(CommandSlot),
    Missing(NameBytes),
}

struct PublicationReceipt {
    world: Weak<RefCell<NativeNameWorld>>,
    owner: u64,
    interpreter: u64,
    original: Rc<[u8]>,
    address: PublicationAddress,
    purpose: NativePublicationPurpose,
    observed_binding_generation: Option<u64>,
    revision: Cell<Option<u64>>,
    present: Cell<Option<bool>>,
    consumed: Cell<bool>,
}

/// An engine-issued receipt retaining the exact selected namespace incarnation.
#[derive(Clone)]
pub struct NativePreparedPublication(Rc<PublicationReceipt>);

impl std::fmt::Debug for NativePreparedPublication {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativePreparedPublication")
            .field("key", &self.key())
            .field("original", &self.original())
            .finish_non_exhaustive()
    }
}

impl NativePreparedPublication {
    /// Original supplied bytes retained exclusively for reporting.
    #[must_use]
    pub fn original(&self) -> Rc<[u8]> {
        Rc::clone(&self.0.original)
    }

    /// A reporting key cannot be consumed in place of this opaque receipt.
    #[must_use]
    pub fn key(&self) -> NativePublicationKey {
        let (namespace, simple) = match &self.0.address {
            PublicationAddress::Slot(slot) => (u64::from(slot.namespace.0), slot.simple.clone()),
            PublicationAddress::Missing(selected) => (u64::MAX, selected.clone()),
        };
        NativePublicationKey {
            owner: self.0.owner,
            interpreter: self.0.interpreter,
            namespace,
            simple,
        }
    }

    /// The selected entry kind, independent of the written bytes.
    #[must_use]
    pub fn purpose(&self) -> NativePublicationPurpose {
        self.0.purpose
    }

    /// Committed binding generation observed during preparation, when present.
    /// This is the previous binding, never a token for the replacement being
    /// published. Pending native bindings have bridge-owned incarnation identity
    /// until VM registration issues their actual generation.
    #[must_use]
    pub fn observed_binding_generation(&self) -> Option<u64> {
        self.0.observed_binding_generation
    }
}

/// Typed host failures; none is a guest Tcl completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativePublicationError {
    /// No authenticated native name recipe is available.
    ProtocolUnavailable,
    ScopeExpired,
    ForeignReceipt,
    NamespaceExpired,
    NamespaceIdExhausted,
    InvalidConstructedPath,
    RevisionExhausted,
    UnclassifiedDeletionEffects,
    GuestDeletionExecutionRequired,
    ReceiptNotNoted,
    ReceiptAlreadyNoted,
    ReceiptAlreadyConsumed,
    WrongDisposition,
    Projection(NameProjectionUnavailable),
}

impl From<NameProjectionUnavailable> for NativePublicationError {
    fn from(error: NameProjectionUnavailable) -> Self {
        Self::Projection(error)
    }
}

impl std::fmt::Display for NativePublicationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::ProtocolUnavailable => "native publication name protocol is unavailable",
            Self::ScopeExpired => "native publication interpreter has retired",
            Self::ForeignReceipt => "native publication receipt belongs to another interpreter",
            Self::NamespaceExpired => "native publication namespace incarnation has retired",
            Self::NamespaceIdExhausted => "native publication namespace token space is exhausted",
            Self::InvalidConstructedPath => {
                "native publication path does not extend the actual context"
            }
            Self::RevisionExhausted => "native publication revision space is exhausted",
            Self::UnclassifiedDeletionEffects => "native command deletion effects are unavailable",
            Self::GuestDeletionExecutionRequired => {
                "native command retirement requires synchronous guest execution"
            }
            Self::ReceiptNotNoted => "native publication receipt has no pending disposition",
            Self::ReceiptAlreadyNoted => "native publication receipt was already noted",
            Self::ReceiptAlreadyConsumed => "native publication receipt was already consumed",
            Self::WrongDisposition => {
                "native publication disposition does not match its issued purpose"
            }
            Self::Projection(_) => "native command address projection is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for NativePublicationError {}

/// An owned callback-scope capability, retaining no executing VM borrow.
#[derive(Clone)]
pub struct NativePublicationService {
    world: Weak<RefCell<NativeNameWorld>>,
    owner: u64,
    interpreter: u64,
    context: NsId,
    protocol: NativeNameProtocol,
}

/// Registrar action selected after authenticated pending-revision validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativePublicationCommit {
    /// A newer callback mutation owns the slot; this queued change has no effect.
    Superseded,
    /// Install or remove at this retained exact slot, without written-name parsing.
    Apply { slot: CommandSlot, present: bool },
}

impl NativePublicationService {
    /// Issued only by the VM's actual native capability ingress.
    pub(super) fn new(
        world: &Rc<RefCell<NativeNameWorld>>,
        context: NsId,
        protocol: NativeNameProtocol,
    ) -> Result<Self, NativePublicationError> {
        let state = world.borrow();
        if !state.live {
            return Err(NativePublicationError::ScopeExpired);
        }
        state.namespace_context(context, protocol)?;
        Ok(Self {
            world: Rc::downgrade(world),
            owner: state.owner,
            interpreter: state.interpreter,
            context,
            protocol,
        })
    }

    pub(super) fn validate_world(
        &self,
        expected: &Rc<RefCell<NativeNameWorld>>,
    ) -> Result<(), NativePublicationError> {
        let actual = self.world()?;
        if Rc::ptr_eq(&actual, expected) {
            Ok(())
        } else {
            Err(NativePublicationError::ForeignReceipt)
        }
    }

    fn world(&self) -> Result<Rc<RefCell<NativeNameWorld>>, NativePublicationError> {
        let world = self
            .world
            .upgrade()
            .ok_or(NativePublicationError::ScopeExpired)?;
        {
            let state = world.borrow();
            if !state.live || state.owner != self.owner || state.interpreter != self.interpreter {
                return Err(NativePublicationError::ScopeExpired);
            }
        }
        Ok(world)
    }

    fn authenticate(
        &self,
        receipt: &NativePreparedPublication,
    ) -> Result<Rc<RefCell<NativeNameWorld>>, NativePublicationError> {
        let world = self.world()?;
        let issued_world = receipt
            .0
            .world
            .upgrade()
            .ok_or(NativePublicationError::ScopeExpired)?;
        if !Rc::ptr_eq(&world, &issued_world)
            || receipt.0.owner != self.owner
            || receipt.0.interpreter != self.interpreter
        {
            return Err(NativePublicationError::ForeignReceipt);
        }
        if receipt.0.consumed.get() {
            return Err(NativePublicationError::ReceiptAlreadyConsumed);
        }
        if let PublicationAddress::Slot(slot) = &receipt.0.address
            && !world.borrow().namespace_available(slot.namespace)
        {
            return Err(NativePublicationError::NamespaceExpired);
        }
        Ok(world)
    }

    /// Select and retain an actual address before native delete callbacks run.
    /// Creation publishes missing namespace edges immediately, without publishing
    /// the new command binding. Deletion never creates a missing namespace.
    ///
    /// # Errors
    /// Refuses expired scope, unsupported native policy, or guest retirement
    /// effects that this owned callback service cannot execute synchronously.
    pub fn prepare(
        &self,
        original: &[u8],
        purpose: NativePublicationPurpose,
    ) -> Result<NativePreparedPublication, NativePublicationError> {
        let world = self.world()?;
        let mut state = world.borrow_mut();
        state.namespace_context(self.context, self.protocol)?;
        let address = match purpose {
            NativePublicationPurpose::CreateCCommand => PublicationAddress::Slot(
                state.selected_create_slot(self.context, self.protocol, original)?,
            ),
            NativePublicationPurpose::DeleteCCommand => {
                match state.lookup_slot(self.context, self.protocol, original)? {
                    Some(slot) => PublicationAddress::Slot(slot),
                    None => PublicationAddress::Missing(NameBytes::from(
                        self.protocol
                            .command_lookup_input(
                                state.namespace_context(self.context, self.protocol)?,
                                original,
                            )?
                            .selected(),
                    )),
                }
            }
        };
        if let PublicationAddress::Slot(slot) = &address {
            state.preflight_retirement(slot)?;
        }
        let observed_binding_generation = match &address {
            PublicationAddress::Slot(slot) if !state.pending.contains_key(slot) => {
                state.generation_at(slot)
            }
            PublicationAddress::Slot(_) | PublicationAddress::Missing(_) => None,
        };
        Ok(NativePreparedPublication(Rc::new(PublicationReceipt {
            world: Rc::downgrade(&world),
            owner: self.owner,
            interpreter: self.interpreter,
            original: Rc::from(original),
            address,
            purpose,
            observed_binding_generation,
            revision: Cell::new(None),
            present: Cell::new(None),
            consumed: Cell::new(false),
        })))
    }

    /// Inspect committed and pending native presence at the issued exact slot.
    ///
    /// # Errors
    /// Refuses foreign, consumed or expired receipts.
    pub fn observed_presence(
        &self,
        receipt: &NativePreparedPublication,
    ) -> Result<bool, NativePublicationError> {
        let world = self.authenticate(receipt)?;
        Ok(match &receipt.0.address {
            PublicationAddress::Slot(slot) => world.borrow().observed_slot_presence(slot),
            PublicationAddress::Missing(_) => false,
        })
    }

    /// Publish a pending disposition after bridge-local mutation and callbacks.
    /// Revisions follow actual mutation order; outer creation therefore wins
    /// over creation performed by its old command's delete callback.
    ///
    /// # Errors
    /// Refuses foreign, consumed, already noted or wrongly disposed receipts.
    pub fn note_publication(
        &self,
        receipt: &NativePreparedPublication,
        present: bool,
    ) -> Result<(), NativePublicationError> {
        let world = self.authenticate(receipt)?;
        if receipt.0.revision.get().is_some() {
            return Err(NativePublicationError::ReceiptAlreadyNoted);
        }
        if present != (receipt.0.purpose == NativePublicationPurpose::CreateCCommand) {
            return Err(NativePublicationError::WrongDisposition);
        }
        let PublicationAddress::Slot(slot) = &receipt.0.address else {
            return Err(NativePublicationError::WrongDisposition);
        };
        let mut state = world.borrow_mut();
        let revision = state
            .next_revision
            .checked_add(1)
            .ok_or(NativePublicationError::RevisionExhausted)?;
        state.next_revision = revision;
        state
            .pending
            .insert(slot.clone(), PublicationOverlay { revision, present });
        receipt.0.revision.set(Some(revision));
        receipt.0.present.set(Some(present));
        Ok(())
    }

    /// Validate a queued change without consuming or clearing its pending revision.
    ///
    /// # Errors
    /// Refuses foreign, expired, consumed or unnoted receipts.
    pub fn inspect_commit(
        &self,
        receipt: &NativePreparedPublication,
    ) -> Result<NativePublicationCommit, NativePublicationError> {
        let world = self.authenticate(receipt)?;
        let revision = receipt
            .0
            .revision
            .get()
            .ok_or(NativePublicationError::ReceiptNotNoted)?;
        let present = receipt
            .0
            .present
            .get()
            .ok_or(NativePublicationError::ReceiptNotNoted)?;
        let PublicationAddress::Slot(slot) = &receipt.0.address else {
            return Err(NativePublicationError::WrongDisposition);
        };
        Ok(
            if world
                .borrow()
                .pending
                .get(slot)
                .is_some_and(|pending| pending.revision == revision)
            {
                NativePublicationCommit::Apply {
                    slot: slot.clone(),
                    present,
                }
            } else {
                NativePublicationCommit::Superseded
            },
        )
    }

    /// Consume after the registrar has successfully applied or superseded the change.
    /// Clears only its own pending revision, preserving reentrant replacements.
    ///
    /// # Errors
    /// Refuses foreign, expired, consumed or unnoted receipts.
    pub fn finish_commit(
        &self,
        receipt: &NativePreparedPublication,
    ) -> Result<(), NativePublicationError> {
        let world = self.authenticate(receipt)?;
        let revision = receipt
            .0
            .revision
            .get()
            .ok_or(NativePublicationError::ReceiptNotNoted)?;
        if let PublicationAddress::Slot(slot) = &receipt.0.address {
            let mut state = world.borrow_mut();
            if state
                .pending
                .get(slot)
                .is_some_and(|pending| pending.revision == revision)
            {
                state.pending.remove(slot);
            }
        }
        receipt.0.consumed.set(true);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::TclVersion;

    #[test]
    fn vm_native_namespace_ingress_projects_exact_bytes_under_all_c_releases() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let mut vm = super::super::Vm::new();
            vm.set_runtime_version(version);
            let context = vm.intern_ns_path(ByteNamespacePath::from_segments(["a:"]));
            let selected = vm
                .namespace_address_from_native_bytes(context, b"q\xff::r\0::hidden")
                .unwrap();
            assert_eq!(
                selected,
                super::super::NativeNamespaceAddress::C {
                    path: ByteNamespacePath::from_segments([
                        b"a:".as_slice(),
                        b"q\xff".as_slice(),
                        b"r".as_slice(),
                    ]),
                    absolute: false,
                }
            );
            let absolute = vm
                .namespace_address_from_native_bytes(context, b"::q\xff")
                .unwrap();
            assert_eq!(
                absolute,
                super::super::NativeNamespaceAddress::C {
                    path: ByteNamespacePath::from_segments([b"q\xff".as_slice()]),
                    absolute: true,
                }
            );
        }
    }

    #[test]
    fn vm_namespace_reporting_keeps_opaque_components_and_checks_unicode() {
        let mut vm = super::super::Vm::new();
        let namespace = vm.intern_ns_path(ByteNamespacePath::from_segments([
            b"a:".as_slice(),
            b"q\xff\0tail".as_slice(),
        ]));
        assert_eq!(vm.ns_name_bytes(namespace).as_bytes(), b"a:::q\xff\0tail");
        assert_eq!(vm.ns_name_checked_utf8(namespace), None);
    }

    fn world() -> Rc<RefCell<NativeNameWorld>> {
        Rc::new(RefCell::new(NativeNameWorld::new(11, 7)))
    }

    fn service(
        world: &Rc<RefCell<NativeNameWorld>>,
        context: NsId,
        version: TclVersion,
    ) -> NativePublicationService {
        NativePublicationService::new(world, context, NativeNameProtocol::C(version)).unwrap()
    }

    fn install(world: &Rc<RefCell<NativeNameWorld>>, slot: CommandSlot, generation: u64) {
        let mut state = world.borrow_mut();
        let key = format!("private-token-{generation}");
        state
            .command_identity
            .by_slot
            .insert(slot.clone(), key.clone());
        state
            .command_identity
            .slot_by_key
            .insert(key.clone(), slot.clone());
        state.command_identity.generations.insert(key, generation);
        state.note_deletion_effects(slot, generation, NativeDeletionEffects::NoGuestExecution);
    }

    #[test]
    fn reserved_slot_without_a_command_generation_is_absent() {
        let world = world();
        let slot = CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from(b"reserved"),
        };
        {
            let mut state = world.borrow_mut();
            state
                .command_identity
                .by_slot
                .insert(slot.clone(), "reservation".into());
            state
                .command_identity
                .slot_by_key
                .insert("reservation".into(), slot.clone());
        }
        let service = service(&world, ROOT_NS, TclVersion::V9_0);
        let missing = service
            .prepare(b"reserved", NativePublicationPurpose::DeleteCCommand)
            .unwrap();
        assert!(!service.observed_presence(&missing).unwrap());
        assert_eq!(missing.key().namespace, u64::MAX);
        let publication = service
            .prepare(b"reserved", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        assert_eq!(publication.key().namespace, u64::from(ROOT_NS.0));
        assert!(!service.observed_presence(&publication).unwrap());
        service.note_publication(&publication, true).unwrap();
        assert!(service.observed_presence(&publication).unwrap());
    }

    #[test]
    fn c_api_create_keeps_exact_context_and_creates_namespaces_before_binding() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let world = world();
            let context = world
                .borrow_mut()
                .create_child(ROOT_NS, &NameBytes::from("a:"))
                .unwrap();
            let service = service(&world, context, version);
            let simple = service
                .prepare(
                    b"x\0ignored::tail",
                    NativePublicationPurpose::CreateCCommand,
                )
                .unwrap();
            assert_eq!(simple.key().namespace, u64::from(ROOT_NS.0));
            assert_eq!(simple.key().simple.as_bytes(), b"x");
            let qualified = service
                .prepare(b"q::p", NativePublicationPurpose::CreateCCommand)
                .unwrap();
            let state = world.borrow();
            let child = state.ns_children[&(context, NameBytes::from("q"))];
            assert_eq!(qualified.key().namespace, u64::from(child.0));
            assert_eq!(state.ns_arena[child.0 as usize], ["a:", "q"]);
            assert_eq!(state.ns_child_order[&context].keys(), vec![b"q".as_slice()]);
            assert!(state.command_identity.by_slot.is_empty());
            assert!(!service.observed_presence(&qualified).unwrap());
        }
    }

    #[test]
    fn preparation_leaves_old_binding_visible_until_actual_mutation() {
        let world = world();
        let old = CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from("x"),
        };
        install(&world, old.clone(), 1);
        let service = service(&world, ROOT_NS, TclVersion::V9_1);
        let prepared = service
            .prepare(b"x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        assert!(service.observed_presence(&prepared).unwrap());
        assert_eq!(world.borrow().pending_slot_presence(&old), None);
        assert_eq!(
            service.inspect_commit(&prepared),
            Err(NativePublicationError::ReceiptNotNoted)
        );
        service.note_publication(&prepared, true).unwrap();
        assert_eq!(
            service.inspect_commit(&prepared).unwrap(),
            NativePublicationCommit::Apply {
                slot: old,
                present: true
            }
        );
    }

    #[test]
    fn outer_create_supersedes_nested_create_by_mutation_order() {
        let world = world();
        let service = service(&world, ROOT_NS, TclVersion::V9_1);
        let outer = service
            .prepare(b"x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        let nested = service
            .prepare(b"::x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        service.note_publication(&nested, true).unwrap();
        service.note_publication(&outer, true).unwrap();
        assert_eq!(
            service.inspect_commit(&nested).unwrap(),
            NativePublicationCommit::Superseded
        );
        service.finish_commit(&nested).unwrap();
        assert!(matches!(
            service.inspect_commit(&outer).unwrap(),
            NativePublicationCommit::Apply { present: true, .. }
        ));
        service.finish_commit(&outer).unwrap();
        assert!(world.borrow().pending.is_empty());
        assert_eq!(
            service.observed_presence(&outer),
            Err(NativePublicationError::ReceiptAlreadyConsumed)
        );
    }

    #[test]
    fn finishing_an_older_change_preserves_a_reentrant_newer_revision() {
        let world = world();
        let service = service(&world, ROOT_NS, TclVersion::V9_1);
        let first = service
            .prepare(b"x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        service.note_publication(&first, true).unwrap();
        assert!(matches!(
            service.inspect_commit(&first).unwrap(),
            NativePublicationCommit::Apply { .. }
        ));
        let newer = service
            .prepare(b"x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        service.note_publication(&newer, true).unwrap();
        service.finish_commit(&first).unwrap();
        assert!(matches!(
            service.inspect_commit(&newer).unwrap(),
            NativePublicationCommit::Apply { .. }
        ));
    }

    #[test]
    fn pending_create_delete_and_tombstone_lookup_share_the_actual_candidates() {
        let world = world();
        let context = world
            .borrow_mut()
            .create_child(ROOT_NS, &NameBytes::from("n"))
            .unwrap();
        install(
            &world,
            CommandSlot {
                namespace: ROOT_NS,
                simple: NameBytes::from("x"),
            },
            1,
        );
        let root = service(&world, ROOT_NS, TclVersion::V9_1);
        let local = service(&world, context, TclVersion::V9_1);
        let create = local
            .prepare(b"::n::x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        local.note_publication(&create, true).unwrap();
        let delete = local
            .prepare(b"x", NativePublicationPurpose::DeleteCCommand)
            .unwrap();
        assert_eq!(delete.key().namespace, u64::from(context.0));
        local.note_publication(&delete, false).unwrap();
        let fallback = local
            .prepare(b"x", NativePublicationPurpose::DeleteCCommand)
            .unwrap();
        assert_eq!(fallback.key().namespace, u64::from(ROOT_NS.0));
        assert!(root.observed_presence(&fallback).unwrap());
        assert_eq!(
            local.inspect_commit(&create).unwrap(),
            NativePublicationCommit::Superseded
        );
        assert!(matches!(
            local.inspect_commit(&delete).unwrap(),
            NativePublicationCommit::Apply { present: false, .. }
        ));
    }

    #[test]
    fn missing_delete_does_not_create_namespace_or_grant_a_fake_slot() {
        let world = world();
        let service = service(&world, ROOT_NS, TclVersion::V9_1);
        let missing = service
            .prepare(b"absent::x", NativePublicationPurpose::DeleteCCommand)
            .unwrap();
        assert_eq!(missing.key().namespace, u64::MAX);
        assert_eq!(missing.key().simple.as_bytes(), b"absent::x");
        assert!(!service.observed_presence(&missing).unwrap());
        assert_eq!(world.borrow().ns_arena.len(), 1);
        assert_eq!(
            service.note_publication(&missing, false),
            Err(NativePublicationError::WrongDisposition)
        );
    }

    #[test]
    fn world_identity_and_consumption_are_authenticated_independently_of_public_key() {
        let first_world = world();
        let second_world = world();
        let first = service(&first_world, ROOT_NS, TclVersion::V9_1);
        let second = service(&second_world, ROOT_NS, TclVersion::V9_1);
        let receipt = first
            .prepare(b"x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        assert_eq!(receipt.key().owner, second.owner);
        assert_eq!(receipt.key().interpreter, second.interpreter);
        assert_eq!(
            second.observed_presence(&receipt),
            Err(NativePublicationError::ForeignReceipt)
        );
        first.note_publication(&receipt, true).unwrap();
        assert_eq!(
            first.note_publication(&receipt, true),
            Err(NativePublicationError::ReceiptAlreadyNoted)
        );
        first.finish_commit(&receipt).unwrap();
        assert_eq!(
            first.finish_commit(&receipt),
            Err(NativePublicationError::ReceiptAlreadyConsumed)
        );
    }

    #[test]
    fn retired_interpreters_and_namespaces_do_not_retarget_to_recreations() {
        let world = world();
        let namespace = world
            .borrow_mut()
            .create_child(ROOT_NS, &NameBytes::from("n"))
            .unwrap();
        let service = service(&world, namespace, TclVersion::V9_1);
        let receipt = service
            .prepare(b"::n::x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        {
            let mut state = world.borrow_mut();
            state.dead_namespaces.insert(namespace);
            state.ns_children.remove(&(ROOT_NS, NameBytes::from("n")));
            state
                .ns_intern
                .remove(&ByteNamespacePath::from_segments(["n"]));
            let replacement = state.create_child(ROOT_NS, &NameBytes::from("n")).unwrap();
            assert_ne!(namespace, replacement);
        }
        assert_eq!(
            service.observed_presence(&receipt),
            Err(NativePublicationError::NamespaceExpired)
        );
        world.borrow_mut().retire();
        assert_eq!(
            service
                .prepare(b"x", NativePublicationPurpose::CreateCCommand)
                .unwrap_err(),
            NativePublicationError::ScopeExpired
        );
    }

    #[test]
    fn retained_context_creates_under_its_token_instead_of_same_named_live_namespace() {
        let world = world();
        let old = world
            .borrow_mut()
            .create_child(ROOT_NS, &NameBytes::from("n"))
            .unwrap();
        let replacement;
        {
            let mut state = world.borrow_mut();
            state.ns_children.remove(&(ROOT_NS, NameBytes::from("n")));
            state
                .ns_intern
                .remove(&ByteNamespacePath::from_segments(["n"]));
            let mut retained = super::super::RetainedNamespace::new(old);
            retained
                .subtree
                .insert(ByteNamespacePath::from_segments(["n"]), old);
            state.ns_deferral.retained.insert(old, retained);
            state.ns_deferral.owners.insert(old, old);
            replacement = state.create_child(ROOT_NS, &NameBytes::from("n")).unwrap();
        }
        let service = service(&world, old, TclVersion::V9_1);
        let receipt = service
            .prepare(b"q::x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        let selected = NsId(u32::try_from(receipt.key().namespace).unwrap());
        let state = world.borrow();
        assert_eq!(state.ns_parents[selected.0 as usize], Some(old));
        assert_eq!(state.ns_deferral.owners.get(&selected), Some(&old));
        assert!(
            !state
                .ns_children
                .contains_key(&(replacement, NameBytes::from("q")))
        );
        assert!(
            !state
                .ns_intern
                .contains_key(&ByteNamespacePath::from_segments(["n", "q"]))
        );
    }

    #[test]
    fn guest_deletion_effects_refuse_before_native_callbacks_and_are_generation_bound() {
        let world = world();
        let slot = CommandSlot {
            namespace: ROOT_NS,
            simple: NameBytes::from("x"),
        };
        install(&world, slot.clone(), 2);
        let service = service(&world, ROOT_NS, TclVersion::V9_1);
        world.borrow_mut().note_deletion_effects(
            slot.clone(),
            2,
            NativeDeletionEffects::GuestExecutionRequired,
        );
        assert_eq!(
            service
                .prepare(b"x", NativePublicationPurpose::CreateCCommand)
                .unwrap_err(),
            NativePublicationError::GuestDeletionExecutionRequired
        );
        assert_eq!(
            service
                .prepare(b"x", NativePublicationPurpose::DeleteCCommand)
                .unwrap_err(),
            NativePublicationError::GuestDeletionExecutionRequired
        );
        world.borrow_mut().note_deletion_effects(
            slot.clone(),
            1,
            NativeDeletionEffects::NoGuestExecution,
        );
        assert_eq!(
            service
                .prepare(b"x", NativePublicationPurpose::DeleteCCommand)
                .unwrap_err(),
            NativePublicationError::UnclassifiedDeletionEffects
        );
        world.borrow_mut().note_deletion_effects(
            slot.clone(),
            2,
            NativeDeletionEffects::NoGuestExecution,
        );
        world.borrow_mut().remove_deletion_effects(&slot, 1);
        assert!(
            service
                .prepare(b"x", NativePublicationPurpose::DeleteCCommand)
                .is_ok()
        );
    }

    #[test]
    fn service_and_receipts_do_not_keep_an_interpreter_alive() {
        let world = world();
        let service = service(&world, ROOT_NS, TclVersion::V9_1);
        let receipt = service
            .prepare(b"x", NativePublicationPurpose::CreateCCommand)
            .unwrap();
        assert_eq!(Rc::strong_count(&world), 1);
        drop(world);
        assert_eq!(
            service.observed_presence(&receipt),
            Err(NativePublicationError::ScopeExpired)
        );
    }
}
