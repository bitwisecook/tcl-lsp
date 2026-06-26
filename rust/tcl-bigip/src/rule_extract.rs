//! Locate and extract `ltm rule` / `gtm rule` blocks in BIG-IP configs.
//!
//! Port of the Python `dialects.f5.bigip.rule_extract` module. Used to
//! detect whether a file is a tmsh config wrapping iRules and to pull out
//! each embedded `when EVENT { … }` rule body together with its byte/line
//! ranges, so an embedded iRule can be opened in a scratch editor with the
//! `f5-irules` dialect and written back to the original config file.
//!
//! The data layer ([`crate::parser::parse_bigip_conf`]) already models
//! `ltm rule` / `gtm rule` stanzas as typed objects, but its
//! [`crate::range::Range`] convention differs from the Python
//! `EmbeddedRule` (its range starts at the opening `{` and ends one past
//! the closing `}`, and its `source` is the trimmed body). This module
//! reproduces the Python `EmbeddedRule` offset semantics exactly: the
//! range spans the whole stanza (`ltm rule …` header through the closing
//! `}` inclusive) and explicit `*_offset` fields locate the braces and
//! body.
//!
//! Offsets are byte offsets into `source`. Only ASCII structural bytes
//! (`{`, `}`, `"`, `\`) drive the scan, so every recorded offset lands on
//! a UTF-8 char boundary (matching the rest of the parser). For ASCII
//! sources — every BIG-IP config in practice — byte offsets coincide with
//! the Python code-point offsets.

use tcl_lexer::LineIndex;

use crate::range::Range;

/// An iRule block found inside a BIG-IP configuration file.
///
/// Mirrors the Python `EmbeddedRule` dataclass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedRule {
    /// Full object path, e.g. `/Common/my_irule`.
    pub full_path: String,
    /// Short name (final path segment), e.g. `my_irule`.
    pub name: String,
    /// Stanza header, e.g. `ltm rule /Common/my_irule`.
    pub header: String,
    /// The Tcl source between the outermost `{ }` (braces excluded).
    pub body: String,
    /// Byte offset of the first char after the opening `{`.
    pub body_start_offset: usize,
    /// Byte offset of the last char before the closing `}`.
    pub body_end_offset: usize,
    /// Byte offset of the opening `{`.
    pub block_start_offset: usize,
    /// Byte offset one past the closing `}`.
    pub block_end_offset: usize,
    /// Range covering the entire stanza (header start through the closing
    /// `}`, inclusive).
    pub range: Range,
}

/// `true` when `source` contains at least one `ltm rule` or `gtm rule`
/// stanza header (a conf-wrapped iRules file).
///
/// Matches the Python regex `^((?:ltm|gtm)\s+rule\s+(/[\w/.-]+))\s*\{`
/// with multiline semantics: a header begins at the start of a line.
#[must_use]
pub fn is_conf_wrapped_irules(source: &str) -> bool {
    !find_rule_headers(source).is_empty()
}

/// Find all `ltm rule` and `gtm rule` blocks in `source`.
///
/// Brace-balanced extraction that skips braces inside double-quoted
/// strings and honours backslash escapes — a faithful port of the Python
/// `find_embedded_rules`.
#[must_use]
pub fn find_embedded_rules(source: &str) -> Vec<EmbeddedRule> {
    let line_index = LineIndex::new(source);
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut rules = Vec::new();

    for header_match in find_rule_headers(source) {
        let RuleHeader {
            header,
            full_path,
            header_start,
            brace_pos,
        } = header_match;
        let name = full_path
            .rsplit('/')
            .next()
            .unwrap_or(&full_path)
            .to_owned();

        // Scan from just after the opening `{`, tracking brace depth.
        let mut pos = brace_pos + 1;
        let mut depth: i32 = 1;
        while pos < len && depth > 0 {
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'"' => {
                    // Skip a quoted string — braces inside quotes are not
                    // structural.
                    pos += 1;
                    while pos < len && bytes[pos] != b'"' {
                        if bytes[pos] == b'\\' {
                            pos += 1; // skip escaped char
                        }
                        pos += 1;
                    }
                }
                b'\\' => pos += 1, // skip escaped char
                _ => {}
            }
            pos += 1;
        }

        // `pos` now sits one past the closing `}`. The body excludes both
        // braces; `pos - 1` is the closing `}`.
        let block_end_offset = pos;
        let body_start_offset = brace_pos + 1;
        let body_end_offset = pos.saturating_sub(1);
        let body = source
            .get(body_start_offset..body_end_offset)
            .unwrap_or("")
            .to_owned();

        let range =
            Range::from_offsets(source, &line_index, header_start, block_end_offset.saturating_sub(1));

        rules.push(EmbeddedRule {
            full_path,
            name,
            header,
            body,
            body_start_offset,
            body_end_offset,
            block_start_offset: header_start,
            block_end_offset,
            range,
        });
    }

    rules
}

