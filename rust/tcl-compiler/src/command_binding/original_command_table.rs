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

//! Original native name operands for the canonical source command table.
//! Geometry is conditional source metadata; installations occur only in the
//! existing selected operation transfer, never by a publication-side replay.

use super::{
    BTreeSet, BindingKind, MayBinding, ModuleCommandBindings, ResolvedCommandTarget,
    SourceCommandKey, SourceExecutionContext, SourceNamespaceKey, SourceNativeInvocation,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, InvocationFacts, StateTransition,
    TransitionSubject,
};

#[cfg(any(test, debug_assertions))]
std::thread_local! {
    static BASELINE_LOOKUP_TOTALS: std::cell::Cell<(u64, u128)> = const {
        std::cell::Cell::new((0, 0))
    };
}

#[cfg(any(test, debug_assertions))]
pub(super) fn baseline_lookup_totals() -> (u64, u128) {
    BASELINE_LOOKUP_TOTALS.with(std::cell::Cell::get)
}

/// The start-up environment never changes, so the switch is read once; the
/// timers below sit on every command lookup.
#[cfg(any(test, debug_assertions))]
fn source_phases_traced() -> bool {
    static TRACED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACED.get_or_init(|| tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES").is_some())
}

#[cfg(any(test, debug_assertions))]
struct BaselineLookupTiming(std::time::Instant);

#[cfg(any(test, debug_assertions))]
impl BaselineLookupTiming {
    fn at_query() -> Option<Self> {
        source_phases_traced().then(|| Self(std::time::Instant::now()))
    }
}

#[cfg(any(test, debug_assertions))]
impl Drop for BaselineLookupTiming {
    fn drop(&mut self) {
        BASELINE_LOOKUP_TOTALS.with(|totals| {
            let (calls, nanos) = totals.get();
            totals.set((
                calls.saturating_add(1),
                nanos.saturating_add(self.0.elapsed().as_nanos()),
            ));
        });
    }
}

// Optional measurement state is thread-local and does not participate in a
// source world, correspondence check, cache key, or semantic equality. Query
// times include nested queries; clone time measures only the ledger clone.
#[cfg(any(test, debug_assertions))]
#[derive(Clone, Copy)]
pub(super) struct ProviderValidationTotals {
    pub(super) calls: u64,
    pub(super) nanos: u128,
    pub(super) required_rows: u64,
    pub(super) dependency_rows: u64,
    pub(super) optional_rows: u64,
}

#[cfg(any(test, debug_assertions))]
#[derive(Clone, Copy)]
pub(super) struct OriginalGeometryTotals {
    pub(super) calls: u64,
    pub(super) nanos: u128,
    pub(super) world_clones: u64,
    pub(super) clone_nanos: u128,
}

#[cfg(any(test, debug_assertions))]
impl OriginalGeometryTotals {
    const ZERO: Self = Self {
        calls: 0,
        nanos: 0,
        world_clones: 0,
        clone_nanos: 0,
    };
}

#[cfg(any(test, debug_assertions))]
#[derive(Clone, Copy)]
pub(super) struct OriginalSourceQueryTotals {
    pub(super) provider: ProviderValidationTotals,
    pub(super) geometry: [OriginalGeometryTotals; 7],
}

#[cfg(any(test, debug_assertions))]
std::thread_local! {
    static PROVIDER_VALIDATION_TOTALS: std::cell::Cell<ProviderValidationTotals> = const {
        std::cell::Cell::new(ProviderValidationTotals {
            calls: 0,
            nanos: 0,
            required_rows: 0,
            dependency_rows: 0,
            optional_rows: 0,
        })
    };
    static PROVIDER_VALIDATION_DEPTH: std::cell::Cell<u64> = const {
        std::cell::Cell::new(0)
    };
    static ORIGINAL_GEOMETRY_TOTALS: std::cell::Cell<[OriginalGeometryTotals; 7]> = const {
        std::cell::Cell::new([OriginalGeometryTotals::ZERO; 7])
    };
}

mod catalogue_candidates;
mod command_keys;
mod slot_presence;
mod target_resolution;

pub(super) use catalogue_candidates::{CatalogueSourceCell, classify_catalogue_source_cell};
pub(super) use target_resolution::OriginalCommandTargetSelection;

#[cfg(any(test, debug_assertions))]
pub(super) fn original_source_query_totals() -> OriginalSourceQueryTotals {
    OriginalSourceQueryTotals {
        provider: PROVIDER_VALIDATION_TOTALS.with(std::cell::Cell::get),
        geometry: ORIGINAL_GEOMETRY_TOTALS.with(std::cell::Cell::get),
    }
}

#[cfg(any(test, debug_assertions))]
pub(super) struct ProviderValidationTiming(std::time::Instant);

#[cfg(any(test, debug_assertions))]
impl ProviderValidationTiming {
    pub(super) fn at_query() -> Option<Self> {
        source_phases_traced().then(|| {
            PROVIDER_VALIDATION_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
            Self(std::time::Instant::now())
        })
    }

    pub(super) fn required_row() {
        PROVIDER_VALIDATION_TOTALS.with(|totals| {
            let mut value = totals.get();
            value.required_rows = value.required_rows.saturating_add(1);
            totals.set(value);
        });
    }

    pub(super) fn optional_row() {
        PROVIDER_VALIDATION_TOTALS.with(|totals| {
            let mut value = totals.get();
            value.optional_rows = value.optional_rows.saturating_add(1);
            totals.set(value);
        });
    }

    pub(super) fn dependency_row() {
        if PROVIDER_VALIDATION_DEPTH.with(std::cell::Cell::get) != 0 {
            PROVIDER_VALIDATION_TOTALS.with(|totals| {
                let mut value = totals.get();
                value.dependency_rows = value.dependency_rows.saturating_add(1);
                totals.set(value);
            });
        }
    }
}

#[cfg(any(test, debug_assertions))]
impl Drop for ProviderValidationTiming {
    fn drop(&mut self) {
        let elapsed = self.0.elapsed().as_nanos();
        PROVIDER_VALIDATION_TOTALS.with(|totals| {
            let mut value = totals.get();
            value.calls = value.calls.saturating_add(1);
            value.nanos = value.nanos.saturating_add(elapsed);
            totals.set(value);
        });
        PROVIDER_VALIDATION_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

#[cfg(any(test, debug_assertions))]
pub(super) const ORIGINAL_GEOMETRY_QUERIES: [&str; 7] = [
    "command-key",
    "slot-paths",
    // registry-axis-ok: irreducible — a field label of the command-table trace, not the namespace command; until never
    "namespace",
    "metadata-advice",
    "lookup-key",
    "baseline",
    "slot-for-key",
];

#[cfg(any(test, debug_assertions))]
struct OriginalGeometryTiming {
    query: usize,
    started: std::time::Instant,
}

#[cfg(any(test, debug_assertions))]
impl OriginalGeometryTiming {
    fn at_query(query: usize) -> Option<Self> {
        source_phases_traced().then(|| Self {
            query,
            started: std::time::Instant::now(),
        })
    }

    fn record_clone(&self, elapsed: Option<u128>) {
        let Some(elapsed) = elapsed else {
            return;
        };
        ORIGINAL_GEOMETRY_TOTALS.with(|totals| {
            let mut values = totals.get();
            values[self.query].world_clones = values[self.query].world_clones.saturating_add(1);
            values[self.query].clone_nanos = values[self.query].clone_nanos.saturating_add(elapsed);
            totals.set(values);
        });
    }
}

#[cfg(any(test, debug_assertions))]
impl Drop for OriginalGeometryTiming {
    fn drop(&mut self) {
        let elapsed = self.started.elapsed().as_nanos();
        ORIGINAL_GEOMETRY_TOTALS.with(|totals| {
            let mut values = totals.get();
            values[self.query].calls = values[self.query].calls.saturating_add(1);
            values[self.query].nanos = values[self.query].nanos.saturating_add(elapsed);
            totals.set(values);
        });
    }
}

// Fixed Registry descriptors are independently authored baseline metadata.
// C registration and Jim's flat global table use different naming purposes;
// neither projection manufactures an original source operand or native entry.
pub(super) fn original_registry_metadata_slot(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &str,
) -> Option<tcl_core_types::ByteCommandSlot> {
    with_registry_metadata_slot(
        policy,
        name,
        |slot: Option<&tcl_core_types::ByteCommandSlot>| slot.cloned(),
    )
}

/// Whether a fixed Registry spelling projects to exactly `slot`.
pub(super) fn registry_metadata_slot_is(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &str,
    slot: &tcl_core_types::ByteCommandSlot,
) -> bool {
    with_registry_metadata_slot(policy, name, |projected| projected == Some(slot))
}

/// The projection is pure in its policy and spelling, so each thread retains a
/// bounded memo of it.
fn with_registry_metadata_slot<R>(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &str,
    read: impl FnOnce(Option<&tcl_core_types::ByteCommandSlot>) -> R,
) -> R {
    type Projected = rustc_hash::FxHashMap<
        tcl_syntax::naming::NamePolicyProtocol,
        rustc_hash::FxHashMap<Box<str>, Option<tcl_core_types::ByteCommandSlot>>,
    >;
    const MAX_PROJECTED: usize = 8192;
    thread_local! {
        static PROJECTED: std::cell::RefCell<Projected> = std::cell::RefCell::default();
    }
    PROJECTED.with_borrow_mut(|projected| {
        let names = projected.entry(policy).or_default();
        if !names.contains_key(name) {
            if names.len() >= MAX_PROJECTED {
                names.clear();
            }
            names.insert(
                Box::from(name),
                project_registry_metadata_slot(policy, name),
            );
        }
        read(names[name].as_ref())
    })
}

fn project_registry_metadata_slot(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &str,
) -> Option<tcl_core_types::ByteCommandSlot> {
    use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol};
    if !name.is_ascii() || name.as_bytes().contains(&0) {
        return None;
    }
    match policy.recipe() {
        NativeNameProtocol::C(_) => policy
            .recipe()
            .command_c_api_publication_slot(NativeNameContext::root(), name.as_bytes())
            .ok(),
        NativeNameProtocol::Jim084 => {
            let input = policy
                .recipe()
                .command_lookup_input(NativeNameContext::root(), name.as_bytes())
                .ok()?;
            Some(tcl_core_types::ByteCommandSlot::new(
                tcl_core_types::ByteNamespacePath::root(),
                tcl_core_types::NameBytes::from(input.jim_flat_key()?),
            ))
        }
    }
}

const MAX_BASELINE_METADATA_NAMES: usize = 4096;

/// Derived spelling projection over one immutable Registry baseline. Current
/// table occupancy, namespace geometry and native absence are checked outside.
#[derive(Debug, Clone)]
pub(super) struct RegistryMetadataSlotIndex {
    policy: tcl_syntax::naming::NamePolicyProtocol,
    semantics: std::sync::Arc<tcl_registry::registry::EffectiveRegistrySemantics>,
    names: BTreeSet<String>,
    slots: std::collections::HashMap<tcl_core_types::ByteCommandSlot, Option<String>>,
}

impl RegistryMetadataSlotIndex {
    fn new(
        policy: tcl_syntax::naming::NamePolicyProtocol,
        semantics: &std::sync::Arc<tcl_registry::registry::EffectiveRegistrySemantics>,
    ) -> Option<Self> {
        let names = semantics.binding_names();
        if names.len() > MAX_BASELINE_METADATA_NAMES {
            return None;
        }
        Some(Self {
            policy,
            semantics: std::sync::Arc::clone(semantics),
            names: names.clone(),
            slots: metadata_slots(policy, names),
        })
    }

    fn matches(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        semantics: &std::sync::Arc<tcl_registry::registry::EffectiveRegistrySemantics>,
    ) -> bool {
        self.policy == policy
            && (std::sync::Arc::ptr_eq(&self.semantics, semantics)
                || self.names == *semantics.binding_names())
    }
}

fn metadata_slots(
    policy: tcl_syntax::naming::NamePolicyProtocol,
    names: &BTreeSet<String>,
) -> std::collections::HashMap<tcl_core_types::ByteCommandSlot, Option<String>> {
    let mut slots = std::collections::HashMap::new();
    for name in names {
        if let Some(slot) = original_registry_metadata_slot(policy, name) {
            slots
                .entry(slot)
                .and_modify(|row| *row = None)
                .or_insert_with(|| Some(name.clone()));
        }
    }
    slots
}

type AdviceRows = super::BTreeMap<String, String>;

/// Pure descriptor geometry over one actual immutable metadata map. The weak
/// map observation prevents a warmed index from surviving in-place COW edits.
#[derive(Debug)]
struct RegistryAdviceSlotIndex {
    policy: tcl_syntax::naming::NamePolicyProtocol,
    rows: std::sync::Weak<AdviceRows>,
    slots: std::collections::HashMap<tcl_core_types::ByteCommandSlot, Option<(String, String)>>,
}

impl RegistryAdviceSlotIndex {
    fn new(
        policy: tcl_syntax::naming::NamePolicyProtocol,
        rows: &std::sync::Arc<AdviceRows>,
    ) -> Self {
        let names = rows.keys().cloned().collect();
        let slots = metadata_slots(policy, &names)
            .into_iter()
            .map(|(slot, metadata)| {
                let row = metadata.map(|metadata| {
                    let name = rows[&metadata].clone();
                    (metadata, name)
                });
                (slot, row)
            })
            .collect();
        Self {
            policy,
            rows: std::sync::Arc::downgrade(rows),
            slots,
        }
    }

    fn matches(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        rows: &std::sync::Arc<AdviceRows>,
    ) -> bool {
        self.policy == policy && self.rows.ptr_eq(&std::sync::Arc::downgrade(rows))
    }
}

/// Bounded derived indexes shared by baseline forks. Map birth and the complete
/// policy select cache reuse only; original live lookup barriers remain outside.
#[derive(Debug, Default)]
pub(super) struct RegistryAdviceSlotMemo {
    entries: std::sync::Mutex<Vec<std::sync::Arc<RegistryAdviceSlotIndex>>>,
}

impl RegistryAdviceSlotMemo {
    fn index(
        &self,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        rows: &std::sync::Arc<AdviceRows>,
    ) -> Option<std::sync::Arc<RegistryAdviceSlotIndex>> {
        const MAX_RETAINED_MAPS: usize = 16;
        // Baselines built from one registry snapshot share its descriptor map
        // allocation, so a fresh baseline adopts the index already derived for it.
        static SHARED: std::sync::Mutex<Vec<std::sync::Arc<RegistryAdviceSlotIndex>>> =
            std::sync::Mutex::new(Vec::new());
        if rows.len() > MAX_BASELINE_METADATA_NAMES {
            return None;
        }
        {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            entries.retain(|entry| entry.rows.strong_count() != 0);
            if let Some(entry) = entries.iter().find(|entry| entry.matches(policy, rows)) {
                return Some(std::sync::Arc::clone(entry));
            }
            if entries.len() >= MAX_RETAINED_MAPS {
                return None;
            }
        }
        let shared = {
            let mut shared = SHARED
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            shared.retain(|entry| entry.rows.strong_count() != 0);
            shared
                .iter()
                .find(|entry| entry.matches(policy, rows))
                .map(std::sync::Arc::clone)
        };
        let index = shared.unwrap_or_else(|| {
            let index = std::sync::Arc::new(RegistryAdviceSlotIndex::new(policy, rows));
            let mut shared = SHARED
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if shared.len() < MAX_RETAINED_MAPS
                && !shared.iter().any(|entry| entry.matches(policy, rows))
            {
                shared.push(std::sync::Arc::clone(&index));
            }
            index
        });
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        entries.retain(|entry| entry.rows.strong_count() != 0);
        if let Some(entry) = entries.iter().find(|entry| entry.matches(policy, rows)) {
            return Some(std::sync::Arc::clone(entry));
        }
        if entries.len() < MAX_RETAINED_MAPS {
            entries.push(std::sync::Arc::clone(&index));
        }
        Some(index)
    }
}

pub(super) struct OriginalCommandOperands {
    inputs: Vec<(TransitionSubject, SignatureSourceNameInput)>,
    procedure_body: bool,
    namespace_ensure: Option<TransitionSubject>,
}

#[cfg(test)]
mod tests;

fn command_operand_subjects(facts: &InvocationFacts) -> Vec<&TransitionSubject> {
    let mut subjects = Vec::new();
    if let Some(transitions) = facts.state_transitions.declared() {
        for fact in transitions.facts() {
            if let StateTransition::Namespace(
                tcl_registry::NamespaceTransition::Ensure {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                }
                | tcl_registry::NamespaceTransition::Delete {
                    namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
                },
            ) = &fact.transition
            {
                subjects.push(subject);
            }
            if let StateTransition::Namespace(
                tcl_registry::NamespaceTransition::Export { patterns, .. }
                | tcl_registry::NamespaceTransition::Import { patterns, .. }
                | tcl_registry::NamespaceTransition::Forget { patterns, .. },
            ) = &fact.transition
            {
                subjects.extend(patterns);
            }
            if let StateTransition::Namespace(tcl_registry::NamespaceTransition::SetPath {
                path,
                ..
            }) = &fact.transition
            {
                subjects.push(path);
            }
            if let StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensemble {
                created: Some(tcl_registry::EnsembleCommandName::Named(name)),
                ..
            }) = &fact.transition
            {
                subjects.push(name);
            }
            let StateTransition::CommandBinding(transition) = &fact.transition else {
                continue;
            };
            match transition {
                CommandBindingTransition::Define { name, .. } => subjects.push(name),
                CommandBindingTransition::Move { from, to } => subjects.extend([from, to]),
                CommandBindingTransition::Delete { interpreter, name } => {
                    subjects.extend(interpreter.as_ref());
                    subjects.push(name);
                }
                CommandBindingTransition::Alias {
                    source_interpreter,
                    alias,
                    target_interpreter,
                    target,
                    arguments,
                    ..
                } => {
                    subjects.extend([source_interpreter, alias, target_interpreter, target]);
                    subjects.extend(arguments);
                }
                CommandBindingTransition::Unknown { .. } => {}
            }
        }
    }
    subjects
}

fn namespace_ensure_subject(facts: &InvocationFacts) -> Option<TransitionSubject> {
    (facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::NamespaceEval,
        )
        && facts.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::NamespaceEval))
    .then(|| {
        let transitions = facts.state_transitions.declared()?;
        let [fact] = transitions.facts() else {
            return None;
        };
        let StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure {
            namespace: tcl_registry::NamespaceTransitionTarget::Named(subject),
        }) = &fact.transition
        else {
            return None;
        };
        facts
            .arg_roles
            .iter()
            .any(|(index, role)| {
                *role == tcl_registry::ArgRole::NamespaceName
                    && facts.argument_offset.checked_add(usize::from(*index))
                        == subject.argument_index()
            })
            .then(|| subject.clone())
    })
    .flatten()
}

