// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original class configuration targets at their independently retained point.

use super::{
    BindingKind, CommandAllocationSite, MayBinding, ModuleCommandBindings, OriginalCommandLookup,
    SourceCommandBindings, SourceCommandDefinition, SourceCommandDefinitionKind,
    SourceCommandReferenceBinding,
};
use crate::signature_scan::{
    original_name::SourceOriginalNameOccurrence,
    scope::{SignatureSourceNameInput, SignatureSourceNameKey},
};

/// Target of one original class configuration. The original operand and
/// lookup point remain independent of the optional local class allocation.
/// A missing local definition is an external obligation, never a class grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassConfigurationTarget {
    site: CommandAllocationSite,
    written: usize,
    input: SignatureSourceNameInput,
    lookup: OriginalCommandLookup,
    local_definition: Option<SourceCommandDefinition>,
    local_allocation: Option<super::CommandAllocation>,
    external_candidate: bool,
    registry_identity: String,
    layer: tcl_registry::ObjectDispatchLayer,
}

impl OriginalSourceClassConfigurationTarget {
    /// Actual original configuration command and source instance.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Exact original written target ordinal selected by the Registry.
    #[must_use]
    pub const fn written_ordinal(&self) -> usize {
        self.written
    }
    /// Original class operand, independent of its reporting spelling.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Original caller lookup paths, with no external class existence grant.
    #[must_use]
    pub const fn lookup(&self) -> &OriginalCommandLookup {
        &self.lookup
    }
    /// Independently selected local class definition at this source point.
    /// Aliases and ordinary object instances cannot donate this category.
    #[must_use]
    pub const fn local_definition(&self) -> Option<&SourceCommandDefinition> {
        self.local_definition.as_ref()
    }
    /// Exact allocation occupying the operand's own slot. This independently
    /// joins original class metadata; the allocation alone proves no class role.
    #[must_use]
    pub const fn local_allocation(&self) -> Option<&super::CommandAllocation> {
        self.local_allocation.as_ref()
    }
    /// Every original path lacked a local occupied command. A workspace may
    /// independently settle this obligation; a builtin cannot become external.
    #[must_use]
    pub const fn is_external_candidate(&self) -> bool {
        self.external_candidate
    }
    pub(crate) fn registry_identity(&self) -> &str {
        &self.registry_identity
    }
    /// Registry-selected class-wide or own-object configuration axis.
    #[must_use]
    pub const fn layer(&self) -> tcl_registry::ObjectDispatchLayer {
        self.layer
    }
}

/// Independently selected actual object operand of one own-object definition.
/// The bounded allocation is independent of a printable object name; a readonly
/// producer is retained only when the original value graph actually supplied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceObjectConfigurationTarget {
    site: CommandAllocationSite,
    written: usize,
    input: Option<SignatureSourceNameInput>,
    instance: super::SourceObjectInstanceProof,
    naming_scope: Option<crate::signature_scan::scope::SignatureNamespaceScope>,
    registry_identity: String,
}

impl OriginalSourceObjectConfigurationTarget {
    /// Genuine configuration command, independently of the manufacture site.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Original written operand ordinal, never an effective alias ordinal.
    #[must_use]
    pub const fn written_ordinal(&self) -> usize {
        self.written
    }
    /// Independently retained original value, if its byte producer was known.
    /// An unnamed allocation does not fabricate a name or an editable word.
    #[must_use]
    pub const fn name_input(&self) -> Option<&SignatureSourceNameInput> {
        self.input.as_ref()
    }
    /// Actual instance proof current at this operand's post-argument point.
    /// Later object changes must independently retain their own currency.
    #[must_use]
    pub const fn instance(&self) -> &super::SourceObjectInstanceProof {
        &self.instance
    }
    /// Original invoking namespace geometry, independent of the object name.
    /// This selects source relation operands, never the object's private frame.
    #[must_use]
    pub const fn original_naming_scope(
        &self,
    ) -> Option<&crate::signature_scan::scope::SignatureNamespaceScope> {
        self.naming_scope.as_ref()
    }
    pub(crate) fn registry_identity(&self) -> &str {
        &self.registry_identity
    }
}

