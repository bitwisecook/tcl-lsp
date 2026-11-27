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

//! Expression operands: operators, function calls, implicit math and expression truth.

use super::super::binding_projection::source_binding;
use super::super::command_identity::CommandAllocationSite;
use super::super::executed_script_source::ExecutedScriptSource;
use super::super::invocation_binding::SourceObserverExecution;
use super::super::namespace_context::SourceNamespaceKey;
use super::super::source_origin::{MaterialisedSourceKind, SourceOriginId};
use super::super::variable_access::SourceVariableEvaluationOwner;
use super::super::{
    FixedMathObservation, ModuleCommandBindings, SourceCommandBindings, SourceCommandTarget,
    SourceExecutionContext, SourceInvocationBinding, SourceNativeInvocation, SourceOutcomes,
    SourceScriptOperands, expression_preparation, literal_object_pool, native_result,
    object_callbacks, source_representation,
};
use super::body_flow::{authored_braced_operand, frozen_script_operands};
use super::outcomes::opaque_source_invocation;
use std::hash::Hash;
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// Reached implicit expression dispatch, sampled after its argument evaluations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceImplicitMathInvocation {
    /// Exact source instance owning the expression AST.
    pub origin: Arc<SourceOriginId>,
    /// AST function-call start in that source instance.
    pub site: u32,
    /// Function name retained by the expression parser.
    pub function: String,
    /// Actual lookup evidence also proves execution observers absent.
    pub unobserved: bool,
    pub(crate) object_callback_effects: Option<object_callbacks::ObjectCallbackEffects>,
    /// Actual mutable command-table lookup; absent for a fixed-table engine.
    pub command_binding: Option<SourceInvocationBinding>,
    /// Actual fixed table retained from interpreter entry, independently of names.
    pub fixed_functions: Option<tcl_runtime_api::native_compilation::NativeMathFunctionTable>,
    /// Actual interpreter owner plus fixed registration identities for emitted guards.
    pub fixed_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
}

impl SourceImplicitMathInvocation {
    /// Actual original operand objects have closed native conversion effects.
    /// Implementation identity and successful value analysis are independent.
    pub(crate) fn object_callback_effects_closed(&self) -> bool {
        self.object_callback_effects == Some(object_callbacks::ObjectCallbackEffects::Closed)
    }
}

impl SourceCommandBindings {
    pub(super) fn walk_concatenated_expression(
        &mut self,
        offset: usize,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(values) = frozen_script_operands(native.invocation.arguments(), offset) else {
            return opaque_source_invocation(state);
        };
        if values.len() == 1 && authored_braced_operand(native, offset, values[0]).is_some() {
            return self.walk_expression_operand(offset, native.script_operands(), state, context);
        }
        self.walk_mapped_expression(offset, &values, native.script_operands(), state, context)
    }

    pub(super) fn walk_expression_operand(
        &mut self,
        index: usize,
        operands: SourceScriptOperands<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let Some(value) = operands.arguments.literal_at(index) else {
            return opaque_source_invocation(state);
        };
        let word = operands.written_word(index);
        let Some(word) = word else {
            return self.walk_materialised_source(
                value,
                index..index + 1,
                MaterialisedSourceKind::Expression,
                state,
                context,
            );
        };
        let (text, source, offset) = match word {
            tcl_source::words::WordExpr::BracedLiteral { text, source } if text == value => {
                (text.as_str(), source, 1)
            }
            tcl_source::words::WordExpr::Literal { text, source } => {
                let effective = source_effective_words(
                    std::slice::from_ref(word),
                    state.baseline.dialect,
                    Some((&state.source_variables, context.registry)),
                );
                if text != value
                    || effective
                        .first()
                        .and_then(|word| word.as_registry_word().literal())
                        != Some(text.as_str())
                {
                    // Changed decoded text cannot carry original source spans.
                    return self.walk_materialised_source(
                        value,
                        index..index + 1,
                        MaterialisedSourceKind::Expression,
                        state,
                        context,
                    );
                }
                (text.as_str(), source, 0)
            }
            _ => {
                return self.walk_materialised_source(
                    value,
                    index..index + 1,
                    MaterialisedSourceKind::Expression,
                    state,
                    context,
                );
            }
        };
        let owner = state.current_source_origin.as_ref().map(|source| {
            SourceVariableEvaluationOwner::NativeExpression {
                invocation: CommandAllocationSite {
                    source: Arc::clone(source),
                    offset: context.invocation_offset,
                },
                parent: context.variable_read_owner.cloned().map(Arc::new),
            }
        });
        let Some(origin) = state.current_source_origin.clone() else {
            return opaque_source_invocation(state);
        };
        let Some(expression_source) = ExecutedScriptSource::contiguous(
            Arc::clone(&origin),
            text,
            source.span.start() + offset,
        ) else {
            return opaque_source_invocation(state);
        };
        self.walk_prepared_expression(
            CommandAllocationSite {
                source: origin,
                offset: context.invocation_offset,
            },
            expression_source,
            state,
            SourceExecutionContext {
                variable_read_owner: owner.as_ref(),
                ..context
            },
        )
    }

