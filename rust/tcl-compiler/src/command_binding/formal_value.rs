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

enum FormalValueSource<'a> {
    Incoming(String),
    Copy(&'a SourceVariableAccess),
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
        if self.root_origin.as_ref() != Some(origin) {
            return None;
        }
        let entry = self.conditional_body_entry_at(origin, source.span.start())?;
        if !entry.matches_parameters(parameters) {
            return None;
        }
        if let Some(access) = self.variable_access_at(source, spelling) {
            return self.formal_value_for_access(&entry, access, registry, true);
        }
        self.original_direct_formal_value(&entry, source, spelling, registry)
    }

    fn original_direct_formal_value(
        &self,
        entry: &SourceConditionalBodyEntry,
        source: &SourceSite,
        spelling: &str,
        registry: &CommandRegistry,
    ) -> Option<String> {
        let mut agreed = None;
        let mut observed = false;
        for (site, observations) in &self.declaration_layouts {
            if site.source != entry.source().origin || !entry.owns_source(&site.source, site.offset)
            {
                continue;
            }
            let Some(observations) =
                super::declaration_layout::original_declaration_layouts(observations)
            else {
                continue;
            };
            for observation in observations {
                if !matches!(observation.entry.as_ref(), super::declaration_layout::OriginalDiagnosticFrameEntry::Body(body) if body.as_ref() == entry)
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
                let context = &observation.snapshot.state.source_variables;
                if observation.snapshot.state.has_opaque_domain() {
                    return None;
                }
                let dialect = context.invocation_dialect?;
                if observation.words[..index].iter().any(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                    .literal_bytes()
                    .is_none()
                }) {
                    return None;
                }
                if !Self::declared_body_owns_context(entry, context) {
                    continue;
                }
                let value = declared_incoming_formal_name(entry, spelling, context)?;
                let tokens = self.declaration_original_tokens_at(entry, site.offset)?;
                let report = tokens
                    .source_binding
                    .as_ref()?
                    .declaration_flow_report(registry)?;
                if !report.conditional_handler_keeps_incoming(site, &value) {
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

/// Identity of an ordinary declared formal, conditional on the untouched
/// declaration prefix. This does not require or construct an activation cell.
fn declared_incoming_formal_name(
    entry: &SourceConditionalBodyEntry,
    spelling: &str,
    context: &ResolveContext,
) -> Option<String> {
    if context.dynamic_bindings || context.dynamic_traces {
        return None;
    }
    let dialect = context.invocation_dialect?;
    let name =
        tcl_syntax::naming::var_reference_for_style(spelling, dialect.lexer_grammar.braced_var);
    if tcl_syntax::naming::is_qualified(name.as_bytes())
        || tcl_syntax::naming::split_element_ref(name).is_some()
        || super::declaration_layout::local_read_scope_is_excluded(context, name)
    {
        return None;
    }
    let plan = tcl_syntax::formal_params::bind_formal_arguments(
        entry.parameters(),
        entry.parameters().len(),
        dialect.parameter_grammar()?,
    )
    .ok()?;
    plan.into_iter().find_map(|binding| {
        use tcl_syntax::formal_params::FormalArgumentBinding;
        let (parameter, slot) = match binding {
            FormalArgumentBinding::Value { parameter, .. }
            | FormalArgumentBinding::Default { parameter }
            | FormalArgumentBinding::Rest { parameter, .. } => {
                (parameter, entry.parameters()[parameter].name.as_str())
            }
            FormalArgumentBinding::CallerLink { .. } => return None,
        };
        (name == slot).then(|| entry.parameters()[parameter].name.clone())
    })
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
    fn declaration_formal_identity_does_not_donate_activation_contents() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let source = "proc p {target} {upvar 1 $target v}";
        let inventory = super::SourceCommandBindings::analyse(source, config, registry);
        let origin = inventory.root_origin.as_ref().unwrap();
        let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
        let entry = inventory.conditional_body_entry_at(origin, offset).unwrap();
        let observation = &inventory.declaration_layouts[&super::super::CommandAllocationSite {
            source: origin.clone(),
            offset,
        }][0];
        let mut context = observation.snapshot.state.source_variables.as_ref().clone();
        context.define_unknown_contents("target", registry);
        assert!(
            context
                .incoming_activation_slot("$target", "target", registry)
                .is_none()
        );
        assert_eq!(
            super::declared_incoming_formal_name(&entry, "${target}", &context).as_deref(),
            Some("target")
        );
        assert!(super::declared_incoming_formal_name(&entry, "$::target", &context).is_none());
        assert!(super::declared_incoming_formal_name(&entry, "$target(index)", &context).is_none());
        assert!(
            context
                .incoming_activation_slot("$target", "target", registry)
                .is_none()
        );
        context.dynamic_bindings = true;
        assert!(super::declared_incoming_formal_name(&entry, "$target", &context).is_none());
        context.dynamic_bindings = false;
        context.dynamic_traces = true;
        assert!(super::declared_incoming_formal_name(&entry, "$target", &context).is_none());
    }

    #[test]
    fn unentered_formal_advice_uses_original_snapshot_without_actual_read_or_cpp() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
            for (source, expected) in [
                ("proc p {target} {upvar 1 $target v}", Some("target")),
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
                    continue;
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
                        .symbolic_formal_value_at(
                            &origin,
                            source_site,
                            spelling,
                            &["target"],
                            registry
                        )
                        .is_none()
                );
                let binding = tokens.source_binding.as_ref().unwrap();
                assert!(binding.proved_execution_target().is_none());
                assert!(binding.compiler_lookup_state.is_none());
                assert!(binding.original_normal_result(&tokens).is_none());
            }
        }
    }
}
