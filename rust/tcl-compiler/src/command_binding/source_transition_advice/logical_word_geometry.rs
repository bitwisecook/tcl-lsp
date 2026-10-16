// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original body word geometry survives unavailable command selection.

use super::{
    AdviceInvocationContext, AdviceNamingPolicy, OriginalSourceCatalogueBody,
    OriginalSourceTransitionAdviceTape,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::SourceOriginId;
use std::sync::Arc;
use tcl_lexer::{NativeWord, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OriginalLogicalBodyWords {
    origin: Arc<SourceOriginId>,
    input: ResolvedAnalysisInput,
    body: OriginalSourceCatalogueBody,
    original: Arc<[NativeWord]>,
}
impl OriginalLogicalBodyWords {
    fn capture(
        context: &AdviceInvocationContext<'_>,
        original: &[NativeWord],
        body: OriginalSourceCatalogueBody,
    ) -> Option<Self> {
        let AdviceNamingPolicy::Logical(input) = context.policy else {
            return None;
        };
        let head = original.first()?;
        if input.lexer_config() != context.config
            || !std::ptr::eq(input.borrowed_context_registry(), context.context)
            || head.image() != context.origin.source_image()
            || !body.matches_source(head.image(), context.config)
            || !body.matches_context(context.context)
            || head.span().start() < body.content_span().start()
            || original.last()?.span().end() > body.content_span().end()
            || original
                .iter()
                .any(|word| word.image() != head.image() || word.config() != context.config)
        {
            return None;
        }
        Some(Self {
            origin: Arc::clone(context.origin),
            input: input.clone(),
            body,
            original: Arc::from(original),
        })
    }
    pub(super) fn original_words(
        &self,
        origin: &Arc<SourceOriginId>,
        input: &ResolvedAnalysisInput,
    ) -> Option<&[NativeWord]> {
        if !Arc::ptr_eq(&self.origin, origin)
            || &self.input != input
            || !input.has_logical_source_name_context()
            || !self
                .body
                .matches_source(origin.source_image(), input.lexer_config())
            || !self.body.matches_context(input.borrowed_context_registry())
            || self.original.iter().any(|word| {
                word.image() != origin.source_image() || word.config() != input.lexer_config()
            })
        {
            return None;
        }
        Some(&self.original)
    }

    fn original_word(
        &self,
        origin: &Arc<SourceOriginId>,
        input: &ResolvedAnalysisInput,
        span: Span,
    ) -> Option<(NativeWord, u32)> {
        let original = self.original_words(origin, input)?;
        let mut words = original.iter().filter(|word| {
            word.span() == span
                || word
                    .tokens()
                    .first()
                    .is_some_and(|token| token.span == span)
        });
        let word = words.next()?;
        words
            .next()
            .is_none()
            .then_some((word.clone(), self.original.first()?.span().start()))
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(super) fn retain_logical_body_words(
        &mut self,
        context: &AdviceInvocationContext<'_>,
        original: &[NativeWord],
        body: OriginalSourceCatalogueBody,
    ) {
        let Some(receipt) = OriginalLogicalBodyWords::capture(context, original, body) else {
            return;
        };
        let Some(head) = original.first() else {
            return;
        };
        super::source_class::merge_receipt(
            &mut self.logical_body_words,
            head.span().start(),
            Some(receipt),
        );
    }
    pub(super) fn logical_body_original_word_at_span(
        &self,
        origin: &Arc<SourceOriginId>,
        input: &ResolvedAnalysisInput,
        span: Span,
    ) -> Option<(NativeWord, u32)> {
        let mut selected = None;
        for (&offset, receipt) in &self.logical_body_words {
            let receipt = receipt.as_ref()?;
            if receipt.original.first()?.span().start() != offset {
                return None;
            }
            let Some(current) = receipt.original_word(origin, input, span) else {
                continue;
            };
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return None;
            }
            selected = Some(current);
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    fn analyse(source: &str) -> crate::analyser::AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }
    #[test]
    fn logical_body_word_geometry_keeps_dynamic_heads_without_selected_schema() {
        for source in [
            "proc f {} {set ns mypkg; ${ns}::dowork arg}",
            "proc f {} {if 1 {${ns}::dowork arg}}",
            "oo::class create C {method run {} {${ns}::dowork arg}}",
        ] {
            let analysis = analyse(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let realm = analysis.retained_command_realm().unwrap();
            let image = tcl_lexer::SourceImage::document(source);
            let start = u32::try_from(source.find("${ns}::dowork").unwrap()).unwrap();
            let span =
                tcl_lexer::Span::new(start, start + u32::try_from("${ns}::dowork".len()).unwrap());
            let (word, head) = realm
                .original_written_word_at_span_in_source(&image, span, input.lexer_config())
                .expect("original body syntax does not require selected dynamic head");
            assert_eq!(head, start);
            assert_eq!(word.try_text().unwrap(), "${ns}::dowork");
            assert!(!word.group().expand);
            assert!(
                crate::registry_invocation::source_structure::source_registry_words_at(
                    source, &analysis, start
                )
                .is_none()
            );
        }
    }
    #[test]
    fn logical_body_word_geometry_refuses_changed_source_and_inert_script_data() {
        let source = "proc f {} {${ns}::dowork arg}; set inert {${ns}::dowork arg}";
        let analysis = analyse(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let start = u32::try_from(source.find("${ns}::dowork").unwrap()).unwrap();
        let span =
            tcl_lexer::Span::new(start, start + u32::try_from("${ns}::dowork".len()).unwrap());
        let image = tcl_lexer::SourceImage::document(source);
        assert!(
            realm
                .original_written_word_at_span_in_source(&image, span, input.lexer_config())
                .is_some()
        );
        let changed = tcl_lexer::SourceImage::document(&source.replace("dowork", "changed"));
        assert!(
            realm
                .original_written_word_at_span_in_source(&changed, span, input.lexer_config())
                .is_none()
        );
        let mut config = input.lexer_config();
        config.strict_quoting ^= true;
        assert!(
            realm
                .original_written_word_at_span_in_source(&image, span, config)
                .is_none()
        );
        let inert = u32::try_from(source.rfind("${ns}::dowork").unwrap()).unwrap();
        let inert_span =
            tcl_lexer::Span::new(inert, inert + u32::try_from("${ns}::dowork".len()).unwrap());
        assert!(
            realm
                .original_written_word_at_span_in_source(&image, inert_span, input.lexer_config())
                .is_none()
        );
    }
}