    pub(in super::super) fn walk_expression(
        &mut self,
        expression: &crate::expr_ast::ExprNode,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use crate::expr_ast::ExprNode;
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        if tcl_source::limits::MAX_SOURCE_NEST_DEPTH.exceeded(context.depth) {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        }
        let nested = SourceExecutionContext {
            control: (context)
                .control
                .with_source_purpose(tcl_lexer::SourceSegmentationPurpose::NestedScript),
            depth: context.depth + 1,
            ..context
        };
        let coercion = matches!(
            expression,
            ExprNode::Unary { .. } | ExprNode::Binary { .. } | ExprNode::Ternary { .. }
        )
        .then(|| {
            expression_preparation::RuntimeExpressionCoercion::capture(expression, state, context)
        })
        .flatten();
        let mut outcomes = match expression {
            ExprNode::Command { text, start, .. } => {
                if let Some(script) = text
                    .strip_prefix('[')
                    .and_then(|text| text.strip_suffix(']'))
                {
                    self.walk_source(
                        script,
                        base.saturating_add(*start).saturating_add(1),
                        state,
                        &nested,
                    )
                } else {
                    state.mark_opaque_binding_mutation();
                    SourceOutcomes::invocation(state, Route::Unknown)
                }
            }
            ExprNode::Binary { op, left, right } => {
                self.walk_binary_expression(*op, left, right, base, state, nested)
            }
            ExprNode::Unary { operand, .. } => self.walk_expression(operand, base, state, nested),
            ExprNode::Var { text, start, .. } => {
                self.walk_expression_variable(expression, text, *start, base, state, nested)
            }

            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => self.walk_ternary_expression(
                condition,
                true_branch,
                false_branch,
                base,
                state,
                nested,
            ),
            ExprNode::Call {
                function,
                args,
                start,
                ..
            } => self.walk_expression_call(
                function,
                args,
                base,
                base.saturating_add(*start),
                state,
                nested,
            ),
            ExprNode::Raw { .. } => {
                state.mark_opaque_binding_mutation();
                SourceOutcomes::invocation(state, Route::Unknown)
            }
            ExprNode::String { text, start, .. } => {
                self.walk_expression_string(text, base.saturating_add(*start), state, nested)
            }
            _ => SourceOutcomes::normal(state),
        };
        if let Some(normal) = &mut outcomes.normal
            && let Some(coercion) = coercion
        {
            coercion.apply(normal);
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_expression_call(
        &mut self,
        function: &str,
        args: &[crate::expr_ast::ExprNode],
        base: u32,
        site: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        let mut input = source_representation::SourceMathInput {
            arity: args.len(),
            closed: false,
        };
        // Each operand's own read receipt and the representation epoch it was
        // taken under.
        let mut receipts = Vec::with_capacity(args.len());
        for argument in args {
            let Some(continuing) = outcomes.normal.take() else {
                break;
            };
            *state = *continuing;
            let evaluated = self.walk_expression(argument, base, state, context);
            if args.len() == 1 {
                input.closed = operator_result(argument)
                    || source_representation::scalar_math_input_is_closed(argument, &evaluated);
            } else {
                receipts.push((
                    source_representation::scalar_math_input_is_closed(argument, &evaluated),
                    evaluated
                        .normal
                        .as_deref()
                        .and_then(|normal| normal.source_variables.representation_epoch),
                ));
            }
            outcomes.join(&evaluated);
        }
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            if args.len() != 1 {
                // An operand read before a later operand ran keeps its
                // receipt only while no representation changed since:
                // `max($a, $b)` reads both and then converts them.
                let current = state.source_variables.representation_epoch;
                input.closed = args.iter().zip(&receipts).all(|(argument, (closed, epoch))| {
                    operator_result(argument)
                        || literal_object_pool::SourceOrdinaryLiteralObject::capture_expression_literal(
                            argument, state,
                        )
                        .is_some()
                        || (*closed && epoch.is_some() && *epoch == current)
                }) && receipts.len() == args.len();
            }
            outcomes.join(&self.walk_math_function(function, input, site, state, context));
        }
        outcomes
    }

    fn walk_expression_string(
        &mut self,
        text: &str,
        offset: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let segments = tcl_source::segment::segment_commands_with_offset_and_config(
            text,
            offset,
            context.config.nested(),
        );
        if let Some(segment) = segments.first() {
            let tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(text).with_base(offset, 0, 0),
                context.config,
                segment,
            );
            if let Some(word) = tokens.words().first() {
                self.walk_substitutions(word, text, offset, state, context)
            } else {
                SourceOutcomes::normal(state)
            }
        } else {
            SourceOutcomes::normal(state)
        }
    }

