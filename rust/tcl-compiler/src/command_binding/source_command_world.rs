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

/// Private flow-sensitive source ledger. Full structural equality participates
/// in the same immutable snapshot and join semantics as the command table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(super) struct OriginalSourceCommandWorld {
    policy: Option<NamePolicyProtocol>,
    unavailable: bool,
    namespaces: Vec<NamespaceGeometry>,
    publications: Vec<OriginalCommandPublication>,
    exports: Vec<(SignatureNamespaceScope, Vec<SignatureSourceNameInput>)>,
}

impl OriginalSourceCommandWorld {
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
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        if self.unavailable || self.policy.is_some_and(|previous| previous != policy) {
            return None;
        }
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

    fn publish(&mut self, publication: OriginalCommandPublication) {
        self.publications
            .retain(|previous| previous.slot != publication.slot);
        self.publications.push(publication);
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
        self.state.original_command_world.publications.iter()
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

    #[cfg(debug_assertions)]
    fn trace_roster_admission(&self, class: &SourceCommandDefinition) {
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ROSTER_BEGIN kind={:?} source={} native={} opaque={} unavailable={} step={} quiet={} taint={} own_tables={}",
                class.kind(),
                class.allocation().site.source == self.source,
                self.state.baseline.native_entry.is_some(),
                self.state.has_opaque_domain(),
                self.state.original_command_world.unavailable,
                self.state.source_step_observed(),
                self.state.command_observers.is_quiet(),
                self.state.tainted_object_dispatch.contains("*"),
                self.state.object_instances.own_methods.len()
            );
        }
    }

    #[cfg(debug_assertions)]
    fn trace_roster_publications(&self, class: &SourceCommandDefinition) {
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ROSTER_PUBLICATIONS count={}",
                self.declarations()
                    .filter(|publication| {
                        publication.kind() == OriginalCommandPublicationKind::Object
                            && publication.definition() == Some(class)
                    })
                    .count()
            );
        }
    }

    #[cfg(debug_assertions)]
    fn trace_roster_target(
        &self,
        class: &SourceCommandDefinition,
        raw: &super::ResolvedCommandTarget,
        identity: &super::CommandIdentity,
    ) {
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ROSTER_TARGET kind={:?} terminal={} registry={} prefix={} allocation={} token_allocation={} runtime={} observed={} taint={} definition={}",
                raw.kind,
                raw.terminal,
                raw.registry_backed,
                raw.prepended.len(),
                raw.implementation_allocation.as_ref() == Some(class.allocation()),
                identity.allocation.as_ref() == Some(class.allocation()),
                identity.runtime.is_some(),
                self.state.source_execution_observed(Some(identity)),
                self.state.tainted_object_dispatch.contains(&raw.command),
                self.state.class_definitions.contains_key(identity)
            );
        }
    }

    #[cfg(debug_assertions)]
    fn trace_roster_definition(
        &self,
        raw: &super::ResolvedCommandTarget,
        definition: &super::ClassDefinitionReceipt,
    ) {
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_ROSTER_DEFINITION generation={} dispatcher={} ancestors={} dependencies={} factory={} support={} inventory={} entries={}",
                definition.implementation_generation == raw.implementation_generation,
                definition.dispatcher.is_some(),
                definition.inherited_classes.len(),
                self.state.class_definition_dependencies_hold(definition),
                definition.original_factory.is_some(),
                definition
                    .original_factory
                    .as_ref()
                    .is_some_and(|factory| factory.recipe().support().is_some()),
                definition.instance_methods.is_some(),
                definition.receiver_method_entries.len()
            );
        }
    }

    /// Current closed instance-name inventory joined to an independently
    /// retained source class declaration. Class-object, native-entry, inherited
    /// and observed dispatch require their own roster and decline here.
    #[must_use]
    pub fn class_instance_method_roster(
        &self,
        class: &SourceCommandDefinition,
    ) -> Option<OriginalClassInstanceMethodRoster> {
        #[cfg(debug_assertions)]
        self.trace_roster_admission(class);
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
        #[cfg(debug_assertions)]
        self.trace_roster_publications(class);
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
        #[cfg(debug_assertions)]
        self.trace_roster_target(class, raw, identity);
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
        #[cfg(debug_assertions)]
        self.trace_roster_definition(raw, definition);
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
    /// Completed original root publication ledger. Partial fallback states,
    /// unknown mutations and observed execution do not supply final liveness.
    #[must_use]
    pub fn original_completed_command_world(&self) -> Option<OriginalCompletedCommandWorld> {
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_COMPLETED_WORLD root_state={}",
                self.original_completed_root_state.is_some()
            );
        }
        let state = self.original_completed_root_state.as_ref()?;
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
            state: Arc::clone(state),
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
                child_scope(&self.scope_for(&input)?, &input)
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

fn child_scope(
    parent: &SignatureNamespaceScope,
    input: &SignatureSourceNameInput,
) -> Option<SignatureNamespaceScope> {
    let recipe = input.policy().recipe();
    match recipe {
        NativeNameProtocol::C(_) => Some(SignatureNamespaceScope::C(
            recipe
                .namespace_address_path(parent.context()?, input.bytes())
                .ok()?,
        )),
        NativeNameProtocol::Jim084 => Some(SignatureNamespaceScope::Jim(
            recipe
                .jim_namespace_canonical_input(parent.context()?, input.bytes())
                .ok()?
                .selected()
                .into(),
        )),
    }
}

pub(super) fn publication_slot(
    scope: &SignatureNamespaceScope,
    input: &SignatureSourceNameInput,
    kind: CommandBindingDefinitionKind,
) -> Option<ByteCommandSlot> {
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
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
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
                    let mut publication = self.world.publications.remove(index);
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
        publication.definition = Some(definition);
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
        publications.next().is_none().then_some(selected)
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
                let mut retained = publication.clone();
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
    fn original_opaque_procedure_completion_and_body_keep_distinct_allocations() {
        // Implementation contract: naming.command.original-completed-cell-occupancy
        // docs/design/analysis/name-resolution-proofs/command-original-completed-cell-occupancy.md
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
        // docs/design/analysis/name-resolution-proofs/tcloo-original-source-instance-method-roster.md
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
        // docs/design/analysis/name-resolution-proofs/command-original-completed-cell-occupancy.md
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
        // docs/design/analysis/name-resolution-proofs/procedure-original-jim-static-list-capture-inputs.md
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
    fn original_world_baseline_is_selected_before_branching() {
        // Implementation contract: naming.command.original-world-baseline-bootstrap
        // docs/design/analysis/name-resolution-proofs/original-world-baseline-bootstrap.md
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
        // docs/design/analysis/name-resolution-proofs/alias-captured-prefix-allocation-contract.md
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
        // docs/design/analysis/name-resolution-proofs/namespace-original-qualified-forget-publications.md
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
        // docs/design/analysis/name-resolution-proofs/namespace-original-qualified-forget-publications.md
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
