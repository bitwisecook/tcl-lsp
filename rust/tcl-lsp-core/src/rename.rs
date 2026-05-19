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
/// Result of [`prepare_rename`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareRename {
    /// Range of the symbol the rename will affect.
    pub range: LspRange,
    /// Suggested placeholder (the symbol's current tail name)
    /// the editor pre-fills in its rename input box.
    pub placeholder: String,
}

/// Validate that the cursor sits on a renameable symbol and
/// return the symbol's range + placeholder text.  Editors
/// call this before `rename` to determine whether to show the
/// rename UI.  Mirrors `prepare_rename` in
/// `lsp/features/rename.py`.
#[must_use]
pub fn prepare_rename(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
) -> Option<PrepareRename> {
    let line_index = LineIndex::new(source);
    // Variable?
    if let Some(var_name) = find_var_at_position(source, line, character) {
        let byte_offset = crate::definition::byte_offset_at(source, line, character);
        if let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            byte_offset,
            &var_name,
        ) {
            return Some(PrepareRename {
                range: span_to_range(&line_index, var_def.definition_span),
                placeholder: var_def.name.clone(),
            });
        }
    }
    // Proc?
    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
            return Some(PrepareRename {
                range: span_to_range(&line_index, proc_def.name_span),
                placeholder: proc_def.name.clone(),
            });
        }
    }
    None
}

/// Compute rename text edits.
///
/// See module-level docs for the dispatch order (variable → proc).
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
        let byte_offset = crate::definition::byte_offset_at(source, line, character);
        let Some(var_def) = crate::definition::lookup_var_in_scope_chain(
            &analysis.global_scope,
            byte_offset,
            &var_name,
        ) else {
            return Vec::new();
        };
        let mut edits = Vec::with_capacity(1 + var_def.references.len());
        edits.push(TextEdit {
            range: span_to_range(&line_index, var_def.definition_span),
            new_text: new_name.to_owned(),
        });
        for r in &var_def.references {
            // `S-rename-rich` brace-ref escaping: a reference
            // span covers the full Var token — for `$x` it
            // includes the `$`; for `${name}` it spans
            // `${name}`.  Read the source text at the span to
            // decide which prefix / suffix shape to keep, then
            // emit a replacement that preserves the leader
            // characters (so `${x}` stays braced after the
            // rename).  Namespace qualifiers (`$ns::var`,
            // `${ns::var}`) keep their prefix.
            let replacement = build_var_ref_replacement(source, *r, new_name);
            edits.push(TextEdit {
                range: span_to_range(&line_index, *r),
                new_text: replacement,
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
            // Namespace-aware rewrite shape: a proc declared at
            // `::myns::greet` keeps its namespace prefix when
            // renamed.  The declaration's `name_span` covers the
            // full qualified token, so the replacement is the
            // new qualified form (prefix retained).  Call sites
            // each use the form the source wrote (qualified ↔
            // short), and we pick the matching replacement.
            let namespace_prefix = namespace_prefix_of(&proc_def.qualified_name);
            let new_qualified = if namespace_prefix.is_empty() {
                format!("::{new_name}")
            } else {
                format!("{namespace_prefix}::{new_name}")
            };
            // The declaration's name span uses the form the
            // user wrote — if it was qualified, keep the
            // qualified form; if short, the short form.  We
            // detect by looking at how the qualified name maps
            // to the spec: if `proc.qualified_name == "::greet"`
            // and `proc.name == "greet"`, the declaration could
            // have been either shape.  Use the qualified form
            // when a namespace prefix is non-empty
            // (`::myns::greet`); use the short form when the
            // proc lives at the top level (`::greet`).
            let new_decl_text = if namespace_prefix.is_empty() {
                new_name.to_owned()
            } else {
                new_qualified.clone()
            };
            let mut edits = Vec::new();
            edits.push(TextEdit {
                range: span_to_range(&line_index, proc_def.name_span),
                new_text: new_decl_text,
            });
            let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
            for inv in &analysis.command_invocations {
                // Decide whether this invocation targets the
                // proc, and what shape the source text used.
                let matches = inv.name == proc_def.name
                    || inv.name == proc_def.qualified_name
                    || inv.name == qname_no_prefix
                    || inv
                        .resolved_qualified_name
                        .as_deref()
                        .is_some_and(|r| r == proc_def.qualified_name);
                if !matches {
                    continue;
                }
                // For top-level procs the qualified form equals
                // the short form for practical purposes, so use
                // the short rewrite everywhere.  For namespaced
                // procs, pick the matching rewrite based on the
                // shape of the call site's text.
                let replacement = if namespace_prefix.is_empty() {
                    new_name.to_owned()
                } else if inv.name.contains("::") {
                    new_qualified.clone()
                } else {
                    new_name.to_owned()
                };
                edits.push(TextEdit {
                    range: span_to_range(&line_index, inv.range),
                    new_text: replacement,
                });
            }
            dedup_edits(&mut edits);
            return edits;
        }
    }

    Vec::new()
}

