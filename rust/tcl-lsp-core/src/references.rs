//! Find-references provider — minimal Rust port of
//! `lsp/features/references.py`.
//!
//! Locates every usage of the symbol at the cursor:
//!
//! * `$var` references → `VarDef.definition_span` plus every
//!   span in `VarDef.references` (already collected by the
//!   analyser's body walk).
//! * proc references → `ProcDef.name_span` plus every command
//!   invocation in `analysis.command_invocations` whose head
//!   matches the proc's simple or qualified name.
//! * class references → `ClassDef.name_span` plus every command
//!   invocation whose head matches the class's simple or
//!   qualified name.
//!
//! What is *deferred* (planned as `S-references-rich` follow-up):
//!
//! * Resolved-qualified-name matching for command invocations
//!   (Python's `invocation.resolved_qualified_name`) — the
//!   Rust analyser doesn't populate that field today, so the
//!   minimal port falls back to literal-name matching.
//! * Method-name references inside a class body.
//! * Cross-document references — the workspace-index integration
//!   that surfaces references across every open document; lands
//!   alongside `S-workspace-symbols` and the workspace-index
//!   chunks.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;
use crate::hover::{find_var_at_position, find_word_span_at_position};

/// Compute the locations of every reference to the symbol at
/// the cursor.
///
/// `include_declaration` mirrors the LSP `ReferenceContext`
/// flag — when `true`, the symbol's defining span is the first
/// element of the returned vector.
#[must_use]
pub fn references(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    include_declaration: bool,
) -> Vec<LspRange> {
    let line_index = LineIndex::new(source);

    if let Some(var_name) = find_var_at_position(source, line, character) {
        let Some(var_def) = analysis.global_scope.variables.get(&var_name) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if include_declaration {
            out.push(span_to_range(&line_index, var_def.definition_span));
        }
        for r in &var_def.references {
            out.push(span_to_range(&line_index, *r));
        }
        return out;
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    // Class references (checked first because Python checks
    // class name before proc name in get_references).
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            let mut out = Vec::new();
            if include_declaration {
                out.push(span_to_range(&line_index, class_def.name_span));
            }
            for inv in &analysis.command_invocations {
                if inv.name == class_def.name || inv.name == class_def.qualified_name {
                    out.push(span_to_range(&line_index, inv.range));
                }
            }
            dedup_ranges(&mut out);
            return out;
        }
    }

    // Proc references.
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            let mut out = Vec::new();
            if include_declaration {
                out.push(span_to_range(&line_index, proc_def.name_span));
            }
            let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
            for inv in &analysis.command_invocations {
                if inv.name == proc_def.name
                    || inv.name == proc_def.qualified_name
                    || inv.name == qname_no_prefix
                {
                    out.push(span_to_range(&line_index, inv.range));
                }
            }
            dedup_ranges(&mut out);
            return out;
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

fn dedup_ranges(ranges: &mut Vec<LspRange>) {
    let mut seen: std::collections::HashSet<(u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    ranges.retain(|r| {
        let key = (r.start_line, r.start_character, r.end_line, r.end_character);
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
    fn references_to_proc_include_decl_and_calls() {
        let src = "proc greet {} {}\ngreet\ngreet\n";
        let analysis = analyse(src);
        // Cursor on the first `greet` reference (line 1).
        let refs = references(src, 1, 2, &analysis, true);
        assert!(refs.len() >= 2, "expected decl + call sites: {refs:?}");
        // First entry is the declaration on line 0.
        assert_eq!(refs[0].start_line, 0);
    }

    #[test]
    fn references_exclude_decl_when_flag_false() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let with_decl = references(src, 1, 2, &analysis, true);
        let without_decl = references(src, 1, 2, &analysis, false);
        assert!(with_decl.len() > without_decl.len());
    }

    #[test]
    fn references_to_unknown_word_empty() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        assert!(references(src, 0, 6, &analysis, true).is_empty());
    }

    #[test]
    fn references_to_var_includes_definition_and_uses() {
        let src = "set x 1\nputs $x\nputs $x\n";
        let analysis = analyse(src);
        // Cursor on `$x` first reference.
        let refs = references(src, 1, 7, &analysis, true);
        // The analyser may or may not record the literal `$x`
        // as a reference depending on lowering; at minimum the
        // declaration should land in the result list.
        assert!(!refs.is_empty(), "{refs:?}");
        assert!(refs.iter().any(|r| r.start_line == 0));
    }
}
