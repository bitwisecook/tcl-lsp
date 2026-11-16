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

//! The advice graph: head lookup, publication keys, cell resolution and applied transitions.

use super::transition_capture::{
    capture_jim_alias_prefix, trace_jim_alias_guard, trace_jim_alias_option,
};
use super::{
    AdviceCell, AdviceGraph, AdviceNamingPolicy, AdvicePublicationPurpose, JimAliasSourceOperands,
    OriginalSourceCommandTransition, SourceAdviceNameInput, SourceAdviceWord,
    SourceAliasCellSelection, catalogue_seed,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_lexer::NativeWord;
#[cfg(test)]
use tcl_registry::model::ContextRegistry;
use tcl_registry::{
    AliasTargetLookup, CommandBindingTransition, InvocationDialect, StateTransition,
    TransitionSubject,
};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext, NativeNameProtocol};

pub(super) fn root_context(policy: NamePolicyProtocol) -> NativeNameContext<'static> {
    static ROOT: std::sync::LazyLock<ByteNamespacePath> =
        std::sync::LazyLock::new(ByteNamespacePath::root);
    match policy.recipe() {
        NativeNameProtocol::C(_) => NativeNameContext::root(),
        NativeNameProtocol::Jim084 => NativeNameContext::with_jim_namespace(&ROOT, b""),
    }
}

pub(super) fn lookup_key(policy: &AdviceNamingPolicy, bytes: &[u8]) -> Option<ByteCommandSlot> {
    let Some(policy) = policy.native() else {
        return AdviceNamingPolicy::logical_slot(bytes);
    };
    match policy.recipe() {
        NativeNameProtocol::C(_) => policy
            .recipe()
            .command_lookup_slot(root_context(policy), bytes)
            .ok(),
        NativeNameProtocol::Jim084 => Some(ByteCommandSlot::new(
            ByteNamespacePath::root(),
            NameBytes::from(
                policy
                    .recipe()
                    .command_lookup_input(root_context(policy), bytes)
                    .ok()?
                    .jim_flat_key()?,
            ),
        )),
    }
}

impl AdviceGraph {
    #[cfg(test)]
    pub(super) fn new(
        context: &ContextRegistry,
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
    ) -> Option<Self> {
        let seed = catalogue_seed::AdviceCatalogueSeed::build(context, policy)?;
        Some(Self::from_catalogue_seed(policy, dialect, &seed))
    }

    pub(super) fn from_catalogue_seed(
        policy: &AdviceNamingPolicy,
        dialect: InvocationDialect,
        seed: &catalogue_seed::AdviceCatalogueSeed,
    ) -> Self {
        Self {
            native_baseline: false,
            stock_source_effects: Vec::new(),
            cells: seed.cells(),
            namespaces: seed.namespaces(),
            namespace_exports: seed.namespace_exports(),
            namespace_paths: BTreeMap::new(),
            namespace_imports: HashMap::new(),
            unknown_exports: BTreeSet::new(),
            unknown_paths: BTreeSet::new(),
            policy: policy.clone(),
            logical_namespace: matches!(policy, AdviceNamingPolicy::Logical(_))
                .then(ByteNamespacePath::root),
            c_source_namespace: (dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                && matches!(
                    policy.native().map(NamePolicyProtocol::recipe),
                    Some(NativeNameProtocol::C(_))
                ))
            .then(ByteNamespacePath::root),
            dialect,
            uncertain_operations: Vec::new(),
            possible_occupants: BTreeSet::new(),
            arity_debts: Vec::new(),
            handles: Vec::new(),
            class_handles: Vec::new(),
            procedure_body: None,
            procedure_declarations: Vec::new(),
            class_declarations: Vec::new(),
            future_procedure_source: false,
            procedure_namespace: None,
            procedure_path: Vec::new(),
            procedure_steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            decoded_phase_steps: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    pub(super) fn native_source_context(&self) -> Option<NativeNameContext<'_>> {
        let policy = self.policy.native()?;
        self.procedure_namespace.as_ref().map_or_else(
            || Some(root_context(policy)),
            |namespace| Some(NativeNameContext::new(namespace)),
        )
    }
    pub(super) fn lookup_head_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        if self.c_source_namespace.is_some() && !self.native_baseline {
            return self.c_source_head_key(bytes);
        }
        if let Some(namespace) = &self.procedure_namespace {
            let native = self.policy.native()?;
            let local = native
                .recipe()
                .command_lookup_slot(NativeNameContext::new(namespace), bytes)
                .ok()?;
            return if self.cells.contains_key(&local) {
                Some(local)
            } else {
                lookup_key(&self.policy, bytes)
            };
        }
        let Some(namespace) = &self.logical_namespace else {
            return lookup_key(&self.policy, bytes);
        };
        self.lookup_logical_head_key(namespace, bytes)
    }

