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

//! Capturing a command transfer before its handler and finishing it into publications.

use super::super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandDefinition,
    SourceExecutionContext, SourceNamespaceKey, SourceNativeInvocation,
};
use super::{
    NamespaceGeometry, OriginalAliasTarget, OriginalCommandPublication,
    OriginalCommandPublicationKind, OriginalSourceCommandWorld, SourceWorldAliasOperands,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::ByteCommandSlot;
use tcl_registry::state_transition::EnsembleManagementOperation;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, EnsembleCommandName, InvocationFacts,
    NamespaceTransition, NamespaceTransitionTarget, StateTransition, TransitionSubject,
};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol, NativeNamePurpose};

pub(super) enum Operation {
    Publish(Box<OriginalCommandPublication>),
    Move(ByteCommandSlot, ByteCommandSlot, SignatureSourceNameInput),
    Delete(ByteCommandSlot),
    Ensure(
        NamespaceTransitionTarget,
        SignatureNamespaceScope,
        Option<SignatureSourceNameInput>,
    ),
    DeleteNamespace(SignatureNamespaceScope),
    Export(SignatureNamespaceScope, bool, Vec<SignatureSourceNameInput>),
    Import(SignatureNamespaceScope, bool, Vec<SignatureSourceNameInput>),
    Forget(SignatureNamespaceScope, Vec<SignatureSourceNameInput>),
    Withdraw,
}

pub(in super::super) struct CapturedOriginalCommandTransfer {
    pub(super) world: OriginalSourceCommandWorld,
    operations: Vec<Operation>,
    current: SourceNamespaceKey,
    pub(super) site: CommandAllocationSite,
}

struct SourceWorldCapture<'a> {
    native: SourceNativeInvocation<'a>,
    pub(super) state: &'a ModuleCommandBindings,
    context: SourceExecutionContext<'a>,
    current: &'a SourceNamespaceKey,
    selected_scope: Option<SignatureNamespaceScope>,
    pub(super) policy: tcl_syntax::naming::NamePolicyProtocol,
    pub(super) site: &'a CommandAllocationSite,
}

