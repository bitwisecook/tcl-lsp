// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current source correspondence for diagnostic edits, independently of meaning.

use tcl_compiler::analyser::{AnalysisResult, Diagnostic};
use tcl_compiler::compiler_checks::Diagnostic as CompilerDiagnostic;
use tcl_lexer::{SourceImage, Span};

use crate::workspace_index::WorkspaceDiagnosticSourceContext;

/// A borrowed current document and its actual retained analysis inputs.
/// Source currency grants no Native handler, name, value, evaluation,
/// equivalence or editing permission; a fix still comes from its own issuer.
pub struct DiagnosticEditSource<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    context: WorkspaceDiagnosticSourceContext,
    registry: tcl_registry::RegistrySemanticKey,
}

impl<'a> DiagnosticEditSource<'a> {
    /// Capture exact current Document bytes, complete config and Registry.
    /// Existing analysis is required; no profile label reconstructs its owner.
    #[must_use]
    pub fn for_analysis(source: &'a str, analysis: &'a AnalysisResult) -> Option<Self> {
        // Implementation contract: naming.editor.original-diagnostic-edit-currency
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
        if analysis.analysis_context_unavailable.is_some() {
            return None;
        }
        let context = WorkspaceDiagnosticSourceContext::for_analysis(analysis)?;
        let input = analysis.resolved_input.as_ref()?;
        if context.image() != &SourceImage::document(source)
            || context.config() != input.lexer_config()
        {
            return None;
        }
        let registry = analysis.resolved_registry()?.snapshot().semantic_key();
        Some(Self {
            source,
            analysis,
            context,
            registry,
        })
    }

