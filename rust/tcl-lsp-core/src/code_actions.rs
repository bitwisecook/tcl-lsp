//! Code-actions provider — Rust port of
//! `lsp/features/code_actions.py`.
//!
//! Surfaces every `CodeFix` the analyser attached to a
//! `Diagnostic` whose span overlaps the requested range.  Each
//! fix lifts to one `CodeAction` with the fix's `description`
//! as the title and a single-edit `WorkspaceEdit` carrying
//! the fix's `(span, new_text)`.
//!
//! What landed:
//!
//! * Catch-result-variable actions — when the analyser emits
//!   W302 (`catch` without result variable), the provider
//!   offers two quick-fixes that splice a trailing ` result`
//!   or ` result opts` after the body's closing brace.
//! * `unset -nocomplain` action — when the analyser emits
//!   W213 (unset on possibly-undefined variable), the provider
//!   offers an `Add '-nocomplain' to unset` quick-fix that
//!   splices the flag right after the `unset` keyword.
//!
//! What is *deferred*:
//!
//! * Package-suggestion actions (`Add 'package require ...'`)
//!   — Python's provider walks the catalogue of stdlib /
//!   tcllib commands and offers the missing `package require`
//!   when an unresolved call matches.  Needs a stub-aware
//!   catalogue lookup (lands alongside
//!   `S-package-suggestions-rich`).
//! * Cross-document refactors (move to file, split namespace)
//!   — lands alongside the workspace-index integration.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

/// One code-action entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    /// Title shown in the editor.
    pub title: String,
    /// Edits the action would apply.
    pub edits: Vec<crate::rename::TextEdit>,
}

/// Compute code actions for `range` in `source`.
///
/// `analysis`, when `Some`, is the analyser result the caller
/// already computed.  When `None`, returns an empty vector
/// (preserves the stub call shape for callers that haven't
/// yet plumbed analysis through).
#[must_use]
pub fn code_actions(
    source: &str,
    range: LspRange,
    analysis: Option<&AnalysisResult>,
) -> Vec<CodeAction> {
    let Some(analysis) = analysis else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut actions = Vec::new();

    for diag in &analysis.diagnostics {
        let diag_start = line_index.position_at(diag.span.start());
        let diag_end = line_index.position_at(diag.span.end());
        let diag_range = LspRange {
            start_line: diag_start.line,
            start_character: diag_start.character,
            end_line: diag_end.line,
            end_character: diag_end.character,
        };
        if !ranges_overlap(diag_range, range) {
            continue;
        }
        // `S-code-actions-rich`: surface synthetic
        // catch-result-variable actions for W302 diagnostics
        // even when the analyser didn't attach a `CodeFix`.
        // Two actions: append ` result` (capture the result)
        // or ` result opts` (capture result + options).  The
        // diagnostic's span end sits past the body's closing
        // `}`, so the insertion point is exactly the diag-end
        // position.
        if diag.code == "W302" {
            let insertion = LspRange {
                start_line: diag_end.line,
                start_character: diag_end.character,
                end_line: diag_end.line,
                end_character: diag_end.character,
            };
            for (title, suffix) in [
                ("Add catch result variable", " result"),
                ("Add catch result + options variables", " result opts"),
            ] {
                actions.push(CodeAction {
                    title: title.to_string(),
                    edits: vec![crate::rename::TextEdit {
                        range: insertion,
                        new_text: suffix.to_string(),
                    }],
                });
            }
        }
        // `S-code-actions-rich`: synthetic W213 quick-fix.
        // W213 fires on `unset $var` when the variable may not
        // exist; the canonical Tcl idiom is `unset -nocomplain
        // $var`, so offer that as a one-click fix.  The diag
        // span starts at `unset`; we splice ` -nocomplain`
        // immediately after the keyword (offset +5).
        if diag.code == "W213" {
            if let Some(action) = build_unset_nocomplain_action(source, diag, &line_index) {
                actions.push(action);
            }
        }
        for fix in &diag.fixes {
            let fix_start = line_index.position_at(fix.span.start());
            let fix_end = line_index.position_at(fix.span.end());
            let title = if fix.description.is_empty() {
                // Fall back to the diagnostic's message
                // (truncated) when the fix didn't carry a
                // description.
                let trimmed: String = diag.message.chars().take(60).collect();
                format!("Fix: {trimmed}")
            } else {
                fix.description.clone()
            };
            actions.push(CodeAction {
                title,
                edits: vec![crate::rename::TextEdit {
                    range: LspRange {
                        start_line: fix_start.line,
                        start_character: fix_start.character,
                        end_line: fix_end.line,
                        end_character: fix_end.character,
                    },
                    new_text: fix.new_text.clone(),
                }],
            });
        }
    }

    actions
}