impl SourceWorldCapture<'_> {
    pub(super) fn original(&self, subject: &TransitionSubject) -> Option<SignatureSourceNameInput> {
        super::super::original_command_table::original_operand(
            self.native,
            subject,
            self.state,
            self.context,
        )
    }
    fn scope_for(&self, input: &SignatureSourceNameInput) -> Option<SignatureNamespaceScope> {
        if tcl_syntax::naming::is_absolute_bytes(input.bytes()) {
            Some(SignatureNamespaceScope::root(Some(self.policy)))
        } else {
            self.selected_scope.clone()
        }
    }
    fn selected_namespace(
        &self,
        target: &NamespaceTransitionTarget,
    ) -> Option<SignatureNamespaceScope> {
        match target {
            NamespaceTransitionTarget::Current => self.selected_scope.clone(),
            NamespaceTransitionTarget::Named(subject) => {
                let input = self.original(subject)?;
                self.scope_for(&input)?.child_from_input(&input)
            }
        }
    }
    fn alias_operation(&self, operands: SourceWorldAliasOperands<'_>) -> Option<Operation> {
        let SourceWorldAliasOperands {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } = operands;
        if !self.original(source_interpreter)?.bytes().is_empty() {
            return None;
        }
        let input = self.original(alias)?;
        let root = SignatureNamespaceScope::root(Some(self.policy));
        let slot = self
            .policy
            .recipe()
            .alias_publication_slot(root.context()?, input.bytes())
            .ok()?;
        let alias_target = self
            .original(target_interpreter)
            .filter(|interpreter| interpreter.bytes().is_empty())
            .and_then(|_| {
                Some(OriginalAliasTarget {
                    input: self.original(target)?,
                    lookup: target_lookup,
                    arguments: arguments
                        .iter()
                        .map(|subject| self.original(subject))
                        .collect::<Option<Vec<_>>>()?,
                })
            });
        Some(Operation::Publish(Box::new(OriginalCommandPublication {
            slot,
            input,
            site: self.site.clone(),
            kind: OriginalCommandPublicationKind::Alias,
            definition: None,
            origin: None,
            implementation_allocation: None,
            alias_target,
        })))
    }
    fn command_operation(&self, transition: &CommandBindingTransition) -> Option<Operation> {
        match transition {
            CommandBindingTransition::Define { name, kind } => {
                let input = self.original(name)?;
                let scope = self.scope_for(&input)?;
                let slot = publication_slot(&scope, &input, *kind)?;
                Some(Operation::Publish(Box::new(OriginalCommandPublication {
                    slot,
                    input,
                    site: self.site.clone(),
                    kind: match kind {
                        CommandBindingDefinitionKind::Procedure => {
                            OriginalCommandPublicationKind::Procedure
                        }
                        CommandBindingDefinitionKind::Object
                        | CommandBindingDefinitionKind::Instance => {
                            OriginalCommandPublicationKind::Object
                        }
                        CommandBindingDefinitionKind::Command => {
                            OriginalCommandPublicationKind::Command
                        }
                    },
                    definition: None,
                    origin: None,
                    implementation_allocation: None,
                    alias_target: None,
                })))
            }
            CommandBindingTransition::Move { from, to } => {
                let from = self.original(from)?;
                let to = self.original(to)?;
                let (key, _) = self
                    .state
                    .original_occupied_command_for_input(self.current, &from)?;
                let source = self.state.original_byte_slot_for_key(&key, self.policy)?;
                let scope = self.scope_for(&to)?;
                let selected = self
                    .policy
                    .recipe()
                    .rename_destination_input(scope.context()?, to.bytes())
                    .ok()?;
                if selected.selected().is_empty() {
                    return Some(Operation::Delete(source));
                }
                let destination = self
                    .policy
                    .recipe()
                    .rename_destination_slot(self.scope_for(&to)?.context()?, to.bytes())
                    .ok()?;
                Some(Operation::Move(source, destination, to))
            }
            CommandBindingTransition::Delete { interpreter, name } => {
                if let Some(interpreter) = interpreter
                    && !self.original(interpreter)?.bytes().is_empty()
                {
                    return None;
                }
                let input = self.original(name)?;
                let namespace = if interpreter.is_some() {
                    self.state.source_root_namespace_key()?
                } else {
                    self.current.clone()
                };
                let (key, _) = self
                    .state
                    .original_occupied_command_for_input(&namespace, &input)?;
                Some(Operation::Delete(
                    self.state.original_byte_slot_for_key(&key, self.policy)?,
                ))
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup,
            } => self.alias_operation(SourceWorldAliasOperands {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup: *target_lookup,
            }),
            CommandBindingTransition::Unknown { .. } => None,
        }
    }
    /// A literal `namespace ensemble create` publishes its one command slot;
    /// a `-command` word is a source declaration like any other command name,
    /// while the current namespace's own name has no source word to publish,
    /// so only its previous occupant is retired.
    fn ensemble_operation(&self, created: &EnsembleCommandName) -> Option<Operation> {
        match created {
            EnsembleCommandName::Named(name) => {
                self.command_operation(&CommandBindingTransition::Define {
                    name: name.clone(),
                    kind: CommandBindingDefinitionKind::Command,
                })
            }
            EnsembleCommandName::CurrentNamespace => self
                .selected_scope
                .as_ref()
                .and_then(namespace_own_command_slot)
                .map(Operation::Delete),
        }
    }
    fn namespace_operation(&self, transition: &NamespaceTransition) -> Option<Operation> {
        match transition {
            NamespaceTransition::Ensure { namespace } => {
                self.selected_namespace(namespace).map(|scope| {
                    let input = match namespace {
                        NamespaceTransitionTarget::Current => None,
                        NamespaceTransitionTarget::Named(subject) => self.original(subject),
                    };
                    Operation::Ensure(namespace.clone(), scope, input)
                })
            }
            NamespaceTransition::Delete { namespace } => self
                .selected_namespace(namespace)
                .map(Operation::DeleteNamespace),
            NamespaceTransition::Export {
                namespace,
                patterns,
            } => {
                let mut inputs = patterns
                    .iter()
                    .map(|subject| self.original(subject))
                    .collect::<Option<Vec<_>>>();
                inputs
                    .as_mut()
                    .and_then(|inputs| {
                        let bytes = inputs
                            .iter()
                            .map(SignatureSourceNameInput::bytes)
                            .collect::<Vec<_>>();
                        let (clear, _) =
                            NamespaceTransition::export_pattern_byte_operands(&bytes, self.policy)?;
                        if clear {
                            inputs.remove(0);
                        }
                        Some((clear, inputs.clone()))
                    })
                    .zip(self.selected_namespace(namespace))
                    .map(|((clear, inputs), scope)| Operation::Export(scope, clear, inputs))
            }
            NamespaceTransition::Import {
                namespace,
                force,
                patterns,
            } => force
                .zip(self.selected_namespace(namespace))
                .zip(
                    patterns
                        .iter()
                        .map(|subject| self.original(subject))
                        .collect::<Option<Vec<_>>>(),
                )
                .map(|((force, scope), inputs)| Operation::Import(scope, force, inputs)),
            NamespaceTransition::Forget {
                namespace,
                patterns,
            } => self
                .selected_namespace(namespace)
                .zip(
                    patterns
                        .iter()
                        .map(|subject| self.original(subject))
                        .collect::<Option<Vec<_>>>(),
                )
                .map(|(scope, inputs)| Operation::Forget(scope, inputs)),
            NamespaceTransition::SetPath { .. }
            | NamespaceTransition::SetUnknown { .. }
            | NamespaceTransition::Ensemble { .. } => None,
        }
    }
}

