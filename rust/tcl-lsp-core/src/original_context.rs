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

//! Current complete source and metadata, independently of a consumer's purpose.

use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LexerConfig, SourceChannel};
use tcl_registry::{CommandRegistry, model::ContextRegistry};

/// Currency of an editor query over the actual Document source channel.
/// Advice purpose is separate; this grants no execution, identity or editing.
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
        let realm = analysis.retained_command_realm()?;
        let image = realm.original_source_image()?;
        if image.channel() != SourceChannel::Document
            || image.bytes() != source.as_bytes()
            || input.lexer_config() != config
            || !analysis.matches_original_source_image(image, config)
            || !realm.matches_resolved_analysis_input(input)
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

/// Exact readonly selection position in one retained current Document source.
/// A clamped line/column or surrogate interior cannot select another occurrence.
/// This supplies no declaration, lookup, frame, source absence or edit authority.
pub(crate) struct CurrentSourcePosition<'a> {
    current: CurrentSourceContext<'a>,
    source: &'a str,
    analysis: &'a AnalysisResult,
    offset: u32,
}

impl<'a> CurrentSourcePosition<'a> {
    pub(crate) fn capture(
        source: &'a str,
        analysis: &'a AnalysisResult,
        line: u32,
        character: u32,
    ) -> Option<Self> {
        let current = CurrentSourceContext::capture(source, analysis)?;
        let index = tcl_lexer::LineIndex::new_lsp(source);
        let offset = index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(character), source);
        let position = index.position_at_utf16(offset, source);
        if position.line != line || position.character.get() != character {
            return None;
        }
        Some(Self {
            current,
            source,
            analysis,
            offset,
        })
    }

    /// A caret exactly after a complete original variable word may select its
    /// last name character. Current right-hand roots, literal tails, malformed
    /// words and invalid UTF-16 positions cannot borrow this affinity.
    pub(crate) fn capture_variable(
        source: &'a str,
        analysis: &'a AnalysisResult,
        line: u32,
        character: u32,
    ) -> Option<Self> {
        let mut position = Self::capture(source, analysis, line, character)?;
        if let Some(offset) = position.variable_word_end_offset() {
            position.offset = offset;
        }
        Some(position)
    }

    fn variable_word_end_offset(&self) -> Option<u32> {
        let image = tcl_lexer::SourceImage::document(self.source);
        let config = self.current.config();
        if self
            .analysis
            .original_variable_root_in_source(&image, config, self.offset)
            .is_some()
            || self
                .analysis
                .original_variable_symbol_conflicts
                .iter()
                .any(|span| span.start() <= self.offset && self.offset < span.end())
        {
            return None;
        }
        let prefix = self.source.get(..usize::try_from(self.offset).ok()?)?;
        let previous = prefix.chars().next_back()?;
        let previous_offset = self
            .offset
            .checked_sub(u32::try_from(previous.len_utf8()).ok()?)?;
        let root =
            self.analysis
                .original_variable_root_in_source(&image, config, previous_offset)?;
        crate::source_structure::original_variable_word_end(
            self.source,
            self.analysis,
            root,
            self.offset,
        )?;
        let name = root.name_span()?;
        let last = self
            .source
            .get(name.as_range())?
            .char_indices()
            .next_back()?
            .0;
        let offset = name.start().checked_add(u32::try_from(last).ok()?)?;
        (self
            .analysis
            .original_variable_root_in_source(&image, config, offset)
            == Some(root))
        .then_some(offset)
    }

    pub(crate) fn line_character(&self) -> (u32, u32) {
        let position =
            tcl_lexer::LineIndex::new_lsp(self.source).position_at_utf16(self.offset, self.source);
        (position.line, position.character.get())
    }

    pub(crate) const fn offset(&self) -> u32 {
        self.offset
    }
    pub(crate) const fn source(&self) -> &'a str {
        self.source
    }
    pub(crate) const fn analysis(&self) -> &'a AnalysisResult {
        self.analysis
    }
    pub(crate) const fn context(&self) -> &CurrentSourceContext<'a> {
        &self.current
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
        assert_eq!(
            crate::completion::completions(source, 0, 14, analysis, None, None, profile),
            [] as [crate::completion::CompletionItem; 0]
        );
        assert_eq!(
            crate::definition::definition(source, 0, 13, analysis),
            [] as [crate::definition::LspRange; 0]
        );
        assert!(crate::hover::hover(source, 0, 13, analysis, None).is_none());
        assert_eq!(
            crate::type_definition::type_definition(source, 0, 13, analysis),
            [] as [crate::definition::LspRange; 0]
        );
        assert_eq!(
            crate::call_hierarchy::prepare(source, 0, 13, analysis),
            [] as [crate::call_hierarchy::CallHierarchyItem; 0]
        );
    }

    #[test]
    fn supplied_editor_entries_do_not_use_logical_purpose_as_source_currency() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let source = "proc p {} {}; p";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        assert!(CurrentSourceContext::capture(source, &analysis).is_some());
        assert_ne!(
            crate::definition::definition(source, 0, 13, &analysis),
            [] as [crate::definition::LspRange; 0]
        );
        assert!(crate::hover::hover(source, 0, 13, &analysis, None).is_some());
        assert_ne!(
            crate::call_hierarchy::prepare(source, 0, 13, &analysis),
            [] as [crate::call_hierarchy::CallHierarchyItem; 0]
        );
        assert_ne!(
            crate::completion::completions(
                source,
                0,
                14,
                &analysis,
                None,
                None,
                tcl_dialect::DialectProfile::plain_tcl()
            ),
            [] as [crate::completion::CompletionItem; 0]
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
        assert_eq!(
            crate::type_definition::type_definition(
                &source.replace("my m", "my x"),
                0,
                cursor,
                &analysis
            ),
            [] as [crate::definition::LspRange; 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            crate::type_definition::type_definition(source, 0, cursor, &analysis),
            [] as [crate::definition::LspRange; 0]
        );
    }
}

