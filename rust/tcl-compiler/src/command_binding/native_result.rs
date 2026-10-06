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
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Result values are captured at native completion, before subsequent argv effects.

use super::{
    Arc, ModuleCommandBindings, SourceCommandTarget, SourceExecutionContext, SourceOutcomes,
};
use crate::registry_invocation::EffectiveInvocationWord;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::native_result::NativeResultSelection;

pub(super) fn retain_store_results(
    outcomes: &mut SourceOutcomes,
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
) {
    super::source_representation::retain_store_representation(
        outcomes, facts, target, arguments, context,
    );
    retain_numeric_store(outcomes, facts, target, arguments, context);
    super::object_instance::retain_object_store(outcomes, facts, target, arguments, context);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EvaluatedSourceValue {
    pub text: String,
    pub representation: tcl_syntax::value::ValueRepresentation,
    pub numeric: Option<crate::native_numeric::FrozenSourceNumeric>,
}

/// Equal completed bytes survive disagreement about the objects that supplied
/// them. Representation and numeric receipts still require unanimous proof.
pub(super) fn joined_result_values(
    left: &Arc<EvaluatedSourceValue>,
    right: &Arc<EvaluatedSourceValue>,
) -> Option<Arc<EvaluatedSourceValue>> {
    if left.text != right.text {
        return None;
    }
    if left == right {
        return Some(Arc::clone(left));
    }
    Some(Arc::new(EvaluatedSourceValue {
        text: left.text.clone(),
        representation: if left.representation == right.representation {
            left.representation
        } else {
            tcl_syntax::value::ValueRepresentation::Unknown
        },
        numeric: if left.numeric == right.numeric {
            left.numeric.clone()
        } else {
            None
        },
    }))
}

/// Frozen ordinary written operands; expansion and captured alias prefixes
/// have independent indexing and cannot inherit this object receipt adapter.
pub(super) fn numeric_call_arguments<'a>(
    context: SourceExecutionContext<'a>,
    target: &SourceCommandTarget,
) -> Option<&'a [Option<Arc<EvaluatedSourceValue>>]> {
    if !target.prepended.is_empty() {
        return None;
    }
    let words = context.written_arguments?;
    if words.iter().any(|word| {
        matches!(
            word,
            EffectiveInvocationWord::Expanded | EffectiveInvocationWord::KnownExpansion(_)
        )
    }) {
        return None;
    }
    let values = context.written_values?;
    (words.len() == values.len()).then_some(values.get(1..)?)
}

pub(super) fn retain_numeric_store(
    outcomes: &mut SourceOutcomes,
    facts: &tcl_registry::InvocationFacts,
    target: &SourceCommandTarget,
    arguments: tcl_registry::InvocationArguments<'_>,
    context: SourceExecutionContext<'_>,
) {
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        )
        || !target.prepended.is_empty()
        || arguments.exact_argv_len() != Some(2)
        || context.written_arguments.is_none_or(|words| {
            words.iter().any(|word| {
                matches!(
                    word,
                    EffectiveInvocationWord::Expanded | EffectiveInvocationWord::KnownExpansion(_)
                )
            })
        })
    {
        return;
    }
    let Some(value) = context
        .written_values
        .and_then(|values| values.get(2))
        .and_then(Option::as_ref)
    else {
        return;
    };
    let Some(numeric) = &value.numeric else {
        return;
    };
    let Some(normal) = &mut outcomes.normal else {
        return;
    };
    let variables = Arc::make_mut(&mut normal.source_variables);
    if variables.representation_epoch != Some(numeric.epoch) {
        return;
    }
    let Some(name) = arguments.literal_at(0) else {
        return;
    };
    let place = crate::var_resolve::resolve_literal_access(
        name,
        variables,
        false,
        context.registry,
        tcl_registry::TraceOperation::Write,
    );
    if place.observed
        || place.dynamic
        || variables.dynamic_bindings
        || variables.dynamic_traces
        || variables.contents_presence(&place) != crate::var_resolve::ContentsPresence::Defined
        || variables.store_would_error(&place)
        || variables.read_contents_origin(&place, context.registry)
            != crate::var_resolve::ContentsOrigin::WrittenAt(context.invocation_offset)
        || variables.literal_contents_at(&place, context.registry) != Some(value.text.as_str())
    {
        return;
    }
    let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) else {
        return;
    };
    variables.value_representations.insert(
        key,
        crate::native_numeric::StoredNativeRepresentation::Numeric(Arc::clone(&numeric.object)),
    );
    if let Some(result) = &mut outcomes.normal_value {
        Arc::make_mut(result).numeric = Some(numeric.clone());
    }
}

