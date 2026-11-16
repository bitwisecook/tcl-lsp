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
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_none() {
        return;
    }
    eprintln!(
        "ORIGINAL_BODY_INVENTORY stage={stage} offset={offset:?} passed={passed} count={count}"
    );
}

/// One original expression command region under its retained parent schema.
/// This source-only owner supplies neither a script Body role nor execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSourceExpressionCommand {
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    ordinal: usize,
    original: NativeWord,
    command: Span,
}

impl OriginalSourceExpressionCommand {
    pub(super) fn content_span(&self) -> Span {
        self.command
    }

    pub(super) fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.parent.matches_source(image, config)
            && self.original.image() == image
            && self.original.config() == config
            && self
                .parent
                .arguments()
                .get(self.ordinal)
                .is_some_and(|word| word.original == self.original)
            && self.original.content_span().is_ok_and(|extent| {
                extent.start() <= self.command.start() && self.command.end() <= extent.end()
            })
    }

    pub(super) fn matches_context(&self, context: &tcl_registry::model::ContextRegistry) -> bool {
        self.parent.matches_context(context)
    }
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
impl AdviceGraph {
    /// Take the command world a completed body left behind, keeping this
    /// script's own namespace, frame and handle context.
    fn adopt_completed_body_world(&mut self, body: Self) {
        let Self {
            stock_source_effects,
            cells,
            namespaces,
            namespace_exports,
            namespace_paths,
            namespace_imports,
            unknown_exports,
            unknown_paths,
            uncertain_operations,
            possible_occupants,
            arity_debts,
            procedure_declarations,
            class_declarations,
            ..
        } = body;
        self.stock_source_effects = stock_source_effects;
        self.cells = cells;
        self.namespaces = namespaces;
        self.namespace_exports = namespace_exports;
        self.namespace_paths = namespace_paths;
        self.namespace_imports = namespace_imports;
        self.unknown_exports = unknown_exports;
        self.unknown_paths = unknown_paths;
        self.uncertain_operations = uncertain_operations;
        self.possible_occupants = possible_occupants;
        self.arity_debts = arity_debts;
        self.procedure_declarations = procedure_declarations;
        self.class_declarations = class_declarations;
    }
}

