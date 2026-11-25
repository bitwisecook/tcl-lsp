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

//! Original source command publications and namespace geometry.
//!
//! This ledger advances at the same Registry operation boundary as the source
//! command table. Original counted operands select native naming purposes;
//! compatibility labels never recover their bytes. A modeled publication is
//! independent of native interpreter existence, compilation and body entry.

#[cfg(test)]
use super::SourceCommandBindings;
use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandDefinition,
    SourceExecutionContext, SourceNamespaceKey, SourceNativeInvocation,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::ByteCommandSlot;
#[cfg(test)]
use tcl_registry::CommandBindingDefinitionKind;
use tcl_registry::{InvocationFacts, TransitionSubject};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol};

mod completed_world;
mod covered_alias;
mod transfer_capture;

pub use completed_world::OriginalCompletedCommandWorld;
pub(super) use completed_world::{trace_completed_world_flag, trace_completed_world_option};
pub(super) use transfer_capture::{capture, namespace_own_command_slot, publication_slot};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NamespaceGeometry {
    key: SourceNamespaceKey,
    scope: SignatureNamespaceScope,
    allocation: Option<CommandAllocationSite>,
}

/// Persistent namespace rows share unchanged branches between flow snapshots.
/// The lazy index records each key's first row and row count.
#[derive(Debug, Clone, Default)]
struct NamespaceLedger {
    rows: Arc<imbl::Vector<NamespaceGeometry>>,
    first: std::sync::OnceLock<Arc<rustc_hash::FxHashMap<SourceNamespaceKey, (usize, usize)>>>,
    c_scopes: std::sync::OnceLock<
        Arc<rustc_hash::FxHashMap<tcl_core_types::ByteNamespacePath, Vec<SourceNamespaceKey>>>,
    >,
}

impl NamespaceLedger {
    fn push(&mut self, row: NamespaceGeometry) {
        Arc::make_mut(&mut self.rows).push_back(row);
        self.first = std::sync::OnceLock::new();
        self.c_scopes = std::sync::OnceLock::new();
    }

    fn retain(&mut self, mut keep: impl FnMut(&NamespaceGeometry) -> bool) {
        let kept: Vec<bool> = self.rows.iter().map(&mut keep).collect();
        if kept.iter().all(|&kept| kept) {
            return;
        }
        let mut kept = kept.into_iter();
        Arc::make_mut(&mut self.rows).retain(|_| kept.next().unwrap_or(false));
        self.first = std::sync::OnceLock::new();
        self.c_scopes = std::sync::OnceLock::new();
    }

    /// `key`'s first row position and its number of rows.
    fn rows_for(&self, key: &SourceNamespaceKey) -> Option<(usize, usize)> {
        let index = self.first.get_or_init(|| {
            let mut index = rustc_hash::FxHashMap::<_, (usize, usize)>::default();
            for (position, row) in self.rows.iter().enumerate() {
                index.entry(row.key.clone()).or_insert((position, 0)).1 += 1;
            }
            Arc::new(index)
        });
        index.get(key).copied()
    }

    /// The first row for `key`, as a scan from the front would find it.
    fn first_for(&self, key: &SourceNamespaceKey) -> Option<&NamespaceGeometry> {
        self.rows_for(key).map(|(position, _)| &self.rows[position])
    }

    /// The row for `key` when it is the only one; a duplicated key has none.
    fn unique_for(&self, key: &SourceNamespaceKey) -> Option<&NamespaceGeometry> {
        self.rows_for(key)
            .and_then(|(position, count)| (count == 1).then(|| &self.rows[position]))
    }

    fn has_key(&self, key: &SourceNamespaceKey) -> bool {
        self.first_for(key).is_some()
    }

    /// Keys whose first row's scope is C `path`.
    fn keys_with_c_scope(&self, path: &tcl_core_types::ByteNamespacePath) -> &[SourceNamespaceKey] {
        self.c_scopes
            .get_or_init(|| {
                let mut index = rustc_hash::FxHashMap::<_, Vec<SourceNamespaceKey>>::default();
                for (position, row) in self.rows.iter().enumerate() {
                    if let SignatureNamespaceScope::C(path) = &row.scope
                        && self
                            .rows_for(&row.key)
                            .is_some_and(|(first, _)| first == position)
                    {
                        index.entry(path.clone()).or_default().push(row.key.clone());
                    }
                }
                Arc::new(index)
            })
            .get(path)
            .map_or(&[], Vec::as_slice)
    }
}

impl From<Vec<NamespaceGeometry>> for NamespaceLedger {
    fn from(rows: Vec<NamespaceGeometry>) -> Self {
        Self {
            rows: Arc::new(rows.into_iter().collect()),
            first: std::sync::OnceLock::new(),
            c_scopes: std::sync::OnceLock::new(),
        }
    }
}

