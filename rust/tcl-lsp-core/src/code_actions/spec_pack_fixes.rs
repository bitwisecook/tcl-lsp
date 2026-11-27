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

//! Did-you-mean quick fixes for `SpecTcl` pack-load notices.

use super::context_diagnostics::ContextDiagnostic;
use super::diagnostic_context::ContextDiagnosticData;
use super::spec_notice::SpecPackNoticeKind;
use super::{ActionKind, CodeAction};
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

/// The diagnostic code every `SpecTcl` pack-load notice carries.
///
/// One code for the family rather than one per notice kind: the notices are
/// *degradations* the loader chose to report, not a diagnostic catalogue with
/// per-code documentation and quick fixes behind it. A user who does not want
/// them silences the family.
///
/// It lives here rather than beside the publisher so the code the notices go
/// out under and the code the quick-fix below reads back are one string.
pub const SPEC_PACK_DIAGNOSTIC_CODE: &str = "SPECTCL";

/// How many candidates a did-you-mean rewrite offers.
///
/// One, not a menu: a pack-load notice names one word, and three near-equal
/// candidates out of a closed vocabulary is a worse answer than none — at that
/// point the author wants the vocabulary, which completion already offers on
/// the same line, not a ranking of guesses.
const SPEC_PACK_SUGGESTIONS: usize = 1;

/// Did-you-mean source edits require an actual typed loader notice and
/// independently current complete source/configuration/Registry. Prose is
/// presentation and cannot select a word, row, vocabulary or edit extent.
#[must_use]
pub fn spec_pack_quick_fixes(
    source: &str,
    analysis: &AnalysisResult,
    diags: &[ContextDiagnostic],
) -> Vec<CodeAction> {
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    if registry.document_grammar().is_none() {
        return Vec::new();
    }
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();
    for d in diags.iter().filter(|d| d.code == SPEC_PACK_DIAGNOSTIC_CODE) {
        let Some(data) = d.data.as_ref().and_then(ContextDiagnosticData::notice) else {
            continue;
        };
        if !data.matches(source, analysis, registry, d) {
            continue;
        }
        let subject = &data.subject;
        let word = subject.word();
        let offset = subject.span().start();
        let Some(head) = statement_head_at(source, offset, config) else {
            continue;
        };
        let vocabulary = crate::oo_body::definition_grammar_from_analysis(source, analysis, offset);
        let grammar = crate::source_structure::SourceDefinitionGrammar::for_selected_source(
            &vocabulary,
            source,
            analysis,
        );
        let candidates: Vec<&str> = match subject.kind() {
            SpecPackNoticeKind::Property if head == word => {
                grammar.map(|g| g.members.iter().map(|m| m.keyword).collect())
            }
            SpecPackNoticeKind::Flag { row } if head == *row => registry
                .get(&head)
                .map(|spec| spec.options.iter().map(|opt| opt.name).collect()),
            _ => None,
        }
        .unwrap_or_default();
        let Some(&suggestion) = tcl_source::text::suggest_similar(
            word,
            candidates,
            SPEC_PACK_SUGGESTIONS,
            tcl_source::text::scaled_max_distance(word),
        )
        .first() else {
            continue;
        };
        out.push(CodeAction::new(
            format!("Change `{word}` to `{suggestion}`"),
            vec![crate::rename::TextEdit {
                range: crate::definition::span_to_range(source, &line_index, subject.span()),
                new_text: suggestion.into(),
            }],
            ActionKind::QuickFix,
            None,
        ));
    }
    out
}

/// The keyword of the statement `offset` falls inside.
///
/// A statement is not a line: Tcl ends one at `;` as readily as at a newline,
/// so on `arity 1; arty 2` the line's first word names the *neighbouring* row
/// and every candidate set derived from it is the wrong one. The segmenter is
/// what knows where a row begins, and the descent into braced words is what
/// brings a nested row — every row of a pack is nested at least twice — within
/// its reach.
///
/// `None` when no statement covers the offset (a word inside a comment, a
/// nest deeper than the walk follows), which costs an action rather than
/// offering one computed against the wrong row.
pub(super) fn statement_head_at(
    source: &str,
    offset: u32,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    fn walk(
        source: &str,
        slice: &str,
        base: u32,
        offset: u32,
        depth: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<String> {
        // The same bound every other walk over a nested body carries.
        if depth > 32 {
            return None;
        }
        for cmd in tcl_source::segment::segment_commands_with_offset_and_config(
            slice,
            base,
            config.at_depth(depth),
        ) {
            if offset < cmd.span.start() || offset >= cmd.span.end() {
                continue;
            }
            // A braced word holding the offset is a script of its own, and the
            // row the notice points at is one of *its* statements.
            for tok in cmd.argv.iter().skip(1) {
                if tok.kind != tcl_lexer::TokenType::Str {
                    continue;
                }
                let inner_start = tok
                    .span
                    .start()
                    .saturating_add(u32::from(tok.content_offset));
                let inner_end = tok
                    .span
                    .end()
                    .min(u32::try_from(source.len()).unwrap_or(u32::MAX));
                if inner_start > inner_end || offset < inner_start || offset >= inner_end {
                    continue;
                }
                let Some(inner) = source.get(inner_start as usize..inner_end as usize) else {
                    continue;
                };
                return walk(source, inner, inner_start, offset, depth + 1, config);
            }
            return Some(cmd.name().to_owned());
        }
        None
    }

    walk(source, source, 0, offset, 0, config)
}