pub(in super::super) fn capture(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<CapturedOriginalCommandTransfer> {
    let current = context.namespace_identity();
    let site = CommandAllocationSite {
        source: state.current_source_origin.clone()?,
        offset: native.segment.span.start(),
    };
    let mut world = (*state.original_command_world).clone();
    let mut operations = Vec::new();
    let Some(policy) = world.select_policy(state) else {
        return Some(CapturedOriginalCommandTransfer {
            world,
            operations: vec![Operation::Withdraw],
            current,
            site,
        });
    };
    if state.current_source_origin.is_none()
        || state.source_step_observed()
        || state.source_execution_observed(None)
    {
        operations.push(Operation::Withdraw);
    }
    if facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::NamespaceEval,
        )
        && state.baseline.native_entry.is_none()
        && !super::super::original_namespace_body::has_original_namespace_body_delegation(
            native, facts, state, context,
        )
    {
        operations.push(Operation::Withdraw);
    }
    let selected_scope = world.scope(&current, policy);
    let capture = SourceWorldCapture {
        native,
        state,
        context,
        current: &current,
        selected_scope,
        policy,
        site: &site,
    };
    if let Some(transitions) = facts.state_transitions.declared() {
        for fact in transitions.facts() {
            let operation = match &fact.transition {
                StateTransition::CommandBinding(transition) => {
                    capture.command_operation(transition)
                }
                StateTransition::Namespace(
                    NamespaceTransition::SetPath { .. }
                    | NamespaceTransition::SetUnknown { .. }
                    | NamespaceTransition::Ensemble {
                        operation: Some(EnsembleManagementOperation::Configure),
                        ..
                    },
                ) => continue,
                StateTransition::Namespace(NamespaceTransition::Ensemble {
                    created: Some(created),
                    ..
                }) => capture.ensemble_operation(created),
                StateTransition::Namespace(transition) => capture.namespace_operation(transition),
                StateTransition::Widen(widening)
                    if widening.domains.iter().any(|domain| {
                        matches!(
                            domain,
                            tcl_registry::StateTransitionDomain::CommandBindings
                                | tcl_registry::StateTransitionDomain::Namespaces
                        )
                    }) =>
                {
                    None
                }
                _ => continue,
            };
            operations.push(operation.unwrap_or(Operation::Withdraw));
        }
    }
    // A handler that only runs one braced literal script writes what that
    // script writes; the body walk records those commands one by one.
    if facts.effects.accesses().iter().any(|access| {
        matches!(
            access.domain,
            tcl_registry::world_effect::WorldStateDomain::CommandBindings
                | tcl_registry::world_effect::WorldStateDomain::NamespaceLookup
        ) && !matches!(
            access.mode,
            tcl_registry::world_effect::EffectAccessMode::Read
        )
    }) && !super::super::original_delegated_script::has_delegated_literal_script(
        native, facts, state, context,
    ) {
        operations.push(Operation::Withdraw);
    }
    Some(CapturedOriginalCommandTransfer {
        world,
        operations,
        current,
        site,
    })
}