/// Return the [`EmbeddedRule`] whose block contains `offset`, or `None`.
///
/// The block span is inclusive of both endpoints
/// (`block_start_offset <= offset <= block_end_offset`), matching the
/// Python `find_rule_at_offset`.
#[must_use]
pub fn find_rule_at_offset(source: &str, offset: usize) -> Option<EmbeddedRule> {
    find_embedded_rules(source)
        .into_iter()
        .find(|rule| rule.block_start_offset <= offset && offset <= rule.block_end_offset)
}

/// Return `source` with the body of `rule` replaced by `new_body`.
///
/// Port of the Python `replace_rule_body`.
#[must_use]
pub fn replace_rule_body(source: &str, rule: &EmbeddedRule, new_body: &str) -> String {
    let head = source.get(..rule.body_start_offset).unwrap_or("");
    let tail = source.get(rule.body_end_offset..).unwrap_or("");
    format!("{head}{new_body}{tail}")
}

/// A located `ltm rule` / `gtm rule` header.
struct RuleHeader {
    /// The header text up to (but excluding) the opening `{`, trimmed of
    /// trailing whitespace — e.g. `ltm rule /Common/my_irule`.
    header: String,
    /// The matched full object path — e.g. `/Common/my_irule`.
    full_path: String,
    /// Byte offset of the start of the header (the `l` of `ltm` / `g` of
    /// `gtm`), at column 0 of its line.
    header_start: usize,
    /// Byte offset of the opening `{`.
    brace_pos: usize,
}

/// Locate every `(?:ltm|gtm) rule /path {` header that begins at the start
/// of a line, returning the header text, path, header start offset, and
/// opening-brace offset.
///
/// This is the structural equivalent of the Python `_RULE_HEADER_RE`
/// (`^((?:ltm|gtm)\s+rule\s+(/[\w/.-]+))\s*\{`, `re.MULTILINE`). The
/// project never parses Tcl/tmsh with a regex engine in the hot path, so
/// the match is performed by a small hand-rolled scanner with identical
/// semantics. Path characters are `[\w/.-]` (word chars, `/`, `.`, `-`).
fn find_rule_headers(source: &str) -> Vec<RuleHeader> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut line_start = 0usize;

    while line_start < len {
        if let Some(h) = try_match_header(bytes, source, line_start) {
            out.push(h);
        }
        // Advance to the start of the next line.
        match source[line_start..].find('\n') {
            Some(rel) => line_start += rel + 1,
            None => break,
        }
    }

    out
}

/// Try to match a rule header anchored at byte `start` (a line start).
fn try_match_header(bytes: &[u8], source: &str, start: usize) -> Option<RuleHeader> {
    let len = bytes.len();
    let mut pos = start;

    // `(?:ltm|gtm)` — exactly at the line start (no leading whitespace, to
    // mirror the `^` anchor immediately preceding the alternation).
    let module = source.get(pos..pos + 3)?;
    if module != "ltm" && module != "gtm" {
        return None;
    }
    pos += 3;

    // `\s+`
    let after_ws = skip_ws(bytes, pos);
    if after_ws == pos {
        return None;
    }
    pos = after_ws;

    // `rule`
    if source.get(pos..pos + 4)? != "rule" {
        return None;
    }
    pos += 4;

    // `\s+`
    let after_ws2 = skip_ws(bytes, pos);
    if after_ws2 == pos {
        return None;
    }
    pos = after_ws2;

    // `(/[\w/.-]+)` — must start with `/`, then one or more path chars.
    if pos >= len || bytes[pos] != b'/' {
        return None;
    }
    let path_start = pos;
    pos += 1;
    let body_chars_start = pos;
    while pos < len && is_path_char(bytes[pos]) {
        pos += 1;
    }
    if pos == body_chars_start {
        // `/` with no following path chars — `+` requires at least one.
        return None;
    }
    let full_path = source[path_start..pos].to_owned();
    let header_end = pos;

    // `\s*\{`
    pos = skip_ws(bytes, pos);
    if pos >= len || bytes[pos] != b'{' {
        return None;
    }

    Some(RuleHeader {
        header: source[start..header_end].to_owned(),
        full_path,
        header_start: start,
        brace_pos: pos,
    })
}

/// Advance past ASCII/Unicode whitespace bytes (`\s` in the regex covers
/// space, tab, newline, carriage return, form feed, vertical tab).
fn skip_ws(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x0b) {
        pos += 1;
    }
    pos
}

/// `true` for a `[\w/.-]` path byte: ASCII word char (`[A-Za-z0-9_]`),
/// `/`, `.`, or `-`.
fn is_path_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'/' | b'.' | b'-')
}
