// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original index candidates under current Logical variable scope advice.

use super::LogicalVariableScopeAdvice;
use tcl_lexer::{ExecutablePart, NativeWord, Span};

/// Literal key data and substituting original index syntax are different inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalArrayIndexKind {
    /// Literal element-name data that source construction must keep inert.
    LiteralNameData(String),
    /// Original active index source whose substitutions remain syntax.
    ComputedOriginalSyntax(String),
}

/// A readonly candidate reborrowed from a genuine original occurrence.
/// The enclosing advice independently retains current source/input/scope.
#[derive(Debug)]
pub struct LogicalArrayIndexCandidate<'advice, 'analysis> {
    advice: &'advice LogicalVariableScopeAdvice<'analysis>,
    word: NativeWord,
    span: Span,
    root: String,
    kind: LogicalArrayIndexKind,
}

impl LogicalArrayIndexCandidate<'_, '_> {
    /// The original occurrence's literal-data or computed-source interpretation.
    #[must_use]
    pub const fn kind(&self) -> &LogicalArrayIndexKind {
        &self.kind
    }

    /// Literal key data or original index syntax, as distinguished by [`Self::kind`].
    #[must_use]
    pub fn index(&self) -> &str {
        match &self.kind {
            LogicalArrayIndexKind::LiteralNameData(index)
            | LogicalArrayIndexKind::ComputedOriginalSyntax(index) => index,
        }
    }

    /// Construct source without turning literal key data into substitutions.
    /// Computed indices keep their actual original syntax and source frame.
    #[must_use]
    pub fn source_reference(&self, require_braces: bool) -> Option<String> {
        let config = self.advice.config;
        let realm = self.advice.analysis.retained_command_realm()?;
        let (word, _) =
            realm.original_written_word_at_span_in_source(&self.advice.image, self.span, config)?;
        if word != self.word {
            return None;
        }
        match &self.kind {
            LogicalArrayIndexKind::LiteralNameData(index) => {
                tcl_syntax::naming::array_source_reference(
                    &self.root,
                    index,
                    config,
                    require_braces,
                )
            }
            LogicalArrayIndexKind::ComputedOriginalSyntax(index) => {
                if require_braces {
                    return None;
                }
                let reference = format!("${}({index})", self.root);
                let parsed =
                    tcl_lexer::word_parts::whole_var_ref(reference.as_bytes(), config).ok()??;
                (parsed.name == self.root.as_bytes() && parsed.index == Some(index.as_bytes()))
                    .then_some(reference)
            }
        }
    }
}

impl<'a> LogicalVariableScopeAdvice<'a> {
    /// Index vocabulary from actual original occurrences selecting this declaration.
    /// The legacy untyped index set cannot issue these candidates. Missing whole
    /// words, foreign scopes and unsupported computed operands remain unavailable.
    #[must_use]
    pub fn array_index_candidates(&self, name: &str) -> Vec<LogicalArrayIndexCandidate<'_, 'a>> {
        let Some(declaration) = self.variable_definition(name) else {
            return Vec::new();
        };
        let Some(realm) = self.analysis.retained_command_realm() else {
            return Vec::new();
        };
        let mut candidates = Vec::new();
        for span in std::iter::once(declaration.definition_span)
            .chain(declaration.references.iter().copied())
        {
            let Some((word, head)) =
                realm.original_written_word_at_span_in_source(&self.image, span, self.config)
            else {
                continue;
            };
            let Some(occurrence) =
                self.analysis
                    .logical_variable_scope_advice(&self.image, self.config, span.start())
            else {
                continue;
            };
            let Some((root, kind)) = index_input(&word, span, head, self) else {
                continue;
            };
            if !occurrence
                .variable_definition(&root)
                .is_some_and(|selected| std::ptr::eq(selected, declaration))
            {
                continue;
            }
            if matches!(kind, LogicalArrayIndexKind::ComputedOriginalSyntax(_))
                && (self.namespace != occurrence.namespace
                    || self
                        .eligible_scopes()
                        .next_back()
                        .zip(occurrence.eligible_scopes().next_back())
                        .is_none_or(|(current, original)| !std::ptr::eq(current, original)))
            {
                continue;
            }
            if candidates
                .iter()
                .any(|candidate: &LogicalArrayIndexCandidate<'_, '_>| candidate.kind == kind)
            {
                continue;
            }
            candidates.push(LogicalArrayIndexCandidate {
                advice: self,
                word,
                span,
                root: name.to_owned(),
                kind,
            });
        }
        candidates
    }
}