    fn walk_binary_expression(
        &mut self,
        op: crate::expr_ast::BinOp,
        left: &crate::expr_ast::ExprNode,
        right: &crate::expr_ast::ExprNode,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use crate::expr_ast::BinOp;
        let short = match op {
            BinOp::And | BinOp::WordAnd => Some(false),
            BinOp::Or | BinOp::WordOr => Some(true),
            _ => None,
        };
        let condition_coercion =
            short.and_then(|_| expression_preparation::root_result_coercion(left, state, context));
        let mut outcomes = self.walk_expression(left, base, state, context);
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            if let Some(coercion) = condition_coercion {
                coercion.apply(state);
            }
            let truth = source_completed_expression_truth(left, outcomes.normal_value.as_deref())
                .or_else(|| source_expression_truth(left, state, context.registry));
            if short.is_some() && truth == short {
                outcomes.join(&SourceOutcomes::normal(state));
            } else {
                let incoming = state.clone();
                outcomes.join(&self.walk_expression(right, base, state, context));
                if short.is_some() && truth.is_none() {
                    outcomes.join(&SourceOutcomes::normal(&incoming));
                }
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_ternary_expression(
        &mut self,
        condition: &crate::expr_ast::ExprNode,
        true_branch: &crate::expr_ast::ExprNode,
        false_branch: &crate::expr_ast::ExprNode,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let condition_coercion =
            expression_preparation::root_result_coercion(condition, state, context);
        let mut outcomes = self.walk_expression(condition, base, state, context);
        if let Some(continuing) = outcomes.normal.take() {
            *state = *continuing;
            if let Some(coercion) = condition_coercion {
                coercion.apply(state);
            }
            match source_expression_truth(condition, state, context.registry) {
                Some(true) => {
                    outcomes.join(&self.walk_expression(true_branch, base, state, context));
                }
                Some(false) => {
                    outcomes.join(&self.walk_expression(false_branch, base, state, context));
                }
                None => {
                    let mut alternative = state.clone();
                    outcomes.join(&self.walk_expression(true_branch, base, state, context));
                    outcomes.join(&self.walk_expression(
                        false_branch,
                        base,
                        &mut alternative,
                        context,
                    ));
                }
            }
        }
        outcomes.publish(state);
        outcomes
    }

    fn walk_math_function(
        &mut self,
        function: &str,
        input: source_representation::SourceMathInput,
        site: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        use tcl_registry::mathfunc::NativeMathFunctionDispatch;
        let dispatch = state
            .baseline
            .dialect
            .or_else(|| {
                context
                    .registry
                    .profile()
                    .map(tcl_native_model::InvocationDialect::of_profile)
            })
            .and_then(tcl_registry::mathfunc::native_function_dispatch);
        let Some(dispatch) = dispatch else {
            state.mark_opaque_binding_mutation();
            return SourceOutcomes::invocation(state, Route::Unknown);
        };
        self.retain_math_object_callback_effects(state, site, input.closed);
        match dispatch {
            NativeMathFunctionDispatch::FixedTable => {
                self.record_fixed_math_diagnostic_presence(function, site, state);
                if let Some(origin) = &state.current_source_origin {
                    let table = (!state.opaque_domain)
                        .then(|| {
                            state
                                .baseline
                                .native_entry
                                .as_ref()
                                .and_then(|entry| entry.math_functions.clone().map(|table| tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite { interpreter: entry.interpreter, table }))
                        })
                        .flatten();
                    let observation = FixedMathObservation {
                        declaration_preview: self.declaration_preview_depth != 0,
                        frame: state.variable_frame.clone(),
                        namespace: context.namespace_identity(),
                        prerequisite: table.clone(),
                    };
                    let observations = self
                        .implicit_fixed_math_invocations
                        .entry((Arc::clone(origin), site, function.to_owned()))
                        .or_default();
                    if !observations.contains(&observation) {
                        observations.push(observation);
                    }
                    if let Some(prerequisite) = table.as_ref()
                        && let tcl_runtime_api::native_compilation::NativeMathFunctionResolution::Present(row) = prerequisite.table.lookup(function)
                        && let Some(identity) = row.registry_identity.as_deref()
                        && let Some(dialect) = state.baseline.dialect
                        && let Some(protocol) = context.registry.fixed_scalar_math_protocol(
                            dialect, identity, row.arity, input.arity,
                        )
                    {
                        return source_representation::scalar_math_result(state, protocol, input.closed);
                    }
                }
                state.mark_opaque_binding_mutation();
                return SourceOutcomes::invocation(state, Route::Unknown);
            }
            NativeMathFunctionDispatch::CommandTable => {}
        }
        self.walk_command_table_math_function(function, input, site, state, context)
    }

    fn walk_command_table_math_function(
        &mut self,
        function: &str,
        input: source_representation::SourceMathInput,
        site: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let absolute = tcl_registry::mathfunc::qualified_name(function);
        let mut effective = vec![
            crate::registry_invocation::EffectiveInvocationWord::Literal(
                tcl_syntax::naming::unroot_rooted_key(&absolute)
                    .unwrap_or(&absolute)
                    .to_owned(),
            ),
        ];
        effective.extend(std::iter::repeat_n(
            crate::registry_invocation::EffectiveInvocationWord::Dynamic,
            input.arity,
        ));
        let context = SourceExecutionContext {
            invocation_offset: site,
            ..context
        };
        let mut scope = self.prepare_command_observers(&effective, state, &context);
        let Some(continuing) = scope.entry.normal.take() else {
            scope.entry.publish(state);
            return scope.entry;
        };
        *state = *continuing;
        let mut outcomes = std::mem::take(&mut scope.entry);
        let mut input = input;
        if scope.was_observed {
            input.closed = false;
            self.retain_math_object_callback_effects(state, site, false);
        }
        let namespace_key = context.namespace_key.cloned().or_else(|| {
            state
                .baseline
                .native_entry
                .is_none()
                .then(|| SourceNamespaceKey::authored(context.namespace))
        });
        let mut binding =
            namespace_key
                .as_ref()
                .map_or_else(SourceInvocationBinding::default, |namespace| {
                    source_binding(
                        state,
                        tcl_syntax::naming::unroot_rooted_key(&absolute).unwrap_or(&absolute),
                        namespace,
                    )
                });
        binding.entered_execution_observer = if scope.was_observed {
            SourceObserverExecution::MayObserved
        } else {
            SourceObserverExecution::Unobserved
        };
        binding.object_callback_effects = Some(if input.closed {
            object_callbacks::ObjectCallbackEffects::Closed
        } else {
            object_callbacks::ObjectCallbackEffects::Unknown
        });
        if let Some(origin) = &state.current_source_origin {
            let bindings = self
                .implicit_math_invocations
                .entry((Arc::clone(origin), site, function.to_owned()))
                .or_default();
            if !bindings.contains(&binding) {
                bindings.push(binding.clone());
            }
        }
        let Some(target) = binding.proved_target() else {
            state.mark_opaque_binding_mutation();
            let dispatched = SourceOutcomes::invocation(state, Route::Unknown);
            outcomes.join(&self.finish_command_observer_scope(dispatched, &scope, &context));
            outcomes.publish(state);
            return outcomes;
        };
        let dispatched = if target.registry_backed {
            Self::walk_registry_math_function(target, input, state, context)
        } else {
            self.walk_document_target(site, None, &effective, state, target, context)
        };
        outcomes.join(&self.finish_command_observer_scope(dispatched, &scope, &context));
        outcomes.publish(state);
        outcomes
    }

    fn walk_registry_math_function(
        target: &SourceCommandTarget,
        input: source_representation::SourceMathInput,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        let mut arguments = target
            .prepended
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        arguments.extend(std::iter::repeat_n(
            tcl_registry::InvocationWord::DynamicNonOption,
            input.arity,
        ));
        let mut invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&target.command),
            &arguments,
        );
        if let Some(dialect) = state.baseline.dialect {
            invocation = invocation.with_dialect(dialect);
        }
        let route = context
            .registry
            .invocation_completion_route_in_frame(
                &target.command,
                invocation.arguments(),
                invocation
                    .arguments()
                    .dialect()
                    .and_then(tcl_native_model::InvocationDialect::authoring_query)
                    .map(|query| query.with_realm(context.realm())),
                state.source_variables.alias_frame(),
            )
            .unwrap_or(Route::Unknown);
        if let Some(facts) = tcl_registry::model::semantic::resolve_structured_invocation_in_realm(
            context.registry,
            context
                .registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile),
            invocation,
            context.realm(),
        )
        .resolved()
        .map(|resolved| resolved.facts())
            && facts.effects.callback().kinds == tcl_registry::CallbackKinds::NONE
            && facts.effects.accesses().iter().all(|access| {
                access.mode == tcl_registry::EffectAccessMode::Read
                    || !matches!(
                        access.domain,
                        tcl_registry::WorldStateDomain::CommandBindings
                            | tcl_registry::WorldStateDomain::NamespaceLookup
                            | tcl_registry::WorldStateDomain::VariableStore
                    )
            })
        {
            if let Some(protocol) = facts.normal_scalar_math_protocol(invocation.arguments()) {
                return source_representation::scalar_math_result(state, protocol, input.closed);
            }
            if facts
                .numeric_object_conversion_arguments(invocation.arguments())
                .is_some()
                && !input.closed
            {
                // The selected implementation remains current for this call,
                // but its original operands can mutate interpreter worlds
                // through updateStringProc/freeIntRepProc during conversion.
                state.mark_opaque_binding_mutation();
            }
            return SourceOutcomes::invocation(state, route);
        }
        state.mark_opaque_binding_mutation();
        SourceOutcomes::invocation(state, route)
    }
}

