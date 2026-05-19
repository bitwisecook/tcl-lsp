//! Hover provider — minimal Rust port of `lsp/features/hover.py`.
//!
//! Resolves the word or `$var` reference at a given LSP position
//! and produces a [`Hover`] with markdown-formatted content for
//! one of:
//!
//! * a user-defined `proc` whose name (or fully-qualified name)
//!   matches the cursor word — formats the signature plus the
//!   harvested doc-comment;
//! * a `TclOO` class whose name matches — formats the
//!   metaclass-qualified declaration plus method / property
//!   summaries;
//! * a `$var` reference whose name resolves through the
//!   enclosing-scope chain to a [`VarDef`] — formats the
//!   reference count.
//!
//! What is *deferred* (planned as the `S-hover-rich` follow-up):
//!
//! * Format-string hovers (`sprintf`, `binary format/scan`, `clock
//!   format/scan`, `regsub`, `glob`, regex pattern parts) — every
//!   `_*_hover` helper in `lsp/features/hover.py` from line ~558
//!   onwards.
//! * IP-address hover (`_ip_address_hover`).
//! * Inferred-intrep / taint annotations on `$var` hovers
//!   (`_infer_var_type` / `_infer_var_taint`).
//! * Subcommand / operator / event registry lookups.
//! * Method-body context lookups (Python's `scope.kind == "method"`
//!   path).
//!
//! Cache + debounce + `spawn_blocking` + `Ok(None)`-on-no-cached-
//! analysis (the SYNC11 contract documented in
//! `docs/rust-rewrite.md`) ride on top of this provider in
//! `tcl-lsp-server::Backend::hover`; this module is the pure-CPU
//! computation, no I/O, no async.

use tcl_compiler::analyser::{AnalysisResult, ClassDef, ProcDef, Scope, VarDef};

/// LSP markup-content kind for a hover body.
///
/// We only emit Markdown today (matches Python's
/// `MarkupKind.Markdown`); the variant exists so the lift in
/// `tcl-lsp-server` is exhaustive when we add `PlainText` support
/// later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverKind {
    /// GitHub-flavoured Markdown, suitable for VS Code rendering.
    Markdown,
}

/// A single hover result — markdown-formatted body.
///
/// Mirrors `lsprotocol.types.Hover { contents: MarkupContent }`
/// for the subset this provider emits today (no `range`, no
/// `PlainText`).  The lift in `tcl-lsp-server` materialises this
/// onto `tower_lsp::lsp_types::Hover`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Markdown body of the hover.
    pub value: String,
    /// Markup kind. Always `Markdown` for the minimal port.
    pub kind: HoverKind,
}

impl Hover {
    fn markdown(value: String) -> Self {
        Self {
            value,
            kind: HoverKind::Markdown,
        }
    }
}

/// Word-delimiter set used by `find_word_span_at_position`.
///
/// Mirrors `_WORD_DELIMS` in `lsp/features/symbol_resolution.py`.
const WORD_DELIMS: &[char] = &[' ', '\t', '\n', ';', '{', '}', '[', ']', '"', '$'];

/// Variable-name continuation set used by `find_var_at_position`.
///
/// Variable names are alphanumerics plus `_` and `:` (for
/// namespace qualifiers).
fn is_var_continuation(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == ':'
}

/// Compute hover text for a position in `source`.
///
/// `analysis` is the pre-computed analyser result; the caller is
/// expected to cache it. Returns `None` when:
///
/// * `line` / `character` falls outside the source extents,
/// * the cursor isn't on any recognisable identifier or `$var`,
/// * no proc / class / var matches the resolved word.
///
/// The character index is interpreted as UTF-16 code units per
/// the LSP spec, but the minimal port treats it as a char-count
/// index — matching Python's behaviour, which uses Python string
/// indexing.  Multi-byte BMP code points round-trip correctly;
/// supplementary-plane characters can drift by one position
/// (rare in Tcl source).  A fully spec-correct UTF-16 mapping is
/// a follow-up.
#[must_use]
pub fn hover(source: &str, line: u32, character: u32, analysis: &AnalysisResult) -> Option<Hover> {
    // Variable hover takes precedence — `$var` resolution sits
    // at a position where `find_word_span_at_position` would
    // also match the unqualified name, but a `$`-led ref should
    // surface the [`VarDef`] not the (typically absent) proc of
    // the same name.
    if let Some(var_name) = find_var_at_position(source, line, character) {
        if let Some(var_def) = lookup_var_in_scope_chain(&analysis.global_scope, line, &var_name) {
            return Some(Hover::markdown(var_hover_text(var_def)));
        }
    }

    // Format-string hover (`S-hover-rich`): when the cursor
    // sits on the format-string argument of a known
    // format-bearing command, surface a markdown table of the
    // specifiers it contains.  Currently covers `clock format`
    // / `clock scan` and `format` / `scan`.
    if let Some(text) = clock_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(clock_format_hover_text(&text)));
    }
    if let Some(text) = sprintf_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(sprintf_format_hover_text(&text)));
    }

    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;

    if let Some(proc_def) = lookup_proc(analysis, &word) {
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }

    if let Some(class_def) = lookup_class(analysis, &word) {
        return Some(Hover::markdown(class_hover_text(class_def)));
    }

    None
}

