// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Existing W311 advice from original selected channel-configuration syntax.

use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;

fn original_static_literal(
    original: &OriginalDiagnosticInvocation,
    argument: usize,
) -> Option<String> {
    let bytes =
        tcl_syntax::word_rules::original_static_word_source_bytes(original.word(argument)?)?;
    String::from_utf8(bytes).ok()
}

fn channel_diagnostic(original: &OriginalDiagnosticInvocation) -> Option<Diagnostic> {
    let layout =
        original.with_schema(|schema| schema.authored_source_channel_configuration())??;
    let encoding = original_static_literal(original, layout.encoding)?;
    let translation = original_static_literal(original, layout.translation)?;
    if encoding != layout.binary_value || translation == layout.binary_value {
        return None;
    }
    let subject = original.subject(
        RegistrySourceDiagnosticKind::ChannelConfiguration,
        Some(layout.translation),
    )?;
    Some(
        Diagnostic::new(
            DiagCode::W311,
            original.word(layout.translation)?.span(),
            "Channel configured with -encoding binary and a non-binary \
                              -translation. Binary encoding implies no translation; the \
                              conflicting -translation may silently corrupt data or enable \
                              encoding-differential attacks."
                .to_owned(),
            Severity::Warning,
        )
        .with_subject(subject),
    )
}

impl Analyser {
    /// Selected source relationship advice supplies no channel configuration,
    /// runtime option value, completion or edit permission.
    pub(in crate::analyser) fn emit_w311_encoding_mismatch(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        if let Some(diagnostic) = original
            .filter(|original| original.matches_analysis(&self.result, &self.source))
            .and_then(channel_diagnostic)
        {
            self.result.diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{AnalysisResult, DiagnosticSubject, ResolvedAnalysisInput};
    use std::sync::Arc;

    fn input(context: Arc<tcl_registry::model::ContextRegistry>) -> ResolvedAnalysisInput {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        input
    }
    fn current_input() -> ResolvedAnalysisInput {
        input(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        )
    }
    fn analyse(source: &str, input: &ResolvedAnalysisInput) -> Analyser {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        analyser.analyse(source, "tcl");
        analyser
    }
    fn findings(result: &AnalysisResult) -> Vec<&Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagCode::W311)
            .collect()
    }

    #[test]
    fn original_channel_configuration_keeps_available_aliases_prefixes_and_captured_values() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for (source, argument, written, value) in [
            (
                "fconfigure $channel -encoding binary -translation crlf",
                4,
                Some(4),
                "crlf",
            ),
            (
                "chan configure $channel -encoding binary -translation {é[$literal]}",
                5,
                Some(5),
                "{é[$literal]}",
            ),
            (
                "rename fconfigure Held; Held $channel -enc binary -trans crlf",
                4,
                Some(4),
                "crlf",
            ),
            (
                "interp alias {} configure {} chan configure; configure $channel -encoding binary -translation crlf",
                5,
                Some(4),
                "crlf",
            ),
            (
                "interp alias {} configure {} fconfigure stdout -encoding binary -translation crlf; rename configure moved; moved",
                4,
                None,
                "crlf",
            ),
            (
                "fconfigure stdout -encoding utf-8 -translation auto -encoding binary -translation crlf",
                8,
                Some(8),
                "crlf",
            ),
        ] {
            let analyser = analyse(source, &input);
            let found = findings(&analyser.result);
            assert_eq!(
                found.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            assert_eq!(&source[found[0].span.as_range()], value);
            assert!(found[0].fixes.is_empty());
            let Some(DiagnosticSubject::RegistrySource(subject)) = found[0].subject() else {
                panic!("original channel option subject")
            };
            assert_eq!(
                subject.kind(),
                RegistrySourceDiagnosticKind::ChannelConfiguration
            );
            assert_eq!(subject.argument(), Some(argument));
            assert_eq!(subject.written_argument(), written);
            assert!(subject.supplemental_literals().is_empty());
        }
    }

    #[test]
    fn original_channel_configuration_refuses_replacements_and_unclosed_or_dynamic_values() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for source in [
            "proc fconfigure args {}; fconfigure stdout -encoding binary -translation crlf",
            "namespace eval custom {proc fconfigure args {}}; custom::fconfigure stdout -encoding binary -translation crlf",
            "interp alias {} configure {} fconfigure; rename fconfigure Held; configure stdout -encoding binary -translation crlf",
            "interp alias {} configure {} fconfigure; rename fconfigure {}; configure stdout -encoding binary -translation crlf",
            "chan $selector stdout -encoding binary -translation crlf",
            "fconfigure stdout -encoding binary -translation $mode",
            "fconfigure stdout -encoding $mode -translation crlf",
            "fconfigure stdout -encoding binary -translation crlf -encoding $mode",
            "fconfigure stdout -encoding binary -translation crlf -translation binary",
            "fconfigure stdout -encoding binary -translation crlf -encoding utf-8",
            "fconfigure stdout -encoding binary -translation",
            "fconfigure stdout -encoding binary -translation crlf $option value",
            "fconfigure stdout -encoding binary -translation crlf -unknown value",
            "fconfigure stdout -encoding binary -translation crlf {*}$options",
        ] {
            assert!(
                findings(&analyse(source, &input).result).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_channel_configuration_keeps_actual_availability_and_source_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut configured = registry.get("fconfigure").unwrap().clone();
        configured.name = "configuration_current";
        configured.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(configured);
        let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        assert!(Arc::ptr_eq(current.commands(), older.commands()));
        let source = "configuration_current stdout -encoding binary -translation crlf";
        let input = self::input(current);
        assert_eq!(findings(&analyse(source, &input).result).len(), 1);
        assert!(findings(&analyse(source, &self::input(older)).result).is_empty());
        let mut analyser = analyse(source, &input);
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        )
        .pop()
        .unwrap();
        let original = analyser
            .original_diagnostic_call_at(segment.argv[0], segment.arg_tokens())
            .unwrap();
        for axis in 0..4 {
            let mut changed = analyser.result.clone();
            match axis {
                0 => changed.resolved_input = None,
                1 => changed.resolved_input = Some(current_input()),
                2 => changed.body_lexer_config = None,
                _ => {
                    let mut config = input.lexer_config();
                    config.strict_quoting = !config.strict_quoting;
                    changed.resolved_input = Some(ResolvedAnalysisInput::new(
                        input.unit_profile(),
                        input.unit_profile(),
                        input.context_registry(),
                        config,
                    ));
                }
            }
            assert!(
                !original.matches_analysis(&changed, source),
                "owner axis {axis}"
            );
        }
        assert!(!original.matches_analysis(
            &analyser.result,
            "configuration_current stdout -encoding binary -translation auto"
        ));
        analyser.result.resolved_input = None;
        analyser.result.diagnostics.clear();
        analyser.emit_w311_encoding_mismatch(Some(&original));
        assert!(findings(&analyser.result).is_empty());
    }
}
