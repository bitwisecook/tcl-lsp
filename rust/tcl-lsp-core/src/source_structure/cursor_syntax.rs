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

//! Source syntax at a cursor: variable references, word prefixes, complete words and method sites.

use super::SourceStructure;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{ExecutablePart, LexerConfig, NativeWord, SourceImage, Span, WordKind};
use tcl_registry::ArgRole;

/// Source labels from one exact original variable component. These labels
/// describe selected syntax only; they grant no cell, runtime read or edit.
pub(crate) struct SourceVariableReference<'a> {
    pub(crate) root: &'a str,
    pub(crate) name_span: Span,
    pub(crate) whole_span: Span,
    pub(crate) token_span: Span,
}

fn reference_from_arena<'a>(
    source: &'a str,
    arena: &tcl_lexer::ExecutablePartArena,
    cursor: u32,
) -> Option<SourceVariableReference<'a>> {
    let part = arena.variable_part_at(cursor)?;
    let original = source.get(part.span.as_range())?;
    let config = arena.config();
    let root =
        tcl_syntax::naming::variable_reference_root_bytes(original.as_bytes(), config).ok()??;
    let tcl_lexer::ExecutablePart::Variable { name, .. } = part.part else {
        return None;
    };
    arena.bytes(name)?.starts_with(root).then_some(())?;
    let name_span = Span::new(
        name.start(),
        name.start().checked_add(u32::try_from(root.len()).ok()?)?,
    );
    Some(SourceVariableReference {
        root: std::str::from_utf8(root).ok()?,
        name_span,
        whole_span: part.span,
        token_span: arena.source_span(part)?,
    })
}

fn variable_in_structure<'a>(
    source: &'a str,
    structure: &SourceStructure,
    config: LexerConfig,
    cursor: u32,
    expression_source: bool,
) -> Option<SourceVariableReference<'a>> {
    let command = structure
        .commands
        .iter()
        .filter(|command| {
            let span = command.execution_span(source);
            span.start() <= cursor && cursor < span.end()
        })
        .min_by_key(|command| command.execution_span(source).len())?;
    if command.is_partial {
        return None;
    }
    let image = SourceImage::document(source);
    let plan =
        tcl_lexer::native_script_words_in(image.clone(), command.execution_span(source), config)
            .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let word = plan
        .commands
        .first()?
        .words
        .iter()
        .find(|word| word.span().start() <= cursor && cursor < word.span().end())?;
    if word.group().expand {
        return None;
    }
    if word.group().kind != WordKind::Braced {
        return reference_from_arena(source, word.executable_parts(), cursor);
    }
    // The actual source role/data classifier must already have proved this
    // position substituting. Braced data never reaches this expression path;
    // original script bodies have their own smaller selected command above.
    if !expression_source {
        return None;
    }
    let arena = tcl_lexer::ExecutablePartArena::decompose(
        image,
        word.content_span().ok()?,
        tcl_lexer::SubstFlags::default(),
        config,
    )
    .ok()?;
    reference_from_arena(source, &arena, cursor)
}

/// Partial cursor syntax from the current source's innermost genuinely owned
/// lexical script region. Missing source/phase correspondence stays unavailable;
/// no completed name, variable frame or selected binding follows.
pub(crate) fn original_variable_cursor_prefix(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Result<
    Option<tcl_lexer::word_parts::VariableCursorPrefix>,
    tcl_lexer::word_parts::VariableCursorUnavailable,
> {
    use tcl_lexer::word_parts::{VariableCursorUnavailable, variable_cursor_prefix_in};
    let config = analysis
        .body_lexer_config
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) || cursor as usize > source.len() {
        return Err(VariableCursorUnavailable::SourceGeometry);
    }
    let structure = SourceStructure::capture(source, Some(analysis), config)
        .ok_or(VariableCursorUnavailable::LexicalStream)?;
    let region = structure
        .scripts
        .iter()
        .filter(|(span, _)| span.start() <= cursor && cursor <= span.end())
        .min_by_key(|(span, _)| span.len())
        .map(|(span, _)| *span)
        .ok_or(VariableCursorUnavailable::SourceGeometry)?;
    let prefix = variable_cursor_prefix_in(&image, region, cursor, config)?;
    if prefix.is_some() {
        return Ok(prefix);
    }
    // Complete selected expression references retain an independent original
    // component even when their enclosing word is braced source syntax.
    let Some(reference) = cursor
        .checked_sub(1)
        .and_then(|at| original_variable_reference_at(source, analysis, at))
    else {
        return Ok(None);
    };
    variable_cursor_prefix_in(&image, reference.whole_span, cursor, config)
}

