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
//! Class renames are also supported: when the cursor sits on a
//! `oo::class create ClassName` declaration name (or on any
//! `ClassName new` / `ClassName create instance` invocation
//! head), the provider rewrites the declaration's name span
//! and every invocation that targets the class.  The same
//! `is_safe_symbol_name` / `is_builtin_command_name` gates
//! apply.
//!
//! What is *still deferred* (planned as further
//! `S-rename-rich` sub-strips):
//!
//! * Method rename — the Python provider has a separate code
//!   path for method-name renames inside a class body.
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
    // Class?  Mirrors the proc walk but against `all_classes`.
    for (qname, class_def) in &analysis.all_classes {
        if class_def.name == word || qname == &word || qname == &format!("::{word}") {
            return Some(PrepareRename {
                range: span_to_range(&line_index, class_def.name_span),
                placeholder: class_def.name.clone(),
            });
        }
    }
    None
}

/// Compute rename text edits.
///
/// See module-level docs for the dispatch order (variable → proc → class).
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
        return rename_var(
            source,
            line,
            character,
            new_name,
            analysis,
            &line_index,
            &var_name,
        );
    }

    let Some((word, _start, _end)) = find_word_span_at_position(source, line, character) else {
        return Vec::new();
    };

    if let Some(edits) = rename_proc(&word, new_name, analysis, registry, &line_index) {
        return edits;
    }
    if let Some(edits) = rename_class(&word, new_name, analysis, registry, &line_index) {
        return edits;
    }
    Vec::new()
}

/// Variable-rename path — declaration span + every read site,
/// with brace-ref escaping (`$x` / `${x}` / `$ns::x` /
/// `${ns::x}` keep their leader characters and any namespace
/// qualifier).
fn rename_var(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
    var_name: &str,
) -> Vec<TextEdit> {
    let byte_offset = crate::definition::byte_offset_at(source, line, character);
    let Some(var_def) =
        crate::definition::lookup_var_in_scope_chain(&analysis.global_scope, byte_offset, var_name)
    else {
        return Vec::new();
    };
    let mut edits = Vec::with_capacity(1 + var_def.references.len());
    edits.push(TextEdit {
        range: span_to_range(line_index, var_def.definition_span),
        new_text: new_name.to_owned(),
    });
    for r in &var_def.references {
        // `S-rename-rich` brace-ref escaping — see
        // [`build_var_ref_replacement`].
        let replacement = build_var_ref_replacement(source, *r, new_name);
        edits.push(TextEdit {
            range: span_to_range(line_index, *r),
            new_text: replacement,
        });
    }
    edits
}

/// Proc-rename path — declaration name span + every matching
/// call site.  Namespace-aware: the declaration keeps its
/// prefix; call sites pick the rewrite that matches the form
/// the source wrote (qualified ↔ short).  `Some(edits)` when
/// a proc matched `word` (even if the edit set ended up empty
/// from a safety gate); `None` when no proc matched.
fn rename_proc(
    word: &str,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    let (qname, proc_def) = analysis.all_procs.iter().find(|(qname, p)| {
        p.name == word || qname.as_str() == word || qname.as_str() == format!("::{word}")
    })?;
    if let Some(registry) = registry {
        if is_builtin_command_name(new_name, registry) {
            return Some(Vec::new());
        }
    }
    let namespace_prefix = namespace_prefix_of(&proc_def.qualified_name);
    let (new_qualified, new_decl_text) = qualified_and_decl_text(namespace_prefix, new_name);
    let mut edits = vec![TextEdit {
        range: span_to_range(line_index, proc_def.name_span),
        new_text: new_decl_text,
    }];
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
    for inv in &analysis.command_invocations {
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
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(TextEdit {
            range: span_to_range(line_index, inv.range),
            new_text: replacement,
        });
    }
    dedup_edits(&mut edits);
    Some(edits)
}

/// Class-rename path — `oo::class create ClassName`
/// declaration plus every `ClassName new` / `ClassName create
/// instance` invocation head.  Same shape as [`rename_proc`]
/// minus the resolved-qualified-name matching (the analyser
/// doesn't populate that for class invocations today).
fn rename_class(
    word: &str,
    new_name: &str,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    line_index: &LineIndex,
) -> Option<Vec<TextEdit>> {
    let (qname, class_def) = analysis.all_classes.iter().find(|(qname, c)| {
        c.name == word || qname.as_str() == word || qname.as_str() == format!("::{word}")
    })?;
    if let Some(registry) = registry {
        if is_builtin_command_name(new_name, registry) {
            return Some(Vec::new());
        }
    }
    let qname_no_prefix = qname.strip_prefix("::").unwrap_or(qname.as_str());
    let namespace_prefix = namespace_prefix_of(&class_def.qualified_name);
    let (new_qualified, new_decl_text) = qualified_and_decl_text(namespace_prefix, new_name);
    let mut edits = vec![TextEdit {
        range: span_to_range(line_index, class_def.name_span),
        new_text: new_decl_text,
    }];
    for inv in &analysis.command_invocations {
        let matches = inv.name == class_def.name
            || inv.name == class_def.qualified_name
            || inv.name == qname_no_prefix;
        if !matches {
            continue;
        }
        // Skip an invocation whose range contains the class
        // declaration site (defence in depth — the analyser's
        // recording shape doesn't usually file this here).
        if inv.range.start() <= class_def.name_span.start()
            && class_def.name_span.end() <= inv.range.end()
        {
            continue;
        }
        let replacement =
            invocation_replacement(namespace_prefix, &new_qualified, new_name, &inv.name);
        edits.push(TextEdit {
            range: span_to_range(line_index, inv.range),
            new_text: replacement,
        });
    }
    dedup_edits(&mut edits);
    Some(edits)
}

