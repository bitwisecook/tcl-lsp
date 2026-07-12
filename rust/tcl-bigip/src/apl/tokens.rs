// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lexical tokeniser for F5 iApp APL (Application Presentation Language),
//! for syntax highlighting.
//!
//! [`parse_apl`](super::parse_apl) is a *structural* extractor: it yields the
//! declared sections / tables / fields with a range on each **name** only.
//! Highlighting needs a type for every lexeme — keywords, attributes,
//! validators, strings, escapes, numbers — so this module is a separate,
//! self-contained lexer over the same grammar.
//!
//! # Non-overlapping by construction
//!
//! The LSP spec forbids overlapping semantic tokens.  This lexer scans each
//! line into lexemes **once** and classifies each lexeme at most once, so no
//! two emitted tokens can share a byte.  Where a nested construct needs its
//! own type — an escape inside a string, a validator name inside quotes — the
//! enclosing string is *split* into fragments around it rather than being
//! overlaid (see [`push_string`]).
//!
//! This is the one behaviour that must not be reintroduced from the retired
//! Python implementation, which ran each of its regexes independently over the
//! line and appended every match: `define choice x` matched both its
//! `define` and its `field-type` rule, stacking two tokens on the same span.

use std::sync::LazyLock;

use regex::Regex;

/// Semantic classification of an APL source lexeme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AplTokenKind {
    /// A `#`-introduced comment line (but not `#include` / `#inline`).
    Comment,
    /// A preprocessor directive: `#include`, `#inline`.
    Directive,
    /// A block-introducing keyword: `section`, `text`, `table`, `row`.
    SectionKw,
    /// A field-type keyword: `string`, `choice`, `password`, …
    FieldType,
    /// The `define` keyword.
    Define,
    /// The name bound by a `define`.
    DefineName,
    /// The `optional` keyword (guards a conditional block).
    Optional,
    /// A field attribute: `default`, `display`, `required`, `validator`.
    Attribute,
    /// The name following a `section` / `text` / `table` / `row` keyword.
    SectionName,
    /// The name following a field-type keyword.
    FieldName,
    /// A `$var` / `${var}` substitution.
    Variable,
    /// A quoted-string fragment.
    Str,
    /// A numeric literal.
    Number,
    /// The `=>` operator.
    Operator,
    /// A backslash escape inside a string.
    Escape,
    /// A known validator name inside a `validator "…"` value.
    Validator,
}

/// A single lexical token, as absolute byte offsets into the source.
///
/// `end` is exclusive.  Byte offsets (not columns) so the LSP layer can do the
/// UTF-16 conversion with the same `LineIndex` it uses for every other
/// feature — the structural [`parse_apl`](super::parse_apl) counts *code
/// points* instead, which is wrong for non-ASCII sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AplToken {
    /// Absolute byte offset of the first byte.
    pub start: u32,
    /// Absolute byte offset one past the last byte.
    pub end: u32,
    /// What the lexeme is.
    pub kind: AplTokenKind,
}

/// Keywords that introduce a named block.
const SECTION_KEYWORDS: &[&str] = &["section", "text", "table", "row"];

/// Field-type keywords.  A field declaration is `<field-type> <name> …`.
const FIELD_TYPE_KEYWORDS: &[&str] = &[
    "addrport",
    "choice",
    "disena",
    "editchoice",
    "enadis",
    "enadisdry",
    "falsetrue",
    "indefint",
    "message",
    "multichoice",
    "noyes",
    "password",
    "string",
    "tcpprof",
    "truefalse",
    "yesno",
];

/// Attribute keywords that modify a field declaration.
const ATTRIBUTE_KEYWORDS: &[&str] = &["default", "display", "required", "validator"];

/// Validator names APL ships with, recognised inside `validator "…"`.
const VALIDATOR_NAMES: &[&str] = &[
    "FQDN",
    "IpAddress",
    "IpOrFqdn",
    "NonNegativeNumber",
    "Number",
    "PortNumber",
];

/// A bare numeric literal: decimal, float, or hex.
static NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:0[xX][0-9a-fA-F]+|[+-]?(?:[0-9]*\.)?[0-9]+f?)$")
        .expect("APL number pattern is valid")
});

