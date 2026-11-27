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
    AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation, capture_transition,
};
#[cfg(test)]
use super::{original_words, registry_words};
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, NativeWord, Span};
use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};

mod commands;
mod expressions;
mod regions;

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
    dialect: tcl_native_model::InvocationDialect,
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
