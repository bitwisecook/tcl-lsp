// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Existing literal regex-shape advice at original selected pattern operands.

use crate::analyser::DiagnosticSubject;
use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind, source_regex_pattern_arguments,
};
use crate::analyser::state::Analyser;
use crate::analyser::types::{Diagnostic, Severity};
use tcl_core_types::DiagCode;
use tcl_registry::spec::CaseMatchMode;
use tcl_syntax::word_rules::OriginalSourceListElement;

struct OriginalPattern {
    argument: usize,
    path: Vec<usize>,
    value: String,
}

fn field_patterns(
    argument: usize,
    path: Vec<usize>,
    field: &OriginalSourceListElement,
    case: tcl_registry::CaseListSpec,
) -> Option<Vec<OriginalPattern>> {
    if case.pattern_is_list(field.value()) {
        return Some(
            field
                .elements()?
                .into_iter()
                .enumerate()
                .map(|(ordinal, child)| {
                    let mut path = path.clone();
                    path.push(ordinal);
                    OriginalPattern {
                        argument,
                        path,
                        value: child.value().to_owned(),
                    }
                })
                .collect(),
        );
    }
    Some(vec![OriginalPattern {
        argument,
        path,
        value: field.value().to_owned(),
    }])
}

/// The shared case parser owns flag geometry. Only the selected descriptor
/// interprets a resolved flag's mode and whether the next field is its value.
fn clause_is_regex(
    clause: &tcl_syntax::case_list::Clause,
    value: &str,
    case: tcl_registry::CaseListSpec,
    outer: CaseMatchMode,
) -> Option<bool> {
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: case.clause_flags,
        clause_value_flags: case.clause_value_flags,
    };
    let mut regexp = outer == CaseMatchMode::Regexp;
    let mut flags = clause.flags.iter();
    while let Some(flag) = flags.next() {
        let canonical = shape.resolve_flag(value.get(flag.content_range())?)?;
        regexp |= case.clause_regex_flag == Some(canonical);
        if shape.flag_takes_value(canonical) {
            flags.next()?;
        }
    }
    Some(regexp)
}

fn clause_list_patterns(
    original: &OriginalDiagnosticInvocation,
    argument: usize,
    case: tcl_registry::CaseListSpec,
    mode: CaseMatchMode,
) -> Option<Vec<OriginalPattern>> {
    let word = original.word(argument)?;
    let source_value = tcl_syntax::word_rules::original_static_word_source_bytes(word)?;
    let value = std::str::from_utf8(&source_value).ok()?;
    // A proven computed list has no original field positions. It cannot borrow
    // the written variable word as if its value occupied that source range.
    (original.literal(argument)? == value).then_some(())?;
    let fields = tcl_syntax::word_rules::original_static_word_list_elements(word)?;
    let shape = tcl_syntax::case_list::CaseListShape {
        clause_flags: case.clause_flags,
        clause_value_flags: case.clause_value_flags,
    };
    let clauses =
        tcl_syntax::case_list::split_case_list_with_syntax(value, &shape, word.config().list_parse);
    let mut patterns = Vec::new();
    for (ordinal, clause) in clauses.iter().enumerate() {
        if !clause.valid
            || clause.pattern.is_none()
            || clause.body.is_none()
                && !(case.allow_omitted_final_body && ordinal + 1 == clauses.len())
        {
            return None;
        }
        if !clause_is_regex(clause, value, case, mode)? {
            continue;
        }
        let field_index = clause.pattern_index?;
        let field = fields.get(field_index)?;
        if !case.is_keyword_pattern(field.value(), ordinal, clauses.len()) {
            patterns.extend(field_patterns(argument, vec![field_index], field, case)?);
        }
    }
    Some(patterns)
}