/// One lexeme of a line, as byte offsets relative to the source.
#[derive(Debug, Clone, Copy)]
struct Lexeme {
    start: usize,
    end: usize,
    class: LexClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexClass {
    /// A bare word: identifier, keyword, or number.
    Word,
    /// A `"…"` quoted string (including both delimiters; may be unterminated).
    Quoted,
    /// A `$var` / `${var}` substitution.
    Var,
    /// `=>`.
    Arrow,
    /// A structural delimiter: `{`, `}`, `(`, `)`, `;`, `,`.
    Punct,
}

/// What the next bare word on the line is expected to be, given the keyword
/// that opened the statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pending {
    None,
    /// The type word of a `define <type> <name>`.
    DefineType,
    /// The name word of a `define <type> <name>`.
    DefineName,
    /// The name after `section` / `text` / `table` / `row`.
    SectionName,
    /// The name after a field-type keyword.
    FieldName,
}

/// The byte spans of the **embedded Tcl** regions in `source` — the *content* of
/// each top-level `[ … ]` bracket expression, outside comments and strings.
///
/// APL embeds Tcl in bracket expressions, and the KCS feature doc is explicit
/// that they "receive full Tcl semantic highlighting".  [`tokenise_apl`] emits
/// nothing inside these spans, leaving them for the caller to walk as Tcl.
#[must_use]
pub fn embedded_tcl_regions(source: &str) -> Vec<(u32, u32)> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let (mut i, mut depth, mut start) = (0usize, 0u32, 0usize);
    let (mut in_comment, mut in_string) = (false, false);
    let mut line_start = true;

    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\n' {
            in_comment = false;
            in_string = false;
            line_start = true;
            i += 1;
            continue;
        }
        if in_comment {
            i += 1;
            continue;
        }
        // A `#` line is a comment (but `#include` / `#inline` are directives, and
        // neither carries a bracket expression, so treating them as comments here
        // is harmless).
        if line_start && c == b'#' {
            in_comment = true;
            i += 1;
            continue;
        }
        if !c.is_ascii_whitespace() {
            line_start = false;
        }
        if c == b'\\' && i + 1 < bytes.len() {
            i += 2;
            continue;
        }
        if c == b'"' {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if !in_string {
            if c == b'[' {
                if depth == 0 {
                    start = i + 1;
                }
                depth += 1;
            } else if c == b']' && depth > 0 {
                depth -= 1;
                if depth == 0 && i > start {
                    spans.push((
                        u32::try_from(start).unwrap_or(0),
                        u32::try_from(i).unwrap_or(0),
                    ));
                }
            }
        }
        i += 1;
    }
    spans
}

/// Tokenise APL source for highlighting.
///
/// The returned tokens are sorted by `start` and are guaranteed
/// **non-overlapping**.  Embedded Tcl regions (see [`embedded_tcl_regions`]) are
/// left empty for the caller to walk as Tcl.
#[must_use]
pub fn tokenise_apl(source: &str) -> Vec<AplToken> {
    let embedded = embedded_tcl_regions(source);
    let mut out: Vec<AplToken> = Vec::new();
    let mut line_start = 0usize;

    for line in source.split('\n') {
        // A line overlapping an embedded `[ … ]` region is Tcl, not APL — leave
        // it to the caller's Tcl walk rather than mis-typing it as APL.
        let (ls, le) = (line_start, line_start + line.len());
        let embedded_here = embedded
            .iter()
            .any(|&(s, e)| ls < e as usize && le > s as usize);
        if !embedded_here {
            tokenise_line(source, line, line_start, &mut out);
        }
        // `+ 1` for the '\n' that `split` consumed.  A trailing '\r' (CRLF) is
        // whitespace to the lexer, so it never lands inside a token.
        line_start += line.len() + 1;
    }

    debug_assert!(
        out.windows(2).all(|w| w[0].end <= w[1].start),
        "APL tokens must be sorted and non-overlapping"
    );
    out
}

