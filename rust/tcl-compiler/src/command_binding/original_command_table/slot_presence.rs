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

//! Whether an original slot is present, and the bindings view of its key.

use super::super::{
    BTreeSet, MayBinding, ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey,
};
use super::{
    BaselineLookupTiming, OriginalGeometryTiming, OriginalLookupSlot, RegistryMetadataSlotIndex,
    original_registry_metadata_slot,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};

impl ModuleCommandBindings {
    /// Readonly pre-fallback presence for a genuine original name. Known
    /// absent namespace holders can support absence advice but never produce
    /// a table identity or dispatch target.
    pub(in super::super) fn original_slot_presence_for_input(
        &self,
        input: &SignatureSourceNameInput,
        current: &SourceNamespaceKey,
    ) -> super::super::SourceCommandSlotPresence {
        self.original_slot_presence_in_view(input, current, false)
    }

    /// The same presence for absence advice, which also accepts a world
    /// withheld only by steps the diagnostic view models.
    pub(in super::super) fn original_diagnostic_slot_presence_for_input(
        &self,
        input: &SignatureSourceNameInput,
        current: &SourceNamespaceKey,
    ) -> super::super::SourceCommandSlotPresence {
        self.original_slot_presence_in_view(input, current, true)
    }

    fn original_slot_presence_in_view(
        &self,
        input: &SignatureSourceNameInput,
        current: &SourceNamespaceKey,
        diagnostic: bool,
    ) -> super::super::SourceCommandSlotPresence {
        use super::super::SourceCommandSlotPresence as Presence;
        let input_is_current = input.is_current(&self.source_variables);
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
            eprintln!(
                "JIM_SOURCE_OWNER stage=jim.presence.entry input={:?} policy={:?} current={:?} input_current={} opaque={} unclosed={} native_entry={} unknown_entry={} namespace_known={}",
                input.bytes(),
                input.policy(),
                current,
                input_is_current,
                self.opaque_domain,
                self.unclosed_command_world,
                self.baseline.native_entry.is_some(),
                self.baseline.unknown_entry,
                self.source_variables.namespace_known
            );
        }
        if (self.unclosed_command_world && !(diagnostic && self.diagnostic_command_world_closed))
            || !input_is_current
            || self.baseline.unknown_entry
            || (!tcl_syntax::naming::is_absolute_bytes(input.bytes())
                && matches!(current, SourceNamespaceKey::Authored(_))
                && !self.source_variables.namespace_known)
        {
            return Presence::Unknown;
        }
        let Some(paths) =
            self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())
        else {
            return Presence::Unknown;
        };
        self.original_slot_presence_in_paths(paths, input.policy())
    }

    fn original_slot_presence_in_paths(
        &self,
        paths: Vec<Vec<tcl_core_types::ByteCommandSlot>>,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> super::super::SourceCommandSlotPresence {
        use super::super::SourceCommandSlotPresence as Presence;
        let mut present = false;
        let mut absent = false;
        for path in paths {
            let mut fallthrough = true;
            for slot in path {
                let Some(key) = self.original_lookup_key_for_slot(&slot, policy) else {
                    return Presence::Unknown;
                };
                let Some(key) = key.key() else {
                    continue;
                };
                let Some(bindings) = self.original_bindings_for_key(&key) else {
                    return Presence::Unknown;
                };
                if bindings.is_empty() || bindings.contains(&MayBinding::Unknown) {
                    return Presence::Unknown;
                }
                present |= bindings.iter().any(|binding| {
                    matches!(binding, MayBinding::Target(_) | MayBinding::Imported(_))
                });
                fallthrough = bindings.contains(&MayBinding::Missing);
                if !fallthrough {
                    break;
                }
            }
            absent |= fallthrough;
        }
        match (present, absent) {
            (true, false) => Presence::Present,
            (false, true) => Presence::Absent,
            (true, true) => Presence::MayPresent,
            (false, false) => Presence::Unknown,
        }
    }

    /// Readonly presence for a sealed original expression function-name value.
    /// The source owner separately closes operands, observers and pool effects.
    /// No command-head word or runtime function invocation is manufactured.
    pub(in super::super) fn original_function_diagnostic_presence(
        &self,
        current: &SourceNamespaceKey,
        name: &tcl_registry::mathfunc::NativeExpressionFunctionCommandName,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> super::super::SourceCommandSlotPresence {
        // Proof: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        use super::super::SourceCommandSlotPresence as Presence;
        if name.protocol() != policy.recipe()
            || self
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(policy)
            || self.baseline.unknown_entry
            || (!self.source_variables.namespace_known
                && matches!(current, SourceNamespaceKey::Authored(_)))
        {
            return Presence::Unknown;
        }
        let Some(paths) =
            self.original_command_slot_paths_for_bytes(current, name.command_bytes(), policy)
        else {
            return Presence::Unknown;
        };
        let qualifier_retained = !paths.is_empty()
            && paths.iter().all(|path| {
                path.iter().any(|slot| {
                    self.original_lookup_key_for_slot(slot, policy)
                        .is_some_and(|key| matches!(key, OriginalLookupSlot::Retained(_)))
                })
            });
        let presence = self.original_slot_presence_in_paths(paths, policy);
        self.original_diagnostic_presence_after_absence(
            current,
            policy,
            presence,
            qualifier_retained,
        )
    }

    pub(in super::super) fn original_diagnostic_presence_after_absence(
        &self,
        current: &SourceNamespaceKey,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        presence: super::super::SourceCommandSlotPresence,
        qualifier_retained: bool,
    ) -> super::super::SourceCommandSlotPresence {
        use super::super::SourceCommandSlotPresence as Presence;
        if presence != Presence::Absent {
            return presence;
        }
        if !qualifier_retained
            || self.unknown_lookup_namespaces.contains(current)
            || self.namespace_unknown_handlers.contains(current)
        {
            return Presence::Unknown;
        }
        let Some(fallback) = self.baseline.dialect.and_then(|dialect| {
            tcl_registry::command_lookup::native_lookup_fallback_policy(
                dialect,
                tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
            )
        }) else {
            return Presence::Unknown;
        };
        let Some(paths) =
            self.original_registry_command_paths(current, fallback.default_handler, policy)
        else {
            return Presence::Unknown;
        };
        if paths.is_empty() {
            return Presence::Unknown;
        }
        for handler in paths.into_iter().flatten() {
            let Some(expected) = self.original_initial_bindings_for_key(&handler) else {
                return Presence::Unknown;
            };
            let Some(actual) = self.original_bindings_for_key(&handler) else {
                return Presence::Unknown;
            };
            if actual != expected && actual != BTreeSet::from([MayBinding::Missing]) {
                return Presence::Unknown;
            }
        }
        presence
    }

    /// Each original lookup path must reach a retained qualifier before an
    /// absent command can become a diagnostic. Missing holders grant no
    /// runtime namespace or fallback identity.
    pub(in super::super) fn original_diagnostic_qualifier_is_retained(
        &self,
        input: &SignatureSourceNameInput,
        current: &SourceNamespaceKey,
    ) -> bool {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return false,
        }
        self.original_command_slot_paths_for_bytes(current, input.bytes(), input.policy())
            .is_some_and(|paths| {
                !paths.is_empty()
                    && paths.iter().all(|path| {
                        path.iter().any(|slot| {
                            self.original_lookup_key_for_slot(slot, input.policy())
                                .is_some_and(|key| matches!(key, OriginalLookupSlot::Retained(_)))
                        })
                    })
            })
    }

    /// Actual binding alternatives for one retained original byte key. Fixed
    /// ASCII Registry metadata is selected separately from source publication;
    /// an earlier occupied builtin therefore blocks a later source fallback.
    pub(in super::super) fn original_bindings_for_key(
        &self,
        key: &SourceCommandKey,
    ) -> Option<BTreeSet<MayBinding>> {
        self.original_bindings_view(key).map(|view| view.clone())
    }

    /// [`Self::original_bindings_for_key`] without copying the alternatives:
    /// retained rows are borrowed and baseline rows are shared.
    pub(in super::super) fn original_bindings_view(
        &self,
        key: &SourceCommandKey,
    ) -> Option<BindingsView<'_>> {
        if let Some(bindings) = self.bindings.get(key) {
            return Some(BindingsView::Borrowed(bindings));
        }
        self.original_baseline_bindings_for_key(key, true)
    }

    pub(in super::super) fn original_initial_bindings_for_key(
        &self,
        key: &SourceCommandKey,
    ) -> Option<BTreeSet<MayBinding>> {
        if let Some(bindings) = self.original_entry_bindings.get(key) {
            return Some(bindings.clone());
        }
        self.original_baseline_bindings_for_key(key, false)
            .map(|view| view.clone())
    }

    fn original_baseline_bindings_for_key(
        &self,
        key: &SourceCommandKey,
        retained_authored_changes: bool,
    ) -> Option<BindingsView<'_>> {
        #[cfg(any(test, debug_assertions))]
        let geometry_timing = OriginalGeometryTiming::at_query(5);
        #[cfg(any(test, debug_assertions))]
        let _timing = BaselineLookupTiming::at_query();
        let SourceCommandKey::Slot { namespace, simple } = key else {
            return None;
        };
        let query = self.original_command_world.query_view(self);
        #[cfg(any(test, debug_assertions))]
        if let Some(timing) = geometry_timing.as_ref() {
            timing.record_clone(query.clone_nanos());
        }
        let policy = query.selected_policy()?;
        let scope = query.world().scope(namespace, policy)?;
        let selected = match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(_) => {
                let SignatureNamespaceScope::C(path) = scope else {
                    return None;
                };
                tcl_core_types::ByteCommandSlot {
                    namespace: path,
                    simple: simple.clone(),
                }
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => tcl_core_types::ByteCommandSlot {
                namespace: tcl_core_types::ByteNamespacePath::root(),
                simple: simple.clone(),
            },
        };
        // Native entry command rows already occupy exact keys. An absent row
        // cannot gain a Registry implementation from authored metadata.
        if self.baseline.native_entry.is_some() {
            return Some(BindingsView::missing());
        }
        let index = self
            .baseline
            .metadata_slots
            .get_or_init(|| RegistryMetadataSlotIndex::new(policy, &self.baseline.semantics));
        let name = if let Some(index) = index
            .as_ref()
            .filter(|index| index.matches(policy, &self.baseline.semantics))
        {
            let Some(row) = index.slots.get(&selected) else {
                return Some(BindingsView::missing());
            };
            row.as_ref()?
        } else {
            let mut metadata = self
                .baseline
                .semantics
                .binding_names()
                .iter()
                .filter(|name| {
                    original_registry_metadata_slot(policy, name).as_ref() == Some(&selected)
                });
            let Some(name) = metadata.next() else {
                return Some(BindingsView::missing());
            };
            if metadata.next().is_some() {
                return None;
            }
            name
        };
        let authored = SourceCommandKey::from(name.clone());
        if retained_authored_changes && let Some(bindings) = self.bindings.get(&authored) {
            return Some(BindingsView::Borrowed(bindings));
        }
        if !self
            .baseline
            .semantics
            .binding_names()
            .contains(name.as_str())
        {
            return Some(BindingsView::missing());
        }
        Some(BindingsView::Shared(unmodified_builtin_bindings(
            name,
            || Self::unmodified_bindings(&authored, self.baseline.semantics.binding_names()),
        )))
    }

    /// Existing modeled keys share one binding projection. Original byte
    /// cells use retained namespace geometry and independently selected
    /// baseline metadata; explicitly authored keys retain their legacy role.
    pub(in super::super) fn binding_alternatives(
        &self,
        key: &(impl super::super::CommandKeyQuery + ?Sized),
    ) -> BTreeSet<MayBinding> {
        let key = key.command_key();
        if matches!(key.as_ref(), SourceCommandKey::Slot { .. }) {
            return self
                .original_bindings_for_key(key.as_ref())
                .unwrap_or_else(|| BTreeSet::from([MayBinding::Unknown]));
        }
        self.bindings.get(key.as_ref()).cloned().unwrap_or_else(|| {
            Self::unmodified_bindings(key.as_ref(), self.baseline.semantics.binding_names())
        })
    }
}