impl std::ops::Deref for NamespaceLedger {
    type Target = imbl::Vector<NamespaceGeometry>;
    fn deref(&self) -> &Self::Target {
        &self.rows
    }
}

impl PartialEq for NamespaceLedger {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.rows, &other.rows) || self.rows == other.rows
    }
}

impl Eq for NamespaceLedger {}

impl std::hash::Hash for NamespaceLedger {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.rows.len().hash(state);
        self.rows.hash(state);
    }
}

/// Category of an original source publication, without runtime-kind authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OriginalCommandPublicationKind {
    /// Procedure declaration selected by the Registry transition owner.
    Procedure,
    /// Object/class command declaration; class identity remains independent.
    Object,
    /// Other Registry-described source command declaration.
    Command,
    /// An interpreter alias, separate from its eventual target implementation.
    Alias,
    /// A namespace import retaining its independently owned original command.
    Imported,
}

/// One original source command occupying an exact modeled byte slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalCommandPublication {
    slot: ByteCommandSlot,
    input: SignatureSourceNameInput,
    site: CommandAllocationSite,
    kind: OriginalCommandPublicationKind,
    definition: Option<SourceCommandDefinition>,
    origin: Option<CommandAllocationSite>,
    implementation_allocation: Option<super::CommandAllocation>,
    alias_target: Option<OriginalAliasTarget>,
}

/// Independently captured original alias lookup recipe. Captured arguments
/// retain their source producers; none becomes a written callsite operand.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalAliasTarget {
    input: SignatureSourceNameInput,
    lookup: tcl_registry::AliasTargetLookup,
    arguments: Vec<SignatureSourceNameInput>,
}

impl OriginalAliasTarget {
    /// Original target value, independent of any alias reporting label.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Actual Registry-selected global or caller-relative lookup mode.
    #[must_use]
    pub const fn lookup(&self) -> tcl_registry::AliasTargetLookup {
        self.lookup
    }
    /// Independently captured prefix values in their actual argv order.
    #[must_use]
    pub fn arguments(&self) -> &[SignatureSourceNameInput] {
        &self.arguments
    }
}

impl OriginalCommandPublication {
    /// Exact current publication geometry; no native namespace incarnation.
    #[must_use]
    pub const fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }
    /// Original operand producer, distinct from the current constructed slot.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Actual operation that created this command allocation.
    #[must_use]
    pub const fn declaration_site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Selected source publication category.
    #[must_use]
    pub const fn kind(&self) -> OriginalCommandPublicationKind {
        self.kind
    }
    /// Independently joined current implementation allocation, when available.
    #[must_use]
    pub const fn definition(&self) -> Option<&SourceCommandDefinition> {
        self.definition.as_ref()
    }
    /// Source declaration retained through a namespace import.
    #[must_use]
    pub const fn imported_origin(&self) -> Option<&CommandAllocationSite> {
        self.origin.as_ref()
    }
    /// Independently selected naming policy.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.input.policy()
    }
    /// Original alias target and captured argv, when their own producers exist.
    #[must_use]
    pub const fn alias_target(&self) -> Option<&OriginalAliasTarget> {
        self.alias_target.as_ref()
    }

    pub(super) fn has_implementation_allocation(
        &self,
        allocation: &super::CommandAllocation,
    ) -> bool {
        self.implementation_allocation.as_ref() == Some(allocation)
    }
}

/// Private flow-sensitive source ledger. Full structural equality participates
/// in the same immutable snapshot and join semantics as the command table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(super) struct OriginalSourceCommandWorld {
    policy: Option<NamePolicyProtocol>,
    unavailable: bool,
    namespaces: NamespaceLedger,
    // Original rows are immutable across flow snapshots. A changed row must
    // detach; ledger order and full row equality/hash remain semantic.
    publications: Vec<Arc<OriginalCommandPublication>>,
    exports: Vec<(SignatureNamespaceScope, Vec<SignatureSourceNameInput>)>,
    jim_stock: MayHoldJimStockPublication,
}

/// Whether any Jim stock publication was ever published into this ledger.
/// Publications only leave by join or removal, so a clear flag proves none
/// remain; a set flag proves nothing. A hint outside equality and hash.
#[derive(Debug, Clone, Copy, Default)]
struct MayHoldJimStockPublication(bool);
impl PartialEq for MayHoldJimStockPublication {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for MayHoldJimStockPublication {}
impl std::hash::Hash for MayHoldJimStockPublication {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// Readonly access to the selected original ledger. An already initialised
/// ledger is borrowed; preparation owns a private clone and never changes the
/// flow-sensitive state. Measurement fields are outside semantic identity.
pub(super) struct OriginalSourceCommandQuery<'a> {
    world: std::borrow::Cow<'a, OriginalSourceCommandWorld>,
    selected_policy: Option<NamePolicyProtocol>,
    #[cfg(any(test, debug_assertions))]
    clone_nanos: Option<u128>,
}

impl OriginalSourceCommandQuery<'_> {
    pub(super) fn world(&self) -> &OriginalSourceCommandWorld {
        &self.world
    }

