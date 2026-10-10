// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Index advice from one sealed source schema and its original operands.

use tcl_core_types::DiagCode;
use tcl_registry::{SourceIndexBounds as Kind, SourceIndexBoundsInvocation};

use super::{
    absolute_index, describe_index, describe_index_string, pair_slice_empty, resolve_index,
};
use crate::analyser::diagnostic_registry::{
    OriginalDiagnosticInvocation, RegistrySourceDiagnosticKind,
};
use crate::analyser::{
    DiagnosticSubject,
    types::{Diagnostic, Severity},
};

pub(in crate::analyser) fn original_index_diagnostics(
    original: &OriginalDiagnosticInvocation,
) -> Vec<Diagnostic> {
    // naming.diagnostic.registry-source-ownership
    // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
    let Some(selected) = original
        .with_schema(|schema| schema.authored_source_index_bounds())
        .flatten()
    else {
        return Vec::new();
    };
    let Some(dialect) = selected.dialect else {
        return Vec::new();
    };
    match selected.operation {
        Kind::ListIndex => list_indices(original, &selected, dialect),
        Kind::ListRange | Kind::ListReplace => list_pair(original, &selected, dialect),
        Kind::StringIndex | Kind::StringInsert => string_single(original, &selected, dialect),
        Kind::StringRange | Kind::StringReplace => string_pair(original, &selected, dialect),
    }
}

fn report(
    original: &OriginalDiagnosticInvocation,
    code: DiagCode,
    first: usize,
    last: usize,
    message: String,
) -> Vec<Diagnostic> {
    // A range must be original contiguous written operands. Captured values
    // keep their earlier single-word anchor and original effective ordinal.
    let Some(first_word) = original.word(first) else {
        return Vec::new();
    };
    if first_word.image() != original.head().image() {
        return Vec::new();
    }
    let kind = RegistrySourceDiagnosticKind::IndexBounds;
    let Some(subject) = original
        .subject_range(kind, first..=last)
        .or_else(|| original.subject(kind, Some(first)))
    else {
        return Vec::new();
    };
    let DiagnosticSubject::RegistrySource(ref source) = subject else {
        return Vec::new();
    };
    vec![Diagnostic::new(code, source.span(), message, Severity::Warning).with_subject(subject)]
}

fn list_indices(
    original: &OriginalDiagnosticInvocation,
    selected: &SourceIndexBoundsInvocation,
    dialect: tcl_registry::InvocationDialect,
) -> Vec<Diagnostic> {
    let start = selected.arguments.start;
    let Some(mut list) = original.literal(start).map(str::to_owned) else {
        return Vec::new();
    };
    for argument in start + 1..selected.arguments.end {
        let (Ok(values), Some(text)) = (
            dialect.word_values.split_list(&list).map(|values| {
                values
                    .into_iter()
                    .map(std::borrow::Cow::into_owned)
                    .collect::<Vec<_>>()
            }),
            original.literal(argument),
        ) else {
            return Vec::new();
        };
        let Ok(length) = i64::try_from(values.len()) else {
            return Vec::new();
        };
        let Some(index) = resolve_index(text, length, dialect.numbers) else {
            return Vec::new();
        };
        if !(0..length).contains(&index) {
            return report(
                original,
                DiagCode::W230,
                argument,
                argument,
                format!(
                    "Index '{}' {}; lindex silently returns empty string.",
                    text.trim(),
                    describe_index(index, length)
                ),
            );
        }
        if argument + 1 < selected.arguments.end {
            let Ok(index) = usize::try_from(index) else {
                return Vec::new();
            };
            list = values[index].clone();
        }
    }
    Vec::new()
}

