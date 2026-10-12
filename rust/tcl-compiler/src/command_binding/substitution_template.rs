// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reached native template substitution, retaining ordered reads and errors.

use super::{
    Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext,
    SourceNativeInvocation, SourceOutcomes, opaque_source_invocation, retained_script_operand,
};
use tcl_lexer::word_parts::{SubstFlags, WordPart};
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::substitution::{SubstitutionTemplateSelection, TemplateParseErrors};

impl SourceCommandBindings {
    #[inline(never)]
    pub(super) fn walk_native_substitution_template(
        &mut self,
        native: SourceNativeInvocation<'_>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let arguments = native.invocation.arguments();
        let query = arguments
            .dialect()
            .and_then(tcl_registry::InvocationDialect::authoring_query)
            .map(|query| query.with_realm(context.realm));
        let Some(invocation) = context
            .registry
            .resolve_structured_invocation(*native.invocation, query)
            .resolved()
        else {
            return opaque_source_invocation(state);
        };
        let (template_at, kinds, parse_errors) = match invocation.native_substitution_template() {
            SubstitutionTemplateSelection::Template {
                template_at,
                kinds,
                parse_errors,
            } => (template_at, kinds, parse_errors),
            SubstitutionTemplateSelection::InvalidArguments => {
                return SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error));
            }
            SubstitutionTemplateSelection::Unknown => return opaque_source_invocation(state),
        };
        // Embedded command completions require the independent subst completion
        // protocol. Keep that residual until this owner represents it exactly.
        if kinds.commands {
            return opaque_source_invocation(state);
        }
        let Some(template) = retained_script_operand(
            template_at,
            native.script_operands(),
            state,
            native.segment.span.start(),
            context.config,
        ) else {
            return opaque_source_invocation(state);
        };
        let previous = state
            .current_source_origin
            .replace(Arc::clone(&template.origin));
        let mut outcomes =
            self.walk_substitution_value(&template, kinds, parse_errors, state, context);
        outcomes.restore_source_origin(previous.as_ref());
        outcomes.publish(state);
        outcomes
    }

    fn observe_template_scalar(
        &mut self,
        name: &str,
        at: u32,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        // Jim treats a braced name ending in parentheses as dictionary sugar
        // too. Its container conversion remains a separate execution obligation.
        if tcl_syntax::naming::split_array_name(name).1.is_some() {
            return opaque_source_invocation(state);
        }
        let access = crate::var_resolve::resolve_literal_access(
            name,
            &state.source_variables,
            false,
            context.registry,
            tcl_registry::TraceOperation::Read,
        );
        // A parsed template name supplies a physical read, not a fabricated
        // written-word/SSA reference or native object identity.
        if access.observed {
            self.walk_captured_observed_read(at, &access, Some(name), state, context)
        } else {
            super::source_representation::unobserved_read_result(state, &access, context.registry)
        }
    }

    fn walk_template_variable(
        &mut self,
        variable: &tcl_lexer::word_parts::VarRef<'_>,
        template: &super::ExecutedScriptSource,
        parse_errors: TemplateParseErrors,
        template_span: Option<tcl_lexer::Span>,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        if parse_errors == TemplateParseErrors::Jim084 {
            let name = std::str::from_utf8(variable.name).ok()?;
            return Some(self.observe_template_scalar(
                name,
                template_span?.start(),
                state,
                context,
            ));
        }
        let range = variable.source_range_in(template.text.bytes(), context.config)?;
        let span =
            variable.source_span_in(template.text.bytes(), template.base(), context.config)?;
        Some(self.observe_variable_substitution(
            template.try_text().ok()?.get(range)?,
            &crate::ir::SourceSite::source(span),
            template.try_text().ok()?,
            template.base(),
            state,
            context,
        ))
    }

    fn walk_substitution_value(
        &mut self,
        template: &super::ExecutedScriptSource,
        kinds: tcl_registry::substitution::SubstitutionKinds,
        parse_errors: TemplateParseErrors,
        state: &mut ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> SourceOutcomes {
        let flags = SubstFlags {
            vars: kinds.variables,
            cmds: kinds.commands,
            backslashes: kinds.backslashes,
            ..SubstFlags::default()
        };
        let Ok(parts) = tcl_lexer::word_parts::decompose_template_spanned(
            template.text.bytes(),
            flags,
            context.config,
            parse_errors.variable_syntax(),
        ) else {
            return opaque_source_invocation(state);
        };
        let mut outcomes = SourceOutcomes::normal(state);
        let mut value = Some(String::new());
        for part in parts {
            let template_span = part.template_source_span(
                template.text.bytes(),
                template.base(),
                parse_errors.variable_syntax(),
            );
            let Some(continuing) = outcomes.normal.take() else {
                break;
            };
            *state = *continuing;
            let (next, component) = match part.part {
                WordPart::Text(bytes) => (
                    SourceOutcomes::normal(state),
                    String::from_utf8(bytes.into_owned()).ok(),
                ),
                WordPart::Variable(variable) if variable.index.is_none() => {
                    let Some(next) = self.walk_template_variable(
                        &variable,
                        template,
                        parse_errors,
                        template_span,
                        state,
                        context,
                    ) else {
                        outcomes.join(&opaque_source_invocation(state));
                        value = None;
                        break;
                    };
                    let component = next.normal_value.as_ref().map(|value| value.text.clone());
                    (next, component)
                }
                WordPart::ParseError(_) if parse_errors == TemplateParseErrors::Rejected => (
                    SourceOutcomes::invocation(state, Route::Tcl(CompletionCode::Error)),
                    None,
                ),
                WordPart::ParseError(_)
                | WordPart::Variable(_)
                | WordPart::Command(_)
                | WordPart::Expression(_) => {
                    outcomes.join(&opaque_source_invocation(state));
                    value = None;
                    break;
                }
            };
            value = value.zip(component).map(|(mut value, component)| {
                value.push_str(&component);
                value
            });
            outcomes.join(&next);
        }
        outcomes.normal_representation = None;
        outcomes.normal_value = outcomes.normal.as_ref().and(value).map(|text| {
            Arc::new(super::native_result::EvaluatedSourceValue {
                text,
                representation: tcl_syntax::value::ValueRepresentation::Unknown,
                numeric: None,
            })
        });
        outcomes.normal_object = None;
        outcomes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_template_reads_keep_observer_updates_and_partial_parse_effects() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let context = tcl_registry::model::ingress::static_context_for(profile);
            for (source, expected) in [
                (
                    r"set x 7; proc Read args {set ::x 8}; trace add variable x read Read; set value [subst -nocommands {return $x}]; list $value",
                    "return 8",
                ),
                (
                    r#"set x 7; set reads 0; proc Read args {incr ::reads}; trace add variable x read Read; set template "\$x \${bad"; catch {subst -nocommands $template}; list $reads"#,
                    "1",
                ),
                (
                    r"set x 7; set reads 0; proc Read args {incr ::reads; error BOOM}; trace add variable x read Read; catch {subst -nocommands {return $x}}; list $reads",
                    "1",
                ),
            ] {
                let bindings = SourceCommandBindings::analyse(
                    source,
                    tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
                    context.commands(),
                );
                let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
                let binding = bindings.invocation_at_source("list", at);
                assert_eq!(
                    binding.evaluated_argument_values,
                    [Some(expected.to_owned())],
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn jim_template_scalar_acceptance_keeps_written_words_strict() {
        let context = tcl_registry::model::ingress::static_context_for("jim");
        let source =
            r#"set x 7; set template "\${x"; set value [subst -nocommands $template]; list $value"#;
        let bindings = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
            context.commands(),
        );
        let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
        assert_eq!(
            bindings
                .invocation_at_source("list", at)
                .evaluated_argument_values,
            [Some("7".into())]
        );
        let source =
            r#"set template "\$[1+2]"; set value [subst -nocommands $template]; list $value"#;
        let bindings = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
            context.commands(),
        );
        let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
        assert!(bindings.invocation_at_source("list", at).unknown);
    }

    #[test]
    fn jim_template_escaping_and_free_text_names_use_native_policy() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = tcl_registry::model::ingress::static_context_for(profile);
            let source = r"set x 7; set template {\$x}; set value [subst -nocommands -nobackslashes $template]; list $value";
            let bindings = SourceCommandBindings::analyse(
                source,
                tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
                context.commands(),
            );
            let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
            let expected = if profile == "jim" { r"\$x" } else { r"\7" };
            assert_eq!(
                bindings
                    .invocation_at_source("list", at)
                    .evaluated_argument_values,
                [Some(expected.into())],
                "{profile}"
            );
        }
        let context = tcl_registry::model::ingress::static_context_for("jim");
        let source = r#"set {x y} 8; set template "\${x y"; set value [subst -nocommands $template]; list $value"#;
        let bindings = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
            context.commands(),
        );
        let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
        assert_eq!(
            bindings
                .invocation_at_source("list", at)
                .evaluated_argument_values,
            [Some("8".into())]
        );
        let source = r#"set a [list b VALUE]; set template "\${a(b)"; set value [subst -nocommands $template]; list $value"#;
        let bindings = SourceCommandBindings::analyse(
            source,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
            context.commands(),
        );
        let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
        assert!(bindings.invocation_at_source("list", at).unknown);
    }

    #[test]
    fn native_template_flags_and_scalar_values_are_frozen_without_object_proof() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let context = tcl_registry::model::ingress::static_context_for(profile);
            for (flags, expected) in [
                ("-nocommands -nobackslashes", r"return 7\n"),
                ("-nocommands -novariables -nobackslashes", r"return $x\n"),
            ] {
                let source =
                    format!(r"set x 7; set value [subst {flags} {{return $x\n}}]; list $value");
                let bindings = SourceCommandBindings::analyse(
                    &source,
                    tcl_lexer::LexerConfig::for_profile(context.commands().profile()),
                    context.commands(),
                );
                let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
                let binding = bindings.invocation_at_source("list", at);
                assert_eq!(
                    binding.evaluated_argument_values,
                    [Some(expected.to_owned())],
                    "{profile}: {source}"
                );
                let place = crate::var_resolve::resolve_literal_place(
                    "value",
                    &binding.variable_context,
                    false,
                    context.commands(),
                );
                assert!(
                    binding
                        .variable_context
                        .contents_native_numeric_at(&place, context.commands())
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn array_index_commands_and_unknown_templates_retain_their_effect_residual() {
        let registry = tcl_registry::CommandRegistry::build_default();
        for source in [
            "set k OLD; array set a {z VALUE}; subst -nocommands {$a([set k z])}; list $k",
            "subst -nocommands $unknown; list DONE",
        ] {
            let bindings = SourceCommandBindings::analyse(
                source,
                tcl_lexer::LexerConfig::default(),
                &registry,
            );
            let at = u32::try_from(source.rfind("list ").unwrap()).unwrap();
            assert!(
                bindings.invocation_at_source("list", at).unknown,
                "{source}"
            );
        }
    }
}