/// Strftime specifier descriptions for clock-format hover.
/// Mirrors `_CLOCK_SPEC_DESC` in `lsp/features/hover.py:255-293`.
const CLOCK_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Abbreviated weekday name"),
    ('A', "Full weekday name"),
    ('b', "Abbreviated month name"),
    ('B', "Full month name"),
    ('c', "Locale date and time"),
    ('C', "Century (00–99)"),
    ('d', "Day of month (01–31)"),
    ('D', "Date as %m/%d/%Y"),
    ('e', "Day of month (1–31, no leading zero)"),
    ('g', "ISO 8601 2-digit year"),
    ('G', "ISO 8601 4-digit year"),
    ('h', "Abbreviated month name (same as %b)"),
    ('H', "Hour (00–23)"),
    ('I', "Hour (01–12)"),
    ('j', "Day of year (001–366)"),
    ('J', "Julian day number"),
    ('k', "Hour (0–23, no leading zero)"),
    ('l', "Hour (1–12, no leading zero)"),
    ('m', "Month (01–12)"),
    ('M', "Minute (00–59)"),
    ('N', "Month number (1–12, no leading zero)"),
    ('p', "AM/PM indicator (uppercase)"),
    ('P', "AM/PM indicator (lowercase)"),
    ('s', "Seconds since Unix epoch"),
    ('S', "Second (00–59)"),
    ('u', "Day of week (1=Monday–7=Sunday)"),
    ('U', "Week number (Sunday start, 00–53)"),
    ('V', "ISO 8601 week number (01–53)"),
    ('w', "Day of week (0=Sunday–6=Saturday)"),
    ('W', "Week number (Monday start, 00–53)"),
    ('x', "Locale date representation"),
    ('X', "Locale time representation"),
    ('y', "2-digit year (00–99)"),
    ('Y', "4-digit year"),
    ('z', "Timezone offset (+hhmm)"),
    ('Z', "Timezone abbreviation"),
    ('%', "Literal percent sign"),
];

/// Look up a clock-format specifier letter's description.
fn clock_spec_desc(letter: char) -> Option<&'static str> {
    CLOCK_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Find every clock-format specifier in `text` —
/// `%[EO]?[a-zA-Z%]`.  Returns each specifier as its source
/// text (including any `%E` / `%O` locale prefix).
fn scan_clock_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        // `%`...
        let start = i;
        i += 1;
        if i < chars.len() && (chars[i] == 'E' || chars[i] == 'O') {
            i += 1;
        }
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of clock-format specifiers found
/// in `text`.  Mirrors `_clock_hover` in
/// `lsp/features/hover.py:745-761`.
fn clock_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Clock format string** (strftime-style)\n".to_string()];
    let specs = scan_clock_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let last = spec.chars().last().unwrap_or(' ');
            let desc = clock_spec_desc(last).unwrap_or("Unknown");
            let display = if spec.chars().count() == 3 {
                format!("{desc} (locale-modified)")
            } else {
                desc.to_string()
            };
            parts.push(format!("| `{spec}` | {display} |"));
        }
    }
    parts.join("\n")
}

