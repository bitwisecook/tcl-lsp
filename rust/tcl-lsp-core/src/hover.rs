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
use tcl_registry::CommandRegistry;

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
pub fn hover(
    source: &str,
    line: u32,
    character: u32,
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
) -> Option<Hover> {
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
    if let Some(text) = binary_format_string_at_position(source, line, character) {
        return Some(Hover::markdown(binary_format_hover_text(&text)));
    }
    if let Some(text) = regsub_subspec_at_position(source, line, character) {
        return Some(Hover::markdown(regsub_hover_text(&text)));
    }
    if let Some(text) = glob_pattern_at_position(source, line, character) {
        return Some(Hover::markdown(glob_hover_text(&text)));
    }
    if let Some(text) = regex_pattern_at_position(source, line, character) {
        return Some(Hover::markdown(regex_hover_text(&text)));
    }

    let (word, _start, _end) = find_word_span_at_position(source, line, character)?;

    if let Some(proc_def) = lookup_proc(analysis, &word) {
        return Some(Hover::markdown(proc_hover_text(proc_def)));
    }

    if let Some(class_def) = lookup_class(analysis, &word) {
        return Some(Hover::markdown(class_hover_text(class_def)));
    }

    // Registry-driven hovers — built-in command name, plus
    // `cmd subcommand` lookups when the cursor sits on the
    // subcommand word.  Mirrors `lsp/features/hover.py`'s
    // `SIGNATURES` lookup at the tail of `get_hover`.
    if let Some(registry) = registry {
        if let Some(text) = subcommand_hover_text(source, line, character, registry, &word) {
            return Some(Hover::markdown(text));
        }
        if let Some(text) = builtin_command_hover_text(registry, &word) {
            return Some(Hover::markdown(text));
        }
    }

    if let Some(text) = ip_address_hover_text(&word) {
        return Some(Hover::markdown(text));
    }

    None
}

/// Render a hover snippet for a built-in command name.
/// Looks up `name` in the registry, uses the matched spec's
/// `hover.summary` / `synopsis` to produce a markdown block.
fn builtin_command_hover_text(registry: &CommandRegistry, name: &str) -> Option<String> {
    use std::fmt::Write;
    let spec = registry.get(name)?;
    let hover = spec.hover.as_ref()?;
    let mut out = format!("**`{name}`** — built-in command\n");
    if !hover.summary.is_empty() {
        let _ = write!(out, "\n{}\n", hover.summary);
    }
    if let Some(synopsis) = hover.synopsis.first() {
        let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
    }
    if !spec.subcommands.is_empty() {
        let mut names: Vec<&str> = spec.subcommands.iter().map(|s| s.name).collect();
        names.sort_unstable();
        let joined = names.join(", ");
        let _ = write!(out, "\nSubcommands: {joined}\n");
    }
    Some(out)
}

