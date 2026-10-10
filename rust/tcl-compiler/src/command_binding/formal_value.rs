// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Symbolic formal input provenance in an original body's owned frame.
//!
//! A read receipt identifies an original formal value, rather than the cell
//! currently named by its source spelling. Declared and entered frame receipts
//! remain separate. Recorded stores and source-instance
//! attestations own copy chains. This query grants no actual activation, caller
//! address, successful alias, completed store or concrete value.

use super::{SourceCommandBindings, SourceOriginId, SourceVariableAccess};
use crate::{
    command_binding::conditional_body::SourceConditionalBodyEntry,
    ir::{CommandTokens, SourceSite},
    var_resolve::{ContentsOrigin, ResolveContext},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tcl_registry::CommandRegistry;

macro_rules! trace_formal {
    ($source:expr, $stage:literal, $available:expr) => {
        #[cfg(debug_assertions)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
            eprintln!(
                "ORIGINAL_FORMAL_VALUE span={:?} stage={} available={}",
                $source.span, $stage, $available
            );
        }
    };
}

enum FormalValueSource<'a> {
    Incoming(String),
    Copy(&'a SourceVariableAccess),
}

fn direct_formal_argument_value(
    site: &super::CommandAllocationSite,
    observation: &super::declaration_layout::DeclarationLayoutObservation,
    source: &SourceSite,
    index: usize,
    parameters: &[&str],
    registry: &CommandRegistry,
) -> Option<(CommandTokens, String)> {
    // naming.tcloo.original-declared-receiver-caller-traits
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
    let context = &observation.snapshot.state.source_variables;
    // Command-table uncertainty cannot change this conditional input.
    // Only independently retained caller-frame aliases may precede a
    // receiver declaration's formal fetch; other aliases remain terminal.
    // The shared resolver, incoming-value graph and original operand
    // checks below still own this formal's slot, writes and observers.
    trace_formal!(source, "static-bindings", !context.dynamic_bindings);
    if context.dynamic_bindings
        || !context.traced.is_empty()
        || !context.untracked_traces.is_empty()
        || !context.trace_registrations.is_empty()
        || !context.possible_trace_registrations.is_empty()
    {
        return None;
    }
    trace_formal!(
        source,
        "original-topology",
        observation.entry.original_formal_topology().is_some()
    );
    let topology = observation.entry.original_formal_topology()?;
    if topology.parameters().len() != parameters.len()
        || !topology
            .parameters()
            .iter()
            .zip(parameters)
            .all(|(formal, expected)| formal.name.as_slice() == expected.as_bytes())
    {
        return None;
    }
    let tokens = super::declaration_preview::declaration_tokens(site, observation)?;
    let native = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        observation.config,
    )?;
    let word = native.get(index)?;
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    if !matches!(
        part.part,
        tcl_lexer::ExecutablePart::Variable { index: None, .. }
    ) {
        return None;
    }
    let root =
        crate::signature_scan::variable_name::SignatureSourceVariableRoot::from_original_word(
            word,
            part.span,
            tcl_syntax::word_rules::WordValueRules::from_config(&observation.config),
            topology.original_input().policy(),
        )?;
    let receiver = crate::var_resolve::resolve_evaluated_variable_input(
        tcl_syntax::naming::NativeVariableInputForm::Separate {
            root: root.bytes(),
            element: None,
        },
        context,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    trace_formal!(
        source,
        "fresh-scalar-formal",
        topology.fresh_scalar_formal_name(&receiver, context)
    );
    if !topology.fresh_scalar_formal_name(&receiver, context) {
        return None;
    }
    let value = std::str::from_utf8(receiver.cell.as_ref()?.name.as_bytes())
        .ok()?
        .to_owned();
    Some((tokens, value))
}

