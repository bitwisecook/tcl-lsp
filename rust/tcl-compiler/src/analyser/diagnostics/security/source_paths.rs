// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Existing channel/file advice from selected original path operands.

use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind,
};
use crate::analyser::state::Analyser;
use crate::analyser::DiagnosticSubject;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::Traits;

/// Declared post-head sink ordinals retain the command descriptor's coordinate,
/// including captured prefixes. Access/permission operands do not influence
/// pipeline classification. No current channel is established.
fn channel_path_arguments(schema: &tcl_registry::ResolvedInvocation<'_, '_>) -> Option<Vec<usize>> {
    if !schema.semantics.traits.contains(Traits::OPENS_CHANNEL)
        || schema.semantics.traits.contains(Traits::TAINT_SOURCE)
    {
        return None;
    }
    let count = schema.words.arguments().exact_argv_len()?;
    let selected = schema.authored_source_descriptors();
    Some(
        selected
            .command
            .taint_code_sink_args?
            .iter()
            .map(|&index| usize::from(index))
            .filter(|&index| index < count)
            .collect(),
    )
}

fn source_file_argument(schema: &tcl_registry::ResolvedInvocation<'_, '_>) -> Option<usize> {
    schema.authored_source_file_argument()
}

fn path_arguments(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Vec<(DiagCode, RegistrySourceDiagnosticKind, usize)> {
    let mut positions = channel_path_arguments(schema)
        .unwrap_or_default()
        .into_iter()
        .map(|argument| {
            (
                DiagCode::W103,
                RegistrySourceDiagnosticKind::ChannelPath,
                argument,
            )
        })
        .collect::<Vec<_>>();
    if let Some(argument) = source_file_argument(schema) {
        positions.push((
            DiagCode::W300,
            RegistrySourceDiagnosticKind::SourceFilePath,
            argument,
        ));
    }
    positions
}

fn pipeline_prefix(word: &tcl_lexer::NativeWord) -> bool {
    let arena = word.executable_parts();
    arena
        .list(arena.root())
        .first()
        .and_then(|part| arena.text(part))
        .is_some_and(|text| text.starts_with(b"|"))
}

/// A proven value is an independent advisory facet; the original source word
/// and captured producer ordinal stay unchanged. Unknown dynamic words keep
/// the existing warning. These findings establish no native channel/file entry.
fn path_diagnostic(
    original: &OriginalDiagnosticInvocation,
    argument: usize,
    kind: RegistrySourceDiagnosticKind,
) -> Option<Diagnostic> {
    let word = original.word(argument)?;
    let command = original.command();
    let value = original.literal(argument);
    let dynamic = super::super::usage::original_word_has_substitution(word);
    let (code, severity, message) = match kind {
        RegistrySourceDiagnosticKind::ChannelPath => {
            if let Some(value) = value {
                if !value.starts_with('|') {
                    return None;
                }
                (
                    DiagCode::W103,
                    Severity::Hint,
                    format!(
                        "{command} with a pipeline (\"|\") executes an external command. Ensure the command is not influenced by untrusted input."
                    ),
                )
            } else if dynamic && pipeline_prefix(word) {
                (
                    DiagCode::W103,
                    Severity::Warning,
                    format!(
                        "{command} with a pipeline containing variable/command substitution risks command injection. Validate and sanitize the command before passing to {command}."
                    ),
                )
            } else if dynamic {
                (
                    DiagCode::W103,
                    Severity::Warning,
                    format!(
                        "{command} with a dynamic argument (variable or command substitution): if the value starts with \"|\", it will execute a command pipeline. Validate input or use explicit I/O commands."
                    ),
                )
            } else {
                return None;
            }
        }
        RegistrySourceDiagnosticKind::SourceFilePath if value.is_none() && dynamic => (
            DiagCode::W300,
            Severity::Warning,
            format!(
                "{command} with a dynamic path (variable or command substitution) executes arbitrary Tcl code. Ensure the path is not influenced by untrusted input."
            ),
        ),
        _ => return None,
    };
    let subject = original.subject(kind, Some(argument))?;
    Some(Diagnostic::new(code, word.span(), message, severity).with_subject(subject))
}

impl Analyser {
    pub(in crate::analyser) fn emit_w103_open_pipeline(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let Some(arguments) = original.with_schema(channel_path_arguments).flatten() else {
            return;
        };
        self.result
            .diagnostics
            .extend(arguments.into_iter().filter_map(|argument| {
                path_diagnostic(
                    original,
                    argument,
                    RegistrySourceDiagnosticKind::ChannelPath,
                )
            }));
    }

    pub(in crate::analyser) fn emit_w300_source_variable(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if !original.matches_analysis(&self.result, &self.source) {
            return;
        }
        let Some(argument) = original.with_schema(source_file_argument).flatten() else {
            return;
        };
        self.result.diagnostics.extend(path_diagnostic(
            original,
            argument,
            RegistrySourceDiagnosticKind::SourceFilePath,
        ));
    }

    /// Called after the whole-CU WordIndex proves an original written value at
    /// its authentic source point. Missing/stale CUs retain the old warning;
    /// captures never borrow the calling site's value proof.
    pub(in crate::analyser) fn refine_original_path_arguments(
        &mut self,
        written: &OriginalDiagnosticInvocation,
        projected: &OriginalDiagnosticInvocation,
    ) {
        if written.words() != projected.words()
            || !projected.matches_analysis(&self.result, &self.source)
        {
            return;
        }
        let (Some(written_positions), Some(positions)) = (
            written.with_schema(path_arguments),
            projected.with_schema(path_arguments),
        ) else {
            return;
        };
        for (code, kind, argument) in positions {
            let value_proven = projected
                .supplemental_literals()
                .iter()
                .any(|(ordinal, _)| *ordinal == argument);
            if !value_proven && written_positions.contains(&(code, kind, argument)) {
                continue;
            }

            self.result.diagnostics.retain(|diagnostic| !(diagnostic.code == code
                && matches!(diagnostic.subject(), Some(DiagnosticSubject::RegistrySource(subject))
                    if subject.kind() == kind && subject.words() == written.words()
                        && subject.argument() == Some(argument))));
            self.result
                .diagnostics
                .extend(path_diagnostic(projected, argument, kind));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
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
    fn current_input(dialect: &str) -> ResolvedAnalysisInput {
        input(tcl_registry::model::ingress::resolve_environment(dialect).default_context_registry())
    }
    fn analyse(source: &str, input: &ResolvedAnalysisInput) -> Analyser {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser
    }
    fn findings(result: &AnalysisResult, code: DiagCode) -> Vec<&Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|finding| finding.code == code)
            .collect()
    }
    fn cu(source: &str, input: &ResolvedAnalysisInput) -> crate::compilation_unit::CompilationUnit {
        crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
    }
    fn original_at(
        analyser: &Analyser,
        source: &str,
        call: &str,
    ) -> (
        OriginalDiagnosticInvocation,
        crate::segmenter::SegmentedCommand,
    ) {
        let base = u32::try_from(source.find(call).unwrap()).unwrap();
        let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
            call,
            base,
            analyser.lexer_config(),
        );
        let segment = segments.pop().unwrap();
        let original = analyser
            .original_diagnostic_call_at(segment.argv[0], segment.arg_tokens())
            .expect("the original whole-source call");
        (original, segment)
    }
    fn queue_written(analyser: &mut Analyser, source: &str, call: &str, code: DiagCode) {
        analyser
            .result
            .diagnostics
            .retain(|finding| finding.code != code);
        let (original, segment) = original_at(analyser, source, call);
        analyser.proven_sites.clear();
        if code == DiagCode::W103 {
            analyser.emit_w103_open_pipeline(Some(&original));
        } else {
            analyser.emit_w300_source_variable(Some(&original));
        }
        analyser.record_proven_site(&crate::analyser::diagnostics::CallWords {
            cmd_tok: segment.argv[0],
            args: segment.args(),
            arg_tokens: segment.arg_tokens(),
            arg_single: segment.arg_single_token(),
            arg_expand_in: segment.expand_word.as_deref().unwrap_or(&[]),
        });
        assert_eq!(
            findings(&analyser.result, code).len(),
            1,
            "original dynamic advice"
        );
    }

    #[test]
    fn original_path_advice_keeps_alias_captures_literal_modes_and_unicode_spans() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input("tcl8.6");
        for (source, code, argument, written, operand) in [
            (
                "interp alias {} channel {} open; channel \"pré $path\"",
                DiagCode::W103,
                0,
                Some(0),
                "\"pré $path\"",
            ),
            (
                "interp alias {} channel {} open {|literal}; channel $mode",
                DiagCode::W103,
                0,
                None,
                "{|literal}",
            ),
            (
                "interp alias {} load {} source -encoding utf-8; load \"rép/$path\"",
                DiagCode::W300,
                2,
                Some(0),
                "\"rép/$path\"",
            ),
            (
                "rename source Held; Held $path",
                DiagCode::W300,
                0,
                Some(0),
                "$path",
            ),
        ] {
            let analyser = analyse(source, &input);
            let diagnostics = findings(&analyser.result, code);
            assert_eq!(
                diagnostics.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            assert_eq!(&source[diagnostics[0].span.as_range()], operand);
            assert_eq!(
                diagnostics[0].severity,
                if written.is_none() {
                    Severity::Hint
                } else {
                    Severity::Warning
                }
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
                panic!("original typed path subject")
            };
            assert_eq!(subject.argument(), Some(argument));
            assert_eq!(subject.written_argument(), written);
        }
        for (source, code) in [
            ("open {|literal} $mode", DiagCode::W103),
            ("open {literal.txt} $mode", DiagCode::W103),
            ("socket $host 80", DiagCode::W103),
            ("source -encoding $encoding literal.tcl", DiagCode::W300),
        ] {
            let diagnostics = analyse(source, &input);
            let expected = usize::from(source.starts_with("open {|"));
            assert_eq!(
                findings(&diagnostics.result, code).len(),
                expected,
                "{source}"
            );
            if expected == 1 {
                assert_eq!(
                    findings(&diagnostics.result, code)[0].severity,
                    Severity::Hint
                );
            }
        }
    }

    #[test]
    fn original_path_advice_respects_known_targets_options_and_actual_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let current = current_input("tcl8.6");
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.context_registry().commands())),
        );
        assert!(Arc::ptr_eq(
            current.context_registry().commands(),
            older.commands()
        ));
        assert_eq!(
            findings(
                &analyse("source -encoding utf-8 $path", &current).result,
                DiagCode::W300
            )
            .len(),
            1
        );
        assert!(
            findings(
                &analyse("source -encoding utf-8 $path", &input(older)).result,
                DiagCode::W300
            )
            .is_empty()
        );
        assert_eq!(
            findings(
                &analyse("source -nopkg $path", &current_input("tcl9.0")).result,
                DiagCode::W300
            )
            .len(),
            1
        );
        assert!(
            findings(
                &analyse("source -nopkg $path", &current).result,
                DiagCode::W300
            )
            .is_empty()
        );
        for (source, code) in [
            ("proc open args {return CUSTOM}; open $path", DiagCode::W103),
            (
                "interp alias {} channel {} open; rename open Held; channel $path",
                DiagCode::W103,
            ),
            (
                "namespace eval custom {proc source args {return CUSTOM}}; custom::source $path",
                DiagCode::W300,
            ),
            (
                "interp alias {} load {} source; rename source {}; load $path",
                DiagCode::W300,
            ),
            ("source -enc utf-8 $path", DiagCode::W300),
            ("source $option utf-8 $path", DiagCode::W300),
            ("source {*}$paths", DiagCode::W300),
        ] {
            let analyser = analyse(source, &current);
            assert!(
                findings(&analyser.result, code).is_empty(),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
        }
    }

    #[test]
    fn original_path_values_use_the_complete_cu_at_the_written_source_point() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input("tcl8.6");
        for (source, call, code, expected) in [
            (
                "proc f {} {set p literal.txt; open $p}",
                "open $p",
                DiagCode::W103,
                None,
            ),
            (
                "proc f {} {set p {|literal}; open $p}",
                "open $p",
                DiagCode::W103,
                Some(Severity::Hint),
            ),
            (
                "interp alias {} store {} set; proc f {} {store p lib.tcl; source $p}",
                "source $p",
                DiagCode::W300,
                None,
            ),
            (
                "interp alias {} load {} source -encoding utf-8; proc f {} {set p lib.tcl; load $p}",
                "load $p",
                DiagCode::W300,
                None,
            ),
            (
                "proc f {} {set {$p} {|literal}; open ${$p}}",
                "open ${$p}",
                DiagCode::W103,
                Some(Severity::Hint),
            ),
            (
                "proc f {} {set scalar(open {|literal}; open ${scalar(open}}",
                "open ${scalar(open}",
                DiagCode::W103,
                Some(Severity::Hint),
            ),
        ] {
            let unit = cu(source, &input);
            assert!(unit.function("::f").is_some(), "actual source procedure");
            let mut analyser = analyse(source, &input);
            queue_written(&mut analyser, source, call, code);
            analyser.emit_proven_word_diagnostics(&unit);
            let diagnostics = findings(&analyser.result, code);
            assert_eq!(
                diagnostics.len(),
                usize::from(expected.is_some()),
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            if let Some(severity) = expected {
                assert_eq!(diagnostics[0].severity, severity);
                let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject()
                else {
                    panic!("proven source subject")
                };
                assert_eq!(
                    subject.supplemental_literals(),
                    &[(0, "|literal".to_owned())]
                );
                assert_eq!(
                    &source[diagnostics[0].span.as_range()],
                    call.strip_prefix("open ").unwrap()
                );
            }
        }
        for source in [
            "proc f {p} {open $p}",
            "proc f {} {set p literal.txt; set p $unknown; open $p}",
            "proc set args {return CUSTOM}; proc f {} {set p literal.txt; open $p}",
        ] {
            let analyser = analyse(source, &input);
            assert_eq!(
                findings(&analyser.result, DiagCode::W103).len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            assert_eq!(
                findings(&analyser.result, DiagCode::W103)[0].severity,
                Severity::Warning
            );
        }
    }

    #[test]
    fn original_source_path_warning_keeps_a_proven_option_and_unproven_filename() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input("tcl8.6");
        let source = "proc f {path} {set option -encoding; source $option utf-8 $path}";
        let analyser = analyse(source, &input);
        let diagnostics = findings(&analyser.result, DiagCode::W300);
        assert_eq!(
            diagnostics.len(),
            1,
            "a proven option retains the unproven path warning"
        );
        let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
            panic!("selected file form")
        };
        assert_eq!(subject.argument(), Some(2));
        assert_eq!(
            subject.supplemental_literals(),
            &[(0, "-encoding".to_owned())]
        );
    }

    #[test]
    fn original_path_refinement_cannot_borrow_missing_foreign_or_stale_cu_inputs() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input("tcl8.6");
        let source = "proc f {} {set p literal.txt; open $p}";
        let unit = cu(source, &input);
        assert!(unit.function("::f").is_some());
        for axis in 0..6 {
            let mut stale = unit.clone();
            match axis {
                0 => stale.ir_module.source_metadata_input = None,
                1 => {
                    stale.ir_module.source =
                        tcl_lexer::SourceImage::document("proc f {} {set p literal.txt; open $q}")
                }
                2 => {
                    stale
                        .procedures
                        .values_mut()
                        .next()
                        .unwrap()
                        .source_metadata_input = None
                }
                3 => stale.ir_module.source_metadata_input = Some(current_input("tcl9.0")),
                4 => {
                    stale.ir_module.lexer_config.strict_quoting =
                        !stale.ir_module.lexer_config.strict_quoting
                }
                _ => stale.ir_module.retained_source_bindings = None,
            }
            let mut analyser = analyse(source, &input);
            queue_written(&mut analyser, source, "open $p", DiagCode::W103);
            let before = findings(&analyser.result, DiagCode::W103)[0].clone();
            analyser.emit_proven_word_diagnostics(&stale);
            assert_eq!(
                findings(&analyser.result, DiagCode::W103),
                vec![&before],
                "CU owner axis {axis}"
            );
        }
    }

    #[test]
    fn original_path_advice_requires_the_unchanged_analysis_input_and_image() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input("tcl8.6");
        let analyser = analyse("open $path; source $path", &input);
        let originals = ["open $path", "source $path"]
            .map(|call| original_at(&analyser, "open $path; source $path", call).0);
        for axis in 0..4 {
            let mut changed = analyse("open $path; source $path", &input);
            changed.result.diagnostics.clear();
            match axis {
                0 => changed.result.resolved_input = None,
                1 => {
                    let mut changed_input = input.clone();
                    changed_input.config.strict_quoting = !changed_input.config.strict_quoting;
                    changed.result.resolved_input = Some(changed_input);
                }
                2 => changed.source = "open $other; source $other".to_owned(),
                _ => changed.result.resolved_input = Some(current_input("tcl9.0")),
            }
            changed.emit_w103_open_pipeline(Some(&originals[0]));
            changed.emit_w300_source_variable(Some(&originals[1]));
            assert!(
                findings(&changed.result, DiagCode::W103).is_empty(),
                "original owner axis {axis}"
            );
            assert!(
                findings(&changed.result, DiagCode::W300).is_empty(),
                "original owner axis {axis}"
            );
        }
    }

    #[test]
    fn original_path_advice_has_whole_source_and_per_item_value_parity() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input("tcl8.6");
        for source in [
            "proc f {} {set p {|literal}; open $p}",
            "proc f {} {set p lib.tcl; source $p}",
            "interp alias {} channel {} open; proc f {p} {channel $p}",
        ] {
            let whole = analyse(source, &input);
            let mut incremental = Analyser::new().with_resolved_input(input.clone());
            let incremental = incremental.analyse_per_item(source, "tcl");
            for code in [DiagCode::W103, DiagCode::W300] {
                assert_eq!(
                    findings(&whole.result, code),
                    findings(&incremental, code),
                    "{source}"
                );
            }
        }
    }
}
