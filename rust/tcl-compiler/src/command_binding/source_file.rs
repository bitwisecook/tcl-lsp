// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reached file evaluation through an exact driver-attested loader.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceNativeInvocation, SourceOutcomes, opaque_source_invocation,
};
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::source_file::SourceFileSelection;

impl SourceCommandBindings {
    pub(super) fn walk_selected_source_file(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let arguments = native.invocation.arguments();
        let operands = match tcl_registry::source_file::select(arguments) {
            SourceFileSelection::Selected(operands) => operands,
            SourceFileSelection::Invalid => {
                return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
            }
            SourceFileSelection::Unknown => return opaque_source_invocation(state),
        };
        let Some(dialect) = arguments.dialect() else {
            return opaque_source_invocation(state);
        };
        let Some(path) = arguments.literal_at(operands.path_at) else {
            return opaque_source_invocation(state);
        };
        let encoding = match operands.encoding_at {
            Some(index) => match arguments.literal_at(index) {
                Some(encoding) => Some(encoding),
                None => return opaque_source_invocation(state),
            },
            None => None,
        };
        if state.loader_handler_unknown {
            return opaque_source_invocation(state);
        }
        let mut loaders = state
            .baseline
            .trusted_source_modules
            .iter()
            .filter(|loader| loader.matches_selected_file(path, encoding, dialect));
        let Some(loader) = loaders.next() else {
            return opaque_source_invocation(state);
        };
        if loaders.any(|alternative| alternative != loader) {
            return opaque_source_invocation(state);
        }
        let script = loader.source.clone();
        let Some(spec) = native.compilation_spec else {
            return opaque_source_invocation(state);
        };
        let compilation =
            spec.body_context(state.baseline.compilation_dialect(), context.compilation);
        let previous_origin = state
            .current_source_origin
            .replace(Arc::clone(&script.origin));
        let mut evaluated = self.walk_source_image(
            &script.text,
            script.base(),
            state,
            &SourceExecutionContext {
                realm: tcl_dialect::model::InvocationRealm::InterpreterRuntime,
                compilation,
                compilation_snapshot: None,
                selected_compilation: None,
                original_variable_compilation: None,
                depth: context.depth + 1,
                ..context
            },
        );
        evaluated.restore_source_origin(previous_origin.as_ref());
        let mut outcomes = SourceOutcomes::default();
        if let Some(normal) = &evaluated.normal {
            outcomes.add_with_result(
                Route::Tcl(CompletionCode::Ok),
                normal,
                evaluated.normal_value.as_ref(),
            );
        }
        for (route, branch) in &evaluated.abrupt {
            outcomes.add_with_result(
                tcl_registry::source_file::completion_route(dialect, *route),
                branch,
                evaluated.result_for(*route),
            );
        }
        outcomes.publish(state);
        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, TrustedSourceModuleLoader};
    use tcl_dialect::model::Family;

    fn analyse(source: &str, files: &[TrustedSourceModuleLoader]) -> SourceCommandBindings {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                trusted_source_modules: files,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    fn loader(text: &str) -> TrustedSourceModuleLoader {
        TrustedSourceModuleLoader::new(
            Family::Tcl,
            "actual.tcl".to_owned(),
            None,
            "file-implementation".to_owned(),
            &Arc::from(text),
        )
    }

    #[test]
    fn selected_file_return_preserves_loaded_commands_and_caller_continuation() {
        let source = "source actual.tcl; loaded";
        let bindings = analyse(
            source,
            &[loader("proc loaded {} {return YES}; return DONE")],
        );
        let offset = u32::try_from(source.rfind("loaded").unwrap()).unwrap();
        let call = bindings.invocation_at_source("loaded", offset);
        assert!(call.proved_handler_target().is_some());
        assert!(!call.unknown);
        assert!(bindings.deferred.values().any(|body| {
            matches!(
                body.source_origin
                    .as_deref()
                    .map(super::super::SourceOriginId::kind),
                Some(super::super::SourceOriginKind::Loaded { .. })
            )
        }));
    }

    #[test]
    fn loaded_replacement_keeps_the_computed_heads_actual_reference() {
        let source = "proc greet {} {return local}; source actual.tcl; set cmd greet; $cmd";
        let bindings = analyse(source, &[loader("proc greet {} {return loaded}")]);
        let offset = u32::try_from(source.rfind("$cmd").unwrap()).unwrap();
        let call = bindings.invocation_at_source("$cmd", offset);
        let reference = call.command_reference("greet").unwrap_or_else(|| {
            panic!("loaded computed head lost its positioned reference: {call:#?}")
        });
        let definition = reference
            .definition()
            .expect("actual loaded procedure allocation");
        assert!(matches!(
            definition.allocation().site.source.kind(),
            super::super::SourceOriginKind::Loaded { .. }
        ));
        assert_eq!(definition.allocation().command, "::greet");
    }

    #[test]
    fn file_error_keeps_prior_installation_on_captured_error_route() {
        let source = "catch {source actual.tcl}; loaded";
        let bindings = analyse(source, &[loader("proc loaded {} {return YES}; error STOP")]);
        let offset = u32::try_from(source.rfind("loaded").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("loaded", offset);
        assert!(
            binding.proved_handler_target().is_some(),
            "unknown={} absent={} targets={:?} points={:?} failures={:?}",
            binding.unknown,
            binding.may_be_absent,
            binding.targets,
            bindings
                .points
                .iter()
                .map(|point| (point.offset, &point.head, point.state.has_opaque_domain()))
                .collect::<Vec<_>>(),
            bindings.compilation_failures
        );
    }

    #[test]
    fn conflicting_or_unselected_file_contract_retains_unknown_execution() {
        let source = "source actual.tcl; loaded";
        for files in [
            vec![],
            vec![loader("proc loaded {} {}"), loader("proc other {} {}")],
        ] {
            let offset = u32::try_from(source.rfind("loaded").unwrap()).unwrap();
            assert!(
                analyse(source, &files)
                    .invocation_at_source("loaded", offset)
                    .unknown
            );
        }
    }
}