/// The slot `namespace ensemble create` without `-command` publishes: the
/// namespace's own qualified name in its parent, and `::` for the root.
pub(in super::super) fn namespace_own_command_slot(
    scope: &SignatureNamespaceScope,
) -> Option<ByteCommandSlot> {
    let SignatureNamespaceScope::C(path) = scope else {
        return None;
    };
    let mut namespace = path.clone();
    let simple = namespace.pop().unwrap_or_default();
    Some(ByteCommandSlot { namespace, simple })
}

pub(in super::super) fn publication_slot(
    scope: &SignatureNamespaceScope,
    input: &SignatureSourceNameInput,
    kind: CommandBindingDefinitionKind,
) -> Option<ByteCommandSlot> {
    match input {
        SignatureSourceNameInput::OriginalWord(_) | SignatureSourceNameInput::OriginalValue(_) => {}
        SignatureSourceNameInput::OriginalVariableRoot(_)
        | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
    }
    let recipe = input.policy().recipe();
    let context = scope.context()?;
    let slot = if matches!(
        kind,
        CommandBindingDefinitionKind::Object | CommandBindingDefinitionKind::Instance
    ) {
        recipe
            .oo_object_publication_slot(context, input.bytes())
            .ok()?
    } else {
        recipe
            .command_publication_slot(context, input.bytes())
            .ok()?
    };
    if kind == CommandBindingDefinitionKind::Procedure
        && let NativeNameProtocol::C(version) = recipe
    {
        tcl_registry::native_procedure::procedure_name_creation_error(
            tcl_native_model::InvocationDialect::for_version(version),
            slot.namespace.is_root(),
            slot.simple.as_bytes(),
        )?
        .ok()?;
    }
    Some(slot)
}

impl CapturedOriginalCommandTransfer {
    fn ensure_namespace(
        &mut self,
        state: &ModuleCommandBindings,
        target: &NamespaceTransitionTarget,
        scope: SignatureNamespaceScope,
        input: Option<&SignatureSourceNameInput>,
    ) {
        let key = match (target, input) {
            (NamespaceTransitionTarget::Named(subject), Some(input))
                if matches!(input.policy().recipe(), NativeNameProtocol::C(_))
                    && (subject.literal().is_none()
                        || !matches!(self.current, SourceNamespaceKey::Authored(_))) =>
            {
                state.original_namespace_key_for_input(&self.current, input)
            }
            _ => state.namespace_target_key_at(target, &self.current),
        };
        let Some(key) = key else {
            self.world.withdraw();
            return;
        };
        // Every intermediate allocation belongs to this same
        // reached Ensure, with its own counted path and lifetime.
        for parent in state.namespaces.iter().filter(|namespace| {
                        matches!(namespace, SourceNamespaceKey::Allocated { site, .. } if site == &self.site)
                    }) {
                        let Some(path) = parent.exact_native_path() else {
                            continue;
                        };
                        if !self.world.namespaces.has_key(parent) {
                            self.world.namespaces.push(NamespaceGeometry {
                                key: parent.clone(),
                                scope: SignatureNamespaceScope::C(path.clone()),
                                allocation: Some(self.site.clone()),
                            });
                        }
                    }
        // An authored `namespace eval a::b` also creates `::a`; each authored
        // ancestor takes its prefix of this same geometry.
        if let (SourceNamespaceKey::Authored(_), SignatureNamespaceScope::C(path)) = (&key, &scope)
        {
            let mut ancestor = path.parent();
            while let Some(parent) = ancestor.filter(|parent| !parent.is_root()) {
                if let Some(spelling) = tcl_syntax::naming::checked_namespace_path_key(&parent) {
                    let parent_key = SourceNamespaceKey::authored(spelling);
                    if state.namespaces.contains(&parent_key)
                        && !self.world.namespaces.has_key(&parent_key)
                    {
                        self.world.namespaces.push(NamespaceGeometry {
                            key: parent_key,
                            scope: SignatureNamespaceScope::C(parent.clone()),
                            allocation: Some(self.site.clone()),
                        });
                    }
                }
                ancestor = parent.parent();
            }
        }
        if self
            .world
            .namespaces
            .iter()
            .any(|entry| entry.key == key && entry.scope != scope)
        {
            self.world.withdraw();
            return;
        }
        if !self.world.namespaces.has_key(&key) {
            self.world.namespaces.push(NamespaceGeometry {
                key,
                scope,
                allocation: Some(self.site.clone()),
            });
        }
    }