fn list_pair(
    original: &OriginalDiagnosticInvocation,
    selected: &SourceIndexBoundsInvocation,
    dialect: tcl_registry::InvocationDialect,
) -> Vec<Diagnostic> {
    let start = selected.arguments.start;
    let (Some(list), Some(lo_text), Some(hi_text)) = (
        original.literal(start),
        original.literal(start + 1),
        original.literal(start + 2),
    ) else {
        return Vec::new();
    };
    let Ok(values) = dialect.word_values.split_list(list) else {
        return Vec::new();
    };
    let Ok(length) = i64::try_from(values.len()) else {
        return Vec::new();
    };
    let (Some(lo), Some(hi)) = (
        resolve_index(lo_text, length, dialect.numbers),
        resolve_index(hi_text, length, dialect.numbers),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(lo, hi, length) {
        return Vec::new();
    }
    let verb = if selected.operation == Kind::ListRange {
        "lrange slice is empty"
    } else if lo < 0 && hi < 0 {
        "lreplace prepends instead of replacing (both indices resolve before the list)"
    } else if lo >= length && hi >= length {
        "lreplace appends instead of replacing (both indices resolve past the list)"
    } else {
        "lreplace touches no element (first > last after clamping)"
    };
    report(
        original,
        DiagCode::W230,
        start + 1,
        start + 2,
        format!(
            "{verb}: first='{lo_text}' resolves to {lo}, last='{hi_text}' resolves to {hi} (list has {length} element{}).",
            if length == 1 { "" } else { "s" }
        ),
    )
}

fn string_length(
    original: &OriginalDiagnosticInvocation,
    argument: usize,
    dialect: tcl_registry::InvocationDialect,
) -> Option<i64> {
    let count = tcl_dialect::StringCharacterModel::count_for(
        dialect.characters,
        original.literal(argument)?,
    )?;
    i64::try_from(count).ok()
}

fn string_single(
    original: &OriginalDiagnosticInvocation,
    selected: &SourceIndexBoundsInvocation,
    dialect: tcl_registry::InvocationDialect,
) -> Vec<Diagnostic> {
    let start = selected.arguments.start;
    let Some(text) = original.literal(start + 1) else {
        return Vec::new();
    };
    let text = text.trim();
    if absolute_index(text, dialect.numbers).is_some_and(|index| index < 0) {
        let message = if selected.operation == Kind::StringInsert {
            format!(
                "string insert: index '{text}' is negative; the insertion position is clamped to the beginning."
            )
        } else {
            format!("string index: index '{text}' is negative; result is empty.")
        };
        return report(original, DiagCode::W232, start + 1, start + 1, message);
    }
    if selected.operation == Kind::StringIndex
        && let Some(length) = string_length(original, start, dialect)
        && let Some(index) = resolve_index(text, length, dialect.numbers)
        && !(0..length).contains(&index)
    {
        return report(
            original,
            DiagCode::W232,
            start + 1,
            start + 1,
            format!(
                "string index: '{text}' {}; returns empty string.",
                describe_index_string(index, length)
            ),
        );
    }
    Vec::new()
}

fn string_pair(
    original: &OriginalDiagnosticInvocation,
    selected: &SourceIndexBoundsInvocation,
    dialect: tcl_registry::InvocationDialect,
) -> Vec<Diagnostic> {
    let start = selected.arguments.start;
    let (Some(first), Some(last)) = (original.literal(start + 1), original.literal(start + 2))
    else {
        return Vec::new();
    };
    let operation = if selected.operation == Kind::StringRange {
        "range"
    } else {
        "replace"
    };
    let verb = if selected.operation == Kind::StringRange {
        "slice is empty"
    } else {
        "replace is a no-op"
    };
    if let (Some(lo), Some(hi)) = (
        absolute_index(first.trim(), dialect.numbers),
        absolute_index(last.trim(), dialect.numbers),
    ) && lo < 0
        && hi < 0
    {
        return report(
            original,
            DiagCode::W232,
            start + 1,
            start + 2,
            format!("string {operation}: both indices are negative ('{first}', '{last}'); {verb}."),
        );
    }
    let Some(length) = string_length(original, start, dialect) else {
        return Vec::new();
    };
    let (Some(lo), Some(hi)) = (
        resolve_index(first, length, dialect.numbers),
        resolve_index(last, length, dialect.numbers),
    ) else {
        return Vec::new();
    };
    if !pair_slice_empty(lo, hi, length) {
        return Vec::new();
    }
    report(
        original,
        DiagCode::W232,
        start + 1,
        start + 2,
        format!(
            "string {operation}: {verb}: first='{first}' resolves to {lo}, last='{last}' resolves to {hi} (string has {length} character{}).",
            if length == 1 { "" } else { "s" }
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
    use std::sync::Arc;

    fn input(
        context: Arc<tcl_registry::model::ContextRegistry>,
        grammar: tcl_dialect::LexerGrammar,
    ) -> ResolvedAnalysisInput {
        // The authored naming domain is Logical; catalogue availability and
        // number grammar are independently supplied. No Native entry follows.
        let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
        profile.grammar = grammar;
        let profile = profile.intern();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(grammar),
        );
        assert!(input.has_logical_source_name_context());
        input
    }
    fn current_input() -> ResolvedAnalysisInput {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        input(
            context,
            tcl_dialect::DialectProfile::find("tcl9.0")
                .expect("authored Tcl 9.0 grammar")
                .grammar,
        )
    }
    fn analyse(source: &str, input: &ResolvedAnalysisInput) -> (Analyser, AnalysisResult) {
        let mut analyser = Analyser::new().with_resolved_input(input.clone());
        let result = analyser.analyse(source, "tcl");
        assert!(result.analysis_context_unavailable.is_none());
        assert_eq!(result.resolved_input.as_ref(), Some(input));
        (analyser, result)
    }
    fn indices(result: &AnalysisResult) -> Vec<&Diagnostic> {
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.code, DiagCode::W230 | DiagCode::W232))
            .collect()
    }
    fn last_original(analyser: &Analyser, source: &str) -> OriginalDiagnosticInvocation {
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            analyser.lexer_config(),
        )
        .pop()
        .unwrap();
        analyser
            .original_diagnostic_call_at(command.argv[0], command.arg_tokens())
            .unwrap()
    }

    #[test]
    fn original_index_bounds_select_aliases_moves_and_reject_source_replacements() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        for source in [
            "::lindex {a b} 9",
            "rename lindex Held; Held {a b} 9",
            "interp alias {} at {} lindex; at {a b} 9",
            "interp alias {} at {} lindex {λ $literal}; at 9",
            "rename string Held; Held index {λé} 9",
        ] {
            let (_, result) = analyse(source, &input);
            let diagnostics = indices(&result);
            assert_eq!(diagnostics.len(), 1, "{source}");
            let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostics[0].subject() else {
                panic!("original source subject: {source}")
            };
            assert_eq!(subject.kind(), RegistrySourceDiagnosticKind::IndexBounds);
        }
        for source in [
            "proc lindex args {return CUSTOM}; lindex {a b} 9",
            "rename lindex {}; lindex {a b} 9",
            "namespace eval custom {proc lindex args {return CUSTOM}}; ::custom::lindex {a b} 9",
            "interp alias {} at {} lindex; rename lindex Held; at {a b} 9",
            "proc string args {return CUSTOM}; string index {λé} 9",
        ] {
            let (_, result) = analyse(source, &input);
            assert!(indices(&result).is_empty(), "{source}");
        }
    }

    #[test]
    fn original_index_subjects_keep_captured_ordinals_unicode_and_written_pair_geometry() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        let source = "interp alias {} at {} string range {λé} 9; at 12";
        let (analyser, result) = analyse(source, &input);
        let diagnostics = indices(&result);
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = diagnostics[0];
        assert_eq!(
            &source[diagnostic.span.start() as usize..diagnostic.span.end() as usize],
            "9"
        );
        let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostic.subject() else {
            panic!("typed original subject")
        };
        assert_eq!(subject.argument(), Some(2));
        assert_eq!(subject.written_argument(), None);
        let original = last_original(&analyser, source);
        assert!(original.word(2).unwrap().span().end() < original.head().span().start());
        assert_eq!(subject.span(), original.word(2).unwrap().span());
        let source = "string range {λé} 9 12";
        let (_, result) = analyse(source, &input);
        let diagnostics = indices(&result);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            &source[diagnostics[0].span.start() as usize..diagnostics[0].span.end() as usize],
            "9 12"
        );
    }

    #[test]
    fn original_index_advice_retains_availability_and_refuses_missing_foreign_or_stale_owners() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let current = current_input();
        let source = "string insert {λé} -1 VALUE";
        let (analyser, result) = analyse(source, &current);
        assert_eq!(indices(&result).len(), 1);
        let context = current.context_registry();
        let older = Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(context.commands())),
        );
        let older_input = input(older, current.analyser_profile().grammar);
        let (_, older_result) = analyse(source, &older_input);
        assert!(indices(&older_result).is_empty());
        let original = last_original(&analyser, source);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        assert!(OriginalDiagnosticInvocation::new(original.words().clone(), foreign).is_none());
        let mut missing = result.clone();
        missing.resolved_input = None;
        assert!(
            crate::registry_invocation::source_structure::source_registry_words_at(
                source, &missing, 0
            )
            .is_none()
        );
        let mut stale = result.clone();
        let mut changed = current.clone();
        changed.config.strict_quoting = !changed.config.strict_quoting;
        stale.resolved_input = Some(changed);
        assert!(
            crate::registry_invocation::source_structure::source_registry_words_at(
                source, &stale, 0
            )
            .is_none()
        );
        assert!(
            crate::registry_invocation::source_structure::source_registry_words_at(
                "string insert {λé} 00 VALUE",
                &result,
                0
            )
            .is_none()
        );
    }

    #[test]
    fn original_index_supplemental_values_keep_literal_subjects_and_nested_list_paths() {
        // naming.diagnostic.registry-source-ownership
        // docs/design/analysis/name-resolution-proofs/diagnostic-registry-source-ownership.md
        let input = current_input();
        let source = "interp alias {} at {} lindex; at $literal $index";
        let (analyser, _) = analyse(source, &input);
        let original = last_original(&analyser, source);
        assert!(original_index_diagnostics(&original).is_empty());
        let projected = original
            .with_supplemental_literals(&[(0, "a b".to_owned()), (1, "9".to_owned())])
            .unwrap();
        let diagnostics = original_index_diagnostics(&projected);
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(
            &source[diagnostic.span.start() as usize..diagnostic.span.end() as usize],
            "$index"
        );
        let Some(DiagnosticSubject::RegistrySource(subject)) = diagnostic.subject() else {
            panic!("typed source subject")
        };
        assert_eq!(subject.written_argument(), Some(1));
        assert_eq!(
            subject.supplemental_literals(),
            &[(0, "a b".to_owned()), (1, "9".to_owned())]
        );
        assert_eq!(subject.words(), original.words());
        for source in [
            "lindex {{a b}} 0 1",
            "lindex {{a b}} $first 9",
            "lindex {a b} {*}$indices",
        ] {
            let (_, result) = analyse(source, &input);
            assert!(indices(&result).is_empty(), "{source}");
        }
        let source = "lindex {{a b}} 0 2";
        let (_, result) = analyse(source, &input);
        let diagnostics = indices(&result);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            &source[diagnostics[0].span.start() as usize..diagnostics[0].span.end() as usize],
            "2"
        );
        assert!(diagnostics[0].message.contains("2 element"));
    }
}
