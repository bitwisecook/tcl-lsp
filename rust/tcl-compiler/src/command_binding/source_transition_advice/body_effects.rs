// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original immediate-body mutation coverage for conditional source advice.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation, capture_transition, original_words, registry_words,
};
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, NativeScriptCommandWords, NativeWord, Span};
use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};

const MAX_DEPTH: usize = 64;
const MAX_COMMANDS: usize = 2048;

#[cfg(test)]
fn trace_body_inventory(stage: &str, offset: Option<u32>, passed: bool, count: usize) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_none() {
        return;
    }
    eprintln!(
        "ORIGINAL_BODY_INVENTORY stage={stage} offset={offset:?} passed={passed} count={count}"
    );
}

pub(super) struct BodyEffectProjection {
    pub(super) effects_complete: bool,
    pub(super) inventory: OriginalSourceTransitionAdviceTape,
}

struct BodyWalk<'context, 'source> {
    context: &'context AdviceInvocationContext<'source>,
    visited: usize,
    unavailable_lifecycle_effects: bool,
    inventory: Option<OriginalSourceTransitionAdviceTape>,
    authored_prefixes: OriginalSourceTransitionAdviceTape,
    source_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
    logical_scope: Option<
        Arc<crate::command_binding::source_declared_command::OriginalDeclaredLogicalBodyContext>,
    >,
    member_catalogue_body:
        Option<Arc<crate::registry_invocation::OriginalSourceDefinitionMemberScriptBody>>,
    catalogue_body: Option<super::OriginalSourceCatalogueBody>,
}

struct PreparedBodyRegionBranch {
    graph: AdviceGraph,
    logical_scope: Option<Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
}

// Keep materialisation frames outside recursive source-body descent.
#[inline(never)]
pub(super) fn boxed_body_resolution<'r: 'w, 'w>(
    context: &'r tcl_registry::model::ContextRegistry,
    dialect: tcl_registry::InvocationDialect,
    command: &'w str,
    words: &'w [tcl_registry::InvocationWord<'w>],
) -> Box<tcl_registry::StructuredInvocationResolution<'r, 'w>> {
    Box::new(
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            context.commands(),
            Some(context.context()),
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal(command),
                words,
            )
            .with_dialect(dialect),
            tcl_dialect::model::InvocationRealm::RuleLoader,
        ),
    )
}