/// Command binding alternatives, borrowed from the table or shared with every
/// other lookup of the same unmodified baseline row.
pub(in super::super) enum BindingsView<'a> {
    Borrowed(&'a BTreeSet<MayBinding>),
    Shared(std::sync::Arc<BTreeSet<MayBinding>>),
}

impl BindingsView<'_> {
    pub(super) fn missing() -> Self {
        static MISSING: std::sync::LazyLock<std::sync::Arc<BTreeSet<MayBinding>>> =
            std::sync::LazyLock::new(|| std::sync::Arc::new(BTreeSet::from([MayBinding::Missing])));
        Self::Shared(std::sync::Arc::clone(&MISSING))
    }

    /// Exactly `{Missing}`, without building a set to compare against.
    pub(in super::super) fn is_only_missing(&self) -> bool {
        self.len() == 1 && self.first() == Some(&MayBinding::Missing)
    }
}

impl std::ops::Deref for BindingsView<'_> {
    type Target = BTreeSet<MayBinding>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(bindings) => bindings,
            Self::Shared(bindings) => bindings,
        }
    }
}

/// The unmodified builtin row for `name` depends on the name alone, so every
/// lookup of it shares one set.
fn unmodified_builtin_bindings(
    name: &str,
    build: impl FnOnce() -> BTreeSet<MayBinding>,
) -> std::sync::Arc<BTreeSet<MayBinding>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};
    static ROWS: OnceLock<Mutex<HashMap<String, Arc<BTreeSet<MayBinding>>>>> = OnceLock::new();
    let rows = ROWS.get_or_init(Mutex::default);
    if let Some(row) = rows
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(name)
    {
        return Arc::clone(row);
    }
    let row = Arc::new(build());
    rows.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(name.to_owned())
        .or_insert(row)
        .clone()
}