/// An operator leaves a fresh or already-converted stock numeric object, so
/// a later numeric conversion of it runs no object hooks. A ternary passes
/// its selected operand through unconverted.
fn operator_result(node: &crate::expr_ast::ExprNode) -> bool {
    matches!(
        node,
        crate::expr_ast::ExprNode::Binary { .. } | crate::expr_ast::ExprNode::Unary { .. }
    )
}

pub(in super::super) fn parse_source_expression(
    source: &str,
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> crate::expr_ast::ExprNode {
    if let Some(dialect) = state.baseline.dialect {
        crate::expr_parser::parse_expr_with_syntax_context(
            source,
            &dialect.expression_parse_context(registry.profile()),
        )
    } else {
        crate::expr_parser::parse_expr_for_profile(source, registry.profile())
    }
}

pub(in super::super) fn source_operand_truth(
    index: usize,
    operands: SourceScriptOperands<'_>,
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> Option<bool> {
    let value = operands.arguments.literal_at(index)?;
    let expression = parse_source_expression(value, state, registry);
    source_expression_truth(&expression, state, registry)
}

/// Only canonical Boolean command results can supply this model decision.
fn source_completed_expression_truth(
    expression: &crate::expr_ast::ExprNode,
    value: Option<&native_result::EvaluatedSourceValue>,
) -> Option<bool> {
    let value = value?;
    if value.numeric.is_some() || !matches!(value.text.as_str(), "0" | "1") {
        return None;
    }
    let mut expression = expression;
    let mut negated = false;
    while let crate::expr_ast::ExprNode::Unary {
        op: crate::expr_ast::UnaryOp::Not,
        operand,
    } = expression
    {
        expression = operand;
        negated = !negated;
    }
    if !matches!(expression, crate::expr_ast::ExprNode::Command { .. }) {
        return None;
    }
    let truth = tcl_syntax::boolean::parse_boolean_strict(&value.text)?;
    Some(truth ^ negated)
}

fn source_expression_truth(
    expression: &crate::expr_ast::ExprNode,
    state: &ModuleCommandBindings,
    registry: &CommandRegistry,
) -> Option<bool> {
    use crate::expr_ast::ExprNode;
    let mut env = crate::tcl_expr_eval::Env::new();
    let mut pending = vec![expression];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::Var { text, name, .. } => {
                let value = state
                    .source_variables
                    .substitution_literal_value(text, registry)?;
                env.insert(name.clone(), crate::tcl_expr_eval::EnvValue::text(value));
            }
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => pending.extend([
                condition.as_ref(),
                true_branch.as_ref(),
                false_branch.as_ref(),
            ]),
            // Calls can name replaced math commands; folding their nominal
            // builtin implementation would lose the source binding proof.
            _ => return None,
        }
    }
    let mut policy = crate::tcl_expr_eval::FoldPolicy::from_registry(registry);
    if let Some(dialect) = state.baseline.dialect {
        policy = policy.with_invocation_dialect(dialect);
    }
    crate::tcl_expr_eval::analyse_tcl_expr_with_resolved_math_bindings(
        expression,
        &env,
        policy,
        &|_, _| None,
        None,
    )
    .and_then(|evaluation| match evaluation.value {
        crate::tcl_expr_eval::TclValue::Float(value) if value.is_nan() => None,
        value => Some(value.is_truthy()),
    })
}

