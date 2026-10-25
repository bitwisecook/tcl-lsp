// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original source command publications and namespace geometry.
//!
//! This ledger advances at the same Registry operation boundary as the source
//! command table. Original counted operands select native naming purposes;
//! compatibility labels never recover their bytes. A modeled publication is
//! independent of native interpreter existence, compilation and body entry.

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceCommandBindings,
    SourceCommandDefinition, SourceExecutionContext, SourceNamespaceKey, SourceNativeInvocation,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::ByteCommandSlot;
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, InvocationFacts, NamespaceTransition,
    NamespaceTransitionTarget, StateTransition, TransitionSubject,
};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol, NativeNamePurpose};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NamespaceGeometry {
    key: SourceNamespaceKey,
    scope: SignatureNamespaceScope,
    allocation: Option<CommandAllocationSite>,
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

/// Complete modeled alias binding effect for an original caller scan. This
/// carries source inventory coverage only, never handler entry or completion.
pub(crate) struct OriginalCoveredAliasTransition {
    _publication: OriginalCommandPublication,
    _target: super::CommandAllocation,
}

impl super::SourceInvocationBinding {
    fn original_covered_alias_metadata(
        &self,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<()> {
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let metadata = self.original_invocation_metadata_for_module(tokens, module, registry)?;
        if metadata.source_analysis_input() != Some(input)
            || self.unknown
            || self.may_be_absent
            || !self.original_argument_completion.is_normal()
        {
            return None;
        }
        Some(())
    }

    /// Join an original alias transition to its exact retained post-transfer
    /// publication and closed source procedure target. An independent source
    /// interpretation, final world or name label cannot replace this owner.
    pub(crate) fn original_covered_alias_transition(
        &self,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
        transition: &CommandBindingTransition,
    ) -> Option<OriginalCoveredAliasTransition> {
        let CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } = transition
        else {
            return None;
        };
        self.original_covered_alias_metadata(tokens, module, input, registry)?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.owns_original_tokens(tokens).then_some(())?;
        let positioned = retained.positioned_for_module(module, registry)?;
        let site = self.invocation_site()?;
        let before = &self.lookup_state.as_ref()?.state;
        if before.has_opaque_domain() || before.source_step_observed() {
            return None;
        }
        let operands = self.original_variable_operands.as_ref()?;
        let operand = |subject: &TransitionSubject| {
            operands.input(subject.argument_index()?, &before.source_variables)
        };
        if !operand(source_interpreter)?.bytes().is_empty()
            || !operand(target_interpreter)?.bytes().is_empty()
        {
            return None;
        }
        let alias_input = operand(alias)?;
        let target_input = operand(target)?;
        let prefix = arguments.iter().map(operand).collect::<Option<Vec<_>>>()?;
        let mut selected = None;
        let mut continuations = positioned
            .points
            .iter()
            .filter(|point| point.continuation_of.as_ref() == Some(site));
        let first = continuations.next()?;
        for point in std::iter::once(first).chain(continuations) {
            let state = &point.state;
            if point.declaration_preview
                || point.dispatch
                || point.namespace_key != self.lookup_namespace_key
                || state.variable_frame != self.variable_frame
                || state.current_source_origin.as_ref() != Some(&site.source)
                || state.baseline != before.baseline
                || state.has_opaque_domain()
                || state.source_step_observed()
                || state.source_execution_observed(None)
            {
                return None;
            }
            let mut rows = state
                .original_command_world
                .publications
                .iter()
                .filter(|row| {
                    row.declaration_site() == site
                        && row.kind() == OriginalCommandPublicationKind::Alias
                        && row.name_input() == alias_input
                });
            let publication = rows.next()?;
            rows.next().is_none().then_some(())?;
            let original = publication.alias_target()?;
            if original.name_input() != target_input
                || original.lookup() != *target_lookup
                || original.arguments().iter().collect::<Vec<_>>() != prefix
                || publication.implementation_allocation.as_ref()?.site != *site
            {
                return None;
            }
            let covered = original_covered_alias_target(
                state,
                before,
                publication,
                alias_input,
                module,
                retained,
                site,
            )?;
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &covered)
            {
                return None;
            }
            selected = Some(covered);
        }
        let (publication, target) = selected?;
        Some(OriginalCoveredAliasTransition {
            _publication: publication,
            _target: target,
        })
    }
}

fn original_covered_alias_target(
    state: &ModuleCommandBindings,
    before: &ModuleCommandBindings,
    publication: &OriginalCommandPublication,
    alias_input: &SignatureSourceNameInput,
    module: &crate::ir::Module,
    retained: &super::RetainedSourceModuleBindings,
    site: &CommandAllocationSite,
) -> Option<(OriginalCommandPublication, super::CommandAllocation)> {
    let root = state.source_root_namespace_key()?;
    let key = state.original_command_key_for_slot(publication.slot(), publication.policy())?;
    let bindings = state.original_bindings_for_key(&key)?;
    let super::MayBinding::Target(alias) = bindings.first()? else {
        return None;
    };
    if bindings.len() != 1 {
        return None;
    }
    let previous = before.original_bindings_for_key(&key)?;
    if previous != std::collections::BTreeSet::from([super::MayBinding::Missing]) {
        return None;
    }
    if alias.kind != super::BindingKind::Alias
        || !publication.has_implementation_allocation(alias.implementation_allocation.as_ref()?)
    {
        return None;
    }
    let selection = state.original_targets_for_input(
        alias_input,
        &root,
        super::CommandTargetLookup::NamedSlots,
    )?;
    if selection.unknown || selection.may_be_absent || selection.targets.is_empty() {
        return None;
    }
    let first_target = selection.targets.first()?;
    let allocation = first_target.implementation_allocation.as_ref()?;
    if allocation.site.source != site.source
        || selection.targets.iter().any(|target| {
            target.kind != super::BindingKind::Proc
                || target.registry_backed
                || !target.terminal
                || target.implementation_allocation.as_ref() != Some(allocation)
                || target.command != first_target.command
                || target.prepended != first_target.prepended
                || selection
                    .original_prefixes
                    .get(target)
                    .and_then(Option::as_ref)
                    .is_none()
        })
    {
        return None;
    }
    let (id, procedure) = module.source_procedure_for_allocation(allocation)?;
    if !retained.matches_original_procedure_summary(id, procedure)
        || procedure.span.start() != allocation.site.offset
    {
        return None;
    }
    Some((publication.clone(), allocation.clone()))
}

/// Private flow-sensitive source ledger. Full structural equality participates
/// in the same immutable snapshot and join semantics as the command table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(super) struct OriginalSourceCommandWorld {
    policy: Option<NamePolicyProtocol>,
    unavailable: bool,
    namespaces: Vec<NamespaceGeometry>,
    // Original rows are immutable across flow snapshots. A changed row must
    // detach; ledger order and full row equality/hash remain semantic.
    publications: Vec<Arc<OriginalCommandPublication>>,
    exports: Vec<(SignatureNamespaceScope, Vec<SignatureSourceNameInput>)>,
}

/// Readonly access to the selected original ledger. An already initialised
/// ledger is borrowed; preparation owns a private clone and never changes the
/// flow-sensitive state. Measurement fields are outside semantic identity.
pub(super) struct OriginalSourceCommandQuery<'a> {
    world: std::borrow::Cow<'a, OriginalSourceCommandWorld>,
    selected_policy: Option<NamePolicyProtocol>,
    #[cfg(test)]
    clone_nanos: Option<u128>,
}