/// The applicability a class declared in an entered body carries: a Logical
/// body always, a C source namespace body on a Tcl release, nothing else.
fn body_class_obligation(
    context: &AdviceInvocationContext<'_>,
    graph: &AdviceGraph,
) -> Option<SourceCommandTransitionObligation> {
    match context.policy {
        AdviceNamingPolicy::Logical(_) => {
            Some(SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability)
        }
        AdviceNamingPolicy::Native(policy)
            if matches!(
                policy.recipe(),
                tcl_syntax::naming::NativeNameProtocol::C(_)
            ) && context.dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                && !graph.native_baseline
                && graph.c_source_namespace.is_some() =>
        {
            Some(SourceCommandTransitionObligation::ConditionalCBodyApplicability)
        }
        AdviceNamingPolicy::Native(_) => None,
    }
}

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
    pub(super) fn normal_case_source_layout(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<tcl_registry::normal_source_case::NormalSourceCaseLayout> {
        let Some(tcl_syntax::naming::NativeNameProtocol::C(version)) = self
            .policy
            .native()
            .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
        else {
            return None;
        };
        if self.dialect.tcl_version != Some(version)
            || schema.words.arguments().dialect() != Some(self.dialect)
        {
            return None;
        }
        tcl_registry::normal_source_case::case_layout_on_normal_source_continuation(
            schema,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )
    }

    pub(super) fn apply_source_body_lookup_effects(
        &self,
        graph: &mut AdviceGraph,
        command: &str,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        residual_footprint: bool,
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
        if residual_footprint {
            Self::apply_source_lookup_residual_effects(graph, invocation, schema);
        }
    }

    pub(super) fn apply_source_lookup_residual_effects(
        graph: &mut AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let transitions = schema.state_transitions();
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
                        &super::OriginalSourceCatalogueBody::Member(Arc::clone(&body)),
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
        // A body is reached only after the invocation's own conditions ran, so
        // their possible effects (including possibly created commands) stay
        // ahead of every body catalogue lookup.
        let mut after_conditions = graph.clone();
        let mut conditions = BodyWalk {
            context: self,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: None,
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: None,
        };
        if conditions
            .expressions(&mut after_conditions, &advice, schema, 0)
            .is_none()
        {
            after_conditions.widen(invocation.native);
        }
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
                &after_conditions,
                &super::OriginalSourceCatalogueBody::Script(Arc::new(body)),
                invocation.native,
            );
        }
    }

    fn retain_original_body_command_catalogue(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        body: &super::OriginalSourceCatalogueBody,
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

    pub(super) fn retain_original_expression_catalogue(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let (roles, complete) = self.authored_roles(schema);
        if !complete
            || !roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.traits.intersects(
                tcl_registry::traits::FRAME_REACH_TRAITS | tcl_registry::Traits::DEFERS_BODY,
            )
            || invocation.native.iter().any(|word| word.group().expand)
            || schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return;
        }
        let Some(advice) = self.schema_advice(invocation, graph, schema) else {
            return;
        };
        self.retain_logical_math_functions(tape, graph, &advice, schema);
        let mut branch = graph.clone();
        let mut walk = BodyWalk {
            context: self,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: None,
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: None,
        };
        if walk
            .expression_commands(&mut branch, &advice, schema, 0, true)
            .is_none()
        {
            return;
        }
        // Only immutable child schemas leave the discarded expression branch.
        for (offset, receipt) in walk.authored_prefixes.body_catalogue {
            super::source_class::merge_receipt(&mut tape.body_catalogue, offset, receipt);
        }
    }

    pub(super) fn retain_native_expression_syntax_inventory(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let (roles, complete) = self.authored_roles(schema);
        if !matches!(self.policy, AdviceNamingPolicy::Native(_))
            || graph.procedure_body.is_none()
            || !complete
            || !roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)
            || invocation.native.iter().any(|word| word.group().expand)
        {
            return;
        }
        let Some(advice) = self.schema_advice(invocation, graph, schema) else {
            return;
        };
        let mut branch = graph.clone();
        let mut walk = BodyWalk {
            context: self,
            visited: 0,
            unavailable_lifecycle_effects: false,
            inventory: None,
            authored_prefixes: OriginalSourceTransitionAdviceTape::default(),
            source_body: None,
            logical_scope: None,
            member_catalogue_body: None,
            catalogue_body: None,
        };
        if walk.expressions(&mut branch, &advice, schema, 0).is_none() {
            return;
        }
        // Expression syntax remains owned when its possible child has no handler.
        // Speculative effects and other advice stay in the discarded branch.
        for (offset, receipt) in walk.authored_prefixes.native_procedure_may_commands {
            super::source_class::merge_receipt(
                &mut tape.native_procedure_may_commands,
                offset,
                receipt,
            );
        }
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
            let native_parent = matches!(self.policy, AdviceNamingPolicy::Native(_));
            if !native_parent {
                graph.record_uncertainty(native);
            }
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
            if matches!(
                self.query,
                Some(super::SourceAdviceQuery::NativeDeferredAutoload)
            ) {
                walk.authored_prefixes
                    .native_deferred_autoload_prefixes
                    .clone_from(&tape.native_deferred_autoload_prefixes);
            }
            let walked = walk.substitutions(graph, native, 0);
            if native_parent {
                graph.record_uncertainty(native);
            }
            walked?;
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
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
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
    fn normal_case_regions(
        &self,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Vec<crate::registry_invocation::OriginalSourceScriptBody>> {
        self.context.normal_case_source_layout(schema)?;
        let owner = words.normal_source_case_bodies(self.context.context)?;
        Some(owner.bodies()?.to_vec())
    }

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
            words
                .source_script_bodies_for_mutation_coverage(self.context.context)
                .or_else(|| self.normal_case_regions(&words, schema))
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
        let runs_once = self.body_runs_once(schema, lifecycle, regions.len());
        for region in regions {
            if !region.matches_source(self.context.origin.source_image(), self.context.config)
                || !region.matches_context(self.context.context)
            {
                return None;
            }
            if self.declares_tcloo_members_only(&region) {
                continue;
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
            if runs_once {
                graph.adopt_completed_body_world(prepared.graph);
            } else {
                Self::retain_prepared_region_effects(
                    graph,
                    &original,
                    prepared,
                    advice.original_words(),
                );
            }
        }
        Some(())
    }

    /// Whether this invocation is a `TclOO` definer whose every script region
    /// only declares members. Unavailable coverage answers no.
    fn definition_declares_members_only(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> bool {
        if schema
            .authored_source_definition_body_grammar()
            .is_none_or(|grammar| grammar.family != tcl_registry::definer::DefinerFamily::TclOo)
            || !original_operand_effects(invocation.native).is_empty()
        {
            return false;
        }
        boxed_body_advice(self.context, invocation, graph, schema)
            .and_then(|advice| boxed_region_words(self.context, &advice))
            .and_then(|words| {
                words.source_script_bodies_for_mutation_coverage(self.context.context)
            })
            .is_some_and(|regions| {
                regions
                    .iter()
                    .all(|region| self.declares_tcloo_members_only(region))
            })
    }

    /// A namespace body is its command's one script and runs once, in the
    /// same invocation: whatever follows the command in this script runs only
    /// after the body completed. Its command-world effects are then this
    /// script's own, where every other region stays a possibility.
    fn body_runs_once(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        lifecycle: bool,
        regions: usize,
    ) -> bool {
        matches!(self.context.policy, AdviceNamingPolicy::Native(_))
            && !lifecycle
            && regions == 1
            && schema.semantics.body_kind == tcl_registry::BodyKind::Structural
            && schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DECLARES_NAMESPACE)
    }

    /// A `TclOO` definition body whose every command is a written member of
    /// its grammar with no substitution. Such a body declares methods,
    /// relations and state on the class; it publishes no command and runs no
    /// member body, so it leaves the command world as it found it.
    fn declares_tcloo_members_only(
        &self,
        region: &crate::registry_invocation::OriginalSourceScriptBody,
    ) -> bool {
        let Some(members) =
            region.definition_member_region(self.context.context, region.content_span())
        else {
            return false;
        };
        members.grammar().family == tcl_registry::definer::DefinerFamily::TclOo
            && members
                .complete_original_commands()
                .is_some_and(|commands| {
                    commands.iter().all(|command| {
                        original_operand_effects(command).is_empty()
                            && members.script_bodies(command).is_some()
                    })
                })
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
                        let child = selected.body_scope(
                            Arc::new(region.clone()),
                            input,
                            self.logical_scope.clone(),
                        )?;
                        branch.logical_namespace = Some(child.namespace_recipe().clone());
                        child_scope = Some(child);
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
        // An untouched branch still shares the original's cells: nothing to compare.
        let cells_changed = !branch.cells.shares(&original.cells);
        for (key, cell) in original.cells.iter().filter(|_| cells_changed) {
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
        // A command first created by a possible branch is not installed, but
        // a later lookup may no longer fall through its coordinate.
        for key in branch.cells.keys().filter(|_| cells_changed) {
            if !original.cells.contains_key(key) {
                graph.possible_occupants.insert(key.clone());
            }
        }
        graph.possible_occupants.extend(branch.possible_occupants);
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
        for debt in branch.arity_debts {
            if !graph.arity_debts.contains(&debt) {
                graph.arity_debts.push(debt);
            }
        }
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
        self.expression_commands(graph, advice, schema, depth, false)
    }

    #[cfg(test)]
    fn trace_expression_roles(
        advice: &OriginalSourceCommandTransitionAdvice,
        complete: bool,
        roles: &[(u8, tcl_registry::ArgRole)],
    ) {
        trace_body_inventory(
            "expressions.roles",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            complete,
            roles.len(),
        );
    }

    #[cfg(test)]
    fn trace_expression_value(
        argument: &super::SourceAdviceWord,
        expression_span: Span,
        source: &str,
    ) {
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_body_inventory(
                "expressions.original_value",
                Some(expression_span.start()),
                argument.value.as_deref() == Some(source.as_bytes()),
                source.len(),
            );
        }
    }

    #[cfg(test)]
    fn trace_expression_parts(
        expression_span: Span,
        substitutions: Option<&tcl_syntax::expr::LiveExpressionSubstitutions>,
    ) {
        trace_body_inventory(
            "expressions.checked",
            Some(expression_span.start()),
            substitutions.is_some(),
            substitutions.map_or(0, |parts| parts.commands.len()),
        );
    }

    #[cfg(test)]
    fn trace_expression_child(start: u32, end: u32, walked: Option<()>) {
        trace_body_inventory(
            "expressions.child",
            Some(start),
            walked.is_some(),
            (end - start) as usize,
        );
    }

    fn body_expression_context(&self) -> tcl_syntax::expr::parser::ExprParseContext {
        let mut context = self
            .context
            .dialect
            .expression_parse_context(self.context.context.commands().profile());
        context.lexer_grammar = self.context.config.grammar_over(context.lexer_grammar);
        context
    }

    fn expression_commands(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
        catalogue: bool,
    ) -> Option<()> {
        let (roles, complete) = self.context.authored_roles(schema);
        #[cfg(test)]
        Self::trace_expression_roles(advice, complete, &roles);
        if !complete {
            // A normal-source case owns only a subject and literal case list;
            // argv substitutions were inspected separately, never as Expr.
            let words = boxed_region_words(self.context, advice)?;
            self.normal_case_regions(&words, schema)?;
            return Some(());
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
            Self::trace_expression_value(argument, expression_span, source);
            if argument.value.as_deref() != Some(source.as_bytes()) {
                return None;
            }
            let context = self.body_expression_context();
            let substitutions = self.checked_body_expression_substitutions(graph, source, &context);
            #[cfg(test)]
            Self::trace_expression_parts(expression_span, substitutions.as_ref());
            let substitutions = substitutions?;
            // A condition of a selected first-match chain runs once, before any
            // clause body, so its command children are ordered by threading each
            // one's possible effects into the next. Variable reads (traces),
            // unwalked math-function calls and repeated or fixture-timed
            // conditions still leave the parent operation itself uncertain.
            let ordered = schema.semantics.clause_grammar.is_some_and(
                tcl_registry::ClauseGrammarSpec::evaluates_conditions_once_before_bodies,
            ) && tcl_syntax::expr::OriginalSourceExpression::capture(
                source,
                &context,
                self.context.config,
            )
            .is_some_and(|expression| expression.function_calls().is_empty());
            if !substitutions.variables.is_empty()
                || (!ordered && !substitutions.commands.is_empty())
            {
                graph.record_uncertainty(advice.original_words());
            }
            let original = (!ordered).then(|| graph.clone());
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
                let threaded;
                let base = if let Some(original) = &original {
                    original
                } else {
                    threaded = graph.clone();
                    &threaded
                };
                let mut branch = base.clone();
                let parent_catalogue = self.catalogue_body.clone();
                if catalogue {
                    let owner = OriginalSourceExpressionCommand {
                        parent: Arc::new(advice.clone()),
                        ordinal,
                        original: argument.original.clone(),
                        command: Span::new(start, end),
                    };
                    if !owner
                        .matches_source(self.context.origin.source_image(), self.context.config)
                        || !owner.matches_context(self.context.context)
                    {
                        return None;
                    }
                    self.catalogue_body = Some(super::OriginalSourceCatalogueBody::Expression(
                        Arc::new(owner),
                    ));
                }
                let walked = self.script(&mut branch, Span::new(start, end), depth.checked_add(1)?);
                self.catalogue_body = parent_catalogue;
                #[cfg(test)]
                Self::trace_expression_child(start, end, walked);
                walked?;
                Self::retain_possible_effects(graph, base, branch, advice.original_words());
            }
        }
        Some(())
    }

    fn checked_body_expression_substitutions(
        &self,
        graph: &AdviceGraph,
        source: &str,
        context: &tcl_syntax::expr::parser::ExprParseContext,
    ) -> Option<tcl_syntax::expr::LiveExpressionSubstitutions> {
        match self.context.policy {
            AdviceNamingPolicy::Logical(input) if input.has_logical_source_name_context() => {
                tcl_syntax::expr::checked_source_expression_substitutions(source, context)
            }
            AdviceNamingPolicy::Logical(_) => None,
            AdviceNamingPolicy::Native(_) => {
                if graph.c_source_namespace.is_some() && !graph.native_baseline {
                    tcl_syntax::expr::checked_source_expression_substitutions(source, context)
                } else {
                    tcl_syntax::expr::checked_expression_substitutions(source, context)
                }
            }
        }
    }

    fn script(&mut self, graph: &mut AdviceGraph, span: Span, depth: usize) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let plan = self
            .context
            .origin
            .original_script_words_in(
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
        self.context.observe_publication_proposal(graph);
        self.inventory
            .as_mut()
            .unwrap_or(&mut self.authored_prefixes)
            .retain_arity_prefix(graph, &native.words);
        let retained = self.command_inner(graph, native, depth);
        self.context.observe_publication_proposal(graph);
        retained
    }

    fn retain_native_procedure_call(
        &mut self,
        graph: &AdviceGraph,
        native: &NativeScriptCommandWords,
        written: &[super::SourceAdviceWord],
    ) {
        if matches!(self.context.policy, AdviceNamingPolicy::Native(_)) {
            drop(
                AdviceInvocationContext::retain_original_procedure_call_receipt(
                    self.inventory
                        .as_mut()
                        .unwrap_or(&mut self.authored_prefixes),
                    graph,
                    &native.words,
                    written,
                ),
            );
        }
    }

    fn command_inner(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        depth: usize,
    ) -> Option<()> {
        self.mark_child_header(native)?;
        self.retain_original_body_word_syntax(graph, native);
        if self.retain_logical_body_constructor_calls(graph, native) {
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
        self.retain_native_procedure_call(graph, native, &written);
        // A walk retaining Logical source-word recipes, as
        // `retain_logical_navigation` above does, records every call capture.
        // Other walks record one only where they hold a recipe at its head.
        let owns_logical_recipe = self.inventory.is_some() && self.logical_scope.is_some();
        // Syntax ownership survives unavailable handler/effect inventory.
        let tape = self
            .inventory
            .as_mut()
            .unwrap_or(&mut self.authored_prefixes);
        self.context
            .retain_native_procedure_may_command(tape, graph, &native.words, &written);
        let Some(head) = written
            .first()
            .and_then(|word| word.input.as_ref())
            .cloned()
        else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        if owns_logical_recipe {
            OriginalSourceTransitionAdviceTape::retain_logical_procedure_call(
                tape,
                graph,
                &native.words,
            );
        } else {
            tape.retain_recipe_logical_procedure_call(graph, &native.words);
        }
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
        self.context.retain_arity_debt(graph, invocation, &schema)?;
        note_source_point_body_walk(
            self.context,
            invocation.native,
            self.catalogue_body.is_some() || self.member_catalogue_body.is_some(),
        );
        // Catalogue walks record syntax under conditional applicability; they
        // cannot issue an execution-point graph or veto an entered-body walk.
        if self.catalogue_body.is_none() && self.member_catalogue_body.is_none() {
            self.context
                .observe_source_point(graph, invocation.native, &schema);
        }
        self.retain_requested_body_class_relation(graph, invocation, &schema);
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
        self.finish_original_body_command(graph, native, invocation, &schema, depth)
    }

    fn retain_logical_body_constructor_calls(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
    ) -> bool {
        let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body) else {
            return false;
        };
        matches!(self.context.policy, AdviceNamingPolicy::Logical(_))
            && self.context.retain_original_body_constructor_calls(
                tape,
                graph,
                &native.words,
                (
                    body,
                    SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
                ),
            )
    }

    fn retain_original_body_word_syntax(
        &mut self,
        graph: &AdviceGraph,
        native: &NativeScriptCommandWords,
    ) {
        let syntax_body = self
            .member_catalogue_body
            .as_ref()
            .map(|body| super::OriginalSourceCatalogueBody::Member(Arc::clone(body)))
            .or_else(|| self.catalogue_body.clone())
            .or_else(|| {
                self.source_body
                    .as_ref()
                    .map(|body| super::OriginalSourceCatalogueBody::Script(Arc::clone(body)))
            });
        if let Some(body) = syntax_body {
            let tape = self
                .inventory
                .as_mut()
                .unwrap_or(&mut self.authored_prefixes);
            tape.retain_logical_body_words(self.context, &native.words, body.clone());
            tape.retain_logical_absolute_navigation(graph, &native.words, body);
        }
        if let (Some(tape), Some(scope)) = (&mut self.inventory, &self.logical_scope) {
            tape.retain_logical_summary_scope(&native.words, scope);
            tape.retain_logical_navigation(graph, &native.words, scope);
        }
    }

    fn finish_original_body_command(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        let immediate = (self
            .context
            .authored_roles(schema)
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            || self.context.normal_case_source_layout(schema).is_some())
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY)
            && schema.authored_source_procedure_arguments().is_none();
        let advice = boxed_body_advice(self.context, invocation, graph, schema)?;
        if self.expressions(graph, &advice, schema, depth).is_none() {
            graph.widen(&native.words);
        }
        if !immediate {
            self.member_catalogue_possible_bodies(graph, &advice, schema, depth.checked_add(1)?);
        } else if self
            .regions(graph, &advice, schema, depth.checked_add(1)?)
            .is_none()
        {
            // A `TclOO` definition script runs in a new frame in its definer's
            // namespace (`::oo::define`, `::oo::objdefine` or a configurable
            // support namespace), and the definition-member inventory owns its
            // members. Its remaining reach into this walk — `uplevel`/`upvar`
            // to the caller frame and absolute names — is that of an
            // unresolved command, so it records the same uncertainty instead
            // of withdrawing every namespace's path and export advice.
            if schema
                .authored_source_definition_body_grammar()
                .is_some_and(|grammar| {
                    grammar.family == tcl_registry::definer::DefinerFamily::TclOo
                })
            {
                graph.record_uncertainty(&native.words);
            } else {
                graph.widen(&native.words);
            }
        }
        Some(())
    }
    /// A readonly member-body catalogue also follows possible same-invocation
    /// bodies whose complete role layout is unavailable (a `switch` with a
    /// computed subject or option). The parent operation keeps its recorded
    /// uncertainty; every candidate runs on its own branch, so no candidate
    /// effect reaches the parent walk. Deferred, frame-shifting, other-
    /// interpreter and declaration bodies are never followed.
    fn member_catalogue_possible_bodies(
        &mut self,
        graph: &AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) {
        if self.member_catalogue_body.is_none()
            || self.inventory.is_some()
            || self.source_body.is_some()
            || depth > MAX_DEPTH
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.traits.intersects(
                tcl_registry::Traits::DEFERS_BODY | tcl_registry::Traits::DEFINES_PROCEDURE,
            )
            || schema.authored_source_procedure_arguments().is_some()
            || schema.authored_source_definition_body_grammar().is_some()
        {
            return;
        }
        let Some(words) = boxed_region_words(self.context, advice) else {
            return;
        };
        let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(words.arguments().get(ordinal)?.literal_bytes()?).ok()
        });
        if interpreter != tcl_registry::world_effect::InterpreterScope::Current {
            return;
        }
        let context = self.context.context;
        let mut bodies = words
            .conditional_source_case_bodies(context)
            .filter(|case| {
                case.matches_source(self.context.origin.source_image(), self.context.config)
            })
            .map_or_default(|case| case.bodies().to_vec());
        bodies.extend(words.source_script_bodies_for(
            context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        ));
        bodies.sort_by_key(|body| (body.content_span().start(), body.content_span().end()));
        bodies.dedup_by_key(|body| body.content_span());
        for body in bodies {
            // Each candidate's own written operand must run before the
            // invocation returns; a stored callback body never qualifies.
            let same_invocation = words
                .operands()
                .iter()
                .position(|operand| {
                    operand.as_ref().and_then(
                        crate::registry_invocation::source_structure::OriginalOperandSource::word,
                    ) == Some(body.original_container())
                })
                .and_then(|argument| {
                    words
                        .with_source_schema(context, |schema| {
                            schema.authored_source_script_timing_at(argument)
                        })
                        .flatten()
                })
                == Some(tcl_registry::ScriptTiming::SameInvocation);
            if !same_invocation
                || !body.matches_source(self.context.origin.source_image(), self.context.config)
                || !body.matches_context(context)
            {
                continue;
            }
            let mut branch = graph.clone();
            branch.record_uncertainty(advice.original_words());
            // A malformed or over-deep candidate only withholds its own rows.
            let _ = self.script(&mut branch, body.content_span(), depth);
        }
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
        // The independent readonly catalogue owns Jim source syntax even when
        // this walk has no execution/mutation-completeness inventory.
        let jim_parent = self.source_body.as_ref().or(match &self.catalogue_body {
            Some(super::OriginalSourceCatalogueBody::Script(body)) => {
                // A syntax-only Logical catalogue cannot borrow the enclosing
                // graph's namespace. Its own selected body scope must agree.
                let owned = match self.context.policy {
                    AdviceNamingPolicy::Logical(input) => {
                        self.logical_scope.as_ref().is_some_and(|scope| {
                            scope.matches_input(input)
                                && scope.body_arc() == body
                                && graph.logical_namespace.as_ref()
                                    == Some(scope.namespace_recipe())
                        })
                    }
                    AdviceNamingPolicy::Native(_) => true,
                };
                owned.then_some(body)
            }
            _ => None,
        });
        let authored_jim = jim_parent.and_then(|body| {
            self.context
                .authored_jim_class_source(graph, invocation, schema, Some(body))
        });
        let class = self
            .inventory
            .as_mut()
            .zip(self.source_body.as_ref())
            .and_then(|(tape, body)| {
                let obligation = body_class_obligation(self.context, graph)?;
                self.context.source_class_declaration_in_body(
                    tape,
                    graph,
                    invocation,
                    schema,
                    (body, obligation, self.logical_scope.as_ref()),
                )
            });
        // A TclOO definer whose script only declares members publishes its
        // class and nothing else, as at the top level, where such a region is
        // skipped; its blanket footprint would mark every later command of
        // the body as following an unknown mutation.
        let residual_footprint = !self.definition_declares_members_only(graph, invocation, schema);
        self.context.apply_source_body_lookup_effects(
            graph,
            command,
            invocation,
            schema,
            residual_footprint,
        );
        AdviceInvocationContext::install_source_class_handle(graph, class_handle);
        if let Some(tape) = &mut self.inventory {
            AdviceInvocationContext::install_source_class(tape, graph, class);
        }
        AdviceInvocationContext::install_authored_jim_class(
            self.inventory
                .as_mut()
                .unwrap_or(&mut self.authored_prefixes),
            graph,
            authored_jim,
        );
    }
    /// Answer a class-relation query whose parent this body walk retains,
    /// under the body ownership its class declaration and advice carry.
    fn retain_requested_body_class_relation(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        // A catalogue walk reads syntax on a branch graph of its own and
        // owns no parent advice, so it neither answers nor refuses a query.
        if self.inventory.is_none()
            || self.source_body.is_none()
            || self.catalogue_body.is_some()
            || self.member_catalogue_body.is_some()
        {
            return;
        }
        let owner = self.source_body.as_ref().and_then(|body| {
            Some((
                body,
                body_class_obligation(self.context, graph)?,
                self.logical_scope.as_ref(),
            ))
        });
        self.context.retain_requested_class_relation_with(
            invocation,
            graph,
            schema,
            owner,
            |advice| {
                if let Some(body) = &self.source_body {
                    Self::retain_source_body(self.context.policy, advice, body);
                    if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
                        advice.logical_scope.clone_from(&self.logical_scope);
                    }
                }
            },
        );
    }
    fn retain_child_schema(
        &mut self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        self.retain_original_body_catalogues(graph, invocation, schema)?;
        if let (Some(tape), None) = (&mut self.inventory, &self.source_body) {
            // Immediate original operands use their root point, not a made-up
            // body descriptor or a future frame's applicability.
            if graph.blocks_registry_source(invocation.head) {
                tape.registry_barriers
                    .insert(invocation.native.first()?.span().start());
                return Some(());
            }
            return self.context.retain_source_inventory(
                tape,
                invocation,
                graph,
                schema,
                None,
                self.logical_scope.as_ref().map_or(
                    super::logical_navigation::LogicalCallbackScope::Unavailable,
                    super::logical_navigation::LogicalCallbackScope::Declared,
                ),
            );
        }
        let retained_producer = if let Some(body) = &self.source_body {
            let mut advice = self.context.schema_advice(invocation, graph, schema)?;
            Self::retain_source_body(self.context.policy, &mut advice, body);
            if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
                advice.logical_scope.clone_from(&self.logical_scope);
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
        self.context.retain_logical_callback_candidates(
            tape,
            invocation,
            graph,
            schema,
            self.logical_scope.as_ref().map_or(
                super::logical_navigation::LogicalCallbackScope::Unavailable,
                super::logical_navigation::LogicalCallbackScope::Declared,
            ),
        );
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
            advice.logical_scope.clone_from(&self.logical_scope);
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

    fn retain_original_body_catalogues(
        &mut self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        // Retain possible case syntax from the actual operand walk separately
        // from an ordinary invocation or any entered-body inventory.
        if matches!(self.context.policy, AdviceNamingPolicy::Native(_))
            && self.source_body.is_none()
            && self.catalogue_body.is_none()
            && self.member_catalogue_body.is_none()
            && schema.authored_source_case_invocation().is_some()
        {
            let offset = invocation.native.first()?.span().start();
            if self.authored_prefixes.conditional_case_syntax.len() < MAX_COMMANDS {
                let receipt = (!graph.blocks_registry_source(invocation.head))
                    .then(|| self.context.schema_advice(invocation, graph, schema))
                    .flatten();
                super::source_class::merge_receipt(
                    &mut self.authored_prefixes.conditional_case_syntax,
                    offset,
                    receipt,
                );
            }
        }
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
                    let contextual_self = body
                        .receiver_grammar(self.context.context)
                        .is_some_and(|grammar| grammar.family.members_are_tcloo_methods())
                        && schema
                            .semantics
                            .traits
                            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
                        && self.context.context.commands().method_dispatch_keyword(
                            std::str::from_utf8(invocation.head.bytes()).ok()?,
                        ) == Some(tcl_registry::MethodDispatchKind::SelfDispatch);
                    if (selected.namespace_route()
                        != tcl_syntax::naming::NativeCommandNamespaceRoute::Root
                        && !contextual_self)
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
mod logical_body_inventory_tests;

#[cfg(test)]
mod original_empty_coverage_tests;

#[cfg(test)]
mod original_source_expression_topology_tests;

#[cfg(test)]
mod original_expression_purpose_frontier_tests;

#[cfg(test)]
mod original_expression_catalogue_tests;

#[cfg(test)]
mod ordered_condition_uncertainty_tests;

#[cfg(test)]
mod possible_occupant_lookup_tests;

#[cfg(test)]
mod normal_case_mutation_coverage_tests;

#[cfg(test)]
fn note_source_point_body_walk(
    context: &AdviceInvocationContext<'_>,
    original: &[NativeWord],
    catalogue: bool,
) {
    context.note_source_point_body_walk(original, catalogue);
}

#[cfg(not(test))]
const fn note_source_point_body_walk(_: &AdviceInvocationContext<'_>, _: &[NativeWord], _: bool) {}