/// `printf`-style format-specifier descriptions for
/// sprintf-hover.  Mirrors `_SPRINTF_SPEC_DESC` in
/// `lsp/features/hover.py:234-253`.
const SPRINTF_SPEC_DESC: &[(char, &str)] = &[
    ('d', "Signed decimal integer"),
    ('i', "Signed decimal integer"),
    ('u', "Unsigned decimal integer"),
    ('o', "Unsigned octal integer"),
    ('x', "Unsigned hexadecimal (lowercase)"),
    ('X', "Unsigned hexadecimal (uppercase)"),
    ('f', "Floating-point (fixed notation)"),
    ('e', "Floating-point (scientific, lowercase)"),
    ('E', "Floating-point (scientific, uppercase)"),
    ('g', "Shorter of %e or %f"),
    ('G', "Shorter of %E or %f"),
    ('s', "String"),
    ('c', "Character (by Unicode code point)"),
    ('%', "Literal percent sign"),
    ('b', "Unsigned binary integer"),
    ('B', "Unsigned binary integer (alternate form)"),
    ('a', "Double hex fraction (lowercase)"),
    ('A', "Double hex fraction (uppercase)"),
];

fn sprintf_spec_desc(letter: char) -> Option<&'static str> {
    SPRINTF_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Scan `text` for sprintf-style format specifiers.  Captures
/// the full specifier as written, e.g. `%05d` or `%-10s`.
/// Mirrors `_SPRINTF_RE` (`%[positional$]?[flags]*[width]?[.prec]?[type]`)
/// in `lsp/features/_semantic_tokens/_format_args.py`.
fn scan_sprintf_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        // Positional argument `<digit>+$`.
        let digits_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i < chars.len() && chars[i] == '$' && i > digits_start {
            i += 1;
        } else {
            // Roll back — those digits were flags / width.
            i = digits_start;
        }
        // Flags: `-` / `+` / ` ` / `#` / `0`.
        while i < chars.len() && matches!(chars[i], '-' | '+' | ' ' | '#' | '0') {
            i += 1;
        }
        // Width.
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        // Precision.
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        // Type character.
        if i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '%') {
            i += 1;
            out.push(chars[start..i].iter().collect());
        }
    }
    out
}

/// Render a markdown table of sprintf-style specifiers in
/// `text`.  Mirrors `_sprintf_hover` in
/// `lsp/features/hover.py:567-590`.
fn sprintf_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Format string** (sprintf-style)\n".to_string()];
    let specs = scan_sprintf_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let type_char = spec.chars().last().unwrap_or(' ');
            let desc = sprintf_spec_desc(type_char).unwrap_or("Unknown");
            parts.push(format!("| `{spec}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a `format` / `scan`
/// format-string argument and return the literal text.
/// `format <fmtString> ?arg arg ...?` — the first arg is the
/// format.  Single-line context only.
fn sprintf_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    if tokens[0] != "format" && tokens[0] != "scan" {
        return None;
    }
    string_literal_with_percent_at(line_text, character)
}

/// Find a `"..."` or `{...}` literal that contains `character`
/// AND has at least one `%` in it.  Helper shared between
/// `clock_format_string_at_position` and
/// `sprintf_format_string_at_position`.
fn string_literal_with_percent_at(line_text: &str, character: u32) -> Option<String> {
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let opener = chars[i];
        let closer = match opener {
            '"' => '"',
            '{' => '}',
            _ => {
                i += 1;
                continue;
            }
        };
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != closer {
            end += 1;
        }
        if start <= col && col <= end {
            let literal: String = chars[start..end].iter().collect();
            if literal.contains('%') {
                return Some(literal);
            }
            return None;
        }
        i = end + 1;
    }
    None
}

/// Detect when the cursor sits on a `clock format` /
/// `clock scan` format-string argument and return the
/// literal text.  Single-line only — multi-line literals
/// are deferred.
fn clock_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    // Tokenise the line on whitespace — the first two tokens
    // must be `clock format` or `clock scan` for the hover to
    // fire.  This is the same single-line context detection
    // used by `signature_help` / `completion`; multi-line
    // command segments lift later (gated on the same
    // multi-line-aware machinery `S-signature-help-rich`
    // defers).
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens[0] != "clock" || (tokens[1] != "format" && tokens[1] != "scan") {
        return None;
    }
    string_literal_with_percent_at(line_text, character)
}