fn case_patterns(original: &OriginalDiagnosticInvocation) -> Option<Vec<OriginalPattern>> {
    let (case, layout) =
        original.with_schema(|schema| schema.authored_source_case_pattern_layout())??;
    if let Some(argument) = layout.clause_list_index {
        return clause_list_patterns(original, argument, case, layout.mode);
    }
    let start = layout.inline_clause_start?;
    let values = (0..original.words().arguments().len())
        .map(|index| original.literal(index))
        .collect::<Vec<_>>();
    let clauses = case.inline_clause_positions(&values, start)?;
    let mut patterns = Vec::new();
    for (ordinal, clause) in clauses.iter().enumerate() {
        if layout.mode != CaseMatchMode::Regexp && clause.mode != CaseMatchMode::Regexp {
            continue;
        }
        let argument = clause.pattern_index;
        let Some(value) = original.literal(argument) else {
            continue;
        };
        if case.is_keyword_pattern(value, ordinal, clauses.len()) {
            continue;
        }
        if case.pattern_is_list(value) {
            let fields = tcl_syntax::word_rules::original_static_word_list_elements(
                original.word(argument)?,
            )?;
            patterns.extend(
                fields
                    .into_iter()
                    .enumerate()
                    .map(|(index, field)| OriginalPattern {
                        argument,
                        path: vec![index],
                        value: field.value().to_owned(),
                    }),
            );
        } else {
            patterns.push(OriginalPattern {
                argument,
                path: Vec::new(),
                value: value.to_owned(),
            });
        }
    }
    Some(patterns)
}