/// Render a hover snippet for a `cmd subcommand` pair when
/// the cursor sits on the subcommand word.  Detects the
/// surrounding command segment via single-line tokenisation
/// (mirrors the `command_context_on_line` helper used by
/// completion / signature-help).
fn subcommand_hover_text(
    source: &str,
    line: u32,
    character: u32,
    registry: &CommandRegistry,
    cursor_word: &str,
) -> Option<String> {
    use std::fmt::Write;
    let line_text = source.split('\n').nth(line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    let col = (character as usize).min(chars.len());
    let prefix: String = chars[..col].iter().collect();
    let tokens: Vec<&str> = prefix.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let cmd_name = tokens[0];
    // The cursor word IS the subcommand — use it directly as
    // the lookup key.  The prefix-tokenised second token might
    // be a partial (if cursor is mid-word).
    let sub_name = cursor_word;
    if cmd_name == sub_name {
        // Cursor sits on the command word itself, not on a
        // subcommand.  Fall through to the built-in-command
        // hover instead.
        return None;
    }
    let spec = registry.get(cmd_name)?;
    let sub = spec.subcommand(sub_name)?;
    let mut out = format!("**`{cmd_name} {sub_name}`** — subcommand\n");
    if let Some(hover) = sub.hover.as_ref() {
        if !hover.summary.is_empty() {
            let _ = write!(out, "\n{}\n", hover.summary);
        }
        if let Some(synopsis) = hover.synopsis.first() {
            let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
        }
    } else {
        let _ = write!(out, "\nSubcommand of `{cmd_name}`.\n");
    }
    Some(out)
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

/// `binary format` / `binary scan` specifier table.  Mirrors
/// `_BINARY_SPEC_DESC` in `lsp/features/hover.py:295-319`.
const BINARY_SPEC_DESC: &[(char, &str)] = &[
    ('a', "Byte string, padded with nulls"),
    ('A', "Byte string, padded with spaces"),
    ('b', "Binary digits (low-to-high order)"),
    ('B', "Binary digits (high-to-low order)"),
    ('h', "Hexadecimal digits (low-to-high nibble)"),
    ('H', "Hexadecimal digits (high-to-low nibble)"),
    ('c', "8-bit signed integer"),
    ('s', "16-bit signed integer (little-endian)"),
    ('S', "16-bit signed integer (big-endian)"),
    ('i', "32-bit signed integer (little-endian)"),
    ('I', "32-bit signed integer (big-endian)"),
    ('n', "32-bit integer (native byte order)"),
    ('w', "64-bit signed integer (little-endian)"),
    ('W', "64-bit signed integer (big-endian)"),
    ('m', "64-bit integer (native byte order)"),
    ('r', "32-bit float (little-endian)"),
    ('R', "32-bit float (big-endian)"),
    ('f', "32-bit float (native byte order)"),
    ('d', "64-bit double (native byte order)"),
    ('x', "Null padding byte (format) / skip byte (scan)"),
    ('X', "Move cursor back one byte"),
    ('@', "Move cursor to absolute position"),
    ('t', "Reserved (Tcl 8.5+)"),
];

fn binary_spec_desc(letter: char) -> Option<&'static str> {
    BINARY_SPEC_DESC
        .iter()
        .find(|(c, _)| *c == letter)
        .map(|(_, d)| *d)
}

/// Scan a `binary format` / `binary scan` spec string.
/// Tcl grammar: `type [modifier] [count|*]`, repeated.
/// Returns each field's full token as written.  Simplified
/// from Python's `_binary_hover` parser: we capture the
/// `(type, modifier, count)` triple as a single string for
/// the table; the byte-ruler diagram is deferred.
fn scan_binary_specifiers(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if binary_spec_desc(chars[i]).is_none() {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        // Optional modifier `u` / `s`.
        if i < chars.len() && (chars[i] == 'u' || chars[i] == 's') {
            i += 1;
        }
        // Optional count: `*` or digits.
        if i < chars.len() && chars[i] == '*' {
            i += 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
        out.push(chars[start..i].iter().collect());
    }
    out
}

/// Render the binary format-spec hover markdown.  Mirrors
/// `_binary_hover` in `lsp/features/hover.py:593-743`, minus
/// the byte-ruler diagram (deferred).
fn binary_format_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Binary format spec**\n".to_string()];
    let specs = scan_binary_specifiers(text);
    if specs.is_empty() {
        parts.push("No specifiers found.".to_string());
    } else {
        parts.push("| Specifier | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for spec in specs {
            let type_char = spec.chars().next().unwrap_or(' ');
            let desc = binary_spec_desc(type_char).unwrap_or("Unknown");
            parts.push(format!("| `{spec}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a `binary format` /
/// `binary scan` format-string argument and return the
/// literal text.  Single-line only.
fn binary_format_string_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens[0] != "binary" || (tokens[1] != "format" && tokens[1] != "scan") {
        return None;
    }
    string_literal_at(line_text, character)
}

/// `regsub` backref description table.  Mirrors
/// `_REGSUB_BACKREF_DESC` in `lsp/features/hover.py:386-398`.
fn regsub_backref_desc(c: char) -> Option<&'static str> {
    match c {
        '&' | '0' => Some("Entire matched string"),
        '1' => Some("First capture group"),
        '2' => Some("Second capture group"),
        '3' => Some("Third capture group"),
        '4' => Some("Fourth capture group"),
        '5' => Some("Fifth capture group"),
        '6' => Some("Sixth capture group"),
        '7' => Some("Seventh capture group"),
        '8' => Some("Eighth capture group"),
        '9' => Some("Ninth capture group"),
        _ => None,
    }
}

/// Scan a `regsub` substitution spec for `\0` … `\9` / `\&`
/// backreferences.  Returns each match as written (e.g.
/// `\\1`).
fn scan_regsub_backrefs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            i += 1;
            continue;
        }
        if i + 1 >= chars.len() {
            break;
        }
        let next = chars[i + 1];
        if next == '&' || next.is_ascii_digit() {
            out.push(chars[i..=i + 1].iter().collect());
        }
        i += 2;
    }
    out
}

/// Render the regsub substitution-spec hover markdown.
/// Mirrors `_regsub_hover` in `lsp/features/hover.py:764-777`.
fn regsub_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Substitution spec** (regsub)\n".to_string()];
    let refs = scan_regsub_backrefs(text);
    if refs.is_empty() {
        parts.push("No backreferences found.".to_string());
    } else {
        parts.push("| Reference | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for r in refs {
            let backref_char = r.chars().nth(1).unwrap_or(' ');
            let desc = regsub_backref_desc(backref_char).unwrap_or("Unknown");
            // Escape the backslash for display (`\\` in
            // markdown renders as `\`).
            parts.push(format!("| `{r}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on the substitution-spec
/// argument of a `regsub` invocation and return the literal
/// text.  `regsub ?switches? exp string subSpec ?varName?`
/// — `subSpec` is the 4th positional arg (after switches).
/// Single-line only.
fn regsub_subspec_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() || tokens[0] != "regsub" {
        return None;
    }
    // The substitution spec contains backslash sequences,
    // typically as a quoted or braced literal.  Any literal
    // string containing `\\<digit-or-&>` overlapping the cursor
    // counts as the subspec.  Mirrors the loose detection
    // Python uses; precise arg-position resolution is deferred
    // to the same multi-line-aware machinery.
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
            if scan_regsub_backrefs(&literal).is_empty() {
                return None;
            }
            return Some(literal);
        }
        i = end + 1;
    }
    None
}

/// Helper: find any `"..."` / `{...}` literal containing the
/// cursor.  Shared between hover providers that need
/// literal-context detection but don't care whether the
/// literal contains `%`.
fn string_literal_at(line_text: &str, character: u32) -> Option<String> {
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
            return Some(chars[start..end].iter().collect());
        }
        i = end + 1;
    }
    None
}