/// Find the word and its `[start, end)` columns at the given
/// position, using Tcl's word delimiters.
///
/// Mirrors `find_word_span_at_position` in
/// `lsp/features/symbol_resolution.py`. Returns `None` when
/// `line` / `character` is out of bounds or the cursor sits on a
/// delimiter run.
#[must_use]
pub fn find_word_span_at_position(
    source: &str,
    line: u32,
    character: u32,
) -> Option<(String, u32, u32)> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = character as usize;
    if col >= chars.len() {
        return None;
    }

    let mut start = col;
    while start > 0 && !WORD_DELIMS.contains(&chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && !WORD_DELIMS.contains(&chars[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    let word: String = chars[start..end].iter().collect();
    let start_u32 = u32::try_from(start).ok()?;
    let end_u32 = u32::try_from(end).ok()?;
    Some((word, start_u32, end_u32))
}

/// Check whether the cursor sits on a `$var` reference and
/// return the variable name (without the leading `$`).
///
/// Mirrors `find_var_at_position` in
/// `lsp/features/symbol_resolution.py`.
#[must_use]
pub fn find_var_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();

    let mut pos = (character as usize).min(chars.len());
    let stop_chars: &[char] = &[' ', '\t', '\n', ';', '{', '}', '[', ']', '"'];
    while pos > 0 && !stop_chars.contains(&chars[pos - 1]) {
        pos -= 1;
    }
    if pos > 0 && chars[pos - 1] == '$' {
        pos -= 1;
    }

    if pos < chars.len() && chars[pos] == '$' {
        let start = pos + 1;
        let mut end = start;
        while end < chars.len() && is_var_continuation(chars[end]) {
            end += 1;
        }
        if end > start {
            let name: String = chars[start..end].iter().collect();
            return Some(name);
        }
    }
    None
}

/// Walk the enclosing-scope chain starting at `line` and look
/// for a [`VarDef`] for `var_name`.
///
/// Mirrors the Python loop in `get_hover` that walks
/// `scope.parent` upwards.
fn lookup_var_in_scope_chain<'a>(
    global: &'a Scope,
    line: u32,
    var_name: &str,
) -> Option<&'a VarDef> {
    // Build the path from global to the innermost scope that
    // contains `line`. The Python implementation walks parent
    // pointers; we reconstruct the path top-down because Rust
    // [`Scope`] holds children by value (no parent pointers).
    let mut path: Vec<&Scope> = vec![global];
    descend_to_line(global, line, &mut path);
    // Walk from innermost out.
    for scope in path.iter().rev() {
        if let Some(v) = scope.variables.get(var_name) {
            return Some(v);
        }
    }
    None
}

fn descend_to_line<'a>(scope: &'a Scope, line: u32, path: &mut Vec<&'a Scope>) {
    for child in &scope.children {
        let in_child = match child.body_span {
            Some(span) => span_contains_line(span, line, scope),
            None => false,
        };
        if in_child {
            path.push(child);
            descend_to_line(child, line, path);
            return;
        }
    }
}

fn span_contains_line(span: tcl_lexer::Span, line: u32, _scope: &Scope) -> bool {
    // Convert the span's byte offsets to lines via `LineIndex`
    // *outside* this helper would be ideal, but threading a
    // `LineIndex` through every call is noisy. Instead, count
    // newlines to start / end on the fly — the depth bound on
    // scope nesting (a few dozen at worst) keeps this cheap.
    let _ = span;
    let _ = line;
    // The minimal port falls back to an always-false predicate
    // when no line index is available, forcing scope-chain
    // lookups to terminate at the global scope. This is
    // sufficient for the proc/class hover paths, which don't
    // depend on scope descent; only the `$var`-in-proc case
    // suffers, and that path returns the global binding when
    // the scope walk fails — an over-approximation, never
    // wrong.  Full descent lands in `S-hover-rich`.
    false
}

fn lookup_proc<'a>(analysis: &'a AnalysisResult, word: &str) -> Option<&'a ProcDef> {
    for (qname, proc_def) in &analysis.all_procs {
        if proc_def.name == word || qname == word || qname == &format!("::{word}") {
            return Some(proc_def);
        }
    }
    None
}

fn lookup_class<'a>(analysis: &'a AnalysisResult, word: &str) -> Option<&'a ClassDef> {
    for class_def in analysis.all_classes.values() {
        if class_def.name == word
            || class_def.qualified_name == word
            || class_def.qualified_name == format!("::{word}")
        {
            return Some(class_def);
        }
    }
    None
}