fn original_patterns(original: &OriginalDiagnosticInvocation) -> Vec<OriginalPattern> {
    let mut patterns = original
        .with_schema(source_regex_pattern_arguments)
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|argument| {
            Some(OriginalPattern {
                argument,
                path: Vec::new(),
                value: original.literal(argument)?.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    patterns.extend(case_patterns(original).unwrap_or_default());
    patterns.sort_by(|a, b| (a.argument, &a.path).cmp(&(b.argument, &b.path)));
    patterns.dedup_by(|a, b| a.argument == b.argument && a.path == b.path);
    patterns
}

fn pattern_diagnostics(original: &OriginalDiagnosticInvocation) -> Vec<Diagnostic> {
    original_patterns(original).into_iter().filter_map(|pattern| {
        if !super::has_redos_shape(&pattern.value) { return None; }
        let subject = if pattern.path.is_empty() {
            original.subject(RegistrySourceDiagnosticKind::PatternShape, Some(pattern.argument))
        } else {
            original.list_element_subject(RegistrySourceDiagnosticKind::PatternShape, pattern.argument, &pattern.path)
        }?;
        let DiagnosticSubject::RegistrySource(subject) = subject else { return None };
        Some(Diagnostic::new(DiagCode::W303, subject.span(),
            "Regular expression may be vulnerable to catastrophic backtracking (ReDoS). Nested quantifiers like (a+)+ can cause exponential matching time on crafted input.".to_owned(),
            Severity::Warning).with_subject(DiagnosticSubject::RegistrySource(subject)))
    }).collect()
}

impl Analyser {
    /// W303 retains selected pattern syntax and the bounded literal shape
    /// detector. It grants no regex evaluation, runtime or matching behaviour.
    pub(in crate::analyser) fn emit_w303_redos(
        &mut self,
        original: Option<&OriginalDiagnosticInvocation>,
    ) {
        let Some(original) = original else { return };
        if original.matches_analysis(&self.result, &self.source) {
            self.result
                .diagnostics
                .extend(pattern_diagnostics(original));
        }
    }

    /// The full-CU value owner may resolve an option that selects a still
    /// literal pattern elsewhere. Its original field geometry is independent
    /// of the proven option's extent; no generic token-span filter can join it.
    pub(in crate::analyser) fn refine_original_pattern_arguments(
        &mut self,
        written: &OriginalDiagnosticInvocation,
        projected: &OriginalDiagnosticInvocation,
    ) {
        if written.words() != projected.words()
            || !written.matches_analysis(&self.result, &self.source)
            || !projected.matches_analysis(&self.result, &self.source)
        {
            return;
        }
        self.result.diagnostics.retain(|diagnostic| !(diagnostic.code == DiagCode::W303
            && matches!(diagnostic.subject(), Some(DiagnosticSubject::RegistrySource(subject))
                if subject.kind() == RegistrySourceDiagnosticKind::PatternShape && subject.words() == written.words())));
        self.result
            .diagnostics
            .extend(pattern_diagnostics(projected));
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
    fn current_input() -> ResolvedAnalysisInput {
        input(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
        )
    }
    fn analyse(source: &str, input: &ResolvedAnalysisInput) -> Analyser {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        analyser.analyse_and_retain_result_for_test(source, "tcl");
        analyser
    }
    fn findings(result: &AnalysisResult) -> Vec<&Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|finding| finding.code == DiagCode::W303)
            .collect()
    }
    fn original_at(analyser: &Analyser, source: &str, call: &str) -> OriginalDiagnosticInvocation {
        let start = u32::try_from(source.find(call).unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            call,
            start,
            analyser.lexer_config(),
        )
        .pop()
        .unwrap();
        analyser
            .original_diagnostic_call_at(segment.argv[0], segment.arg_tokens())
            .expect("original pattern source")
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

    fn queue_written(analyser: &mut Analyser, source: &str, call: &str) {
        let start = u32::try_from(source.find(call).unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            call,
            start,
            analyser.lexer_config(),
        )
        .pop()
        .unwrap();
        analyser
            .original_diagnostic_call_at(segment.argv[0], segment.arg_tokens())
            .expect("original CU query site");
        analyser.proven_sites.clear();
        analyser.record_proven_site(&crate::analyser::diagnostics::CallWords {
            cmd_tok: segment.argv[0],
            args: segment.args(),
            arg_tokens: segment.arg_tokens(),
            arg_single: segment.arg_single_token(),
            arg_expand_in: segment.expand_word.as_deref().unwrap_or(&[]),
        });
        assert_eq!(analyser.proven_sites.len(), 1);
    }

    #[test]
    fn original_pattern_shape_keeps_captured_ordinals_and_list_field_spans() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for (source, argument, written, path, span) in [
            (
                "interp alias {} rx {} regexp -nocase {(é+)+}; rx literal",
                1,
                None,
                vec![],
                "{(é+)+}",
            ),
            (
                "interp alias {} rx {} regexp -nocase; rename rx moved; moved {(é+)+} literal",
                1,
                Some(0),
                vec![],
                "{(é+)+}",
            ),
            (
                "rename regexp Held; Held \"(é+)+\" literal",
                0,
                Some(0),
                vec![],
                "\"(é+)+\"",
            ),
            (
                "interp alias {} choose {} switch -regexp; choose $s {(é+)+ {BODY} default {OTHER}}",
                2,
                Some(1),
                vec![0],
                "(é+)+",
            ),
            (
                "interp alias {} choose {} switch -regexp literal {(é+)+ {BODY}}; choose",
                2,
                None,
                vec![0],
                "(é+)+",
            ),
            (
                "switch -regexp $s {(a+)+ {FIRST} (é+)+ {SECOND} default {OTHER}}",
                2,
                Some(2),
                vec![0],
                "(a+)+",
            ),
        ] {
            let analyser = analyse(source, &input);
            let found = findings(&analyser.result);
            assert_eq!(
                found.len(),
                if source.contains("SECOND") { 2 } else { 1 },
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            assert_eq!(&source[found[0].span.as_range()], span);
            assert_eq!(found[0].severity, Severity::Warning);
            let Some(DiagnosticSubject::RegistrySource(subject)) = found[0].subject() else {
                panic!("selected pattern subject")
            };
            assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::PatternShape);
            assert_eq!(subject.argument(), Some(argument));
            assert_eq!(subject.written_argument(), written);
            assert_eq!(subject.list_element_path(), path.as_slice());
            if found.len() == 2 {
                let Some(DiagnosticSubject::RegistrySource(second)) = found[1].subject() else {
                    panic!("second field")
                };
                assert_eq!(second.list_element_path(), &[2]);
                assert_eq!(&source[second.span().as_range()], "(é+)+");
            }
        }
    }

    #[test]
    fn original_pattern_shape_uses_selected_modes_targets_and_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut regex = registry.get("regexp").unwrap().clone();
        regex.name = "pattern_current";
        regex.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(regex);
        let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(current.commands())),
        );
        assert!(Arc::ptr_eq(current.commands(), older.commands()));
        let source = "pattern_current {(a+)+} literal";
        assert_eq!(
            findings(&analyse(source, &input(Arc::clone(&current))).result).len(),
            1
        );
        assert!(findings(&analyse(source, &input(older)).result).is_empty());
        let current = input(current);
        for source in [
            "regsub -all {(a+)+} literal replacement out",
            "lsearch -regexp $items {(a+)+}",
            "switch -regexp $s {(a+)+} $body default $other",
            "switch -reg $s {(a+)+ {BODY}}",
        ] {
            assert_eq!(
                findings(&analyse(source, &current).result).len(),
                1,
                "{source}"
            );
        }
        for source in [
            "proc regexp args {return CUSTOM}; regexp {(a+)+} literal",
            "namespace eval custom {proc regexp args {return CUSTOM}}; custom::regexp {(a+)+} literal",
            "interp alias {} rx {} regexp; rename regexp Held; rx {(a+)+} literal",
            "interp alias {} rx {} regexp; rename regexp {}; rx {(a+)+} literal",
            "regexp $option {(a+)+} literal",
            "regexp {*}$options {(a+)+} literal",
            "switch $option $s {(a+)+ {BODY}}",
            "switch -glob $s {(a+)+ {BODY}}",
            "switch -exact $s {(a+)+ {BODY}}",
            "string match {(a+)+} literal",
            "lsearch $items {(a+)+}",
        ] {
            assert!(
                findings(&analyse(source, &current).result).is_empty(),
                "{source}"
            );
        }
    }

    #[test]
    fn original_pattern_list_geometry_keeps_the_full_selected_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input();
        let source =
            r"switch -regexp $s {\(é+\)+ {BODY} # {COMMENT_PATTERN} ; {SEPARATOR_PATTERN}}";
        let analyser = analyse(source, &input);
        let found = findings(&analyser.result);
        assert_eq!(found.len(), 1);
        assert_eq!(&source[found[0].span.as_range()], r"\(é+\)+");
        let malformed = "switch -regexp $s {(é+)+ \"unterminated}";
        assert!(findings(&analyse(malformed, &input).result).is_empty());
        let mut lenient = input.clone();
        lenient.config.list_parse = tcl_dialect::ListParse::Lenient;
        let lenient_result = analyse(malformed, &lenient);
        assert_eq!(findings(&lenient_result.result).len(), 1);
        assert_eq!(
            &malformed[findings(&lenient_result.result)[0].span.as_range()],
            "(é+)+"
        );
        // A computed clause list retains a value but cannot acquire field spans.
        let source = "switch -regexp $s $clauses";
        let analyser = analyse(source, &input);
        let written = original_at(&analyser, source, source);
        let projected = written
            .with_supplemental_literals(&[(2, "(a+)+ {BODY}".to_owned())])
            .unwrap();
        assert!(case_patterns(&projected).is_none());
        assert!(
            projected
                .list_element_subject(RegistrySourceDiagnosticKind::PatternShape, 2, &[0])
                .is_none()
        );
    }

    #[test]
    fn original_pattern_shape_values_require_the_complete_original_cu() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input();
        for source in [
            "proc f {} {set p {(a+)+}; regexp $p literal}",
            "interp alias {} store {} set; proc f {} {store p {(a+)+}; regexp $p literal}",
            "proc f {} {set {$p} {(a+)+}; regexp ${$p} literal}",
            "proc f {} {set {p(open} {(a+)+}; regexp ${p(open} literal}",
        ] {
            let unit = cu(source, &input);
            assert!(unit.function("::f").is_some(), "genuine source procedure");
            let mut analyser = analyse(source, &input);
            analyser
                .result
                .diagnostics
                .retain(|finding| finding.code != DiagCode::W303);
            let start = source.rfind("regexp ").unwrap();
            queue_written(&mut analyser, source, &source[start..source.len() - 1]);
            analyser.emit_proven_word_diagnostics(&unit);
            let found = findings(&analyser.result);
            assert_eq!(
                found.len(),
                1,
                "{source}: {:?}",
                analyser.result.diagnostics
            );
            let Some(DiagnosticSubject::RegistrySource(subject)) = found[0].subject() else {
                panic!("proven original pattern")
            };
            assert_eq!(subject.supplemental_literals(), &[(0, "(a+)+".to_owned())]);
            assert!(subject.list_element_path().is_empty());
        }
        for source in [
            "proc f {p} {regexp $p literal}",
            "proc f {} {set p {(a+)+}; set p safe; regexp $p literal}",
            "proc set args {return CUSTOM}; proc f {} {set p {(a+)+}; regexp $p literal}",
        ] {
            assert!(
                findings(&analyse(source, &input).result).is_empty(),
                "{source}"
            );
        }
        let source = "proc f {} {set p {(a+)+}; regexp $p literal}";
        let unit = cu(source, &input);
        for axis in 0..5 {
            let mut changed = unit.clone();
            match axis {
                0 => changed.ir_module.source_metadata_input = None,
                1 => changed.ir_module.retained_source_bindings = None,
                2 => {
                    changed.ir_module.source_metadata_input = Some(self::input(
                        tcl_registry::model::ingress::resolve_environment("tcl9.0")
                            .default_context_registry(),
                    ))
                }
                3 => {
                    changed.ir_module.lexer_config.strict_quoting =
                        !changed.ir_module.lexer_config.strict_quoting
                }
                _ => {
                    changed
                        .procedures
                        .values_mut()
                        .next()
                        .unwrap()
                        .source_metadata_input = None
                }
            }
            let mut analyser = analyse(source, &input);
            analyser
                .result
                .diagnostics
                .retain(|finding| finding.code != DiagCode::W303);
            queue_written(&mut analyser, source, "regexp $p literal");
            analyser.emit_proven_word_diagnostics(&changed);
            assert!(
                findings(&analyser.result).is_empty(),
                "CU owner axis {axis}"
            );
        }
    }

    #[test]
    fn original_pattern_shape_keeps_a_proven_mode_and_original_literal_clause() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input();
        let source = "proc f {s} {set mode -regexp; switch $mode $s {(é+)+ {BODY}}}";
        let unit = cu(source, &input);
        assert!(unit.function("::f").is_some());
        let mut analyser = analyse(source, &input);
        analyser
            .result
            .diagnostics
            .retain(|finding| finding.code != DiagCode::W303);
        let call = "switch $mode $s {(é+)+ {BODY}}";
        let original = original_at(&analyser, source, call);
        assert!(original_patterns(&original).is_empty());
        queue_written(&mut analyser, source, call);
        analyser.emit_proven_word_diagnostics(&unit);
        let found = findings(&analyser.result);
        assert_eq!(found.len(), 1);
        assert_eq!(&source[found[0].span.as_range()], "(é+)+");
        let Some(DiagnosticSubject::RegistrySource(subject)) = found[0].subject() else {
            panic!("selected clause")
        };
        assert_eq!(subject.list_element_path(), &[0]);
        assert_eq!(
            subject.supplemental_literals(),
            &[(0, "-regexp".to_owned())]
        );
    }

    #[test]
    fn original_pattern_shape_refuses_changed_or_missing_source_owners() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input();
        let source = "regexp {(a+)+} literal";
        let original = original_at(&analyse(source, &input), source, source);
        for axis in 0..5 {
            let mut analyser = analyse(source, &input);
            analyser.result.diagnostics.clear();
            match axis {
                0 => analyser.result.resolved_input = None,
                1 => analyser.source = "regexp {(b+)+} literal".to_owned(),
                2 => {
                    analyser.result.resolved_input = Some(self::input(
                        tcl_registry::model::ingress::resolve_environment("tcl9.0")
                            .default_context_registry(),
                    ))
                }
                3 => {
                    let mut changed = input.clone();
                    changed.config.strict_quoting = !changed.config.strict_quoting;
                    analyser.result.resolved_input = Some(changed);
                }
                _ => {}
            }
            analyser.emit_w303_redos(if axis == 4 { None } else { Some(&original) });
            assert!(
                findings(&analyser.result).is_empty(),
                "original owner axis {axis}"
            );
        }
    }

    #[test]
    fn original_pattern_shape_keeps_whole_source_and_per_item_parity() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let input = current_input();
        for source in [
            "# Unicode prefix é\nproc f {} {set p {(a+)+}; regexp $p literal}",
            "interp alias {} rx {} regexp -nocase; proc f {} {rx {(é+)+} literal}",
            "proc f {s} {switch -regexp $s {(é+)+ {BODY} default {OTHER}}}",
        ] {
            let whole = analyse(source, &input);
            let mut item = Analyser::new().with_resolved_input(input.clone());
            let result = item.analyse_per_item(source, "tcl");
            assert_eq!(findings(&result), findings(&whole.result), "{source}");
            assert_eq!(findings(&result).len(), 1);
        }
    }
}
