//! Call-hierarchy provider — minimal Rust port of
//! `lsp/features/call_hierarchy.py`.
//!
//! Returns a single hierarchy item for the user-defined `proc`
//! at the cursor.  The incoming/outgoing edges are stub-empty;
//! computing them requires per-proc call-site / callee
//! tracking that the analyser doesn't surface today (deferred
//! to `S-call-hierarchy-rich`).

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::find_word_span_at_position;

/// One hierarchy item — proc identification plus its name and
/// definition span for editor display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallHierarchyItem {
    /// Proc name (qualified).
    pub name: String,
    /// Detail (e.g. parameter list summary).
    pub detail: Option<String>,
    /// Range of the entire definition.
    pub range: LspRange,
    /// Range of just the name token.
    pub selection_range: LspRange,
}

/// Resolve a "prepare call hierarchy" request to a single
/// item — the proc whose name is at the cursor.
#[must_use]
pub fn prepare(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Vec<CallHierarchyItem> {
    let line_index = LineIndex::new(source);
    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            let name_range = span_to_range(&line_index, proc_def.name_span);
            let body_range = span_to_range(&line_index, proc_def.body_span);
            let detail = if proc_def.params.is_empty() {
                None
            } else {
                Some(format!("({} params)", proc_def.params.len()))
            };
            // The full proc range covers from the proc name
            // to the body's end.
            let full_range = LspRange {
                start_line: name_range.start_line,
                start_character: name_range.start_character,
                end_line: body_range.end_line,
                end_character: body_range.end_character,
            };
            return vec![CallHierarchyItem {
                name: qname.clone(),
                detail,
                range: full_range,
                selection_range: name_range,
            }];
        }
    }
    Vec::new()
}

fn span_to_range(line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LspRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
    }
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
    fn prepare_resolves_proc_at_cursor() {
        let src = "proc greet {} {}\n";
        let analysis = analyse(src);
        let items = prepare(src, 0, 6, &analysis);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "::greet");
    }

    #[test]
    fn prepare_returns_empty_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(prepare(src, 0, 6, &analysis).is_empty());
    }
}
