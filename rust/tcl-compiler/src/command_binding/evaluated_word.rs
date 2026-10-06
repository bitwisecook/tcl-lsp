// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Freeze each reached template component before the next component executes.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext, SourceOutcomes,
};
use crate::ir::WordPart;

impl SourceCommandBindings {
    pub(super) fn walk_template_substitutions(
        &mut self,
        parts: &[WordPart],
        document: &str,
        base: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let mut outcomes = SourceOutcomes::normal(state);
        let mut value = Some(String::new());
        for part in parts {
            let Some(continuing) = outcomes.normal.take() else {
                break;
            };
            *state = *continuing;
            let next = match part {
                WordPart::Variable { spelling, source } => self.observe_variable_substitution(
                    spelling, source, document, base, state, context,
                ),
                WordPart::CommandSubstitution { spelling, source } => {
                    let script = spelling
                        .strip_prefix('[')
                        .and_then(|text| text.strip_suffix(']'))
                        .unwrap_or(spelling);
                    self.walk_source(script, source.span.start() + 1, state, &context)
                }
                _ => SourceOutcomes::normal(state),
            };
            let component = match part {
                WordPart::Text { text, .. } => {
                    let bytes = tcl_syntax::backslash::decode_bytes_in(
                        text.as_bytes(),
                        context.config.escapes,
                    );
                    std::str::from_utf8(&bytes).ok().map(str::to_owned)
                }
                WordPart::Variable { .. } | WordPart::CommandSubstitution { .. } => {
                    next.normal_value.as_ref().map(|value| value.text.clone())
                }
                WordPart::Opaque { .. } => None,
            };
            value = value.zip(component).map(|(mut value, component)| {
                value.push_str(&component);
                value
            });
            outcomes.join(&next);
        }
        outcomes.normal_representation = None;
        outcomes.normal_rhs_read = None;
        outcomes.normal_value = outcomes.normal.as_ref().and(value).map(|text| {
            Arc::new(super::native_result::EvaluatedSourceValue {
                text,
                representation: tcl_syntax::value::ValueRepresentation::Unknown,
                numeric: None,
            })
        });
        outcomes.publish(state);
        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_expression_prepares_the_frozen_components() {
        let source = r#"set a 3; set b 4; expr "$a + $b"; set done 1"#;
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        let script = super::super::ExecutedScriptSource::contiguous(
            Arc::clone(bindings.source_origin().unwrap()),
            source,
            0,
        )
        .unwrap();
        let prepared = bindings.expression_preparations_for_script(&script);
        assert_eq!(prepared.len(), 1);
        assert_eq!(prepared[0].witness.source(), "3 + 4");
        assert_ne!(prepared[0].source.origin, script.origin);
        let done = u32::try_from(source.find("set done").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("set", done)
                .proved_execution_target()
                .is_some()
        );
    }

    #[test]
    fn later_template_substitution_cannot_change_an_earlier_component() {
        let source = r#"set x OLD; set value "$x[set x NEW]"; list $value"#;
        let registry = tcl_registry::CommandRegistry::build_default();
        let bindings =
            SourceCommandBindings::analyse(source, tcl_lexer::LexerConfig::default(), &registry);
        let offset = u32::try_from(source.find("list").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("list", offset);
        assert_eq!(
            binding.evaluated_argument_values,
            [Some("OLDNEW".to_owned())]
        );
    }

    #[test]
    fn script_admission_realm_depends_on_operand_evaluation_not_source_mapping() {
        use super::super::{CommandAllocationSite, SourceAnalysisOptions};
        use tcl_dialect::model::InvocationRealm;

        let registry_context = tcl_registry::model::ingress::static_context_for("f5-irules");
        let registry = registry_context.commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        for (source, realm) in [
            ("eval {gets ch c}", InvocationRealm::RuleLoader),
            (r#"eval "gets\ ch c""#, InvocationRealm::RuleLoader),
            ("eval {gets} {ch c}", InvocationRealm::RuleLoader),
            (
                "set script {gets ch c}; eval $script",
                InvocationRealm::InterpreterRuntime,
            ),
            ("eval [list gets ch c]", InvocationRealm::InterpreterRuntime),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                config,
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..crate::environment_ingress::authoring_native_compilation()
                        },
                    ..SourceAnalysisOptions::default()
                },
            );
            let site = CommandAllocationSite {
                source: Arc::clone(bindings.source_origin().unwrap()),
                offset: u32::try_from(source.rfind("eval").unwrap()).unwrap(),
            };
            let script = bindings.executed_script_at(&site, 0).expect(source);
            assert_eq!(script.text.try_text().unwrap(), "gets ch c", "{source}");
            let child = bindings.invocation_at_origin(&script.origin, script.base());
            assert_eq!(child.invocation_realm(), Some(realm), "{source}");
            if realm == InvocationRealm::InterpreterRuntime {
                assert!(child.proved_target().is_some(), "{source}");
            }
        }
    }
}