fn original_formal_observation_binding(
    site: &super::CommandAllocationSite,
    observation: &super::declaration_layout::DeclarationLayoutObservation,
) -> super::SourceInvocationBinding {
    super::SourceInvocationBinding {
        dispatch_site: Some(site.clone()),
        declaration_layout_observations: Some(vec![observation.clone()].into()),
        lookup_state: Some(Arc::clone(&observation.snapshot)),
        lookup_namespace_key: observation.namespace.clone(),
        variable_context: Arc::clone(&observation.snapshot.state.source_variables),
        variable_frame: observation.entry.frame().clone(),
        ..Default::default()
    }
}

impl SourceCommandBindings {
    pub(crate) fn symbolic_formal_value_at(
        &self,
        origin: &Arc<SourceOriginId>,
        source: &SourceSite,
        spelling: &str,
        parameters: &[&str],
        registry: &CommandRegistry,
    ) -> Option<String> {
        if self.root_origin.as_ref() != Some(origin) {
            return None;
        }
        let entry = self.conditional_body_entry_at(origin, source.span.start())?;
        if !entry.matches_parameters(parameters) {
            return None;
        }
        let access = self.variable_access_at(source, spelling)?;
        self.formal_value_for_access(&entry, access, registry, false)
    }

    /// Formal input in the immutable declaration's own frame. Entered-call
    /// observations cannot donate values or erase this conditional provenance.
    pub(crate) fn symbolic_declaration_formal_value_at(
        &self,
        origin: &Arc<SourceOriginId>,
        source: &SourceSite,
        spelling: &str,
        parameters: &[&str],
        registry: &CommandRegistry,
    ) -> Option<String> {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        trace_formal!(
            source,
            "root-origin",
            self.root_origin.as_ref() == Some(origin)
        );
        trace_formal!(
            source,
            "conditional-entry",
            self.conditional_body_entry_at(origin, source.span.start())
                .is_some()
        );
        trace_formal!(
            source,
            "variable-access",
            self.variable_access_at(source, spelling).is_some()
        );
        if self.root_origin.as_ref() != Some(origin) {
            return None;
        }
        if let Some(entry) = self.conditional_body_entry_at(origin, source.span.start())
            && entry.matches_parameters(parameters)
            && let Some(access) = self.variable_access_at(source, spelling)
            && let Some(value) = self.formal_value_for_access(&entry, access, registry, true)
        {
            return Some(value);
        }
        let value =
            self.original_direct_formal_value(origin, source, spelling, parameters, registry);
        trace_formal!(source, "original-direct-result", value.is_some());
        value
    }

    fn original_direct_formal_value(
        &self,
        origin: &Arc<SourceOriginId>,
        source: &SourceSite,
        spelling: &str,
        parameters: &[&str],
        registry: &CommandRegistry,
    ) -> Option<String> {
        let mut agreed = None;
        let mut observed = false;
        for (site, observations) in &self.declaration_layouts {
            if &site.source != origin {
                continue;
            }
            let Some(observations) =
                super::declaration_layout::original_declaration_layouts(observations)
            else {
                continue;
            };
            for observation in observations {
                if !matches!(
                    observation.entry.as_ref(),
                    super::declaration_layout::OriginalDiagnosticFrameEntry::Body(_)
                        | super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredProcedure(
                            _
                        )
                        | super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredReceiver(
                            _
                        )
                ) || !observation.entry.owns_source(origin, source.span.start())
                {
                    continue;
                }
                let Some(index) = observation
                    .words
                    .iter()
                    .position(|word| word.sole_variable_substitution() == Some((spelling, source)))
                else {
                    continue;
                };
                trace_formal!(source, "matching-declaration-word", true);
                if !self.original_formal_alias_prefix_preserves_input(site, observation, registry) {
                    return None;
                }
                let (mut tokens, value) = direct_formal_argument_value(
                    site,
                    observation,
                    source,
                    index,
                    parameters,
                    registry,
                )?;
                let binding = original_formal_observation_binding(site, observation);
                trace_formal!(
                    source,
                    "operand-lookup",
                    binding.original_operands_preserve_lookup(&tokens, registry, observation, 0)
                );
                if !binding.original_operands_preserve_lookup(&tokens, registry, observation, 0) {
                    return None;
                }
                tokens.source_binding = Some(self.attach_declaration_operand_layout(
                    binding,
                    Some(origin),
                    site.offset,
                ));
                // naming.compiler.original-formal-argument-entry
                // docs/design/analysis/name-resolution-proofs/original-formal-argument-entry.md
                // Source entry precedes this handler. All original operand
                // effects were independently closed above, so unavailable
                // dispatch cannot erase the incoming formal fetch; prior
                // writers/aliases/observers still withdraw the report facet.
                let report = tokens
                    .source_binding
                    .as_ref()?
                    .declaration_flow_report(registry)?;
                trace_formal!(
                    source,
                    "incoming-argument-entry",
                    report.conditional_argument_keeps_incoming(site, &value)
                );
                if !report.conditional_argument_keeps_incoming(site, &value) {
                    return None;
                }
                if agreed.as_ref().is_some_and(|previous| previous != &value) {
                    return None;
                }
                observed = true;
                agreed = Some(value);
            }
        }
        observed.then_some(agreed).flatten()
    }