    pub(super) const fn selected_policy(&self) -> Option<NamePolicyProtocol> {
        self.selected_policy
    }

    #[cfg(any(test, debug_assertions))]
    pub(super) const fn clone_nanos(&self) -> Option<u128> {
        self.clone_nanos
    }
}

impl OriginalSourceCommandWorld {
    /// Borrow only when the existing selection would change no ledger field.
    /// All other entries take the original private clone and policy selection.
    pub(super) fn query_view<'a>(
        &'a self,
        state: &ModuleCommandBindings,
    ) -> OriginalSourceCommandQuery<'a> {
        // naming.command.original-world-readonly-query
        // docs/design/analysis/name-resolution-proofs/original-world-readonly-query.md
        if let Some(policy) = self.current_policy(state)
            && self.policy == Some(policy)
            && state
                .source_root_namespace_key()
                .is_some_and(|root| self.namespaces.has_key(&root))
        {
            return OriginalSourceCommandQuery {
                world: std::borrow::Cow::Borrowed(self),
                selected_policy: Some(policy),
                #[cfg(any(test, debug_assertions))]
                clone_nanos: None,
            };
        }
        #[cfg(any(test, debug_assertions))]
        let clone_started = tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_SOURCE_PHASES")
            .is_some()
            .then(std::time::Instant::now);
        let mut world = self.clone();
        #[cfg(any(test, debug_assertions))]
        let clone_nanos = clone_started.map(|started| started.elapsed().as_nanos());
        let selected_policy = world.select_policy(state);
        OriginalSourceCommandQuery {
            world: std::borrow::Cow::Owned(world),
            selected_policy,
            #[cfg(any(test, debug_assertions))]
            clone_nanos,
        }
    }

    fn current_policy(&self, state: &ModuleCommandBindings) -> Option<NamePolicyProtocol> {
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        (!self.unavailable && self.policy.is_none_or(|previous| previous == policy))
            .then_some(policy)
    }

    /// The naming policy this world selected at its baseline, if any.
    pub(super) const fn selected_policy(&self) -> Option<NamePolicyProtocol> {
        self.policy
    }

    /// Select the immutable source baseline before any branch or operation.
    /// Native entry state and unknown history retain their independent owners.
    pub(super) fn for_baseline(state: &ModuleCommandBindings) -> Self {
        let mut world = Self::default();
        if state.baseline.native_entry.is_none() && !state.baseline.unknown_entry {
            let _ = world.select_policy(state);
        }
        world
    }

    /// Initialise once from the actual entry's namespace contexts after its
    /// command rows have been installed. No Registry metadata fills native
    /// absence, and no withdrawn source ledger can be reseeded here.
    pub(super) fn for_runtime_entry(
        state: &ModuleCommandBindings,
        entry: &tcl_runtime_api::NativeCompilationEntry,
    ) -> Self {
        let mut world = Self {
            unavailable: true,
            ..Self::default()
        };
        if !entry.closed
            || state.has_opaque_domain()
            || state.baseline.unknown_entry
            || state.baseline.native_entry.as_deref() != Some(entry)
            || *state.original_command_world != Self::default()
        {
            return world;
        }
        let Some(policy) = entry.command_name_policy() else {
            return world;
        };
        if state
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
            != Some(policy)
        {
            return world;
        }
        let Some(root) = state.source_root_namespace_key() else {
            return world;
        };
        let mut namespaces = Vec::new();
        for row in &entry.namespaces {
            let Some(key) = super::runtime_entry::native_namespace_key(entry, row.token) else {
                return world;
            };
            let scope = match policy.recipe() {
                NativeNameProtocol::C(_) => SignatureNamespaceScope::C(row.path.clone()),
                NativeNameProtocol::Jim084 => {
                    let Some(object) = row.jim_namespace_object.as_ref() else {
                        // An unrelated C-shaped holder is not a Jim namespace
                        // object. Keep its geometry unavailable when reached.
                        continue;
                    };
                    SignatureNamespaceScope::Jim(object.clone())
                }
            };
            namespaces.push(NamespaceGeometry {
                key,
                scope,
                allocation: None,
            });
        }
        let Some(current) =
            super::runtime_entry::native_namespace_key(entry, entry.current_namespace)
        else {
            return world;
        };
        if !namespaces.iter().any(|namespace| namespace.key == root)
            || !namespaces.iter().any(|namespace| namespace.key == current)
        {
            return world;
        }
        world.policy = Some(policy);
        world.namespaces = namespaces.into();
        world.unavailable = false;
        world
    }

    /// Whether the ledger has been withdrawn on this path.
    pub(super) const fn is_withdrawn(&self) -> bool {
        self.unavailable
    }

    /// Retain the geometry of an authored C namespace a procedure body is
    /// executing in. The body runs only while its namespace exists; this
    /// records nothing about the namespace's commands.
    pub(super) fn enter_authored_namespace(
        &mut self,
        key: &SourceNamespaceKey,
        policy: NamePolicyProtocol,
    ) {
        let SourceNamespaceKey::Authored(path) = key else {
            return;
        };
        if self.unavailable
            || self.policy != Some(policy)
            || !matches!(policy.recipe(), NativeNameProtocol::C(_))
            || self.namespaces.has_key(key)
        {
            return;
        }
        let Ok(address) = policy.recipe().namespace_address_path(
            tcl_syntax::naming::NativeNameContext::root(),
            path.as_bytes(),
        ) else {
            return;
        };
        self.namespaces.push(NamespaceGeometry {
            key: key.clone(),
            scope: SignatureNamespaceScope::C(address),
            allocation: None,
        });
    }

    pub(super) fn withdraw(&mut self) {
        self.unavailable = true;
    }

    /// This world reopened for a replay from an available `entry`: the replay
    /// is withdrawn only where its own walk reaches an unknown command again.
    /// An entry that is itself unavailable leaves the world as it is.
    pub(super) fn with_entry_availability(&self, entry: &Self) -> Self {
        let mut world = self.clone();
        world.unavailable &= entry.unavailable;
        world
    }

    pub(super) fn join(&mut self, other: &Self) -> bool {
        if self == other {
            return false;
        }
        let before = self.clone();
        if self.policy != other.policy || other.unavailable {
            self.unavailable = true;
        }
        self.namespaces
            .retain(|namespace| other.namespaces.contains(namespace));
        self.publications
            .retain(|publication| other.publications.contains(publication));
        if self.exports != other.exports {
            self.unavailable = true;
        }
        *self != before
    }

    pub(super) fn select_policy(
        &mut self,
        state: &ModuleCommandBindings,
    ) -> Option<NamePolicyProtocol> {
        let policy = self.current_policy(state)?;
        let initialize_metadata = self.policy.is_none()
            && state.baseline.native_entry.is_none()
            && !state.baseline.unknown_entry;
        self.policy = Some(policy);
        let root = state.source_root_namespace_key()?;
        if !self.namespaces.has_key(&root) {
            self.namespaces.push(NamespaceGeometry {
                key: root,
                scope: SignatureNamespaceScope::root(Some(policy)),
                allocation: None,
            });
        }
        if initialize_metadata && matches!(policy.recipe(), NativeNameProtocol::C(_)) {
            // Registry binding holders are explicit baseline metadata. This
            // selects their source-model geometry once; it never parses an
            // arbitrary authored namespace label or revives a deleted holder.
            for command in state.baseline.semantics.binding_names() {
                if !command.is_ascii() || command.as_bytes().contains(&0) {
                    continue;
                }
                let Ok(slot) = policy.recipe().command_c_api_publication_slot(
                    tcl_syntax::naming::NativeNameContext::root(),
                    command.as_bytes(),
                ) else {
                    continue;
                };
                let namespace = SourceNamespaceKey::authored(
                    tcl_syntax::naming::key_holder_and_tail(command).0,
                );
                if state.namespaces.contains(&namespace)
                    && !state.unknown_lookup_namespaces.contains(&namespace)
                    && !self.namespaces.has_key(&namespace)
                {
                    self.namespaces.push(NamespaceGeometry {
                        key: namespace,
                        scope: SignatureNamespaceScope::C(slot.namespace),
                        allocation: None,
                    });
                }
            }
        }
        Some(policy)
    }

    /// Geometry supplied by the independently selected loader's fixed metadata.
    /// It retains modeled holders only; no native namespace token is created.
    pub(super) fn retain_trusted_provider_namespaces(
        &mut self,
        state: &mut ModuleCommandBindings,
        loader: &super::TrustedPackageLoader,
    ) -> Option<()> {
        // Proof: naming.package.attested-source-loader-transfer
        // docs/design/analysis/name-resolution-proofs/package-attested-source-loader-transfer.md
        let policy = self.select_policy(state)?;
        if !matches!(policy.recipe(), NativeNameProtocol::C(_))
            || state.baseline.native_entry.is_some()
            || state.has_opaque_domain()
        {
            return None;
        }
        let mut paths = Vec::new();
        for command in loader
            .command_surface
            .iter()
            .chain(&loader.optional_command_surface)
        {
            if !command.is_ascii() || command.as_bytes().contains(&0) {
                return None;
            }
            paths.push(
                policy
                    .recipe()
                    .command_c_api_publication_slot(
                        tcl_syntax::naming::NativeNameContext::root(),
                        command.as_bytes(),
                    )
                    .ok()?
                    .namespace,
            );
        }
        for namespace in loader
            .namespace_exports
            .keys()
            .chain(loader.optional_namespace_exports.keys())
        {
            if !namespace.is_ascii() || namespace.as_bytes().contains(&0) {
                return None;
            }
            paths.push(
                policy
                    .recipe()
                    .namespace_address_path(
                        tcl_syntax::naming::NativeNameContext::root(),
                        namespace.as_bytes(),
                    )
                    .ok()?,
            );
        }
        for mut path in paths {
            loop {
                let name =
                    String::from_utf8(tcl_syntax::naming::native_namespace_full_name_bytes(&path))
                        .ok()?;
                let key = SourceNamespaceKey::authored(name);
                let scope = SignatureNamespaceScope::C(path.clone());
                if self
                    .namespaces
                    .iter()
                    .any(|entry| entry.key == key && entry.scope != scope)
                    || state.unknown_lookup_namespaces.contains(&key)
                    || state.unknown_namespace_paths.contains(&key)
                {
                    return None;
                }
                Arc::make_mut(&mut state.namespaces).insert(key.clone());
                if !self.namespaces.has_key(&key) {
                    self.namespaces.push(NamespaceGeometry {
                        key,
                        scope,
                        allocation: None,
                    });
                }
                let Some(parent) = path.parent() else {
                    break;
                };
                path = parent;
            }
        }
        Some(())
    }

    /// The C geometry of an authored namespace the flow walk ensured outside
    /// an original source transfer (`namespace eval ::a {…}` in the IR walk),
    /// so a lookup in it reads its slots instead of an unknown scope. A label
    /// whose path does not spell it back again stays unmodelled.
    pub(super) fn ensure_authored_namespace(state: &mut ModuleCommandBindings, label: &str) {
        let key = SourceNamespaceKey::authored(label);
        let world = &state.original_command_world;
        let Some(policy) = world
            .current_policy(state)
            .filter(|policy| world.policy == Some(*policy))
        else {
            return;
        };
        if !matches!(policy.recipe(), NativeNameProtocol::C(_))
            || world.namespaces.has_key(&key)
            || !label.is_ascii()
            || label.as_bytes().contains(&0)
            || state.unknown_lookup_namespaces.contains(&key)
            || state.unknown_namespace_paths.contains(&key)
        {
            return;
        }
        let Ok(path) = policy.recipe().namespace_address_path(
            tcl_syntax::naming::NativeNameContext::root(),
            label.as_bytes(),
        ) else {
            return;
        };
        if tcl_syntax::naming::checked_namespace_path_key(&path).as_deref() != Some(label) {
            return;
        }
        Arc::make_mut(&mut state.original_command_world)
            .namespaces
            .push(NamespaceGeometry {
                key,
                scope: SignatureNamespaceScope::C(path),
                allocation: None,
            });
    }

    /// The geometry of a namespace a source command allocated at `site`, such
    /// as a qualifier a `TclOO` object publication creates.
    pub(super) fn ensure_allocated_namespace(
        state: &mut ModuleCommandBindings,
        key: &SourceNamespaceKey,
        site: &CommandAllocationSite,
    ) {
        let Some(path) = key.exact_native_path().cloned() else {
            return;
        };
        if state.original_command_world.namespaces.has_key(key) {
            return;
        }
        Arc::make_mut(&mut state.original_command_world)
            .namespaces
            .push(NamespaceGeometry {
                key: key.clone(),
                scope: SignatureNamespaceScope::C(path),
                allocation: Some(site.clone()),
            });
    }

    /// Original source creation lineage, independent of namespace existence,
    /// command cleanup, variable allocation or successful deletion.
    pub(super) fn source_namespace_creation(
        &self,
        key: &SourceNamespaceKey,
        policy: NamePolicyProtocol,
    ) -> Option<&CommandAllocationSite> {
        if self.unavailable || self.policy != Some(policy) {
            return None;
        }
        self.namespaces.unique_for(key)?.allocation.as_ref()
    }

    /// Retained declaration geometry, independent of the current publication
    /// ledger's availability. This never establishes a live namespace or table.
    pub(super) fn conditional_scope(
        &self,
        key: &SourceNamespaceKey,
        policy: NamePolicyProtocol,
    ) -> Option<SignatureNamespaceScope> {
        if self.policy.is_some_and(|selected| selected != policy) {
            return None;
        }
        match key {
            SourceNamespaceKey::Native(context)
                if matches!(policy.recipe(), NativeNameProtocol::C(_)) =>
            {
                Some(SignatureNamespaceScope::C(context.path.clone()))
            }
            SourceNamespaceKey::Allocated { path, .. }
                if matches!(policy.recipe(), NativeNameProtocol::C(_)) =>
            {
                Some(SignatureNamespaceScope::C(path.clone()))
            }
            _ => self
                .namespaces
                .first_for(key)
                .map(|entry| entry.scope.clone()),
        }
    }

    pub(super) fn conditional_namespace_for_scope(
        &self,
        scope: &SignatureNamespaceScope,
        policy: NamePolicyProtocol,
    ) -> Option<SourceNamespaceKey> {
        if self.policy.is_some_and(|selected| selected != policy) {
            return None;
        }
        let mut matches = self.namespaces.iter().filter(|entry| &entry.scope == scope);
        let selected = matches.next()?.key.clone();
        matches.next().is_none().then_some(selected)
    }

    /// Whether [`Self::scope`] of `key` under `policy` is a Jim scope.
    pub(super) fn has_jim_scope(
        &self,
        key: &SourceNamespaceKey,
        policy: NamePolicyProtocol,
    ) -> bool {
        let retained = !matches!(policy.recipe(), NativeNameProtocol::C(_))
            || matches!(key, SourceNamespaceKey::Authored(_));
        !self.unavailable
            && self.policy.is_none_or(|selected| selected == policy)
            && retained
            && self
                .namespaces
                .first_for(key)
                .is_some_and(|entry| matches!(entry.scope, SignatureNamespaceScope::Jim(_)))
    }

    /// Every key of `namespaces` whose [`Self::scope`] under `policy` is C
    /// `path`, without visiting the others.
    pub(super) fn keys_with_c_scope_view<'a>(
        &self,
        namespaces: &'a super::namespace_context::SourceNamespaceSet,
        path: &tcl_core_types::ByteNamespacePath,
        policy: NamePolicyProtocol,
    ) -> Vec<&'a SourceNamespaceKey> {
        if self.unavailable || self.policy.is_some_and(|selected| selected != policy) {
            return Vec::new();
        }
        let c = matches!(policy.recipe(), NativeNameProtocol::C(_));
        let mut keys: Vec<&SourceNamespaceKey> = if c {
            namespaces.keys_at_path(path).iter().collect()
        } else {
            Vec::new()
        };
        keys.extend(
            self.namespaces
                .keys_with_c_scope(path)
                .iter()
                .filter(|key| !c || matches!(key, SourceNamespaceKey::Authored(_)))
                .filter_map(|key| namespaces.get(key)),
        );
        keys
    }

    pub(super) fn scope(
        &self,
        key: &SourceNamespaceKey,
        policy: NamePolicyProtocol,
    ) -> Option<SignatureNamespaceScope> {
        if self.unavailable || self.policy.is_some_and(|selected| selected != policy) {
            return None;
        }
        match key {
            SourceNamespaceKey::Native(context) => match policy.recipe() {
                NativeNameProtocol::C(_) => Some(SignatureNamespaceScope::C(context.path.clone())),
                NativeNameProtocol::Jim084 => self
                    .namespaces
                    .first_for(key)
                    .map(|entry| entry.scope.clone()),
            },
            SourceNamespaceKey::Allocated { path, .. }
                if matches!(policy.recipe(), NativeNameProtocol::C(_)) =>
            {
                Some(SignatureNamespaceScope::C(path.clone()))
            }
            _ => self
                .namespaces
                .first_for(key)
                .map(|entry| entry.scope.clone()),
        }
    }

    pub(super) fn retained_scope_for_source_advice(
        &self,
        key: &SourceNamespaceKey,
        policy: NamePolicyProtocol,
    ) -> Option<SignatureNamespaceScope> {
        if self.policy != Some(policy) {
            return None;
        }
        self.namespaces
            .unique_for(key)
            .map(|entry| entry.scope.clone())
    }

    /// Whether a Jim stock publication may remain in this ledger.
    pub(super) fn may_hold_jim_stock_publication(&self) -> bool {
        self.jim_stock.0
    }

    pub(super) fn jim_stock_source_effects(
        &self,
        bindings: &super::SourceCommandBindings,
        point_index: usize,
        owner: &Arc<super::OriginalSourceAnalysisOwner>,
    ) -> Vec<super::jim_stock_class::OriginalJimStockSourceEffects> {
        let Some(point) = bindings.points.get(point_index) else {
            return Vec::new();
        };
        let state = &point.state;
        if !std::ptr::eq(self, state.original_command_world.as_ref()) {
            return Vec::new();
        }
        if !self.jim_stock.0
            || self.unavailable
            || state.has_opaque_domain()
            || state.unclosed_command_world
        {
            return Vec::new();
        }
        self.publications
            .iter()
            .filter_map(|publication| {
                let SignatureSourceNameInput::OriginalValue(value) = &publication.input else {
                    return None;
                };
                let name = value.jim_stock_name()?;
                name.matches_class_continuation(point).then_some(())?;
                let publications = self.complete_jim_stock_publications(state, name)?;
                name.source_class_effects(owner, bindings, point_index, publications)
            })
            .collect()
    }
    fn complete_jim_stock_publications(
        &self,
        state: &ModuleCommandBindings,
        dispatcher: &super::jim_stock_class::OriginalJimStockName,
    ) -> Option<Vec<Arc<OriginalCommandPublication>>> {
        let mut publications = Vec::new();
        for helper in tcl_registry::definer::SourceJimStockClassRecipe.publications() {
            let mut matches = self.publications.iter().filter(|publication| {
                let SignatureSourceNameInput::OriginalValue(value) = &publication.input else {
                    return false;
                };
                value
                    .jim_stock_name()
                    .is_some_and(|name| dispatcher.same_source_class_helper(name, *helper))
            });
            let publication = matches.next()?;
            matches.next().is_none().then_some(())?;
            let key =
                state.original_command_key_for_slot(publication.slot(), publication.policy())?;
            let bindings = state.original_bindings_for_key(&key)?;
            let super::MayBinding::Target(target) = bindings.first()? else {
                return None;
            };
            (bindings.len() == 1
                && publication
                    .has_implementation_allocation(target.implementation_allocation.as_ref()?))
            .then_some(())?;
            publications.push(Arc::clone(publication));
        }
        Some(publications)
    }

    pub(super) fn publish_jim_stock_helper(
        &mut self,
        state: &ModuleCommandBindings,
        installed: &super::jim_stock_class::InstalledJimSourceHelper,
    ) -> Option<()> {
        if !installed.name.matches_publication_state(state)
            || self.select_policy(state)? != installed.name.policy()
            || state
                .original_command_key_for_slot(&installed.slot, installed.name.policy())
                .as_ref()
                != Some(&installed.key)
        {
            return None;
        }
        let alternatives = state.bindings.get(&installed.key)?;
        let super::MayBinding::Target(target) = alternatives.first()? else {
            return None;
        };
        if alternatives.len() != 1
            || target.implementation_allocation.as_ref() != Some(&installed.allocation)
        {
            return None;
        }
        let alias = installed
            .alias_target
            .as_ref()
            .map(|input| OriginalAliasTarget {
                input: input.clone(),
                lookup: tcl_registry::AliasTargetLookup::Global,
                arguments: Vec::new(),
            });
        self.publish(OriginalCommandPublication {
            slot: installed.slot.clone(),
            input: SignatureSourceNameInput::OriginalValue(
                crate::signature_scan::scope::SignatureSourceNameValue::from_jim_stock_name(
                    &installed.name,
                ),
            ),
            site: installed.allocation.site.clone(),
            kind: if alias.is_some() {
                OriginalCommandPublicationKind::Alias
            } else {
                OriginalCommandPublicationKind::Command
            },
            definition: None,
            origin: None,
            implementation_allocation: Some(installed.allocation.clone()),
            alias_target: alias,
        });
        Some(())
    }

    fn publish(&mut self, publication: OriginalCommandPublication) {
        if let SignatureSourceNameInput::OriginalValue(value) = &publication.input
            && value.jim_stock_name().is_some()
        {
            self.jim_stock.0 = true;
        }
        self.publications
            .retain(|previous| previous.slot != publication.slot);
        self.publications.push(Arc::new(publication));
    }
}