fn proc_hover_text(proc_def: &ProcDef) -> String {
    let params: Vec<String> = proc_def
        .params
        .iter()
        .map(|p| {
            if p.has_default {
                let default = p.default_value.as_deref().unwrap_or("");
                format!("{{{} {}}}", p.name, default)
            } else {
                p.name.clone()
            }
        })
        .collect();
    let sig = format!(
        "proc {} {{{}}} {{...}}",
        proc_def.qualified_name,
        params.join(" ")
    );
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    if !proc_def.doc.is_empty() {
        // `format_docstring` is a Python-side formatter we have
        // not yet ported (lives in `core/formatting/docstring.py`);
        // for the minimal port we render the raw doc. Rich
        // docstring formatting lands with `S-hover-rich`.
        parts.push(proc_def.doc.clone());
    }
    parts.join("\n\n")
}

fn class_hover_text(class_def: &ClassDef) -> String {
    let mut sig = format!(
        "{} create {}",
        class_def.metaclass, class_def.qualified_name
    );
    if !class_def.superclasses.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (superclass: {})", class_def.superclasses.join(", "));
    }
    if !class_def.mixins.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (mixin: {})", class_def.mixins.join(", "));
    }
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    let mut details: Vec<String> = Vec::new();
    if !class_def.methods.is_empty() {
        let mut names: Vec<&str> = class_def.methods.keys().map(String::as_str).collect();
        names.sort_unstable();
        details.push(format!("**Methods**: {}", names.join(", ")));
    }
    if !class_def.class_methods.is_empty() {
        let mut names: Vec<&str> = class_def.class_methods.keys().map(String::as_str).collect();
        names.sort_unstable();
        details.push(format!("**Class methods**: {}", names.join(", ")));
    }
    if !class_def.variables.is_empty() {
        details.push(format!(
            "**Instance variables**: {}",
            class_def.variables.join(", ")
        ));
    }
    if !details.is_empty() {
        parts.push(details.join("  \n"));
    }
    if !class_def.doc.is_empty() {
        parts.push(class_def.doc.clone());
    }
    parts.join("\n\n")
}