    fn original_formal_alias_prefix_preserves_input(
        &self,
        site: &super::CommandAllocationSite,
        observation: &super::declaration_layout::DeclarationLayoutObservation,
        registry: &CommandRegistry,
    ) -> bool {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        let context = &observation.snapshot.state.source_variables;
        if matches!(
            observation.entry.as_ref(),
            super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredReceiver(_)
        ) {
            // An unresolved alias transition can withdraw the prefix without
            // installing an enumerable alias. Empty maps cannot discharge it.
            return observation.entry.owns_original_context(context)
                && self
                    .original_formal_prefix_report(site, observation, registry)
                    .is_some_and(|report| report.conditional_alias_prefix_targets_caller(site));
        }
        context.alias_bindings.is_empty()
            && context.name_alias_bindings.is_empty()
            && context.upvar_aliases.is_empty()
    }

    fn original_formal_prefix_report(
        &self,
        site: &super::CommandAllocationSite,
        observation: &super::declaration_layout::DeclarationLayoutObservation,
        registry: &CommandRegistry,
    ) -> Option<Arc<super::declaration_flow::DeclarationFlowReport>> {
        let binding = original_formal_observation_binding(site, observation);
        self.attach_declaration_operand_layout(binding, Some(&site.source), site.offset)
            .declaration_flow_report(registry)
    }

    /// Direct original argument components in the declaration's owned frame.
    /// Nested commands cannot donate their reads as outer argument components.
    pub(crate) fn symbolic_declaration_formal_components_in_word(
        &self,
        origin: &Arc<SourceOriginId>,
        invocation: u32,
        word: &crate::ir::WordExpr,
        parameters: &[&str],
        registry: &CommandRegistry,
    ) -> Vec<String> {
        self.symbolic_formal_components_in_word_for(
            origin, invocation, word, parameters, registry, true,
        )
    }

    fn symbolic_formal_components_in_word_for(
        &self,
        origin: &Arc<SourceOriginId>,
        invocation: u32,
        word: &crate::ir::WordExpr,
        parameters: &[&str],
        registry: &CommandRegistry,
        declaration_only: bool,
    ) -> Vec<String> {
        fn direct(
            owner: &super::SourceVariableEvaluationOwner,
            site: &super::CommandAllocationSite,
        ) -> bool {
            match owner {
                super::SourceVariableEvaluationOwner::InvocationArguments {
                    invocation, ..
                } => invocation == site,
                super::SourceVariableEvaluationOwner::Alternatives(owners) => {
                    !owners.is_empty() && owners.iter().all(|owner| direct(owner, site))
                }
                _ => false,
            }
        }
        let site = super::CommandAllocationSite {
            source: Arc::clone(origin),
            offset: invocation,
        };
        let span = word.source().span;
        self.variable_accesses
            .values()
            .flatten()
            .filter(|access| {
                span.start() <= access.source.span.start()
                    && access.source.span.end() <= span.end()
                    && direct(&access.owner, &site)
            })
            .filter_map(|access| {
                let project = if declaration_only {
                    Self::symbolic_declaration_formal_value_at
                } else {
                    Self::symbolic_formal_value_at
                };
                project(
                    self,
                    origin,
                    &access.source,
                    &access.original_spelling,
                    parameters,
                    registry,
                )
            })
            .collect()
    }