/// Tokenise one line, appending to `out`.  `base` is the byte offset of the
/// line within `source`.
fn tokenise_line(source: &str, line: &str, base: usize, out: &mut Vec<AplToken>) {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return;
    }

    // A `#` line is a comment unless it is a directive.  Emitted whole, and no
    // other rule runs on it — so a `#` comment can never overlap anything.
    if trimmed.starts_with('#')
        && !trimmed.starts_with("#include")
        && !trimmed.starts_with("#inline")
    {
        let indent = line.len() - trimmed.len();
        push(
            out,
            base + indent,
            base + line.trim_end().len(),
            AplTokenKind::Comment,
        );
        return;
    }

    let lexemes = lex_line(line, base);
    let mut pending = Pending::None;
    // A keyword only opens a statement at the start of one — so a field-type
    // word used as a *value* (`default string`) is not mistaken for a
    // declaration.
    let mut at_stmt_start = true;
    let mut idx = 0usize;

    while idx < lexemes.len() {
        let lx = lexemes[idx];
        let text = &source[lx.start..lx.end];

        // Structural punctuation resets the statement state.
        if lx.class == LexClass::Punct {
            // `optional (` — the keyword was emitted when we saw it.
            pending = Pending::None;
            at_stmt_start = matches!(text, "{" | "}" | ";");
            idx += 1;
            continue;
        }

        // A pending name slot consumes the next *bare word* only.  Anything
        // else (a quoted string after `message`, a `{` after `text`) cancels
        // it — which is why `message "Welcome…"` no longer mis-types the
        // opening `"Welcome` as a field name, as the Python version did.
        if pending != Pending::None {
            if take_pending_name(&mut pending, lx, text, out) {
                at_stmt_start = false;
                idx += 1;
                continue;
            }
            pending = Pending::None;
        }

        match lx.class {
            LexClass::Quoted => push_string(source, lx.start, lx.end, out),
            LexClass::Var => push(out, lx.start, lx.end, AplTokenKind::Variable),
            LexClass::Arrow => push(out, lx.start, lx.end, AplTokenKind::Operator),
            LexClass::Word => {
                if at_stmt_start && text == "define" {
                    push(out, lx.start, lx.end, AplTokenKind::Define);
                    pending = Pending::DefineType;
                } else if at_stmt_start && SECTION_KEYWORDS.contains(&text) {
                    push(out, lx.start, lx.end, AplTokenKind::SectionKw);
                    pending = Pending::SectionName;
                } else if at_stmt_start && FIELD_TYPE_KEYWORDS.contains(&text) {
                    push(out, lx.start, lx.end, AplTokenKind::FieldType);
                    pending = Pending::FieldName;
                } else if text == "optional"
                    && lexemes
                        .get(idx + 1)
                        .is_some_and(|n| &source[n.start..n.end] == "(")
                {
                    push(out, lx.start, lx.end, AplTokenKind::Optional);
                } else if ATTRIBUTE_KEYWORDS.contains(&text) {
                    push(out, lx.start, lx.end, AplTokenKind::Attribute);
                    // `validator "IpAddress"` — split the quoted value so the
                    // validator name gets its own type without overlaying the
                    // string.
                    if text == "validator"
                        && let Some(next) = lexemes.get(idx + 1)
                        && next.class == LexClass::Quoted
                        && push_validator_value(source, next.start, next.end, out)
                    {
                        idx += 2;
                        at_stmt_start = false;
                        continue;
                    }
                } else if text.starts_with("#include") || text.starts_with("#inline") {
                    push(out, lx.start, lx.end, AplTokenKind::Directive);
                } else if NUMBER_RE.is_match(text) {
                    push(out, lx.start, lx.end, AplTokenKind::Number);
                }
            }
            LexClass::Punct => unreachable!("punctuation handled above"),
        }

        at_stmt_start = false;
        idx += 1;
    }
}