pub(in super::super) fn source_effective_words(
    words: &[tcl_source::words::WordExpr],
    dialect: Option<tcl_native_model::InvocationDialect>,
    context: Option<(&crate::var_resolve::ResolveContext, &CommandRegistry)>,
) -> Vec<crate::registry_invocation::EffectiveInvocationWord> {
    use crate::registry_invocation::EffectiveInvocationWord;
    words
        .iter()
        .map(|word| {
            if let (
                tcl_source::words::WordExpr::Variable { spelling, .. },
                Some((variables, registry)),
            ) = (word, context)
                && let Some(value) = variables.substitution_literal_value(spelling, registry)
            {
                return EffectiveInvocationWord::Literal(value.to_owned());
            }
            dialect.map_or_else(
                || match crate::registry_invocation::invocation_word(word) {
                    tcl_registry::InvocationWord::Literal(value) => {
                        EffectiveInvocationWord::Literal(value.to_owned())
                    }
                    tcl_registry::InvocationWord::KnownBytes(value) => {
                        EffectiveInvocationWord::from_bytes(value)
                    }
                    tcl_registry::InvocationWord::Dynamic
                    | tcl_registry::InvocationWord::DynamicNonOption => {
                        EffectiveInvocationWord::Dynamic
                    }
                    tcl_registry::InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                    tcl_registry::InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                    tcl_registry::InvocationWord::ArrayElementName { root } => {
                        EffectiveInvocationWord::ArrayElementName {
                            root: root.to_owned(),
                        }
                    }
                },
                |dialect| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                },
            )
        })
        .collect()
}