    fn formal_value_for_access(
        &self,
        entry: &SourceConditionalBodyEntry,
        access: &SourceVariableAccess,
        registry: &CommandRegistry,
        declaration_only: bool,
    ) -> Option<String> {
        let mut work = vec![(access, false)];
        let mut active = HashSet::new();
        let mut edges = HashMap::<SourceSite, Vec<FormalValueSource<'_>>>::new();
        let mut resolved = HashMap::<SourceSite, String>::new();
        while let Some((read, expanded)) = work.pop() {
            if resolved.contains_key(&read.source) {
                continue;
            }
            if expanded {
                let mut unanimous = None;
                for source in edges.remove(&read.source)? {
                    let value = match source {
                        FormalValueSource::Incoming(value) => value,
                        FormalValueSource::Copy(prior) => resolved.get(&prior.source)?.clone(),
                    };
                    if unanimous
                        .as_ref()
                        .is_some_and(|previous| previous != &value)
                    {
                        return None;
                    }
                    unanimous = Some(value);
                }
                resolved.insert(read.source.clone(), unanimous?);
                active.remove(&read.source);
                continue;
            }
            if !active.insert(read.source.clone()) {
                return None;
            }
            let mut sources = Vec::new();
            for context in read.context_alternatives().iter().filter(|context| {
                if declaration_only {
                    Self::declared_body_owns_context(entry, context)
                } else {
                    self.original_body_owns_context(entry, context)
                }
            }) {
                sources.push(self.formal_value_in_context(entry, read, context, registry)?);
            }
            trace_formal!(&read.source, "owned-read-context", !sources.is_empty());
            if sources.is_empty() {
                return None;
            }
            work.push((read, true));
            for source in &sources {
                if let FormalValueSource::Copy(prior) = source {
                    work.push((prior, false));
                }
            }
            edges.insert(read.source.clone(), sources);
        }
        resolved.remove(&access.source)
    }