impl OriginalSourceCommandQuery<'_> {
    pub(super) fn world(&self) -> &OriginalSourceCommandWorld {
        &self.world
    }

    pub(super) const fn selected_policy(&self) -> Option<NamePolicyProtocol> {
        self.selected_policy
    }

    #[cfg(test)]
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

        if let Some(policy) = self.current_policy(state)
            && self.policy == Some(policy)
            && state
                .source_root_namespace_key()
                .is_some_and(|root| self.namespaces.iter().any(|entry| entry.key == root))
        {
            return OriginalSourceCommandQuery {
                world: std::borrow::Cow::Borrowed(self),
                selected_policy: Some(policy),
                #[cfg(test)]
                clone_nanos: None,
            };
        }
        let mut world = self.clone();
        let selected_policy = world.select_policy(state);
        OriginalSourceCommandQuery {
            world: std::borrow::Cow::Owned(world),
            selected_policy,
            #[cfg(test)]
            clone_nanos: None,
        }
    }

    fn current_policy(&self, state: &ModuleCommandBindings) -> Option<NamePolicyProtocol> {
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        (!self.unavailable && self.policy.is_none_or(|previous| previous == policy))
            .then_some(policy)
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
        world.namespaces = namespaces;
        world.unavailable = false;
        world
    }

    pub(super) fn withdraw(&mut self) {
        self.unavailable = true;
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
        if !self.namespaces.iter().any(|entry| entry.key == root) {
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
                    && !self.namespaces.iter().any(|entry| entry.key == namespace)
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
                if !self.namespaces.iter().any(|entry| entry.key == key) {
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
        let mut entries = self.namespaces.iter().filter(|entry| &entry.key == key);
        let entry = entries.next()?;
        if entries.next().is_some() {
            return None;
        }
        entry.allocation.as_ref()
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
                .iter()
                .find(|entry| &entry.key == key)
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
                    .iter()
                    .find(|entry| &entry.key == key)
                    .map(|entry| entry.scope.clone()),
            },
            SourceNamespaceKey::Allocated { path, .. }
                if matches!(policy.recipe(), NativeNameProtocol::C(_)) =>
            {
                Some(SignatureNamespaceScope::C(path.clone()))
            }
            _ => self
                .namespaces
                .iter()
                .find(|entry| &entry.key == key)
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
        let mut entries = self.namespaces.iter().filter(|entry| &entry.key == key);
        let entry = entries.next()?;
        entries.next().is_none().then(|| entry.scope.clone())
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
        self.publications
            .retain(|previous| previous.slot != publication.slot);
        self.publications.push(Arc::new(publication));
    }
}

/// Closed modeled root world. An abrupt or merely partial source prefix can
/// never issue this receipt; it grants no physical interpreter execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalCompletedCommandWorld {
    source: Arc<super::SourceOriginId>,
    config: tcl_lexer::LexerConfig,
    state: Arc<ModuleCommandBindings>,
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

impl OriginalClassInstanceMethodRoster {
    /// Exact source class implementation allocation retained by the roster.
    #[must_use]
    pub const fn class_definition(&self) -> &SourceCommandDefinition {
        &self.definition
    }

    /// Current byte publication in the completed source world.
    #[must_use]
    pub const fn publication(&self) -> &OriginalCommandPublication {
        &self.publication
    }

    /// Current modeled class token; no actual invocation point is supplied.
    #[must_use]
    pub const fn class_target(&self) -> &super::SourceCommandTarget {
        &self.target
    }

    /// Independently selected source naming policy of this roster.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Exact current source declaration occupying this own instance name.
    /// Builtin-only names have no source declaration entry; actual calls remain
    /// independently selected at their original invocation point.
    #[must_use]
    pub fn source_method_for_input(
        &self,
        input: &SignatureSourceNameInput,
    ) -> Option<&super::SourceReceiverMethodEntry> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        if input.policy() != self.policy {
            return None;
        }
        super::SourceReceiverMethodEntry::for_original_input(
            &self.entries,
            super::SourceMethodReceiver::Instance,
            input,
        )
    }

    /// Selected grammar's default visibility for a proposed source name.
    /// This does not select or change any current method implementation.
    #[must_use]
    pub fn default_exported(&self, name: &[u8]) -> bool {
        self.factory.grammar().member_default_exported_bytes(name)
    }

    /// All counted names in this closed instance-side source roster.
    pub fn names(&self) -> impl Iterator<Item = &tcl_core_types::NameBytes> {
        self.names.iter()
    }

    /// Read-only proposed name collision under the selected counted purpose.
    #[must_use]
    pub fn contains(&self, name: &[u8]) -> bool {
        self.policy
            .recipe()
            .oo_method_input(name)
            .is_ok_and(|input| {
                self.names
                    .contains(&tcl_core_types::NameBytes::from(input.selected()))
            })
    }
}

impl OriginalCompletedCommandWorld {
    fn class_relation_allocations(
        &self,
        class: &SourceCommandDefinition,
    ) -> Option<(super::CommandAllocation, Vec<super::CommandAllocation>)> {
        let mut actual = self.state.bindings.values().filter_map(|targets| {
            if targets.len() != 1 {
                return None;
            }
            let super::MayBinding::Target(target) = targets.first()? else {
                return None;
            };
            let definition = self.state.source_definition_for_implementation(target)?;
            (&definition == class).then_some(target)
        });
        let target = actual.next()?;
        if actual.any(|other| other != target)
            || target.kind != super::BindingKind::Class
            || !target.terminal
            || !target.prepended.is_empty()
            || self.state.tainted_object_dispatch.contains("*")
            || self.state.tainted_object_dispatch.contains(&target.command)
        {
            return None;
        }
        let definition = self.state.class_definitions.get(target.token.as_ref()?)?;
        if definition.dispatcher.is_some()
            || definition.implementation_generation != target.implementation_generation
            || !self.state.class_definition_dependencies_hold(definition)
        {
            return None;
        }
        let candidates = definition
            .inherited_classes
            .iter()
            .map(|base| {
                let mut classes = self.state.bindings.values().filter_map(|targets| {
                    if targets.len() != 1 {
                        return None;
                    }
                    let super::MayBinding::Target(raw) = targets.first()? else {
                        return None;
                    };
                    if raw.token.as_ref() != base.identity.as_ref()
                        || raw.implementation_generation != base.implementation_generation
                        || raw.implementation_allocation != base.implementation_allocation
                    {
                        return None;
                    }
                    self.state.source_definition_for_implementation(raw)
                });
                let class = classes.next()?;
                if classes.any(|other| other != class) {
                    return None;
                }
                (class.kind() == super::SourceCommandDefinitionKind::Class)
                    .then(|| class.allocation().clone())
            })
            .collect::<Option<Vec<_>>>()?;
        Some((class.allocation().clone(), candidates))
    }