#[inline(never)]
pub(super) fn boxed_body_advice(
    context: &AdviceInvocationContext<'_>,
    invocation: AdviceInvocation<'_>,
    graph: &AdviceGraph,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<Box<OriginalSourceCommandTransitionAdvice>> {
    context
        .schema_advice(invocation, graph, schema)
        .map(Box::new)
}

#[inline(never)]
pub(super) fn boxed_region_words(
    context: &AdviceInvocationContext<'_>,
    advice: &OriginalSourceCommandTransitionAdvice,
) -> Option<Box<crate::registry_invocation::source_structure::OriginalRegistryWords>> {
    let source = context.origin.source_image().try_text().ok()?;
    crate::registry_invocation::source_structure::source_transition_words_from_advice(
        source,
        context.config,
        context.context,
        advice.clone(),
    )
    .map(Box::new)
}

/// Original substitution order, omitting descendants visited with their parent.
pub(super) enum OriginalOperandEffect {
    Command(Span),
    Unknown,
}

pub(super) fn original_operand_effects(native: &[NativeWord]) -> Vec<OriginalOperandEffect> {
    let mut covered = Vec::<Span>::new();
    let mut effects = Vec::new();
    for word in native {
        for part in word.executable_parts().all_parts() {
            match part.part {
                ExecutablePart::Text(_) => {}
                ExecutablePart::Command { body, .. } => {
                    if covered
                        .iter()
                        .any(|outer| outer.start() <= body.start() && body.end() <= outer.end())
                    {
                        continue;
                    }
                    effects.push(OriginalOperandEffect::Command(body));
                    covered.push(body);
                }
                _ => effects.push(OriginalOperandEffect::Unknown),
            }
        }
    }
    effects
}

impl AdviceInvocationContext<'_> {
    pub(super) fn apply_source_body_lookup_effects(
        &self,
        graph: &mut AdviceGraph,
        command: &str,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let transitions = schema.state_transitions();
        let receipt = capture_transition(
            self.origin,
            invocation.native,
            command,
            invocation.arguments,
            transitions.clone(),
            invocation.lineage,
        );
        if receipt
            .as_ref()
            .is_some_and(|receipt| graph.apply(receipt).is_none())
            || (receipt.is_none() && !transitions.facts().is_empty())
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            graph.widen(invocation.native);
        }
        let effects = schema.effects();
        // Transition effect coverage removes an already represented trace
        // write from the generic footprint. Source observer state still
        // requires its own owner; a covered registration is not a quiet read.
        let changes_observers = transitions
            .facts()
            .iter()
            .any(|fact| matches!(fact.transition, tcl_registry::StateTransition::Trace(_)))
            || [
                tcl_registry::StateTransitionDomain::VariableTraces,
                tcl_registry::StateTransitionDomain::CommandTraces,
                tcl_registry::StateTransitionDomain::ExecutionTraces,
            ]
            .into_iter()
            .any(|domain| transitions.widens(domain));
        if changes_observers
            || effects.requires_world_barrier()
            || effects.accesses().iter().any(|access| {
                access.mode != EffectAccessMode::Read
                    && matches!(
                        access.domain,
                        WorldStateDomain::VariableTraces
                            | WorldStateDomain::CommandTraces
                            | WorldStateDomain::ExecutionTraces
                    )
            })
        {
            // A trace registration can change future callback state without
            // synchronously reentering this invocation. Its exact producer
            // remains unresolved until a separate observer-state owner closes
            // that state. Neither it nor a synchronous callback footprint
            // permits execution or semantic rewriting from source advice.
            graph.record_uncertainty(invocation.native);
        }
        if effects.accesses().iter().any(|access| {
            access.mode != EffectAccessMode::Read
                && matches!(
                    access.domain,
                    WorldStateDomain::CommandBindings | WorldStateDomain::NamespaceLookup
                )
        }) {
            graph.widen(invocation.native);
        }
    }

    /// Caller coordinates describe source phases under the selected stock
    /// descriptor, independently of actual provider, entry or phase effects.
    pub(super) fn retain_c_source_lifecycle_inventory(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
        depth: usize,
    ) -> bool {
        if depth > MAX_DEPTH || self.c_source_lifecycle_selection(graph, schema).is_none() {
            return false;
        }
        let Some(mut advice) = self.schema_advice(invocation, graph, schema) else {
            return false;
        };
        advice.c_source_body = parent.cloned();
        advice.obligations.push(
            SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability,
        );
        let mut branch = graph.clone();
        branch.record_uncertainty(invocation.native);
        let mut walk = BodyWalk {
            context: self,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: Some(OriginalSourceTransitionAdviceTape::default()),
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: None,
        };
        if walk.regions(&mut branch, &advice, schema, depth).is_some()
            && let Some(inventory) = walk.inventory
        {
            let retained = !inventory.represented.is_empty();
            tape.extend_inventory(inventory);
            return retained;
        }
        false
    }

    pub(super) fn retain_definition_member_prefix_inventory(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        if schema.authored_source_definition_body_grammar().is_none() {
            return;
        }
        let Some(advice) = self.schema_advice(invocation, graph, schema) else {
            return;
        };
        let Ok(source) = self.origin.source_image().try_text() else {
            return;
        };
        let Some(words) =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                self.config,
                self.context,
                advice,
            )
        else {
            return;
        };
        let mut remaining = MAX_COMMANDS;
        for parent in words.source_script_bodies(self.context) {
            let Some(region) = parent.definition_member_region(self.context, parent.content_span())
            else {
                continue;
            };
            for original in region.original_commands() {
                let Some(next) = remaining.checked_sub(1) else {
                    return;
                };
                remaining = next;
                let Some(bodies) = region.script_bodies(original) else {
                    continue;
                };
                for body in bodies {
                    if !matches!(
                        body.member_row().map(|row| row.effect),
                        Some(tcl_registry::definer::MemberEffect::Callable { .. })
                    ) || !body.matches_source(self.origin.source_image(), self.config)
                        || !body.matches_context(self.context)
                    {
                        continue;
                    }
                    let body = Arc::new(body);
                    self.retain_original_body_command_catalogue(
                        tape,
                        graph,
                        super::OriginalSourceCatalogueBody::Member(Arc::clone(&body)),
                        body.original_words(),
                    );
                    let mut branch = graph.clone();
                    branch.record_uncertainty(body.original_words());
                    let mut walk = BodyWalk {
                        context: self,
                        visited: 0,
                        unavailable_lifecycle_effects: false,
                        inventory: None,
                        authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
                        source_body: None,
                        logical_scope: None,
                        member_catalogue_body: Some(body.clone()),
                        catalogue_body: None,
                    };
                    if walk.script(&mut branch, body.content_span(), 0).is_none() {
                        continue;
                    }
                    for prefix in walk.authored_prefixes.authored_prefixes.values_mut() {
                        *prefix = prefix.take().and_then(|prefix| {
                            prefix.retain_definition_member_body(Arc::clone(&body), self.context)
                        });
                    }
                    tape.extend_inventory(walk.authored_prefixes);
                }
            }
        }
    }

    /// The same guarded body issuer also retains source-only syntax where a
    /// current namespace/frame interpretation is unavailable. Effects stay out
    /// of this catalogue and known shadows/deletions remain authoritative.
    pub(super) fn retain_original_body_catalogue(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        if schema.authored_source_definition_body_grammar().is_some() {
            return;
        }
        if !self
            .authored_roles(schema)
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        {
            return;
        }
        if schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
        }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return;
        }
        let Some(advice) = self.schema_advice(invocation, graph, schema) else {
            return;
        };
        let Ok(source) = self.origin.source_image().try_text() else {
            return;
        };
        let Some(words) =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                self.config,
                self.context,
                advice,
            )
        else {
            return;
        };
        for body in words.source_script_bodies(self.context) {
            if !body.matches_source(self.origin.source_image(), self.config)
                || !body.matches_context(self.context)
            {
                continue;
            }
            self.retain_original_body_command_catalogue(
                tape,
                graph,
                super::OriginalSourceCatalogueBody::Script(Arc::new(body)),
                invocation.native,
            );
        }
    }

    fn retain_original_body_command_catalogue(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        body: super::OriginalSourceCatalogueBody,
        original: &[NativeWord],
    ) {
        let mut branch = graph.clone();
        branch.record_uncertainty(original);
        let mut walk = BodyWalk {
            context: self,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: None,
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: Some(body.clone()),
        };
        // Complete source prefixes keep their separately owned fatal tail.
        let _ = walk.script(&mut branch, body.content_span(), 0);
        tape.extend_inventory(walk.authored_prefixes);
    }

    /// Reuse the original executable arena before selecting source head advice.
    /// Known child transitions are processed; unresolved variable/observer
    /// effects stay separate from the post-operand execution lookup.
    pub(super) fn inspect_operand_effects(
        &self,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        tape: &mut OriginalSourceTransitionAdviceTape,
    ) -> Option<bool> {
        let substitutions = native.iter().any(|word| {
            word.executable_parts()
                .all_parts()
                .any(|part| !matches!(part.part, ExecutablePart::Text(_)))
        });
        if substitutions {
            graph.record_uncertainty(native);
            let mut walk = BodyWalk {
                context: self,
                visited: 0,
                unavailable_lifecycle_effects: false,
                inventory: matches!(self.policy, AdviceNamingPolicy::Logical(_))
                    .then(OriginalSourceTransitionAdviceTape::default),
                source_body: None,
                logical_scope: None,
                member_catalogue_body: None,
                catalogue_body: None,
                authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            };
            walk.substitutions(graph, native, 0)?;
            tape.extend_inventory(walk.inventory.unwrap_or_default());
            tape.extend_inventory(walk.authored_prefixes);
        }
        Some(substitutions)
    }

    /// Inspect real original regions before carrying unresolved source advice.
    /// Every possible branch may withdraw an earlier alias; a body never
    /// installs its speculative new declarations into the enclosing graph.
    pub(super) fn apply_immediate_body_effects(
        &self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        logical_scope: Option<
            &Arc<
                crate::command_binding::source_declared_command::OriginalDeclaredLogicalBodyContext,
            >,
        >,
    ) -> BodyEffectProjection {
        let mut walk = BodyWalk {
            context: self,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: ((matches!(self.policy, AdviceNamingPolicy::Logical(_))
                && schema.semantics.body_kind == tcl_registry::BodyKind::Plain)
                || (graph.c_source_namespace.is_some()
                    && !graph.native_baseline
                    && (schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                        || (schema.semantics.body_kind == tcl_registry::BodyKind::Structural
                            && schema
                                .semantics
                                .traits
                                .contains(tcl_registry::Traits::DECLARES_NAMESPACE)))))
            .then(OriginalSourceTransitionAdviceTape::default),
            source_body: None,
            logical_scope: logical_scope.cloned(),
            member_catalogue_body: None,
            catalogue_body: None,
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
        };
        #[cfg(test)]
        trace_body_inventory(
            "immediate.inventory",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            walk.inventory.is_some(),
            advice.original_words().len(),
        );
        let expressions_complete = walk.expressions(graph, advice, schema, 0).is_some();
        #[cfg(test)]
        trace_body_inventory(
            "immediate.expressions",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            expressions_complete,
            walk.visited,
        );
        if !expressions_complete && walk.inventory.is_some() {
            // The conditional source question survives unavailable effects;
            // execution state is still widened by the caller independently.
            graph.record_uncertainty(advice.original_words());
        }
        let regions_complete = (expressions_complete || walk.inventory.is_some())
            && walk.regions(graph, advice, schema, 0).is_some();
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            let offset = advice
                .original_words()
                .first()
                .map(|word| word.span().start());
            trace_body_inventory("immediate.regions", offset, regions_complete, walk.visited);
            if let Some(inventory) = &walk.inventory {
                trace_body_inventory(
                    "immediate.retained_advice",
                    offset,
                    regions_complete,
                    inventory.advice.len(),
                );
                trace_body_inventory(
                    "immediate.represented",
                    offset,
                    regions_complete,
                    inventory.represented.len(),
                );
            }
        }
        let mut inventory = if regions_complete {
            walk.inventory.unwrap_or_default()
        } else {
            OriginalSourceTransitionAdviceTape::default()
        };
        inventory.extend_inventory(walk.authored_prefixes);
        BodyEffectProjection {
            effects_complete: expressions_complete
                && regions_complete
                && !walk.unavailable_lifecycle_effects,
            inventory,
        }
    }
}