#[derive(Clone, Copy)]
struct OriginalAliasOperation<'a> {
    source_interpreter: &'a TransitionSubject,
    alias: &'a TransitionSubject,
    target_interpreter: &'a TransitionSubject,
    target: &'a TransitionSubject,
    arguments: &'a [TransitionSubject],
    target_lookup: tcl_registry::AliasTargetLookup,
    declaration: u32,
}

impl OriginalCommandOperands {
    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Self {
        let subjects = command_operand_subjects(facts);
        let inputs = subjects
            .into_iter()
            .filter_map(|subject| {
                original_operand(native, subject, state, context)
                    .map(|input| (subject.clone(), input))
            })
            .collect();
        let namespace_ensure = namespace_ensure_subject(facts);
        let formal_topology =
            super::formal_topology::from_invocation(native, facts, state, context);
        let procedure_body = matches!(
            facts.procedure_definition,
            Some(tcl_registry::native_procedure::NativeProcedureDefinitionSelection::Valid(_))
        ) && formal_topology.is_some()
            && facts
                .arg_roles
                .iter()
                .filter(|(_, role)| *role == tcl_registry::ArgRole::Body)
                .all(|(index, _)| {
                    super::retained_script_operand(
                        facts.argument_offset + usize::from(*index),
                        native.script_operands(),
                        state,
                        native.segment.span.start(),
                        context.config,
                    )
                    .is_some()
                })
            && facts
                .arg_roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Body);
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_COMMAND_CAPTURE offset={} facts={:?} procedure={:?} roles={:?} argument_offset={} inputs={:?} formals={} body={} opaque={} namespace={:?} baseline_policy={:?} variable_dialect={:?} variable_policy={:?}",
                native.segment.span.start(),
                facts.operation,
                facts.procedure_definition,
                facts.arg_roles,
                facts.argument_offset,
                inputs,
                formal_topology.is_some(),
                procedure_body,
                state.has_opaque_domain(),
                context.namespace_identity(),
                state.baseline.execution_name_policy,
                state.source_variables.invocation_dialect,
                state.source_variables.execution_name_policy
            );
        }
        Self {
            inputs,
            procedure_body,
            namespace_ensure,
        }
    }

    fn input(&self, subject: &TransitionSubject) -> Option<&SignatureSourceNameInput> {
        let mut selected = self
            .inputs
            .iter()
            .filter(|(owned, _)| owned == subject)
            .map(|(_, input)| input);
        let first = selected.next()?;
        match first {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        selected.all(|input| input == first).then_some(first)
    }

    /// Retained actual Ensure operand; the Registry ordinal and the complete
    /// original invocation independently supply this producer.
    pub(super) fn namespace_input(
        &self,
        subject: &TransitionSubject,
    ) -> Option<&SignatureSourceNameInput> {
        self.input(subject)
    }

    /// A whole `NamespaceName` operand of the independently selected eval
    /// recipe. A qualifier derived from a command name is a different input.
    pub(super) fn namespace_ensure_input(
        &self,
        subject: &TransitionSubject,
    ) -> Option<&SignatureSourceNameInput> {
        (self.namespace_ensure.as_ref() == Some(subject)).then_some(())?;
        self.input(subject)
    }

    pub(super) fn procedure_key(
        &self,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        current: &SourceNamespaceKey,
    ) -> Option<SourceCommandKey> {
        let mut names = facts
            .state_transitions
            .declared()?
            .command_bindings()
            .filter_map(|transition| match transition {
                CommandBindingTransition::Define {
                    name,
                    kind: CommandBindingDefinitionKind::Procedure,
                } => Some(name),
                _ => None,
            });
        let name = names.next()?;
        if names.next().is_some() || !self.procedure_body {
            return None;
        }
        state.original_definition_key(
            current,
            self.input(name)?,
            CommandBindingDefinitionKind::Procedure,
        )
    }

    /// [`Self::procedure_key`]'s namespace, where the table holds none.
    pub(super) fn procedure_namespace_missing(
        &self,
        facts: &InvocationFacts,
        state: &ModuleCommandBindings,
        current: &SourceNamespaceKey,
    ) -> bool {
        let Some(transitions) = facts.state_transitions.declared() else {
            return false;
        };
        let mut names = transitions
            .command_bindings()
            .filter_map(|transition| match transition {
                CommandBindingTransition::Define {
                    name,
                    kind: CommandBindingDefinitionKind::Procedure,
                } => Some(name),
                _ => None,
            });
        let (Some(name), None) = (names.next(), names.next()) else {
            return false;
        };
        self.procedure_body
            && self.input(name).is_some_and(|input| {
                state.original_definition_namespace_missing(
                    current,
                    input,
                    CommandBindingDefinitionKind::Procedure,
                )
            })
    }

    fn apply_alias(
        &self,
        state: &mut ModuleCommandBindings,
        operation: OriginalAliasOperation<'_>,
    ) -> Option<()> {
        let OriginalAliasOperation {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
            declaration,
        } = operation;
        if !self.input(source_interpreter)?.bytes().is_empty() {
            return Some(());
        }
        let root = state.source_root_namespace_key()?;
        let key = state.original_publication_key(
            &root,
            self.input(alias)?,
            super::namespace_slots::PublicationPurpose::Alias,
        )?;
        let binding = if self.input(target_interpreter)?.bytes().is_empty() {
            let target = self.input(target)?;
            let prepended = arguments
                .iter()
                .map(|argument| self.input(argument).map(effective_input))
                .collect::<Option<Vec<_>>>()?;
            MayBinding::Target(ResolvedCommandTarget {
                command: std::str::from_utf8(target.bytes())
                    .unwrap_or_default()
                    .to_owned(),
                prepended,
                registry_backed: false,
                kind: BindingKind::Alias,
                implementation_generation: declaration,
                terminal: false,
                target_lookup,
                token: None,
                implementation_allocation: None,
            })
        } else {
            MayBinding::Unknown
        };
        state.retain_rebound_name(&key);
        state.install(key, binding);
        Some(())
    }

    /// Install the one command a literal `namespace ensemble create` creates,
    /// at its `-command` operand or the current namespace's own name.
    pub(super) fn apply_ensemble_creation(
        &self,
        state: &mut ModuleCommandBindings,
        created: &tcl_registry::EnsembleCommandName,
        current: &SourceNamespaceKey,
        declaration: u32,
    ) -> Option<SourceCommandKey> {
        let key = match created {
            tcl_registry::EnsembleCommandName::Named(name) => state.original_definition_key(
                current,
                self.input(name)?,
                CommandBindingDefinitionKind::Command,
            )?,
            tcl_registry::EnsembleCommandName::CurrentNamespace => {
                let policy = state.baseline.execution_name_policy?.native_recipe()?;
                let scope = state.original_namespace_geometry(current, policy)?;
                let slot = super::source_command_world::namespace_own_command_slot(&scope)?;
                state.original_command_key_for_slot(&slot, policy)?
            }
        };
        state.install_created_command(key.clone(), declaration);
        Some(key)
    }

    /// One selected operation mutates the canonical command table. Its source
    /// operands never authorize execution, Normal completion or native headers.
    pub(super) fn apply(
        &self,
        state: &mut ModuleCommandBindings,
        transition: &CommandBindingTransition,
        current: &SourceNamespaceKey,
        declaration: u32,
    ) -> Option<()> {
        match transition {
            CommandBindingTransition::Define { name, kind } => {
                let input = self.input(name)?;
                let key =
                    state.original_created_definition_key(current, input, *kind, declaration)?;
                if *kind == CommandBindingDefinitionKind::Procedure && !self.procedure_body {
                    return None;
                }
                let label = ModuleCommandBindings::command_key_label(&key).unwrap_or_default();
                let target = ResolvedCommandTarget {
                    command: label,
                    prepended: Vec::new(),
                    registry_backed: false,
                    kind: BindingKind::of_definition(*kind),
                    implementation_generation: declaration,
                    terminal: true,
                    target_lookup: tcl_registry::AliasTargetLookup::Global,
                    token: None,
                    implementation_allocation: None,
                };
                state.install(key, MayBinding::Target(target));
                #[cfg(test)]
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
                {
                    eprintln!(
                        "ORIGINAL_COMMAND_INSTALLED offset={declaration} bindings={:?}",
                        state.bindings
                    );
                }
            }
            CommandBindingTransition::Move { from, to } => {
                let from = self.input(from)?;
                let to = self.input(to)?;
                let (source, bindings) =
                    state.original_occupied_command_for_input(current, from)?;
                let scope = state.original_namespace_geometry(current, to.policy())?;
                let selected = to
                    .policy()
                    .recipe()
                    .rename_destination_input(scope.context()?, to.bytes())
                    .ok()?;
                if selected.selected().is_empty() {
                    state.remove(source);
                    return Some(());
                }
                let destination = state.original_publication_key(
                    current,
                    to,
                    super::namespace_slots::PublicationPurpose::Rename,
                )?;
                let prior = state.original_bindings_for_key(&destination)?;
                if prior != BTreeSet::from([MayBinding::Missing]) {
                    return None;
                }
                state.replace(source.clone(), BTreeSet::from([MayBinding::Missing]));
                state.retain_rebound_name(&source);
                state.retain_rebound_name(&destination);
                state.replace(destination, bindings);
            }
            CommandBindingTransition::Delete { interpreter, name } => {
                if let Some(interpreter) = interpreter
                    && !self.input(interpreter)?.bytes().is_empty()
                {
                    return Some(());
                }
                let current = if interpreter.is_some() {
                    state.source_root_namespace_key()?
                } else {
                    current.clone()
                };
                let (key, _) =
                    state.original_occupied_command_for_input(&current, self.input(name)?)?;
                state.remove(key);
            }
            CommandBindingTransition::Alias {
                source_interpreter,
                alias,
                target_interpreter,
                target,
                arguments,
                target_lookup,
            } => {
                self.apply_alias(
                    state,
                    OriginalAliasOperation {
                        source_interpreter,
                        alias,
                        target_interpreter,
                        target,
                        arguments,
                        target_lookup: *target_lookup,
                        declaration,
                    },
                )?;
            }
            CommandBindingTransition::Unknown { .. } => return None,
        }
        Some(())
    }
}