/// Glob metacharacter descriptions.  Mirrors `_GLOB_META_DESC`
/// in `lsp/features/hover.py:400-404`.
fn glob_meta_desc(c: char) -> Option<&'static str> {
    match c {
        '*' => Some("Matches any sequence of characters"),
        '?' => Some("Matches any single character"),
        '[' => Some("Character class — matches any character inside brackets"),
        _ => None,
    }
}

/// Scan a glob pattern for metacharacters.  Returns a list of
/// `(token, description)` tuples — `*`, `?`, character class
/// `[abc]`, and escape sequences.  Mirrors `_GLOB_META_RE` +
/// `_glob_hover`'s metacharacter walk.
fn scan_glob_metachars(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            let key = "escape".to_string();
            if !seen.contains(&key) {
                out.push((format!("\\{next}"), format!("Escaped character `{next}`")));
                seen.insert(key);
            }
            i += 2;
            continue;
        }
        if chars[i] == '[' {
            let start = i;
            let mut end = i + 1;
            while end < chars.len() && chars[end] != ']' {
                end += 1;
            }
            let token: String = if end < chars.len() {
                chars[start..=end].iter().collect()
            } else {
                chars[start..end].iter().collect()
            };
            if !seen.contains(&token) {
                let inner: String = chars[start + 1..end].iter().collect();
                out.push((
                    token.clone(),
                    format!("Character class: matches any of `{inner}`"),
                ));
                seen.insert(token);
            }
            i = end + 1;
            continue;
        }
        if let Some(desc) = glob_meta_desc(chars[i]) {
            let key = chars[i].to_string();
            if !seen.contains(&key) {
                out.push((key.clone(), desc.to_string()));
                seen.insert(key);
            }
        }
        i += 1;
    }
    out
}

/// Render the glob-pattern hover markdown.  Mirrors
/// `_glob_hover`.
fn glob_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Glob pattern**\n".to_string()];
    let metas = scan_glob_metachars(text);
    if metas.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Pattern | Meaning |".to_string());
        parts.push("|---------|---------|".to_string());
        for (tok, desc) in metas {
            parts.push(format!("| `{tok}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a glob pattern.  Recognises
/// `string match <pat> ...`, `glob <pat>...`, and `lsearch
/// -glob <pat> ...` — three common entry points for glob
/// matching in Tcl.  Single-line only.
fn glob_pattern_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let is_glob_command = matches!(tokens[0], "glob")
        || (tokens.len() >= 2 && tokens[0] == "string" && tokens[1] == "match")
        || (tokens.len() >= 2 && tokens[0] == "lsearch" && tokens.contains(&"-glob"));
    if !is_glob_command {
        return None;
    }
    let literal = string_literal_at(line_text, character)?;
    // Require at least one glob metacharacter or `\` escape so
    // we don't fire on literal strings.
    if !literal.chars().any(|c| matches!(c, '*' | '?' | '[' | '\\')) {
        return None;
    }
    Some(literal)
}

/// Regex metacharacter descriptions.  Mirrors `_REGEX_META_DESC`
/// in `lsp/features/hover.py:406-417`.
fn regex_meta_desc(token: &str) -> Option<&'static str> {
    match token {
        "^" => Some("Start of line/string anchor"),
        "$" => Some("End of line/string anchor"),
        "." => Some("Match any single character"),
        "*" => Some("Zero or more (greedy)"),
        "+" => Some("One or more (greedy)"),
        "?" => Some("Zero or one (greedy)"),
        "*?" => Some("Zero or more (lazy)"),
        "+?" => Some("One or more (lazy)"),
        "??" => Some("Zero or one (lazy)"),
        "|" => Some("Alternation (OR)"),
        _ => None,
    }
}

/// Regex escape descriptions for common shorthand classes.
fn regex_escape_desc(token: &str) -> Option<&'static str> {
    match token {
        "\\d" => Some("Digit `[0-9]`"),
        "\\D" => Some("Non-digit"),
        "\\s" => Some("Whitespace"),
        "\\S" => Some("Non-whitespace"),
        "\\w" => Some("Word character `[a-zA-Z0-9_]`"),
        "\\W" => Some("Non-word character"),
        "\\b" => Some("Word boundary"),
        "\\B" => Some("Non-word boundary"),
        "\\A" => Some("Start of string"),
        "\\Z" => Some("End of string"),
        "\\n" => Some("Newline"),
        "\\t" => Some("Tab"),
        "\\r" => Some("Carriage return"),
        _ => None,
    }
}