/// Original word-cursor geometry in the current innermost selected script.
/// Braced data and unsupported expression/child phases cannot create a script
/// owner; the returned argv ordinal is lexical source data only.
pub(crate) fn original_word_cursor_prefix(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Result<
    Option<tcl_lexer::word_parts::WordCursorPrefix>,
    tcl_lexer::word_parts::WordCursorUnavailable,
> {
    use tcl_lexer::word_parts::{WordCursorUnavailable, word_cursor_prefix_in};
    let config = analysis
        .body_lexer_config
        .ok_or(WordCursorUnavailable::SourceGeometry)?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) || cursor as usize > source.len() {
        return Err(WordCursorUnavailable::SourceGeometry);
    }
    let structure = SourceStructure::capture(source, Some(analysis), config)
        .ok_or(WordCursorUnavailable::LexicalStream)?;
    let region = structure
        .scripts
        .iter()
        .filter(|(span, _)| span.start() <= cursor && cursor <= span.end())
        .min_by_key(|(span, _)| span.len())
        .map(|(span, _)| *span)
        .ok_or(WordCursorUnavailable::SourceGeometry)?;
    word_cursor_prefix_in(&image, region, cursor, config)
}

/// A complete word selected by the existing configured cursor and whole-vector
/// producers in the actual innermost source script. This is syntax only.
pub(crate) struct OriginalSourceCursorWord {
    pub(super) cursor: tcl_lexer::word_parts::WordCursorPrefix,
    pub(super) word: NativeWord,
}

impl OriginalSourceCursorWord {
    /// Static Unicode value of the whole original word, never a computed tail
    /// or source-coordinate inverse of that value.
    pub(crate) fn static_unicode_value(&self) -> Option<String> {
        if self.word.span() != self.cursor.container_span()
            || !self
                .cursor
                .matches_source(self.word.image(), self.word.config())
            || self.word.group().expand
        {
            return None;
        }
        String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
            &self.word,
        )?)
        .ok()
    }
}

/// Rejoin a partial lexical cursor to its actual complete original command
/// vector. Partial prefixes, malformed commands, computed/expanded words and
/// source data do not acquire a name identity or binding receipt here.
pub(crate) fn original_complete_cursor_word(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<OriginalSourceCursorWord> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let prefix = original_word_cursor_prefix(source, analysis, cursor).ok()??;
    if prefix.is_empty_boundary() {
        return None;
    }
    let (_, extent) = prefix.command_source()?;
    // Reborrow only the authentic selected command extent, rather than lexing
    // the whole enclosing script again after the configured cursor capture.
    let plan =
        tcl_lexer::native_script_words_in(SourceImage::document(source), extent, current.config())
            .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some() || command.source_extent != Some(extent) {
        return None;
    }
    let word = command.words.get(prefix.word_index())?;
    if word.span() != prefix.container_span()
        || !(word.span().start() <= cursor && cursor < word.span().end())
        || word.group().expand
    {
        return None;
    }
    Some(OriginalSourceCursorWord {
        cursor: prefix,
        word: word.clone(),
    })
}

/// Whole-command geometry for a terminal original variable component. A
/// command-name completion prefix is a different purpose and may be absent
/// for this genuine dynamic word. This supplies no name, frame or binding.
pub(crate) fn original_variable_word_end(
    source: &str,
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::variable_name::SignatureSourceVariableRoot,
    end: u32,
) -> Option<()> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let structure = SourceStructure::capture(source, Some(analysis), current.config())?;
    let extent = structure
        .commands
        .iter()
        .filter(|command| {
            command.span.start() <= root.part_span().start()
                && root.part_span().end() <= command.span.end()
        })
        .min_by_key(|command| command.span.len())?
        .span;
    let plan =
        tcl_lexer::native_script_words_in(SourceImage::document(source), extent, current.config())
            .ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some()
        || command.source_extent != Some(extent)
        || root.part_span().end() != end
    {
        return None;
    }
    let mut words = command.words.iter().filter(|word| {
        word.image() == root.source_image()
            && word.config() == root.lexer_config()
            && !word.group().expand
            && word.span().start() <= root.part_span().start()
            && word.span().end() == end
    });
    words.next()?;
    words.next().is_none().then_some(())
}

/// Reborrowed original variable geometry for one immutable analysis source.
/// This view owns syntax only; callers retain name resolution and edit guards.
pub(crate) struct OriginalVariableReferences<'a> {
    pub(super) source: &'a str,
    pub(super) analysis: &'a AnalysisResult,
    pub(super) config: LexerConfig,
    pub(super) structure: SourceStructure,
}