/// Build a replacement string for a variable reference span.
///
/// The reference span covers the full Var token (`$x`,
/// `${name}`, `$ns::var`, `${ns::var}`).  We read the source
/// bytes at the span to decide which leader characters (`$`,
/// `${`, `}`) to preserve, then splice in `new_tail` in place
/// of the existing tail name.
///
/// For `${name}` and `$name`, we strip the prefix to find the
/// inner text, find its namespace prefix (`ns::`), and emit
/// `${ns::new_tail}` / `$ns::new_tail` so namespace-qualified
/// refs keep their qualification.
fn build_var_ref_replacement(source: &str, span: tcl_lexer::Span, new_tail: &str) -> String {
    let start = span.start() as usize;
    let end = span.end() as usize;
    let bytes = source.as_bytes();
    if start >= bytes.len() || end > bytes.len() {
        return new_tail.to_owned();
    }
    let text = &source[start..end];
    if let Some(rest) = text.strip_prefix("${") {
        let inner = rest.strip_suffix('}').unwrap_or(rest);
        let ns_prefix = match inner.rfind("::") {
            Some(idx) => &inner[..idx + 2],
            None => "",
        };
        return format!("${{{ns_prefix}{new_tail}}}");
    }
    if let Some(rest) = text.strip_prefix('$') {
        let ns_prefix = match rest.rfind("::") {
            Some(idx) => &rest[..idx + 2],
            None => "",
        };
        return format!("${ns_prefix}{new_tail}");
    }
    let ns_prefix = match text.rfind("::") {
        Some(idx) => &text[..idx + 2],
        None => "",
    };
    format!("{ns_prefix}{new_tail}")
}