    pub(in super::super) fn finish(mut self, state: &mut ModuleCommandBindings) {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_WORLD_FINISH offset={} old_unavailable={} captured_unavailable={} operations={:?}",
                self.site.offset,
                state.original_command_world.unavailable,
                self.world.unavailable,
                self.operations
                    .iter()
                    .map(|operation| match operation {
                        Operation::Withdraw => "withdraw",
                        Operation::Publish(_) => "publish",
                        Operation::Move(..) => "move",
                        Operation::Delete(_) => "delete",
                        Operation::Ensure(..) => "ensure",
                        Operation::DeleteNamespace(_) => "delete-namespace",
                        Operation::Export(..) => "export",
                        Operation::Import(..) => "import",
                        Operation::Forget(..) => "forget",
                    })
                    .collect::<Vec<_>>()
            );
        }
        // Existing operation/observer transfers can withdraw the ledger after
        // capture. Installing a captured copy must never revive that history.
        if state.original_command_world.unavailable {
            self.world.withdraw();
        }
        for operation in std::mem::take(&mut self.operations) {
            match operation {
                Operation::Withdraw => self.world.withdraw(),
                Operation::Publish(mut publication) => {
                    publication.definition =
                        emitted_definition(state, &publication.site, publication.kind);
                    publication.implementation_allocation =
                        emitted_allocation(state, &publication.site, publication.kind);
                    self.world.publish(*publication);
                }
                Operation::Move(from, to, input) => {
                    // A selected baseline command has no source publication.
                    // Its canonical table transfer still owns occupancy.
                    let Some(index) = self
                        .world
                        .publications
                        .iter()
                        .position(|entry| entry.slot == from)
                    else {
                        continue;
                    };
                    let mut publication =
                        Arc::unwrap_or_clone(self.world.publications.remove(index));
                    publication.slot = to;
                    publication.input = input;
                    self.world.publish(publication);
                }
                Operation::Delete(slot) => {
                    self.world.publications.retain(|entry| entry.slot != slot);
                }
                Operation::Ensure(target, scope, input) => {
                    self.ensure_namespace(state, &target, scope, input.as_ref());
                }
                Operation::DeleteNamespace(scope) => {
                    self.world
                        .namespaces
                        .retain(|entry| !scope_is_within(&entry.scope, &scope));
                    self.world
                        .publications
                        .retain(|entry| !slot_is_within(&entry.slot, &scope));
                    self.world
                        .exports
                        .retain(|(namespace, _)| !scope_is_within(namespace, &scope));
                }
                Operation::Export(scope, clear, inputs) => {
                    if let Some((_, previous)) = self
                        .world
                        .exports
                        .iter_mut()
                        .find(|(namespace, _)| namespace == &scope)
                    {
                        if clear {
                            previous.clear();
                        }
                        previous.extend(inputs);
                    } else {
                        self.world.exports.push((scope, inputs));
                    }
                }
                Operation::Import(scope, force, inputs) => {
                    if self.world.import(&scope, force, &inputs).is_none() {
                        self.world.withdraw();
                    }
                }
                Operation::Forget(scope, inputs) => {
                    if self.world.forget(&scope, &inputs).is_none() {
                        self.world.withdraw();
                    }
                }
            }
        }
        state.original_command_world = Arc::new(self.world);
    }
}

fn emitted_definition(
    state: &ModuleCommandBindings,
    site: &CommandAllocationSite,
    kind: OriginalCommandPublicationKind,
) -> Option<SourceCommandDefinition> {
    let mut definitions = state.bindings.values().filter_map(|bindings| {
        if bindings.len() != 1 {
            return None;
        }
        let super::super::MayBinding::Target(target) = bindings.first()? else {
            return None;
        };
        let definition = state.source_definition_for_implementation(target)?;
        (definition.allocation().site == *site
            && matches!(
                (kind, definition.kind()),
                (
                    OriginalCommandPublicationKind::Procedure,
                    super::super::SourceCommandDefinitionKind::Procedure
                ) | (
                    OriginalCommandPublicationKind::Object,
                    super::super::SourceCommandDefinitionKind::Class
                )
            ))
        .then_some(definition)
    });
    let selected = definitions.next()?;
    definitions
        .all(|definition| definition == selected)
        .then_some(selected)
}