fn effective_input(
    input: &SignatureSourceNameInput,
) -> crate::registry_invocation::EffectiveInvocationWord {
    match std::str::from_utf8(input.bytes()) {
        Ok(text) => crate::registry_invocation::EffectiveInvocationWord::Literal(text.to_owned()),
        Err(_) => {
            crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(input.bytes().into())
        }
    }
}

pub(super) fn original_operand(
    native: SourceNativeInvocation<'_>,
    subject: &TransitionSubject,
    state: &ModuleCommandBindings,
    _context: SourceExecutionContext<'_>,
) -> Option<SignatureSourceNameInput> {
    native
        .original_variable_operands
        .input(subject.argument_index()?, &state.source_variables)
        .cloned()
}

enum OriginalLookupSlot {
    Retained(SourceCommandKey),
    MissingHolder,
}

impl OriginalLookupSlot {
    fn key(self) -> Option<SourceCommandKey> {
        match self {
            Self::Retained(key) => Some(key),
            Self::MissingHolder => None,
        }
    }
}

type OriginalAliasLoopPaths = Vec<Vec<(tcl_core_types::ByteCommandSlot, Option<SourceCommandKey>)>>;

#[cfg(test)]
mod conditional_catalogue_source_tests;

#[cfg(test)]
#[path = "original_advice_slot_tests.rs"]
mod advice_slot_tests;
