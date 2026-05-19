//! Rename provider — Rust port of `lsp/features/rename.py`.
//!
//! Computes a workspace edit that renames the symbol at the
//! cursor across the current document.  Ports:
//!
//! * `$var` references → rewrite the `VarDef.definition_span`
//!   and every `VarDef.references` span to the new name.
//! * proc references → rewrite `ProcDef.name_span` and every
//!   matching command-invocation head to the new name.
//!
//! With **`S-rename-rich` safety gating** (this commit): the
//! caller may pass a [`tcl_registry::CommandRegistry`].  When
//! provided, the new name is validated against:
//!
//! * `is_safe_symbol_name(name)` — must match
//!   `^[A-Za-z_][A-Za-z0-9_]*$` (Python's `_SAFE_SYMBOL_RE`).
//!   Applies to every rename target.
//! * `is_builtin_command_name(name, registry)` — proc renames
//!   refuse to overwrite a built-in command name.  Mirrors
//!   Python's `_is_builtin_command_name` check (lines 36-37
//!   and the proc-rename gate at line 301).
//!
//! When the new name fails either check, [`rename`] returns an
//! empty `Vec<TextEdit>` so the editor refuses the rename
//! rather than producing a partial edit set.
//!
//! What is *still deferred* (planned as further
//! `S-rename-rich` sub-strips):
//!
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
use tcl_registry::CommandRegistry;

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

/// `true` when `name` matches the safe-symbol shape
/// `^[A-Za-z_][A-Za-z0-9_]*$`.  Mirrors Python's
/// `_SAFE_SYMBOL_RE` / `_is_safe_symbol_name`.
///
/// Used by [`rename`] to reject new names that contain
/// whitespace, punctuation, leading digits, or other characters
/// that would produce invalid Tcl symbol references.
#[must_use]
pub fn is_safe_symbol_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `true` when `name` is registered as a built-in command in
/// `registry`.  Mirrors Python's `_is_builtin_command_name`.
///
/// Used by [`rename`] to refuse proc renames that would
/// overwrite a built-in command name (the editor would then
/// silently dispatch the user's calls to the built-in instead
/// of the renamed proc).
#[must_use]
pub fn is_builtin_command_name(name: &str, registry: &CommandRegistry) -> bool {
    registry.command_names().any(|n| n == name)
}

/// Compute the text edits for a rename of the symbol at the
/// cursor.
///
/// Returns an empty vector when no recognisable symbol is at
/// the position, the new name fails the safety gate, or a
/// proc rename would shadow a built-in command.  The caller
/// (server) is responsible for wrapping the output in a
/// `WorkspaceEdit { changes: { uri: edits } }`.
///
/// `registry`, when `Some`, enables `S-rename-rich` safety
/// gating:
///
/// * Every rename target rejects new names that don't match
///   `is_safe_symbol_name` — invalid syntax is refused.
/// * Proc renames additionally refuse names registered as
///   built-in commands via [`is_builtin_command_name`].
///
/// When `registry` is `None`, the safety gate degrades to the
/// shape-only check (`is_safe_symbol_name`) — built-in shadow
/// detection requires a registry.
#[must_use]
pub fn rename(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Vec<TextEdit> {
    // Shape gate first — applies to every rename target.
    if !is_safe_symbol_name(new_name) {
        return Vec::new();
    }
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
            // Built-in shadow gate — proc renames only.  Skipped
            // when no registry is provided (the minimal port's
            // behaviour).
            if let Some(registry) = registry {
                if is_builtin_command_name(new_name, registry) {
                    return Vec::new();
                }
            }
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
        let edits = rename(src, 0, 6, "hi", &analysis, None);
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
        assert!(rename(src, 0, 6, "x", &analysis, None).is_empty());
    }

    #[test]
    fn rename_var_includes_decl_span() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        // Cursor inside `$x`.
        let edits = rename(src, 1, 7, "y", &analysis, None);
        assert!(!edits.is_empty());
        assert!(edits.iter().all(|e| e.new_text == "y"));
    }

    // -- S-rename-rich: safety gating --------------------------------

    #[test]
    fn is_safe_symbol_name_accepts_canonical_identifiers() {
        assert!(is_safe_symbol_name("foo"));
        assert!(is_safe_symbol_name("Foo"));
        assert!(is_safe_symbol_name("_underscore"));
        assert!(is_safe_symbol_name("a1"));
        assert!(is_safe_symbol_name("snake_case_42"));
    }

    #[test]
    fn is_safe_symbol_name_rejects_invalid_shapes() {
        assert!(!is_safe_symbol_name(""), "empty name should be invalid");
        assert!(!is_safe_symbol_name("1leading_digit"));
        assert!(!is_safe_symbol_name("has space"));
        assert!(!is_safe_symbol_name("has-dash"));
        assert!(!is_safe_symbol_name("has::colon"));
        assert!(!is_safe_symbol_name("with$dollar"));
        // Tcl-valid but rename-rejects (forces editor to refuse
        // partially-edited symbols).
        assert!(!is_safe_symbol_name("dotted.name"));
    }

    #[test]
    fn rename_returns_empty_for_unsafe_new_name() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        // Whitespace in the new name fails the shape gate.
        assert!(rename(src, 0, 6, "bad name", &analysis, None).is_empty());
        // Leading digit fails.
        assert!(rename(src, 0, 6, "1lead", &analysis, None).is_empty());
        // Dash fails.
        assert!(rename(src, 0, 6, "with-dash", &analysis, None).is_empty());
    }

    #[test]
    fn rename_var_also_rejects_unsafe_new_name() {
        // The shape gate applies to variable renames too.
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        assert!(rename(src, 1, 7, "bad name", &analysis, None).is_empty());
    }

    #[test]
    fn rename_proc_to_builtin_command_name_blocked() {
        // Renaming `greet` to `puts` would shadow the built-in
        // `puts` command — the safety gate must reject the
        // rename when a registry is provided.
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let edits = rename(src, 0, 6, "puts", &analysis, Some(&registry));
        assert!(
            edits.is_empty(),
            "rename to built-in `puts` must produce no edits, got {edits:?}",
        );
    }

    #[test]
    fn rename_proc_to_non_builtin_succeeds_with_registry() {
        // Renaming `greet` to a non-built-in name should still
        // succeed when the registry is provided.
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let edits = rename(src, 0, 6, "salut", &analysis, Some(&registry));
        assert!(
            !edits.is_empty(),
            "rename to non-built-in `salut` should produce edits",
        );
        assert!(edits.iter().all(|e| e.new_text == "salut"));
    }

    #[test]
    fn rename_var_to_builtin_name_allowed() {
        // The built-in-shadow gate is proc-only.  Variable
        // names live in a separate namespace and can use any
        // shape-valid identifier — including `puts`.
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        let registry = CommandRegistry::build_default();
        let edits = rename(src, 1, 7, "puts", &analysis, Some(&registry));
        assert!(
            !edits.is_empty(),
            "variable rename to `puts` should succeed (different namespace)",
        );
    }
}