impl<'a> OriginalVariableReferences<'a> {
    pub(crate) fn capture(source: &'a str, analysis: &'a AnalysisResult) -> Option<Self> {
        let config = analysis.body_lexer_config?;
        let structure = SourceStructure::capture(source, Some(analysis), config)?;
        Some(Self {
            source,
            analysis,
            config,
            structure,
        })
    }

    pub(crate) fn reference_at(&self, cursor: u32) -> Option<SourceVariableReference<'a>> {
        if crate::definition::offset_is_inert(self.source, self.analysis, cursor)? {
            return None;
        }
        variable_in_structure(self.source, &self.structure, self.config, cursor, true)
    }
}

/// Original receiver and static selector syntax at a current command point.
/// These values grant no object identity, dispatch, visibility or edit coverage.
pub(crate) fn original_method_site_at(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<(String, String, bool)> {
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    if crate::inert_text::offset_in_physical_comment_in_analysis(source, analysis, cursor)? {
        return None;
    }
    let config = current.config();
    let structure = SourceStructure::capture(source, Some(analysis), config)?;
    let command = structure
        .commands
        .iter()
        .filter(|command| {
            let span = command.execution_span(source);
            span.start() <= cursor && cursor < span.end()
        })
        .min_by_key(|command| command.execution_span(source).len())?;
    if command.is_partial {
        return None;
    }
    if tcl_compiler::registry_invocation::source_structure::source_registry_words(
        source, analysis, command,
    )
    .is_some_and(|words| {
        words
            .written_argument_roles()
            .into_iter()
            .any(|(argument, role)| argument == 0 && role == ArgRole::Expr)
    }) {
        return None;
    }
    let image = SourceImage::document(source);
    let plan =
        tcl_lexer::native_script_words_in(image, command.execution_span(source), config).ok()?;
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    if plan.fatal_tail.is_some() {
        return None;
    }
    let head = command.words.first()?;
    let selector = command.words.get(1)?;
    if command.words.iter().any(|word| word.group().expand)
        || !(selector.span().start() <= cursor && cursor < selector.span().end())
    {
        return None;
    }
    let method = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        selector,
    )?)
    .ok()?;
    if method.contains('\0') {
        return None;
    }
    let arena = head.executable_parts();
    if let [part] = arena.list(arena.root()) {
        match part.part {
            ExecutablePart::Variable { .. } => {
                let written = arena.bytes(part.span)?;
                let reference = tcl_lexer::word_parts::whole_var_ref(written, config).ok()??;
                let name = if reference.index.is_some() {
                    let name_at =
                        (reference.name.as_ptr() as usize).checked_sub(written.as_ptr() as usize)?;
                    std::str::from_utf8(written.get(name_at..)?).ok()?
                } else {
                    std::str::from_utf8(reference.name).ok()?
                };
                if name.contains('\0') {
                    return None;
                }
                return Some((name.to_owned(), method, true));
            }
            ExecutablePart::Command { .. } => {
                let written = std::str::from_utf8(arena.bytes(part.span)?).ok()?;
                return Some((written.to_owned(), method, false));
            }
            ExecutablePart::Text(_) => {}
            ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => return None,
        }
    }
    let receiver = String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        head,
    )?)
    .ok()?;
    if receiver.contains('\0') {
        return None;
    }
    Some((receiver, method, false))
}

/// Selected original substitution syntax in one complete retained analysis.
/// Unknown comment/body applicability or stale correspondence declines. The
/// caller independently chooses lexical or original-variable resolution.
pub(crate) fn original_variable_reference_at<'a>(
    source: &'a str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<SourceVariableReference<'a>> {
    let config = analysis.body_lexer_config?;
    if crate::definition::offset_is_inert(source, analysis, cursor)? {
        return None;
    }
    let structure = SourceStructure::capture(source, Some(analysis), config)?;
    variable_in_structure(source, &structure, config, cursor, true)
}

/// Explicit standalone source syntax utility. It has no retained analysis and
/// offers no resolution, execution or edit authority. Actual editor consumers
/// use `original_variable_reference_at` instead of recapturing this input.
pub(crate) fn template_variable_reference_at(
    source: &str,
    config: LexerConfig,
    cursor: u32,
) -> Option<SourceVariableReference<'_>> {
    let structure = SourceStructure::capture(source, None, config)?;
    variable_in_structure(source, &structure, config, cursor, false)
}
