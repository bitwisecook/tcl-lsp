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

//! Formatting provider.
//!
//! [`formatting`] produces a single full-document `TextEdit`
//! by running the token-aware [`engine::format_tcl`] with a
//! default [`FormatterConfig`].  The engine is more than a
//! whitespace pass — it normalises:
//!
//! * Brace placement (K&R: `{` kept on the command line, the
//!   matching `}` re-anchored).
//! * Indentation through selected script regions (configurable width),
//!   including continuation lines.
//! * Trailing-whitespace trimming and tab-to-space conversion.
//! * Blank-line policy (collapsing runs, blank lines around
//!   procs) and a single trailing newline.
//! * Comment normalisation, switch-body formatting, long-line
//!   backslash splitting, and `&&` / `||` expression wrapping.
//!
//! Body layout uses current Registry source roles and complete dialect grammar.
//! Expression or keyword rewriting requires its own selected permission.
//!
//! [`range_formatting`] re-normalises just the requested line
//! slice (extended to whole lines), selecting its script region through
//! current body presentation and complete lexical words, so
//! `textDocument/rangeFormatting` ("format selection") leaves
//! the rest of the document untouched.
//!
//! Original-source expression bracing consumes a separate bounded literal
//! equivalence receipt. Dynamic expressions retain their written operands.

mod analysis_layout;
pub mod config;
pub mod docstring;
pub mod engine;
pub(crate) mod keywords;
mod source_layout;

pub use config::{
    DocstringStyle, DocstringTagStyle, FormatterConfig, IndentStyle, LINE_ENDING_AUTO,
    detect_line_ending,
};
pub use docstring::{
    DocstringInfo, ParamDoc, generate_stub_for_proc, parse_docstring, render_comment_block,
    resolve_tag_style,
};
pub use engine::{format_tcl, format_tcl_with_input};

use crate::definition::LspRange;
use crate::rename::TextEdit;
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

/// Canonicalise document line endings before they reach the formatter parser.
///
/// Tcl reads a script channel with automatic newline translation, so both
/// CRLF and a lone CR are parser-facing LF line endings. This boundary is for
/// documents only: run-time string values keep raw CR / CRLF semantics.
fn normalise_document_line_endings(source: &str) -> std::borrow::Cow<'_, str> {
    let lone_cr_normalised = tcl_lexer::normalise_lone_cr(source);
    if lone_cr_normalised.contains("\r\n") {
        std::borrow::Cow::Owned(lone_cr_normalised.replace("\r\n", "\n"))
    } else {
        lone_cr_normalised
    }
}

/// Compute formatting edits for the entire document.
///
/// Runs the token-aware [`engine::format_tcl`] with the documented lenient
/// modern-Tcl context and returns a single `TextEdit` that
/// replaces the whole document with its normalised form, or an
/// empty `Vec` when the document is already normalised.
#[must_use]
pub fn formatting(source: &str, registry: &CommandRegistry) -> Vec<TextEdit> {
    formatting_with(source, &FormatterConfig::default(), registry)
}