/// Complete modeled instance-method name roster of one current source class.
/// Builtin names come from its independently selected stock grammar. This is
/// collision advice, with no Native table, call, visibility or edit grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalClassInstanceMethodRoster {
    definition: SourceCommandDefinition,
    publication: OriginalCommandPublication,
    target: super::SourceCommandTarget,
    policy: NamePolicyProtocol,
    names: Arc<std::collections::BTreeSet<tcl_core_types::NameBytes>>,
    entries: Arc<super::SourceReceiverMethodEntries>,
    factory: tcl_registry::native_tcloo_bootstrap::NativeClassFactoryRecipe,
}

/// Capture original operands before the existing operation mutates the table.
/// This is not a second command parser or an independent completion decision.
#[derive(Clone, Copy)]
struct SourceWorldAliasOperands<'a> {
    source_interpreter: &'a TransitionSubject,
    alias: &'a TransitionSubject,
    target_interpreter: &'a TransitionSubject,
    target: &'a TransitionSubject,
    arguments: &'a [TransitionSubject],
    target_lookup: tcl_registry::AliasTargetLookup,
}

impl ModuleCommandBindings {
    /// Complete the same publication operation after genuine class registration.
    /// A conditional layout, historical label or later equal name cannot supply
    /// the independently retained current implementation allocation.
    pub(super) fn retain_original_class_publication_definition(
        &mut self,
        site: &CommandAllocationSite,
        key: &super::SourceCommandKey,
        input: &SignatureSourceNameInput,
    ) -> bool {
        let policy = input.policy();
        if self.current_source_origin.as_ref() != Some(&site.source)
            || !input.is_current(&self.source_variables)
        {
            return false;
        }
        let Some(slot) = self.original_byte_slot_for_key(key, policy) else {
            return false;
        };
        let Some(publication) = self.original_publication_at(&slot, policy) else {
            return false;
        };
        if publication.kind != OriginalCommandPublicationKind::Object
            || publication.site != *site
            || publication.input != *input
            || publication.origin.is_some()
            || publication.alias_target.is_some()
        {
            return false;
        }
        let Some(bindings) = self.original_bindings_for_key(key) else {
            return false;
        };
        let alternatives = bindings.into_iter().collect::<Vec<_>>();
        let [super::MayBinding::Target(target)] = alternatives.as_slice() else {
            return false;
        };
        let Some(allocation) = target.implementation_allocation.as_ref() else {
            return false;
        };
        if target.kind != super::BindingKind::Class
            || target.registry_backed
            || !target.terminal
            || !target.prepended.is_empty()
            || target
                .token
                .as_ref()
                .and_then(|token| token.allocation.as_ref())
                != Some(allocation)
            || allocation.site != *site
            || !publication.has_implementation_allocation(allocation)
        {
            return false;
        }
        let Some(definition) = self.source_definition_for_implementation(target) else {
            return false;
        };
        if definition.kind() != super::SourceCommandDefinitionKind::Class
            || definition.allocation() != allocation
            || publication
                .definition
                .as_ref()
                .is_some_and(|old| old != &definition)
        {
            return false;
        }
        let world = Arc::make_mut(&mut self.original_command_world);
        let Some(publication) = world
            .publications
            .iter_mut()
            .find(|publication| publication.slot == slot && publication.policy() == policy)
        else {
            return false;
        };
        Arc::make_mut(publication).definition = Some(definition);
        true
    }

