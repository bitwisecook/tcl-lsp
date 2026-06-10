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
//! * `Add 'package require <pkg>'` action — the analyser emits
//!   W120 (package-gated command without `package require`)
//!   carrying an insert `CodeFix`; the provider lifts it via
//!   the generic `diag.fixes` path below.
//!
//! * Package-*suggestion* actions ([`package_require_actions`]) —
//!   for the word at the cursor, fuzzy-rank known package names
//!   (the registry's `required_package` / `tcllib_package`
//!   catalogue) against the `::`-prefix and offer
//!   `Add 'package require <pkg>'` (skipping already-required
//!   packages).  Mirrors `_package_require_actions` +
//!   `rank_package_suggestions`.
//!
//! What is *deferred*:
//!
//! * The Python workspace `package_resolver`'s *installed*-package
//!   set — [`package_require_actions`] derives its catalogue from
//!   the registry instead, so locally-installed-but-unregistered
//!   packages aren't suggested.
//! * Cross-document refactors (move to file, split namespace)
//!   — lands alongside the workspace-index integration.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::{utf16_col_to_char_col, LspRange};

/// LSP code-action kind.  Maps to the dotted strings the editor / e2e
/// `only` filter use (`quickfix`, `refactor.extract`, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    /// `quickfix` — a diagnostic fix.
    QuickFix,
    /// `refactor.extract` — extract proc.
    RefactorExtract,
    /// `refactor.inline` — inline proc.
    RefactorInline,
    /// `refactor.rewrite` — expression rewrites (De Morgan, invert).
    RefactorRewrite,
    /// `refactor` — generic refactor (IP conversion).
    Refactor,
    /// `source` — source action (generate docstring).
    Source,
}

impl ActionKind {
    /// The dotted LSP kind string.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QuickFix => "quickfix",
            Self::RefactorExtract => "refactor.extract",
            Self::RefactorInline => "refactor.inline",
            Self::RefactorRewrite => "refactor.rewrite",
            Self::Refactor => "refactor",
            Self::Source => "source",
        }
    }
}

/// A command attached to a code action (e.g. the post-extract rename).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCommand {
    /// Command identifier (e.g. `tclLsp.renameSymbolAtPosition`).
    pub command: String,
    /// Integer arguments (line / start / end for the rename command).
    pub args: Vec<u32>,
}

/// One code-action entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    /// Title shown in the editor.
    pub title: String,
    /// Edits the action would apply.
    pub edits: Vec<crate::rename::TextEdit>,
    /// LSP kind (drives the editor's `only` filter).
    pub kind: ActionKind,
    /// Optional command run after the edit (e.g. trigger a rename).
    pub command: Option<ActionCommand>,
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
        let diag_start = line_index.position_at_utf16(diag.span.start(), source);
        let diag_end = line_index.position_at_utf16(diag.span.end(), source);
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
                    kind: ActionKind::QuickFix,
                    command: None,
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
            let fix_start = line_index.position_at_utf16(fix.span.start(), source);
            let fix_end = line_index.position_at_utf16(fix.span.end(), source);
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
                kind: ActionKind::QuickFix,
                command: None,
            });
        }
    }

    // Range-based refactors / source actions that don't depend on a diagnostic.
    actions.extend(continuation_comment_actions(
        source,
        range,
        analysis,
        &line_index,
    ));
    actions.extend(ip_conversion_actions(source, range, &line_index));
    actions.extend(expr_rewrite_actions(source, range, &line_index));
    actions.extend(docstring_actions(source, range, analysis, &line_index));
    actions.extend(extract_inline_actions(source, range, analysis, &line_index));

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
    let pos = line_index.position_at_utf16(insert_offset, source);
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
        kind: ActionKind::QuickFix,
        command: None,
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