pub(super) fn coerce_native_representations(
    facts: &tcl_registry::InvocationFacts,
    native: super::SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) {
    let arguments = native.invocation.arguments();
    if facts.normal_scalar_math_protocol(arguments).is_some() {
        // The prepared native object's input-effect owner already applied the
        // selected GetDouble protocol, including Jim subtype weakening.
        return;
    }
    if facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
        && matches!(
            facts.representation_effect,
            tcl_registry::representation::RepresentationEffect::CoerceExpressionValues { .. }
        )
    {
        // The reached prepared-expression owner inspects the original tree
        // before coercion. It preserves already numeric operands and withdraws
        // receipts for other operands, rejected preparation or opaque source.
        return;
    }
    if matches!(
        facts.operand_representation_coercions,
        tcl_registry::representation::RepresentationCoercionSelection::None
    ) {
        return;
    }
    // A known getter can shimmer an ordinary shared literal while preserving
    // its bytes and stock class. Fresh-pool provenance alone then supplies no
    // current numeric representation for another pooled expression result.
    if let Some(pool) = &mut state.ordinary_literal_pool {
        pool.withdraw_numeric_representations();
    }
    if coerce_indexed_native_representations(facts, native, state, context) {
        return;
    }
    let values = match &facts.operand_representation_coercions {
        tcl_registry::representation::RepresentationCoercionSelection::Operands {
            arguments: selected,
        } => selected
            .iter()
            .map(|&index| arguments.literal_at(index).map(str::to_owned))
            .collect::<Option<Vec<_>>>(),
        _ => None,
    };
    let variables = Arc::make_mut(&mut state.source_variables);
    if let Some(coercions) = facts
        .representation_effect
        .successful_ordinary_container_coercions(arguments, facts.argument_offset)
    {
        for coercion in coercions {
            variables.coerce_ordinary_container_representations(
                values.as_deref(),
                super::container_coercion::target(coercion),
            );
        }
    } else {
        variables.invalidate_representations_for_values(values.as_deref());
    }
}

fn coerce_indexed_native_representations(
    facts: &tcl_registry::InvocationFacts,
    native: super::SourceNativeInvocation<'_>,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> bool {
    let arguments = native.invocation.arguments();
    let Some(indices) = facts
        .representation_effect
        .ordinary_container_index_arguments(arguments, facts.argument_offset)
    else {
        return false;
    };
    let Some(coercions) = facts
        .representation_effect
        .successful_ordinary_container_coercions(arguments, facts.argument_offset)
    else {
        return false;
    };
    let mut numeric = Vec::new();
    let mut other = Vec::new();
    for index in indices {
        if let Some(place) = current_numeric_index(native, index, state, context) {
            numeric.push(place);
        } else {
            other.push(index);
        }
    }
    let variables = Arc::make_mut(&mut state.source_variables);
    if !variables.invalidate_numeric_index_representations(&numeric, context.registry) {
        return true;
    }
    if !other.is_empty() {
        let values = other
            .iter()
            .map(|&index| arguments.literal_at(index).map(str::to_owned))
            .collect::<Option<Vec<_>>>();
        variables.invalidate_representations_for_values(values.as_deref());
    }
    for coercion in coercions {
        let values = arguments
            .literal_at(coercion.argument())
            .map(|value| vec![value.to_owned()]);
        variables.coerce_ordinary_container_representations(
            values.as_deref(),
            super::container_coercion::target(coercion),
        );
    }
    true
}