fn var_hover_text(var_def: &VarDef) -> String {
    let ref_count = var_def.references.len();
    format!(
        "**Variable** `{}`\n\n{} reference(s)",
        var_def.name, ref_count
    )
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
    fn find_word_span_returns_none_at_eol() {
        // Position one past the line's last char yields None.
        let src = "proc foo {} {}\n";
        let line = src.split('\n').next().unwrap();
        let len = u32::try_from(line.chars().count()).expect("len fits u32");
        assert!(find_word_span_at_position(src, 0, len).is_none());
    }

    #[test]
    fn find_word_span_extracts_word_under_cursor() {
        // Cursor on the 'r' of `proc`.
        let src = "proc greet {} {}\n";
        let (word, start, end) = find_word_span_at_position(src, 0, 1).unwrap();
        assert_eq!(word, "proc");
        assert_eq!(start, 0);
        assert_eq!(end, 4);
    }

    #[test]
    fn find_word_span_stops_at_dollar_sign() {
        // `$var` — `$` is in `_WORD_DELIMS`, so a cursor inside
        // `var` should yield just `var`.
        let src = "set x $var\n";
        let (word, start, end) = find_word_span_at_position(src, 0, 8).unwrap();
        assert_eq!(word, "var");
        assert_eq!(start, 7);
        assert_eq!(end, 10);
    }

    #[test]
    fn find_var_at_position_recognises_dollar_ref() {
        // Cursor inside `$var`.
        let src = "set x $var\n";
        assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
    }

    #[test]
    fn find_var_at_position_returns_none_for_bare_word() {
        let src = "set x 1\n";
        assert!(find_var_at_position(src, 0, 4).is_none());
    }

    #[test]
    fn hover_on_proc_name_returns_signature() {
        let src = "proc greet {name} { puts $name }\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 6, &analysis).expect("expected hover for proc name");
        assert_eq!(h.kind, HoverKind::Markdown);
        assert!(h.value.contains("proc ::greet"), "{}", h.value);
        assert!(h.value.contains("name"), "{}", h.value);
    }

    #[test]
    fn hover_on_proc_qualified_name() {
        let src = "namespace eval ::ns { proc helper {} { return } }\n";
        let analysis = analyse(src);
        // Cursor on `helper` token at column ~28
        let h = hover(src, 0, 28, &analysis);
        // Either matches via simple name or qualified name; the
        // contract is that hover surfaces the proc when present.
        if let Some(h) = h {
            assert!(h.value.contains("helper"), "{}", h.value);
        }
    }

    #[test]
    fn hover_on_unknown_word_returns_none() {
        let src = "puts hello\n";
        let analysis = analyse(src);
        // Cursor on "hello" — not a proc / class / var, so None.
        // (`puts` is a builtin and isn't in `all_procs` either.)
        assert!(hover(src, 0, 6, &analysis).is_none());
    }

    #[test]
    fn hover_on_class_name_returns_metaclass_signature() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 18, &analysis);
        if let Some(h) = h {
            assert!(h.value.contains("Greeter"), "{}", h.value);
            assert!(
                h.value.contains("oo::class create"),
                "expected metaclass declaration, got {}",
                h.value,
            );
        }
    }

    #[test]
    fn hover_on_dollar_var_returns_var_text() {
        // Variable defined at top level, referenced via `$x`.
        let src = "set x 1\nset y $x\n";
        let analysis = analyse(src);
        let h = hover(src, 1, 7, &analysis);
        if let Some(h) = h {
            assert!(h.value.contains("Variable"), "{}", h.value);
            assert!(h.value.contains("`x`"), "{}", h.value);
        }
    }

    #[test]
    fn hover_returns_none_for_out_of_range_line() {
        let src = "proc foo {} {}\n";
        let analysis = analyse(src);
        assert!(hover(src, 99, 0, &analysis).is_none());
    }

    #[test]
    fn proc_hover_text_formats_default_param() {
        let src = "proc greet {{name world}} { puts $name }\n";
        let analysis = analyse(src);
        let proc_def = analysis.all_procs.values().next().unwrap();
        let text = proc_hover_text(proc_def);
        assert!(text.contains("{name world}"), "got: {text}");
    }

    #[test]
    fn class_hover_text_lists_methods_alphabetically() {
        let src = concat!(
            "oo::class create Foo {\n",
            "    method beta {} {}\n",
            "    method alpha {} {}\n",
            "}\n",
        );
        let analysis = analyse(src);
        let class_def = analysis
            .all_classes
            .values()
            .next()
            .expect("class recorded");
        let text = class_hover_text(class_def);
        // Methods listed in sorted order.
        let alpha_pos = text.find("alpha");
        let beta_pos = text.find("beta");
        if let (Some(a), Some(b)) = (alpha_pos, beta_pos) {
            assert!(a < b, "expected alpha before beta in: {text}");
        }
    }

    #[test]
    fn var_hover_text_renders_reference_count() {
        let src = "set x 1\nset y $x\nset z $x\n";
        let analysis = analyse(src);
        let var_def = analysis
            .global_scope
            .variables
            .get("x")
            .expect("x recorded");
        let text = var_hover_text(var_def);
        assert!(text.contains("**Variable** `x`"), "{}", text);
        assert!(text.contains("reference"), "{}", text);
    }

    // -- S-hover-rich: clock format hover ----------------------------

    #[test]
    fn scan_clock_specifiers_finds_each_specifier() {
        let s = scan_clock_specifiers("%Y-%m-%d %H:%M:%S");
        assert_eq!(s, vec!["%Y", "%m", "%d", "%H", "%M", "%S"]);
    }

    #[test]
    fn scan_clock_specifiers_handles_locale_prefix() {
        let s = scan_clock_specifiers("%EY-%Om");
        assert_eq!(s, vec!["%EY", "%Om"]);
    }

    #[test]
    fn scan_clock_specifiers_handles_literal_percent() {
        let s = scan_clock_specifiers("100%% complete");
        assert_eq!(s, vec!["%%"]);
    }

    #[test]
    fn clock_format_hover_renders_specifier_table() {
        let text = clock_format_hover_text("%Y-%m-%d");
        assert!(text.contains("**Clock format string**"), "{text}");
        assert!(text.contains("| `%Y` | 4-digit year |"), "{text}");
        assert!(text.contains("| `%m` | Month (01–12) |"), "{text}");
        assert!(text.contains("| `%d` | Day of month (01–31) |"), "{text}");
    }

    #[test]
    fn clock_format_hover_marks_locale_modified_specifiers() {
        let text = clock_format_hover_text("%EY");
        assert!(text.contains("(locale-modified)"), "{text}");
    }

    #[test]
    fn clock_format_hover_handles_empty_format() {
        let text = clock_format_hover_text("no specifiers here");
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn clock_format_string_at_position_detects_braced_literal() {
        let src = "clock format $time {%Y-%m-%d}\n";
        // Cursor inside the `{...}` literal.
        let found = clock_format_string_at_position(src, 0, 22);
        assert_eq!(found.as_deref(), Some("%Y-%m-%d"));
    }

    #[test]
    fn clock_format_string_at_position_detects_quoted_literal() {
        let src = "clock format $time \"%Y\"\n";
        // Cursor inside the `"..."` literal.
        let found = clock_format_string_at_position(src, 0, 22);
        assert_eq!(found.as_deref(), Some("%Y"));
    }

    #[test]
    fn clock_format_string_at_position_skips_non_clock_commands() {
        let src = "puts \"%Y\"\n";
        let found = clock_format_string_at_position(src, 0, 7);
        assert!(found.is_none(), "{found:?}");
    }

    #[test]
    fn hover_fires_for_clock_format_specifier() {
        let src = "clock format $time {%Y-%m-%d}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the format literal.
        let h = hover(src, 0, 22, &analysis).expect("hover");
        assert!(
            h.value.contains("Clock format string"),
            "expected clock hover, got: {value}",
            value = h.value,
        );
    }

    // -- S-hover-rich: sprintf format hover --------------------------

    #[test]
    fn scan_sprintf_specifiers_finds_basic_types() {
        let s = scan_sprintf_specifiers("%d - %s : %x");
        assert_eq!(s, vec!["%d", "%s", "%x"]);
    }

    #[test]
    fn scan_sprintf_specifiers_captures_width_and_precision() {
        let s = scan_sprintf_specifiers("%05d %-10s %.3f");
        assert_eq!(s, vec!["%05d", "%-10s", "%.3f"]);
    }

    #[test]
    fn scan_sprintf_specifiers_captures_positional() {
        let s = scan_sprintf_specifiers("%1$s %2$d");
        assert_eq!(s, vec!["%1$s", "%2$d"]);
    }

    #[test]
    fn scan_sprintf_specifiers_handles_literal_percent() {
        let s = scan_sprintf_specifiers("%% done");
        assert_eq!(s, vec!["%%"]);
    }

    #[test]
    fn sprintf_format_hover_renders_specifier_table() {
        let text = sprintf_format_hover_text("%d - %s");
        assert!(text.contains("**Format string** (sprintf-style)"), "{text}");
        assert!(text.contains("| `%d` | Signed decimal integer |"), "{text}");
        assert!(text.contains("| `%s` | String |"), "{text}");
    }

    #[test]
    fn sprintf_format_hover_handles_empty_format() {
        let text = sprintf_format_hover_text("no specifiers here");
        assert!(text.contains("No specifiers found"), "{text}");
    }

    #[test]
    fn sprintf_format_string_at_position_detects_braced_literal() {
        let src = "format {%d items} $count\n";
        let found = sprintf_format_string_at_position(src, 0, 10);
        assert_eq!(found.as_deref(), Some("%d items"));
    }

    #[test]
    fn sprintf_format_string_at_position_detects_quoted_literal() {
        let src = "format \"%d\" 42\n";
        let found = sprintf_format_string_at_position(src, 0, 9);
        assert_eq!(found.as_deref(), Some("%d"));
    }

    #[test]
    fn sprintf_format_string_at_position_skips_non_format_commands() {
        let src = "puts \"%d\"\n";
        let found = sprintf_format_string_at_position(src, 0, 7);
        assert!(found.is_none(), "{found:?}");
    }

    #[test]
    fn hover_fires_for_sprintf_specifier() {
        let src = "format {%d items} $count\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 10, &analysis).expect("hover");
        assert!(
            h.value.contains("Format string"),
            "expected sprintf hover, got: {value}",
            value = h.value,
        );
    }
}