#[cfg(test)]
mod variable_end_affinity_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut preparer = Analyser::new();
        let input = preparer.prepare_analysis_input("tcl8.6").unwrap();
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "tcl8.6")
    }

    fn at(source: &str, offset: usize) -> (u32, u32) {
        let position = tcl_lexer::LineIndex::new_lsp(source)
            .position_at_utf16(u32::try_from(offset).unwrap(), source);
        (position.line, position.character.get())
    }

    fn assert_end_answer(source: &str, read: &str) {
        let analysis = analyse(source);
        assert!(!analysis.allows_lexical_declaration_advice());
        let end = source.rfind(read).unwrap() + read.len();
        let (line, character) = at(source, end);
        let original = CurrentSourcePosition::capture(source, &analysis, line, character).unwrap();
        let projected =
            CurrentSourcePosition::capture_variable(source, &analysis, line, character).unwrap();
        assert_eq!(original.offset(), u32::try_from(end).unwrap());
        assert!(
            projected.offset() < original.offset(),
            "actual complete original variable word: {source}"
        );
        assert!(
            analysis
                .original_variable_root_in_source(
                    &tcl_lexer::SourceImage::document(source),
                    analysis.body_lexer_config.unwrap(),
                    projected.offset()
                )
                .is_some()
        );
        let (inside_line, inside_character) = projected.line_character();
        let inside =
            crate::definition::definition(source, inside_line, inside_character, &analysis);
        assert!(
            !inside.is_empty(),
            "genuine current variable declaration: {source}"
        );
        assert_eq!(
            crate::definition::definition(source, line, character, &analysis),
            inside
        );
        let inside = crate::hover::hover(source, inside_line, inside_character, &analysis, None)
            .expect("genuine current variable hover");
        assert!(inside.value.contains("Variable"), "{}", inside.value);
        assert_eq!(
            crate::hover::hover(source, line, character, &analysis, None)
                .unwrap()
                .value,
            inside.value
        );
        assert!(matches!(
            crate::variable_symbol::select(source, &analysis, line, character),
            std::ops::ControlFlow::Break(Some(_))
        ));
    }

    #[test]
    fn original_variable_word_end_keeps_native_scalar_qualified_array_and_unicode_owners() {
        let source = "set x 42\nputs $x\n";
        let analysis = analyse(source);
        let image = tcl_lexer::SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let root = analysis
            .original_variable_root_in_source(&image, config, 15)
            .unwrap();
        assert!(
            tcl_lexer::word_parts::word_cursor_prefix_in(
                &image,
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                15,
                config
            )
            .unwrap()
            .is_none(),
            "variable syntax is not a command-name completion prefix"
        );
        assert!(
            crate::source_structure::original_variable_word_end(source, &analysis, root, 16)
                .is_some(),
            "the actual complete original command owns the variable word independently"
        );
        for (source, read) in [
            ("set x 42\nputs $x", "$x"),
            ("set x 42\nputs $x\n", "$x"),
            ("set x 42\r\nputs $x\r\n", "$x"),
            ("set ::x 42\nputs $::x\n", "$::x"),
            ("set data(key) 42\nputs $data(key)\n", "$data(key)"),
            ("set {café🧪} 42\nputs ${café🧪}\n", "${café🧪}"),
        ] {
            assert_end_answer(source, read);
        }
    }

    #[test]
    fn original_variable_word_end_preserves_adjacent_roots_and_rejects_literal_tails() {
        let source = "set x 42\nset y 7\nputs $x$y\n";
        let analysis = analyse(source);
        let boundary = source.rfind("$x").unwrap() + 2;
        let (line, character) = at(source, boundary);
        let selected =
            CurrentSourcePosition::capture_variable(source, &analysis, line, character).unwrap();
        assert_eq!(selected.offset(), u32::try_from(boundary).unwrap());
        let root = analysis
            .original_variable_root_in_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
                selected.offset(),
            )
            .expect("right-hand original variable root");
        assert_eq!(root.bytes(), b"y");
        let definitions = crate::definition::definition(source, line, character, &analysis);
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].start_line, 1);
        let source = "set data(key) 42\nset key key\nputs $data($key)\n";
        let analysis = analyse(source);
        let boundary = source.rfind("$key").unwrap() + "$key".len();
        let (line, character) = at(source, boundary);
        let selected =
            CurrentSourcePosition::capture_variable(source, &analysis, line, character).unwrap();
        assert_eq!(selected.offset(), u32::try_from(boundary).unwrap());
        let root = analysis
            .original_variable_root_in_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
                selected.offset(),
            )
            .expect("outer original array root at the closing delimiter");
        assert_eq!(root.bytes(), b"data");
        for source in ["set x 42\nputs $x-tail\n", "set x 42\nputs $x\"tail\"\n"] {
            let analysis = analyse(source);
            let end = source.rfind("$x").unwrap() + 2;
            let (line, character) = at(source, end);
            let position =
                CurrentSourcePosition::capture_variable(source, &analysis, line, character)
                    .unwrap();
            assert_eq!(
                position.offset(),
                u32::try_from(end).unwrap(),
                "literal tail remains in the original word"
            );
            assert!(crate::definition::definition(source, line, character, &analysis).is_empty());
        }
        let source = "set x 42\nputs $x\n";
        let analysis = analyse(source);
        assert!(crate::definition::definition(source, 2, 0, &analysis).is_empty());
        assert!(crate::hover::hover(source, 2, 0, &analysis, None).is_none());
    }

    #[test]
    fn original_variable_word_end_refuses_surrogate_interiors_and_invalid_positions() {
        let source = "set {café🧪} 42\nputs ${café🧪}\n";
        assert_end_answer(source, "${café🧪}");
        let analysis = analyse(source);
        let (line, character) = at(source, source.rfind('🧪').unwrap());
        for (line, character) in [(line, character + 1), (line, u32::MAX), (u32::MAX, 0)] {
            assert!(
                CurrentSourcePosition::capture_variable(source, &analysis, line, character)
                    .is_none()
            );
            assert!(crate::definition::definition(source, line, character, &analysis).is_empty());
            assert!(crate::hover::hover(source, line, character, &analysis, None).is_none());
        }
        let changed = source.replace("42", "43");
        let end = source.rfind("${café🧪}").unwrap() + "${café🧪}".len();
        let (line, character) = at(source, end);
        assert!(
            CurrentSourcePosition::capture_variable(&changed, &analysis, line, character).is_none()
        );
        assert!(crate::definition::definition(&changed, line, character, &analysis).is_empty());
        assert!(crate::hover::hover(&changed, line, character, &analysis, None).is_none());
    }

    #[test]
    fn original_variable_word_end_keeps_unknown_names_and_incomplete_words_unavailable() {
        let source = "puts $puts\n";
        let analysis = analyse(source);
        let end = source.find("$puts").unwrap() + "$puts".len();
        let (line, character) = at(source, end);
        let projected =
            CurrentSourcePosition::capture_variable(source, &analysis, line, character).unwrap();
        assert!(
            projected.offset() < u32::try_from(end).unwrap(),
            "real root geometry is not a declaration"
        );
        assert!(crate::definition::definition(source, line, character, &analysis).is_empty());
        assert!(crate::hover::hover(source, line, character, &analysis, None).is_none());
        for (source, read) in [
            ("set x 42\nputs \"$x", "$x"),
            ("set x 42\nputs {*}$x\n", "$x"),
            ("set x 42\nputs {$x}\n", "{$x}"),
        ] {
            let analysis = analyse(source);
            let end = source.rfind(read).unwrap() + read.len();
            let (line, character) = at(source, end);
            assert!(
                CurrentSourcePosition::capture_variable(source, &analysis, line, character)
                    .is_none_or(|position| position.offset() == u32::try_from(end).unwrap()),
                "no original complete variable word: {source}"
            );
            assert!(crate::definition::definition(source, line, character, &analysis).is_empty());
        }
    }
}