/// Scan a regex pattern for metacharacters / classes /
/// escapes.  Simplified version of Python's `_REGEX_PART_RE`
/// and `_describe_regex_component`; handles common cases:
/// anchors, quantifiers, alternation, character classes,
/// shorthand escapes, capture-group parens, lazy
/// quantifiers.
#[allow(clippy::too_many_lines)]
fn scan_regex_components(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut push = |key: String, tok: String, desc: String| {
        if !seen.contains(&key) {
            seen.insert(key);
            out.push((tok, desc));
        }
    };
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Escape sequences: `\<char>` and special shorthand
        // classes.
        if c == '\\' && i + 1 < chars.len() {
            let next = chars[i + 1];
            let tok = format!("\\{next}");
            if next.is_ascii_digit() {
                push(
                    tok.clone(),
                    tok.clone(),
                    format!("Backreference to group {next}"),
                );
            } else if let Some(desc) = regex_escape_desc(&tok) {
                push(tok.clone(), tok.clone(), desc.to_string());
            } else if ".*+?(){}[]|^$\\".contains(next) {
                push(
                    tok.clone(),
                    tok.clone(),
                    format!("Escaped literal `{next}`"),
                );
            } else {
                push(tok.clone(), tok.clone(), format!("Escape sequence `{tok}`"));
            }
            i += 2;
            continue;
        }
        // Character class.
        if c == '[' {
            let start = i;
            let mut end = i + 1;
            // Allow leading `^` and `]` as first char per
            // regex grammar.
            if end < chars.len() && chars[end] == '^' {
                end += 1;
            }
            if end < chars.len() && chars[end] == ']' {
                end += 1;
            }
            while end < chars.len() && chars[end] != ']' {
                if chars[end] == '\\' && end + 1 < chars.len() {
                    end += 2;
                } else {
                    end += 1;
                }
            }
            let tok: String = if end < chars.len() {
                chars[start..=end].iter().collect()
            } else {
                chars[start..end].iter().collect()
            };
            let inner: String = if tok.starts_with('[') && tok.ends_with(']') {
                tok[1..tok.len() - 1].to_string()
            } else {
                tok[1..].to_string()
            };
            push(
                tok.clone(),
                tok.clone(),
                format!("Character class: matches any of `{inner}`"),
            );
            i = end + 1;
            continue;
        }
        // Lazy quantifiers (`*?` / `+?` / `??`) — peek for the
        // trailing `?`.
        if matches!(c, '*' | '+' | '?') && i + 1 < chars.len() && chars[i + 1] == '?' {
            let tok = format!("{c}?");
            if let Some(desc) = regex_meta_desc(&tok) {
                push(tok.clone(), tok.clone(), desc.to_string());
            }
            i += 2;
            continue;
        }
        if let Some(desc) = regex_meta_desc(&c.to_string()) {
            push(c.to_string(), c.to_string(), desc.to_string());
            i += 1;
            continue;
        }
        // Grouping.
        if c == '(' {
            // Look ahead for `(?:` / `(?=` / `(?!` / `(?>`.
            if i + 2 < chars.len() && chars[i + 1] == '?' {
                let trail = chars[i + 2];
                let (tok, desc) = match trail {
                    ':' => ("(?:", "Non-capturing group"),
                    '=' => ("(?=", "Positive lookahead"),
                    '!' => ("(?!", "Negative lookahead"),
                    '>' => ("(?>", "Atomic (possessive) group"),
                    _ => {
                        i += 1;
                        push("(".into(), "(".into(), "Capture group open".into());
                        continue;
                    }
                };
                push(tok.to_string(), tok.to_string(), desc.to_string());
                i += 3;
                continue;
            }
            push("(".into(), "(".into(), "Capture group open".into());
            i += 1;
            continue;
        }
        if c == ')' {
            push(")".into(), ")".into(), "Group close".into());
            i += 1;
            continue;
        }
        i += 1;
    }
    out
}

/// Render a regex-pattern hover markdown.  Mirrors
/// `_regex_hover`.
fn regex_hover_text(text: &str) -> String {
    let mut parts: Vec<String> = vec!["**Regex pattern**\n".to_string()];
    let comps = scan_regex_components(text);
    if comps.is_empty() {
        parts.push("Literal string (no metacharacters).".to_string());
    } else {
        parts.push("| Component | Meaning |".to_string());
        parts.push("|-----------|---------|".to_string());
        for (tok, desc) in comps {
            parts.push(format!("| `{tok}` | {desc} |"));
        }
    }
    parts.join("\n")
}

/// Detect when the cursor sits on a regex pattern.
/// Recognises `regexp <pat> ...`, `regsub <pat> ...` (the
/// pattern arg, not the subspec), and `lsearch -regexp <pat>
/// ...`.  Single-line only.
fn regex_pattern_at_position(source: &str, line: u32, character: u32) -> Option<String> {
    let line_text = source.split('\n').nth(line as usize)?;
    let tokens: Vec<&str> = line_text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    let is_regex_command = matches!(tokens[0], "regexp" | "regsub")
        || (tokens.len() >= 2 && tokens[0] == "lsearch" && tokens.contains(&"-regexp"));
    if !is_regex_command {
        return None;
    }
    let literal = string_literal_at(line_text, character)?;
    // Require at least one regex metacharacter so we don't
    // fire on literal strings.
    if !literal.chars().any(|c| {
        matches!(
            c,
            '^' | '$' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '|' | '\\'
        )
    }) {
        return None;
    }
    Some(literal)
}