    /// Exact whole original source instance for this completed Normal world.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        self.source.source_image()
    }
    /// Complete original lexer configuration.
    #[must_use]
    pub const fn lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
    /// Original source publications that survive the completed modeled world.
    pub fn declarations(&self) -> impl Iterator<Item = &OriginalCommandPublication> {
        self.state
            .original_command_world
            .publications
            .iter()
            .map(Arc::as_ref)
    }
    /// Exact byte-slot presence among independently retained source publications.
    #[must_use]
    pub fn declaration_at(
        &self,
        slot: &ByteCommandSlot,
        policy: NamePolicyProtocol,
    ) -> Option<&OriginalCommandPublication> {
        self.declarations()
            .find(|entry| entry.policy() == policy && entry.slot() == slot)
    }

    /// Read-only occupancy of one exact command cell in this independently
    /// completed root world. Registry baseline bindings and source/imported
    /// allocations share the canonical table. Unknown holders, policies and
    /// mixed binding alternatives decline; no insertion or execution is granted.
    #[must_use]
    pub fn command_slot_occupied(
        &self,
        slot: &ByteCommandSlot,
        policy: NamePolicyProtocol,
    ) -> Option<bool> {
        if self.state.has_opaque_domain()
            || self.state.original_command_world.unavailable
            || self.state.original_command_world.policy != Some(policy)
        {
            return None;
        }
        let key = self.state.original_command_key_for_slot(slot, policy)?;
        if !self.state.namespaces.contains(key.holder().as_ref())
            || self
                .state
                .unknown_lookup_namespaces
                .contains(key.holder().as_ref())
        {
            return None;
        }
        let bindings = self.state.original_bindings_for_key(&key)?;
        if bindings.is_empty() {
            return None;
        }
        if bindings == std::collections::BTreeSet::from([super::MayBinding::Missing]) {
            return Some(false);
        }
        bindings
            .iter()
            .all(|binding| {
                matches!(
                    binding,
                    super::MayBinding::Target(_) | super::MayBinding::Imported(_)
                )
            })
            .then_some(true)
    }

    /// Current closed instance-name inventory joined to an independently
    /// retained source class declaration. Class-object, native-entry, inherited
    /// and observed dispatch require their own roster and decline here.
    #[must_use]
    pub fn class_instance_method_roster(
        &self,
        class: &SourceCommandDefinition,
    ) -> Option<OriginalClassInstanceMethodRoster> {
        if class.kind() != super::SourceCommandDefinitionKind::Class
            || class.allocation().site.source != self.source
            || self.state.baseline.native_entry.is_some()
            || self.state.has_opaque_domain()
            || self.state.original_command_world.unavailable
            || self.state.source_step_observed()
            || !self.state.command_observers.is_quiet()
            || self.state.tainted_object_dispatch.contains("*")
            || !self.state.object_instances.own_methods.is_empty()
        {
            return None;
        }
        let mut publications = self.declarations().filter(|publication| {
            publication.kind() == OriginalCommandPublicationKind::Object
                && publication.definition() == Some(class)
        });
        let publication = publications.next()?;
        if publications.next().is_some() {
            return None;
        }
        let policy = publication.policy();
        policy.recipe().oo_method_input(b"").ok()?;
        let key = self
            .state
            .original_command_key_for_slot(publication.slot(), policy)?;
        let bindings = self.state.original_bindings_for_key(&key)?;
        let targets = bindings.into_iter().collect::<Vec<_>>();
        let [super::MayBinding::Target(raw)] = targets.as_slice() else {
            return None;
        };
        let identity = raw.token.as_ref()?;
        if raw.kind != super::BindingKind::Class
            || !raw.terminal
            || raw.registry_backed
            || !raw.prepended.is_empty()
            || raw.implementation_allocation.as_ref() != Some(class.allocation())
            || identity.allocation.as_ref() != Some(class.allocation())
            || identity.runtime.is_some()
            || self.state.source_execution_observed(Some(identity))
            || self.state.tainted_object_dispatch.contains(&raw.command)
        {
            return None;
        }
        let definition = self.state.class_definitions.get(identity)?;
        if definition.implementation_generation != raw.implementation_generation
            || definition.dispatcher.is_some()
            || !definition.inherited_classes.is_empty()
            || !self.state.class_definition_dependencies_hold(definition)
            || definition
                .original_factory
                .as_ref()?
                .recipe()
                .support()
                .is_some()
        {
            return None;
        }
        let target = super::SourceCommandTarget {
            command: raw.command.clone(),
            prepended: raw.prepended.clone(),
            original_prepended: None,
            logical_original_prefix: None,
            registry_backed: raw.registry_backed,
            kind: raw.kind,
            identity: raw.token.clone(),
            implementation_generation: raw.implementation_generation,
            implementation_allocation: raw.implementation_allocation.clone(),
            runtime_implementation_generation: self
                .state
                .runtime_implementation_generation(Some(identity)),
        };
        if !self.state.retained_target_is_current(&target) {
            return None;
        }
        Some(OriginalClassInstanceMethodRoster {
            definition: class.clone(),
            publication: publication.clone(),
            target,
            policy,
            names: Arc::clone(definition.instance_methods.as_ref()?),
            entries: Arc::clone(&definition.receiver_method_entries),
            factory: definition.original_factory.as_ref()?.recipe(),
        })
    }

    /// Actual declaring provider selected by one original retained body.
    /// The output report and independently addressable source operand are distinct.
    pub(crate) fn defining_class_name_for_body(
        &self,
        method: &crate::ir::MethodDef,
    ) -> Option<(String, String)> {
        let source = method.body.executed_source.as_deref()?;
        let matches = |declaration: &super::CommandAllocationSite,
                       body: &super::ExecutedScriptSource,
                       formals: &[(String, Option<String>)]| {
            method
                .span
                .is_some_and(|span| span.start() == declaration.offset)
                && source == body
                && method
                    .params
                    .iter()
                    .map(String::as_str)
                    .eq(formals.iter().map(|(name, _)| name.as_str()))
        };
        let mut providers = self
            .state
            .class_definitions
            .iter()
            .filter(|(identity, definition)| match method.kind {
                crate::ir::MethodKind::Method => {
                    definition.receiver_method_entries.values().any(|entry| {
                        entry.receiver() == super::SourceMethodReceiver::Instance
                            && !entry.is_native_class_delegate()
                            && entry
                                .declaring_class()
                                .and_then(|provider| provider.identity.as_ref())
                                == Some(*identity)
                            && matches(entry.declaration(), entry.body(), entry.formals())
                    })
                }
                crate::ir::MethodKind::Constructor => {
                    definition
                        .constructor_provider
                        .as_ref()
                        .and_then(|provider| provider.identity.as_ref())
                        == Some(*identity)
                        && definition
                            .constructor_entry
                            .as_deref()
                            .is_some_and(|entry| {
                                matches(entry.declaration(), entry.body(), entry.formals())
                            })
                }
                crate::ir::MethodKind::Destructor => {
                    definition
                        .destructor_provider
                        .as_ref()
                        .and_then(|provider| provider.identity.as_ref())
                        == Some(*identity)
                        && definition.destructor_entry.as_deref().is_some_and(|entry| {
                            matches(entry.declaration(), entry.body(), entry.formals())
                        })
                }
                crate::ir::MethodKind::ClassMethod => false,
            });
        let (identity, _) = providers.next()?;
        if providers.next().is_some() {
            return None;
        }
        let allocation = identity.allocation.as_ref()?;
        let mut publications = self.declarations().filter(|publication| {
            publication.kind() == OriginalCommandPublicationKind::Object
                && publication.definition().is_some_and(|definition| {
                    definition.kind() == super::SourceCommandDefinitionKind::Class
                        && definition.allocation() == allocation
                })
        });
        let publication = publications.next()?;
        if publications.next().is_some() {
            return None;
        }
        let roster = self.class_instance_method_roster(publication.definition()?)?;
        if roster.target.identity.as_ref() != Some(identity) {
            return None;
        }
        let source_operand = tcl_syntax::naming::native_command_source_spelling(
            publication.policy().recipe(),
            publication.slot(),
        )?;
        let report = String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(
            publication.slot(),
        ))
        .ok()?;
        Some((report, source_operand))
    }

    /// Match this completed world's source publications against an authentic
    /// lookup point. Independent source worlds can contribute declarations;
    /// the same shared rank owner and alternative-consensus guard apply.
    #[must_use]
    pub fn matching_declarations(
        &self,
        lookup: &super::OriginalCommandLookup,
    ) -> Option<Vec<&OriginalCommandPublication>> {
        lookup.matching_slot_publications(
            self.declarations()
                .map(|publication| (publication.slot(), publication.policy(), publication)),
        )
    }
}

