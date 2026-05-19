//! Linked editing range provider — Rust port of
//! `lsp/features/linked_editing_range.py`.
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
use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_lexer::{LineIndex, Span};

/// Word pattern matching the character set used for Tcl proc
/// names.  Editors validate live edits against this regex.
pub const WORD_PATTERN: &str = r"[A-Za-z_][A-Za-z0-9_]*";

/// A bundle of linked-editing ranges and their validating word
/// pattern.  Mirrors `lsprotocol.types.LinkedEditingRanges`.
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
    let proc = cursor_proc(source, line, character, &word, analysis)?;
    let line_index = LineIndex::new(source);

    let mut ranges: Vec<LspRange> = Vec::new();
    ranges.push(span_to_range(&line_index, proc.name_span));

    for inv in &analysis.command_invocations {
        let resolved_matches = inv
            .resolved_qualified_name
            .as_deref()
            .is_some_and(|q| q == proc.qualified_name);
        if !matches_self_call(inv.name.as_str(), proc) && !resolved_matches {
            // Resolved-qualified-name follow-up: a relative
            // call inside `namespace eval ::ns { ... }` to its
            // own proc surfaces with `name = "greet"` and
            // `resolved_qualified_name = Some("::ns::greet")`.
            continue;
        }
        if !span_contains(proc.body_span, inv.range.start()) {
            continue;
        }
        ranges.push(span_to_range(&line_index, inv.range));
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
    source: &str,
    line: u32,
    character: u32,
    word: &str,
    analysis: &'a AnalysisResult,
) -> Option<&'a ProcDef> {
    let byte_offset = crate::definition::byte_offset_at(source, line, character);
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

fn span_contains(span: Span, offset: u32) -> bool {
    // `Span` is half-open `[start, end)` so `offset == span.end()`
    // sits one byte past the span — strictly before the end is the
    // correct containment check (PR #454 Copilot review).
    span.start() <= offset && offset < span.end()
}

fn span_to_range(line_index: &LineIndex, span: Span) -> LspRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LspRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
    }
}

fn dedup_ranges(ranges: &mut Vec<LspRange>) {
    let mut seen: std::collections::HashSet<(u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    ranges.retain(|r| {
        seen.insert((
            r.start_line,
            r.start_character,
            r.end_line,
            r.end_character,
        ))
    });
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