pub(super) fn current_numeric_index(
    native: super::SourceNativeInvocation<'_>,
    index: usize,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<crate::place::Place> {
    // The final original word has no later argv substitution that could replace
    // its captured object. Enter observers withdraw this original-word ingress.
    context.written_representations?;
    numeric_call_arguments(context, native.target)?;
    if native.invocation.arguments().exact_argv_len()? != index.checked_add(1)? {
        return None;
    }
    let variables = &state.source_variables;
    let place = super::argument_reads::current_argument_place(native, index, state, context)?;
    variables
        .contents_already_native_numeric_at(place, context.registry)
        .then(|| place.clone())
}

impl SourceOutcomes {
    pub(super) fn join_abrupt_result(
        &mut self,
        route: Route,
        state: &ModuleCommandBindings,
        value: Option<&Arc<EvaluatedSourceValue>>,
        object: Option<&Arc<super::SourceObjectInstanceProof>>,
    ) {
        let present = self.abrupt.iter().any(|(known, _)| *known == route);
        let previous = self.result_for(route).cloned();
        let previous_object = self.object_for(route).cloned();
        self.add_abrupt(route, state);
        let joined = if present {
            previous
                .as_ref()
                .zip(value)
                .and_then(|(left, right)| joined_result_values(left, right))
        } else {
            value.cloned()
        };
        if let Some(value) = joined {
            self.abrupt_values.push((route, value));
        }
        if let Some(object) = object
            && (!present || previous_object.as_ref() == Some(object))
            && state.receiver_allocation_is_current(object)
        {
            self.abrupt_objects.push((route, Arc::clone(object)));
        }
    }

    pub(super) fn add_with_result(
        &mut self,
        route: Route,
        state: &ModuleCommandBindings,
        value: Option<&Arc<EvaluatedSourceValue>>,
    ) {
        let previous_normal = self.normal.is_some();
        let previous_value = self.normal_value.clone();
        let previous_abrupt = self.abrupt_values.clone();
        let previous_routes = self
            .abrupt
            .iter()
            .map(|(route, _)| *route)
            .collect::<Vec<_>>();
        self.add(route, state);
        if route.normal_possible() {
            self.normal_value = if previous_normal {
                previous_value
                    .as_ref()
                    .zip(value)
                    .and_then(|(left, right)| joined_result_values(left, right))
            } else {
                value.cloned()
            };
        }
        if let Some(value) = value {
            for (added, _) in &self.abrupt {
                if !route.alternatives().contains(added) && *added != route {
                    continue;
                }
                let previous = previous_abrupt.iter().find(|(known, _)| known == added);
                let joined = if previous_routes.contains(added) {
                    previous.and_then(|(_, known)| joined_result_values(known, value))
                } else {
                    Some(Arc::clone(value))
                };
                if let Some(joined) = joined {
                    self.abrupt_values.retain(|(known, _)| known != added);
                    self.abrupt_values.push((*added, joined));
                }
            }
        }
    }

    /// Preserve an actual object result only when every joined route agrees.
    pub(super) fn add_with_object_result(
        &mut self,
        route: Route,
        state: &ModuleCommandBindings,
        value: Option<&Arc<EvaluatedSourceValue>>,
        object: Option<&Arc<super::SourceObjectInstanceProof>>,
    ) {
        let had_normal = self.normal.is_some();
        let previous = self.normal_object.clone();
        let abrupt = self.abrupt_objects.clone();
        let routes = self
            .abrupt
            .iter()
            .map(|(route, _)| *route)
            .collect::<Vec<_>>();
        self.add_with_result(route, state, value);
        let object = object.filter(|proof| state.receiver_allocation_is_current(proof));
        if route.normal_possible() {
            self.normal_object = if had_normal && previous.as_ref() != object {
                None
            } else {
                object.cloned()
            };
        }
        if let Some(object) = object {
            for (added, _) in &self.abrupt {
                if !route.alternatives().contains(added) && *added != route {
                    continue;
                }
                if !routes.contains(added)
                    || abrupt
                        .iter()
                        .any(|(known, previous)| known == added && previous == object)
                {
                    self.abrupt_objects.retain(|(known, _)| known != added);
                    self.abrupt_objects.push((*added, Arc::clone(object)));
                }
            }
        }
    }

    pub(super) fn object_for(
        &self,
        route: Route,
    ) -> Option<&Arc<super::SourceObjectInstanceProof>> {
        self.abrupt_objects
            .iter()
            .find(|(known, _)| *known == route)
            .map(|(_, object)| object)
    }

    pub(super) fn result_for(&self, route: Route) -> Option<&Arc<EvaluatedSourceValue>> {
        self.abrupt_values
            .iter()
            .find(|(known, _)| *known == route)
            .map(|(_, value)| value)
    }

    pub(super) fn retain_native_result(
        &mut self,
        before: &ModuleCommandBindings,
        target: &SourceCommandTarget,
        effective: &[EffectiveInvocationWord],
        context: SourceExecutionContext<'_>,
    ) {
        if !target.registry_backed {
            return;
        }
        let mut arguments = target
            .prepended
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        arguments.extend(
            effective
                .iter()
                .skip(1)
                .map(EffectiveInvocationWord::as_registry_word),
        );
        let mut invocation = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&target.command),
            &arguments,
        );
        if let Some(dialect) = before.baseline.dialect {
            invocation = invocation.with_dialect(dialect);
        }
        let Some(facts) = super::resolve_source_invocation_facts(
            context.registry,
            context
                .registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile),
            invocation,
            context.realm,
        ) else {
            return;
        };
        let Some(contract) = facts.native_result else {
            return;
        };
        let selection = contract.select(invocation.arguments(), facts.argument_offset);
        if self.normal_value.is_none()
            && let Some(normal) = &self.normal
        {
            self.normal_value =
                selected_result(selection, invocation.arguments(), before, normal, context)
                    .map(Arc::new);
        }
        if let Some(normal) = &self.normal {
            self.normal_representation = selected_result_representation(
                selection,
                invocation.arguments(),
                before,
                normal,
                target,
                context,
            )
            .map(Arc::new)
            .or(self.normal_representation.take().filter(|receipt| {
                receipt.is_current(&normal.source_variables)
                    || receipt.is_immediate_created_result()
            }));
        }
        let returned_object = matches!(
            contract,
            tcl_registry::native_result::NativeResultContract::ReturnResult
        )
        .then(|| selected_return_object(selection, before, target, context))
        .flatten();
        for (route, state) in &self.abrupt {
            if matches!(route, Route::Return(_))
                && let Some(object) = &returned_object
                && state.receiver_allocation_is_current(object)
                && !state.source_step_observed()
                && !state.source_execution_observed(target.identity.as_ref())
            {
                self.abrupt_objects.retain(|(known, _)| known != route);
                self.abrupt_objects.push((*route, Arc::clone(object)));
            }
            if matches!(route, Route::Return(_))
                && let Some(value) =
                    selected_result(selection, invocation.arguments(), before, state, context)
            {
                self.abrupt_values.retain(|(known, _)| known != route);
                self.abrupt_values.push((*route, Arc::new(value)));
            }
        }
    }
}