    fn formal_value_in_context<'a>(
        &'a self,
        entry: &SourceConditionalBodyEntry,
        access: &SourceVariableAccess,
        context: &ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<FormalValueSource<'a>> {
        trace_formal!(
            &access.source,
            "context-static-bindings",
            !context.dynamic_bindings
        );
        trace_formal!(
            &access.source,
            "incoming-formal",
            incoming_formal_value(entry, &access.original_spelling, context, registry).is_some()
        );
        if context.dynamic_bindings {
            return None;
        }
        if let Some(value) =
            incoming_formal_value(entry, &access.original_spelling, context, registry)
        {
            return Some(FormalValueSource::Incoming(value));
        }
        let place = access.place_in_context(context, registry);
        if place.observed || place.dynamic || place.index.is_some() {
            return None;
        }
        let ContentsOrigin::WrittenAt(offset) = context.read_contents_origin(&place, registry)
        else {
            return None;
        };
        let origin = self.root_origin.as_ref()?;
        if !context.contents_have_source(&place, origin) || !entry.owns_source(origin, offset) {
            return None;
        }
        let tokens = self.declaration_original_tokens_at(entry, offset)?;
        let selected =
            crate::registry_invocation::resolved_handler_invocation(registry, None, &tokens)?;
        if selected.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            || selected.arguments.len() != 2
        {
            return None;
        }
        let original = selected.effective.written_argument(1)?;
        let (spelling, source) = tokens
            .words()
            .get(original + 1)?
            .sole_variable_substitution()?;
        let rhs = self.variable_access_at(source, spelling)?;
        Some(FormalValueSource::Copy(rhs))
    }

    pub(super) fn diagnostic_original_tokens_at(
        &self,
        entry: &super::declaration_layout::OriginalDiagnosticFrameEntry,
        offset: u32,
    ) -> Option<CommandTokens> {
        if let super::declaration_layout::OriginalDiagnosticFrameEntry::Body(body) = entry {
            return self.declaration_original_tokens_at(body, offset);
        }
        let origin = self.root_origin.as_ref()?;
        if !entry.owns_source(origin, offset) {
            return None;
        }
        let points = || {
            self.dispatch_points_at(offset)
                .filter(|point| entry.owns_frame(self, &point.state.variable_frame))
        };
        let binding = Self::query_dispatch_points(points());
        if !entry.owns_frame(self, &binding.variable_frame) {
            return None;
        }
        let text = origin.try_text().ok()?;
        let end = points().map(|point| point.end).max()?;
        let config = self.lexer_config?;
        let segmented = crate::segmenter::segment_commands_with_offset_and_config(
            text.get(offset as usize..end as usize)?,
            offset,
            config,
        )
        .into_iter()
        .next()?;
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(text), config, &segmented);
        self.stamp_original_tokens(&mut tokens);
        tokens.source_binding = Some(self.attach_declaration_operand_layout(
            self.attach_invocation_reads(binding, Some(origin), offset),
            Some(origin),
            offset,
        ));
        Some(tokens)
    }

    pub(super) fn declaration_original_tokens_at(
        &self,
        entry: &SourceConditionalBodyEntry,
        offset: u32,
    ) -> Option<CommandTokens> {
        let mut points = self
            .dispatch_points_at(offset)
            .filter(|point| self.original_body_owns_frame(entry, &point.state.variable_frame))
            .peekable();
        if points.peek().is_none() {
            return self.unentered_declaration_tokens_at(entry, offset);
        }
        // The declaration owns each retained activation independently. Their
        // runtime-frame join stays unknown and supplies no execution licence.
        let binding = Self::query_dispatch_points(points);
        let origin = self.root_origin.as_ref()?;
        let text = origin.try_text().ok()?;
        let end = self
            .dispatch_points_at(offset)
            .filter(|point| self.original_body_owns_frame(entry, &point.state.variable_frame))
            .map(|point| point.end)
            .max()?;
        let command = text.get(offset as usize..end as usize)?;
        let config = self.lexer_config?;
        let segmented =
            crate::segmenter::segment_commands_with_offset_and_config(command, offset, config)
                .into_iter()
                .next()?;
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(text), config, &segmented);
        self.stamp_original_tokens(&mut tokens);
        tokens.source_binding = Some(self.attach_declaration_operand_layout(
            self.attach_invocation_reads(binding, Some(origin), offset),
            Some(origin),
            offset,
        ));
        Some(tokens)
    }
    fn unentered_declaration_tokens_at(
        &self,
        entry: &SourceConditionalBodyEntry,
        offset: u32,
    ) -> Option<CommandTokens> {
        let site = super::CommandAllocationSite {
            source: std::sync::Arc::clone(&entry.source().origin),
            offset,
        };
        let observations = super::declaration_layout::original_declaration_layouts(
            self.declaration_layouts.get(&site)?,
        )?;
        let first = observations.clone().next()?;
        if observations.clone().any(|observation| {
            !matches!(observation.entry.as_ref(), super::declaration_layout::OriginalDiagnosticFrameEntry::Body(body) if body.as_ref() == entry)
                || observation.words != first.words || observation.config != first.config
        }) { return None; }
        let text = entry.source().try_text().ok()?;
        let relative = usize::try_from(offset.checked_sub(entry.source().base())?).ok()?;
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            text.get(relative..)?,
            offset,
            first.config,
        )
        .into_iter()
        .next()?;
        let root = entry.source().origin.try_text().ok()?;
        let mut tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(root), first.config, &segment);
        if tokens.words() != first.words.as_ref() {
            return None;
        }
        let mut binding = super::SourceInvocationBinding::unknown();
        binding.runtime_reachability = super::SourceRuntimeReachability::Conditional;
        binding = self.attach_declaration_operand_layout(binding, Some(&site.source), offset);
        tokens.source_binding = Some(binding);
        Some(tokens)
    }
}

