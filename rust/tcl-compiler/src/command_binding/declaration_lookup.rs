// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional original call identity after independently described operands.
//!
//! A pre-operand lookup is insufficient when a substitution can replace its
//! target. This projection retains the original declaration frame and complete
//! grammar, and separately checks the operand effect boundary. It issues no
//! executed argument, normal completion, compiler preparation or native header.

use super::declaration_layout::{DeclarationLayoutObservation, original_declaration_layouts};
use super::{SourceCommandDefinition, SourceInvocationBinding};
use crate::ir::CommandTokens;
use tcl_lexer::{ExecutablePart, NativeWord};
use tcl_registry::CommandRegistry;

#[derive(Clone, Debug, PartialEq, Eq)]
struct OriginalNameSelection {
    definition: Option<SourceCommandDefinition>,
    captured_arguments: bool,
}

impl super::ModuleCommandBindings {
    /// Select a source publication only from its original counted name and
    /// current modeled namespace table. Geometry and publication chronology
    /// are retained separately; no display or native entry is reconstructed.
    fn original_name_lookup_paths(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &super::SourceNamespaceKey,
    ) -> Option<Vec<Vec<tcl_core_types::ByteCommandSlot>>> {
        use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol, NativeNameQualification};
        let policy = input.policy();
        let recipe = policy.recipe();
        let scope = self.original_command_world.scope(namespace, policy)?;
        let mut paths = Vec::new();
        match recipe {
            NativeNameProtocol::Jim084 => {
                paths.push(
                    recipe
                        .jim_command_lookup_keys(scope.context()?, input.bytes())
                        .ok()?
                        .into_iter()
                        .map(|simple| tcl_core_types::ByteCommandSlot {
                            namespace: tcl_core_types::ByteNamespacePath::root(),
                            simple,
                        })
                        .collect::<Vec<_>>(),
                );
            }
            NativeNameProtocol::C(version) => {
                let absolute = recipe
                    .command_lookup_input(scope.context()?, input.bytes())
                    .ok()?
                    .qualification()
                    == NativeNameQualification::Absolute;
                if !absolute && self.unknown_lookup_namespaces.contains(namespace) {
                    return None;
                }
                if !absolute
                    && version.has_namespace_path()
                    && self.unknown_namespace_paths.contains(namespace)
                {
                    return None;
                }
                let empty = std::collections::BTreeSet::from([Vec::new()]);
                for path in self.namespace_paths.get(namespace).unwrap_or(&empty) {
                    let mut candidates = Vec::new();
                    if !absolute {
                        candidates.push(
                            recipe
                                .command_lookup_slot(scope.context()?, input.bytes())
                                .ok()?,
                        );
                        if version.has_namespace_path() {
                            for base in path {
                                if !self.namespaces.contains(base) {
                                    continue;
                                }
                                let selected = self.original_command_world.scope(base, policy)?;
                                let slot = recipe
                                    .command_lookup_slot(selected.context()?, input.bytes())
                                    .ok()?;
                                if !candidates.contains(&slot) {
                                    candidates.push(slot);
                                }
                            }
                        }
                    }
                    let root = recipe
                        .command_lookup_slot(NativeNameContext::root(), input.bytes())
                        .ok()?;
                    if !candidates.contains(&root) {
                        candidates.push(root);
                    }
                    paths.push(candidates);
                }
            }
        }
        Some(paths)
    }

    fn original_name_selection(
        &self,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        namespace: &super::SourceNamespaceKey,
        visiting: &mut std::collections::HashSet<tcl_core_types::ByteCommandSlot>,
    ) -> Option<OriginalNameSelection> {
        let policy = input.policy();
        let paths = self.original_name_lookup_paths(input, namespace)?;
        let mut unanimous = None;
        for path in paths {
            let mut publication = None;
            for slot in &path {
                let key = self.original_command_key_for_slot(slot, policy)?;
                let bindings = self.original_bindings_for_key(&key)?;
                if bindings.is_empty() {
                    return None;
                }
                if bindings
                    .iter()
                    .all(|binding| matches!(binding, super::MayBinding::Missing))
                {
                    continue;
                }
                if bindings.iter().any(|binding| {
                    matches!(
                        binding,
                        super::MayBinding::Missing | super::MayBinding::Unknown
                    )
                }) {
                    return None;
                }
                // An occupied earlier native/baseline slot is not an absent
                // source publication. It stops lookup before a later source
                // procedure can be selected.
                publication = Some(self.original_publication_at(slot, policy)?);
                break;
            }
            let publication = publication?;
            if !visiting.insert(publication.slot().clone()) {
                return None;
            }
            let selected = if publication.kind() == super::OriginalCommandPublicationKind::Alias {
                let alias = publication.alias_target()?;
                let target_namespace = match alias.lookup() {
                    tcl_registry::AliasTargetLookup::CallerNamespace => namespace.clone(),
                    tcl_registry::AliasTargetLookup::Global => self.source_root_namespace_key()?,
                };
                let mut selected =
                    self.original_name_selection(alias.name_input(), &target_namespace, visiting)?;
                selected.captured_arguments |= !alias.arguments().is_empty();
                selected
            } else {
                OriginalNameSelection {
                    definition: publication.definition().cloned(),
                    captured_arguments: false,
                }
            };
            visiting.remove(publication.slot());
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

impl SourceInvocationBinding {
    /// Conditional transfer of every fixed scalar formal object into the
    /// original recursive argv. This proves retention across frame removal,
    /// without classifying an unknown object's representation or release.
    /// A complete body inventory must independently exclude additional local
    /// cells, observers, callbacks and frame introspection.
    pub(crate) fn original_call_retains_formal_roots(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> bool {
        if !self.original_declared_call_keeps_written_arguments(tokens) {
            return false;
        }
        let Some(rows) = self
            .declaration_layout_observations
            .as_deref()
            .and_then(original_declaration_layouts)
        else {
            return false;
        };
        rows.clone().all(|row| {
            let state = &row.snapshot.state;
            let context = &state.source_variables;
            if state.has_opaque_domain()
                || state.source_step_observed()
                || !state.command_observers.registrations.is_empty()
                || context.dynamic_bindings
                || !context.alias_bindings.is_empty()
                || !context.name_alias_bindings.is_empty()
                || !context.upvar_aliases.is_empty()
                || !context.traced.is_empty()
                || !context.untracked_traces.is_empty()
                || !context.trace_registrations.is_empty()
                || !context.possible_trace_registrations.is_empty()
                || !row.entry.owns_original_context(context)
                || !self.original_operands_preserve_lookup(tokens, registry, row, 0)
            {
                return false;
            }
            let Some(topology) = row.entry.original_formal_topology() else {
                return false;
            };
            let Some(names) =
                topology.fixed_scalar_binding_names(tokens.words().len().saturating_sub(1))
            else {
                return false;
            };
            if names.len() != topology.parameters().len()
                || names.iter().collect::<std::collections::HashSet<_>>().len() != names.len()
            {
                return false;
            }
            let Some(site) = self.invocation_site() else {
                return false;
            };
            let Some(words) = crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                row.config,
            ) else {
                return false;
            };
            let protocol = topology.source_string_protocol();
            words.iter().skip(1).zip(names).all(|(word, name)| {
                original_formal_word_retains_root(
                    word, &name, context, protocol, topology, registry,
                )
            })
        })
    }

    /// Conditional before-store effects for explicitly proposed formal
    /// assignments. Fixed binding geometry does not prove that an old value
    /// can be released, nor that its variable observers are quiet.
    pub(crate) fn original_proposed_formal_writes(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        names: &[tcl_core_types::NameBytes],
    ) -> bool {
        let Some(rows) = self
            .declaration_layout_observations
            .as_deref()
            .and_then(original_declaration_layouts)
        else {
            return false;
        };
        rows.clone().all(|row| {
            if !self.original_operands_preserve_lookup(tokens, registry, row, 0) {
                return false;
            }
            let context = &row.snapshot.state.source_variables;
            names.iter().all(|name| {
                let receiver = crate::var_resolve::resolve_evaluated_variable_input(
                    tcl_syntax::naming::NativeVariableInputForm::Separate {
                        root: name.as_bytes(),
                        element: None,
                    },
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Write,
                );
                row.entry
                    .original_formal_topology()
                    .is_some_and(|topology| topology.fresh_scalar_formal_name(&receiver, context))
                    && context
                        .original_normal_value_write(&receiver, registry)
                        .is_some_and(|write| write.is_current(context, registry))
            })
        })
    }

    /// Check explicitly proposed Registry command spellings at this original
    /// conditional point. These authored metadata names are not original word
    /// producers and cannot supply source coordinates or native preparation.
    /// This conditional lookup check does not authorise command insertion,
    /// frame relocation, Normal completion or removal of argument evaluation.
    #[must_use]
    pub fn original_proposed_registry_commands(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        names: &[&str],
    ) -> bool {
        let Some(site) = self.invocation_site() else {
            return false;
        };
        let Some(rows) = self
            .declaration_layout_observations
            .as_deref()
            .and_then(original_declaration_layouts)
        else {
            return false;
        };
        if self
            .declaration_flow_report(registry)
            .and_then(|report| report.invocation_may_be_reached(site))
            != Some(true)
        {
            return false;
        }
        rows.clone().all(|row| {
            let state = &row.snapshot.state;
            !state.source_step_observed()
                && !state.has_opaque_domain()
                && self.original_operands_preserve_lookup(tokens, registry, row, 0)
                && names.iter().all(|name| {
                    let Some(expected) = registry.get(name) else {
                        return false;
                    };
                    let Some(policy) = state
                        .source_variables
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    else {
                        return false;
                    };
                    let Some(paths) =
                        state.original_registry_command_paths(&row.namespace, name, policy)
                    else {
                        return false;
                    };
                    let matches = |target: &super::ResolvedCommandTarget| {
                        target.terminal
                            && target.registry_backed
                            && target.kind == super::BindingKind::Builtin
                            && target.prepended.is_empty()
                            && registry
                                .get(&target.command)
                                .is_some_and(|actual| std::ptr::eq(actual, expected))
                            && !state.runtime_execution_observed(target.token.as_ref())
                            && !state.source_execution_observed(target.token.as_ref())
                    };
                    !paths.is_empty()
                        && paths.iter().all(|path| {
                            let mut may_be_absent = true;
                            for key in path {
                                let Some(bindings) = state.original_bindings_for_key(key) else {
                                    return false;
                                };
                                if bindings.is_empty() {
                                    return false;
                                }
                                for binding in &bindings {
                                    match binding {
                                        super::MayBinding::Missing => {}
                                        super::MayBinding::Target(target) if matches(target) => {}
                                        super::MayBinding::Imported(imported) => {
                                            let Some(targets) = state.objects.get(&imported.origin)
                                            else {
                                                return false;
                                            };
                                            if targets.is_empty()
                                                || targets.iter().any(|target| {
                                                    !matches!(target,
                                            super::MayBinding::Target(target) if matches(target))
                                                })
                                            {
                                                return false;
                                            }
                                        }
                                        _ => return false,
                                    }
                                }
                                if !bindings.contains(&super::MayBinding::Missing) {
                                    may_be_absent = false;
                                    break;
                                }
                            }
                            !may_be_absent
                        })
                })
        })
    }

    /// Whether the authentic selected callee receives exactly the written
    /// post-head words. Captured alias prefixes and expansion remain distinct
    /// even when the terminal procedure allocation is known.
    pub(crate) fn original_declared_call_keeps_written_arguments(
        &self,
        tokens: &CommandTokens,
    ) -> bool {
        let Some(input) = self.original_head_name_input(tokens) else {
            return false;
        };
        let Some(observations) = self.declaration_layout_observations.as_deref() else {
            return false;
        };
        let Some(rows) = original_declaration_layouts(observations) else {
            return false;
        };
        rows.clone().all(|row| {
            row.snapshot
                .state
                .original_name_selection(
                    &input,
                    &row.namespace,
                    &mut std::collections::HashSet::default(),
                )
                .is_some_and(|selected| !selected.captured_arguments)
        }) && tokens
            .words()
            .iter()
            .all(|word| !matches!(word, crate::ir::WordExpr::Expand { .. }))
    }

    /// The original declaration selected at this conditional call point.
    /// Missing or disagreeing targets and operand callbacks remain terminal.
    pub(crate) fn original_declared_definition_reference(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<SourceCommandDefinition> {
        if let Some(reference) = self.evaluated_command_reference() {
            return reference
                .linked_definition()
                .or(reference.definition())
                .cloned();
        }
        let input = self.original_head_name_input(tokens)?;
        let observations =
            original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut definition = None;
        let site = self.invocation_site()?;
        for observation in observations {
            let current = observation
                .snapshot
                .state
                .original_name_selection(
                    &input,
                    &observation.namespace,
                    &mut std::collections::HashSet::default(),
                )?
                .definition?;
            if definition
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            definition = Some(current);
            if observation.snapshot.state.has_opaque_domain()
                || observation.snapshot.state.source_step_observed()
                || !self.original_operands_preserve_lookup(tokens, registry, observation, 0)
            {
                return None;
            }
            // An unknown preceding handler can register observers or replace
            // the callee even when this command's own lexical layout survives.
            if self
                .declaration_flow_report(registry)?
                .invocation_may_be_reached(site)
                != Some(true)
            {
                return None;
            }
        }
        definition
    }

    /// Conditional target of a static command in an original produced
    /// expression. Parent bytes and full grammar own the parsed body; generated
    /// word offsets remain readonly and cannot donate editable source sites.
    pub(crate) fn original_expression_command_definition(
        &self,
        tokens: &CommandTokens,
        written: usize,
        body: tcl_lexer::Span,
        command: usize,
        registry: &CommandRegistry,
    ) -> Option<SourceCommandDefinition> {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameValue};
        let site = self.invocation_site()?;
        let rows = original_declaration_layouts(self.declaration_layout_observations.as_deref()?)?;
        let mut unanimous = None;
        for row in rows {
            if !row.entry.owns_source(&site.source, site.offset)
                || row.snapshot.state.source_step_observed()
                || row.snapshot.state.has_opaque_domain()
                || self
                    .declaration_flow_report(registry)?
                    .invocation_may_be_reached(site)
                    != Some(true)
            {
                return None;
            }
            let originals = crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                row.config,
            )?;
            let original = originals.get(written)?;
            let policy = row
                .snapshot
                .state
                .source_variables
                .execution_name_policy?
                .native_recipe()?;
            let parent = SignatureSourceNameValue::from_original_static_word(
                original,
                tcl_syntax::word_rules::WordValueRules::from_config(&row.config),
                policy,
            )?;
            let source = parent.bytes().get(body.as_range())?;
            let image = tcl_lexer::SourceImage::native(source);
            let plan = tcl_lexer::native_script_words_in(
                image,
                tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
                row.config,
            )
            .ok()?;
            let selected = plan.commands.get(command)?;
            // Earlier generated commands require independent execution effects;
            // an immutable source value cannot manufacture that chronology.
            if command != 0 || plan.commands.len() != 1 {
                return None;
            }
            let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                &selected.words,
                policy.string_protocol(),
            )
            .ok()?;
            if selected
                .words
                .iter()
                .enumerate()
                .any(|(index, word)| word.group().expand || captured.literal(index).is_none())
            {
                return None;
            }
            let head = SignatureSourceNameInput::OriginalValue(
                parent.script_word(body, selected.words.first()?)?,
            );
            let definition = row
                .snapshot
                .state
                .original_name_selection(
                    &head,
                    &row.namespace,
                    &mut std::collections::HashSet::default(),
                )?
                .definition?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &definition)
            {
                return None;
            }
            unanimous = Some(definition);
        }
        unanimous
    }

    /// Original source roles remain applicable only when argument evaluation
    /// preserves the complete selected lookup and its observer boundary. This
    /// source-purpose check supplies no executed argv, Normal completion or
    /// physical compiler preparation.
    pub(crate) fn original_source_operands_preserve_lookup(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> bool {
        if tokens.source_binding.as_ref() != Some(self) {
            return false;
        }
        let Some(site) = self.invocation_site() else {
            return false;
        };
        let Some(advice) = self.declaration_operand_layout_advice(tokens) else {
            return false;
        };
        if !advice.closed_lookup() || advice.targets().is_empty() {
            return false;
        }
        let Some(rows) = self
            .declaration_layout_observations
            .as_deref()
            .and_then(original_declaration_layouts)
        else {
            return false;
        };
        let registry_key = registry.snapshot().semantic_key();
        rows.clone().all(|row| {
            let state = &row.snapshot.state;
            row.entry.owns_source(&site.source, site.offset)
                && row.words.as_ref() == tokens.words()
                && state.baseline.registry_snapshot.as_ref() == Some(&registry_key)
                && !state.has_opaque_domain()
                && !state.source_step_observed()
                && advice.targets().iter().all(|target| {
                    target.registry_backed
                        && !state.source_execution_observed(target.identity.as_ref())
                        && !state.runtime_execution_observed(target.identity.as_ref())
                })
                && self.original_operands_preserve_lookup(tokens, registry, row, 0)
        })
    }

    pub(super) fn original_operands_preserve_lookup(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
        observation: &DeclarationLayoutObservation,
        depth: u32,
    ) -> bool {
        if depth > 256 {
            return false;
        }
        let Some(site) = self.invocation_site() else {
            return false;
        };
        let Some(dialect) = observation
            .snapshot
            .state
            .source_variables
            .invocation_dialect
        else {
            return false;
        };
        let Some(protocol) = dialect.native_string_protocol() else {
            return false;
        };
        let Some(words) = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            observation.config,
        ) else {
            return false;
        };
        words.iter().all(|word| {
            Self::original_word_preserves_lookup(
                word,
                tokens,
                registry,
                observation,
                protocol,
                depth,
            )
        })
    }

    fn original_scalar_read_preserves_lookup(
        word: &NativeWord,
        name: tcl_lexer::Span,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        observation: &DeclarationLayoutObservation,
        registry: &CommandRegistry,
    ) -> bool {
        let arena = word.executable_parts();
        let context = &observation.snapshot.state.source_variables;
        let Some(bytes) = arena.bytes(name) else {
            return false;
        };
        let Ok(bytes) = tcl_syntax::backslash::native_source_literal_bytes(
            bytes,
            arena.image().channel(),
            protocol,
        ) else {
            return false;
        };
        let receiver = crate::var_resolve::resolve_evaluated_variable_input(
            tcl_syntax::naming::NativeVariableInputForm::Separate {
                root: &bytes,
                element: None,
            },
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        if receiver.dynamic || receiver.cell.is_none() {
            return false;
        }
        if receiver.observed {
            // Only the incoming observer residual on an original
            // fresh scalar formal is excluded. Explicit traces,
            // aliases and an interrupted prefix remain unknown.
            if context.dynamic_bindings
                || !context.traced.is_empty()
                || !context.untracked_traces.is_empty()
                || !context.trace_registrations.is_empty()
                || !context.possible_trace_registrations.is_empty()
                || !observation
                    .entry
                    .original_formal_topology()
                    .is_some_and(|topology| topology.fresh_scalar_formal_name(&receiver, context))
            {
                return false;
            }
        }
        true
    }

    fn original_bracket_preserves_lookup(
        word: &NativeWord,
        body: tcl_lexer::Span,
        parent: &CommandTokens,
        registry: &CommandRegistry,
        observation: &DeclarationLayoutObservation,
        depth: u32,
    ) -> bool {
        let arena = word.executable_parts();
        let Some(bytes) = arena.bytes(body) else {
            return false;
        };
        let image = tcl_lexer::SourceImage::from_bytes(bytes, arena.image().channel());
        let Some(segments) = crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            body.start(),
            observation.config,
        ) else {
            return false;
        };
        for segment in segments {
            if segment.is_partial {
                return false;
            }
            let mut tokens = CommandTokens::from_segmented(
                &arena.image().source_map(),
                observation.config,
                &segment,
            );
            tokens.inherit_nested_bindings(parent);
            let Some(binding) = tokens.source_binding.as_ref() else {
                return false;
            };
            let Some(advice) = binding.declaration_operand_layout_advice(&tokens) else {
                return false;
            };
            if !advice.closed_lookup() {
                return false;
            }
            let Some(selected) =
                crate::registry_invocation::declaration_invocation_flow(registry, &tokens, &advice)
            else {
                return false;
            };
            if !selected.effects.lookup_stable
                || selected.effects.unknown_reads
                || selected.effects.unknown_writes
                || !matches!(
                    selected.flow,
                    tcl_registry::script_body_flow::ScriptBodyFlow::None
                )
                || !binding.original_operands_preserve_lookup(
                    &tokens,
                    registry,
                    observation,
                    depth + 1,
                )
                || selected.effective.words.iter().any(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        observation.config.escapes,
                        advice.dialect().word_values,
                    )
                    .literal_bytes()
                    .is_none()
                })
            {
                return false;
            }
        }
        true
    }

    fn original_word_preserves_lookup(
        word: &NativeWord,
        parent: &CommandTokens,
        registry: &CommandRegistry,
        observation: &DeclarationLayoutObservation,
        protocol: tcl_syntax::native_string::NativeStringProtocol,
        depth: u32,
    ) -> bool {
        if word.group().expand {
            // Expanding an unknown object requires list conversion independently
            // of the read that produced it. Static original expansion is inert.
            return tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                std::slice::from_ref(word),
                protocol,
            )
            .is_ok_and(|words| words.literal(0).is_some());
        }
        let arena = word.executable_parts();
        let components = arena.list(arena.root());
        for component in components {
            match &component.part {
                ExecutablePart::Text(_) => {}
                ExecutablePart::Variable { name, index: None } if components.len() == 1 => {
                    if !Self::original_scalar_read_preserves_lookup(
                        word,
                        *name,
                        protocol,
                        observation,
                        registry,
                    ) {
                        return false;
                    }
                    // No concatenation or conversion of an unknown formal is
                    // admitted here. A whole-word object fetch copies its value.
                }
                ExecutablePart::Command { body } => {
                    if !Self::original_bracket_preserves_lookup(
                        word,
                        *body,
                        parent,
                        registry,
                        observation,
                        depth,
                    ) {
                        return false;
                    }
                }
                ExecutablePart::Variable { .. }
                | ExecutablePart::Expression { .. }
                | ExecutablePart::ParseError(_) => return false,
            }
        }
        true
    }
}

