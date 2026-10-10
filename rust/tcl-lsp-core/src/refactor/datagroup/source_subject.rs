// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Selected original case layouts and independent scalar subject advice.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::registry_invocation::source_structure::source_registry_words_at;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{ExecutablePart, LexerConfig, NativeWord, SourceImage};
use tcl_registry::spec::{CaseInvocation, CaseMatchMode};
use tcl_registry::{CommandRegistry, InvocationWord, InvocationWords, ResolvedInvocation};

/// One scalar variable-reference word in an explicitly supplied source fragment.
/// This is selected-grammar syntax only: it retains no complete-document input,
/// native operand, cell, read, execution or editing permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarVariableSourceSyntax {
    name: String,
    reference: String,
}

impl ScalarVariableSourceSyntax {
    /// Scalar name from the selected whole-reference and closed-element owners.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Unchanged complete reference spelling, including any quote wrapper.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

/// Classify exactly one scalar substitution using the supplied lexical grammar.
/// Fragment word geometry is checked by the shared lexer; inert data, arrays,
/// compound text, expansion and malformed references decline. This does not
/// capture actual analysis or convert the fragment into an original operand.
#[must_use]
pub fn scalar_variable_source_syntax(
    source: &str,
    config: LexerConfig,
) -> Option<ScalarVariableSourceSyntax> {
    let plan = tcl_lexer::native_script_words_in(
        SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    let [word] = command.words.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some() || word.bytes() != source.as_bytes() {
        return None;
    }
    scalar_syntax(word)
}

/// Exactly one genuine scalar substitution under its original lexical grammar.
/// Original spelling is retained for proposed operands; the reported root is
/// never re-emitted as a bare variable word. No native cell/read or edit follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalScalarVariableSubject {
    word: NativeWord,
    name: String,
    reference: String,
}

impl OriginalScalarVariableSubject {
    /// Source root bytes projected through their checked unchanged UTF-8 view.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Complete original subject word, including any genuine quote wrapper.
    #[must_use]
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

/// Selected exact, case-sensitive switch syntax with an arbitrary genuine
/// original subject word and completed literal pattern/body data. This grants
/// source layout only, without matching, evaluation, dispatch or edit authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalExactCaseSource {
    subject: NativeWord,
    pairs: Vec<(String, String)>,
}

impl OriginalExactCaseSource {
    /// Genuine original subject syntax under its complete lexical configuration.
    #[must_use]
    pub const fn subject(&self) -> &NativeWord {
        &self.subject
    }

    /// Completed literal pattern/body data; these values carry no source identity.
    #[must_use]
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    fn into_scalar_switch(self) -> Option<OriginalExactSwitchSource> {
        Some(OriginalExactSwitchSource {
            subject: scalar_subject(&self.subject)?,
            pairs: self.pairs,
        })
    }
}

/// Selected exact, case-sensitive switch syntax with static pattern/body data.
/// This is readonly source advice: it establishes neither matching, entered
/// bodies, Native command identity nor eligibility to apply a refactoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalExactSwitchSource {
    subject: OriginalScalarVariableSubject,
    pairs: Vec<(String, String)>,
}

impl OriginalExactSwitchSource {
    /// Independently checked original scalar source substitution.
    #[must_use]
    pub const fn subject(&self) -> &OriginalScalarVariableSubject {
        &self.subject
    }

    /// Completed literal pattern/body values under the same list grammar.
    /// These values are data, without execution or source-word identity.
    #[must_use]
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }
}

fn scalar_syntax(word: &NativeWord) -> Option<ScalarVariableSourceSyntax> {
    if word.group().expand {
        return None;
    }
    let spelling = word.try_text().ok()?;
    let arena = word.executable_parts();
    let [component] = arena.list(arena.root()) else {
        return None;
    };
    let ExecutablePart::Variable { index: None, .. } = &component.part else {
        return None;
    };
    let source = arena.bytes(component.span)?;
    let reference = tcl_lexer::word_parts::whole_var_ref(source, word.config()).ok()??;
    if reference.index.is_some()
        || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
    {
        return None;
    }
    let root = tcl_syntax::naming::variable_reference_root_bytes(source, word.config()).ok()??;
    Some(ScalarVariableSourceSyntax {
        name: std::str::from_utf8(root).ok()?.to_owned(),
        reference: spelling.to_owned(),
    })
}