/// Fill the pending name slot from `lx`, if it can.
///
/// Returns `true` when the lexeme was consumed as the pending name.  A slot is
/// only ever filled by a *bare word*: anything else (a quoted value, a `{`)
/// leaves it unconsumed so the caller cancels the slot and falls back to the
/// general rules.
fn take_pending_name(
    pending: &mut Pending,
    lx: Lexeme,
    text: &str,
    out: &mut Vec<AplToken>,
) -> bool {
    if lx.class != LexClass::Word {
        return false;
    }
    match *pending {
        Pending::DefineType => {
            push(out, lx.start, lx.end, AplTokenKind::FieldType);
            *pending = Pending::DefineName;
            true
        }
        Pending::DefineName => {
            push(out, lx.start, lx.end, AplTokenKind::DefineName);
            *pending = Pending::None;
            true
        }
        Pending::SectionName => {
            push(out, lx.start, lx.end, AplTokenKind::SectionName);
            *pending = Pending::None;
            true
        }
        // An attribute keyword in the name slot means the field was declared
        // without a name — let the general rules type it as an attribute.
        Pending::FieldName if !ATTRIBUTE_KEYWORDS.contains(&text) => {
            push(out, lx.start, lx.end, AplTokenKind::FieldName);
            *pending = Pending::None;
            true
        }
        Pending::FieldName | Pending::None => false,
    }
}

/// Split a line into lexemes.  Offsets are absolute (`base` + position).
fn lex_line(line: &str, base: usize) -> Vec<Lexeme> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        let start = i;
        let class = match c {
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    match bytes[i] {
                        b'\\' if i + 1 < bytes.len() => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
                LexClass::Quoted
            }
            b'$' => {
                i += 1;
                if i < bytes.len() && bytes[i] == b'{' {
                    while i < bytes.len() && bytes[i] != b'}' {
                        i += 1;
                    }
                    // Include the closing `}` — the whole `${var}` is one token.
                    if i < bytes.len() {
                        i += 1;
                    }
                } else {
                    while i < bytes.len()
                        && (bytes[i].is_ascii_alphanumeric()
                            || bytes[i] == b'_'
                            || bytes[i] == b':')
                    {
                        i += 1;
                    }
                    // An array subscript is part of the reference: `$foo(bar)`.
                    if i < bytes.len() && bytes[i] == b'(' {
                        while i < bytes.len() && bytes[i] != b')' {
                            i += 1;
                        }
                        if i < bytes.len() {
                            i += 1;
                        }
                    }
                }
                LexClass::Var
            }
            b'=' if bytes.get(i + 1) == Some(&b'>') => {
                i += 2;
                LexClass::Arrow
            }
            b'{' | b'}' | b'(' | b')' | b';' | b',' => {
                i += 1;
                LexClass::Punct
            }
            _ => {
                while i < bytes.len()
                    && !bytes[i].is_ascii_whitespace()
                    && !matches!(bytes[i], b'{' | b'}' | b'(' | b')' | b';' | b',' | b'"')
                {
                    i += 1;
                }
                LexClass::Word
            }
        };

        out.push(Lexeme {
            start: base + start,
            end: base + i,
            class,
        });
    }

    out
}

/// Emit a quoted string as `Str` fragments split around its backslash escapes,
/// so the escapes carry their own type without overlapping the string.
fn push_string(source: &str, start: usize, end: usize, out: &mut Vec<AplToken>) {
    let text = &source[start..end];
    let bytes = text.as_bytes();
    let mut frag = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'\\' || i + 1 >= bytes.len() {
            i += 1;
            continue;
        }
        // The escape's extent: \ooo, \xHH…, \uHHHH, or a single char.
        let mut esc_end = i + 2;
        match bytes[i + 1] {
            b'0'..=b'7' => {
                while esc_end < bytes.len()
                    && esc_end < i + 4
                    && bytes[esc_end].is_ascii_digit()
                    && bytes[esc_end] < b'8'
                {
                    esc_end += 1;
                }
            }
            b'x' => {
                while esc_end < bytes.len() && bytes[esc_end].is_ascii_hexdigit() {
                    esc_end += 1;
                }
            }
            b'u' => {
                while esc_end < bytes.len() && esc_end < i + 6 && bytes[esc_end].is_ascii_hexdigit()
                {
                    esc_end += 1;
                }
            }
            _ => {}
        }

        if frag < i {
            push(out, start + frag, start + i, AplTokenKind::Str);
        }
        push(out, start + i, start + esc_end, AplTokenKind::Escape);
        i = esc_end;
        frag = esc_end;
    }

    if frag < bytes.len() {
        push(out, start + frag, start + bytes.len(), AplTokenKind::Str);
    }
}