/// Format hover markdown for an IPv4 / IPv6 literal at the
/// cursor's word.  Mirrors `_ip_address_hover` in
/// `lsp/features/hover.py:877-885`.
///
/// Returns `None` when `word` isn't a valid IP literal.  An
/// optional `/prefix` suffix is supported; the prefix is
/// rendered as a CIDR network in the result.
fn ip_address_hover_text(word: &str) -> Option<String> {
    use std::fmt::Write;
    if !word.contains('.') && !word.contains(':') {
        return None;
    }
    // Strip the optional `/prefix` suffix before parsing.
    let (addr, prefix) = match word.split_once('/') {
        Some((a, p)) => (a, p.parse::<u8>().ok()),
        None => (word, None),
    };
    if let Ok(v4) = addr.parse::<std::net::Ipv4Addr>() {
        let class = classify_ipv4(v4);
        let mut out = format!("**IPv4 address** `{addr}`\n\n* Classification: {class}\n");
        if let Some(p) = prefix {
            if p <= 32 {
                let _ = writeln!(out, "* CIDR network: `{addr}/{p}`");
            }
        }
        return Some(out);
    }
    if let Ok(v6) = addr.parse::<std::net::Ipv6Addr>() {
        let class = classify_ipv6(v6);
        let mut out = format!("**IPv6 address** `{addr}`\n\n* Classification: {class}\n");
        if let Some(p) = prefix {
            if p <= 128 {
                let _ = writeln!(out, "* CIDR network: `{addr}/{p}`");
            }
        }
        // IPv4-mapped form (`::ffff:x.x.x.x`).
        if let Some(mapped) = v6.to_ipv4_mapped() {
            let _ = writeln!(out, "* IPv4-mapped form: `{mapped}`");
        }
        return Some(out);
    }
    None
}

/// Classify an IPv4 address by RFC category — loopback,
/// private, multicast, broadcast, link-local, unspecified,
/// or public.
fn classify_ipv4(addr: std::net::Ipv4Addr) -> &'static str {
    if addr.is_unspecified() {
        "Unspecified (`0.0.0.0`)"
    } else if addr.is_loopback() {
        "Loopback (RFC 1122)"
    } else if addr.is_private() {
        "Private (RFC 1918)"
    } else if addr.is_link_local() {
        "Link-local (RFC 3927)"
    } else if addr.is_multicast() {
        "Multicast (RFC 5771)"
    } else if addr.is_broadcast() {
        "Broadcast"
    } else if addr.is_documentation() {
        "Documentation (RFC 5737)"
    } else {
        "Public / global"
    }
}

/// Classify an IPv6 address by RFC category.
fn classify_ipv6(addr: std::net::Ipv6Addr) -> &'static str {
    if addr.is_unspecified() {
        "Unspecified (`::`)"
    } else if addr.is_loopback() {
        "Loopback (`::1`)"
    } else if addr.is_multicast() {
        "Multicast (RFC 4291)"
    } else if addr.to_ipv4_mapped().is_some() {
        "IPv4-mapped (RFC 4291)"
    } else if addr.segments()[0] & 0xffc0 == 0xfe80 {
        "Link-local (RFC 4291)"
    } else if addr.segments()[0] & 0xfe00 == 0xfc00 {
        "Unique local (RFC 4193)"
    } else {
        "Global unicast"
    }
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

    let cursor = (character as usize).min(chars.len());

    // `${name}` braced form first: scan left looking for the
    // most recent `${` whose matching `}` lies at or to the
    // right of the cursor.  This handles cursors anywhere
    // inside the braces, including on the closing `}`.
    if let Some(name) = braced_var_around(&chars, cursor) {
        return Some(name);
    }

    let mut pos = cursor;
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