impl BodyWalk<'_, '_> {
    fn regions(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let words = boxed_region_words(self.context, advice);
        #[cfg(test)]
        trace_body_inventory(
            "regions.original_vector",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            words.is_some(),
            advice.original_words().len(),
        );
        let words = words?;
        let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(words.arguments().get(ordinal)?.literal_bytes()?).ok()
        });
        if interpreter != tcl_registry::world_effect::InterpreterScope::Current {
            return None;
        }
        let lifecycle = advice.obligations().contains(
            &SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability,
        ) && self
            .context
            .c_source_lifecycle_selection(graph, schema)
            .is_some();
        let regions = if lifecycle {
            Some(words.source_script_bodies(self.context.context))
        } else {
            words.source_script_bodies_for_mutation_coverage(self.context.context)
        };
        #[cfg(test)]
        trace_body_inventory(
            "regions.coverage",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            regions.is_some(),
            regions.as_ref().map_or(0, Vec::len),
        );
        let regions = regions?;
        // Checked empty coverage supplies no evaluated child mutation and remains
        // complete. Reference-only bodies are one such complete case.
        let original = graph.clone();
        for region in regions {
            if !region.matches_source(self.context.origin.source_image(), self.context.config)
                || !region.matches_context(self.context.context)
            {
                return None;
            }
            let span = region.content_span();
            // Changing namespace source coordinates retain their independently
            // selected policy; no frame or body execution follows.
            let mut prepared =
                self.prepare_region_branch(&original, advice, schema, &region, lifecycle)?;
            let catalogue = self.catalogue_body.clone();
            if catalogue.is_some()
                && (schema.semantics.body_kind != tcl_registry::BodyKind::Plain
                    || schema
                        .semantics
                        .traits
                        .intersects(tcl_registry::traits::FRAME_REACH_TRAITS)
                    || schema.facts().frame_effect.is_some())
            {
                self.catalogue_body = Some(super::OriginalSourceCatalogueBody::Script(Arc::new(
                    region.clone(),
                )));
            }
            let source_body = (self.inventory.is_some()
                && (schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                    || prepared.graph.c_source_namespace.is_some()))
            .then(|| Arc::new(region));
            let parent = std::mem::replace(&mut self.source_body, source_body);
            let parent_scope =
                std::mem::replace(&mut self.logical_scope, prepared.logical_scope.take());
            let walked = self.script(&mut prepared.graph, span, depth);
            self.logical_scope = parent_scope;
            self.source_body = parent;
            self.catalogue_body = catalogue;
            walked?;
            Self::retain_prepared_region_effects(
                graph,
                &original,
                prepared,
                advice.original_words(),
            );
        }
        Some(())
    }

    #[inline(never)]
    fn prepare_region_branch(
        &self,
        original: &AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        region: &crate::registry_invocation::OriginalSourceScriptBody,
        lifecycle: bool,
    ) -> Option<Box<PreparedBodyRegionBranch>> {
        let span = region.content_span();
        let mut branch = original.clone();
        let mut child_scope = self.logical_scope.clone();
        if span.start() != span.end()
            && schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            && !lifecycle
        {
            let invocation = AdviceInvocation {
                native: advice.original_words(),
                head: advice.original_head(),
                arguments: advice.arguments(),
                lineage: advice.lineage(),
            };
            match self.context.policy {
                AdviceNamingPolicy::Logical(input) => {
                    if schema.semantics.frame_effect.is_some_and(|frame| {
                        frame.layout == tcl_registry::FrameArgLayout::ScriptInSelectedFrame
                    }) {
                        let scope = self.logical_scope.as_ref()?;
                        if !scope.matches_input(input)
                            || !scope.owns_words(advice.original_words())
                            || original.logical_namespace.as_ref() != Some(scope.namespace_recipe())
                        {
                            return None;
                        }
                        let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                            level,
                            ..
                        } = schema.authored_logical_source_frame_argument_layout()?
                        else {
                            return None;
                        };
                        let body = Arc::new(region.clone());
                        let child = if level.is_global_frame() {
                            scope.with_global_body(body)?
                        } else if level.is_current_frame() {
                            scope.with_body(body)?
                        } else {
                            return None;
                        };
                        branch.logical_namespace = Some(child.namespace_recipe().clone());
                        child_scope = Some(Arc::new(child));
                    } else {
                        let selected = self
                            .context
                            .logical_namespace_scope(original, invocation, schema)?;
                        branch.logical_namespace = Some(selected.namespace().clone());
                    }
                }
                AdviceNamingPolicy::Native(_) => {
                    branch.c_source_namespace = Some(
                        self.context
                            .c_source_namespace_scope(original, invocation, schema)?,
                    );
                }
            }
        }
        Some(Box::new(PreparedBodyRegionBranch {
            graph: branch,
            logical_scope: child_scope,
        }))
    }

    #[inline(never)]
    fn retain_prepared_region_effects(
        graph: &mut AdviceGraph,
        original: &AdviceGraph,
        prepared: Box<PreparedBodyRegionBranch>,
        operation: &[NativeWord],
    ) {
        Self::retain_possible_effects(graph, original, prepared.graph, operation);
    }

    fn retain_possible_effects(
        graph: &mut AdviceGraph,
        original: &AdviceGraph,
        branch: AdviceGraph,
        operation: &[NativeWord],
    ) {
        for (key, cell) in &original.cells {
            if branch.cells.get(key) != Some(cell) {
                // Possible original source mutations block an earlier schema;
                // they do not prove that a delete or replacement succeeded.
                if graph.native_baseline
                    && matches!(cell, AdviceCell::Descriptor(_, lineage) if lineage.is_empty())
                {
                    // The authentic entry schema is a source possibility only;
                    // a conditional branch does not prove its replacement ran.
                    graph.record_uncertainty(operation);
                } else {
                    graph.cells.insert(key.clone(), AdviceCell::Shadowed);
                }
            }
        }
        for namespace in original
            .namespace_paths
            .keys()
            .chain(branch.namespace_paths.keys())
        {
            if original.namespace_paths.get(namespace) != branch.namespace_paths.get(namespace) {
                graph.unknown_paths.insert(namespace.clone());
                graph.namespace_paths.remove(namespace);
            }
        }
        for namespace in original
            .namespace_exports
            .keys()
            .chain(branch.namespace_exports.keys())
        {
            if original.namespace_exports.get(namespace) != branch.namespace_exports.get(namespace)
            {
                graph.unknown_exports.insert(namespace.clone());
            }
        }
        graph.unknown_paths.extend(branch.unknown_paths);
        graph.unknown_exports.extend(branch.unknown_exports);
        for operation in branch.uncertain_operations {
            if !graph.uncertain_operations.contains(&operation) {
                graph.uncertain_operations.push(operation);
            }
        }
    }

    fn expressions(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        let (roles, complete) = self.context.authored_roles(schema);
        #[cfg(test)]
        trace_body_inventory(
            "expressions.roles",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            complete,
            roles.len(),
        );
        if !complete {
            return None;
        }
        for (ordinal, role) in roles {
            if role != tcl_registry::ArgRole::Expr {
                continue;
            }
            let ordinal = schema
                .facts()
                .argument_offset
                .checked_add(usize::from(ordinal))?;
            let argument = advice.arguments().get(ordinal)?;
            if !matches!(
                argument.origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            ) {
                return None;
            }
            let expression_span = argument.original.content_span().ok()?;
            let source = argument
                .original
                .image()
                .try_text()
                .ok()?
                .get(expression_span.as_range())?;
            // Cooked values and incomplete expressions cannot lend their
            // original positions to the checked expression/script bridge.
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
                trace_body_inventory(
                    "expressions.original_value",
                    Some(expression_span.start()),
                    argument.value.as_deref() == Some(source.as_bytes()),
                    source.len(),
                );
            }
            if argument.value.as_deref() != Some(source.as_bytes()) {
                return None;
            }
            let mut context = self
                .context
                .dialect
                .expression_parse_context(self.context.context.commands().profile());
            context.lexer_grammar = self.context.config.grammar_over(context.lexer_grammar);
            let substitutions = match self.context.policy {
                AdviceNamingPolicy::Logical(input) if input.has_logical_source_name_context() => {
                    tcl_syntax::expr::checked_source_expression_substitutions(source, &context)
                }
                AdviceNamingPolicy::Logical(_) => None,
                AdviceNamingPolicy::Native(_) => {
                    if graph.c_source_namespace.is_some() && !graph.native_baseline {
                        tcl_syntax::expr::checked_source_expression_substitutions(source, &context)
                    } else {
                        tcl_syntax::expr::checked_expression_substitutions(source, &context)
                    }
                }
            };
            #[cfg(test)]
            trace_body_inventory(
                "expressions.checked",
                Some(expression_span.start()),
                substitutions.is_some(),
                substitutions
                    .as_ref()
                    .map_or(0, |parts| parts.commands.len()),
            );
            let substitutions = substitutions?;
            if !substitutions.variables.is_empty() || !substitutions.commands.is_empty() {
                graph.record_uncertainty(advice.original_words());
            }
            let original = graph.clone();
            for command in substitutions.commands {
                // The shared expression owner supplies whole brackets. Lazy
                // branches are lexical possibilities, never entered scripts.
                let start = expression_span
                    .start()
                    .checked_add(command.start())?
                    .checked_add(1)?;
                let end = expression_span
                    .start()
                    .checked_add(command.end())?
                    .checked_sub(1)?;
                let mut branch = original.clone();
                let walked = self.script(&mut branch, Span::new(start, end), depth.checked_add(1)?);
                #[cfg(test)]
                trace_body_inventory(
                    "expressions.child",
                    Some(start),
                    walked.is_some(),
                    (end - start) as usize,
                );
                walked?;
                Self::retain_possible_effects(graph, &original, branch, advice.original_words());
            }
        }
        Some(())
    }

    fn script(&mut self, graph: &mut AdviceGraph, span: Span, depth: usize) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            self.context.origin.source_image().clone(),
            span,
            self.context.config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok();
        #[cfg(test)]
        trace_body_inventory(
            "script.plan",
            Some(span.start()),
            plan.is_some(),
            plan.as_ref().map_or(0, |plan| plan.commands.len()),
        );
        let plan = plan?;
        #[cfg(test)]
        trace_body_inventory(
            "script.complete",
            Some(span.start()),
            plan.fatal_tail.is_none(),
            plan.commands.len(),
        );
        if plan.fatal_tail.is_some() && self.catalogue_body.is_none() {
            return None;
        }
        for command in plan.commands {
            self.visited = self.visited.checked_add(1)?;
            if self.visited > MAX_COMMANDS {
                return None;
            }
            self.command(graph, &command, depth)?;
        }
        Some(())
    }

    fn substitutions(
        &mut self,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        depth: usize,
    ) -> Option<()> {
        for effect in original_operand_effects(native) {
            match effect {
                OriginalOperandEffect::Command(body) => {
                    self.script(graph, body, depth.checked_add(1)?)?;
                }
                OriginalOperandEffect::Unknown => graph.record_uncertainty(native),
            }
        }
        Some(())
    }

    fn command(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        depth: usize,
    ) -> Option<()> {
        self.mark_child_header(native)?;
        if let (Some(tape), Some(scope)) = (&mut self.inventory, &self.logical_scope) {
            tape.retain_logical_summary_scope(&native.words, scope);
        }
        if let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body)
            && matches!(self.context.policy, AdviceNamingPolicy::Logical(_))
            && self.context.retain_original_body_constructor_calls(
                tape,
                graph,
                &native.words,
                (
                    body,
                    SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
                ),
            )
        {
            return Some(());
        }
        let class_handle = self.original_body_class_handle(graph, &native.words);
        let original_selection = class_handle.as_ref().and_then(|_| {
            let written = original_words(&native.words, self.context.policy)?;
            graph.resolve(written.first()?.input.as_ref()?)
        });
        self.substitutions(graph, &native.words, depth)?;
        let Some(written) = original_words(&native.words, self.context.policy) else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        let Some(head) = written
            .first()
            .and_then(|word| word.input.as_ref())
            .cloned()
        else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        let Some((command, mut arguments, lineage)) =
            original_selection.or_else(|| graph.resolve(&head))
        else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.into_iter().skip(1));
        let words = registry_words(&arguments);
        let resolution =
            boxed_body_resolution(self.context.context, self.context.dialect, &command, &words);
        let Some(schema) = resolution.resolved() else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        let invocation = AdviceInvocation {
            native: &native.words,
            head: &head,
            arguments: &arguments,
            lineage: &lineage,
        };
        self.retain_child_schema(graph, invocation, &schema)?;
        let retained_lifecycle = if let Some(tape) = &mut self.inventory {
            self.context.retain_c_source_lifecycle_inventory(
                tape,
                graph,
                invocation,
                &schema,
                self.source_body.as_ref(),
                depth.saturating_add(1),
            )
        } else {
            false
        };
        self.apply_original_body_class_effects(graph, &command, invocation, &schema, class_handle);
        if retained_lifecycle {
            // The independent SOURCE walk above owns these conditional phases.
            // Mutation coverage cannot duplicate it without its applicability,
            // and it cannot prove the provider, phase entry or phase effects.
            graph.record_uncertainty(&native.words);
            self.unavailable_lifecycle_effects = true;
            return Some(());
        }
        let immediate = self
            .context
            .authored_roles(&schema)
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY);
        let advice = boxed_body_advice(self.context, invocation, graph, &schema)?;
        if self.expressions(graph, &advice, &schema, depth).is_none() {
            graph.widen(&native.words);
        }
        if immediate
            && self
                .regions(graph, &advice, &schema, depth.checked_add(1)?)
                .is_none()
        {
            graph.widen(&native.words);
        }
        Some(())
    }
    fn original_body_class_handle(
        &self,
        graph: &AdviceGraph,
        native: &[NativeWord],
    ) -> Option<super::source_instance::PendingSourceClassHandle> {
        if !matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
            return None;
        }
        let tape = self.inventory.as_ref()?;
        let written = original_words(native, self.context.policy)?;
        let update = self
            .context
            .before_operand_class_handle(tape, graph, native, &written);
        self.context.retain_source_class_handle_body(
            update,
            self.source_body.as_ref()?,
            SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
        )
    }
    fn apply_original_body_class_effects(
        &mut self,
        graph: &mut AdviceGraph,
        command: &str,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        class_handle: Option<super::source_instance::PendingSourceClassHandle>,
    ) {
        let class = self
            .inventory
            .as_ref()
            .and(self.source_body.as_ref())
            .and_then(|body| {
                if !matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
                    return None;
                }
                self.context.source_class_declaration_in_body(
                    graph,
                    invocation,
                    schema,
                    (
                        body,
                        SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
                    ),
                )
            });
        self.context
            .apply_source_body_lookup_effects(graph, command, invocation, schema);
        AdviceInvocationContext::install_source_class_handle(graph, class_handle);
        if let Some(tape) = &mut self.inventory {
            AdviceInvocationContext::install_source_class(tape, graph, class);
        }
    }
    fn retain_child_schema(
        &mut self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        if let Some(body) = &self.catalogue_body {
            let offset = invocation.native.first()?.span().start();
            if self.authored_prefixes.body_catalogue.len() < MAX_COMMANDS {
                let receipt = (!graph.blocks_registry_source(invocation.head)).then(|| {
                    let mut advice = self.context.schema_advice(invocation, graph, schema)?;
                    advice.catalogue_body = Some(body.clone());
                    advice.obligations.push(SourceCommandTransitionObligation::ConditionalOriginalBodyCatalogueApplicability);
                    (advice.matches_source(self.context.origin.source_image(), self.context.config) && advice.matches_context(self.context.context)).then_some(advice)
                }).flatten();
                super::source_class::merge_receipt(
                    &mut self.authored_prefixes.body_catalogue,
                    offset,
                    receipt,
                );
            }
        }
        if let Some(body) = &self.member_catalogue_body {
            let offset = invocation.native.first()?.span().start();
            if self.authored_prefixes.member_catalogue.len() < 256 {
                let receipt = (|| {
                    let policy = self.context.policy.native()?;
                    let selected = policy
                        .recipe()
                        .command_lookup_projection(
                            super::root_context(policy),
                            invocation.head.bytes(),
                        )
                        .ok()?;
                    if selected.namespace_route()
                        != tcl_syntax::naming::NativeCommandNamespaceRoute::Root
                        || graph.blocks_registry_source(invocation.head)
                        || !body
                            .matches_source(self.context.origin.source_image(), self.context.config)
                        || !body.matches_context(self.context.context)
                    {
                        return None;
                    }
                    let mut advice = self.context.schema_advice(invocation, graph, schema)?;
                    advice.member_catalogue_body = Some(Arc::clone(body));
                    advice.obligations.push(SourceCommandTransitionObligation::ConditionalMemberBodyCatalogueApplicability);
                    (advice.matches_source(self.context.origin.source_image(), self.context.config)
                        && advice.matches_context(self.context.context))
                    .then_some(advice)
                })();
                super::source_class::merge_receipt(
                    &mut self.authored_prefixes.member_catalogue,
                    offset,
                    receipt,
                );
            }
        }
        if let (Some(tape), None) = (&mut self.inventory, &self.source_body) {
            // Immediate original operands use their root point, not a made-up
            // body descriptor or a future frame's applicability.
            if graph.blocks_registry_source(invocation.head) {
                tape.registry_barriers
                    .insert(invocation.native.first()?.span().start());
                return Some(());
            }
            return self
                .context
                .retain_source_inventory(tape, invocation, graph, schema, None);
        }
        let retained_producer = if let Some(body) = &self.source_body {
            let mut advice = self.context.schema_advice(invocation, graph, schema)?;
            Self::retain_source_body(self.context.policy, &mut advice, body);
            if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
                advice.logical_scope = self.logical_scope.clone();
            }
            Some(advice)
        } else {
            None
        };
        self.context.retain_authored_command_prefix(
            &mut self.authored_prefixes,
            invocation,
            graph,
            schema,
            retained_producer.as_ref(),
        );
        let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body) else {
            return Some(());
        };
        self.context
            .retain_source_callback_targets(tape, invocation, graph, schema);
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        if graph.blocks_registry_source(invocation.head) {
            tape.registry_barriers
                .insert(invocation.native.first()?.span().start());
            return Some(());
        }
        let mut advice = self.context.schema_advice(invocation, graph, schema)?;
        Self::retain_source_body(self.context.policy, &mut advice, body);
        if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
            advice.logical_scope = self.logical_scope.clone();
        }
        if let Some(previous) = tape.advice.get(&advice.site.offset) {
            if previous != &advice {
                tape.advice.remove(&advice.site.offset);
                tape.registry_barriers.insert(advice.site.offset);
                return Some(());
            }
        } else if tape.registry_barriers.contains(&advice.site.offset) {
            return Some(());
        }
        self.context.retain_produced_command_prefixes(
            tape,
            invocation,
            graph,
            schema,
            Some(&advice),
        );
        tape.advice.insert(advice.site.offset, advice);
        Some(())
    }

    fn retain_source_body(
        policy: &AdviceNamingPolicy,
        advice: &mut OriginalSourceCommandTransitionAdvice,
        body: &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    ) {
        match policy {
            AdviceNamingPolicy::Logical(_) => {
                advice.logical_body = Some(Arc::clone(body));
                advice
                    .obligations
                    .push(SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability);
            }
            AdviceNamingPolicy::Native(_) => {
                advice.c_source_body = Some(Arc::clone(body));
                advice
                    .obligations
                    .push(SourceCommandTransitionObligation::ConditionalCBodyApplicability);
                if matches!(body.source_words().source(),
                    crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(parent)
                    if parent.obligations().contains(&SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability))
                {
                    advice.obligations.push(SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability);
                }
            }
        }
    }

    fn mark_child_header(&mut self, native: &NativeScriptCommandWords) -> Option<()> {
        if let Some(tape) = &mut self.inventory {
            tape.represented
                .insert(native.words.first()?.span().start());
        }
        Some(())
    }
}

