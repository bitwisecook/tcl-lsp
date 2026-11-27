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

//! The completed command world of a document: declarations, class rosters and relations.

use super::super::{Arc, ModuleCommandBindings, SourceCommandBindings, SourceCommandDefinition};
use super::{
    OriginalClassInstanceMethodRoster, OriginalCommandPublication, OriginalCommandPublicationKind,
};
use crate::command_binding::SourceOriginLexical;
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_core_types::ByteCommandSlot;
use tcl_syntax::naming::NamePolicyProtocol;

/// Closed modeled root world. An abrupt or merely partial source prefix can
/// never issue this receipt; it grants no physical interpreter execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalCompletedCommandWorld {
    pub(super) source: Arc<tcl_names::origin::SourceOriginId>,
    config: tcl_lexer::LexerConfig,
    pub(super) state: Arc<ModuleCommandBindings>,
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
    pub const fn class_target(&self) -> &super::super::SourceCommandTarget {
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
    ) -> Option<&super::super::SourceReceiverMethodEntry> {
        self.source_entry_for_input(super::super::SourceMethodReceiver::Instance, input)
    }

    /// Exact current source declaration occupying this class-object name: a
    /// `self` method or a `classmethod`, which tclsh 9.0 installs on the class.
    #[must_use]
    pub fn source_class_object_method_for_input(
        &self,
        input: &SignatureSourceNameInput,
    ) -> Option<&super::super::SourceReceiverMethodEntry> {
        self.source_entry_for_input(super::super::SourceMethodReceiver::Class, input)
    }

    fn source_entry_for_input(
        &self,
        receiver: super::super::SourceMethodReceiver,
        input: &SignatureSourceNameInput,
    ) -> Option<&super::super::SourceReceiverMethodEntry> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        if input.policy() != self.policy {
            return None;
        }
        super::super::SourceReceiverMethodEntry::for_original_input(&self.entries, receiver, input)
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

    /// Whether a proposed name is already a method of the class object itself:
    /// a class-object builtin of the selected factory (`new`, `create`, …) or
    /// a class-side source entry. tclsh 9.0: a `classmethod new` replaces the
    /// class's own `new`, so such a name is a collision, not a fresh slot.
    #[must_use]
    pub fn class_object_contains(&self, name: &[u8]) -> bool {
        let Ok(input) = self.policy.recipe().oo_method_input(name) else {
            return true;
        };
        let selected = input.selected();
        self.factory
            .grammar()
            .builtin_object_methods
            .iter()
            .any(|method| method.name.as_bytes() == selected)
            || self.entries.contains_key(&(
                super::super::SourceMethodReceiver::Class,
                tcl_core_types::NameBytes::from(selected),
            ))
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
    /// The command an exact original lookup selects in the completed world.
    pub(crate) fn reference_for_lookup(
        &self,
        lookup: &super::super::OriginalCommandLookup,
    ) -> Option<super::super::SourceCommandReference> {
        self.state.original_reference_for_lookup(lookup, None)
    }

    /// Same authenticated completed state for a private conditional syntax view.
    /// Callers retain the cut/declaration owner; this grants no body execution.
    pub(in super::super) fn source_cut_body_state(&self) -> &Arc<ModuleCommandBindings> {
        &self.state
    }

    fn class_relation_allocations(
        &self,
        class: &SourceCommandDefinition,
    ) -> Option<(
        tcl_names::command::CommandAllocation,
        Vec<tcl_names::command::CommandAllocation>,
    )> {
        let mut actual = self.state.bindings.values().filter_map(|targets| {
            if targets.len() != 1 {
                return None;
            }
            let super::super::MayBinding::Target(target) = targets.first()? else {
                return None;
            };
            let definition = self.state.source_definition_for_implementation(target)?;
            (&definition == class).then_some(target)
        });
        let target = actual.next()?;
        if actual.any(|other| other != target)
            || target.kind != tcl_names::command::BindingKind::Class
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
                    let super::super::MayBinding::Target(raw) = targets.first()? else {
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
                (class.kind() == super::super::SourceCommandDefinitionKind::Class)
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
        if bindings == std::collections::BTreeSet::from([super::super::MayBinding::Missing]) {
            return Some(false);
        }
        bindings
            .iter()
            .all(|binding| {
                matches!(
                    binding,
                    super::super::MayBinding::Target(_) | super::super::MayBinding::Imported(_)
                )
            })
            .then_some(true)
    }

    #[cfg(debug_assertions)]
    fn trace_roster_admission(&self, class: &SourceCommandDefinition) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
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
        raw: &super::super::ResolvedCommandTarget,
        identity: &tcl_names::command::CommandIdentity,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
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
        raw: &super::super::ResolvedCommandTarget,
        definition: &super::super::ClassDefinitionReceipt,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
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
        if class.kind() != super::super::SourceCommandDefinitionKind::Class
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
        let [super::super::MayBinding::Target(raw)] = targets.as_slice() else {
            return None;
        };
        let identity = raw.token.as_ref()?;
        #[cfg(debug_assertions)]
        self.trace_roster_target(class, raw, identity);
        if raw.kind != tcl_names::command::BindingKind::Class
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
        let target = super::super::SourceCommandTarget {
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
        let matches = |declaration: &tcl_names::command::CommandAllocationSite,
                       body: &tcl_names::origin::ExecutedScriptSource,
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
                        entry.receiver() == super::super::SourceMethodReceiver::Instance
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
                    definition.kind() == super::super::SourceCommandDefinitionKind::Class
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
        lookup: &super::super::OriginalCommandLookup,
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
    ) -> Option<(
        tcl_names::command::CommandAllocation,
        Vec<tcl_names::command::CommandAllocation>,
    )> {
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
                (definition.kind() == super::super::SourceCommandDefinitionKind::Class
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

    /// Whether the root's final modelled command world holds an unknown
    /// command-binding mutation. Possible errors, unknown commands reaching a
    /// handler and closed effects are not mutations of the command table.
    #[must_use]
    pub(crate) fn final_world_has_unknown_command_mutation(&self) -> bool {
        self.final_state.opaque_binding_mutation
    }

    /// Completed original root publication ledger. Partial fallback states,
    /// unknown mutations and observed execution do not supply final liveness.
    #[must_use]
    pub fn original_completed_command_world(&self) -> Option<OriginalCompletedCommandWorld> {
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some() {
            eprintln!(
                "ORIGINAL_COMPLETED_WORLD root_state={}",
                self.original_completed_root_state.is_some()
            );
        }
        let state = trace_completed_world_option(
            "ledger.root-state",
            self.original_completed_root_state.as_ref(),
        )?;
        self.completed_original_world_from_normal_state(Arc::clone(state))
    }

    /// The caller must independently own a completed whole-root Normal route.
    pub(in super::super) fn completed_original_world_from_normal_state(
        &self,
        state: Arc<ModuleCommandBindings>,
    ) -> Option<OriginalCompletedCommandWorld> {
        if trace_completed_world_flag("ledger.unknown-entry", state.baseline.unknown_entry)
            || trace_completed_world_flag(
                "ledger.unavailable",
                state.original_command_world.unavailable,
            )
            || trace_completed_world_flag("ledger.source-step", state.source_step_observed())
            || trace_completed_world_flag(
                "ledger.source-execution",
                state.source_execution_observed(None),
            )
        {
            return None;
        }
        Some(OriginalCompletedCommandWorld {
            source: trace_completed_world_option("ledger.root-origin", self.root_origin.clone())?,
            config: trace_completed_world_option("ledger.config", self.lexer_config)?,
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

// Temporary WORK-only owner diagnostic; no source or execution permission.
#[inline]
pub(in super::super) fn trace_completed_world_option<T>(
    stage: &str,
    value: Option<T>,
) -> Option<T> {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_COMPLETED_COMMAND_WORLD").is_some() {
        eprintln!(
            "COMPLETED_COMMAND_WORLD stage={stage} present={}",
            value.is_some()
        );
    }
    value
}

#[inline]
pub(in super::super) fn trace_completed_world_flag(stage: &str, value: bool) -> bool {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_COMPLETED_COMMAND_WORLD").is_some() {
        eprintln!("COMPLETED_COMMAND_WORLD stage={stage} value={value}");
    }
    value
}