struct OriginalConfigurationInvocation {
    binding: super::SourceInvocationBinding,
    ordinal: usize,
    registry_identity: String,
    layer: tcl_registry::ObjectDispatchLayer,
}

impl SourceCommandBindings {
    /// Retain a static class operand from the authentic full configuration
    /// vector. The Registry selects the operation and ordinal; every original
    /// reaching row must agree and its operand effects must preserve lookup.
    #[must_use]
    pub fn original_class_configuration_target(
        &self,
        original: &SourceOriginalNameOccurrence,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalSourceClassConfigurationTarget> {
        let target = self.original_class_configuration_target_at_span(
            original.name_input().span(),
            config,
            registry,
        )?;
        (target.site() == original.site()
            && target.name_input().original_word_key()? == original.name_input())
        .then_some(target)
    }

    /// Independently retained original target value at its authentic written
    /// operand. Readonly variables keep their value lineage; they acquire no
    /// editable word or class category from that lineage alone.
    #[must_use]
    pub fn original_class_configuration_target_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalSourceClassConfigurationTarget> {
        let operation = self.original_configuration_at_span(span, config, registry)?;
        let binding = &operation.binding;
        let site = binding.invocation_site()?;
        let input = binding.original_retained_written_name_input(operation.ordinal)?;
        if !input.is_current(&binding.variable_context) {
            return None;
        }
        let rows = super::declaration_layout::original_declaration_layouts(
            binding.declaration_layout_observations.as_deref()?,
        )?;
        let mut local = None;
        for row in rows.clone() {
            let selected = row.snapshot.state.original_local_class_for_operand(
                &input,
                &row.namespace,
                site,
            )?;
            if local.as_ref().is_some_and(|previous| previous != &selected) {
                return None;
            }
            local = Some(selected);
        }
        let lookup = binding.original_lookup_from_rows(&input, rows.collect())?;
        let (local_definition, local_allocation, external_candidate) = local?;
        Some(OriginalSourceClassConfigurationTarget {
            site: site.clone(),
            written: operation.ordinal,
            input,
            lookup,
            local_definition,
            local_allocation,
            external_candidate,
            registry_identity: operation.registry_identity,
            layer: operation.layer,
        })
    }

    /// Select the own-object operand of the genuine Registry configuration.
    /// A variable operand must retain its actual reached instance read through
    /// argv; a named operand must select the actual instance token. Class
    /// commands, aliases and equal reporting names cannot donate that receipt.
    #[must_use]
    pub fn original_object_configuration_target(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalSourceObjectConfigurationTarget> {
        let operation = self.original_configuration_at_span(span, config, registry)?;
        if operation.layer != tcl_registry::ObjectDispatchLayer::Object {
            return None;
        }
        let binding = &operation.binding;
        let (_, tokens) = binding.original_recorded_command()?;
        let word = tokens.words().get(operation.ordinal)?;
        let input = binding.original_retained_written_name_input(operation.ordinal);
        let state = &binding.lookup_state.as_ref()?.state;
        if binding.entered_execution_observer.observed()
            || state.has_opaque_domain()
            || state.source_step_observed()
            || state.source_execution_observed(None)
        {
            return None;
        }
        let instance = if let Some((_, source)) = word.sole_variable_substitution() {
            let accesses =
                self.variable_accesses_for_invocation_args(binding.invocation_site()?.offset);
            let mut instances = accesses
                .iter()
                .filter(|access| {
                    &access.source == source && binding.retains_object_instance_at_dispatch(access)
                })
                .filter_map(super::SourceVariableAccess::proved_object_instance);
            let first = instances.next()?.clone();
            if !instances.all(|instance| instance == &first) {
                return None;
            }
            first
        } else {
            state
                .original_named_instance_for_operand(
                    input.as_ref()?,
                    &binding.lookup_namespace_key,
                )?
                .clone()
        };
        if !state.receiver_allocation_is_current(&instance) {
            return None;
        }
        let head = binding.original_head_name_input(&tokens)?;
        let naming_scope = binding
            .original_command_lookup(&tokens, &head)?
            .original_naming_scope()
            .cloned();
        Some(OriginalSourceObjectConfigurationTarget {
            site: binding.invocation_site()?.clone(),
            written: operation.ordinal,
            input,
            instance,
            naming_scope,
            registry_identity: operation.registry_identity,
        })
    }

    fn original_configuration_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<OriginalConfigurationInvocation> {
        if self.lexer_config != Some(config) {
            return None;
        }
        let origin = self.root_origin.as_ref()?;
        let mut found = None;
        for (offset, _) in self.dispatch_points.range(..=span.start()).rev() {
            let binding = self.invocation_at_source("", *offset);
            let Some((_, tokens)) = binding.original_recorded_command() else {
                continue;
            };
            let site = binding.invocation_site()?;
            if &site.source != origin {
                continue;
            }
            let native = crate::registry_invocation::original_native_compiler_words(
                origin.source_image(),
                tokens.words(),
                *offset,
                config,
            )?;
            let mut ordinals = tokens.words().iter().zip(&native).enumerate().filter_map(
                |(index, (word, native))| {
                    (word.source().span == span
                        || native.span() == span
                        || native
                            .tokens()
                            .first()
                            .is_some_and(|token| token.span == span))
                    .then_some(index)
                },
            );
            let Some(ordinal) = ordinals.next() else {
                continue;
            };
            if ordinals.next().is_some() {
                return None;
            }
            let (registry_identity, selected, layer) =
                binding.original_configuration_operation(&tokens, config, registry)?;
            if ordinal != selected {
                return None;
            }
            if found.is_some() {
                return None;
            }
            found = Some(OriginalConfigurationInvocation {
                binding,
                ordinal,
                registry_identity,
                layer,
            });
        }
        found
    }
}

fn original_configuration_dispatch_operand(
    selected: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<(usize, tcl_registry::ObjectDispatchLayer)> {
    let transitions = selected.state_transitions();
    let mut operands = transitions
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::ObjectDispatch(
                tcl_registry::ObjectDispatchTransition::Configure { target, layer },
            ) => target.argument_index().map(|ordinal| (ordinal, *layer)),
            _ => None,
        });
    let (ordinal, layer) = operands.next()?;
    if operands.next().is_some() {
        return None;
    }
    Some((ordinal, layer))
}

impl super::SourceInvocationBinding {
    fn original_configuration_operation(
        &self,
        tokens: &crate::ir::CommandTokens,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(String, usize, tcl_registry::ObjectDispatchLayer)> {
        let site = self.invocation_site()?;
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let mut unanimous = None;
        for row in rows {
            if row.config != config
                || !row.entry.owns_source(&site.source, site.offset)
                || row.snapshot.state.has_opaque_domain()
                || row.snapshot.state.source_step_observed()
                || !self.original_operands_preserve_lookup(tokens, registry, row, 0)
            {
                return None;
            }
            let dialect = row.snapshot.state.source_variables.invocation_dialect?;
            let native = crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                &row.words,
                site.offset,
                config,
            )?;
            let policy = row
                .snapshot
                .state
                .baseline
                .execution_name_policy?
                .native_recipe()?;
            let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
            let head = SignatureSourceNameInput::OriginalWord(
                SignatureSourceNameKey::from_original_native_word(native.first()?, rules, policy)?,
            );
            let selection = row.snapshot.state.original_targets_for_input(
                &head,
                &row.namespace,
                super::CommandTargetLookup::NamedSlots,
            )?;
            if selection.unknown || selection.may_be_absent || selection.targets.len() != 1 {
                return None;
            }
            let target = selection.targets.first()?;
            if !target.registry_backed || !target.terminal || !target.prepended.is_empty() {
                return None;
            }
            let values = if let Some(frozen) = self.frozen_written_words() {
                if frozen.len() != native.len()
                    || frozen.iter().any(|word| {
                        word.expansion_len().is_some()
                            || matches!(
                                word,
                                crate::registry_invocation::EffectiveInvocationWord::Expanded
                            )
                    })
                {
                    return None;
                }
                frozen.to_vec()
            } else {
                native.iter().map(|word| SignatureSourceNameKey::from_original_native_word(word, rules, policy)
                    .map_or(crate::registry_invocation::EffectiveInvocationWord::Dynamic,
                        |key| crate::registry_invocation::EffectiveInvocationWord::ByteLiteral(key.bytes().into())))
                    .collect::<Vec<_>>()
            };
            let arguments = values
                .get(1..)?
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect::<Vec<_>>();
            let invocation = registry.resolve_structured_invocation(
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&target.command),
                    &arguments,
                )
                .with_dialect(dialect),
                dialect.authoring_query(),
            );
            let resolved = invocation.resolved()?;
            let (ordinal, layer) = original_configuration_dispatch_operand(&resolved)?;
            let selected = (target.command.clone(), ordinal.checked_add(1)?, layer);
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }
}