fn selected_return_object(
    selection: NativeResultSelection,
    before: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> Option<Arc<super::SourceObjectInstanceProof>> {
    if before.source_step_observed() || before.source_execution_observed(target.identity.as_ref()) {
        return None;
    }
    let NativeResultSelection::Argument(index) = selection else {
        return None;
    };
    numeric_call_arguments(context, target)?;
    context.written_objects?.get(index.checked_add(1)?)?.clone()
}

fn selected_result_representation(
    selection: NativeResultSelection,
    arguments: tcl_registry::InvocationArguments<'_>,
    before: &ModuleCommandBindings,
    after: &ModuleCommandBindings,
    target: &SourceCommandTarget,
    context: SourceExecutionContext<'_>,
) -> Option<super::source_representation::FrozenSourceRepresentation> {
    use super::source_representation::FrozenSourceRepresentation;
    let variables = &after.source_variables;
    let shape = selection.created_representation(before.baseline.dialect?, context.compilation);
    if let Some(created) = FrozenSourceRepresentation::capture(shape, variables) {
        return Some(created);
    }
    match selection {
        NativeResultSelection::Argument(index) => {
            numeric_call_arguments(context, target)?;
            context
                .written_representations?
                .get(index.checked_add(1)?)
                .copied()
                .flatten()
                .filter(|receipt| receipt.is_current(variables))
        }
        NativeResultSelection::VariableValue { variable_at, phase } => {
            let operation = match phase {
                tcl_registry::native_result::VariableResultPhase::AfterRead => {
                    tcl_registry::TraceOperation::Read
                }
                tcl_registry::native_result::VariableResultPhase::AfterWrite => {
                    tcl_registry::TraceOperation::Write
                }
            };
            let captured = crate::var_resolve::resolve_literal_access(
                arguments.literal_at(variable_at)?,
                &before.source_variables,
                false,
                context.registry,
                operation,
            );
            if operation == tcl_registry::TraceOperation::Read
                && !variables.read_produces_value(&captured, context.registry)
            {
                return None;
            }
            FrozenSourceRepresentation::capture(
                variables.contents_representation_at(&captured),
                variables,
            )
        }
        _ => None,
    }
}

fn selected_result(
    selection: NativeResultSelection,
    arguments: tcl_registry::InvocationArguments<'_>,
    before: &ModuleCommandBindings,
    after: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<EvaluatedSourceValue> {
    let mut representation = before
        .baseline
        .dialect
        .map_or(tcl_syntax::value::ValueRepresentation::Unknown, |dialect| {
            selection.created_representation(dialect, context.compilation)
        });
    let text = match selection {
        NativeResultSelection::Argument(index) => arguments.literal_at(index)?.to_owned(),
        NativeResultSelection::EmptyString => String::new(),
        NativeResultSelection::ListArguments { from, len } => {
            let values = (from..from.checked_add(len)?)
                .map(|index| arguments.literal_at(index))
                .collect::<Option<Vec<_>>>()?;
            tcl_syntax::list::join_list(values)
        }
        NativeResultSelection::DictionaryArguments { .. }
        | NativeResultSelection::DictionaryValue { .. } => {
            selection.dictionary_literal_result(arguments)?
        }
        NativeResultSelection::VariableExistence { variable_at } => {
            let captured = crate::var_resolve::resolve_literal_access(
                arguments.literal_at(variable_at)?,
                &before.source_variables,
                false,
                context.registry,
                tcl_registry::TraceOperation::Read,
            );
            if after
                .source_variables
                .existence_after_read_at(&captured, context.registry)?
            {
                "1".to_owned()
            } else {
                "0".to_owned()
            }
        }
        NativeResultSelection::VariableValue { variable_at, phase } => {
            let name = arguments.literal_at(variable_at)?;
            let operation = match phase {
                tcl_registry::native_result::VariableResultPhase::AfterRead => {
                    tcl_registry::TraceOperation::Read
                }
                tcl_registry::native_result::VariableResultPhase::AfterWrite => {
                    tcl_registry::TraceOperation::Write
                }
            };
            let captured = crate::var_resolve::resolve_literal_access(
                name,
                &before.source_variables,
                false,
                context.registry,
                operation,
            );
            if let Some(value) = after.source_variables.literal_contents_for_access_at(
                &captured,
                context.registry,
                operation,
            ) {
                representation = after.source_variables.contents_representation_at(&captured);
                value.to_owned()
            } else if after.source_variables.contents_presence(&captured)
                == crate::var_resolve::ContentsPresence::Undefined
                && phase.missing_cell_result(before.baseline.dialect?)
                    == NativeResultSelection::EmptyString
            {
                String::new()
            } else {
                return None;
            }
        }
        NativeResultSelection::ListRange { .. }
        | NativeResultSelection::InvalidArguments
        | NativeResultSelection::Unknown => return None,
    };
    Some(EvaluatedSourceValue {
        text,
        representation,
        numeric: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

    fn analyse(source: &str) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn completed_bytes_join_independently_of_result_representation() {
        use super::{Arc, EvaluatedSourceValue, ModuleCommandBindings, Route, SourceOutcomes};
        use tcl_registry::completion::CompletionCode;
        use tcl_syntax::value::ValueRepresentation;

        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let state = ModuleCommandBindings::initial(registry);
        let list = Arc::new(EvaluatedSourceValue {
            text: "k 4".to_owned(),
            representation: ValueRepresentation::List,
            numeric: None,
        });
        let dictionary = Arc::new(EvaluatedSourceValue {
            representation: ValueRepresentation::Dict,
            ..(*list).clone()
        });
        for code in [CompletionCode::Ok, CompletionCode::Return] {
            let route = Route::Tcl(code);
            let mut outcomes = SourceOutcomes::default();
            outcomes.add_with_result(route, &state, Some(&list));
            outcomes.add_with_result(route, &state, Some(&dictionary));
            let value = if code == CompletionCode::Ok {
                outcomes.normal_value.as_ref()
            } else {
                outcomes.result_for(route)
            }
            .expect("every completed alternative supplies identical bytes");
            assert_eq!(value.text, "k 4");
            assert_eq!(value.representation, ValueRepresentation::Unknown);
            assert!(value.numeric.is_none());

            outcomes.add_with_result(route, &state, None);
            assert!(if code == CompletionCode::Ok {
                outcomes.normal_value.is_none()
            } else {
                outcomes.result_for(route).is_none()
            });
        }
        let different = Arc::new(EvaluatedSourceValue {
            text: "other 5".to_owned(),
            ..(*dictionary).clone()
        });
        assert!(super::joined_result_values(&list, &different).is_none());
    }

    #[test]
    fn native_store_result_materialises_the_next_eval_script() {
        let source = "eval [set script {rename set saved}]; saved x 1";
        let analysis = analyse(source);
        let offset = u32::try_from(source.rfind("saved").unwrap()).unwrap();
        let target = analysis.invocation_at_source("saved", offset);
        assert!(
            target
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed && target.command == "::set"),
            "{target:#?}"
        );
    }

    #[test]
    fn native_return_result_survives_its_procedure_boundary() {
        let source = "proc script {} {return {rename set saved}}; eval [script]; saved x 1";
        let analysis = analyse(source);
        let offset = u32::try_from(source.rfind("saved").unwrap()).unwrap();
        let target = analysis.invocation_at_source("saved", offset);
        assert!(
            target
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed && target.command == "::set"),
            "{target:#?}"
        );
    }

    #[test]
    fn native_results_are_frozen_before_later_argv_stores() {
        let source = "list [set x OLD] [set x NEW]";
        let analysis = analyse(source);
        let binding = analysis.invocation_at_source("list", 0);
        assert_eq!(
            binding.evaluated_argument_values,
            [Some("OLD".to_owned()), Some("NEW".to_owned())]
        );
    }
}