fn emitted_allocation(
    state: &ModuleCommandBindings,
    site: &CommandAllocationSite,
    kind: OriginalCommandPublicationKind,
) -> Option<super::super::CommandAllocation> {
    let mut allocations = state.bindings.values().filter_map(|bindings| {
        if bindings.len() != 1 {
            return None;
        }
        let super::super::MayBinding::Target(target) = bindings.first()? else {
            return None;
        };
        let allocation = target.implementation_allocation.as_ref()?;
        (allocation.site == *site
            && matches!(
                (kind, target.kind),
                (
                    OriginalCommandPublicationKind::Alias,
                    super::super::BindingKind::Alias
                ) | (
                    OriginalCommandPublicationKind::Procedure,
                    super::super::BindingKind::Proc
                ) | (
                    OriginalCommandPublicationKind::Object,
                    super::super::BindingKind::Class
                ) | (
                    OriginalCommandPublicationKind::Command,
                    super::super::BindingKind::Command
                )
            ))
        .then(|| allocation.clone())
    });
    let selected = allocations.next()?;
    allocations
        .all(|allocation| allocation == selected)
        .then_some(selected)
}

fn scope_is_within(child: &SignatureNamespaceScope, parent: &SignatureNamespaceScope) -> bool {
    match (child, parent) {
        (SignatureNamespaceScope::C(child), SignatureNamespaceScope::C(parent)) => {
            child.as_segments().starts_with(parent.as_segments())
        }
        (SignatureNamespaceScope::Jim(child), SignatureNamespaceScope::Jim(parent)) => {
            child == parent
                || parent.is_empty()
                || tcl_syntax::naming::authored_namespace_hint_tail_bytes(
                    child.as_bytes(),
                    parent.as_bytes(),
                )
                .is_some()
        }
        _ => false,
    }
}

fn slot_is_within(slot: &ByteCommandSlot, scope: &SignatureNamespaceScope) -> bool {
    match scope {
        SignatureNamespaceScope::C(path) => {
            slot.namespace.as_segments().starts_with(path.as_segments())
        }
        SignatureNamespaceScope::Jim(value) => {
            value.is_empty()
                || tcl_syntax::naming::authored_namespace_hint_tail_bytes(
                    slot.simple.as_bytes(),
                    value.as_bytes(),
                )
                .is_some()
        }
        SignatureNamespaceScope::Symbolic(_) => false,
    }
}

impl OriginalSourceCommandWorld {
    pub(super) fn import(
        &mut self,
        destination: &SignatureNamespaceScope,
        force: bool,
        inputs: &[SignatureSourceNameInput],
    ) -> Option<()> {
        let policy = self.policy?;
        for input in inputs {
            match input {
                SignatureSourceNameInput::OriginalWord(_)
                | SignatureSourceNameInput::OriginalValue(_) => {}
                SignatureSourceNameInput::OriginalVariableRoot(_)
                | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
            }
            let parts = policy
                .recipe()
                .namespace_pattern_parts(
                    destination.context()?,
                    input.bytes(),
                    NativeNamePurpose::NamespaceImportPattern,
                )
                .ok()?;
            let source = match parts.source? {
                tcl_syntax::naming::NativeNamespacePatternSource::C(path) => {
                    SignatureNamespaceScope::C(path)
                }
                tcl_syntax::naming::NativeNamespacePatternSource::Jim(value) => {
                    SignatureNamespaceScope::Jim(value)
                }
            };
            if !self.namespaces.iter().any(|entry| entry.scope == source) {
                return None;
            }
            let exports = self
                .exports
                .iter()
                .find(|(namespace, _)| namespace == &source)
                .map_or(&[][..], |(_, inputs)| inputs.as_slice());
            let mut imported = Vec::new();
            for publication in &self.publications {
                if !slot_is_within(&publication.slot, &source) {
                    continue;
                }
                let Some(simple) = source_simple(&publication.slot, &source) else {
                    continue;
                };
                if !matches_pattern(
                    policy,
                    parts.tail.as_bytes(),
                    simple,
                    tcl_syntax::native_glob::NativeNameGlobPurpose::ImportSearch,
                )? {
                    continue;
                }
                if !policy.recipe().is_jim084()
                    && !exports.iter().any(|pattern| {
                        matches_pattern(
                            policy,
                            pattern.bytes(),
                            simple,
                            tcl_syntax::native_glob::NativeNameGlobPurpose::ExportFilter,
                        ) == Some(true)
                    })
                {
                    continue;
                }
                let slot = policy
                    .recipe()
                    .command_publication_slot(destination.context()?, simple)
                    .ok()?;
                if !force
                    && self
                        .publications
                        .iter()
                        .any(|previous| previous.slot == slot)
                {
                    return None;
                }
                let mut retained = publication.as_ref().clone();
                retained.slot = slot;
                retained.input = input.clone();
                retained.origin = Some(
                    publication
                        .origin
                        .clone()
                        .unwrap_or_else(|| publication.site.clone()),
                );
                retained.kind = OriginalCommandPublicationKind::Imported;
                imported.push(retained);
            }
            for publication in imported {
                self.publish(publication);
            }
        }
        Some(())
    }

