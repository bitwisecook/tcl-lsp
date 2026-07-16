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

//! Linked editing range provider.
//!
//! When the cursor sits on a proc name (either at the
//! declaration site or at a self-call inside the proc's own
//! body), this provider returns every range that should be
//! edited in lock-step — typically the declaration plus all
//! recursive self-calls.  Editors that honour the
//! `linkedEditingRangeProvider` capability paint these as
//! linked-edit chips so renaming one updates the others as
//! the user types.
//!
//! The result is intentionally narrow: we only return *self*
//! call sites that fall inside the proc's body span.  Cross-
//! proc rename remains the job of the rename / references
//! providers; linked editing is a live-edit affordance for the
//! "I'm writing a recursive proc and want to rename it" case.

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_lexer::{LineIndex, Span};

/// Word pattern matching the character set used for Tcl proc
/// names.  Editors validate live edits against this regex.
pub const WORD_PATTERN: &str = r"[A-Za-z_][A-Za-z0-9_]*";

/// A bundle of linked-editing ranges and their validating word
/// pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedEditingRanges {
    /// The ranges that edit together.
    pub ranges: Vec<LspRange>,
    /// Regex that newly-typed text must match before the
    /// editor commits the linked edits.
    pub word_pattern: String,
}

/// Return linked-editing ranges for `(line, character)`.
///
/// Returns `None` when:
///
/// * The cursor isn't on an identifier word.
/// * The word doesn't match a proc declaration whose body
///   (or own name span) contains the cursor.
/// * Fewer than two ranges are linkable (a single range
///   provides no benefit over a direct edit).
#[must_use]
pub fn linked_editing_ranges(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<LinkedEditingRanges> {
    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;
    let line_index = LineIndex::new(source);
    let proc = cursor_proc(&line_index, source, line, character, &word, analysis)?;

    // Every linked range must cover *identical* text (LSP contract: editing one
    // mirrors verbatim into the others).  The declaration's name-span text is
    // the canonical content; each call site is narrowed to the sub-span whose
    // text equals it, so a namespace-qualified self-call (`::greet`) links only
    // its `greet` tail — not the whole `::greet`, which would drop the `::` and
    // corrupt the call under rename-as-you-type (RUST_ISSUE_040).
    let decl_text = source
        .get(proc.name_span.start() as usize..proc.name_span.end() as usize)
        .unwrap_or("");
    let mut ranges: Vec<LspRange> = Vec::new();
    ranges.push(span_to_range(source, &line_index, proc.name_span));

    for inv in &analysis.command_invocations {
        // The call's resolved qualified name is authoritative when the analyser
        // settled one: a bare `greet` inside `namespace eval ::b { … }` nested
        // in `proc ::a::greet` resolves to `::b::greet`, so it must NOT link to
        // `::a::greet` even though its text equals `greet` (the old OR of the
        // name match with the resolved match wrongly linked it, corrupting the
        // unrelated call under rename-as-you-type).  Only when no qualified name
        // was settled do we fall back to the literal self-name match.
        let links_to_proc = match inv.resolved_qualified_name.as_deref() {
            Some(q) => q == proc.qualified_name,
            None => matches_self_call(inv.name.as_str(), proc),
        };
        if !links_to_proc {
            continue;
        }
        if !span_contains(proc.body_span, inv.range.start()) {
            continue;
        }
        // Only link the sub-span of the call head whose source text is
        // identical to the declaration name.
        let Some(matched) = identical_text_subspan(source, inv.range, decl_text) else {
            continue;
        };
        ranges.push(span_to_range(source, &line_index, matched));
    }

    dedup_ranges(&mut ranges);
    if ranges.len() < 2 {
        return None;
    }
    Some(LinkedEditingRanges {
        ranges,
        word_pattern: WORD_PATTERN.to_owned(),
    })
}

/// Find the proc the cursor sits inside — either on its name
/// span or anywhere in its body.  Matches the proc by short or
/// qualified name.
fn cursor_proc<'a>(
    line_index: &LineIndex,
    source: &str,
    line: u32,
    character: u32,
    word: &str,
    analysis: &'a AnalysisResult,
) -> Option<&'a ProcDef> {
    let byte_offset = crate::definition::byte_offset_at(line_index, source, line, character);
    for proc in analysis.all_procs.values() {
        if proc.name != word && proc.qualified_name != word {
            continue;
        }
        if span_contains(proc.name_span, byte_offset) {
            return Some(proc);
        }
        if span_contains(proc.body_span, byte_offset) {
            return Some(proc);
        }
    }
    None
}

fn matches_self_call(name: &str, proc: &ProcDef) -> bool {
    name == proc.name || name == proc.qualified_name
}