/// Return the namespace prefix of a qualified name — everything
/// before the final `::`.  `"::myns::greet"` → `"::myns"`;
/// `"::greet"` → `""` (proc lives at global scope, no enclosing
/// namespace).  `"greet"` → `""` likewise.
fn namespace_prefix_of(qualified: &str) -> &str {
    let trimmed = qualified.trim_start_matches("::");
    match trimmed.rfind("::") {
        Some(idx) => &qualified[..qualified.len() - (trimmed.len() - idx)],
        None => "",
    }
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
        // Declaration replaces just `x` → `y`; reference
        // replaces `$x` → `$y` so the `$` prefix is preserved.
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains(&"y"), "{texts:?}");
        assert!(texts.contains(&"$y"), "{texts:?}");
    }

    // -- S-rename-rich: brace-ref escaping ---------------------------

    #[test]
    fn rename_var_preserves_braced_reference_form() {
        let src = "set x 1\nputs ${x}\n";
        let analysis = analyse(src);
        // Cursor inside `${x}` on the `x`.
        let edits = rename(src, 1, 7, "y", &analysis, None);
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(texts.contains(&"y"), "{texts:?}");
        assert!(texts.contains(&"${y}"), "{texts:?}");
    }

    #[test]
    fn prepare_rename_returns_range_for_proc() {
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 0, 6, &analysis).expect("expected prepare_rename on proc name");
        assert_eq!(p.placeholder, "greet");
        // Anchored at the proc name span.
        assert_eq!(p.range.start_line, 0);
        assert_eq!(p.range.start_character, 5);
    }

    #[test]
    fn prepare_rename_returns_range_for_var() {
        let src = "set x 1\nputs $x\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 1, 7, &analysis).expect("expected prepare_rename on var");
        assert_eq!(p.placeholder, "x");
    }

    #[test]
    fn prepare_rename_returns_none_for_unknown_word() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        // `hello` isn't a proc or var declaration.
        assert!(prepare_rename(src, 0, 6, &analysis).is_none());
    }

    #[test]
    fn build_var_ref_replacement_handles_all_forms() {
        // Pure helper exercise — bare `$x`, braced `${x}`, and
        // qualified forms all produce the right replacement text.
        // Namespace-qualified refs preserve the prefix so e.g.
        // `$ns::z` → `$ns::c` even if the analyser doesn't yet
        // surface namespace-scoped variable lookups (the helper
        // is the building block; the lookup side lands in a
        // follow-up sub-strip).
        let src = "  $x  ${y}  $ns::z  ${ns::w}  ";
        let span_x = tcl_lexer::Span::new(2, 4);
        let span_braced_y = tcl_lexer::Span::new(6, 10);
        let span_qualified_z = tcl_lexer::Span::new(12, 18);
        let span_braced_w = tcl_lexer::Span::new(20, 28);
        assert_eq!(build_var_ref_replacement(src, span_x, "a"), "$a");
        assert_eq!(build_var_ref_replacement(src, span_braced_y, "b"), "${b}");
        assert_eq!(
            build_var_ref_replacement(src, span_qualified_z, "c"),
            "$ns::c"
        );
        assert_eq!(
            build_var_ref_replacement(src, span_braced_w, "d"),
            "${ns::d}"
        );
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

    // -- S-rename-rich: namespace-aware proc renames ----------------

    #[test]
    fn namespace_prefix_of_qualified_names() {
        assert_eq!(namespace_prefix_of("::myns::greet"), "::myns");
        assert_eq!(namespace_prefix_of("::a::b::c"), "::a::b");
        assert_eq!(namespace_prefix_of("::greet"), "");
        assert_eq!(namespace_prefix_of("greet"), "");
    }

    #[test]
    fn rename_namespaced_proc_keeps_namespace_prefix_at_decl() {
        // `proc ::myns::greet {} {}` renamed to `hello` should
        // rewrite the declaration's name span to
        // `::myns::hello` (keeping the namespace prefix), not
        // just `hello` (which would clobber the prefix).
        let src = "proc ::myns::greet {} {}\n";
        let analysis = analyse(src);
        let edits = rename(src, 0, 14, "hello", &analysis, None);
        assert!(!edits.is_empty(), "{edits:?}");
        assert!(
            edits.iter().any(|e| e.new_text == "::myns::hello"),
            "expected qualified replacement at decl, got {:?}",
            edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn rename_namespaced_proc_rewrites_call_sites_appropriately() {
        // Source has both a qualified call (`::myns::greet`)
        // and a short call (`greet` inside a `namespace eval`
        // block).  The qualified call gets the qualified
        // replacement; the short call gets the short
        // replacement.
        let src = "proc ::myns::greet {} {}\n\
                   ::myns::greet\n\
                   namespace eval ::myns {\n\
                       greet\n\
                   }\n";
        let analysis = analyse(src);
        let edits = rename(src, 0, 14, "hello", &analysis, None);
        let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        // Should include both `::myns::hello` (qualified) and
        // `hello` (short).
        assert!(
            replacements.contains(&"::myns::hello"),
            "expected `::myns::hello` somewhere; got {replacements:?}",
        );
        assert!(
            replacements.contains(&"hello"),
            "expected `hello` somewhere; got {replacements:?}",
        );
    }

    #[test]
    fn rename_top_level_proc_uses_short_form_at_decl() {
        // For a top-level proc (`proc greet {} {}` →
        // `proc.qualified_name == "::greet"`, no enclosing
        // namespace prefix), the declaration rewrite stays
        // unqualified (`hello`, not `::hello`).
        let src = "proc greet {} {}\ngreet\n";
        let analysis = analyse(src);
        let edits = rename(src, 0, 6, "hello", &analysis, None);
        assert!(
            edits.iter().any(|e| e.new_text == "hello"),
            "expected short `hello` at decl; got {:?}",
            edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
        );
        // No qualified `::hello` rewrites either — the source
        // never uses a qualified call.
        assert!(
            edits.iter().all(|e| e.new_text == "hello"),
            "expected every edit to be `hello`; got {:?}",
            edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
        );
    }
}