impl SourceCommandBindings {
    /// Retained MAY ancestry under the original selected definer and member.
    /// This source projection supplies no method dispatch or entered receiver.
    pub(crate) fn original_class_relation_candidates(
        &self,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
        relation: &crate::ir::ClassRelation,
    ) -> Option<(super::CommandAllocation, Vec<super::CommandAllocation>)> {
        let retained = module.retained_source_bindings.as_deref()?;
        let positioned = retained.positioned_for_module(module, registry)?;
        if !std::ptr::eq(self, positioned) {
            return None;
        }
        let publishes = self.original_class_relation_source(module, registry, relation)?;
        let definer = relation.definer.as_ref();
        let binding = definer.source_binding.as_ref()?;
        let config = binding.original_lexer_config_for_tokens(definer)?;
        let world = self.original_completed_command_world()?;
        let class = if publishes {
            let site = binding.invocation_site()?;
            let mut definitions = world.declarations().filter_map(|publication| {
                let definition = publication.definition()?;
                (definition.kind() == super::SourceCommandDefinitionKind::Class
                    && definition.allocation().site == *site)
                    .then_some(definition)
            });
            let first = definitions.next()?.clone();
            if definitions.any(|other| *other != first) {
                return None;
            }
            first
        } else {
            let selected = self.original_class_configuration_target_at_span(
                *definer.argv.get(relation.target_word)?,
                config,
                registry,
            )?;
            if selected.site() != binding.invocation_site()?
                || selected.written_ordinal().checked_add(1)? != relation.target_word
            {
                return None;
            }
            selected.local_definition()?.clone()
        };
        world.class_relation_allocations(&class)
    }

    fn original_class_relation_source(
        &self,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
        relation: &crate::ir::ClassRelation,
    ) -> Option<bool> {
        let retained = module.retained_source_bindings.as_deref()?;
        let definer = relation.definer.as_ref();
        let member = relation.member.as_ref();
        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)
                .or_else(|| {
                    if module.source_metadata_input.is_some()
                        || !module.source_entry.metadata_context.is_standalone()
                        || module.source_entry.native_entry.is_some()
                        || module.source_entry.unknown_entry
                    {
                        return None;
                    }
                    registry
                        .profile()
                        .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                        .map(crate::registry_invocation::InvocationMetadataContext::from)
                })?;
        let binding = definer.source_binding.as_ref()?;
        binding.original_source_advice_purpose(definer, metadata, registry)?;
        if !retained.owns_original_tokens(definer)
            || definer.synthetic.is_some()
            || member.synthetic.is_some()
            || !member.original_words_match_argv_text()
        {
            return None;
        }
        let config = binding.original_lexer_config_for_tokens(definer)?;
        let site = binding.invocation_site()?;
        let original =
            site.source
                .original_lexical_command(definer.words(), site.offset, config)?;
        let body = original.native_words().get(relation.body_word)?;
        let member_offset = member.words().first()?.source().span.start();
        let original_member =
            site.source
                .original_lexical_command(member.words(), member_offset, config)?;
        if original_member.native_words().iter().any(|word| {
            word.image() != body.image()
                || word.config() != body.config()
                || word.span().start() < body.span().start()
                || word.span().end() > body.span().end()
        }) {
            return None;
        }
        let invocation =
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry,
                metadata,
                definer,
                Some(self),
            )?;
        let realm = binding.invocation_realm()?;
        invocation.with_metadata_schema(registry, metadata, realm, |selected| {
            let grammar = selected.authored_source_definition_body_grammar()?;
            let head = member.argv_texts.first()?;
            let row = grammar
                .source_member_in(head.as_bytes(), Some(metadata.context().authoring_query()))?;
            if row.all_args_ref != Some(tcl_registry::definer::MemberRefKind::Class)
                || !selected
                    .authored_source_script_arguments()?
                    .contains(&relation.body_word.checked_sub(1)?)
            {
                return None;
            }
            if let Some(publication) = selected.authored_source_command_publication() {
                return (publication.argument.checked_add(1)? == relation.target_word)
                    .then_some(true);
            }
            selected
                .state_transitions()
                .facts()
                .iter()
                .find_map(|fact| match &fact.transition {
                    tcl_registry::StateTransition::ObjectDispatch(
                        tcl_registry::ObjectDispatchTransition::Configure { target, .. },
                    ) => (target.argument_index()?.checked_add(1)? == relation.target_word)
                        .then_some(false),
                    _ => None,
                })
        })
    }

    /// Completed original root publication ledger. Partial fallback states,
    /// unknown mutations and observed execution do not supply final liveness.
    #[must_use]
    pub fn original_completed_command_world(&self) -> Option<OriginalCompletedCommandWorld> {
        let state = self.original_completed_root_state.as_ref()?;
        self.completed_original_world_from_normal_state(Arc::clone(state))
    }

    /// The caller must independently own a completed whole-root Normal route.
    pub(super) fn completed_original_world_from_normal_state(
        &self,
        state: Arc<ModuleCommandBindings>,
    ) -> Option<OriginalCompletedCommandWorld> {
        if state.baseline.unknown_entry
            || state.original_command_world.unavailable
            || state.source_step_observed()
            || state.source_execution_observed(None)
        {
            return None;
        }
        Some(OriginalCompletedCommandWorld {
            source: self.root_origin.clone()?,
            config: self.lexer_config?,
            state,
        })
    }
}

impl crate::realm::CommandBindingRealm {
    /// Preserve the same independently completed Normal source publication
    /// world. A legacy realm projection cannot substitute its reporting map.
    #[must_use]
    pub fn original_completed_command_world(&self) -> Option<OriginalCompletedCommandWorld> {
        self.source_bindings_ref()
            .original_completed_command_world()
    }
}