    pub(super) fn original_publication_at(
        &self,
        slot: &ByteCommandSlot,
        policy: NamePolicyProtocol,
    ) -> Option<&OriginalCommandPublication> {
        if self.original_command_world.unavailable
            || self.original_command_world.policy != Some(policy)
        {
            return None;
        }
        let mut publications = self
            .original_command_world
            .publications
            .iter()
            .filter(|publication| publication.slot() == slot && publication.policy() == policy);
        let selected = publications.next()?;
        publications.next().is_none().then_some(selected.as_ref())
    }
}

// A namespace body's closed normal world is independent of a possible
// compiler/guest error. This grants neither unconditional completion nor an
// entered body, and never clears an already unclosed prefix or child world.
pub(super) fn namespace_body_world_closed_on_normal_return(
    native: SourceNativeInvocation<'_>,
    facts: &InvocationFacts,
    outcomes: &super::SourceOutcomes,
    context: SourceExecutionContext<'_>,
) -> bool {
    use tcl_registry::completion_route::InvocationCompletionRoute as Route;
    use tcl_registry::{NamespaceTransition, OwnedSubcommandResolution, StateTransition};
    if context.compilation().mode != tcl_registry::native_compilation::NativeCompilationMode::Direct
        || context.compilation().frame
            != tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode
        || facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::NamespaceEval,
            )
        || !matches!(
            tcl_registry::script_body_flow::script_body_flow(facts),
            tcl_registry::script_body_flow::ScriptBodyFlow::ConcatenatedScript { .. }
        )
        || !native.target.registry_backed
        || native.target.kind != super::BindingKind::Builtin
        || !native.target.prepended.is_empty()
        || !facts.arg_roles_complete
        || !matches!(facts.subcommand, OwnedSubcommandResolution::Exact { .. })
        || facts.effects.legacy().command_table_mutation
        || super::requires_reentry_guard(facts)
        || !outcomes.abrupt.iter().all(|(route, _)| {
            matches!(
                route,
                Route::Tcl(tcl_registry::completion::CompletionCode::Error)
            )
        })
    {
        return false;
    }
    let Some(transitions) = facts.state_transitions.declared() else {
        return false;
    };
    if transitions.facts().is_empty()
        || !transitions.facts().iter().all(|fact| {
            matches!(
                fact.transition,
                StateTransition::Namespace(NamespaceTransition::Ensure { .. })
            )
        })
    {
        return false;
    }
    let Some(normal) = outcomes.normal.as_deref() else {
        return false;
    };
    !normal.unclosed_command_world
        && !normal.has_opaque_domain()
        && !normal.baseline.unknown_entry
        && !normal.original_command_world.unavailable
        && normal.command_observers.is_quiet()
        && !normal.source_step_observed()
        && !normal.source_execution_observed(None)
        && normal.unknown_handler_is_original()
}

#[cfg(test)]
mod tests;