/// Fuzzy `package require` suggestions for the word at `range`'s start:
/// when an unknown command's prefix (the part before `::`) fuzzy-matches
/// a known package name, offer `Add 'package require <pkg>'`.  Mirrors
/// `lsp/features/code_actions.py::_package_require_actions` +
/// `rank_package_suggestions`.  The package catalogue is derived from the
/// registry's `required_package` / `tcllib_package` fields (the Python
/// workspace `package_resolver`'s installed-package set isn't modelled in
/// the Rust core).
#[must_use]
pub fn package_require_actions(
    source: &str,
    range: LspRange,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<CodeAction> {
    let word = word_at_position(source, range.start_line, range.start_character);
    let prefix = word.split("::").next().unwrap_or("").to_lowercase();
    if prefix.len() < 2 {
        return Vec::new();
    }
    let catalogue = package_catalogue(registry);
    let ranked = rank_package_suggestions(&word, &catalogue, 5);
    let insert_line = package_insert_line(source);
    ranked
        .into_iter()
        .filter(|pkg| !already_required(source, pkg))
        .map(|pkg| CodeAction {
            title: format!("Add 'package require {pkg}'"),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: insert_line,
                    start_character: 0,
                    end_line: insert_line,
                    end_character: 0,
                },
                new_text: format!("package require {pkg}\n"),
            }],
            kind: ActionKind::QuickFix,
            command: None,
        })
        .collect()
}

/// Distinct package names known to the registry (`required_package` +
/// `tcllib_package` across all command specs).
fn package_catalogue(registry: &tcl_registry::CommandRegistry) -> Vec<String> {
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for name in registry.command_names() {
        if let Some(spec) = registry.get(name) {
            if let Some(pkg) = spec.required_package {
                set.insert(pkg.to_owned());
            }
            if let Some(pkg) = spec.tcllib_package {
                set.insert(pkg.to_owned());
            }
        }
    }
    set.into_iter().collect()
}

/// Rank package names against a symbol's prefix (exact / prefix /
/// substring), best first, capped at `limit`.  Mirrors
/// `lsp/features/package_suggestions.py::rank_package_suggestions`.
fn rank_package_suggestions(symbol: &str, packages: &[String], limit: usize) -> Vec<String> {
    let prefix = symbol
        .trim()
        .split("::")
        .next()
        .unwrap_or("")
        .to_lowercase();
    if prefix.len() < 2 {
        return Vec::new();
    }
    let mut ranked: Vec<(u8, &String)> = packages
        .iter()
        .filter_map(|pkg| {
            let lower = pkg.to_lowercase();
            let score = if lower == prefix {
                0
            } else if lower.starts_with(&prefix) {
                1
            } else if lower.contains(&prefix) {
                2
            } else {
                return None;
            };
            Some((score, pkg))
        })
        .collect();
    ranked.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    ranked
        .into_iter()
        .take(limit)
        .map(|(_, pkg)| pkg.clone())
        .collect()
}

/// Line at which to insert a new `package require` — after a leading
/// shebang and any contiguous top-of-file `package require` lines.
/// Mirrors `_package_insert_line`.
fn package_insert_line(source: &str) -> u32 {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut line = 0usize;
    if lines.first().is_some_and(|l| l.starts_with("#!")) {
        line = 1;
    }
    while line < lines.len() && {
        let t = lines[line].trim_start();
        t.starts_with("package require") || t.starts_with("package\trequire")
    } {
        line += 1;
    }
    u32::try_from(line).unwrap_or(0)
}

/// `true` when `source` already `package require`s `pkg`.
fn already_required(source: &str, pkg: &str) -> bool {
    source.split('\n').any(|line| {
        let t = line.trim_start();
        if let Some(rest) = t
            .strip_prefix("package require")
            .or_else(|| t.strip_prefix("package\trequire"))
        {
            let rest = rest.trim_start();
            rest.strip_prefix(pkg)
                .is_some_and(|after| after.is_empty() || after.starts_with(char::is_whitespace))
        } else {
            false
        }
    })
}

