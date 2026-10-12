// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sequential conditional outputs with independently closed physical stores.

use super::{
    Arc, ModuleCommandBindings, SourceExecutionContext, SourceNativeInvocation, SourceOutcomes,
};
use crate::{
    place::{CellOwner, Place, PlaceKind},
    var_resolve::{ContentsPresence, ResolveContext, RootContentsKind},
};
use tcl_registry::{
    CommandRegistry, completion::CompletionCode,
    completion_route::InvocationCompletionRoute as Route,
    variable_output::VariableOutputCommitment,
};

pub(super) fn walk_known_outputs(
    native: SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &mut ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Option<SourceOutcomes> {
    let arguments = native.invocation.arguments();
    let outputs = facts.successful_variable_output_commitments(arguments)?;
    if outputs.is_empty()
        || outputs
            .iter()
            .any(|(_, commitment)| *commitment == VariableOutputCommitment::MayWrite)
    {
        return None;
    }
    let mut candidate = super::boxed_source_branch(state);
    for (index, commitment) in outputs {
        if commitment == VariableOutputCommitment::Unchanged {
            continue;
        }
        let target =
            crate::variable_bindings::variable_output_operand_access_with_original_operands(
                facts,
                arguments,
                index,
                &candidate.source_variables,
                context.registry,
                native.original_variable_operands,
            );
        if target.observed || target.dynamic || target.cell.is_none() {
            return None;
        }
        if candidate.source_variables.store_would_error(&target) {
            *state = *candidate;
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        }
        if !store_is_closed(&target, &candidate.source_variables, context.registry) {
            return None;
        }
        let source = candidate.current_source_origin.clone();
        let variables = Arc::make_mut(&mut candidate.source_variables);
        variables.set_contents_write_source(source);
        variables.publish_captured_store(&target, None, context.invocation_offset);
    }
    *state = *candidate;
    Some(SourceOutcomes::invocation(
        state,
        Route::Tcl(CompletionCode::Ok),
    ))
}

pub(super) fn store_is_closed(
    target: &Place,
    variables: &ResolveContext,
    registry: &CommandRegistry,
) -> bool {
    let Some(cell) = &target.cell else {
        return false;
    };
    if variables.dynamic_bindings || cell.generation == crate::place::CellGeneration::Unknown {
        return false;
    }
    match &cell.owner {
        CellOwner::Activation(owner) if variables.activation.as_ref() == Some(owner) => {}
        CellOwner::Namespace(namespace) if variables.known_namespaces.contains(namespace) => {}
        CellOwner::NamespaceIdentity(namespace)
            if variables.namespace_identities.contains(namespace.as_ref()) => {}
        _ => return false,
    }
    let presence = variables.contents_presence(target);
    match (
        variables
            .invocation_dialect
            .and_then(|dialect| dialect.variable_container_model),
        target.kind,
    ) {
        (Some(tcl_dialect::VariableContainerModel::DictionaryValue), PlaceKind::Scalar) => {
            matches!(
                presence,
                ContentsPresence::Defined | ContentsPresence::Undefined
            )
        }
        (Some(tcl_dialect::VariableContainerModel::DistinctArray), PlaceKind::Scalar) => {
            presence == ContentsPresence::Undefined
                || variables.root_contents_kind(target) == Some(RootContentsKind::Scalar)
        }
        (Some(tcl_dialect::VariableContainerModel::DistinctArray), PlaceKind::ArrayElem) => {
            target
                .index
                .as_ref()
                .is_some_and(|index| index.kind == crate::place::IndexKind::Literal)
                && (variables.root_contents_kind(target) == Some(RootContentsKind::Array)
                    || variables.contents_presence(&target.base()) == ContentsPresence::Undefined)
                && !variables
                    .variable_observers_at(target, tcl_registry::TraceOperation::Write, registry)
                    .unknown_residual
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

    fn after(source: &str, name: &str) -> (SourceCommandBindings, Arc<CommandRegistry>, u32) {
        let registry =
            Arc::clone(tcl_registry::model::ingress::static_context_for("tcl8.6").commands());
        let options = SourceAnalysisOptions {
            invocation_dialect: registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        };
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            &registry,
            options,
        );
        (
            bindings,
            registry,
            u32::try_from(source.rfind(name).unwrap()).unwrap(),
        )
    }

    #[test]
    fn matched_native_outputs_establish_only_closed_physical_stores() {
        for source in [
            "proc f {} {scan 42 %d n; puts $n}",
            "proc f {} {regexp -nocase x X n; puts $n}",
        ] {
            let (bindings, registry, offset) = after(source, "puts");
            let point = bindings.invocation_at_source("puts", offset);
            let target = crate::var_resolve::resolve_literal_access(
                "n",
                &point.variable_context,
                false,
                &registry,
                tcl_registry::TraceOperation::Read,
            );
            assert_eq!(
                point.variable_context.contents_presence(&target),
                ContentsPresence::Defined,
                "{source}"
            );
            assert!(matches!(
                point.variable_context.contents_origin(&target),
                crate::var_resolve::ContentsOrigin::WrittenAt(_)
            ));
        }
    }

    #[test]
    fn unknown_input_keeps_a_bounded_output_possibly_missing() {
        let source = "proc f {input} {scan $input %d n; puts $n}";
        let (bindings, registry, offset) = after(source, "puts");
        let point = bindings.invocation_at_source("puts", offset);
        let target = crate::var_resolve::resolve_literal_access(
            "n",
            &point.variable_context,
            false,
            &registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point.variable_context.closed_contents_presence(&target),
            Some(ContentsPresence::DefinedOrUndefined),
            "target={target:?}, dynamic={}, world={:?}, presence={:?}, origin={:?}",
            point.variable_context.dynamic_bindings,
            point.variable_context.contents_world,
            point.variable_context.contents_presence(&target),
            point.variable_context.contents_origin(&target),
        );
        assert_eq!(
            point.variable_context.contents_presence(&target),
            ContentsPresence::Unknown
        );
        assert!(
            !point
                .variable_context
                .read_produces_value(&target, &registry)
        );
    }

    #[test]
    fn failed_later_output_preserves_earlier_store_and_existing_array() {
        let source = "proc f {} {array set b {old OLD}; catch {scan {42 43} {%d %d} a b}; puts $a; array size b}";
        let (bindings, registry, offset) = after(source, "puts");
        let point = bindings.invocation_at_source("puts", offset);
        let a = crate::var_resolve::resolve_literal_access(
            "a",
            &point.variable_context,
            false,
            &registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            point.variable_context.contents_presence(&a),
            ContentsPresence::Defined
        );
        let b = crate::var_resolve::resolve_literal_place(
            "b",
            &point.variable_context,
            false,
            &registry,
        );
        assert_eq!(
            point.variable_context.root_contents_kind(&b),
            Some(RootContentsKind::Array)
        );
    }
}