fn index_input(
    word: &NativeWord,
    span: Span,
    head: u32,
    advice: &LogicalVariableScopeAdvice<'_>,
) -> Option<(String, LogicalArrayIndexKind)> {
    let config = advice.config;
    if let Some(value) = tcl_syntax::word_rules::original_static_word_source_bytes(word) {
        let source = advice.image.try_text().ok()?;
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            source.get(head as usize..)?,
            head,
            config.nested(),
        )
        .into_iter()
        .next()?;
        let content = word.content_span().ok()?;
        if !crate::var_refs::original_variable_name_role_words(source, advice.analysis, &command)
            .iter()
            .any(|operand| operand.span == content && operand.word.as_bytes() == value)
        {
            return None;
        }
        let value = std::str::from_utf8(&value).ok()?;
        let (root, index) = tcl_syntax::naming::split_element_ref(value)?;
        return Some((
            root.to_owned(),
            LogicalArrayIndexKind::LiteralNameData(index.to_owned()),
        ));
    }
    let arena = word.executable_parts();
    let mut parts = arena.all_parts().filter(|part| {
        matches!(part.part, ExecutablePart::Variable { .. })
            && (part.span == span || arena.source_span(part) == Some(span))
    });
    let part = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let raw = arena.bytes(part.span)?;
    let parsed = tcl_lexer::word_parts::whole_var_ref(raw, config).ok()??;
    let (root, kind) = if let Some(index) = parsed.index {
        let index = std::str::from_utf8(index).ok()?;
        let kind =
            tcl_syntax::word_rules::standalone_static_word_fragment_value(index, false, config)
                .map_or_else(
                    || LogicalArrayIndexKind::ComputedOriginalSyntax(index.to_owned()),
                    LogicalArrayIndexKind::LiteralNameData,
                );
        (std::str::from_utf8(parsed.name).ok()?, kind)
    } else {
        let name = std::str::from_utf8(parsed.name).ok()?;
        let (root, index) = tcl_syntax::naming::split_element_ref(name)?;
        (
            root,
            LogicalArrayIndexKind::LiteralNameData(index.to_owned()),
        )
    };
    Some((root.to_owned(), kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use tcl_lexer::SourceImage;

    #[test]
    fn logical_array_indices_keep_actual_source_kinds_and_refuse_unowned_text() {
        let source =
            "set {arr($literal)} ONE; set arr(key) KEY; puts $arr($computed); puts {arr(UNOWNED)}";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let unowned = source.find("{arr(UNOWNED)}").unwrap();
        let unowned = Span::new(
            u32::try_from(unowned).unwrap(),
            u32::try_from(unowned + "{arr(UNOWNED)".len()).unwrap(),
        );
        let declaration = analysis.global_scope.variables.get_mut("arr").unwrap();
        declaration.array_indices.insert("INVENTED".to_owned());
        declaration.references.push(unowned);
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let advice = analysis
            .logical_variable_scope_advice(&image, config, u32::try_from(source.len()).unwrap())
            .unwrap();
        let candidates = advice.array_index_candidates("arr");
        assert!(candidates.iter().any(|candidate| candidate.kind()
            == &LogicalArrayIndexKind::LiteralNameData("$literal".to_owned())));
        assert!(candidates.iter().any(|candidate| candidate.kind()
            == &LogicalArrayIndexKind::ComputedOriginalSyntax("$computed".to_owned())));
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.index() != "UNOWNED" && candidate.index() != "INVENTED")
        );
        assert!(
            analysis
                .logical_variable_scope_advice(
                    &SourceImage::document(&format!("#{source}")),
                    config,
                    0
                )
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl9.1");
        assert!(
            native
                .logical_variable_scope_advice(&image, native.body_lexer_config.unwrap(), 0)
                .is_none()
        );
    }
}