#[cfg(test)]
mod logical_body_inventory_tests {
    use crate::command_binding::SourceCommandTransitionObligation;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words,
    };

    fn analyse(source: &str) -> crate::analyser::AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn logical_immediate_body_schema_keeps_original_geometry_and_unknown_effects_separate() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for source in [
            "switch $subject one {set local 1} default {set other 2}",
            "if $condition {set local 1}",
            "foreach item $values {set local 1}",
        ] {
            let analysis = analyse(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let config = input.lexer_config();
            let context = input.context_registry();
            let root = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
            let parent = source_registry_words(source, &analysis, &root).unwrap();
            let bodies = parent.source_script_bodies(&context);
            assert!(!bodies.is_empty(), "{source}");
            for body in bodies {
                let span = body.content_span();
                let child = crate::segmenter::segment_commands_with_offset_and_config(
                    &source[span.as_range()],
                    span.start(),
                    config,
                )
                .remove(0);
                let words = source_registry_words(source, &analysis, &child).expect(source);
                let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                    panic!("missing own conditional Logical body: {source}");
                };
                assert_eq!(advice.logical_source_input(), Some(input));
                assert_eq!(advice.logical_source_body(), Some(&body));
                assert_eq!(
                    words.roles(),
                    Some([(0, tcl_registry::ArgRole::VarWrite)].as_slice())
                );
                assert!(advice.obligations().contains(
                    &SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability
                ));
                assert!(
                    advice
                        .obligations()
                        .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                );
                assert!(!words.operands_preserve_source_lookup());
                assert!(
                    !advice
                        .matches_source(&tcl_lexer::SourceImage::native(source.as_bytes()), config)
                );
            }
        }
    }

    #[test]
    fn logical_immediate_body_schema_keeps_child_mutations_and_foreign_owners_terminal() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "switch $subject one {rename set gone; set blocked 1}";
        let analysis = analyse(source);
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let offset = u32::try_from(source.find("set blocked").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[usize::try_from(offset).unwrap()..source.len() - 1],
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(source, &analysis, &segment).is_none());
        let source = "if $condition {set local 1}";
        let mut analysis = analyse(source);
        let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[usize::try_from(offset).unwrap()..source.len() - 1],
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(source, &analysis, &segment).is_some());
        assert!(source_registry_words(&format!("{source} "), &analysis, &segment).is_none());
        analysis.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        });
        assert!(source_registry_words(source, &analysis, &segment).is_none());
        let native = crate::analyser::Analyser::new().analyse(source, "tcl9.1");
        if let Some(words) = source_registry_words(source, &native, &segment)
            && let OriginalRegistrySource::SourceTransitions(advice) = words.source()
        {
            assert!(advice.logical_source_input().is_none());
            assert!(
                !advice.obligations().contains(
                    &SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability
                )
            );
        }
    }
    #[test]
    fn logical_root_substitutions_keep_alias_point_without_future_body_applicability() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "interp alias {} make {} list set x; eval [make $value]";
        let analysis = analyse(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let offset = u32::try_from(source.find("make $value").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "make $value",
            offset,
            input.lexer_config(),
        )
        .remove(0);
        let words = source_registry_words(source, &analysis, &segment).unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("missing genuine original alias substitution point");
        };
        assert_eq!(advice.logical_source_input(), Some(input));
        assert!(advice.logical_source_body().is_none());
        assert!(advice.original_head().native_input().is_none());
        assert!(
            !advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability)
        );
        assert!(
            !advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::DeferredLogicalBodyApplicability)
        );
        assert_eq!(words.command(), "list");
        assert_eq!(
            words
                .arguments()
                .get(0)
                .and_then(|word| word.literal_bytes()),
            Some(b"set".as_slice())
        );
        assert_eq!(
            words
                .arguments()
                .get(1)
                .and_then(|word| word.literal_bytes()),
            Some(b"x".as_slice())
        );
    }

    #[test]
    fn logical_root_substitutions_keep_earlier_mutations_and_owner_refusals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let source = "interp alias {} make {} list set x; eval [make $value]";
        let baseline = analyse(source);
        let offset = u32::try_from(source.find("make $value").unwrap()).unwrap();
        let config = baseline.body_lexer_config.unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            "make $value",
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(source, &baseline, &segment).is_some());
        let changed =
            "interp alias {} make {} list set x; eval [list [rename make {}] [make $value]]";
        let changed_analysis = analyse(changed);
        let offset = u32::try_from(changed.find("make $value").unwrap()).unwrap();
        let blocked = crate::segmenter::segment_commands_with_offset_and_config(
            "make $value",
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(changed, &changed_analysis, &blocked).is_none());
        assert!(source_registry_words(&format!("{source} "), &baseline, &segment).is_none());
        let mut missing = baseline.clone();
        missing.resolved_input = None;
        assert!(source_registry_words(source, &missing, &segment).is_none());
        let mut config_changed = baseline;
        config_changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        });
        assert!(source_registry_words(source, &config_changed, &segment).is_none());
    }
}

