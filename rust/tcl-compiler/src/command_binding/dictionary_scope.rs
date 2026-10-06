// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dictionary source execution shares physical entry and writeback with CFG flow.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceNativeInvocation, SourceOutcomes, boxed_source_branch, opaque_source_invocation,
};
use crate::dictionary_bindings::{
    DictionaryScopeActivation, DictionaryScopeEntry, DictionaryScopeFinish, DictionaryScopeId,
    enter_dictionary_scope, finish_dictionary_scope,
};
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::dictionary_scope::{DictionaryScopeImplementation, DictionaryScopeSelection};

struct ActiveScope {
    activation: DictionaryScopeActivation,
    id: DictionaryScopeId,
}

impl SourceCommandBindings {
    pub(super) fn walk_dictionary_scope(
        &mut self,
        selection: DictionaryScopeSelection,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let plan = match selection {
            DictionaryScopeSelection::Selected(plan)
                if plan.implementation == DictionaryScopeImplementation::NativePrimitive =>
            {
                plan
            }
            DictionaryScopeSelection::Invalid => {
                return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
            }
            _ => return opaque_source_invocation(state),
        };
        let entry = enter_dictionary_scope(
            Arc::make_mut(&mut state.source_variables),
            &plan,
            native.invocation.arguments(),
            context.registry,
            native.segment.span.start(),
        );
        let activation = match entry {
            DictionaryScopeEntry::Entered(activation) => *activation,
            DictionaryScopeEntry::Invalid => {
                return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
            }
            DictionaryScopeEntry::Unknown => return opaque_source_invocation(state),
        };
        if state.source_variables.dynamic_bindings {
            return opaque_source_invocation(state);
        }
        let Some(origin) = &state.current_source_origin else {
            return opaque_source_invocation(state);
        };
        let id = DictionaryScopeId {
            site: super::CommandAllocationSite {
                source: Arc::clone(origin),
                offset: native.segment.span.start(),
            },
            activation: state.source_variables.activation.clone(),
        };
        Arc::make_mut(&mut state.source_variables)
            .dictionary_scopes
            .insert(id.clone(), Arc::new(activation.clone()));
        let scope = ActiveScope { activation, id };
        let entry_state = boxed_source_branch(state);
        let mut executed = self.walk_body_operand(
            plan.body_argument,
            native.script_operands(),
            state,
            &context,
        );
        let mut completed = SourceOutcomes::default();
        // Mapping writes can fail before body entry, including partial mappings.
        let mut partial_entry = entry_state;
        Arc::make_mut(&mut partial_entry.source_variables)
            .dictionary_scopes
            .remove(&scope.id);
        completed.add_abrupt(Route::Tcl(CompletionCode::Error), &partial_entry);
        if let Some(mut normal) = executed.normal.take() {
            finish_scope_path(
                &mut completed,
                &scope,
                Route::Tcl(CompletionCode::Ok),
                &mut normal,
                executed.normal_value.as_ref(),
                native,
                context,
            );
        }
        for (route, mut abrupt) in executed.abrupt.clone() {
            finish_scope_path(
                &mut completed,
                &scope,
                route,
                &mut abrupt,
                executed.result_for(route),
                native,
                context,
            );
        }
        completed.publish(state);
        completed
    }
}

fn finish_scope_path(
    completed: &mut SourceOutcomes,
    scope: &ActiveScope,
    route: Route,
    state: &mut ModuleCommandBindings,
    value: Option<&Arc<super::native_result::EvaluatedSourceValue>>,
    native: SourceNativeInvocation<'_>,
    context: SourceExecutionContext<'_>,
) {
    let activation = &scope.activation;
    let selected = activation.plan.writeback_routes(route);
    Arc::make_mut(&mut state.source_variables)
        .dictionary_scopes
        .remove(&scope.id);
    for propagated in selected.propagated {
        completed.add_with_result(propagated, state, value);
    }
    if !selected.captured {
        return;
    }
    let before_writeback = boxed_source_branch(state);
    match finish_dictionary_scope(
        Arc::make_mut(&mut state.source_variables),
        activation,
        route,
        context.registry,
        native.segment.span.start(),
    ) {
        DictionaryScopeFinish::Skipped | DictionaryScopeFinish::Written(_) => {
            completed.add_with_result(
                activation.plan.completion_after_writeback(route),
                state,
                value,
            );
            // A callback or invalid live value can replace the body's completion.
            completed.add_abrupt(Route::Tcl(CompletionCode::Error), &before_writeback);
        }
        DictionaryScopeFinish::Invalid => {
            completed.add(activation.plan.writeback_failure_route(), state);
        }
        DictionaryScopeFinish::Unknown => {
            state.mark_opaque_binding_mutation();
            completed.add(Route::Unknown, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::SourceAnalysisOptions;

    fn bindings(source: &str) -> SourceCommandBindings {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = context.commands();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(registry.profile()),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                    catch_depth: Some(0),
                    loop_depth: 0,
                },
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn actual_scope_writeback_retains_body_stored_value() {
        let source = "set d {k OLD}; dict update d k v {set v {rename set saved}}; list final";
        let bindings = bindings(source);
        let offset = u32::try_from(source.rfind("list final").unwrap()).unwrap();
        let resolved = bindings.invocation_at_source("list", offset);
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        assert_eq!(
            resolved.variable_context.literal_value("d", registry),
            Some("k {rename set saved}"),
            "{resolved:#?}"
        );
    }

    #[test]
    fn invalid_scope_entry_does_not_enter_body() {
        let source = "dict update missing k v {rename set saved}; saved x 1";
        let bindings = bindings(source);
        let offset = u32::try_from(source.rfind("saved x").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("saved", offset)
                .proved_execution_target()
                .is_none()
        );
    }
}