    /// Exact document text authenticated by the retained source owner.
    #[must_use]
    pub const fn source(&self) -> &'a str {
        self.source
    }

    /// A complete valid UTF-8 source extent, including end-of-file insertions.
    #[must_use]
    pub fn contains_span(&self, span: Span) -> bool {
        self.source.get(span.as_range()).is_some()
    }

    /// Report conversion preserves the original issuer and its current source.
    #[must_use]
    pub fn matches_finding(&self, finding: &crate::diagnostic_policy::Finding) -> bool {
        if let Some(original) = finding.original_analyser_diagnostic() {
            return self.matches_analyser_diagnostic(original);
        }
        if let Some(original) = finding.original_compiler_diagnostic() {
            return self.matches_compiler_diagnostic(original);
        }
        matches!(
            finding.producer,
            crate::diagnostic_policy::Producer::SourceStyle
        ) && self.contains_span(finding.span)
    }

    /// A published fix must occur independently in the current analyser set.
    /// Diagnostic titles and messages are presentation and may differ.
    #[must_use]
    pub fn matches_analyser_diagnostic(&self, diagnostic: &Diagnostic) -> bool {
        self.contains_span(diagnostic.span)
            && self.analysis.diagnostics.iter().any(|current| {
                current.code == diagnostic.code
                    && current.span == diagnostic.span
                    && diagnostic
                        .fixes
                        .iter()
                        .all(|fix| current.fixes.contains(fix))
            })
    }

    /// Compare the check issuer's independent whole-source/config/Registry.
    /// Missing or foreign provenance cannot issue a fix or suppression edit.
    #[must_use]
    pub fn matches_compiler_diagnostic(&self, diagnostic: &CompilerDiagnostic) -> bool {
        let Some(issuer) = diagnostic.source_context.as_ref() else {
            return false;
        };
        self.contains_span(diagnostic.span)
            && issuer.image() == self.context.image()
            && issuer.lexer_config() == self.context.config()
            && issuer.registry().semantic_key() == self.registry
            && self.analysis.resolved_input.as_ref() == Some(issuer.analysis_input())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code_actions::{ActionKind, check_diagnostic_actions, code_actions};
    use crate::definition::LspRange;
    use std::collections::{HashMap, HashSet};
    use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
    use tcl_compiler::compilation_unit::CompilationUnit;
    use tcl_compiler::compiler_checks::run_all_checks;
    use tcl_lexer::LexerConfig;
    use tcl_registry::CommandRegistry;

    fn analysis(source: &str, registry: &CommandRegistry, config: LexerConfig) -> AnalysisResult {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let context = crate::context_for_dialect_profile(profile)
            .with_command_store(registry.snapshot().shared_registry());
        Analyser::new()
            .with_resolved_input(ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::new(context),
                config,
            ))
            .analyse(source, profile.name)
    }
    fn report_of(rows: &[Diagnostic]) -> crate::diagnostic_policy::Report {
        crate::diagnostic_policy::apply(
            rows.iter()
                .cloned()
                .map(crate::diagnostic_policy::Finding::from)
                .collect(),
            &crate::diagnostic_policy::Policy::unrestricted(),
        )
    }
    fn all() -> LspRange {
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: u32::MAX,
            end_character: u32::MAX,
        }
    }

    #[test]
    fn original_diagnostic_actions_require_current_whole_source_and_owned_fixes() {
        // Implementation contract: naming.editor.original-diagnostic-edit-currency
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
        let source = "set x [foo\n# tail";
        let current = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            current
                .diagnostics
                .iter()
                .any(|row| row.code == tcl_core_types::DiagCode::E201
                    && row
                        .fixes
                        .iter()
                        .any(|fix| fix.span == Span::new(10, 10) && fix.new_text == "]")),
            "the real comment-boundary issuer supplies the exact closer insertion"
        );
        let actions = code_actions(
            source,
            all(),
            Some(&current),
            &report_of(&current.diagnostics),
        );
        let fixes: Vec<_> = actions
            .iter()
            .filter(|action| action.kind == ActionKind::QuickFix)
            .collect();
        assert!(
            !fixes.is_empty(),
            "a genuine issued close-bracket fix is retained"
        );
        assert!(fixes.iter().all(|action| {
            action
                .edits
                .iter()
                .all(|edit| edit.range.start_line == 0 && edit.range.end_line == 0)
        }));
        assert!(
            code_actions(
                "set y [foo\n# tail",
                all(),
                Some(&current),
                &report_of(&current.diagnostics)
            )
            .is_empty()
        );
        assert!(
            code_actions(
                &format!("{source} "),
                all(),
                Some(&current),
                &report_of(&current.diagnostics)
            )
            .is_empty()
        );
        assert!(
            code_actions(
                source,
                all(),
                Some(&AnalysisResult::default()),
                &report_of(&current.diagnostics)
            )
            .is_empty()
        );
        let mut published = current.diagnostics.clone();
        for row in &mut published {
            row.message = "counterfactual presentation".to_owned();
        }
        let changed_titles = code_actions(source, all(), Some(&current), &report_of(&published));
        assert_eq!(
            changed_titles
                .iter()
                .filter(|a| a.kind == ActionKind::QuickFix)
                .flat_map(|a| &a.edits)
                .collect::<Vec<_>>(),
            fixes.iter().flat_map(|a| &a.edits).collect::<Vec<_>>()
        );
        let mut foreign = published;
        for row in &mut foreign {
            for fix in &mut row.fixes {
                fix.new_text = "foreign edit".to_owned();
            }
        }
        assert!(
            !code_actions(source, all(), Some(&current), &report_of(&foreign))
                .iter()
                .any(|a| a.kind == ActionKind::QuickFix)
        );
        let mut changed_config = current.clone();
        changed_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .expand_syntax = false;
        assert!(
            code_actions(
                source,
                all(),
                Some(&changed_config),
                &report_of(&current.diagnostics)
            )
            .is_empty()
        );
    }

    fn logical_check_issuer(
        source: &str,
        registry: &CommandRegistry,
        config: LexerConfig,
    ) -> (AnalysisResult, CompilationUnit) {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::resolve_environment("tcl8.6")
            .default_context_registry()
            .with_command_store(registry.snapshot().shared_registry());
        let input =
            ResolvedAnalysisInput::new(profile, profile, std::sync::Arc::new(context), config);
        let current = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            tcl_compiler::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        (current, unit)
    }

    #[test]
    fn original_check_actions_require_the_issuers_source_config_and_registry() {
        // Implementation contract: naming.editor.original-diagnostic-edit-currency
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
        let source = "set x hello
incr x
";
        let registry = CommandRegistry::build_default();
        let config = LexerConfig::default();
        let (current, unit) = logical_check_issuer(source, &registry, config);
        let checks = run_all_checks(&unit, &registry, None);
        assert!(
            checks
                .iter()
                .any(|row| row.code == tcl_compiler::compiler_checks::DiagCode::S100)
        );
        let guard = DiagnosticEditSource::for_analysis(source, &current).unwrap();
        let disabled = HashSet::new();
        let suppressed: HashMap<i32, HashSet<String>> = HashMap::new();
        let actions = check_diagnostic_actions(&guard, all(), &checks, &disabled, &suppressed);
        let suppress = actions
            .iter()
            .find(|a| a.title == "Suppress S100 with a noqa comment")
            .unwrap();
        assert_eq!(
            suppress.edits[0].new_text,
            "# noqa: S100
"
        );
        assert_eq!(suppress.edits[0].range.start_line, 1);
        assert_eq!(suppress.edits[0].range.start_character, 0);
        assert_eq!(suppress.edits[0].range.end_line, 1);
        assert_eq!(suppress.edits[0].range.end_character, 0);
        let foreign_source = "set x other
incr x
";
        let foreign = analysis(foreign_source, &registry, config);
        let other = DiagnosticEditSource::for_analysis(foreign_source, &foreign).unwrap();
        assert!(
            check_diagnostic_actions(&other, all(), &checks, &disabled, &suppressed).is_empty()
        );
        let mut foreign_config = config;
        foreign_config.expand_syntax = !config.expand_syntax;
        let foreign = analysis(source, &registry, foreign_config);
        let other = DiagnosticEditSource::for_analysis(source, &foreign).unwrap();
        assert!(
            check_diagnostic_actions(&other, all(), &checks, &disabled, &suppressed).is_empty()
        );
        let mut foreign_registry = CommandRegistry::build_default();
        foreign_registry.load_irules();
        let foreign = analysis(source, &foreign_registry, config);
        let other = DiagnosticEditSource::for_analysis(source, &foreign).unwrap();
        assert!(
            check_diagnostic_actions(&other, all(), &checks, &disabled, &suppressed).is_empty()
        );
        let mut absent = checks.clone();
        for row in &mut absent {
            row.source_context = None;
        }
        assert!(
            check_diagnostic_actions(&guard, all(), &absent, &disabled, &suppressed).is_empty()
        );
        let mut prose = checks;
        for row in &mut prose {
            row.message = "another title".to_owned();
        }
        assert_eq!(
            check_diagnostic_actions(&guard, all(), &prose, &disabled, &suppressed),
            actions
        );
    }

    #[test]
    fn original_check_currency_refuses_same_store_changed_availability_and_missing_input() {
        // naming.editor.original-diagnostic-edit-currency
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
        // This conditional source warning supplies no physical frame or Normal effect.
        let source = "set x hello\nincr x\n";
        let registry = CommandRegistry::build_default();
        let config = LexerConfig::default();
        let (current, unit) = logical_check_issuer(source, &registry, config);
        let input = current.resolved_input.as_ref().unwrap();
        let profile = input.unit_profile();
        let checks = run_all_checks(&unit, &registry, None);
        assert!(
            checks
                .iter()
                .any(|row| row.code == tcl_compiler::compiler_checks::DiagCode::S100)
        );
        let guard = DiagnosticEditSource::for_analysis(source, &current).unwrap();
        let disabled = HashSet::new();
        let suppressed: HashMap<i32, HashSet<String>> = HashMap::new();
        assert!(
            !check_diagnostic_actions(&guard, all(), &checks, &disabled, &suppressed).is_empty()
        );
        let older_context = tcl_registry::model::resolve_environment("tcl8.4")
            .default_context_registry()
            .with_command_store(registry.snapshot().shared_registry());
        let older_input = ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(older_context),
            config,
        );
        assert!(std::sync::Arc::ptr_eq(
            input.borrowed_context_registry().commands(),
            older_input.borrowed_context_registry().commands(),
        ));
        assert_ne!(input, older_input);
        let older = Analyser::new()
            .with_resolved_input(older_input)
            .analyse(source, profile.name);
        let older_guard = DiagnosticEditSource::for_analysis(source, &older).unwrap();
        assert!(
            check_diagnostic_actions(&older_guard, all(), &checks, &disabled, &suppressed)
                .is_empty()
        );
        let mut missing_unit = unit.clone();
        missing_unit.ir_module.source_metadata_input = None;
        let mut missing = checks.clone();
        tcl_compiler::compiler_checks::retain_diagnostic_source_context(
            &missing_unit,
            &registry,
            &mut missing,
        );
        assert!(missing.iter().all(|row| row.source_context.is_none()));
        assert!(
            check_diagnostic_actions(&guard, all(), &missing, &disabled, &suppressed).is_empty()
        );
    }

    #[test]
    fn original_diagnostic_source_extents_require_actual_byte_boundaries_and_channel() {
        // Implementation contract: naming.editor.original-diagnostic-edit-currency
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-edit-currency.md
        let source = r#"set x "🦀""#;
        let current = Analyser::new().analyse(source, "tcl8.6");
        let guard = DiagnosticEditSource::for_analysis(source, &current).unwrap();
        let start = u32::try_from(source.find('🦀').unwrap()).unwrap();
        assert!(guard.contains_span(Span::new(start, start + 4)));
        assert!(!guard.contains_span(Span::new(start + 1, start + 4)));
        let end = u32::try_from(source.len()).unwrap();
        assert!(guard.contains_span(Span::new(end, end)));
        assert!(!guard.contains_span(Span::new(end, end + 1)));
        assert!(!current.matches_original_source_image(
            &SourceImage::native(source.as_bytes().to_vec()),
            current.body_lexer_config.unwrap()
        ));
    }
}