impl ModuleCommandBindings {
    pub(super) fn original_named_instance_for_operand(
        &self,
        input: &SignatureSourceNameInput,
        current: &super::SourceNamespaceKey,
    ) -> Option<&super::SourceObjectInstanceProof> {
        let paths = self.original_command_paths_for_input(current, input)?;
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings.is_empty() {
                    return None;
                }
                if bindings
                    .iter()
                    .all(|binding| matches!(binding, MayBinding::Missing))
                {
                    continue;
                }
                let mut bindings = bindings.iter();
                let MayBinding::Target(target) = bindings.next()? else {
                    return None;
                };
                if bindings.next().is_some() {
                    return None;
                }
                if target.kind != BindingKind::Command
                    || target.registry_backed
                    || !target.prepended.is_empty()
                {
                    return None;
                }
                let token = target.token.as_ref()?;
                let proof = self.object_instances.named.get(token)?;
                let allocation = token.allocation.as_ref()?;
                if allocation.site != proof.allocation().site
                    || allocation.incarnation != proof.allocation().incarnation
                    || !self.receiver_allocation_is_current(proof)
                {
                    return None;
                }
                selected = Some(proof.as_ref());
                break;
            }
            let selected = selected?;
            if unanimous.is_some_and(|previous| previous != selected) {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }

    fn original_local_class_for_operand(
        &self,
        input: &SignatureSourceNameInput,
        current: &super::SourceNamespaceKey,
        site: &CommandAllocationSite,
    ) -> Option<(
        Option<SourceCommandDefinition>,
        Option<super::CommandAllocation>,
        bool,
    )> {
        let paths = self.original_command_paths_for_input(current, input)?;
        let mut unanimous = None;
        for path in paths {
            let mut selected = None;
            for key in path {
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings.is_empty() {
                    return None;
                }
                if bindings
                    .iter()
                    .all(|binding| matches!(binding, MayBinding::Missing))
                {
                    continue;
                }
                let reference =
                    self.reference_for_command_key(&key, &bindings, Some(site.clone()), None)?;
                let kind = match reference.binding() {
                    SourceCommandReferenceBinding::Direct { kind, .. }
                    | SourceCommandReferenceBinding::Imported { kind, .. } => *kind,
                };
                if !matches!(
                    kind,
                    BindingKind::Class | BindingKind::Builtin | BindingKind::Command
                ) {
                    return None;
                }
                let definition = reference.definition().cloned();
                if definition.as_ref().is_some_and(|definition| {
                    definition.kind() != SourceCommandDefinitionKind::Class
                }) {
                    return None;
                }
                selected = Some((
                    definition,
                    reference.called_implementation_allocation().cloned(),
                    false,
                ));
                break;
            }
            let selected = selected.unwrap_or((None, None, true));
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }
}