/// Extract the identifier word (including `::`) at `(line, character)`.
fn word_at_position(source: &str, line: u32, character: u32) -> String {
    let Some(line_text) = source.split('\n').nth(line as usize) else {
        return String::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == ':';
    let col = utf16_col_to_char_col(line_text, character).min(chars.len());
    let mut start = col;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_word(chars[end]) {
        end += 1;
    }
    chars[start..end].iter().collect()
}

// ---------------------------------------------------------------------------
// W115 — convert a backslash-continued comment to per-line comments.
// ---------------------------------------------------------------------------

fn continuation_comment_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    _line_index: &LineIndex,
) -> Vec<CodeAction> {
    // Only offer when a W115 diagnostic overlaps the range OR the start line is
    // a backslash-continued comment (the editor may drive this with a
    // fabricated W115 it computed client-side, so detect the shape directly).
    let lines: Vec<&str> = source.split('\n').collect();
    let start_line = range.start_line as usize;
    if start_line >= lines.len() {
        return Vec::new();
    }
    let first = lines[start_line];
    let trimmed = first.trim_start();
    if !trimmed.starts_with('#') || !first.trim_end().ends_with('\\') {
        // Not a continued comment at the range start — also bail if no W115.
        let has_w115 = analysis.diagnostics.iter().any(|d| d.code == "W115");
        if !has_w115 {
            return Vec::new();
        }
    }
    // Gather the continuation run starting at `start_line`.
    let mut idx = start_line;
    let mut block: Vec<String> = Vec::new();
    loop {
        if idx >= lines.len() {
            break;
        }
        let line = lines[idx];
        let stripped = line.trim_end();
        let continues = stripped.ends_with('\\');
        // Strip the trailing backslash; preserve leading indentation.
        let body = if continues {
            stripped[..stripped.len() - 1].trim_end()
        } else {
            line.trim_end()
        };
        let indent: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let content = body.trim_start();
        if content.starts_with('#') {
            block.push(format!("{indent}{content}"));
        } else if content.is_empty() {
            block.push(indent);
        } else {
            block.push(format!("{indent}# {content}"));
        }
        if !continues {
            break;
        }
        idx += 1;
    }
    if idx <= start_line {
        return Vec::new();
    }
    let new_text = block.join("\n");
    vec![CodeAction {
        title: "Convert to per-line comments".to_string(),
        edits: vec![crate::rename::TextEdit {
            range: LspRange {
                start_line: range.start_line,
                start_character: 0,
                end_line: u32::try_from(idx).unwrap_or(range.start_line),
                end_character: u32::try_from(lines[idx].chars().count()).unwrap_or(0),
            },
            new_text,
        }],
        kind: ActionKind::QuickFix,
        command: None,
    }]
}

// ---------------------------------------------------------------------------
// IPv4 ↔ IPv6-mapped conversion.
// ---------------------------------------------------------------------------

/// `true` when `s` is a dotted-quad IPv4 literal (each octet 0-255).
fn is_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 3 && p.parse::<u8>().is_ok())
}