#[cfg(test)]
mod original_empty_coverage_tests {
    use super::super::{AdviceTapeInputs, SourceCommandTransitionObligation};
    use super::*;
    use tcl_registry::{
        ArgRole, Arity, CommandRegistry, CommandSpec, InvocationArguments, ScriptTiming,
    };

    fn reference_only(_arguments: InvocationArguments<'_>) -> Vec<(u8, ScriptTiming)> {
        vec![(0, ScriptTiming::ReferenceOnly)]
    }

    fn projection(
        source: &str,
        timing: Option<tcl_registry::ScriptTimingResolver>,
    ) -> (BodyEffectProjection, bool) {
        let mut registry = CommandRegistry::build_default();
        registry.insert(CommandSpec {
            name: "selected-script",
            arity: Arity::exact(1),
            arg_roles: &[(0, ArgRole::Body)],
            script_timing_resolver: timing,
            ..CommandSpec::DEFAULT
        });
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = Arc::new(
            tcl_registry::model::context_for_profile(profile)
                .with_command_store(Arc::new(registry)),
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        } = bindings.prepare_source_advice_tape(&context).unwrap();
        let mut graph = bindings
            .initial_source_advice_graph(&context, &policy)
            .unwrap();
        let invocation = &plan.commands[0];
        let written = original_words(&invocation.words, &policy).unwrap();
        let head = written[0].input.as_ref().unwrap();
        let (command, mut arguments, lineage) = graph.resolve(head).unwrap();
        arguments.extend(written.iter().skip(1).cloned());
        let words = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved().unwrap();
        let selected = AdviceInvocation {
            native: &invocation.words,
            head,
            arguments: &arguments,
            lineage: &lineage,
        };
        let owner = AdviceInvocationContext {
            origin,
            context: &context,
            config,
            dialect,
            policy: &policy,
        };
        let advice = owner.schema_advice(selected, &graph, &schema).unwrap();
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(
            advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability)
        );
        let original = original_words(&plan.commands[1].words, &policy).unwrap();
        let following = original[0].input.as_ref().unwrap();
        let before = graph.resolve(following);
        assert!(before.is_some(), "real selected following Registry command");
        let projection = owner.apply_immediate_body_effects(&mut graph, &advice, &schema, None);
        (projection, graph.resolve(following) == before)
    }

    #[test]
    fn original_reference_only_coverage_is_complete_empty_and_keeps_following_source_lookup() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        // Conditional source mutation coverage only; no execution or handler grant.
        let (projection, unchanged) = projection(
            "selected-script {rename set {}; rename puts {}}; set x 1",
            Some(reference_only),
        );
        assert!(projection.effects_complete);
        assert!(unchanged);
        assert!(projection.inventory.advice.is_empty());
        assert!(projection.inventory.represented.is_empty());
        assert!(projection.inventory.registry_barriers.is_empty());
    }

    #[test]
    fn original_unavailable_evaluated_body_coverage_cannot_claim_complete_empty() {
        // naming.source.original-script-region-purpose
        // docs/design/analysis/name-resolution-proofs/original-script-region-purpose.md
        // A dynamic or recooked selected body remains unavailable mutation coverage.
        for source in [
            "selected-script $body; set x 1",
            "selected-script \"rename set {};\\nrename puts {}\"; set x 1",
        ] {
            let (projection, _unchanged) = projection(source, None);
            assert!(!projection.effects_complete, "{source}");
            assert!(projection.inventory.advice.is_empty());
            assert!(projection.inventory.represented.is_empty());
            assert!(projection.inventory.registry_barriers.is_empty());
        }
    }
}