fn original_formal_word_retains_root(
    word: &NativeWord,
    name: &tcl_core_types::NameBytes,
    context: &crate::var_resolve::ResolveContext,
    protocol: tcl_syntax::native_string::NativeStringProtocol,
    topology: &super::formal_topology::OriginalFormalTopology,
    registry: &CommandRegistry,
) -> bool {
    if word.group().expand {
        return false;
    }
    let arena = word.executable_parts();
    let [component] = arena.list(arena.root()) else {
        return false;
    };
    let ExecutablePart::Variable {
        name: source_name,
        index: None,
    } = component.part
    else {
        return false;
    };
    let Some(bytes) = arena.bytes(source_name) else {
        return false;
    };
    let Ok(bytes) = tcl_syntax::backslash::native_source_literal_bytes(
        bytes,
        arena.image().channel(),
        protocol,
    ) else {
        return false;
    };
    let resolve = |root: &[u8]| {
        crate::var_resolve::resolve_evaluated_variable_input(
            tcl_syntax::naming::NativeVariableInputForm::Separate {
                root,
                element: None,
            },
            context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        )
    };
    let actual = resolve(&bytes);
    let formal = resolve(name.as_bytes());
    actual.cell == formal.cell
        && !actual.dynamic
        && topology.fresh_scalar_formal_name(&actual, context)
        && matches!(
            context.contents_origin(&actual),
            crate::var_resolve::ContentsOrigin::Incoming
        )
}