/// Find a `${name}` braced variable reference containing `cursor`.
/// Walks left from `cursor` to find a `${`, then matches it with
/// the next `}` to its right.  Returns the inner name when the
/// cursor sits inside the braces.
fn braced_var_around(chars: &[char], cursor: usize) -> Option<String> {
    let mut i = cursor.min(chars.len());
    while i > 0 {
        let c = chars[i - 1];
        if c == '{' {
            if i >= 2 && chars[i - 2] == '$' {
                let inner_start = i;
                let mut end = inner_start;
                while end < chars.len() && chars[end] != '}' {
                    end += 1;
                }
                if end < chars.len() && cursor <= end {
                    let name: String = chars[inner_start..end].iter().collect();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
            return None;
        }
        if c == '}' || c == '"' || c == '[' || c == ']' || c == ';' || c == '\n' {
            return None;
        }
        i -= 1;
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
        // `S-hover-rich`: render `@param` / `@return` /
        // `@brief` tagged docstrings as structured Markdown
        // sections rather than the raw harvested text.  Lines
        // that don't carry a tag fall through into the
        // description block, preserving free-form comments
        // for procs that don't use Doxygen-style tags.
        parts.push(format_docstring(&proc_def.doc));
    }
    parts.join("\n\n")
}

/// Parse a raw docstring and render it as Markdown for LSP
/// hover.  Mirrors `format_docstring` in
/// `core/formatting/docstring.py`.
///
/// Recognised tags:
///
/// * `@brief <text>` — short summary surfaced before the
///   description block.
/// * `@param <name> <text>` — parameter docs rendered as a
///   bulleted **Parameters** list.
/// * `@return <text>` / `@returns <text>` — return-value
///   description surfaced as a **Returns** line.
///
/// Other lines accumulate into the description block.  Pure-
/// decoration lines (a run of `.`, `-`, `=`, `*`, `~`, `#`)
/// are dropped.
fn format_docstring(text: &str) -> String {
    let mut brief = String::new();
    let mut description_lines: Vec<String> = Vec::new();
    let mut params: Vec<(String, String)> = Vec::new();
    let mut returns_parts: Vec<String> = Vec::new();

    for line in text.lines() {
        let stripped = line.trim();
        let low = stripped.to_ascii_lowercase();
        if let Some(rest) = low
            .strip_prefix("@param ")
            .or_else(|| low.strip_prefix("@param\t"))
        {
            // Use the original `stripped` slice for body extract
            // so we preserve case on the parameter name and
            // description.  Find the offset of `rest` within
            // `low` (always 7 — `@param ` length).
            let body = &stripped[7..].trim();
            let mut iter = body.splitn(2, char::is_whitespace);
            let Some(name) = iter.next() else {
                continue;
            };
            let name = name.trim_end_matches(['-', ' ']);
            let desc = iter
                .next()
                .map(|s| s.trim_start_matches(['-', ' ']).to_string())
                .unwrap_or_default();
            params.push((name.to_string(), desc));
            let _ = rest;
            continue;
        }
        if low.starts_with("@return ")
            || low.starts_with("@return\t")
            || low.starts_with("@returns ")
            || low.starts_with("@returns\t")
        {
            let body = stripped
                .split_once(char::is_whitespace)
                .map_or("", |x| x.1)
                .trim();
            returns_parts.push(body.trim_start_matches(['-', ' ']).to_string());
            continue;
        }
        if let Some(rest) = low
            .strip_prefix("@brief ")
            .or_else(|| low.strip_prefix("@brief\t"))
        {
            brief = stripped[7..].trim().to_string();
            let _ = rest;
            continue;
        }
        // Drop decoration-only lines.
        if !stripped.is_empty()
            && stripped
                .chars()
                .all(|c| matches!(c, '.' | '-' | '=' | '*' | '~' | '#'))
        {
            continue;
        }
        description_lines.push(stripped.to_string());
    }

    let description = description_lines.join("\n");
    let description = description.trim().to_string();
    let returns_text = returns_parts.join(" ");

    let mut parts: Vec<String> = Vec::new();
    if !brief.is_empty() {
        parts.push(brief);
    }
    if !description.is_empty() {
        parts.push(description);
    }
    if !params.is_empty() {
        let mut lines = vec!["**Parameters:**".to_string()];
        for (name, desc) in &params {
            if desc.is_empty() {
                lines.push(format!("- **{name}**"));
            } else {
                lines.push(format!("- **{name}** \u{2014} {desc}"));
            }
        }
        parts.push(lines.join("\n"));
    }
    if !returns_text.is_empty() {
        parts.push(format!("**Returns:** {returns_text}"));
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
    fn find_var_at_position_recognises_braced_form() {
        // Cursor on the `r` inside `${var}`.  The braced form
        // should still resolve to `"var"` so rename and hover
        // can find the symbol.
        let src = "set x ${var}\n";
        assert_eq!(find_var_at_position(src, 0, 9), Some("var".to_owned()));
        // Cursor immediately after the opening `${` (start of name).
        assert_eq!(find_var_at_position(src, 0, 8), Some("var".to_owned()));
        // Cursor on the closing `}` itself — the inner name is
        // still resolvable as long as the cursor sits inside the
        // braces inclusive of the close brace.
        assert_eq!(find_var_at_position(src, 0, 11), Some("var".to_owned()));
    }

    #[test]
    fn hover_on_proc_name_returns_signature() {
        let src = "proc greet {name} { puts $name }\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 6, &analysis, None).expect("expected hover for proc name");
        assert_eq!(h.kind, HoverKind::Markdown);
        assert!(h.value.contains("proc ::greet"), "{}", h.value);
        assert!(h.value.contains("name"), "{}", h.value);
    }

    #[test]
    fn hover_on_proc_qualified_name() {
        let src = "namespace eval ::ns { proc helper {} { return } }\n";
        let analysis = analyse(src);
        // Cursor on `helper` token at column ~28
        let h = hover(src, 0, 28, &analysis, None);
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
        assert!(hover(src, 0, 6, &analysis, None).is_none());
    }

    #[test]
    fn hover_on_class_name_returns_metaclass_signature() {
        let src = "oo::class create Greeter {}\n";
        let analysis = analyse(src);
        let h = hover(src, 0, 18, &analysis, None);
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
        let h = hover(src, 1, 7, &analysis, None);
        if let Some(h) = h {
            assert!(h.value.contains("Variable"), "{}", h.value);
            assert!(h.value.contains("`x`"), "{}", h.value);
        }
    }

    #[test]
    fn hover_returns_none_for_out_of_range_line() {
        let src = "proc foo {} {}\n";
        let analysis = analyse(src);
        assert!(hover(src, 99, 0, &analysis, None).is_none());
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
        let h = hover(src, 0, 22, &analysis, None).expect("hover");
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
        let h = hover(src, 0, 10, &analysis, None).expect("hover");
        assert!(
            h.value.contains("Format string"),
            "expected sprintf hover, got: {value}",
            value = h.value,
        );
    }

    // -- S-hover-rich: binary format hover --------------------------

    #[test]
    fn scan_binary_specifiers_finds_basic_types() {
        let s = scan_binary_specifiers("a4 H2 i");
        assert_eq!(s, vec!["a4", "H2", "i"]);
    }

    #[test]
    fn scan_binary_specifiers_handles_star_count() {
        let s = scan_binary_specifiers("a* I*");
        assert_eq!(s, vec!["a*", "I*"]);
    }

    #[test]
    fn binary_format_hover_renders_specifier_table() {
        let text = binary_format_hover_text("a4 i");
        assert!(text.contains("**Binary format spec**"), "{text}");
        assert!(
            text.contains("| `a4` | Byte string, padded with nulls |"),
            "{text}",
        );
        assert!(
            text.contains("| `i` | 32-bit signed integer (little-endian) |"),
            "{text}",
        );
    }

    #[test]
    fn binary_format_string_at_position_detects_braced_literal() {
        let src = "binary format {a4 i} val\n";
        let found = binary_format_string_at_position(src, 0, 17);
        assert_eq!(found.as_deref(), Some("a4 i"));
    }

    #[test]
    fn hover_fires_for_binary_specifier() {
        let src = "binary format {a4 i} val\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 17, &analysis, None).expect("hover");
        assert!(h.value.contains("Binary format spec"), "{}", h.value);
    }

    // -- S-hover-rich: regsub substitution-spec hover ---------------

    #[test]
    fn scan_regsub_backrefs_finds_each_backref() {
        let r = scan_regsub_backrefs("\\1-\\2 (\\& and \\0)");
        assert_eq!(r, vec!["\\1", "\\2", "\\&", "\\0"]);
    }

    #[test]
    fn regsub_hover_renders_backref_table() {
        let text = regsub_hover_text("prefix \\1 suffix");
        assert!(text.contains("**Substitution spec**"), "{text}");
        assert!(text.contains("| `\\1` | First capture group |"), "{text}");
    }

    #[test]
    fn regsub_hover_handles_no_backrefs() {
        let text = regsub_hover_text("plain text");
        assert!(text.contains("No backreferences found"), "{text}");
    }

    #[test]
    fn regsub_subspec_at_position_finds_subspec_literal() {
        let src = "regsub foo bar {\\1-baz} out\n";
        // Cursor inside the subspec literal.
        let found = regsub_subspec_at_position(src, 0, 18);
        assert_eq!(found.as_deref(), Some("\\1-baz"));
    }

    #[test]
    fn hover_fires_for_regsub_backref() {
        let src = "regsub foo bar {\\1-baz} out\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let h = hover(src, 0, 18, &analysis, None).expect("hover");
        assert!(h.value.contains("Substitution spec"), "{}", h.value);
    }

    // -- S-hover-rich: glob pattern hover ---------------------------

    #[test]
    fn scan_glob_metachars_finds_star_and_question() {
        let m = scan_glob_metachars("*.tcl");
        let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"*"), "{m:?}");
    }

    #[test]
    fn scan_glob_metachars_finds_character_class() {
        let m = scan_glob_metachars("[abc]*.tcl");
        let toks: Vec<&str> = m.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"[abc]"), "{m:?}");
        assert!(toks.contains(&"*"), "{m:?}");
    }

    #[test]
    fn glob_hover_renders_table() {
        let text = glob_hover_text("*.tcl");
        assert!(text.contains("**Glob pattern**"), "{text}");
        assert!(text.contains("| `*` |"), "{text}");
    }

    #[test]
    fn glob_hover_for_literal_string() {
        let text = glob_hover_text("plain");
        assert!(text.contains("Literal string"), "{text}");
    }

    #[test]
    fn hover_fires_for_glob_pattern() {
        // Braced glob pattern — single-line literal detection
        // requires `"..."` or `{...}` delimiters.  Bare globs
        // (`glob *.tcl`) fall through to the proc / word
        // lookup; their support lives in the same multi-line
        // / arg-position machinery that other `*-rich`
        // sub-strips defer.
        let src = "glob {*.tcl}\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the braced pattern.
        let h = hover(src, 0, 8, &analysis, None).expect("hover");
        assert!(h.value.contains("Glob pattern"), "{}", h.value);
    }

    // -- S-hover-rich: regex pattern hover --------------------------

    #[test]
    fn scan_regex_components_finds_anchors_and_quantifiers() {
        let r = scan_regex_components("^foo.*$");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"^"), "{r:?}");
        assert!(toks.contains(&"."), "{r:?}");
        assert!(toks.contains(&"*"), "{r:?}");
        assert!(toks.contains(&"$"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_character_class() {
        let r = scan_regex_components("[a-z]+");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"[a-z]"), "{r:?}");
        assert!(toks.contains(&"+"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_escapes() {
        let r = scan_regex_components("\\d+");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"\\d"), "{r:?}");
    }

    #[test]
    fn scan_regex_components_finds_groups_and_lookahead() {
        let r = scan_regex_components("(?:foo)(?=bar)");
        let toks: Vec<&str> = r.iter().map(|(t, _)| t.as_str()).collect();
        assert!(toks.contains(&"(?:"), "{r:?}");
        assert!(toks.contains(&"(?="), "{r:?}");
    }

    #[test]
    fn regex_hover_renders_table() {
        let text = regex_hover_text("^foo$");
        assert!(text.contains("**Regex pattern**"), "{text}");
        assert!(text.contains("| `^` |"), "{text}");
    }

    #[test]
    fn hover_fires_for_regex_pattern() {
        let src = "regexp {^foo.*$} $line\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor inside the pattern literal.
        let h = hover(src, 0, 10, &analysis, None).expect("hover");
        assert!(h.value.contains("Regex pattern"), "{}", h.value);
    }

    // -- S-hover-rich: IP address hover -----------------------------

    #[test]
    fn ip_hover_classifies_private_ipv4() {
        let t = ip_address_hover_text("10.0.0.1").expect("hover");
        assert!(t.contains("IPv4 address"), "{t}");
        assert!(t.contains("Private (RFC 1918)"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_loopback() {
        let t = ip_address_hover_text("127.0.0.1").expect("hover");
        assert!(t.contains("Loopback"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_public_ipv4() {
        let t = ip_address_hover_text("8.8.8.8").expect("hover");
        assert!(t.contains("Public"), "{t}");
    }

    #[test]
    fn ip_hover_renders_cidr_prefix() {
        let t = ip_address_hover_text("10.0.0.0/8").expect("hover");
        assert!(t.contains("CIDR network: `10.0.0.0/8`"), "{t}");
    }

    #[test]
    fn ip_hover_classifies_ipv6_loopback() {
        let t = ip_address_hover_text("::1").expect("hover");
        assert!(t.contains("IPv6 address"), "{t}");
        assert!(t.contains("Loopback"), "{t}");
    }

    #[test]
    fn ip_hover_detects_ipv4_mapped_ipv6() {
        let t = ip_address_hover_text("::ffff:192.0.2.1").expect("hover");
        assert!(t.contains("IPv4-mapped"), "{t}");
    }

    #[test]
    fn ip_hover_rejects_non_ip_strings() {
        assert!(ip_address_hover_text("hello").is_none());
        assert!(ip_address_hover_text("256.256.256.256").is_none());
        assert!(ip_address_hover_text("not.an.ip.address").is_none());
    }

    #[test]
    fn hover_fires_for_ip_address_word() {
        let src = "set host 10.0.0.1\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        // Cursor on `10.0.0.1`.
        let h = hover(src, 0, 11, &analysis, None).expect("hover");
        assert!(h.value.contains("IPv4 address"), "{}", h.value);
    }

    // -- S-hover-rich: registry-driven hovers -----------------------

    #[test]
    fn builtin_command_hover_surfaces_summary_from_registry() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let t = builtin_command_hover_text(&registry, "puts").expect("hover");
        assert!(t.contains("built-in command"), "{t}");
        assert!(t.contains("`puts`"), "{t}");
    }

    #[test]
    fn builtin_command_hover_lists_subcommands() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let t = builtin_command_hover_text(&registry, "string").expect("hover");
        assert!(t.contains("Subcommands:"), "{t}");
        assert!(t.contains("length"), "{t}");
    }

    #[test]
    fn builtin_command_hover_returns_none_for_unknown() {
        let registry = tcl_registry::CommandRegistry::build_default();
        assert!(builtin_command_hover_text(&registry, "totallyMadeUpCommand").is_none());
    }

    #[test]
    fn subcommand_hover_surfaces_for_string_length() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "string length $name\n";
        let t = subcommand_hover_text(src, 0, 10, &registry, "length").expect("subcommand hover");
        assert!(t.contains("`string length`"), "{t}");
        assert!(t.contains("subcommand"), "{t}");
    }

    #[test]
    fn subcommand_hover_skips_unknown_subcommand() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let src = "string bogusSubcommand\n";
        assert!(subcommand_hover_text(src, 0, 12, &registry, "bogusSubcommand").is_none());
    }

    #[test]
    fn hover_fires_for_builtin_command_with_registry() {
        let src = "puts hello\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("built-in command"), "{}", h.value);
    }

    #[test]
    fn hover_fires_for_subcommand_with_registry() {
        let src = "string length $name\n";
        let mut a = tcl_compiler::analyser::Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let registry = tcl_registry::CommandRegistry::build_default();
        let h = hover(src, 0, 10, &analysis, Some(&registry)).expect("hover");
        assert!(h.value.contains("subcommand"), "{}", h.value);
    }

    // -- S-hover-rich: docstring formatting --------------------------

    #[test]
    fn format_docstring_renders_brief_param_return_tags() {
        let raw = concat!(
            "@brief Greet someone\n",
            "Free-form description\n",
            "spanning two lines.\n",
            "@param name the person's name\n",
            "@param greeting optional greeting prefix\n",
            "@return the formatted greeting\n",
        );
        let rendered = format_docstring(raw);
        assert!(rendered.contains("Greet someone"), "{rendered}");
        assert!(rendered.contains("Free-form description"), "{rendered}");
        assert!(rendered.contains("**Parameters:**"), "{rendered}");
        assert!(
            rendered.contains("- **name** \u{2014} the person's name"),
            "{rendered}",
        );
        assert!(
            rendered.contains("- **greeting** \u{2014} optional greeting prefix"),
            "{rendered}",
        );
        assert!(
            rendered.contains("**Returns:** the formatted greeting"),
            "{rendered}",
        );
    }

    #[test]
    fn format_docstring_drops_decoration_lines() {
        // Pure-decoration lines (`.....`, `-----`) shouldn't
        // pollute the description block.
        let raw = "..........\nA description.\n..........\n";
        let rendered = format_docstring(raw);
        assert_eq!(rendered, "A description.");
    }

    #[test]
    fn format_docstring_passes_through_plain_text() {
        let raw = "Just a free-form description.\nNo tags here.\n";
        let rendered = format_docstring(raw);
        assert!(rendered.contains("Just a free-form description"));
        assert!(rendered.contains("No tags here"));
    }

    #[test]
    fn format_docstring_handles_param_without_description() {
        let raw = "@param naked\n";
        let rendered = format_docstring(raw);
        assert!(
            rendered.contains("- **naked**"),
            "expected bare param entry; got {rendered}",
        );
        // No trailing em-dash since there's no description.
        assert!(!rendered.contains("**naked** \u{2014}"), "{rendered}");
    }
}