    pub(super) fn forget(
        &mut self,
        destination: &SignatureNamespaceScope,
        inputs: &[SignatureSourceNameInput],
    ) -> Option<()> {
        let policy = self.policy?;
        for input in inputs {
            match input {
                SignatureSourceNameInput::OriginalWord(_)
                | SignatureSourceNameInput::OriginalValue(_) => {}
                SignatureSourceNameInput::OriginalVariableRoot(_)
                | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
            }
            let parts = policy
                .recipe()
                .namespace_pattern_parts(
                    destination.context()?,
                    input.bytes(),
                    NativeNamePurpose::NamespaceForgetPattern,
                )
                .ok()?;
            let mut removed = Vec::new();
            for (index, entry) in self.publications.iter().enumerate() {
                if entry.kind != OriginalCommandPublicationKind::Imported
                    || !slot_is_within(&entry.slot, destination)
                {
                    continue;
                }
                // Implementation contract: naming.namespace.original-qualified-forget-publications
                // docs/design/analysis/name-resolution-proofs/namespace-original-qualified-forget-publications.md
                let matched = match &parts.source {
                    Some(tcl_syntax::naming::NativeNamespacePatternSource::C(path)) => {
                        let site = entry.origin.as_ref()?;
                        let allocation = entry.implementation_allocation.as_ref()?;
                        let mut origins = self.publications.iter().filter(|origin| {
                            origin.kind != OriginalCommandPublicationKind::Imported
                                && &origin.site == site
                                && origin.policy() == policy
                                && origin.implementation_allocation.as_ref() == Some(allocation)
                        });
                        let origin = origins.next()?;
                        if origins.next().is_some() {
                            return None;
                        }
                        &origin.slot.namespace == path
                            && matches_pattern(
                                policy,
                                parts.tail.as_bytes(),
                                origin.slot.simple.as_bytes(),
                                tcl_syntax::native_glob::NativeNameGlobPurpose::ForgetOriginFilter,
                            )?
                    }
                    Some(tcl_syntax::naming::NativeNamespacePatternSource::Jim(_)) => return None,
                    None => matches_pattern(
                        policy,
                        parts.tail.as_bytes(),
                        source_simple(&entry.slot, destination)?,
                        tcl_syntax::native_glob::NativeNameGlobPurpose::ForgetOwnSearch,
                    )?,
                };
                if matched {
                    removed.push(index);
                }
            }
            for index in removed.into_iter().rev() {
                self.publications.remove(index);
            }
        }
        Some(())
    }
}

fn source_simple<'a>(
    slot: &'a ByteCommandSlot,
    scope: &SignatureNamespaceScope,
) -> Option<&'a [u8]> {
    match scope {
        SignatureNamespaceScope::C(path) if path == &slot.namespace => Some(slot.simple.as_bytes()),
        SignatureNamespaceScope::Jim(value) if value.is_empty() => Some(slot.simple.as_bytes()),
        SignatureNamespaceScope::Jim(value) => {
            tcl_syntax::naming::authored_namespace_hint_tail_bytes(
                slot.simple.as_bytes(),
                value.as_bytes(),
            )
        }
        _ => None,
    }
}

fn matches_pattern(
    policy: NamePolicyProtocol,
    pattern: &[u8],
    value: &[u8],
    purpose: tcl_syntax::native_glob::NativeNameGlobPurpose,
) -> Option<bool> {
    tcl_syntax::native_glob::match_native_name_pattern(policy.recipe(), purpose, pattern, value)
        .ok()
}