/// Compute the qualified rewrite (`prefix::new`) and the
/// declaration-site rewrite (qualified when the original lived
/// in a namespace, short at the top level).  Shared by the
/// proc and class rename paths.
fn qualified_and_decl_text(namespace_prefix: &str, new_name: &str) -> (String, String) {
    let new_qualified = if namespace_prefix.is_empty() {
        format!("::{new_name}")
    } else {
        format!("{namespace_prefix}::{new_name}")
    };
    let new_decl_text = if namespace_prefix.is_empty() {
        new_name.to_owned()
    } else {
        new_qualified.clone()
    };
    (new_qualified, new_decl_text)
}

/// Pick the rewrite shape for a single call/invocation site —
/// short form at the top level, qualified form when the call
/// itself was qualified.  Shared by the proc and class rename
/// paths.
fn invocation_replacement(
    namespace_prefix: &str,
    new_qualified: &str,
    new_name: &str,
    inv_name: &str,
) -> String {
    if namespace_prefix.is_empty() {
        new_name.to_owned()
    } else if inv_name.contains("::") {
        new_qualified.to_owned()
    } else {
        new_name.to_owned()
    }
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

    // -- S-rename-rich: class rename --------------------------------

    #[test]
    fn rename_class_at_decl_rewrites_decl_and_calls() {
        // `oo::class create MyClass { ... }` plus a `MyClass new`
        // invocation — both should be rewritten.
        let src = "oo::class create MyClass {\n\
                       method greet {} {}\n\
                   }\n\
                   MyClass new\n";
        let analysis = analyse(src);
        // Cursor on the `MyClass` declaration name (column 17).
        let edits = rename(src, 0, 17, "Renamed", &analysis, None);
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(!edits.is_empty(), "expected non-empty edits");
        assert!(
            texts.iter().all(|t| *t == "Renamed"),
            "every replacement should be `Renamed`; got {texts:?}",
        );
        // The decl site + the `MyClass new` call site = 2 edits
        // (more if the analyser surfaces inheritance refs).
        assert!(edits.len() >= 2, "{edits:?}");
    }

    #[test]
    fn rename_class_at_call_site_also_works() {
        // Cursor on the `MyClass` head of an invocation — same
        // outcome as renaming from the declaration.
        let src = "oo::class create MyClass {\n}\nMyClass new\n";
        let analysis = analyse(src);
        // Cursor on the `MyClass` in `MyClass new` (line 2, col 3).
        let edits = rename(src, 2, 3, "Renamed", &analysis, None);
        assert!(!edits.is_empty(), "{edits:?}");
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert!(
            texts.iter().all(|t| *t == "Renamed"),
            "expected every edit to be `Renamed`; got {texts:?}",
        );
    }

    #[test]
    fn rename_class_rejects_unsafe_new_name() {
        let src = "oo::class create MyClass {}\nMyClass new\n";
        let analysis = analyse(src);
        let edits = rename(src, 0, 17, "1bad", &analysis, None);
        assert!(edits.is_empty(), "{edits:?}");
    }

    #[test]
    fn rename_class_blocked_when_shadowing_builtin() {
        let src = "oo::class create MyClass {}\nMyClass new\n";
        let analysis = analyse(src);
        let mut r = tcl_registry::CommandRegistry::build_default();
        r.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        let edits = rename(src, 0, 17, "if", &analysis, Some(&r));
        assert!(
            edits.is_empty(),
            "renaming to a built-in should be blocked; got {edits:?}",
        );
    }

    #[test]
    fn prepare_rename_returns_range_for_class() {
        let src = "oo::class create MyClass {}\n";
        let analysis = analyse(src);
        let p = prepare_rename(src, 0, 17, &analysis).expect("expected prepare_rename on class");
        assert_eq!(p.placeholder, "MyClass");
        // Anchored at the class name span.
        assert_eq!(p.range.start_line, 0);
        // The span sits at column 17.
        assert_eq!(p.range.start_character, 17);
    }
}