/// Build the `Add '-nocomplain'` quick-fix for a W213
/// diagnostic.  Validates that the diag span starts with the
/// `unset` keyword (defends against the diag shape changing
/// in future analyser revisions) before emitting an
/// insertion edit at offset +5 of the span start.
fn build_unset_nocomplain_action(
    source: &str,
    diag: &tcl_compiler::analyser::Diagnostic,
    line_index: &LineIndex,
) -> Option<CodeAction> {
    let start = diag.span.start() as usize;
    if !source.get(start..start + 5).is_some_and(|s| s == "unset") {
        return None;
    }
    let insert_offset = diag.span.start().checked_add(5)?;
    let pos = line_index.position_at(insert_offset);
    let insertion = LspRange {
        start_line: pos.line,
        start_character: pos.character,
        end_line: pos.line,
        end_character: pos.character,
    };
    Some(CodeAction {
        title: "Add '-nocomplain' to unset".to_string(),
        edits: vec![crate::rename::TextEdit {
            range: insertion,
            new_text: " -nocomplain".to_string(),
        }],
    })
}

/// `true` when `a` and `b` overlap (touch, intersect, or are
/// identical).  Mirrors VS Code's range-context filter for
/// code actions.
fn ranges_overlap(a: LspRange, b: LspRange) -> bool {
    // Convert each range to a (start, end) tuple of
    // (line, character) for ordering.
    let a_start = (a.start_line, a.start_character);
    let a_end = (a.end_line, a.end_character);
    let b_start = (b.start_line, b.start_character);
    let b_end = (b.end_line, b.end_character);
    a_start <= b_end && b_start <= a_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::{Analyser, AnalysisResult, CodeFix, Diagnostic};
    use tcl_lexer::Span;

    fn whole_document_range(source: &str) -> LspRange {
        let line_count = source.lines().count().max(1);
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: u32::try_from(line_count - 1).unwrap_or(0),
            end_character: u32::MAX,
        }
    }

    #[test]
    fn empty_actions_when_analysis_is_none() {
        assert!(code_actions("set x 1\n", whole_document_range("set x 1\n"), None).is_empty());
    }

    #[test]
    fn fix_attached_to_diagnostic_surfaces_as_action() {
        // Build a synthetic AnalysisResult with one diagnostic
        // and one fix.  Verifies the lift logic in isolation
        // from the analyser's diagnostic emitters.
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: "W210".to_string(),
            message: "Variable read before set".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![CodeFix {
                span: Span::new(0, 5),
                new_text: "set var 0".to_string(),
                description: "Initialise `var`".to_string(),
            }],
        });
        let actions = code_actions("set x 1\n", whole_document_range("set x 1\n"), Some(&r));
        assert_eq!(actions.len(), 1, "{actions:?}");
        assert_eq!(actions[0].title, "Initialise `var`");
        assert_eq!(actions[0].edits.len(), 1);
        assert_eq!(actions[0].edits[0].new_text, "set var 0");
    }

    #[test]
    fn no_action_when_range_outside_diagnostic() {
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: "W210".to_string(),
            message: "msg".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![CodeFix {
                span: Span::new(0, 5),
                new_text: "fix".to_string(),
                description: "Fix".to_string(),
            }],
        });
        // Request range on line 99 — far away from the
        // diagnostic's line 0.
        let far_range = LspRange {
            start_line: 99,
            start_character: 0,
            end_line: 99,
            end_character: 10,
        };
        assert!(code_actions("set x 1\n", far_range, Some(&r)).is_empty());
    }

    #[test]
    fn empty_description_falls_back_to_diagnostic_message() {
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: "W210".to_string(),
            message: "Variable read before set".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![CodeFix {
                span: Span::new(0, 5),
                new_text: "x".to_string(),
                description: String::new(), // No description.
            }],
        });
        let actions = code_actions("set x 1\n", whole_document_range("set x 1\n"), Some(&r));
        assert_eq!(actions.len(), 1);
        assert!(actions[0].title.contains("Variable read before set"));
    }

    #[test]
    fn no_actions_when_analyser_has_no_diagnostics_with_fixes() {
        // Run the actual analyser; with a clean source no
        // fixable diagnostics fire and the result is empty.
        let mut a = Analyser::new();
        let analysis = a.analyse("set x 1\nputs $x\n", "tcl8.6").clone();
        let actions = code_actions(
            "set x 1\nputs $x\n",
            whole_document_range("set x 1\nputs $x\n"),
            Some(&analysis),
        );
        assert!(actions.is_empty(), "{actions:?}");
    }

    #[test]
    fn multiple_fixes_on_one_diagnostic_each_become_an_action() {
        let mut r = AnalysisResult::default();
        r.diagnostics.push(Diagnostic {
            code: "Wxxx".to_string(),
            message: "msg".to_string(),
            severity: tcl_compiler::analyser::Severity::Warning,
            span: Span::new(0, 5),
            fixes: vec![
                CodeFix {
                    span: Span::new(0, 5),
                    new_text: "a".into(),
                    description: "A".into(),
                },
                CodeFix {
                    span: Span::new(0, 5),
                    new_text: "b".into(),
                    description: "B".into(),
                },
            ],
        });
        let actions = code_actions("set x 1\n", whole_document_range("set x 1\n"), Some(&r));
        assert_eq!(actions.len(), 2);
        let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"A") && titles.contains(&"B"));
    }

    // -- S-code-actions-rich: W213 unset -nocomplain action ----------

    #[test]
    fn w213_emits_unset_nocomplain_action() {
        // Confirm the analyser emits W213 on `unset xs` inside a
        // proc where `xs` is possibly undefined, then verify the
        // provider surfaces the `-nocomplain` quick-fix.
        let src = "proc foo {} { unset xs }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        assert!(
            analysis.diagnostics.iter().any(|d| d.code == "W213"),
            "expected W213 from {:?}",
            analysis.diagnostics,
        );
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let nocomplain = actions
            .iter()
            .find(|a| a.title == "Add '-nocomplain' to unset");
        assert!(nocomplain.is_some(), "expected quick-fix in {actions:?}");
        let act = nocomplain.unwrap();
        assert_eq!(act.edits.len(), 1);
        assert_eq!(act.edits[0].new_text, " -nocomplain");
        // The edit is an insertion (zero-width range).
        assert_eq!(
            act.edits[0].range.start_character,
            act.edits[0].range.end_character,
        );
    }

    #[test]
    fn w213_action_inserts_after_unset_keyword() {
        // Verify the insertion point is exactly after the 5
        // chars of `unset` — splicing produces a syntactically
        // correct `unset -nocomplain xs` command.
        let src = "proc foo {} { unset xs }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let act = actions
            .iter()
            .find(|a| a.title == "Add '-nocomplain' to unset")
            .expect("expected unset action");
        // Apply the edit and check the result.
        let edit = &act.edits[0];
        let line0 = src.lines().nth(edit.range.start_line as usize).unwrap();
        let chars: Vec<char> = line0.chars().collect();
        let col = edit.range.start_character as usize;
        let before: String = chars[..col].iter().collect();
        let after: String = chars[col..].iter().collect();
        let spliced = format!("{before}{}{after}", edit.new_text);
        assert!(
            spliced.contains("unset -nocomplain xs"),
            "spliced line: {spliced}",
        );
    }

    // -- S-code-actions-rich: catch-result-variable actions ----------

    #[test]
    fn w302_emits_catch_result_variable_actions() {
        // The real analyser emits W302 for `catch {body}` with
        // no result variable.  The provider should surface two
        // synthetic actions appending ` result` / ` result opts`.
        let src = "catch { puts hi }\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Sanity-check the analyser actually emitted W302.
        assert!(
            analysis.diagnostics.iter().any(|d| d.code == "W302"),
            "expected W302 from {:?}",
            analysis.diagnostics,
        );
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
        assert!(titles.contains(&"Add catch result variable"), "{titles:?}",);
        assert!(
            titles.contains(&"Add catch result + options variables"),
            "{titles:?}",
        );
        // Verify the insertion text shapes.
        let result_act = actions
            .iter()
            .find(|a| a.title == "Add catch result variable")
            .unwrap();
        assert_eq!(result_act.edits[0].new_text, " result");
        let opts_act = actions
            .iter()
            .find(|a| a.title == "Add catch result + options variables")
            .unwrap();
        assert_eq!(opts_act.edits[0].new_text, " result opts");
        // Both insertions land at the same position (a zero-
        // width range immediately after the body's closing `}`).
        for act in [result_act, opts_act] {
            let r = act.edits[0].range;
            assert_eq!(r.start_line, r.end_line);
            assert_eq!(r.start_character, r.end_character);
        }
    }
}