fn scalar_subject(word: &NativeWord) -> Option<OriginalScalarVariableSubject> {
    let syntax = scalar_syntax(word)?;
    Some(OriginalScalarVariableSubject {
        word: word.clone(),
        name: syntax.name,
        reference: syntax.reference,
    })
}

fn exact_layout(selected: &ResolvedInvocation<'_, '_>) -> Option<CaseInvocation> {
    if selected.semantics.lowering_hook != Some(tcl_registry::hooks::LoweringHookId::Switch) {
        return None;
    }
    let (_, layout) = selected.authored_source_case_invocation()?;
    (layout.mode == CaseMatchMode::Exact && !layout.nocase).then_some(layout)
}

fn literal_pairs(
    layout: CaseInvocation,
    values: &[Option<&[u8]>],
    config: LexerConfig,
) -> Option<Vec<(String, String)>> {
    let strings = if let Some(list) = layout.clause_list_index {
        let value = std::str::from_utf8(values.get(list)?.as_ref()?).ok()?;
        tcl_syntax::word_rules::WordValueRules::from_config(&config)
            .split_list(value)
            .ok()?
            .into_iter()
            .map(|value| value.into_owned())
            .collect::<Vec<_>>()
    } else {
        values
            .get(layout.inline_clause_start?..)?
            .iter()
            .map(|value| Some(std::str::from_utf8(value.as_ref()?).ok()?.to_owned()))
            .collect::<Option<Vec<_>>>()?
    };
    if strings.is_empty() || !strings.len().is_multiple_of(2) {
        return None;
    }
    Some(
        strings
            .chunks_exact(2)
            .map(|pair| (pair[0].clone(), pair[1].clone()))
            .collect(),
    )
}

/// Read source advice from the actual full analysis and selected original argv.
/// Missing/stale source, known replacement, uncertain options and nonliteral
/// case data decline without nominal recapture. Subject syntax stays original.
#[must_use]
pub fn original_exact_case_source_at_analysis(
    source: &str,
    analysis: &AnalysisResult,
    head_offset: u32,
) -> Option<OriginalExactCaseSource> {
    // naming.refactor.original-exact-case-source
    // docs/design/analysis/name-resolution-proofs/original-exact-case-source.md
    let input = analysis.resolved_input.as_ref()?;
    let words = source_registry_words_at(source, analysis, head_offset)?;
    let layout = words.with_source_schema(input.borrowed_context_registry(), exact_layout)??;
    let subject = words
        .operands()
        .get(layout.subject_index?)?
        .as_ref()?
        .word()?;
    let values = words
        .arguments()
        .iter()
        .map(tcl_compiler::registry_invocation::EffectiveInvocationWord::literal_bytes)
        .collect::<Vec<_>>();
    Some(OriginalExactCaseSource {
        subject: subject.clone(),
        pairs: literal_pairs(layout, &values, input.lexer_config())?,
    })
}

/// Independently project a single scalar substitution from the shared exact
/// source layout. Arrays, compound words and inert data remain unavailable.
#[must_use]
pub fn original_exact_switch_source_at_analysis(
    source: &str,
    analysis: &AnalysisResult,
    head_offset: u32,
) -> Option<OriginalExactSwitchSource> {
    // naming.refactor.original-datagroup-variable-subject
    // docs/design/analysis/name-resolution-proofs/original-datagroup-variable-subject.md
    original_exact_case_source_at_analysis(source, analysis, head_offset)?.into_scalar_switch()
}