/// The sub-span of a call-head span whose source text is *identical* to
/// `decl_text`, so it can join a linked-editing group with the declaration
/// (whose ranges must all share content).
///
/// * When the whole head equals `decl_text`, the full span is returned.
/// * When the head is namespace-qualified and its final `::`-separated
///   component equals `decl_text` (`::greet` / `ns::greet` for a `greet`
///   declaration), the trailing-component sub-span is returned so the `::`
///   qualifier is preserved through the edit.
/// * Otherwise `None` — the texts differ and linking would corrupt one side.
fn identical_text_subspan(source: &str, span: Span, decl_text: &str) -> Option<Span> {
    let text = source.get(span.start() as usize..span.end() as usize)?;
    if text == decl_text {
        return Some(span);
    }
    // Trailing `::`-component match: `<qualifier>::<decl_text>`.
    let tail_start = text.len().checked_sub(decl_text.len())?;
    if tail_start >= 2
        && &text[tail_start..] == decl_text
        && &text[tail_start - 2..tail_start] == "::"
    {
        let start = span.start() + u32::try_from(tail_start).ok()?;
        return Some(Span::new(start, span.end()));
    }
    None
}

fn span_contains(span: Span, offset: u32) -> bool {
    // `Span` is half-open `[start, end)` so `offset == span.end()`
    // sits one byte past the span — strictly before the end is the
    // correct containment check.
    span.start() <= offset && offset < span.end()
}

fn span_to_range(source: &str, line_index: &LineIndex, span: Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

fn dedup_ranges(ranges: &mut Vec<LspRange>) {
    let mut seen: FxHashSet<(u32, u32, u32, u32)> = FxHashSet::default();
    ranges.retain(|r| seen.insert((r.start_line, r.start_character, r.end_line, r.end_character)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn returns_none_for_non_proc_word() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor on `puts` — a built-in, not a user proc.
        assert!(linked_editing_ranges(src, 1, 1, &analysis).is_none());
    }

    #[test]
    fn returns_none_for_proc_with_no_self_calls() {
        let src = "proc greet {name} { return $name }\n";
        let analysis = analyse(src);
        // Cursor on the proc declaration's name.  No self-call
        // inside the body, so the result is `None` (single
        // range can't be linked-edited).
        assert!(linked_editing_ranges(src, 0, 6, &analysis).is_none());
    }

    #[test]
    fn links_recursive_self_call_to_declaration() {
        let src = concat!(
            "proc factorial {n} {\n",
            "    return [factorial 1]\n",
            "}\n",
        );
        let analysis = analyse(src);
        // Cursor on the declaration name `factorial`.
        let result = linked_editing_ranges(src, 0, 6, &analysis)
            .expect("recursive self-call should link to declaration");
        assert!(
            result.ranges.len() >= 2,
            "expected declaration + recursive call, got {result:?}",
        );
        assert_eq!(result.word_pattern, WORD_PATTERN);
    }

    #[test]
    fn qualified_self_call_links_only_the_name_tail() {
        // RUST_ISSUE_040: `::greet` self-call must link only its `greet` tail so
        // every linked range covers identical text (`greet`); linking the whole
        // `::greet` would drop the `::` under rename-as-you-type.
        let src = "proc greet {} { ::greet }\n";
        let analysis = analyse(src);
        let result = linked_editing_ranges(src, 0, 6, &analysis)
            .expect("qualified self-call should link its name tail");
        // Every range must have identical width to the 5-char `greet` decl.
        for r in &result.ranges {
            assert_eq!(
                r.end_character - r.start_character,
                5,
                "a linked range covers different-width text than `greet`: {r:?}"
            );
        }
    }

    #[test]
    fn identical_text_subspan_matches_whole_and_tail() {
        // whole match
        assert_eq!(
            identical_text_subspan("greet", Span::new(0, 5), "greet"),
            Some(Span::new(0, 5))
        );
        // `::greet` (offsets 0..7): tail `greet` is 2..7
        assert_eq!(
            identical_text_subspan("::greet", Span::new(0, 7), "greet"),
            Some(Span::new(2, 7))
        );
        // `ns::greet` (0..9): tail 4..9
        assert_eq!(
            identical_text_subspan("ns::greet", Span::new(0, 9), "greet"),
            Some(Span::new(4, 9))
        );
        // A different name does not match.
        assert_eq!(
            identical_text_subspan("greeter", Span::new(0, 7), "greet"),
            None
        );
    }

    #[test]
    fn cursor_inside_body_also_links() {
        let src = concat!(
            "proc factorial {n} {\n",
            "    return [factorial 1]\n",
            "}\n",
        );
        let analysis = analyse(src);
        // Cursor on the `factorial` self-call inside the body
        // (line 1, column 14 — middle of `factorial`).
        let result = linked_editing_ranges(src, 1, 14, &analysis)
            .expect("cursor inside body on the recursive call should link");
        assert!(result.ranges.len() >= 2, "{result:?}");
    }
}