    pub(super) fn lookup_logical_head_key(
        &self,
        namespace: &ByteNamespacePath,
        bytes: &[u8],
    ) -> Option<ByteCommandSlot> {
        let local = tcl_syntax::naming::authored_source_command_slot(namespace, bytes)?;
        if !tcl_syntax::naming::is_absolute_bytes(bytes)
            && self.unknown_paths.contains(namespace)
            && !self
                .cells
                .get(&local)
                .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            return None;
        }
        let candidates = self.logical_head_candidates(namespace, bytes)?;
        candidates
            .iter()
            .find(|key| {
                self.cells
                    .get(*key)
                    .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
            })
            .cloned()
            .or_else(|| candidates.last().cloned())
    }

    fn logical_head_candidates(
        &self,
        namespace: &ByteNamespacePath,
        bytes: &[u8],
    ) -> Option<Vec<ByteCommandSlot>> {
        tcl_syntax::naming::authored_source_command_candidates(
            namespace,
            self.namespace_paths
                .get(namespace)
                .map_or(&[], Vec::as_slice),
            bytes,
        )
    }

    /// Every coordinate [`Self::lookup_head_key`] may try, in its order.
    pub(super) fn head_lookup_candidates(&self, bytes: &[u8]) -> Option<Vec<ByteCommandSlot>> {
        if self.c_source_namespace.is_some() && !self.native_baseline {
            return self.c_source_head_candidates(bytes);
        }
        if let Some(namespace) = &self.procedure_namespace {
            let local = self
                .policy
                .native()?
                .recipe()
                .command_lookup_slot(NativeNameContext::new(namespace), bytes)
                .ok()?;
            return Some(vec![local, lookup_key(&self.policy, bytes)?]);
        }
        let Some(namespace) = &self.logical_namespace else {
            return Some(vec![lookup_key(&self.policy, bytes)?]);
        };
        self.logical_head_candidates(namespace, bytes)
    }

    /// Whether `selected` was reached past a coordinate a possible branch
    /// may have occupied, so the actual lookup may stop earlier.
    fn passes_possible_occupant(
        &self,
        candidates: Option<Vec<ByteCommandSlot>>,
        selected: &ByteCommandSlot,
    ) -> bool {
        candidates.is_some_and(|candidates| {
            candidates
                .iter()
                .take_while(|key| *key != selected)
                .any(|key| self.possible_occupants.contains(key))
        })
    }

    /// The coordinate a command lookup selects, refused when it falls through
    /// a possible occupant. Raw [`Self::lookup_head_key`] remains only for
    /// coordinate and header enumeration and for publication.
    pub(super) fn selected_head_key(&self, bytes: &[u8]) -> Option<ByteCommandSlot> {
        let selected = self.lookup_head_key(bytes)?;
        if self.possible_occupants.is_empty() {
            return Some(selected);
        }
        (!self.passes_possible_occupant(self.head_lookup_candidates(bytes), &selected))
            .then_some(selected)
    }

    /// [`Self::selected_head_key`] for an explicitly selected Logical namespace.
    pub(super) fn selected_logical_head_key(
        &self,
        namespace: &ByteNamespacePath,
        bytes: &[u8],
    ) -> Option<ByteCommandSlot> {
        let selected = self.lookup_logical_head_key(namespace, bytes)?;
        if self.possible_occupants.is_empty() {
            return Some(selected);
        }
        (!self.passes_possible_occupant(self.logical_head_candidates(namespace, bytes), &selected))
            .then_some(selected)
    }

    pub(super) fn publication_key(
        &self,
        bytes: &[u8],
        purpose: AdvicePublicationPurpose,
    ) -> Option<ByteCommandSlot> {
        if self.c_source_namespace.is_some()
            && !self.native_baseline
            && self.procedure_namespace.is_none()
        {
            return self.c_source_publication_key(bytes, purpose);
        }
        if let Some(namespace) = &self.procedure_namespace {
            let recipe = self.policy.native()?.recipe();
            let context = NativeNameContext::new(namespace);
            return match purpose {
                AdvicePublicationPurpose::Define => recipe.command_publication_slot(context, bytes),
                AdvicePublicationPurpose::Move => recipe.rename_destination_slot(context, bytes),
                AdvicePublicationPurpose::Alias => recipe.alias_publication_slot(context, bytes),
            }
            .ok();
        }
        self.logical_namespace.as_ref().map_or_else(
            || self.policy.publication_slot(bytes, purpose),
            |namespace| tcl_syntax::naming::authored_source_command_slot(namespace, bytes),
        )
    }

    pub(super) fn alias_target_key(
        &self,
        target: &SourceAdviceNameInput,
        lookup: AliasTargetLookup,
    ) -> Option<ByteCommandSlot> {
        match lookup {
            AliasTargetLookup::Global => lookup_key(&self.policy, target.bytes()),
            AliasTargetLookup::CallerNamespace => self.selected_head_key(target.bytes()),
        }
    }

    pub(super) fn declared_source_selection(
        &self,
        origin: &Arc<super::super::SourceOriginId>,
        head: &SourceAdviceNameInput,
        words: &[NativeWord],
    ) -> Option<super::super::DeclaredSourceSelection> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let key = self.selected_head_key(head.bytes())?;
        let AdviceCell::Declared(declaration) = self.cells.get(&key)? else {
            return None;
        };
        super::super::DeclaredSourceSelection::authored(
            super::super::CommandAllocationSite {
                source: Arc::clone(origin),
                offset: words.first()?.span().start(),
            },
            words,
            head.clone(),
            Arc::clone(declaration),
            !self.uncertain_operations.is_empty(),
        )
    }
    pub(super) fn record_uncertainty(&mut self, original: &[NativeWord]) {
        if !self
            .uncertain_operations
            .iter()
            .any(|operation| operation.as_ref() == original)
        {
            self.uncertain_operations.push(Arc::from(original));
        }
    }
    pub(super) fn widen(&mut self, original: &[NativeWord]) {
        self.uncertain_operations.push(Arc::from(original));
        self.handles.clear();
        self.withdraw_namespace_advice();
        let imports = &self.namespace_imports;
        let keep = |slot: &ByteCommandSlot, cell: &AdviceCell| match cell {
            AdviceCell::Descriptor(_, lineage) => lineage.is_empty() || imports.contains_key(slot),
            AdviceCell::Shadowed
            | AdviceCell::Deleted
            | AdviceCell::Declared(_)
            | AdviceCell::SourceProcedure(_)
            | AdviceCell::LogicalSourceProcedure { .. }
            | AdviceCell::SourceClass(_)
            | AdviceCell::AuthoredJimClass(_)
            | AdviceCell::ClassInstance(_)
            | AdviceCell::RegisteredInstance(_)
            | AdviceCell::Alias { .. } => true,
            AdviceCell::CreatedInterpreter(_) => false,
        };
        self.cells.update_if_needed(
            |cells| cells.iter().any(|(slot, cell)| !keep(slot, cell)),
            |cells| cells.retain(|slot, cell| keep(slot, cell)),
        );
    }
    pub(super) fn blocks_registry_source(&self, input: &SourceAdviceNameInput) -> bool {
        let Some(start) = self.lookup_head_key(input.bytes()) else {
            return false;
        };
        if !self.possible_occupants.is_empty()
            && self.passes_possible_occupant(self.head_lookup_candidates(input.bytes()), &start)
        {
            return true;
        }
        let mut blocked = false;
        let cycle = tcl_syntax::naming::alias_chain_loops(start, |key| match self.cells.get(key) {
            Some(AdviceCell::Alias { target, lookup, .. }) => {
                let key = self.alias_target_key(target, *lookup);
                // A caller-namespace target refused past a possible occupant
                // keeps the barrier the raw target would have recorded.
                if key.is_none() && matches!(lookup, AliasTargetLookup::CallerNamespace) {
                    blocked = true;
                }
                Ok::<_, ()>(key)
            }
            Some(
                AdviceCell::Shadowed
                | AdviceCell::Deleted
                | AdviceCell::CreatedInterpreter(_)
                | AdviceCell::RegisteredInstance(_)
                | AdviceCell::SourceClass(_)
                | AdviceCell::AuthoredJimClass(_)
                | AdviceCell::ClassInstance(_)
                | AdviceCell::SourceProcedure(_)
                | AdviceCell::LogicalSourceProcedure { .. }
                | AdviceCell::Declared(_),
            ) => {
                blocked = true;
                Ok(None)
            }
            Some(AdviceCell::Descriptor(_, _)) | None => Ok(None),
        });
        blocked || cycle == Ok(true)
    }

    /// An original Logical scope can retain a possible named catalogue after
    /// unclosed effects. Its uncertainties remain attached to the source advice.
    pub(super) fn readonly_logical_source_candidate(
        &self,
        head: &SourceAdviceNameInput,
        original: &[NativeWord],
        scope: &Arc<super::super::source_declared_command::OriginalDeclaredLogicalBodyContext>,
    ) -> Option<(
        String,
        Vec<SourceAdviceWord>,
        Vec<Arc<OriginalSourceCommandTransition>>,
    )> {
        let AdviceNamingPolicy::Logical(input) = &self.policy else {
            return None;
        };
        if self.uncertain_operations.is_empty()
            || !scope.matches_input(input)
            || !scope.owns_words(original)
            || self.logical_namespace.as_ref() != Some(scope.namespace_recipe())
            || original.first()?.group().expand
            || head.logical_input().is_none()
            || tcl_syntax::naming::is_qualified(head.bytes())
            || self.blocks_registry_source(head)
        {
            return None;
        }
        let local = tcl_syntax::naming::authored_source_command_slot(
            scope.namespace_recipe(),
            head.bytes(),
        )?;
        let key = tcl_syntax::naming::authored_source_command_slot(
            &ByteNamespacePath::root(),
            head.bytes(),
        )?;
        let AdviceCell::Descriptor(command, lineage) = self.cells.get(&key)? else {
            return None;
        };
        if !lineage.is_empty() {
            return None;
        }
        match self.cells.get(&local) {
            None | Some(AdviceCell::Deleted) => {}
            Some(AdviceCell::Descriptor(local_command, local_lineage))
                if local_command == command && local_lineage.is_empty() => {}
            _ => return None,
        }
        Some((command.clone(), Vec::new(), Vec::new()))
    }

    /// Shared original alias/captured-prefix traversal. Terminal cell kind
    /// remains a separate purpose; this issues no Registry or Native target.
    pub(super) fn resolve_source_cell(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<SourceAliasCellSelection<'_>> {
        let start = self.selected_head_key(input.bytes())?;
        self.resolve_source_cell_at_slot(start)
    }

    pub(super) fn resolve_source_cell_at_slot(
        &self,
        start: ByteCommandSlot,
    ) -> Option<SourceAliasCellSelection<'_>> {
        if tcl_syntax::naming::alias_chain_loops(start.clone(), |key| match self.cells.get(key) {
            Some(AdviceCell::Alias { target, lookup, .. }) => {
                Ok::<_, ()>(Some(self.alias_target_key(target, *lookup).ok_or(())?))
            }
            _ => Ok(None),
        })
        .ok()?
        {
            return None;
        }
        let mut key = start;
        let mut prefix = Vec::new();
        let mut lineage = Vec::new();
        loop {
            let cell = self.cells.get(&key)?;
            match cell {
                AdviceCell::Alias {
                    target,
                    prefix: captured,
                    lookup,
                    lineage: steps,
                } => {
                    let mut next = captured.clone();
                    next.extend(prefix);
                    prefix = next;
                    lineage.extend(steps.iter().cloned());
                    key = self.alias_target_key(target, *lookup)?;
                }
                _ => {
                    return Some(SourceAliasCellSelection {
                        cell,
                        slot: key,
                        prefix,
                        lineage,
                    });
                }
            }
        }
    }
    pub(super) fn resolve(
        &self,
        input: &SourceAdviceNameInput,
    ) -> Option<(
        String,
        Vec<SourceAdviceWord>,
        Vec<Arc<OriginalSourceCommandTransition>>,
    )> {
        let mut selected = self.resolve_source_cell(input)?;
        let AdviceCell::Descriptor(command, steps) = selected.cell else {
            return None;
        };
        selected.lineage.extend(steps.iter().cloned());
        Some((command.clone(), selected.prefix, selected.lineage))
    }
    pub(super) fn apply(&mut self, receipt: &Arc<OriginalSourceCommandTransition>) -> Option<()> {
        for fact in receipt.transitions.facts() {
            match &fact.transition {
                StateTransition::CommandBinding(transition) => {
                    self.apply_command(receipt, transition)?;
                }
                StateTransition::Interpreter(transition) => {
                    self.apply_interpreter_transition(receipt, transition)?;
                }
                StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                }) => {
                    receipt.input(subject)?;
                    if !matches!(
                        self.policy.native().map(NamePolicyProtocol::recipe),
                        Some(NativeNameProtocol::Jim084)
                    ) {
                        let mut path = if self.c_source_namespace.is_some() && !self.native_baseline
                        {
                            self.c_source_namespace_path(receipt.input(subject)?)?
                        } else {
                            self.logical_namespace.as_ref().map_or_else(
                                || self.policy.namespace_path(receipt.selected_input(subject)?),
                                |namespace| {
                                    tcl_syntax::naming::authored_source_namespace_path(
                                        namespace,
                                        receipt.selected_input(subject)?,
                                    )
                                },
                            )?
                        };
                        loop {
                            self.namespaces.insert(path.clone());
                            let Some(parent) = path.parent() else {
                                break;
                            };
                            path = parent;
                        }
                    }
                }
                StateTransition::Namespace(transition) => {
                    self.apply_source_namespace(receipt, transition)?;
                }
                _ => {}
            }
        }
        Some(())
    }
    fn apply_move(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        from: &TransitionSubject,
        to: &TransitionSubject,
    ) -> Option<()> {
        let from = self.policy.rename_source(receipt.input(from)?.bytes())?;
        let from_key = self.selected_head_key(&from)?;
        let cell = self.cells.get(&from_key)?.clone();
        let destination = receipt.input(to)?;
        let selected = self.policy.rename_destination(destination.bytes())?;
        if matches!(cell, AdviceCell::Shadowed | AdviceCell::Deleted) {
            return None;
        }
        if selected.is_empty() {
            self.remove_namespace_import_origin(&from_key);
            self.namespace_imports.remove(&from_key);
            self.cells.insert(from_key, AdviceCell::Deleted);
            return Some(());
        }
        let to_key = self.publication_key(destination.bytes(), AdvicePublicationPurpose::Move)?;
        if !self.namespaces.contains(&to_key.namespace) {
            return None;
        }
        if self
            .cells
            .get(&to_key)
            .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
        {
            return None;
        }
        self.move_namespace_import_origin(&from_key, &to_key, receipt);
        if let Some(origin) = self.namespace_imports.remove(&from_key) {
            self.namespace_imports.insert(to_key.clone(), origin);
        }
        self.cells.insert(from_key, AdviceCell::Deleted);
        let moved = match cell {
            AdviceCell::Descriptor(command, mut lineage) => {
                lineage.push(Arc::clone(receipt));
                AdviceCell::Descriptor(command, lineage)
            }
            AdviceCell::Alias {
                target,
                prefix,
                lookup,
                mut lineage,
            } => {
                lineage.push(Arc::clone(receipt));
                AdviceCell::Alias {
                    target,
                    prefix,
                    lookup,
                    lineage,
                }
            }
            AdviceCell::CreatedInterpreter(cell) => {
                AdviceCell::CreatedInterpreter(cell.moved(to_key.clone(), receipt))
            }
            AdviceCell::RegisteredInstance(instance) => {
                AdviceCell::RegisteredInstance(Arc::new(instance.moved(to_key.clone(), receipt)))
            }
            AdviceCell::ClassInstance(instance) => {
                AdviceCell::ClassInstance(Arc::new(instance.moved(to_key.clone(), receipt)))
            }
            AdviceCell::SourceClass(class) => {
                AdviceCell::SourceClass(Arc::new(class.moved(to_key.clone(), receipt)))
            }
            AdviceCell::AuthoredJimClass(class) => {
                AdviceCell::AuthoredJimClass(Arc::new(class.moved(to_key.clone(), receipt)))
            }
            AdviceCell::SourceProcedure(procedure) => {
                AdviceCell::SourceProcedure(Arc::new(procedure.moved(to_key.clone(), receipt)))
            }
            AdviceCell::LogicalSourceProcedure {
                definition,
                mut lineage,
            } => {
                lineage.push(Arc::clone(receipt));
                AdviceCell::LogicalSourceProcedure {
                    definition,
                    lineage,
                }
            }
            AdviceCell::Declared(_) => AdviceCell::Shadowed,
            AdviceCell::Shadowed | AdviceCell::Deleted => return None,
        };
        self.cells.insert(to_key, moved);
        Some(())
    }

    fn apply_command(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        transition: &CommandBindingTransition,
    ) -> Option<()> {
        let input = |subject| receipt.input(subject);
        match transition {
            CommandBindingTransition::Define { name, kind } => {
                let key =
                    self.publication_key(input(name)?.bytes(), AdvicePublicationPurpose::Define)?;
                self.remove_namespace_import_origin(&key);
                self.namespace_imports.remove(&key);
                let cell = if matches!(self.policy, AdviceNamingPolicy::Logical(_))
                    && *kind == tcl_registry::CommandBindingDefinitionKind::Procedure
                {
                    AdviceCell::LogicalSourceProcedure {
                        definition: Arc::clone(receipt),
                        lineage: Vec::new(),
                    }
                } else {
                    AdviceCell::Shadowed
                };
                self.cells.insert(key, cell);
            }
            CommandBindingTransition::Move { from, to } => {
                self.apply_move(receipt, from, to)?;
            }
            CommandBindingTransition::Delete { interpreter, name } => {
                if interpreter
                    .as_ref()
                    .is_some_and(|subject| receipt.is_current_interpreter(subject) != Some(true))
                {
                    return None;
                }
                let key = self.selected_head_key(input(name)?.bytes())?;
                self.remove_namespace_import_origin(&key);
                self.namespace_imports.remove(&key);
                self.cells.insert(key, AdviceCell::Deleted);
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup,
            } => {
                return self.apply_original_alias(
                    receipt,
                    JimAliasSourceOperands {
                        source_interpreter,
                        alias,
                        target_interpreter,
                        target,
                        arguments,
                        target_lookup: *target_lookup,
                    },
                );
            }
            CommandBindingTransition::Unknown { .. } => return None,
        }
        Some(())
    }
    fn apply_original_alias(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        operands: JimAliasSourceOperands<'_>,
    ) -> Option<()> {
        let JimAliasSourceOperands {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } = operands;
        let input = |subject| receipt.input(subject);

        let current_interpreter = trace_jim_alias_option(
            &receipt.descriptor,
            "jim.alias.source_interpreter",
            receipt.is_current_interpreter(source_interpreter),
        )?;
        if !trace_jim_alias_guard(
            &receipt.descriptor,
            "jim.alias.source_current",
            current_interpreter,
        ) {
            return None;
        }
        let alias =
            trace_jim_alias_option(&receipt.descriptor, "jim.alias.alias_input", input(alias))?;
        let key = trace_jim_alias_option(
            &receipt.descriptor,
            "jim.alias.publication",
            self.publication_key(alias.bytes(), AdvicePublicationPurpose::Alias),
        )?;
        self.remove_namespace_import_origin(&key);
        self.namespace_imports.remove(&key);
        if !trace_jim_alias_guard(
            &receipt.descriptor,
            "jim.alias.namespace",
            self.namespaces.contains(&key.namespace),
        ) || !trace_jim_alias_guard(
            &receipt.descriptor,
            "jim.alias.target_current",
            trace_jim_alias_option(
                &receipt.descriptor,
                "jim.alias.target_interpreter",
                receipt.is_current_interpreter(target_interpreter),
            )?,
        ) {
            // A current alias with foreign target semantics withdraws
            // any earlier current procedure/class meaning at its name.
            self.cells.insert(key, AdviceCell::Shadowed);
            return Some(());
        }
        let prefix = capture_jim_alias_prefix(receipt, arguments)?;
        self.cells.insert(
            key,
            AdviceCell::Alias {
                target: Box::new(
                    trace_jim_alias_option(
                        &receipt.descriptor,
                        "jim.alias.target_input",
                        input(target),
                    )?
                    .clone(),
                ),
                prefix,
                lookup: target_lookup,
                lineage: vec![Arc::clone(receipt)],
            },
        );
        Some(())
    }
}