#[cfg(test)]
mod original_source_expression_topology_tests {
    use crate::analyser::ResolvedAnalysisInput;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, OriginalSourceRegistryContext,
    };

    fn input() -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            strict_quoting: true,
            braced_var: tcl_lexer::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        ResolvedAnalysisInput::new(profile, profile, context, config)
    }

    #[test]
    fn original_logical_expression_child_retains_alias_actual_base_and_no_native_recipe() {
        // naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        // Original conditional source schema only; no entered command or runtime output.
        let input = input();
        assert!(input.has_logical_source_name_context());
        let source = "\u{feff}interp alias {} \u{feff}match {} regexp\nif {![\u{feff}match λ λ out]} {puts $out}";
        let syntax = OriginalSourceRegistryContext::capture(source, input.clone());
        let start = u32::try_from(source.find("\u{feff}match λ").unwrap()).unwrap();
        let end = u32::try_from(source.find("]}").unwrap()).unwrap();
        let children = crate::segmenter::segment_commands_with_offset_and_config(
            &source[start as usize..end as usize],
            start,
            input.lexer_config().nested(),
        );
        let [child] = children.as_slice() else {
            panic!("complete single original command child")
        };
        let words = syntax.words(child).unwrap();
        assert_eq!(words.command(), "regexp");
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("original transition advice")
        };
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert!(advice.original_head().native_input().is_none());
        assert_eq!(advice.original_head().bytes(), "\u{feff}match".as_bytes());
        assert_eq!(advice.original_words()[0].span().start(), start);
        assert!(
            advice
                .original_words()
                .iter()
                .all(|word| word.config() == input.lexer_config()
                    && word.image().bytes() == source.as_bytes())
        );
        let foreign_source = format!("{source} ");
        let foreign = tcl_lexer::SourceImage::document(&foreign_source);
        assert!(!advice.matches_source(&foreign, input.lexer_config()));
        let mut changed = input.lexer_config();
        changed.strict_quoting = !changed.strict_quoting;
        assert!(!advice.matches_source(&tcl_lexer::SourceImage::document(source), changed));
        let shifted = crate::segmenter::segment_commands_with_offset_and_config(
            &source[start as usize..end as usize],
            start + 1,
            input.lexer_config().nested(),
        );
        assert!(syntax.words(&shifted[0]).is_none());
    }
}

