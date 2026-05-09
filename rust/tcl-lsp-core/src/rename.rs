//! Rename provider — minimal Rust port of
//! `lsp/features/rename.py`.
//!
//! Computes a workspace edit that renames the symbol at the
//! cursor across the current document.  Ports the simple
//! cases of `lsp/features/rename.py`:
//!
//! * `$var` references → rewrite the `VarDef.definition_span`
//!   and every `VarDef.references` span to the new name.
//! * proc references → rewrite `ProcDef.name_span` and every
//!   matching command-invocation head to the new name.
//!
//! What is *deferred* (planned as `S-rename-rich` follow-up):
//!
//! * Symbol-validity gating (Python's `_is_safe_symbol_name`,
//!   `_is_builtin_command_name`) — the minimal port emits the
//!   edit unconditionally and lets the editor reject invalid
//!   names.
//! * Namespace-aware proc renames (Python's `_namespace_prefix`
//!   /`_tail_name` machinery — when renaming `::ns::greet` to
//!   `hi`, Python knows to rewrite call sites that use the
//!   short `greet` form too).
//! * Variable-name escaping for `${name}` braced references.
//! * Class / method rename — the Python provider has separate
//!   code paths for those that the minimal port doesn't yet
//!   surface.
//! * Cross-document rename — the workspace-index integration
//!   that lands alongside `S-workspace-symbols`.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::{find_var_at_position, find_word_span_at_position};

/// One text edit in a rename — span plus replacement text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Range to replace (byte spans translated to LSP
    /// line/character coordinates).
    pub range: LspRange,
    /// Replacement text.
    pub new_text: String,
}

/// Compute the text edits for a rename of the symbol at the
/// cursor.
///
/// Returns an empty vector when no recognisable symbol is at
/// the position. The caller (server) is responsible for
/// wrapping the output in a `WorkspaceEdit { changes: { uri:
/// edits } }`.
#[must_use]
pub fn rename(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
) -> Vec<TextEdit> {
    let line_index = LineIndex::new(source);

    if let Some(var_name) = find_var_at_position(source, line, character) {
        let Some(var_def) = analysis.global_scope.variables.get(&var_name) else {
            return Vec::new();
        };
        let mut edits = Vec::with_capacity(1 + var_def.references.len());
        edits.push(TextEdit {
            range: span_to_range(&line_index, var_def.definition_span),
            new_text: new_name.to_owned(),
        });
        for r in &var_def.references {
            edits.push(TextEdit {
                range: span_to_range(&line_index, *r),
                new_text: new_name.to_owned(),
            });
        }
        return edits;
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            let mut edits = Vec::new();
            edits.push(TextEdit {
                range: span_to_range(&line_index, proc_def.name_span),
                new_text: new_name.to_owned(),
            });
            let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
            for inv in &analysis.command_invocations {
                if inv.name == proc_def.name
                    || inv.name == proc_def.qualified_name
                    || inv.name == qname_no_prefix
                {
                    edits.push(TextEdit {
                        range: span_to_range(&line_index, inv.range),
                        new_text: new_name.to_owned(),
                    });
                }
            }
            dedup_edits(&mut edits);
            return edits;
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

fn dedup_edits(edits: &mut Vec<TextEdit>) {
    let mut seen: std::collections::HashSet<(u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    edits.retain(|e| {
        let key = (
            e.range.start_line,
            e.range.start_character,
            e.range.end_line,
            e.range.end_character,
        );
        seen.insert(key)
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
    fn rename_proc_includes_decl_and_calls() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let edits = rename(src, 0, 6, "hi", &analysis);
        assert!(!edits.is_empty());
        assert!(edits.iter().all(|e| e.new_text == "hi"));
        // First edit is the declaration on line 0 col 5.
        assert_eq!(edits[0].range.start_line, 0);
        assert_eq!(edits[0].range.start_character, 5);
    }

    #[test]
    fn rename_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(rename(src, 0, 6, "x", &analysis).is_empty());
    }

    #[test]
    fn rename_var_includes_decl_span() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor inside `$x`.
        let edits = rename(src, 1, 7, "y", &analysis);
        assert!(!edits.is_empty());
        assert!(edits.iter().all(|e| e.new_text == "y"));
    }
}