fn incoming_formal_value(
    entry: &SourceConditionalBodyEntry,
    spelling: &str,
    context: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<String> {
    if context.dynamic_bindings {
        return None;
    }
    let grammar = context.invocation_dialect?.parameter_grammar()?;
    let plan = tcl_syntax::formal_params::bind_formal_arguments(
        entry.parameters(),
        entry.parameters().len(),
        grammar,
    )
    .ok()?;
    for formal in plan {
        use tcl_syntax::formal_params::FormalArgumentBinding;
        let (parameter, slot) = match &formal {
            FormalArgumentBinding::Value { parameter, .. }
            | FormalArgumentBinding::Default { parameter } => {
                (*parameter, entry.parameters()[*parameter].name.as_str())
            }
            FormalArgumentBinding::Rest {
                parameter, name, ..
            } => (*parameter, name.as_str()),
            FormalArgumentBinding::CallerLink { .. } => continue,
        };
        if context
            .incoming_activation_slot(spelling, slot, registry)
            .is_some()
        {
            return Some(entry.parameters()[parameter].name.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_receiver_formal_value_uses_its_unentered_declaration() {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
            let text = "oo::class create C {method pick {input} {upvar $input alias}}";
            let mut inventory = super::SourceCommandBindings::analyse(text, config, registry);
            let origin = inventory.root_origin.as_ref().unwrap().clone();
            let offset = u32::try_from(text.find("upvar").unwrap()).unwrap();
            let site = super::super::CommandAllocationSite {
                source: origin.clone(),
                offset,
            };
            let observation = super::super::declaration_layout::original_declaration_layouts(
                inventory.declaration_layouts.get(&site).unwrap(),
            )
            .unwrap()
            .next()
            .unwrap();
            assert!(matches!(
                observation.entry.as_ref(),
                super::super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredReceiver(_)
            ));
            let tokens =
                super::super::declaration_preview::declaration_tokens(&site, observation).unwrap();
            let (spelling, source) = tokens.words()[1].sole_variable_substitution().unwrap();
            inventory.points.clear();
            inventory.dispatch_points.clear();
            inventory.origin_points.clear();
            inventory.variable_accesses.clear();
            inventory.origin_variable_accesses.clear();
            assert_eq!(
                inventory
                    .symbolic_declaration_formal_value_at(
                        &origin,
                        source,
                        spelling,
                        &["input"],
                        registry,
                    )
                    .as_deref(),
                Some("input"),
                "{profile}"
            );
            assert!(
                inventory
                    .symbolic_formal_value_at(&origin, source, spelling, &["input"], registry,)
                    .is_none(),
                "{profile}: no entered receiver value is supplied"
            );
            for changed in ["$::input", "$input(index)", "$other"] {
                assert!(
                    inventory
                        .symbolic_declaration_formal_value_at(
                            &origin,
                            source,
                            changed,
                            &["input"],
                            registry,
                        )
                        .is_none(),
                    "{profile}: {changed}"
                );
            }
            assert!(
                inventory
                    .symbolic_declaration_formal_value_at(
                        &origin,
                        source,
                        spelling,
                        &["different"],
                        registry,
                    )
                    .is_none()
            );
        }
    }

    fn assert_receiver_formal_prefix(profile: &str, prefix: &str, expected: Option<&str>) {
        let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let text =
            format!("oo::class create C {{method pick {{input}} {{{prefix} upvar $input alias}}}}");
        let mut inventory = super::SourceCommandBindings::analyse(&text, config, registry);
        let origin = inventory.root_origin.as_ref().unwrap().clone();
        let offset = u32::try_from(text.rfind("upvar").unwrap()).unwrap();
        let site = super::super::CommandAllocationSite {
            source: origin.clone(),
            offset,
        };
        let observation = super::super::declaration_layout::original_declaration_layouts(
            inventory.declaration_layouts.get(&site).unwrap(),
        )
        .unwrap()
        .next()
        .unwrap();
        assert!(matches!(
            observation.entry.as_ref(),
            super::super::declaration_layout::OriginalDiagnosticFrameEntry::DeclaredReceiver(_)
        ));
        let tokens =
            super::super::declaration_preview::declaration_tokens(&site, observation).unwrap();
        let (spelling, source) = tokens.words()[1].sole_variable_substitution().unwrap();
        inventory.points.clear();
        inventory.dispatch_points.clear();
        inventory.origin_points.clear();
        inventory.variable_accesses.clear();
        inventory.origin_variable_accesses.clear();
        assert_eq!(
            inventory
                .symbolic_declaration_formal_value_at(
                    &origin,
                    source,
                    spelling,
                    &["input"],
                    registry,
                )
                .as_deref(),
            expected,
            "{profile}: {prefix}"
        );
        assert!(
            inventory
                .symbolic_formal_value_at(&origin, source, spelling, &["input"], registry)
                .is_none(),
            "{profile}: conditional source input grants no entered receiver value"
        );
    }

    #[test]
    fn original_receiver_formal_prefix_keeps_unrelated_aliases_and_refuses_clobbers() {
        // naming.tcloo.original-declared-receiver-caller-traits
        // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (prefix, expected) in [
                ("upvar name local; set local VALUE;", Some("input")),
                ("upvar 1 name local; set local VALUE;", Some("input")),
                ("set input CHANGED;", None),
                ("upvar name input;", None),
                ("upvar 0 input local; set local CHANGED;", None),
                (
                    "upvar name local; upvar 0 input local; set local CHANGED;",
                    None,
                ),
                ("upvar $unknown local;", None),
                ("upvar $level input local; set local CHANGED;", None),
                ("trace add variable input read callback;", None),
                ("opaque;", None),
            ] {
                assert_receiver_formal_prefix(profile, prefix, expected);
            }
        }
    }

    #[test]
    fn declaration_formal_identity_does_not_donate_activation_contents() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
            let text = "proc p {target} {upvar 1 ${target} v}";
            let mut inventory = super::SourceCommandBindings::analyse(text, config, registry);
            let origin = inventory.root_origin.as_ref().unwrap().clone();
            let offset = u32::try_from(text.find("upvar").unwrap()).unwrap();
            let site = super::super::CommandAllocationSite {
                source: origin.clone(),
                offset,
            };
            let observation = super::super::declaration_layout::original_declaration_layouts(
                inventory.declaration_layouts.get(&site).unwrap(),
            )
            .unwrap()
            .next()
            .unwrap();
            let tokens =
                super::super::declaration_preview::declaration_tokens(&site, observation).unwrap();
            let (spelling, source) = tokens.words()[2].sole_variable_substitution().unwrap();
            inventory.points.clear();
            inventory.dispatch_points.clear();
            inventory.origin_points.clear();
            inventory.variable_accesses.clear();
            inventory.origin_variable_accesses.clear();
            assert_eq!(
                inventory
                    .symbolic_declaration_formal_value_at(
                        &origin,
                        source,
                        spelling,
                        &["target"],
                        registry,
                    )
                    .as_deref(),
                Some("target"),
                "{profile}"
            );
            for changed in ["$::target", "$target(index)", "$other"] {
                assert!(
                    inventory
                        .symbolic_declaration_formal_value_at(
                            &origin,
                            source,
                            changed,
                            &["target"],
                            registry,
                        )
                        .is_none(),
                    "{profile}: {changed}"
                );
            }
            assert!(
                inventory
                    .symbolic_declaration_formal_value_at(
                        &origin,
                        source,
                        spelling,
                        &["different"],
                        registry,
                    )
                    .is_none()
            );
            assert!(
                inventory
                    .symbolic_formal_value_at(&origin, source, spelling, &["target"], registry,)
                    .is_none()
            );
        }
    }

    fn assert_unentered_formal_case(
        source: &str,
        expected: Option<&str>,
        registry: &tcl_registry::CommandRegistry,
        profile: &str,
        config: tcl_lexer::LexerConfig,
    ) {
        let mut inventory = super::SourceCommandBindings::analyse(source, config, registry);
        let origin = inventory.root_origin.as_ref().unwrap().clone();
        let offset = u32::try_from(source.rfind("upvar").unwrap()).unwrap();
        let entry = inventory
            .conditional_body_entry_at(&origin, offset)
            .unwrap();
        inventory.points.clear();
        inventory.dispatch_points.clear();
        inventory.origin_points.clear();
        inventory.variable_accesses.clear();
        inventory.origin_variable_accesses.clear();
        let Some(tokens) = inventory.declaration_original_tokens_at(&entry, offset) else {
            assert_eq!(expected, None, "{profile}: unavailable original {source}");
            return;
        };
        let word = tokens
            .words()
            .iter()
            .find(|word| {
                word.sole_variable_substitution()
                    .is_some_and(|(spelling, _)| spelling == "$target")
            })
            .unwrap();
        let (spelling, source_site) = word.sole_variable_substitution().unwrap();
        assert_eq!(
            inventory
                .symbolic_declaration_formal_value_at(
                    &origin,
                    source_site,
                    spelling,
                    &["target"],
                    registry
                )
                .as_deref(),
            expected,
            "{profile}: {source}"
        );
        assert!(
            inventory
                .symbolic_declaration_formal_value_at(
                    &origin,
                    source_site,
                    "$different",
                    &["target"],
                    registry
                )
                .is_none()
        );
        assert!(
            inventory
                .symbolic_formal_value_at(&origin, source_site, spelling, &["target"], registry)
                .is_none()
        );
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(binding.proved_execution_target().is_none());
        assert!(binding.compiler_lookup_state.is_none());
        assert!(binding.original_normal_result(&tokens).is_none());
    }

    #[test]
    fn unentered_formal_advice_uses_original_snapshot_without_actual_read_or_cpp() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
            // naming.compiler.original-formal-source-domain
            // docs/design/analysis/name-resolution-proofs/original-formal-source-domain.md
            for (source, expected) in [
                ("proc p {target} {upvar 1 $target v}", Some("target")),
                ("proc p\\uD800 {target} {upvar 1 $target v}", Some("target")),
                ("proc p\\uD801 {target} {upvar 1 $target v}", Some("target")),
                // naming.compiler.original-formal-argument-entry
                // docs/design/analysis/name-resolution-proofs/original-formal-argument-entry.md
                (
                    "proc p\\uD800 {target} {upvar 1 $target local; set local 1}\nproc p\\uD801 {target} {upvar 1 $target local; set local}\np\\uD800 written\np\\uD801 read",
                    Some("target"),
                ),
                (
                    "proc p\\uD800 {target} {upvar 1 $target local}\nproc p\\uD801 {target} {opaque; upvar 1 $target local}",
                    None,
                ),
                (
                    "proc p\\uD800 {target} {upvar 1 $target local}\nproc p\\uD801 {target} {set target overwritten; upvar 1 $target local}",
                    None,
                ),
                ("proc p\\uD800 {target} {opaque; upvar 1 $target v}", None),
                (
                    "proc p\\uD800 {target} {trace add variable target read callback; upvar 1 $target v}",
                    None,
                ),
                (
                    "proc p {target} {set target overwritten; upvar 1 $target v}",
                    None,
                ),
                (
                    "proc p {target} {upvar [set target overwritten] $target v}",
                    None,
                ),
                (
                    "proc p {target} {if {$mode} {set target overwritten}; upvar 1 $target v}",
                    None,
                ),
                ("proc p {target} {opaque; upvar 1 $target v}", None),
                ("proc p {target} {global target; upvar 1 $target v}", None),
                (
                    "proc p {target} {upvar 1 external target; upvar 1 $target v}",
                    None,
                ),
            ] {
                assert_unentered_formal_case(source, expected, registry, profile, config);
            }
        }
    }
}