#[cfg(test)]
mod original_expression_purpose_frontier_tests {
    use super::super::AdviceTapeInputs;
    use super::*;

    #[test]
    fn original_logical_expression_topology_cannot_supply_missing_native_parser_purpose() {
        // naming.source.authored-command-transition-advice
        // docs/design/analysis/name-resolution-proofs/authored-command-transition-advice.md
        // Descriptive source/name/parser axes only, without a Native invocation.
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context =
            crate::environment_ingress::resolve_environment("tcl8.6").default_context_registry();
        let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        let input =
            crate::analyser::ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let source = "if {![regexp x y out]} {puts $out}";
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation");
        let bindings = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let AdviceTapeInputs {
            origin,
            config,
            policy,
            dialect,
            plan,
        } = bindings.prepare_source_advice_tape(&context).unwrap();
        assert!(matches!(policy, AdviceNamingPolicy::Logical(_)));
        let mut graph = bindings
            .initial_source_advice_graph(&context, &policy)
            .unwrap();
        let invocation = &plan.commands[0];
        let written = original_words(&invocation.words, &policy).unwrap();
        let head = written[0].input.as_ref().unwrap();
        let (command, mut arguments, lineage) = graph.resolve(head).unwrap();
        arguments.extend(written.iter().skip(1).cloned());
        let words = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved().unwrap();
        let owner = AdviceInvocationContext {
            origin,
            context: &context,
            config,
            dialect,
            policy: &policy,
        };
        let advice = owner
            .schema_advice(
                AdviceInvocation {
                    native: &invocation.words,
                    head,
                    arguments: &arguments,
                    lineage: &lineage,
                },
                &graph,
                &schema,
            )
            .unwrap();
        assert_eq!(advice.logical_source_input(), Some(&input));
        assert_eq!(
            dialect
                .expression_parse_context(context.commands().profile())
                .native_syntax,
            tcl_syntax::expr::parser::NativeExprSyntax::Unknown
        );
        let mut logical = BodyWalk {
            context: &owner,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: Some(OriginalSourceTransitionAdviceTape::default()),
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: None,
        };
        assert!(
            logical
                .expressions(&mut graph, &advice, &schema, 0)
                .is_some()
        );
        assert_eq!(logical.visited, 1);
        let native_policy = AdviceNamingPolicy::Native(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
                .authored_name_policy()
                .unwrap(),
        );
        let native_owner = AdviceInvocationContext {
            origin,
            context: &context,
            config,
            dialect,
            policy: &native_policy,
        };
        let mut native = BodyWalk {
            context: &native_owner,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: None,
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: None,
        };
        assert!(
            native
                .expressions(&mut graph, &advice, &schema, 0)
                .is_none()
        );
        assert_eq!(native.visited, 0);
        assert!(advice.original_head().native_input().is_none());
        assert_eq!(advice.logical_source_input(), Some(&input));
    }
}