fn ip_conversion_actions(
    source: &str,
    range: LspRange,
    _line_index: &LineIndex,
) -> Vec<CodeAction> {
    let Some(line_text) = source.split('\n').nth(range.start_line as usize) else {
        return Vec::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let col = utf16_col_to_char_col(line_text, range.start_character).min(chars.len());
    // IP-literal characters include hex, `.`, `:`, and `/` for the CIDR suffix.
    let is_ip_char = |c: char| c.is_ascii_hexdigit() || matches!(c, '.' | ':' | '/');
    let mut start = col;
    while start > 0 && is_ip_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ip_char(chars[end]) {
        end += 1;
    }
    if start >= end {
        return Vec::new();
    }
    let word: String = chars[start..end].iter().collect();
    let (addr, suffix) = match word.split_once('/') {
        Some((a, s)) => (a.to_string(), format!("/{s}")),
        None => (word.clone(), String::new()),
    };
    let edit_range = LspRange {
        start_line: range.start_line,
        start_character: char_col_to_utf16_local(line_text, start),
        end_line: range.start_line,
        end_character: char_col_to_utf16_local(line_text, end),
    };
    let make = |title: String, new_addr: String| CodeAction {
        title,
        edits: vec![crate::rename::TextEdit {
            range: edit_range,
            new_text: format!("{new_addr}{suffix}"),
        }],
        kind: ActionKind::Refactor,
        command: None,
    };
    if is_ipv4(&addr) {
        return vec![make(
            "Convert to IPv6-mapped address".to_string(),
            format!("::ffff:{addr}"),
        )];
    }
    if let Some(rest) = addr
        .strip_prefix("::ffff:")
        .or_else(|| addr.strip_prefix("::FFFF:"))
    {
        if is_ipv4(rest) {
            return vec![make(
                "Convert to IPv4 address".to_string(),
                rest.to_string(),
            )];
        }
    }
    Vec::new()
}

/// Codepoint column → UTF-16 column on `line_text`.
fn char_col_to_utf16_local(line_text: &str, char_col: usize) -> u32 {
    line_text
        .chars()
        .take(char_col)
        .map(|c| u32::try_from(c.len_utf16()).unwrap_or(1))
        .sum()
}

// ---------------------------------------------------------------------------
// Expression rewrites: De Morgan + invert comparison.
// ---------------------------------------------------------------------------

fn expr_rewrite_actions(source: &str, range: LspRange, _line_index: &LineIndex) -> Vec<CodeAction> {
    // Single-line, non-empty selection only.
    if range.start_line != range.end_line || range.start_character >= range.end_character {
        return Vec::new();
    }
    let Some(line_text) = source.split('\n').nth(range.start_line as usize) else {
        return Vec::new();
    };
    let chars: Vec<char> = line_text.chars().collect();
    let s = utf16_col_to_char_col(line_text, range.start_character).min(chars.len());
    let e = utf16_col_to_char_col(line_text, range.end_character).min(chars.len());
    if s >= e {
        return Vec::new();
    }
    let sel: String = chars[s..e].iter().collect();
    let mut out = Vec::new();
    let edit_range = LspRange {
        start_line: range.start_line,
        start_character: range.start_character,
        end_line: range.end_line,
        end_character: range.end_character,
    };
    if let Some(rewritten) = demorgan_transform(&sel) {
        out.push(CodeAction {
            title: "Apply De Morgan's law".to_string(),
            edits: vec![crate::rename::TextEdit {
                range: edit_range,
                new_text: rewritten,
            }],
            kind: ActionKind::RefactorRewrite,
            command: None,
        });
    }
    if let Some(rewritten) = invert_comparison(&sel) {
        out.push(CodeAction {
            title: "Invert comparison".to_string(),
            edits: vec![crate::rename::TextEdit {
                range: edit_range,
                new_text: rewritten,
            }],
            kind: ActionKind::RefactorRewrite,
            command: None,
        });
    }
    out
}

/// De Morgan: `!(X && Y)` ↔ `!X || !Y`, `!(X || Y)` ↔ `!X && !Y`.
fn demorgan_transform(sel: &str) -> Option<String> {
    let t = sel.trim();
    // Forward: `!( X <op> Y )`.
    if let Some(inner) = t.strip_prefix("!(").and_then(|s| s.strip_suffix(')')) {
        if let Some((l, r)) = split_top_logical(inner, "&&") {
            return Some(format!("{} || {}", negate(l.trim()), negate(r.trim())));
        }
        if let Some((l, r)) = split_top_logical(inner, "||") {
            return Some(format!("{} && {}", negate(l.trim()), negate(r.trim())));
        }
        return None;
    }
    // Reverse: `!X || !Y` → `!(X && Y)`, `!X && !Y` → `!(X || Y)`.
    if let Some((l, r)) = split_top_logical(t, "||") {
        if let (Some(li), Some(ri)) = (l.trim().strip_prefix('!'), r.trim().strip_prefix('!')) {
            return Some(format!("!({} && {})", li.trim(), ri.trim()));
        }
    }
    if let Some((l, r)) = split_top_logical(t, "&&") {
        if let (Some(li), Some(ri)) = (l.trim().strip_prefix('!'), r.trim().strip_prefix('!')) {
            return Some(format!("!({} || {})", li.trim(), ri.trim()));
        }
    }
    None
}

/// Negate an operand: `$a` → `!$a`, `!$a` → `$a`, `($a && $b)` → `!($a && $b)`.
fn negate(operand: &str) -> String {
    let o = operand.trim();
    if let Some(rest) = o.strip_prefix('!') {
        rest.trim().to_string()
    } else {
        format!("!{o}")
    }
}

/// Split `expr` on the top-level (brace/paren-depth 0) occurrence of `op`.
fn split_top_logical<'a>(expr: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let bytes = expr.as_bytes();
    let opb = op.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + opb.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &bytes[i..i + opb.len()] == opb {
            return Some((&expr[..i], &expr[i + opb.len()..]));
        }
        i += 1;
    }
    None
}

