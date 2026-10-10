// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current complete source and metadata, independently of a consumer's purpose.

use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_registry::{CommandRegistry, model::ContextRegistry};

/// Currency of an analysis-backed source query. The input's advice purpose is
/// checked separately; this receipt grants no execution, identity or editing.
pub(crate) struct CurrentSourceContext<'a> {
    config: LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &'a CommandRegistry,
    context: Arc<ContextRegistry>,
}

impl<'a> CurrentSourceContext<'a> {
    pub(crate) const fn config(&self) -> LexerConfig {
        self.config
    }
    pub(crate) const fn profile(&self) -> &'static tcl_dialect::DialectProfile {
        self.profile
    }
    pub(crate) const fn registry(&self) -> &'a CommandRegistry {
        self.registry
    }
    pub(crate) fn context(&self) -> Arc<ContextRegistry> {
        self.context.clone()
    }

    pub(crate) fn capture(source: &str, analysis: &'a AnalysisResult) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        let config = analysis.body_lexer_config?;
        if input.lexer_config() != config
            || !analysis.matches_original_source_image(&SourceImage::document(source), config)
            || !analysis
                .retained_command_realm()?
                .matches_resolved_analysis_input(input)
        {
            return None;
        }
        Some(Self {
            config,
            profile: analysis.resolved_profile()?,
            registry: analysis.resolved_registry()?,
            context: input.context_registry(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};

    #[test]
    fn current_source_currency_is_independent_of_advice_and_execution_purpose() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "proc p {} {}; p";
        for dialect in ["tcl", "tcl8.6"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            let current = CurrentSourceContext::capture(source, &analysis).unwrap();
            assert_eq!(current.config(), analysis.body_lexer_config.unwrap());
            assert_eq!(
                current.context().context(),
                analysis
                    .resolved_input
                    .as_ref()
                    .unwrap()
                    .availability_context()
            );
            let lexical = analysis.allows_lexical_declaration_advice();
            analysis.dialect = "a reporting label".into();
            assert!(CurrentSourceContext::capture(source, &analysis).is_some());
            assert_eq!(analysis.allows_lexical_declaration_advice(), lexical);
        }
    }

    #[test]
    fn current_source_currency_withdraws_changed_grammar_foreign_input_and_missing_owner() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "proc p {} {}; p";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert!(analysis.allows_lexical_declaration_advice());
        assert!(CurrentSourceContext::capture(source, &analysis).is_some());
        assert!(CurrentSourceContext::capture("proc q {} {}; q", &analysis).is_none());
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        assert!(CurrentSourceContext::capture(source, &analysis).is_none());
        analysis.body_lexer_config = Some(config);
        let input = analysis.resolved_input.take().unwrap();
        analysis.resolved_input = Some(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::resolve_environment("tcl8.4").default_context_registry(),
            config,
        ));
        assert!(CurrentSourceContext::capture(source, &analysis).is_none());
        analysis.resolved_input = None;
        assert!(CurrentSourceContext::capture(source, &analysis).is_none());
    }

    fn editors_withhold(source: &str, analysis: &AnalysisResult) {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        assert!(
            crate::completion::completions(source, 0, 14, analysis, None, None, profile).is_empty()
        );
        assert!(crate::definition::definition(source, 0, 13, analysis).is_empty());
        assert!(crate::hover::hover(source, 0, 13, analysis, None).is_none());
        assert!(crate::type_definition::type_definition(source, 0, 13, analysis).is_empty());
        assert!(crate::call_hierarchy::prepare(source, 0, 13, analysis).is_empty());
    }

    #[test]
    fn supplied_editor_entries_do_not_use_logical_purpose_as_source_currency() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "proc p {} {}; p";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert!(CurrentSourceContext::capture(source, &analysis).is_some());
        assert!(!crate::definition::definition(source, 0, 13, &analysis).is_empty());
        assert!(crate::hover::hover(source, 0, 13, &analysis, None).is_some());
        assert!(!crate::call_hierarchy::prepare(source, 0, 13, &analysis).is_empty());
        assert!(
            !crate::completion::completions(
                source,
                0,
                14,
                &analysis,
                None,
                None,
                tcl_dialect::DialectProfile::plain_tcl()
            )
            .is_empty()
        );
        editors_withhold("proc q {} {}; q", &analysis);
        let config = analysis.body_lexer_config.unwrap();
        analysis.body_lexer_config.as_mut().unwrap().strict_quoting = !config.strict_quoting;
        editors_withhold(source, &analysis);
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = None;
        editors_withhold(source, &analysis);
    }
    #[test]
    fn original_logical_type_advice_keeps_current_class_source_and_withdraws_stale_owner() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "oo::class create C {method m {} {}; method again {} {my m}}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let cursor = u32::try_from(source.rfind('m').unwrap()).unwrap();
        assert!(analysis.allows_lexical_declaration_advice());
        assert_eq!(
            crate::type_definition::type_definition(source, 0, cursor, &analysis).len(),
            1
        );
        assert!(
            crate::type_definition::type_definition(
                &source.replace("my m", "my x"),
                0,
                cursor,
                &analysis
            )
            .is_empty()
        );
        analysis.resolved_input = None;
        assert!(crate::type_definition::type_definition(source, 0, cursor, &analysis).is_empty());
    }
}