/// Explicit standalone authoring projection from the supplied registry and
/// grammar. This is separate from the actual-analysis ingress above.
fn standalone_exact_case_source(
    source: &str,
    command: &SegmentedCommand,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<OriginalExactCaseSource> {
    let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        command,
    );
    let words = tcl_compiler::registry_invocation::original_native_compiler_words(
        &SourceImage::document(source),
        tokens.words(),
        command.argv.first()?.span.start(),
        config,
    )?;
    let values = words
        .iter()
        .map(tcl_syntax::word_rules::original_static_word_source_bytes)
        .collect::<Vec<_>>();
    let facts = words
        .iter()
        .zip(&values)
        .map(|(word, value)| {
            if word.group().expand {
                InvocationWord::Expanded
            } else {
                value
                    .as_deref()
                    .map_or(InvocationWord::Dynamic, InvocationWord::KnownBytes)
            }
        })
        .collect::<Vec<_>>();
    let selected = registry.resolve_structured_invocation(
        InvocationWords::structured(*facts.first()?, &facts[1..]),
        registry.own_surface_query(),
    );
    let layout = exact_layout(&selected.resolved()?)?;
    let values = values[1..].iter().map(Option::as_deref).collect::<Vec<_>>();
    Some(OriginalExactCaseSource {
        subject: words.get(layout.subject_index?.checked_add(1)?)?.clone(),
        pairs: literal_pairs(layout, &values, config)?,
    })
}

/// Scalar extraction advice from explicit supplied Registry and source grammar.
pub(super) fn standalone_exact_switch_source(
    source: &str,
    command: &SegmentedCommand,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<OriginalExactSwitchSource> {
    standalone_exact_case_source(source, command, registry, config)?.into_scalar_switch()
}

#[cfg(test)]
mod tests {

    #[test]
    fn scalar_fragment_syntax_preserves_selected_reference_boundaries() {
        // Implementation contract: naming.refactor.selected-if-scalar-source-syntax
        // docs/design/analysis/name-resolution-proofs/selected-if-scalar-source-syntax.md
        for (dialect, spelling, expected) in [
            ("tcl8.4", "${a{b}", Some("a{b")),
            ("tcl8.6", "${a{b}c}", None),
            ("tcl9.0", "${a{b}", None),
            ("tcl9.1", "${a{b}c}", Some("a{b}c")),
            ("jim", "$café", Some("café")),
            ("tcl8.6", "$café", None),
            ("f5-irules", "$café", None),
            ("tcl8.6", "\"${café}\"", Some("café")),
            ("tcl8.6", "${a b}", Some("a b")),
            ("tcl8.6", "${literal$name}", Some("literal$name")),
            ("tcl8.6", r"${a\b}", Some(r"a\b")),
            ("tcl8.6", "${a(}", Some("a(")),
            ("tcl8.6", "${a(k)tail}", Some("a(k)tail")),
            ("tcl8.6", "$a(k)", None),
            ("tcl8.6", "${a(k)}", None),
            ("tcl8.6", "{$x}", None),
            ("tcl8.6", "\"$x[set y]\"", None),
            ("tcl8.6", "$x.tail", None),
            ("tcl8.6", "${missing", None),
            ("tcl8.6", "$x # comment", None),
        ] {
            let syntax = scalar_variable_source_syntax(spelling, LexerConfig::for_dialect(dialect));
            assert_eq!(
                syntax.as_ref().map(ScalarVariableSourceSyntax::name),
                expected,
                "{dialect}: {spelling:?}"
            );
            if let Some(syntax) = syntax {
                assert_eq!(syntax.reference(), spelling);
            }
        }
    }

    use super::*;

    #[test]
    fn original_scalar_subject_uses_shared_selected_reference_boundaries() {
        // Implementation contract: naming.refactor.original-datagroup-variable-subject
        // docs/design/analysis/name-resolution-proofs/original-datagroup-variable-subject.md
        for (dialect, source, expected) in [
            ("tcl8.6", "${a{b}", "a{b"),
            ("tcl9.0", "${a{b}c}", "a{b}c"),
            ("jim", "$café", "café"),
            ("tcl8.6", "\"${a b}\"", "a b"),
            ("tcl8.6", "${literal$name}", "literal$name"),
        ] {
            let config = LexerConfig::for_dialect(dialect);
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(plan.fatal_tail.is_none());
            assert_eq!(plan.commands.len(), 1);
            let original = scalar_subject(&plan.commands[0].words[0]).expect("one original scalar");
            assert_eq!(original.name(), expected);
            assert_eq!(original.reference(), source);
            assert_eq!(original.word.config(), config);
        }
    }
}