/// Invert the (single) top-level comparison operator in `sel`.
fn invert_comparison(sel: &str) -> Option<String> {
    // Operator inversions, longest-first so `==` wins over `=`.
    const OPS: &[(&str, &str)] = &[
        ("==", "!="),
        ("!=", "=="),
        ("<=", ">"),
        (">=", "<"),
        ("eq", "ne"),
        ("ne", "eq"),
        ("ni", "in"),
        ("in", "ni"),
        ("<", ">="),
        (">", "<="),
    ];
    let t = sel.trim();
    for (from, to) in OPS {
        // Require the operator to be surrounded by spaces so `$a == $b` matches
        // but a bare `<` inside a name doesn't; word ops need word boundaries.
        let needle = format!(" {from} ");
        if let Some(pos) = find_top_level(t, &needle) {
            let mut result = String::with_capacity(t.len());
            result.push_str(&t[..pos]);
            result.push(' ');
            result.push_str(to);
            result.push(' ');
            result.push_str(&t[pos + needle.len()..]);
            return Some(result);
        }
    }
    None
}

/// Find ` needle ` at brace/paren depth 0.
fn find_top_level(expr: &str, needle: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let nb = needle.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i + nb.len() <= bytes.len() {
        match bytes[i] {
            b'(' | b'{' | b'[' => depth += 1,
            b')' | b'}' | b']' => depth -= 1,
            _ => {}
        }
        if depth == 0 && &bytes[i..i + nb.len()] == nb {
            return Some(i);
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// Generate docstring (source action).
// ---------------------------------------------------------------------------

fn docstring_actions(
    source: &str,
    range: LspRange,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<CodeAction> {
    let mut out = Vec::new();
    for proc_def in analysis.all_procs.values() {
        let decl = line_index.position_at_utf16(proc_def.name_span.start(), source);
        if decl.line != range.start_line {
            continue;
        }
        // Skip procs that already carry a doc-comment.
        if !proc_def.doc.is_empty() {
            continue;
        }
        let mut doc = String::from("# \n");
        for p in &proc_def.params {
            doc.push_str("# @param ");
            doc.push_str(&p.name);
            doc.push('\n');
        }
        out.push(CodeAction {
            title: format!("Generate docstring for '{}'", proc_def.name),
            edits: vec![crate::rename::TextEdit {
                range: LspRange {
                    start_line: decl.line,
                    start_character: 0,
                    end_line: decl.line,
                    end_character: 0,
                },
                new_text: doc,
            }],
            kind: ActionKind::Source,
            command: None,
        });
    }
    out
}

// extract / inline proc — deferred to a follow-up (LARGE refactor port).
fn extract_inline_actions(
    _source: &str,
    _range: LspRange,
    _analysis: &AnalysisResult,
    _line_index: &LineIndex,
) -> Vec<CodeAction> {
    Vec::new()
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
        assert!(titles.contains(&"Add catch result variable"), "{titles:?}");
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

    // -- S-code-actions-rich: W120 package-require fix --------------

    #[test]
    fn w120_surfaces_add_package_require_action() {
        // The analyser emits W120 (with a CodeFix) for a
        // package-gated command used without `package require`;
        // the code-actions provider lifts that fix into an
        // `Add 'package require ...'` action.
        let src = "tcl::idna decode example.com\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl9.0").clone();
        assert!(
            analysis.diagnostics.iter().any(|d| d.code == "W120"),
            "expected W120 from {:?}",
            analysis.diagnostics,
        );
        let actions = code_actions(src, whole_document_range(src), Some(&analysis));
        let add = actions
            .iter()
            .find(|a| a.title.starts_with("Add 'package require"));
        assert!(
            add.is_some(),
            "expected package-require action in {actions:?}"
        );
        let act = add.unwrap();
        assert_eq!(act.edits.len(), 1);
        assert_eq!(act.edits[0].new_text, "package require tcl::idna\n");
        // Insertion at the top of the file (line 0, col 0).
        assert_eq!(act.edits[0].range.start_line, 0);
        assert_eq!(act.edits[0].range.start_character, 0);
    }

    fn at(line: u32, character: u32) -> LspRange {
        LspRange {
            start_line: line,
            start_character: character,
            end_line: line,
            end_character: character,
        }
    }

    #[test]
    fn fuzzy_package_require_suggests_known_package() {
        // `http::foo` — the `http` prefix matches the `http` package
        // (http::geturl's required_package).
        let reg = tcl_registry::CommandRegistry::build_default();
        let actions = package_require_actions("http::foo $x\n", at(0, 2), &reg);
        assert!(
            actions
                .iter()
                .any(|a| a.title == "Add 'package require http'"),
            "{actions:?}"
        );
        // The edit inserts at the top of the file.
        let act = actions
            .iter()
            .find(|a| a.title.contains("http"))
            .expect("http action");
        assert_eq!(act.edits[0].new_text, "package require http\n");
        assert_eq!(act.edits[0].range.start_line, 0);
    }

    #[test]
    fn fuzzy_package_require_dedups_already_required() {
        // `http` is already required → no `http` suggestion, and the
        // insert line lands after the existing require.
        let reg = tcl_registry::CommandRegistry::build_default();
        let src = "package require http\nhttp::foo\n";
        let actions = package_require_actions(src, at(1, 2), &reg);
        assert!(
            !actions.iter().any(|a| a.title.contains("require http'")),
            "http already required: {actions:?}"
        );
    }

    #[test]
    fn fuzzy_package_require_ignores_short_prefix() {
        let reg = tcl_registry::CommandRegistry::build_default();
        assert!(package_require_actions("x::y\n", at(0, 0), &reg).is_empty());
    }

    #[test]
    fn rank_package_suggestions_orders_exact_before_prefix() {
        let pkgs = vec!["httpd".to_string(), "http".to_string(), "json".to_string()];
        let ranked = rank_package_suggestions("http::get", &pkgs, 5);
        // Exact (`http`, score 0) before prefix (`httpd`, score 1);
        // `json` doesn't match at all.
        assert_eq!(ranked, vec!["http", "httpd"]);
    }

    #[test]
    fn word_at_position_extracts_namespaced_word() {
        assert_eq!(word_at_position("http::foo $x\n", 0, 2), "http::foo");
        assert_eq!(word_at_position("  set y 1\n", 0, 3), "set");
    }
}