/// Emit a `validator "Name"` value as `"` + validator name + `"`, three
/// non-overlapping tokens.  Returns `false` (emitting nothing) when the quoted
/// value is not a known validator name, so the caller falls back to the plain
/// string path.
fn push_validator_value(source: &str, start: usize, end: usize, out: &mut Vec<AplToken>) -> bool {
    let text = &source[start..end];
    let inner = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or_default();
    if !VALIDATOR_NAMES.contains(&inner) {
        push_string(source, start, end, out);
        return true;
    }
    push(out, start, start + 1, AplTokenKind::Str);
    push(out, start + 1, end - 1, AplTokenKind::Validator);
    push(out, end - 1, end, AplTokenKind::Str);
    true
}

/// Append a token, skipping empty spans.
fn push(out: &mut Vec<AplToken>, start: usize, end: usize, kind: AplTokenKind) {
    if end <= start {
        return;
    }
    out.push(AplToken {
        start: u32::try_from(start).unwrap_or(u32::MAX),
        end: u32::try_from(end).unwrap_or(u32::MAX),
        kind,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render tokens as `(text, kind)` for readable assertions.
    fn toks(src: &str) -> Vec<(&str, AplTokenKind)> {
        tokenise_apl(src)
            .into_iter()
            .map(|t| (&src[t.start as usize..t.end as usize], t.kind))
            .collect()
    }

    /// The invariant the retired Python tokeniser violated: it ran each regex
    /// independently and appended every match, so `define choice x` stacked a
    /// `FieldType` on top of the `define` rule's own tokens.  110 overlapping
    /// pairs in `samples/apl/example.apl` alone.
    fn assert_no_overlap(src: &str) {
        let t = tokenise_apl(src);
        for w in t.windows(2) {
            assert!(
                w[0].end <= w[1].start,
                "overlapping tokens in {src:?}: {:?} then {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn define_emits_keyword_type_and_name_once_each() {
        assert_eq!(
            toks("define choice yesno_choice {"),
            vec![
                ("define", AplTokenKind::Define),
                ("choice", AplTokenKind::FieldType),
                ("yesno_choice", AplTokenKind::DefineName),
            ]
        );
        assert_no_overlap("define choice yesno_choice {");
    }

    #[test]
    fn section_and_field_declarations() {
        assert_eq!(
            toks("section intro {"),
            vec![
                ("section", AplTokenKind::SectionKw),
                ("intro", AplTokenKind::SectionName),
            ]
        );
        assert_eq!(
            toks("    string addr required"),
            vec![
                ("string", AplTokenKind::FieldType),
                ("addr", AplTokenKind::FieldName),
                ("required", AplTokenKind::Attribute),
            ]
        );
    }

    /// A quoted value after a field type is a string, not the field's name.
    /// The Python version's `\S+` name group swallowed the opening quote and
    /// emitted `"Welcome` as a `FIELD_NAME` overlapping the string.
    #[test]
    fn quoted_value_after_field_type_is_not_a_field_name() {
        assert_eq!(
            toks(r#"    message "Welcome to the example iApp template.""#),
            vec![
                ("message", AplTokenKind::FieldType),
                (
                    r#""Welcome to the example iApp template.""#,
                    AplTokenKind::Str
                ),
            ]
        );
        assert_no_overlap(r#"    message "Welcome""#);
    }

    #[test]
    fn comments_and_directives_are_distinguished() {
        assert_eq!(
            toks("# a plain comment"),
            vec![("# a plain comment", AplTokenKind::Comment)]
        );
        assert_eq!(
            toks(r#"#include "f5.apl_common""#),
            vec![
                ("#include", AplTokenKind::Directive),
                (r#""f5.apl_common""#, AplTokenKind::Str),
            ]
        );
    }

    #[test]
    fn validator_value_splits_out_of_the_string() {
        assert_eq!(
            toks(r#"    string addr validator "IpAddress""#),
            vec![
                ("string", AplTokenKind::FieldType),
                ("addr", AplTokenKind::FieldName),
                ("validator", AplTokenKind::Attribute),
                ("\"", AplTokenKind::Str),
                ("IpAddress", AplTokenKind::Validator),
                ("\"", AplTokenKind::Str),
            ]
        );
        assert_no_overlap(r#"string addr validator "IpAddress""#);
    }

    /// An unknown validator name stays an ordinary string.
    #[test]
    fn unknown_validator_value_is_a_plain_string() {
        assert_eq!(
            toks(r#"validator "NotAValidator""#),
            vec![
                ("validator", AplTokenKind::Attribute),
                (r#""NotAValidator""#, AplTokenKind::Str),
            ]
        );
    }

    #[test]
    fn escapes_split_the_string_rather_than_overlaying_it() {
        assert_eq!(
            toks(r#"display "a\nb""#),
            vec![
                ("display", AplTokenKind::Attribute),
                ("\"a", AplTokenKind::Str),
                (r"\n", AplTokenKind::Escape),
                ("b\"", AplTokenKind::Str),
            ]
        );
        assert_no_overlap(r#"display "a\nb""#);
    }

    #[test]
    fn optional_guard_and_arrow_operator() {
        let t = toks("optional ( basic.addr => \"yes\" )");
        assert_eq!(t[0], ("optional", AplTokenKind::Optional));
        assert!(t.contains(&("=>", AplTokenKind::Operator)));
    }

    #[test]
    fn variables_and_numbers() {
        assert_eq!(
            toks("default 8080"),
            vec![
                ("default", AplTokenKind::Attribute),
                ("8080", AplTokenKind::Number),
            ]
        );
        let t = toks("default ${basic.addr}");
        assert_eq!(t[1], ("${basic.addr}", AplTokenKind::Variable));
    }

    /// A field-type word used as a *value* is not a declaration.
    #[test]
    fn field_type_keyword_as_a_value_is_not_a_declaration() {
        let t = toks("    default string");
        assert_eq!(
            t,
            vec![("default", AplTokenKind::Attribute)],
            "`string` here is a value, not a field-type declaration"
        );
    }

    #[test]
    fn whole_sample_file_has_no_overlaps() {
        let src = include_str!("../../../../samples/apl/example.apl");
        assert_no_overlap(src);
        assert!(
            tokenise_apl(src).len() > 50,
            "expected a substantial token stream"
        );
    }

    /// Byte offsets stay valid on non-ASCII input (the structural parser
    /// counts code points here and gets this wrong).
    /// An APL `[ … ]` bracket expression is embedded Tcl (the KCS feature doc is
    /// explicit that it gets full Tcl highlighting), so the APL lexer leaves it
    /// for the caller — but a `[` *inside a string* is just a character.
    #[test]
    fn embedded_tcl_regions_are_left_for_the_tcl_walker() {
        let src = "section a {\n    message \"has [expr 1] inside\"\n}\n[\n    set x 1\n]\n";
        let regions = embedded_tcl_regions(src);
        assert_eq!(regions.len(), 1, "the bracketed block only: {regions:?}");
        let body = &src[regions[0].0 as usize..regions[0].1 as usize];
        assert!(body.contains("set x 1"), "body: {body:?}");
        assert!(
            !body.contains("expr"),
            "a `[` inside a string is not embedded Tcl"
        );

        // The APL around it still tokenises; the Tcl inside it does not.
        let t = toks(src);
        assert!(t.contains(&("section", AplTokenKind::SectionKw)), "{t:?}");
        assert!(
            !t.iter().any(|(text, _)| *text == "set"),
            "the APL lexer must not tokenise embedded Tcl: {t:?}"
        );
        assert_no_overlap(src);
    }

    #[test]
    fn non_ascii_offsets_are_byte_exact() {
        let src = "display \"café\"\nstring addr\n";
        for t in tokenise_apl(src) {
            assert!(
                src.is_char_boundary(t.start as usize) && src.is_char_boundary(t.end as usize),
                "token {t:?} must land on char boundaries"
            );
        }
        assert!(toks(src).contains(&("addr", AplTokenKind::FieldName)));
    }
}