enum Operation {
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

pub(super) struct CapturedOriginalCommandTransfer {
    world: OriginalSourceCommandWorld,
    operations: Vec<Operation>,
    current: SourceNamespaceKey,
    site: CommandAllocationSite,
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

struct SourceWorldCapture<'a> {
    native: SourceNativeInvocation<'a>,
    state: &'a ModuleCommandBindings,
    context: SourceExecutionContext<'a>,
    current: &'a SourceNamespaceKey,
    selected_scope: Option<SignatureNamespaceScope>,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    site: &'a CommandAllocationSite,
}
impl SourceWorldCapture<'_> {
    fn original(&self, subject: &TransitionSubject) -> Option<SignatureSourceNameInput> {
        super::original_command_table::original_operand(
            self.native,
            subject,
            self.state,
            self.context,
        )
    }
    fn scope_for(&self, input: &SignatureSourceNameInput) -> Option<SignatureNamespaceScope> {
        if input.bytes().starts_with(b"::") {
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
                        CommandBindingDefinitionKind::Object => {
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

pub(super) fn capture(
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
        && !super::original_namespace_body::has_original_namespace_body_delegation(
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
                    NamespaceTransition::SetPath { .. } | NamespaceTransition::SetUnknown { .. },
                ) => continue,
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
    if facts.effects.accesses().iter().any(|access| {
        matches!(
            access.domain,
            tcl_registry::world_effect::WorldStateDomain::CommandBindings
                | tcl_registry::world_effect::WorldStateDomain::NamespaceLookup
        ) && !matches!(
            access.mode,
            tcl_registry::world_effect::EffectAccessMode::Read
        )
    }) {
        operations.push(Operation::Withdraw);
    }
    Some(CapturedOriginalCommandTransfer {
        world,
        operations,
        current,
        site,
    })
}

pub(super) fn publication_slot(
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
    let slot = if kind == CommandBindingDefinitionKind::Object {
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
            tcl_registry::InvocationDialect::for_version(version),
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
                        if !self.world.namespaces.iter().any(|entry| &entry.key == parent) {
                            self.world.namespaces.push(NamespaceGeometry {
                                key: parent.clone(),
                                scope: SignatureNamespaceScope::C(path.clone()),
                                allocation: Some(self.site.clone()),
                            });
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
        if !self.world.namespaces.iter().any(|entry| entry.key == key) {
            self.world.namespaces.push(NamespaceGeometry {
                key,
                scope,
                allocation: Some(self.site.clone()),
            });
        }
    }

    pub(super) fn finish(mut self, state: &mut ModuleCommandBindings) {
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
        let super::MayBinding::Target(target) = bindings.first()? else {
            return None;
        };
        let definition = state.source_definition_for_implementation(target)?;
        (definition.allocation().site == *site
            && matches!(
                (kind, definition.kind()),
                (
                    OriginalCommandPublicationKind::Procedure,
                    super::SourceCommandDefinitionKind::Procedure
                ) | (
                    OriginalCommandPublicationKind::Object,
                    super::SourceCommandDefinitionKind::Class
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
) -> Option<super::CommandAllocation> {
    let mut allocations = state.bindings.values().filter_map(|bindings| {
        if bindings.len() != 1 {
            return None;
        }
        let super::MayBinding::Target(target) = bindings.first()? else {
            return None;
        };
        let allocation = target.implementation_allocation.as_ref()?;
        (allocation.site == *site
            && matches!(
                (kind, target.kind),
                (
                    OriginalCommandPublicationKind::Alias,
                    super::BindingKind::Alias
                ) | (
                    OriginalCommandPublicationKind::Procedure,
                    super::BindingKind::Proc
                ) | (
                    OriginalCommandPublicationKind::Object,
                    super::BindingKind::Class
                ) | (
                    OriginalCommandPublicationKind::Command,
                    super::BindingKind::Command
                )
            ))
        .then(|| allocation.clone())
    });
    let selected = allocations.next()?;
    allocations
        .all(|allocation| allocation == selected)
        .then_some(selected)
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

fn scope_is_within(child: &SignatureNamespaceScope, parent: &SignatureNamespaceScope) -> bool {
    match (child, parent) {
        (SignatureNamespaceScope::C(child), SignatureNamespaceScope::C(parent)) => {
            child.as_segments().starts_with(parent.as_segments())
        }
        (SignatureNamespaceScope::Jim(child), SignatureNamespaceScope::Jim(parent)) => {
            child == parent
                || parent.is_empty()
                || child
                    .as_bytes()
                    .strip_prefix(parent.as_bytes())
                    .is_some_and(|tail| tail.starts_with(b"::"))
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
                || slot
                    .simple
                    .as_bytes()
                    .strip_prefix(value.as_bytes())
                    .is_some_and(|tail| tail.starts_with(b"::"))
        }
        SignatureNamespaceScope::Symbolic(_) => false,
    }
}

impl OriginalSourceCommandWorld {
    fn import(
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

    fn forget(
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
        SignatureNamespaceScope::Jim(value) => slot
            .simple
            .as_bytes()
            .strip_prefix(value.as_bytes())?
            .strip_prefix(b"::"),
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
    if context.compilation.mode != tcl_registry::native_compilation::NativeCompilationMode::Direct
        || context.compilation.frame
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
mod tests {
    use super::*;
    use tcl_core_types::{ByteNamespacePath, NameBytes};

    fn analyse(source: &str, version: tcl_dialect::TclVersion) -> SourceCommandBindings {
        analyse_with_compilation(
            source,
            version,
            crate::environment_ingress::authoring_native_compilation(),
        )
    }

    fn analyse_with_compilation(
        source: &str,
        version: tcl_dialect::TclVersion,
        compilation: tcl_registry::native_compilation::NativeCompilationContext,
    ) -> SourceCommandBindings {
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let registry = tcl_registry::model::ingress::static_context_for(match version {
            tcl_dialect::TclVersion::V8_4 => "tcl8.4",
            tcl_dialect::TclVersion::V8_5 => "tcl8.5",
            tcl_dialect::TclVersion::V8_6 => "tcl8.6",
            tcl_dialect::TclVersion::V9_0 => "tcl9.0",
            tcl_dialect::TclVersion::V9_1 => "tcl9.1",
        })
        .commands();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: compilation,
                ..super::super::SourceAnalysisOptions::default()
            },
        )
    }

    fn slot(namespace: &[&[u8]], simple: &[u8]) -> ByteCommandSlot {
        ByteCommandSlot {
            namespace: ByteNamespacePath::from_segments(namespace.iter().copied()),
            simple: NameBytes::from(simple),
        }
    }

    #[test]
    fn evaluated_method_and_publication_operands_refuse_lexical_variable_roots() {
        let bindings = analyse(
            "oo::class create C {method ping {} {}}",
            tcl_dialect::TclVersion::V8_6,
        );
        let world = bindings.original_completed_command_world().unwrap();
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let publication = world.declaration_at(&slot(&[], b"C"), policy).unwrap();
        let roster = world
            .class_instance_method_roster(publication.definition().unwrap())
            .unwrap();
        let entry = roster.entries.values().next().unwrap();
        let input = entry.original_name_input();
        let authored_word = input.original_word_key().unwrap().original_word();
        let value = SignatureSourceNameInput::OriginalValue(
            crate::signature_scan::name_value::SignatureSourceNameValue::from_original_static_word(
                authored_word,
                tcl_syntax::word_rules::WordValueRules::from_config(&authored_word.config()),
                policy,
            )
            .unwrap(),
        );
        let root_scope = SignatureNamespaceScope::root(Some(policy));
        for operand in [input, &value] {
            assert_eq!(roster.source_method_for_input(operand), Some(entry));
            assert_eq!(
                publication_slot(
                    &root_scope,
                    operand,
                    CommandBindingDefinitionKind::Procedure
                ),
                Some(slot(&[], b"ping")),
            );
            assert!(root_scope.child_from_input(operand).is_some());
        }
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document("$ping"),
            tcl_lexer::Span::new(0, 5),
            word.config(),
        )
        .unwrap();
        let variable = &plan.commands[0].words[0];
        let root =
            crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
                variable,
                variable.span(),
                tcl_syntax::word_rules::WordValueRules::from_config(&variable.config()),
                policy,
            )
            .unwrap();
        let root = SignatureSourceNameInput::OriginalVariableRoot(root);
        assert_eq!(root.bytes(), input.bytes());
        assert_eq!(root.policy(), input.policy());
        assert!(roster.source_method_for_input(&root).is_none());
        assert!(
            super::super::SourceReceiverMethodEntry::for_original_input(
                &roster.entries,
                super::super::SourceMethodReceiver::Instance,
                &root,
            )
            .is_none()
        );
        assert!(
            publication_slot(&root_scope, &root, CommandBindingDefinitionKind::Procedure,)
                .is_none()
        );
        assert!(root_scope.child_from_input(&root).is_none());
        let mut transfer = (*world.state.original_command_world).clone();
        let before = transfer.clone();
        assert!(
            transfer
                .import(&root_scope, false, std::slice::from_ref(&root))
                .is_none()
        );
        assert_eq!(transfer, before);
        assert!(
            transfer
                .forget(&root_scope, std::slice::from_ref(&root))
                .is_none()
        );
        assert_eq!(transfer, before);
    }

    #[test]
    fn original_opaque_procedure_completion_and_body_keep_distinct_allocations() {
        // Implementation contract: naming.command.original-completed-cell-occupancy

        let source = r"proc P\uD800 {} {}; proc P\uD801 {} {}; P\uD800";
        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse(source, version);
            let world = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| {
                    panic!(
                        "{}: opaque installed procedure and body",
                        version.dialect_name()
                    )
                });
            let procedures = world
                .declarations()
                .filter(|entry| entry.kind() == OriginalCommandPublicationKind::Procedure)
                .collect::<Vec<_>>();
            assert_eq!(procedures.len(), 2, "{}", version.dialect_name());
            assert_ne!(procedures[0].slot(), procedures[1].slot());
            let point = bindings.invocation_at_source(
                "",
                u32::try_from(source.rfind(r"P\uD800").unwrap()).unwrap(),
            );
            let reference = point
                .original_evaluated_command_reference()
                .unwrap_or_else(|| panic!("{}: opaque call allocation", version.dialect_name()));
            assert_eq!(reference.original_slot(), Some(procedures[0].slot()));
            assert!(
                analyse(r"proc P\uD800 {{arg default extra}} {}", version)
                    .original_completed_command_world()
                    .is_none()
            );
        }
    }

    #[test]
    fn original_class_instance_roster_includes_builtin_and_distinct_source_names() {
        // Implementation contract: naming.tcloo.original-source-instance-method-roster

        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let source =
                r"oo::class create C {method m\uD800 {} {return OLD}; method m\uD800 {} {}}";
            let bindings = analyse(source, version);
            let world = bindings
                .original_completed_command_world()
                .expect("closed source factory");
            let publication = world
                .declaration_at(&slot(&[], b"C"), NamePolicyProtocol::authored_tcl(version))
                .unwrap();
            let class = publication.definition().expect("genuine class declaration");
            let roster = world
                .class_instance_method_roster(class)
                .expect("closed own source inventory");
            assert_eq!(roster.class_definition(), class);
            assert_eq!(roster.publication(), publication);
            assert!(
                roster.contains(b"destroy"),
                "selected builtin blocks a new member"
            );
            assert!(roster.contains(b"m\xed\xa0\x80"));
            assert!(!roster.contains(b"m\xed\xa0\x81"));
            assert!(roster.default_exported(b"m\xed\xa0\x80"));
            assert!(!roster.default_exported(b"Upper"));
            assert!(!roster.default_exported(b"\xed\xa0\x80"));
            let entry = roster
                .entries
                .values()
                .next()
                .expect("current own source entry");
            assert_eq!(
                roster.source_method_for_input(entry.original_name_input()),
                Some(entry),
            );
            assert_eq!(
                entry.declaration().offset,
                u32::try_from(source.rfind("method").unwrap()).unwrap(),
                "latest declaration owns the surviving name",
            );
            assert!(roster.names().any(|name| name.as_bytes() == b"destroy"));
            let foreign = analyse("oo::class create C {}", version)
                .original_completed_command_world()
                .unwrap();
            assert!(foreign.class_instance_method_roster(class).is_none());
            let mut unknown = world.clone();
            let state = Arc::make_mut(&mut unknown.state);
            let token = roster.class_target().identity.as_ref().unwrap();
            Arc::make_mut(&mut state.class_definitions)
                .get_mut(token)
                .unwrap()
                .instance_methods = None;
            assert!(unknown.class_instance_method_roster(class).is_none());
        }
    }

    #[test]
    fn original_completed_occupancy_keeps_baseline_source_and_unknown_holders_separate() {
        // Implementation contract: naming.command.original-completed-cell-occupancy

        for version in tcl_dialect::TclVersion::ALL {
            let policy = NamePolicyProtocol::authored_tcl(version);
            let world = analyse("", version)
                .original_completed_command_world()
                .unwrap();
            assert_eq!(
                world.command_slot_occupied(&slot(&[], b"proc"), policy),
                Some(true)
            );
            assert_eq!(
                world.command_slot_occupied(&slot(&[], b"missing"), policy),
                Some(false)
            );
            assert_eq!(
                world.command_slot_occupied(&slot(&[b"Missing"], b"p"), policy),
                None
            );
            let other = if version == tcl_dialect::TclVersion::V8_4 {
                tcl_dialect::TclVersion::V8_5
            } else {
                tcl_dialect::TclVersion::V8_4
            };
            assert_eq!(
                world.command_slot_occupied(
                    &slot(&[], b"proc"),
                    NamePolicyProtocol::authored_tcl(other)
                ),
                None
            );
            let world = analyse(r"proc P\uD800 {} {}; rename P\uD800 Q\uD801", version)
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}: opaque procedure move", version.dialect_name()));
            assert_eq!(
                world.command_slot_occupied(&slot(&[], b"P\xed\xa0\x80"), policy),
                Some(false)
            );
            assert_eq!(
                world.command_slot_occupied(&slot(&[], b"Q\xed\xa0\x81"), policy),
                Some(true)
            );
            assert!(
                analyse("proc P {} {}; eval $unknown", version)
                    .original_completed_command_world()
                    .is_none()
            );
        }
    }

    #[test]
    fn original_opaque_jim_callable_name_does_not_gate_ascii_static_capture() {
        // Implementation contract: naming.procedure.original-jim-static-list-capture-inputs

        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some("jim")).unwrap();
        let dialect = tcl_registry::InvocationDialect::of_point(point);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let analyse = |source: &str| {
            super::super::SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(dialect),
                    ..Default::default()
                },
            )
        };
        let bindings =
            analyse(r"proc P\uD800 {} {{counter 1}} {}; proc P\uD801 {} {{counter 2}} {}");
        let world = bindings.original_completed_command_world().expect(
            "exact installed callable allocation owns statics independent of reporting name",
        );
        let declarations = world
            .declarations()
            .filter(|declaration| declaration.kind() == OriginalCommandPublicationKind::Procedure)
            .collect::<Vec<_>>();
        assert_eq!(declarations.len(), 2);
        assert_ne!(declarations[0].slot(), declarations[1].slot());
        assert!(
            declarations
                .iter()
                .all(|declaration| declaration.definition().is_some())
        );
        assert!(
            analyse(r"proc P\uD800 {} {counter &counter} {}")
                .original_completed_command_world()
                .is_none()
        );
    }

    #[test]
    fn publication_snapshots_share_original_rows_and_detach_actual_moves_and_replacements() {
        // naming.command.original-publication-row-sharing

        // Actual source ledger snapshots only, with no external evaluation,
        // Native freshness, selected handler or source-edit claim.
        let source = "proc alpha {} {}; proc beta {} {}; rename alpha gamma; rename gamma beta";
        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse(source, version);
            let first_site = u32::try_from(source.find("rename alpha").unwrap()).unwrap();
            let second_site = u32::try_from(source.find("rename gamma").unwrap()).unwrap();
            let first = &bindings
                .dispatch_points_at(first_site)
                .next()
                .unwrap()
                .state
                .original_command_world;
            let second = &bindings
                .dispatch_points_at(second_site)
                .next()
                .unwrap()
                .state
                .original_command_world;
            let complete = bindings
                .original_completed_command_world()
                .expect("completed original rename sequence");
            let final_world = &complete.state.original_command_world;
            let row = |world: &OriginalSourceCommandWorld, name: &[u8]| {
                world
                    .publications
                    .iter()
                    .find(|entry| entry.slot.simple.as_bytes() == name)
                    .cloned()
                    .expect("actual publication")
            };
            let alpha = row(first, b"alpha");
            let beta = row(first, b"beta");
            let gamma = row(second, b"gamma");
            assert!(
                Arc::ptr_eq(&beta, &row(second, b"beta")),
                "an unchanged row retains its actual producer"
            );
            assert!(
                !Arc::ptr_eq(&alpha, &gamma),
                "a moved publication detaches its own row"
            );
            assert_eq!(alpha.name_input().bytes(), b"alpha");
            assert_eq!(gamma.name_input().bytes(), b"gamma");
            assert_eq!(alpha.declaration_site(), gamma.declaration_site());
            assert_eq!(alpha.definition(), gamma.definition());
            let replaced = row(final_world, b"beta");
            assert!(!Arc::ptr_eq(&beta, &replaced));
            assert!(!Arc::ptr_eq(&gamma, &replaced));
            assert_eq!(replaced.declaration_site(), alpha.declaration_site());
            assert_eq!(replaced.name_input().bytes(), b"beta");
            assert_eq!(beta.name_input().bytes(), b"beta");
            assert_ne!(beta.declaration_site(), replaced.declaration_site());
            assert_eq!(first.publications.len(), 2);
            assert_eq!(second.publications.len(), 2);
            assert_eq!(final_world.publications.len(), 1);
            let branch = first.as_ref().clone();
            assert_eq!(&branch, first.as_ref());
            for (original, shared) in first.publications.iter().zip(&branch.publications) {
                assert!(Arc::ptr_eq(original, shared));
            }
            let mut changed = branch;
            let altered = Arc::make_mut(&mut changed.publications[0]);
            altered.definition = None;
            assert_eq!(alpha.name_input().bytes(), b"alpha");
            assert!(
                alpha.definition().is_some(),
                "changing a private branch cannot erase the original definition"
            );
            assert_ne!(&changed, first.as_ref());
        }
    }

    #[test]
    fn publication_row_sharing_preserves_complete_semantic_equality_hash_and_order() {
        // naming.command.original-publication-row-sharing

        use std::hash::{Hash, Hasher};
        let hash = |world: &OriginalSourceCommandWorld| {
            let mut state = std::collections::hash_map::DefaultHasher::new();
            world.hash(&mut state);
            state.finish()
        };
        let source = "proc alpha {} {}; proc beta {} {}; rename alpha gamma";
        for version in tcl_dialect::TclVersion::ALL {
            let first = analyse(source, version)
                .original_completed_command_world()
                .unwrap();
            let independent = analyse(source, version)
                .original_completed_command_world()
                .unwrap();
            let left = first.state.original_command_world.as_ref();
            let right = independent.state.original_command_world.as_ref();
            assert_eq!(left, right);
            assert_eq!(hash(left), hash(right));
            let names = first
                .declarations()
                .map(|publication| publication.slot.simple.clone())
                .collect::<Vec<_>>();
            assert_eq!(
                names,
                [
                    NameBytes::from(b"beta".as_slice()),
                    NameBytes::from(b"gamma".as_slice())
                ]
            );
            assert!(
                first
                    .declarations()
                    .zip(independent.declarations())
                    .all(|(a, b)| a == b)
            );
        }
    }

    #[test]
    fn readonly_original_world_queries_borrow_exact_selected_baselines_and_publications() {
        // naming.command.original-world-readonly-query

        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let state = ModuleCommandBindings::initial(registry);
            let original = state.original_command_world.as_ref();
            let query = original.query_view(&state);
            let policy = original.policy.expect("actual selected baseline recipe");
            assert!(matches!(&query.world, std::borrow::Cow::Borrowed(_)));
            assert!(std::ptr::eq(query.world(), original));
            assert_eq!(query.selected_policy(), Some(policy));
            assert_eq!(query.clone_nanos(), None);
            assert_eq!(
                query
                    .world()
                    .scope(&state.source_root_namespace_key().unwrap(), policy),
                Some(SignatureNamespaceScope::root(Some(policy)))
            );
        }
        let bindings = analyse(
            "proc original {} {}; interp alias {} forwarding {} original; rename original moved",
            tcl_dialect::TclVersion::V9_1,
        );
        let state = bindings.original_completed_root_state.as_ref().unwrap();
        let original = state.original_command_world.as_ref();
        let query = original.query_view(state);
        assert!(std::ptr::eq(query.world(), original));
        assert!(query.world().publications.iter().any(|publication| {
            publication.slot == slot(&[], b"moved")
                && publication.kind == OriginalCommandPublicationKind::Procedure
        }));
        assert!(query.world().publications.iter().any(|publication| {
            publication.slot == slot(&[], b"forwarding")
                && publication.kind == OriginalCommandPublicationKind::Alias
                && publication
                    .alias_target
                    .as_ref()
                    .unwrap()
                    .name_input()
                    .bytes()
                    == b"original"
        }));
        assert!(
            query
                .world()
                .publications
                .iter()
                .all(|publication| { publication.slot != slot(&[], b"original") })
        );
    }