/// Compute whole-document formatting edits with an explicit
/// [`FormatterConfig`].
///
/// Identical to [`formatting`] but honours a caller-supplied
/// config so the server can map an LSP request's `tabSize` /
/// `insertSpaces` onto the formatter's indentation (an explicit
/// client `FormattingOptions` overrides the server default by LSP
/// contract).
#[must_use]
pub fn formatting_with(
    source: &str,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Vec<TextEdit> {
    let formatted = engine::format_tcl(source, config, registry);
    full_document_edit(source, formatted)
}

/// Format a document under its actual resolved context and full lexer rules.
#[must_use]
pub fn formatting_with_input(
    source: &str,
    config: &FormatterConfig,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> Vec<TextEdit> {
    full_document_edit(source, engine::format_tcl_with_input(source, config, input))
}

/// Format a current analysed document. Unavailable, stale or incomplete source
/// owners supply no formatting edits; style settings do not repair that owner.
#[must_use]
pub fn formatting_with_analysis(
    source: &str,
    config: &FormatterConfig,
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> Vec<TextEdit> {
    let Some(input) = original_formatting_input(source, analysis) else {
        return Vec::new();
    };
    let config = config.for_resolved_input(input);
    if !engine::source_within_formatting_budget(&normalise_document_line_endings(source), &config) {
        return Vec::new();
    }
    let context = input.context_registry();
    let Some(layout) = engine::FormattingSourceLayout::with_analysis(source, analysis) else {
        return Vec::new();
    };
    full_document_edit(
        source,
        engine::format_tcl_with_layout(source, &config, context.commands(), layout),
    )
}

fn original_formatting_input<'a>(
    source: &str,
    analysis: &'a tcl_compiler::analyser::AnalysisResult,
) -> Option<&'a tcl_compiler::analyser::ResolvedAnalysisInput> {
    // Analysis replaces lone CRs without changing source offsets. Formatter
    // output and client edit coordinates still use the original terminators.
    let analysis_source = tcl_lexer::normalise_lone_cr(source);
    crate::original_context::CurrentSourceContext::capture(&analysis_source, analysis)?;
    analysis.resolved_input.as_ref()
}

fn full_document_edit(source: &str, formatted: String) -> Vec<TextEdit> {
    if formatted == source {
        return Vec::new();
    }
    // The replace range is expressed in the **client's** coordinates, so it
    // must be built on the client's EOL model (`\n`, `\r\n`, *and* a lone
    // `\r`), not the lexer/CST `\n`-only one.  With `LineIndex::new` an
    // old-Mac document reported an end position on line 0 and the client
    // spliced the formatted text over only the first line, duplicating the
    // rest of the file (and a mixed document lost its tail the same way).
    let line_index = LineIndex::new_lsp(source);
    let end_pos = line_index.position_at_utf16(u32::try_from(source.len()).unwrap_or(0), source);
    vec![TextEdit {
        range: LspRange {
            start_line: 0,
            start_character: 0,
            end_line: end_pos.line,
            end_character: end_pos.character.get(),
        },
        new_text: formatted,
    }]
}

/// Compute formatting edits for a range within the
/// document.
///
/// True range-aware formatting: only the line slice
/// `[range.start_line, range.end_line]` (extended to whole
/// lines) is re-normalised inside its selected source script region.
/// A selection that cuts through a data word is preserved. Emits a single
/// `TextEdit` that replaces the slice
/// with its formatted form, or an empty `Vec` when the
/// slice is already normalised.
///
/// Range-formatting only touches the line range requested;
/// edits outside it are left untouched.  Editors that
/// invoke `textDocument/rangeFormatting` (eg. `format
/// selection`) only need the selected slice to change.
#[must_use]
pub fn range_formatting(
    source: &str,
    range: LspRange,
    config: &FormatterConfig,
    registry: &CommandRegistry,
) -> Vec<TextEdit> {
    range_formatting_impl(source, range, config, registry, None, None)
}

/// Format a selection using whole-document naming under the actual context.
#[must_use]
pub fn range_formatting_with_input(
    source: &str,
    range: LspRange,
    config: &FormatterConfig,
    input: &tcl_compiler::analyser::ResolvedAnalysisInput,
) -> Vec<TextEdit> {
    let config = config.for_resolved_input(input);
    let context = input.context_registry();
    range_formatting_impl(
        source,
        range,
        &config,
        context.commands(),
        Some(input),
        None,
    )
}

/// Format a selection only when the whole original analysis is still current.
#[must_use]
pub fn range_formatting_with_analysis(
    source: &str,
    range: LspRange,
    config: &FormatterConfig,
    analysis: &tcl_compiler::analyser::AnalysisResult,
) -> Vec<TextEdit> {
    let Some(input) = original_formatting_input(source, analysis) else {
        return Vec::new();
    };
    let config = config.for_resolved_input(input);
    if !engine::source_within_formatting_budget(&normalise_document_line_endings(source), &config) {
        return Vec::new();
    }
    let context = input.context_registry();
    let Some(layout) = engine::FormattingSourceLayout::with_analysis(source, analysis) else {
        return Vec::new();
    };
    range_formatting_impl(
        source,
        range,
        &config,
        context.commands(),
        None,
        Some(layout),
    )
}

fn range_formatting_impl(
    source: &str,
    range: LspRange,
    config: &FormatterConfig,
    registry: &CommandRegistry,
    input: Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
    layout: Option<engine::FormattingSourceLayout>,
) -> Vec<TextEdit> {
    // Tcl's source-channel boundary maps every document line ending to LF.
    // The normalised text is only an internal formatter input; raw spans and
    // client coordinates still use the LSP line index below.
    let normalised = normalise_document_line_endings(source);
    // Implementation contract: naming.editor.original-source-formatting-budget
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
    // Budget withdrawal preserves the caller's complete original document and
    // never constructs an unavailable layout that could borrow nominal roles.
    if !engine::source_within_formatting_budget(&normalised, config) {
        return Vec::new();
    }
    let lines: Vec<&str> = normalised.split('\n').collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let line_count = u32::try_from(lines.len()).unwrap_or(u32::MAX);
    let start_line = range.start_line.min(line_count.saturating_sub(1));
    let end_line = range
        .end_line
        .min(line_count.saturating_sub(1))
        .max(start_line);

    // Slice of lines we re-format, run through the same
    // token-aware engine the full-document path uses
    // ([`engine::format_body`]) at the selected presentation depth, so
    // range formatting and full-document formatting share every
    // rule (comment normalisation, switch bodies, line wrapping,
    // configurable indent width, …).
    let slice_end = (end_line as usize) + 1;
    let slice_lines: Vec<&str> = lines[start_line as usize..slice_end].to_vec();
    let slice_text = slice_lines.join("\n");
    // The document's own line ending (or the configured one) — resolved
    // against the whole document, not the slice, so a one-line selection in a
    // CRLF file is not re-emitted with `\n`.
    let line_ending = config.resolved_line_ending(source);
    // The identity facts come from the **whole document**, not the slice: a
    // `rename` above the selection still governs what the selected commands
    // are.
    let identities = layout.unwrap_or_else(|| {
        input.map_or_else(
            || engine::FormattingSourceLayout::new(&normalised, config, registry),
            |input| engine::FormattingSourceLayout::with_input(&normalised, input),
        )
    });
    let index = LineIndex::new(&normalised);
    let slice_source_offset = index.line_start(start_line);
    let slice_source_end = if slice_end < lines.len() {
        index.line_start(u32::try_from(slice_end).unwrap_or(u32::MAX))
    } else {
        u32::try_from(normalised.len()).unwrap_or(u32::MAX)
    };
    let Some(depth) = engine::formatting_range_indent(
        &normalised,
        tcl_lexer::Span::new(slice_source_offset, slice_source_end),
        config,
        registry,
        &identities,
    ) else {
        return Vec::new();
    };
    let formatted_slice = finalise_slice(
        &engine::format_body(
            &slice_text,
            slice_source_offset,
            config,
            registry,
            &identities,
            depth,
        ),
        config,
        line_ending,
    );
    // Skip no-op edits: compare the formatted slice against the slice
    // text *as it currently is in the document* (raw, untrimmed) plus
    // the trailing newline the replacement range carries.  Finalising
    // the original here would hide trailing-whitespace-only changes.
    // The comparison uses the *raw* bytes of the same span so an already
    // formatted CRLF / old-Mac slice compares equal instead of producing a
    // spurious edit that rewrites its own terminators.
    let raw_slice = raw_span(source, start_line, end_line, line_count);
    let original_with_nl = if raw_slice.ends_with('\n') || raw_slice.ends_with('\r') {
        raw_slice.to_owned()
    } else {
        format!("{raw_slice}{line_ending}")
    };
    if formatted_slice == original_with_nl {
        return Vec::new();
    }

    // Replacement range covers the full slice, line-anchored
    // (column 0 of `start_line` to column 0 of the line
    // *after* `end_line`).  When `end_line` is the last line
    // of the document, anchor the end at the post-final-char
    // position so editors interpret the edit correctly.
    let edit_range = if (end_line + 1) < line_count {
        LspRange {
            start_line,
            start_character: 0,
            end_line: end_line + 1,
            end_character: 0,
        }
    } else {
        // `end_line` is the document's last line, so the slice runs
        // to EOF.  Derive the end column via `LineIndex` (the same
        // position encoding the full-document path uses) so the
        // range is correct for non-ASCII text rather than counting
        // raw `char`s — on the client's EOL model, as the range is
        // sent back to the client.
        let line_index = LineIndex::new_lsp(source);
        let end_pos =
            line_index.position_at_utf16(u32::try_from(source.len()).unwrap_or(u32::MAX), source);
        LspRange {
            start_line,
            start_character: 0,
            end_line: end_pos.line,
            end_character: end_pos.character.get(),
        }
    };
    vec![TextEdit {
        range: edit_range,
        new_text: formatted_slice,
    }]
}

/// The raw bytes of the document span the formatted slice replaces.
///
fn raw_span(source: &str, start_line: u32, end_line: u32, line_count: u32) -> &str {
    let index = LineIndex::new_lsp(source);
    let start = index.line_start(start_line) as usize;
    let end = if end_line + 1 < line_count {
        index.line_start(end_line + 1) as usize
    } else {
        source.len()
    };
    source.get(start..end).unwrap_or(source)
}

/// Apply the engine's tail post-processing to a formatted slice:
/// trailing-whitespace trimming, a single trailing newline (the
/// range edit replaces through the start of the following line), and
/// the resolved `line_ending`.  Mirrors the tail of
/// [`engine::format_tcl`] minus the document-level final-newline
/// policy.
fn finalise_slice(text: &str, config: &FormatterConfig, line_ending: &str) -> String {
    let mut out = if config.trim_trailing_whitespace {
        // Brace/quote-aware trim so a multi-line string literal's interior is
        // preserved.
        engine::trim_trailing_ws_preserving_literals(text, config)
    } else {
        text.to_owned()
    };
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if line_ending != "\n" {
        out = out.replace('\n', line_ending);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysed_formatting_keeps_current_source_and_typed_unavailability() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
        let source = "if 1 {\nset value 1\n}\n";
        let config = FormatterConfig::default();
        let analysis = Analyser::new().analyse(source, "tcl");
        let range = LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 2,
            end_character: 1,
        };
        let expected = formatting_with_analysis(source, &config, &analysis);
        assert!(!expected.is_empty());
        assert!(!range_formatting_with_analysis(source, range, &config, &analysis).is_empty());
        let mut reported = analysis.clone();
        reported.dialect = "presentation only".into();
        assert_eq!(
            formatting_with_analysis(source, &config, &reported),
            expected
        );

        let mut unavailable = analysis.clone();
        unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
            environment: "tcl".into(),
            overlay: 1,
        });
        assert!(unavailable.resolved_input.is_some());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
        let mut foreign = analysis.clone();
        let input = foreign.resolved_input.take().unwrap();
        foreign.resolved_input = Some(ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::resolve_environment("tcl8.4").default_context_registry(),
            input.lexer_config(),
        ));
        for withdrawn in [&unavailable, &missing, &changed, &foreign] {
            assert!(formatting_with_analysis(source, &config, withdrawn).is_empty());
            assert!(range_formatting_with_analysis(source, range, &config, withdrawn).is_empty());
        }
        assert!(formatting_with_analysis("set replacement 1", &config, &analysis).is_empty());
        assert!(
            range_formatting_with_analysis("set replacement 1", range, &config, &analysis)
                .is_empty()
        );
    }

    #[test]
    fn analysed_formatting_keeps_original_terminators_and_lone_cr_analysis_currency() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        use tcl_compiler::analyser::Analyser;
        for ending in ["\n", "\r\n", "\r"] {
            let source = format!("if 1 {{{ending}set value 1{ending}}}{ending}");
            let analysis_source = tcl_lexer::normalise_lone_cr(&source);
            let analysis = Analyser::new().analyse(&analysis_source, "tcl");
            let edits = formatting_with_analysis(&source, &FormatterConfig::default(), &analysis);
            assert_eq!(edits.len(), 1);
            assert!(edits[0].new_text.ends_with(ending));
            assert_eq!(edits[0].range.end_line, 3);
            assert_eq!(edits[0].range.end_character, 0);
        }
    }

    #[test]
    fn analysed_formatting_joins_complete_original_alias_words_across_crlf_and_unicode() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        // Source layout/edit coordinates only; captured prefixes gain no written word.
        use tcl_compiler::analyser::Analyser;
        let source =
            "interp alias {} choose {} if 1\r\nchoose {if 1 {puts α}}\r\nset data {a\r\nb}\r\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let config = FormatterConfig::default();
        let edits = formatting_with_analysis(source, &config, &analysis);
        assert_eq!(edits.len(), 1);
        assert!(
            edits[0].new_text.contains("choose {\r\n    if"),
            "{}",
            edits[0].new_text
        );
        assert!(
            edits[0].new_text.contains("        puts α\r\n"),
            "{}",
            edits[0].new_text
        );
        assert!(edits[0].new_text.contains("set data {a\r\nb}"));
        assert_eq!(edits[0].range.end_line, 4);
        assert_eq!(edits[0].range.end_character, 0);
        let selected = LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 30,
        };
        let range = range_formatting_with_analysis(source, selected, &config, &analysis);
        assert_eq!(range.len(), 1);
        assert!(range[0].new_text.contains("        puts α\r\n"));
        assert_eq!(range[0].range.start_line, 1);
        assert_eq!(range[0].range.end_line, 2);
        let data = LspRange {
            start_line: 3,
            start_character: 0,
            end_line: 3,
            end_character: 1,
        };
        assert!(range_formatting_with_analysis(source, data, &config, &analysis).is_empty());
    }

    #[test]
    fn analysed_formatting_keeps_replaced_unknown_and_provider_barrier_purposes_separate() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        // The coverage field neither closes unknown dispatch nor cancels authentic syntax advice.
        use tcl_compiler::analyser::Analyser;
        let config = FormatterConfig::default();
        for source in [
            "proc if {args} {}; if 1 {puts α}\r\n",
            "mystery 1 {puts α}\r\n",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let edits = formatting_with_analysis(source, &config, &analysis);
            let output = edits.first().map_or(source, |edit| edit.new_text.as_str());
            assert!(!output.contains("    puts α"), "{output}");
            assert!(output.contains("{puts α}"), "{output}");
        }
        let source = "if 1 {puts α}\r\n";
        let mut analysis = Analyser::new().analyse(source, "tcl");
        let positive = formatting_with_analysis(source, &config, &analysis);
        assert_eq!(positive.len(), 1);
        assert!(positive[0].new_text.contains("    puts α\r\n"));
        analysis.has_dynamic_providers = true;
        assert_eq!(
            formatting_with_analysis(source, &config, &analysis),
            positive
        );
    }

    #[test]
    fn analysed_formatting_retains_selected_definition_parent_body_geometry() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        // Member script applicability comes from the genuine retained parent vocabulary.
        use tcl_compiler::analyser::Analyser;
        for ending in ["\n", "\r\n", "\r"] {
            let source = format!("oo::class create C {{method m {{}} {{puts α}}}}{ending}");
            let selected_source = tcl_lexer::normalise_lone_cr(&source);
            let analysis = Analyser::new().analyse(&selected_source, "tcl");
            let edits = formatting_with_analysis(&source, &FormatterConfig::default(), &analysis);
            assert_eq!(edits.len(), 1, "{ending:?}");
            let expected = format!("    method m {{}} {{{ending}        puts α{ending}    }}");
            assert!(
                edits[0].new_text.contains(&expected),
                "{}",
                edits[0].new_text
            );
        }
    }

    #[test]
    fn analysed_formatting_member_vocabulary_precedes_same_spelling_global_alias() {
        // naming.editor.original-source-formatting
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
        // Genuine parent syntax; a global alias grants no member role or trait.
        use tcl_compiler::analyser::Analyser;
        let source =
            "interp alias {} method {} if 1\r\noo::class create C {method m {} {puts α}}\r\n";
        let analysis = Analyser::new().analyse(source, "tcl");
        let edits = formatting_with_analysis(source, &FormatterConfig::default(), &analysis);
        assert_eq!(edits.len(), 1);
        assert!(
            edits[0]
                .new_text
                .contains("    method m {} {\r\n        puts α\r\n    }"),
            "{}",
            edits[0].new_text,
        );
        let structure =
            crate::source_structure::SourceSyntaxStructure::capture(source, &analysis).unwrap();
        let head = u32::try_from(source.rfind("method m").unwrap()).unwrap();
        let member = structure.definition_member_at(head).unwrap().unwrap();
        assert_eq!(member.script_bodies().unwrap().len(), 1);
        assert_eq!(member.script_bodies().unwrap()[0].argument(), 2);
        let helper = "proc f {} {method m {} {puts α}}\r\n";
        let analysis = Analyser::new().analyse(helper, "tcl");
        let structure =
            crate::source_structure::SourceSyntaxStructure::capture(helper, &analysis).unwrap();
        let head = u32::try_from(helper.find("method m").unwrap()).unwrap();
        assert!(structure.definition_member_at(head).unwrap().is_none());
    }

    fn range_fmt(source: &str, range: LspRange) -> Vec<TextEdit> {
        let registry = tcl_registry::CommandRegistry::build_default();
        range_formatting(source, range, &FormatterConfig::default(), &registry)
    }

    #[test]
    fn range_formatting_withdraws_before_analysis_above_lexical_budget() {
        // Implementation contract: naming.editor.original-source-formatting-budget
        // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting-budget.md
        let source = format!(
            "set value {{{}}}\r\n",
            "{".repeat(128) + "unformatted  " + &"}".repeat(128)
        );
        let registry = CommandRegistry::build_default();
        let config = FormatterConfig::default();
        let edits = range_formatting(
            &source,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            &config,
            &registry,
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn range_formatting_honours_configurable_indent_width() {
        // The engine (not the old hardcoded 4-space formatter) drives
        // range formatting now, so a non-default `indent_size` is
        // respected — proving range and full-document formatting share
        // the same `FormatterConfig`-aware engine path.
        let registry = tcl_registry::CommandRegistry::build_default();
        let config = FormatterConfig {
            indent_size: 2,
            ..FormatterConfig::default()
        };
        let src = "proc foo {} {\nset x 1\n}\n";
        let edits = range_formatting(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
            &config,
            &registry,
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert!(
            edits[0].new_text.starts_with("  set x 1"),
            "expected 2-space indent; got {:?}",
            edits[0].new_text,
        );
    }

    #[test]
    fn already_formatted_returns_no_edits() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "proc foo {} {\n    set x 1\n}\n";
        assert!(
            formatting(src, &registry).is_empty(),
            "{:?}",
            formatting(src, &registry)
        );
    }

    #[test]
    fn range_depth_uses_selected_bodies_and_ignores_quoted_braces() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        let source = "set literal \"{\n}\"\nproc p {} {\nif {1} {\nset x 1\n}\n}\n";
        let edits = range_fmt(
            source,
            LspRange {
                start_line: 4,
                start_character: 0,
                end_line: 4,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].new_text, "        set x 1\n");
    }

    #[test]
    fn partial_data_word_ranges_preserve_exact_crlf_and_opaque_spelling() {
        // naming.editor.original-source-whitespace-geometry
        // docs/design/analysis/name-resolution-proofs/original-source-whitespace-geometry.md
        for source in [
            "set x \"literal   \r\n{ \\uD800   \r\n}\"\r\n",
            "set x \"[set y \"inner   \r\ntext\"] outer\"\r\n",
            "set x {literal   \r\n{ \\uD800   \r\n}}\r\n",
            "unknown {set x 1   \r\nset y 2}\r\n",
        ] {
            let edits = range_fmt(
                source,
                LspRange {
                    start_line: 1,
                    start_character: 0,
                    end_line: 1,
                    end_character: 100,
                },
            );
            assert!(edits.is_empty(), "{source:?}: {edits:?}");
        }
    }

    #[test]
    fn range_formatting_emits_edit_for_dirty_range() {
        let src = "set x 1   \n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 5,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert!(edits[0].new_text.contains("set x 1"), "{edits:?}");
        // Trailing whitespace stripped.
        assert!(!edits[0].new_text.contains("   "), "{edits:?}");
    }

    #[test]
    fn range_formatting_uses_tcl_source_newline_translation_for_crlf() {
        // The formatter parses the selected CRLF document through Tcl's
        // source-channel boundary, but its edit still covers exactly the
        // first client line and preserves the document's CRLF convention.
        let src = "set x 1   \r\nset y 2\r\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].new_text, "set x 1\r\n");
        assert_eq!(edits[0].range.end_line, 1);
        assert_eq!(edits[0].range.end_character, 0);
    }

    #[test]
    fn range_formatting_no_edits_when_slice_is_clean() {
        // Whole document is already formatted — range over a
        // clean slice should emit no edits.
        let src = "proc foo {} {\n    set x 1\n}\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 2,
                end_character: 0,
            },
        );
        assert!(edits.is_empty(), "{edits:?}");
    }

    #[test]
    fn range_formatting_preserves_brace_depth_from_prefix() {
        // Inside a proc body, the lines should be indented
        // 4 spaces.  Format only line 1 (the body's `set x`
        // line) — the formatter must pick up `depth = 1`
        // from the prefix walk.
        let src = "proc foo {} {\nset x 1\n}\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        // Inside the proc body — should be indented 4 spaces.
        assert!(
            edits[0].new_text.starts_with("    set x 1"),
            "expected indented set; got {:?}",
            edits[0].new_text,
        );
    }

    #[test]
    fn range_formatting_resolves_aliases_at_the_slice_source_offset() {
        let src = concat!(
            "interp alias {} pick {} switch\n",
            "pick subject {default {puts    through_alias}}\n",
        );
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 100,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert!(
            edits[0]
                .new_text
                .contains("default {\n        puts through_alias\n    }"),
            "the alias declaration before the selected slice was ignored: {edits:?}"
        );
    }

    #[test]
    fn range_formatting_clamps_end_at_eof() {
        // Source has 2 lines; request a range whose end
        // extends past EOF.  Should still emit one valid
        // edit anchored at the final line's end.
        let src = "set x 1   \nset y 2\n";
        let edits = range_fmt(
            src,
            LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 99,
                end_character: 0,
            },
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
    }
}