    #[test]
    fn readonly_original_world_queries_prepare_only_owned_missing_policy_or_root() {
        // naming.command.original-world-readonly-query

        let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        let state = ModuleCommandBindings::initial(registry);
        let original = state.original_command_world.as_ref();
        let policy = original.policy.unwrap();
        let root = state.source_root_namespace_key().unwrap();
        let mut missing_root = original.clone();
        missing_root
            .namespaces
            .retain(|namespace| namespace.key != root);
        let query = missing_root.query_view(&state);
        assert!(matches!(&query.world, std::borrow::Cow::Owned(_)));
        assert_eq!(query.selected_policy(), Some(policy));
        assert_eq!(
            query.world().scope(&root, policy),
            Some(SignatureNamespaceScope::root(Some(policy)))
        );
        assert!(
            missing_root
                .namespaces
                .iter()
                .all(|namespace| namespace.key != root)
        );
        let unselected = OriginalSourceCommandWorld::default();
        let query = unselected.query_view(&state);
        assert!(matches!(&query.world, std::borrow::Cow::Owned(_)));
        assert_eq!(query.selected_policy(), Some(policy));
        assert_eq!(query.world(), original);
        assert_eq!(unselected, OriginalSourceCommandWorld::default());
    }

    #[test]
    fn readonly_original_world_queries_keep_policy_refusal_and_cow_withdrawal_current() {
        // naming.command.original-world-readonly-query

        let registry = tcl_registry::model::ingress::static_context_for("tcl9.1").commands();
        let state = ModuleCommandBindings::initial(registry);
        let original = state.original_command_world.as_ref();
        let earlier_query = original.query_view(&state);
        let foreign_registry =
            tcl_registry::model::ingress::static_context_for("tcl8.4").commands();
        let foreign_state = ModuleCommandBindings::initial(foreign_registry);
        let refused = original.query_view(&foreign_state);
        assert!(matches!(&refused.world, std::borrow::Cow::Owned(_)));
        assert_eq!(refused.selected_policy(), None);
        assert_eq!(refused.world(), original);
        let mut current = state.clone();
        current.mark_opaque_binding_mutation();
        assert!(!Arc::ptr_eq(
            &state.original_command_world,
            &current.original_command_world
        ));
        let withdrawn = current.original_command_world.query_view(&current);
        assert!(matches!(&withdrawn.world, std::borrow::Cow::Owned(_)));
        assert_eq!(withdrawn.selected_policy(), None);
        assert!(withdrawn.world().unavailable);
        assert!(!earlier_query.world().unavailable);
        assert_eq!(earlier_query.selected_policy(), original.policy);
        let unknown = ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions {
                unknown_entry: true,
                ..Default::default()
            },
            None,
        );
        let unavailable = unknown.original_command_world.query_view(&unknown);
        assert_eq!(unavailable.selected_policy(), None);
        assert!(unavailable.world().unavailable);
    }

    #[test]
    fn original_world_baseline_is_selected_before_branching() {
        // Implementation contract: naming.command.original-world-baseline-bootstrap

        for version in tcl_dialect::TclVersion::ALL {
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
            let state = ModuleCommandBindings::initial(registry);
            let policy = NamePolicyProtocol::authored_tcl(version);
            assert_eq!(state.original_command_world.policy, Some(policy));
            let mut invoked = (*state.original_command_world).clone();
            assert_eq!(invoked.select_policy(&state), Some(policy));
            assert!(!invoked.join(&state.original_command_world));
            assert!(!invoked.unavailable);
            invoked.withdraw();
            assert_eq!(invoked.select_policy(&state), None);
            assert!(invoked.unavailable);
            let unknown = ModuleCommandBindings::initial_with_options(
                registry,
                super::super::SourceAnalysisOptions {
                    unknown_entry: true,
                    ..super::super::SourceAnalysisOptions::default()
                },
                None,
            );
            assert!(unknown.original_command_world.unavailable);
            assert_eq!(unknown.original_command_world.policy, None);
        }
    }

    #[test]
    fn original_simple_procedure_installs_before_following_command() {
        for version in tcl_dialect::TclVersion::ALL {
            let source = "proc p {} {}; set checkpoint READY; p";
            let bindings = analyse(source, version);
            let point = bindings
                .invocation_at_source("", u32::try_from(source.rfind("; p").unwrap() + 2).unwrap());
            let reference = point
                .original_evaluated_command_reference()
                .expect("original procedure is installed in the actual following lookup state");
            assert_eq!(reference.original_slot(), Some(&slot(&[], b"p")));
            assert_eq!(reference.definition().unwrap().allocation().site.offset, 0);
        }
    }

    #[test]
    fn original_alias_prefix_inputs_follow_actual_allocations_and_held_values() {
        // Implementation contract: naming.alias.captured-prefix-allocation-contract

        for version in tcl_dialect::TclVersion::ALL {
            let source = "set name first; interp alias {} inner {} set $name; interp alias {} outer {} inner; set name second; outer VALUE; set checkpoint $first";
            let bindings = analyse(source, version);
            let offset = u32::try_from(source.find("outer VALUE").unwrap()).unwrap();
            let point = bindings.invocation_at_source("outer", offset);
            let target = point
                .proved_handler_target()
                .expect("one current alias implementation chain");
            let inputs = target
                .original_prepended_name_inputs()
                .expect("captured producer survives a known later cell write");
            assert_eq!(
                inputs
                    .iter()
                    .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes)
                    .collect::<Vec<_>>(),
                vec![b"first".as_slice()]
            );
            assert!(
                inputs[0].original_word_key().is_none(),
                "a captured read is not a written call operand"
            );
            let last = bindings.invocation_at_source(
                "set",
                u32::try_from(source.rfind("set checkpoint").unwrap()).unwrap(),
            );
            assert_eq!(
                last.evaluated_written_argument_value(2),
                Some("VALUE"),
                "the captured input reaches the actual Set operand owner"
            );
            let replaced = "interp alias {} write {} set first; interp alias {} write {} set second; write VALUE; set checkpoint $second";
            let bindings = analyse(replaced, version);
            let point = bindings.invocation_at_source(
                "write",
                u32::try_from(replaced.find("write VALUE").unwrap()).unwrap(),
            );
            let target = point
                .proved_handler_target()
                .expect("replacement has its own current allocation");
            assert_eq!(
                target.original_prepended_name_inputs().unwrap()[0].bytes(),
                b"second"
            );
        }
    }

    #[test]
    fn completed_original_publications_follow_moves_and_namespace_lifetimes() {
        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse(
                "proc P {} {}; rename P Q; namespace eval N {proc p {} {}}",
                version,
            );
            let world = bindings
                .original_completed_command_world()
                .expect("closed genuine Normal root");
            let policy = NamePolicyProtocol::authored_tcl(version);
            assert!(world.declaration_at(&slot(&[], b"P"), policy).is_none());
            let moved = world
                .declaration_at(&slot(&[], b"Q"), policy)
                .expect("retained original move");
            assert_eq!(moved.kind(), OriginalCommandPublicationKind::Procedure);
            assert!(
                moved.definition().is_some(),
                "emitted implementation allocation joins independently"
            );
            assert!(world.declaration_at(&slot(&[b"N"], b"p"), policy).is_some());
            let deleted = analyse(
                "namespace eval N {proc p {} {}}; namespace delete N; namespace eval N {}",
                version,
            );
            assert!(
                deleted
                    .original_completed_command_world()
                    .expect("completed delete and fresh empty namespace")
                    .declaration_at(&slot(&[b"N"], b"p"), policy)
                    .is_none()
            );
        }
    }

    #[test]
    fn completed_world_requires_normal_root_and_closed_mutations() {
        for version in tcl_dialect::TclVersion::ALL {
            for source in [
                "proc P {} {}; error BOOM",
                "proc P {} {}; eval $unknown",
                "proc P {} {}; rename $unknown Q",
            ] {
                assert!(
                    analyse(source, version)
                        .original_completed_command_world()
                        .is_none(),
                    "{version:?}: {source}"
                );
            }
            let bindings = analyse("proc rename args {}; proc P {} {}; rename P Q", version);
            let world = bindings
                .original_completed_command_world()
                .expect("source replacement body completes normally");
            let policy = NamePolicyProtocol::authored_tcl(version);
            assert!(world.declaration_at(&slot(&[], b"P"), policy).is_some());
            assert!(
                world.declaration_at(&slot(&[], b"Q"), policy).is_none(),
                "replaced Registry worker cannot donate a move"
            );
        }
    }

    #[test]
    fn original_qualified_forget_joins_current_origin_after_source_and_alias_moves() {
        // Implementation contract: naming.namespace.original-qualified-forget-publications

        let source = r"rename auto_import savedPrelude; namespace eval A {proc p\uD800 {} {}; proc p\uD801 {} {}; namespace export p*}; namespace eval B {namespace import ::A::*; rename p\uD800 alias\uD800}; rename ::A::p\uD800 ::A::q\uD800; namespace eval B {namespace forget ::A::p*}";
        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse(source, version);
            let world = bindings
                .original_completed_command_world()
                .unwrap_or_else(|| panic!("{}", version.dialect_name()));
            let policy = NamePolicyProtocol::authored_tcl(version);
            let retained = world
                .declaration_at(&slot(&[b"B"], b"alias\xed\xa0\x80"), policy)
                .unwrap();
            let origin = world
                .declaration_at(&slot(&[b"A"], b"q\xed\xa0\x80"), policy)
                .unwrap();
            assert_eq!(retained.kind(), OriginalCommandPublicationKind::Imported);
            assert_eq!(retained.imported_origin(), Some(origin.declaration_site()));
            assert_eq!(retained.definition(), origin.definition());
            assert!(
                world
                    .declaration_at(&slot(&[b"B"], b"p\xed\xa0\x81"), policy)
                    .is_none()
            );
            assert!(
                world
                    .declaration_at(&slot(&[b"A"], b"p\xed\xa0\x81"), policy)
                    .is_some()
            );
        }
    }

    #[test]
    fn original_namespace_world_requires_the_explicit_source_compilation_mode() {
        // Implementation contract: naming.namespace.original-qualified-forget-publications

        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse_with_compilation(
                "namespace eval N {proc p {} {}}",
                version,
                tcl_registry::native_compilation::NativeCompilationContext::default(),
            );
            assert!(
                bindings.original_completed_command_world().is_none(),
                "{version:?}"
            );
        }
    }

    #[test]
    fn original_namespace_import_keeps_the_source_declaration_allocation() {
        for version in tcl_dialect::TclVersion::ALL {
            let bindings = analyse(
                "rename auto_import savedPrelude; namespace eval A {proc p {} {}; namespace export p}; namespace eval B {namespace import ::A::p}",
                version,
            );
            let world = bindings
                .original_completed_command_world()
                .expect("closed original namespace operations");
            let policy = NamePolicyProtocol::authored_tcl(version);
            let origin = world
                .declaration_at(&slot(&[b"A"], b"p"), policy)
                .expect("source declaration");
            let imported = world
                .declaration_at(&slot(&[b"B"], b"p"), policy)
                .expect("exact original import");
            assert_eq!(imported.kind(), OriginalCommandPublicationKind::Imported);
            assert_eq!(imported.imported_origin(), Some(origin.declaration_site()));
            assert_eq!(imported.definition(), origin.definition());
        }
    }
}
