//! Semantic-tokens provider — Rust port of
//! `lsp/features/_semantic_tokens/`.
//!
//! Produces an LSP-encoded semantic-tokens stream covering
//! the common Tcl token categories:
//!
//! * **Keyword** — command heads carrying the registry's
//!   `LANGUAGE_KEYWORD` trait (`if`, `while`, `for`, `foreach`,
//!   `switch`, `return`, `break`, `continue`, `try`, `catch`,
//!   `proc`, `namespace`, `when`, `oo::*`, …) plus the non-command
//!   clause / `TclOO` sub-keywords (`else`, `elseif`, `method`,
//!   `constructor`, …).
//! * **Function** — every other command-head token (user
//!   procs + built-in commands).
//! * **Variable** — `$name` / `${name}` substitutions.
//! * **String** — braced literals (`{...}`) and double-quoted
//!   strings.
//! * **Number** — integer / float literals.
//! * **Comment** — `# ...` comment lines.
//! * **Namespace** — namespace-qualified names containing
//!   `::`.
//! * **Regexp** — the regex-pattern argument of `regexp` / `regsub`
//!   (registry `pattern_type == Regex`, option-skipped positional),
//!   sub-tokenised into ARE components (`RegexpGroup` /
//!   `RegexpCharClass` / `RegexpQuantifier` / `RegexpAnchor` /
//!   `RegexpEscape` / `RegexpBackref` / `RegexpAlternation`).
//! * **Event** — an iRules `when EVENT` event name.
//! * **Format** — `format` / `scan` conversion strings
//!   (`FormatPercent` / `FormatFlag` / `FormatWidth` / `FormatSpec`),
//!   `clock format` / `scan` field strings (`ClockPercent` /
//!   `ClockSpec` / `ClockModifier`), and `binary format` / `scan`
//!   field strings (`BinarySpec` / `BinaryCount` / `BinaryFlag`).
//!
//! The legend is exposed via [`legend_token_types`] and
//! [`legend_token_modifiers`] so the server advertises it in
//! the LSP `initialize` capabilities response.
//!
//! What landed in `S-semantic-tokens-rich`:
//!
//! * Range variant ([`range`]) — same encoding as [`full`]
//!   filtered to tokens whose start position falls inside
//!   the request range.  Server advertises `range: true`.
//! * Delta variant — when the client's `previousResultId`
//!   matches the per-URI cached stream, the server returns the
//!   minimal token-aligned edit computed by [`diff`] (an empty
//!   edit list when nothing changed); a stale / unknown previous
//!   id falls back to a fresh full stream.
//!
//! What is *still deferred* (planned as further
//! `S-semantic-tokens-rich` sub-strips):
//!
//! * `regsub`-replacement (`\1` / `\&`) component highlighting — the
//!   sprintf / clock / binary format dialects have landed; this is the
//!   remaining format sub-token slice.
//! * The `BigIP` object taxonomy (`pool` / `profile` /
//!   `ipAddress` / …) for iRules string arguments.

use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LineIndex, Token, TokenType};
use tcl_registry::CommandRegistry;

/// Encoded semantic-tokens response.  The `data` array is
/// the LSP packed integer encoding (5 ints per token: line
/// delta, column delta, length, type, modifiers).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticTokens {
    /// Packed integer data.
    pub data: Vec<u32>,
}

/// Indexed enum for the token types we emit.  Numeric
/// values must align with the order returned by
/// [`legend_token_types`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
enum TokenKind {
    Keyword = 0,
    Function = 1,
    Variable = 2,
    String = 3,
    Number = 4,
    Comment = 5,
    Namespace = 6,
    /// Regular-expression pattern argument (`regexp` / `regsub`).
    Regexp = 7,
    /// iRules event name (`when EVENT`).
    Event = 8,
    /// Regex group / flags: `(`, `)`, `(?:`, `(?imsx)`.
    RegexpGroup = 9,
    /// Regex character class: `[...]`, `\d` / `\w` / `\s`, `.`.
    RegexpCharClass = 10,
    /// Regex quantifier: `*` `+` `?` `{n,m}` and lazy variants.
    RegexpQuantifier = 11,
    /// Regex anchor: `^` `$` `\A` `\Z` `\b` `\B` `\m` `\M` `\y` `\Y`.
    RegexpAnchor = 12,
    /// Regex escape sequence: `\n` `\t` `\xHH` `\uHHHH` `\<meta>`.
    RegexpEscape = 13,
    /// Regex backreference: `\0`–`\9`.
    RegexpBackref = 14,
    /// Regex alternation pipe: `|`.
    RegexpAlternation = 15,
    /// `format`/`scan` `%` introducer and `$` position separator.
    FormatPercent = 16,
    /// `format`/`scan` conversion type letter (`d` `s` `f` `x` …).
    FormatSpec = 17,
    /// `format`/`scan` flags (`-` `+` `0` `#` space) and length modifier.
    FormatFlag = 18,
    /// `format`/`scan` numeric width / precision values.
    FormatWidth = 19,
    /// `clock format`/`scan` `%` introducer.
    ClockPercent = 20,
    /// `clock` specifier letter (`Y` `m` `d` `H` `M` `S` …).
    ClockSpec = 21,
    /// `clock` locale modifier (`E` / `O`).
    ClockModifier = 22,
    /// `binary format`/`scan` specifier letter (`a` `A` `c` `i` `w` …).
    BinarySpec = 23,
    /// `binary` repeat count (numeric).
    BinaryCount = 24,
    /// `binary` modifier: `u` / `s` (signed/unsigned) or `*` (all).
    BinaryFlag = 25,
}

/// `binary format`/`scan` specifier letters.  Mirrors
/// `_BINARY_FORMAT_SPECIFIERS`.
const BINARY_FORMAT_SPECIFIERS: &[u8] = b"aAbBhHcsSiInwWmrRfdxX@t";

/// Integer specifiers that accept a `u`/`s` signed/unsigned modifier
/// (Tcl 8.5+).  Mirrors `_BINARY_INT_SPECIFIERS`.
const BINARY_INT_SPECIFIERS: &[u8] = b"csSiIntwWmrR";

/// The token-type / token-modifier legend the server
/// advertises during `initialize`.
#[must_use]
pub fn legend_token_types() -> Vec<&'static str> {
    vec![
        "keyword",
        "function",
        "variable",
        "string",
        "number",
        "comment",
        "namespace",
        "regexp",
        "event",
        "regexpGroup",
        "regexpCharClass",
        "regexpQuantifier",
        "regexpAnchor",
        "regexpEscape",
        "regexpBackref",
        "regexpAlternation",
        "formatPercent",
        "formatSpec",
        "formatFlag",
        "formatWidth",
        "clockPercent",
        "clockSpec",
        "clockModifier",
        "binarySpec",
        "binaryCount",
        "binaryFlag",
    ]
}

/// Token-modifiers part of the legend.  Order is fixed and must
/// align with the `1 << index` bits in [`MOD_DEFAULT_LIBRARY`] etc.
/// Mirrors `SEMANTIC_TOKEN_MODIFIERS` in
/// `lsp/features/_semantic_tokens/_constants.py`.
#[must_use]
pub fn legend_token_modifiers() -> Vec<&'static str> {
    vec!["declaration", "definition", "readonly", "defaultLibrary"]
}

/// `defaultLibrary` modifier bit (legend index 3) — set on a
/// command head that resolves to a registry built-in.  Mirrors
/// `1 << _MOD_INDEX["defaultLibrary"]`.
const MOD_DEFAULT_LIBRARY: u32 = 1 << 3;

/// Sub-keywords highlighted as `keyword` that are **not** standalone
/// commands, so they have no `CommandSpec` to carry the
/// `LANGUAGE_KEYWORD` trait: clause keywords of `if`/`try`/`switch`
/// and `TclOO` definition-/method-context words.  Mirrors Python's
/// `_LANGUAGE_KEYWORD_SUB_KEYWORDS` in
/// `lsp/features/_semantic_tokens/_constants.py`.  The standalone
/// commands (`if`, `while`, `proc`, `when`, `oo::*`, …) are sourced
/// from the registry's `LANGUAGE_KEYWORD` trait instead of a
/// hardcoded list.
const LANGUAGE_KEYWORD_SUB_KEYWORDS: &[&str] = &[
    // Clause keywords of if / try / switch — not standalone commands.
    "else",
    "elseif",
    "on",
    "trap",
    "finally",
    // TclOO definition-context keywords without a standalone CommandSpec.
    "method",
    "constructor",
    "destructor",
    "forward",
    "mixin",
    "filter",
    "superclass",
    "renamemethod",
    "deletemethod",
    "export",
    "unexport",
    // TclOO definition-context keywords (9.0+).
    "classmethod",
    "definitionnamespace",
    "initialise",
    "initialize",
    "private",
    "property",
    // TclOO method-body keywords without a standalone CommandSpec.
    "callback",
    "mymethod",
    "link",
];

/// Classify a command-head token name.  Mirrors Python's
/// `_classify_token` (command-name branch): a name is a `keyword`
/// when it carries the registry's `LANGUAGE_KEYWORD` trait or is one
/// of the non-command [`LANGUAGE_KEYWORD_SUB_KEYWORDS`]; a
/// `::`-qualified name is a `namespace`; everything else is a
/// `function`.
fn classify_command_head(name: &str, registry: &CommandRegistry) -> TokenKind {
    let is_keyword = registry.get(name).is_some_and(|s| {
        s.traits
            .contains(tcl_registry::prelude::Traits::LANGUAGE_KEYWORD)
    }) || LANGUAGE_KEYWORD_SUB_KEYWORDS.contains(&name);
    if is_keyword {
        TokenKind::Keyword
    } else if name.contains("::") {
        TokenKind::Namespace
    } else {
        TokenKind::Function
    }
}

/// Compute semantic tokens for the entire document.
#[must_use]
pub fn full(source: &str, dialect: &str, registry: &CommandRegistry) -> SemanticTokens {
    let entries = collect_entries(source, dialect, registry);
    encode_entries(&entries)
}

/// Compute semantic tokens for `range` within the document.
/// Tokens whose start position falls outside the range are
/// dropped.  Delta encoding starts from the first surviving
/// token rather than the document origin, matching the LSP
/// spec for `semanticTokens/range`.
#[must_use]
pub fn range(
    source: &str,
    dialect: &str,
    range: crate::definition::LspRange,
    registry: &CommandRegistry,
) -> SemanticTokens {
    let mut entries = collect_entries(source, dialect, registry);
    entries.retain(|(line, col, _, _, _)| {
        // Half-open interval per LSP `Range` semantics (PR #454
        // Copilot review): start is inclusive, end is exclusive.
        let pos = (*line, *col);
        let start = (range.start_line, range.start_character);
        let end = (range.end_line, range.end_character);
        pos >= start && pos < end
    });
    encode_entries(&entries)
}

/// One collected token: `(line, col, length, kind, modifiers)` with
/// absolute line/column and a token-modifier bitmask (see
/// [`legend_token_modifiers`]).
type Entry = (u32, u32, u32, TokenKind, u32);

/// True when `s` looks like an iRules event name (`^[A-Z][A-Z0-9_]+$`).
/// Mirrors `_EVENT_RE`.
fn is_event_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2
        && bytes[0].is_ascii_uppercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || *b == b'_')
}

/// How a specific argument token should be classified, overriding the
/// default lexer-kind classification.
#[derive(Debug, Clone, Copy)]
enum ArgOverride {
    /// Classify the whole token as this kind (e.g. an event name).
    Kind(TokenKind),
    /// Sub-tokenise the token as a regex pattern (groups / classes /
    /// quantifiers / …); falls back to a single `regexp` token when the
    /// pattern has no metacharacters.
    RegexPattern,
    /// Sub-tokenise the token as a `format`/`scan` conversion string
    /// (`%[pos$][flags][width][.prec][len]type`); falls back to the
    /// default classification when it has no `%` specifiers.
    SprintfFormat,
    /// Sub-tokenise the token as a `clock format`/`scan` field string
    /// (`%Y` / `%Ey` / …); falls back to the default classification when
    /// it has no `%` specifiers.
    ClockFormat,
    /// Sub-tokenise the token as a `binary format`/`scan` field string
    /// (`a3` / `Su` / `c*` / …); falls back to the default classification
    /// when no specifier is recognised.
    BinaryFormat,
}

/// Per-command argument-token classification overrides, keyed by the
/// representative token's start offset.  Two registry-driven cases:
///
/// * a `regexp` / `regsub` regex-pattern argument (the spec's
///   `pattern_type == Regex`, option-skipped first positional) →
///   [`ArgOverride::RegexPattern`] (sub-tokenised into ARE components);
/// * a `when EVENT` event-name argument → [`TokenKind::Event`].
///
/// The format-string (`%Y` / `%s`) and `BigIP` object sub-token
/// taxonomies remain the deferred bulk of `S-semantic-tokens-rich`.
fn special_arg_kinds(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
) -> std::collections::HashMap<u32, ArgOverride> {
    let mut overrides = std::collections::HashMap::new();
    let head = &seg.texts[0];

    // `when EVENT` — the literal event-name argument.
    if head == "when" {
        if let (Some(tok), Some(text)) = (seg.argv.get(1), seg.texts.get(1)) {
            if matches!(tok.kind, TokenType::Esc) && is_event_name(text) {
                overrides.insert(tok.span.start(), ArgOverride::Kind(TokenKind::Event));
            }
        }
    }

    // Regex pattern argument of a `pattern_type == Regex` command.
    if registry
        .get(head)
        .and_then(|s| s.pattern_type)
        .is_some_and(|p| p == tcl_registry::patterns::PatternType::Regex)
    {
        let args = &seg.texts[1..];
        let mut idx = 0;
        while idx < args.len() && args[idx].starts_with('-') && args[idx] != "--" {
            if args[idx] == "-start" && idx + 1 < args.len() {
                idx += 2;
            } else {
                idx += 1;
            }
        }
        if idx < args.len() && args[idx] == "--" {
            idx += 1;
        }
        // `args[idx]` is the pattern; its representative token is
        // `seg.argv[idx + 1]` (argv[0] is the command head).
        if let Some(tok) = seg.argv.get(idx + 1) {
            overrides.insert(tok.span.start(), ArgOverride::RegexPattern);
        }
    }

    // `format FMT …` (arg 1) / `scan STR FMT …` (arg 2) — the conversion
    // string.  Command-name gated, mirroring `_sprintf_format_arg_index`
    // (the registry's `format_string_type` field is never populated).
    let fmt_word = match head.as_str() {
        "format" if seg.argv.len() >= 2 => Some(1),
        "scan" if seg.argv.len() >= 3 => Some(2),
        _ => None,
    };
    if let Some(w) = fmt_word {
        if let Some(tok) = seg.argv.get(w) {
            overrides.insert(tok.span.start(), ArgOverride::SprintfFormat);
        }
    }

    // `clock format/scan … -format FMT` — the `-format` option value.
    // Mirrors `_clock_format_arg_index`.
    if head == "clock" && seg.texts.len() >= 3 && matches!(seg.texts[1].as_str(), "format" | "scan")
    {
        if let Some(i) = (2..seg.texts.len()).find(|&i| seg.texts[i] == "-format") {
            if let Some(tok) = seg.argv.get(i + 1) {
                overrides.insert(tok.span.start(), ArgOverride::ClockFormat);
            }
        }
    }

    // `binary format FMT …` (arg 2) / `binary scan VAL FMT …` (arg 3).
    // Mirrors `_binary_format_arg_index`.
    if head == "binary" && seg.texts.len() >= 3 {
        let bin_word = match seg.texts[1].as_str() {
            "format" => Some(2),
            "scan" if seg.texts.len() >= 4 => Some(3),
            _ => None,
        };
        if let Some(w) = bin_word {
            if let Some(tok) = seg.argv.get(w) {
                overrides.insert(tok.span.start(), ArgOverride::BinaryFormat);
            }
        }
    }

    overrides
}

/// Sub-tokenise a `binary format`/`scan` field string into its
/// specifiers: digit runs → `BinaryCount`, specifier letters →
/// `BinarySpec`, a `u`/`s` modifier after an integer specifier (Tcl 8.5+)
/// or a trailing `*` → `BinaryFlag`.  Whitespace and unrecognised
/// characters are skipped.  Returns `false` when nothing was emitted.
/// Mirrors `_collect_binary_format_spec_tokens`.
fn push_binary_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    dialect: &str,
    entries: &mut Vec<Entry>,
) -> bool {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return false;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    let Some(inner) = source.get(cstart..cend) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let allow_mod = !matches!(dialect, "tcl8.4" | "f5");
    let mut i = 0;
    let mut emitted = false;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Digit run → count.
        let count_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > count_start {
            push_subtoken(
                line_index,
                cstart + count_start,
                &inner[count_start..i],
                TokenKind::BinaryCount,
                entries,
            );
            emitted = true;
        }
        if i >= bytes.len() {
            break;
        }
        let spec = bytes[i];
        if !BINARY_FORMAT_SPECIFIERS.contains(&spec) {
            i += 1;
            continue;
        }
        push_subtoken(
            line_index,
            cstart + i,
            &inner[i..=i],
            TokenKind::BinarySpec,
            entries,
        );
        emitted = true;
        i += 1;
        // Signed/unsigned modifier (Tcl 8.5+) after an integer specifier.
        if i < bytes.len()
            && matches!(bytes[i], b'u' | b's')
            && BINARY_INT_SPECIFIERS.contains(&spec)
            && allow_mod
        {
            push_subtoken(
                line_index,
                cstart + i,
                &inner[i..=i],
                TokenKind::BinaryFlag,
                entries,
            );
            emitted = true;
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'*' {
            push_subtoken(line_index, cstart + i, "*", TokenKind::BinaryFlag, entries);
            emitted = true;
            i += 1;
        }
    }
    emitted
}

/// Sub-tokenise a `clock format`/`scan` field string into its `%`
/// specifiers (`ClockPercent` + optional `ClockModifier` + `ClockSpec`),
/// literal runs classified as `string`.  Returns `false` when there are
/// no specifiers.  Mirrors `_collect_clock_format_spec_tokens` +
/// `_CLOCK_FORMAT_RE`.
fn push_clock_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return false;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    let Some(inner) = source.get(cstart..cend) else {
        return false;
    };
    let re = regex::Regex::new(r"%(?:[EO])?[aAbBcCdDeEgGhHIjJklmMNOpPqQsSuUVwWxXyYzZ%]")
        .expect("valid clock regex");
    let mut matched_any = false;
    let mut pos = 0usize;
    for m in re.find_iter(inner) {
        matched_any = true;
        if m.start() > pos {
            push_subtoken(
                line_index,
                cstart + pos,
                &inner[pos..m.start()],
                TokenKind::String,
                entries,
            );
        }
        // `%`
        push_subtoken(
            line_index,
            cstart + m.start(),
            "%",
            TokenKind::ClockPercent,
            entries,
        );
        let mut spec = &m.as_str()[1..]; // after `%`
        let mut off = m.start() + 1;
        if spec.len() > 1 && matches!(spec.as_bytes()[0], b'E' | b'O') {
            push_subtoken(
                line_index,
                cstart + off,
                &spec[..1],
                TokenKind::ClockModifier,
                entries,
            );
            off += 1;
            spec = &spec[1..];
        }
        if !spec.is_empty() {
            push_subtoken(
                line_index,
                cstart + off,
                spec,
                TokenKind::ClockSpec,
                entries,
            );
        }
        pos = m.end();
    }
    if !matched_any {
        return false;
    }
    if pos < inner.len() {
        push_subtoken(
            line_index,
            cstart + pos,
            &inner[pos..],
            TokenKind::String,
            entries,
        );
    }
    true
}

/// Sub-tokenise a `format`/`scan` conversion string into its `%`
/// specifier components (`FormatPercent` / `FormatFlag` / `FormatWidth`
/// / `FormatSpec`), with literal runs classified as `string`.  Returns
/// `false` (emitting nothing) when there are no `%` specifiers.  Mirrors
/// `_collect_sprintf_format_spec_tokens` + `_SPRINTF_RE`.
fn push_sprintf_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return false;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    let Some(inner) = source.get(cstart..cend) else {
        return false;
    };
    let re = regex::Regex::new(concat!(
        r"%(?:(?P<position>\d+)\\?\$)?(?P<flags>[-+ 0#]*)?",
        r"(?P<width>\*|\d+)?(?:.(?P<precision>\*|\d+))?",
        r"(?P<length>[hlLzq])?(?P<type>[aAbBcdieEfgGosuxX%])",
    ))
    .expect("valid sprintf regex");
    let caps_list: Vec<_> = re.captures_iter(inner).collect();
    if caps_list.is_empty() {
        return false;
    }
    let mut pos_in_text = 0usize;
    for caps in caps_list {
        let m = caps.get(0).expect("group 0");
        if m.start() > pos_in_text {
            push_subtoken(
                line_index,
                cstart + pos_in_text,
                &inner[pos_in_text..m.start()],
                TokenKind::String,
                entries,
            );
        }
        emit_sprintf_spec(line_index, cstart, inner, &caps, entries);
        pos_in_text = m.end();
    }
    if pos_in_text < inner.len() {
        push_subtoken(
            line_index,
            cstart + pos_in_text,
            &inner[pos_in_text..],
            TokenKind::String,
            entries,
        );
    }
    true
}

/// Emit the component sub-tokens of one `%`-specifier match
/// (`%[pos$][flags][width][.prec][len]type`).  Split out of
/// [`push_sprintf_subtokens`] to stay within the line budget.
fn emit_sprintf_spec(
    line_index: &LineIndex,
    cstart: usize,
    inner: &str,
    caps: &regex::Captures,
    entries: &mut Vec<Entry>,
) {
    let m = caps.get(0).expect("group 0");
    let starts_digit = |off: usize| inner.as_bytes().get(off).is_some_and(u8::is_ascii_digit);
    let mut pos = m.start();
    emit_part(
        line_index,
        cstart,
        inner,
        &mut pos,
        m.start() + 1,
        TokenKind::FormatPercent,
        entries,
    );
    if let Some(p) = caps.name("position") {
        emit_part(
            line_index,
            cstart,
            inner,
            &mut pos,
            p.end(),
            TokenKind::FormatWidth,
            entries,
        );
        emit_part(
            line_index,
            cstart,
            inner,
            &mut pos,
            p.end() + 1,
            TokenKind::FormatPercent,
            entries,
        );
    }
    if let Some(f) = caps.name("flags") {
        emit_part(
            line_index,
            cstart,
            inner,
            &mut pos,
            f.end(),
            TokenKind::FormatFlag,
            entries,
        );
    }
    if let Some(w) = caps.name("width") {
        let kind = if starts_digit(w.start()) {
            TokenKind::FormatWidth
        } else {
            TokenKind::FormatFlag
        };
        emit_part(line_index, cstart, inner, &mut pos, w.end(), kind, entries);
    }
    if let Some(pr) = caps.name("precision") {
        // The `.` separator before the precision value.
        emit_part(
            line_index,
            cstart,
            inner,
            &mut pos,
            pr.start(),
            TokenKind::FormatFlag,
            entries,
        );
        let kind = if starts_digit(pr.start()) {
            TokenKind::FormatWidth
        } else {
            TokenKind::FormatFlag
        };
        emit_part(line_index, cstart, inner, &mut pos, pr.end(), kind, entries);
    }
    if let Some(l) = caps.name("length") {
        emit_part(
            line_index,
            cstart,
            inner,
            &mut pos,
            l.end(),
            TokenKind::FormatFlag,
            entries,
        );
    }
    if let Some(t) = caps.name("type") {
        emit_part(
            line_index,
            cstart,
            inner,
            &mut pos,
            t.end(),
            TokenKind::FormatSpec,
            entries,
        );
    }
}

/// Emit `inner[*pos..end]` (absolute offset `cstart + *pos`) as `kind`
/// and advance `*pos`, when non-empty.  The sub-token cursor helper for
/// [`push_sprintf_subtokens`].
fn emit_part(
    line_index: &LineIndex,
    cstart: usize,
    inner: &str,
    pos: &mut usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > *pos {
        push_subtoken(line_index, cstart + *pos, &inner[*pos..end], kind, entries);
        *pos = end;
    }
}

/// Sub-tokenise a regex pattern token into ARE components (groups,
/// character classes, quantifiers, anchors, escapes, backreferences,
/// alternation), with the literal runs between them classified as
/// `regexp`.  Returns `false` (emitting nothing) when the token isn't a
/// braced/quoted literal or contains no metacharacters — the caller then
/// falls back to a single `regexp` token.  Mirrors
/// `_collect_regex_pattern_tokens` + `_REGEX_PART_RE`.
fn push_regex_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return false;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    let Some(inner) = source.get(cstart..cend) else {
        return false;
    };
    if inner.is_empty() {
        return false;
    }
    // ARE metacharacter scanner (the working alternatives of Python's
    // `_REGEX_PART_RE`; its one malformed `\{\d+\}` BRE branch can never
    // match real input and is omitted).
    let re = regex::Regex::new(concat!(
        r"\(\?[imnsxwpq]*(?:[-imnsxwpq]*)?\)", // (?flags)
        r"|\(\?[:=!>]",                        // non-capturing / lookaround open
        r"|\(|\)",                             // group open / close
        r"|\[(?:\^)?\]?(?:[^\]\\]|\\.)*\]",    // [char class]
        r"|\\[AbBdDmMsSwWyYZ]",                // ARE class shortcuts / anchors
        r"|\\[0-9]",                           // backreference
        r"|\\[.*+?(){}\[\]|^$\\]",             // escaped metachar
        r"|\\[aefnrtv]",                       // escape sequences
        r"|\\x[0-9a-fA-F]{1,2}",               // hex escape
        r"|\\u[0-9a-fA-F]{1,4}",               // unicode escape
        r"|\\U[0-9a-fA-F]{1,8}",               // wide unicode escape
        r"|[*+?]\??",                          // quantifiers + lazy
        r"|\{\d+(?:,\d*)?\}",                  // bounded quantifier {n,m}
        r"|\|",                                // alternation
        r"|[\^$]",                             // anchors
        r"|\.",                                // any-char
    ))
    .expect("valid ARE part regex");

    let mut matched_any = false;
    let mut pos = 0usize;
    for m in re.find_iter(inner) {
        matched_any = true;
        if m.start() > pos {
            push_subtoken(
                line_index,
                cstart + pos,
                &inner[pos..m.start()],
                TokenKind::Regexp,
                entries,
            );
        }
        let kind = classify_regex_component(m.as_str());
        push_subtoken(line_index, cstart + m.start(), m.as_str(), kind, entries);
        pos = m.end();
    }
    if !matched_any {
        return false;
    }
    if pos < inner.len() {
        push_subtoken(
            line_index,
            cstart + pos,
            &inner[pos..],
            TokenKind::Regexp,
            entries,
        );
    }
    true
}

/// Classify a single ARE metacharacter run.  Mirrors the component
/// classifier inside `_collect_regex_pattern_tokens`.
fn classify_regex_component(matched: &str) -> TokenKind {
    let bytes = matched.as_bytes();
    if matched.starts_with('[') {
        return TokenKind::RegexpCharClass;
    }
    if matched.starts_with('\\') && bytes.len() >= 2 {
        let ch = bytes[1];
        return if ch.is_ascii_digit() {
            TokenKind::RegexpBackref
        } else if matches!(
            ch,
            b'a' | b'e' | b'f' | b'n' | b'r' | b't' | b'v' | b'x' | b'u' | b'U'
        ) {
            TokenKind::RegexpEscape
        } else if matches!(ch, b'd' | b'D' | b's' | b'S' | b'w' | b'W') {
            TokenKind::RegexpCharClass
        } else if matches!(ch, b'b' | b'B' | b'm' | b'M' | b'y' | b'Y' | b'A' | b'Z') {
            TokenKind::RegexpAnchor
        } else {
            TokenKind::RegexpEscape
        };
    }
    match matched {
        "^" | "$" => TokenKind::RegexpAnchor,
        "|" => TokenKind::RegexpAlternation,
        "." => TokenKind::RegexpCharClass,
        _ if matched.starts_with('(') => TokenKind::RegexpGroup,
        _ => TokenKind::RegexpQuantifier,
    }
}

/// Push one regex sub-token at absolute byte offset `abs_off` covering
/// `text`.  Skips empty / multi-line runs.
fn push_subtoken(
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if text.is_empty() || text.contains('\n') {
        return;
    }
    let pos = line_index.position_at(u32::try_from(abs_off).unwrap_or(0));
    let len_chars = u32::try_from(text.chars().count()).unwrap_or(0);
    entries.push((pos.line, pos.character, len_chars, kind, 0));
}

/// Walk the segmenter + comment scan and return raw
/// [`Entry`] tuples sorted by position.  Shared by `full` and `range`.
fn collect_entries(source: &str, dialect: &str, registry: &CommandRegistry) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let line_index = LineIndex::new(source);

    // Walk every segmented command and classify each token.
    for seg in segment_commands_with_offset_and_config(
        source,
        0,
        tcl_lexer::LexerConfig::for_dialect(dialect),
    ) {
        if seg.argv.is_empty() {
            continue;
        }
        // Classify the command-head token.  A head that resolves to a
        // registry built-in carries the `defaultLibrary` modifier
        // (mirrors `_collect.py:913-920`: `is_cmd_name && function &&
        // builtin`).  User-defined procs aren't in the registry, so they
        // stay plain `function`.
        let head_tok = seg.argv[0];
        let head_text = &seg.texts[0];
        let head_kind = classify_command_head(head_text, registry);
        let head_mods = if head_kind == TokenKind::Function && registry.get(head_text).is_some() {
            MOD_DEFAULT_LIBRARY
        } else {
            0
        };
        push_token(
            &line_index,
            source,
            head_tok,
            head_kind,
            head_mods,
            &mut entries,
        );

        // Registry-driven per-argument overrides (regex patterns / event
        // names) keyed by the representative token's start offset.
        let overrides = special_arg_kinds(&seg, registry);

        // Walk the remaining tokens (arg-position tokens
        // + nested tokens).  Each contributes a classification
        // based on its `TokenType`, unless an override applies.
        for tok in &seg.all_tokens {
            // Skip the head token (already pushed).
            if tok.span == head_tok.span {
                continue;
            }
            match overrides.get(&tok.span.start()) {
                Some(ArgOverride::RegexPattern) => {
                    // Sub-tokenise the regex pattern; if it has no
                    // metacharacters, fall back to one `regexp` token.
                    if !push_regex_subtokens(&line_index, source, *tok, &mut entries) {
                        push_token(
                            &line_index,
                            source,
                            *tok,
                            TokenKind::Regexp,
                            0,
                            &mut entries,
                        );
                    }
                }
                Some(ArgOverride::SprintfFormat) => {
                    // Sub-tokenise the conversion string; if it has no
                    // `%` specifiers, fall back to the default kind.
                    if !push_sprintf_subtokens(&line_index, source, *tok, &mut entries) {
                        if let Some(kind) = classify_arg_token(*tok, source) {
                            push_token(&line_index, source, *tok, kind, 0, &mut entries);
                        }
                    }
                }
                Some(ArgOverride::ClockFormat) => {
                    if !push_clock_subtokens(&line_index, source, *tok, &mut entries) {
                        if let Some(kind) = classify_arg_token(*tok, source) {
                            push_token(&line_index, source, *tok, kind, 0, &mut entries);
                        }
                    }
                }
                Some(ArgOverride::BinaryFormat) => {
                    if !push_binary_subtokens(&line_index, source, *tok, dialect, &mut entries) {
                        if let Some(kind) = classify_arg_token(*tok, source) {
                            push_token(&line_index, source, *tok, kind, 0, &mut entries);
                        }
                    }
                }
                Some(ArgOverride::Kind(kind)) => {
                    push_token(&line_index, source, *tok, *kind, 0, &mut entries);
                }
                None => {
                    if let Some(kind) = classify_arg_token(*tok, source) {
                        push_token(&line_index, source, *tok, kind, 0, &mut entries);
                    }
                }
            }
        }
    }

    // Comments aren't in the segmenter's command stream
    // (it strips them).  Scan the source for `#` comments
    // separately.
    push_comment_tokens(source, &line_index, &mut entries);

    // Sort by (line, column) so the delta encoding works.
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
}

/// Classify a non-head token by its lexer-assigned kind.
fn classify_arg_token(tok: Token, source: &str) -> Option<TokenKind> {
    let span = tok.span;
    let len = (span.end() - span.start()) as usize;
    if len == 0 {
        return None;
    }
    match tok.kind {
        TokenType::Var => Some(TokenKind::Variable),
        TokenType::Str => Some(TokenKind::String),
        TokenType::Esc => {
            // Quoted strings vs barewords vs numbers.  The
            // lexer sets `tok.in_quote = true` on every Esc /
            // Var / Cmd token emitted from inside `"..."`, so
            // multi-fragment quoted strings (e.g. `"a $b c"`)
            // get every literal fragment classified as String
            // — including the leading fragment whose span may
            // not include the opening `"`.  This matches the
            // lexer contract and avoids the prior byte-peek
            // heuristic that missed inner fragments (PR #454
            // Copilot review).
            if tok.in_quote {
                return Some(TokenKind::String);
            }
            let start = span.start() as usize;
            let text = source
                .get(start..(start + len).min(source.len()))
                .unwrap_or("");
            if is_number_literal(text) {
                Some(TokenKind::Number)
            } else if text.contains("::") {
                Some(TokenKind::Namespace)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// `true` when `text` is a Tcl number literal — integer
/// (optionally signed, hex `0x...` or binary `0b...`) or
/// floating-point.
fn is_number_literal(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let trimmed = text.trim_start_matches(['+', '-']);
    if trimmed.is_empty() {
        return false;
    }
    if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit() || c == '_');
    }
    if let Some(rest) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        return !rest.is_empty() && rest.chars().all(|c| matches!(c, '0' | '1' | '_'));
    }
    // Integer or float.  Use Rust's parsers for simplicity.
    text.parse::<i64>().is_ok() || text.parse::<f64>().is_ok()
}

/// Scan `source` for `#` comment lines and push each one as
/// a Comment-kind entry.  Mirrors Python's `_collect_comments`.
fn push_comment_tokens(source: &str, line_index: &LineIndex, entries: &mut Vec<Entry>) {
    let mut byte_pos: u32 = 0;
    let mut line_start = true;
    for c in source.chars() {
        let len = u32::try_from(c.len_utf8()).unwrap_or(1);
        if c == '\n' {
            line_start = true;
            byte_pos += len;
            continue;
        }
        if c.is_whitespace() {
            byte_pos += len;
            continue;
        }
        if line_start && c == '#' {
            // Find the end of the comment line.
            let comment_start = byte_pos;
            let mut p = byte_pos;
            let bytes = source.as_bytes();
            while (p as usize) < bytes.len() && bytes[p as usize] != b'\n' {
                p += 1;
            }
            let comment_end = p;
            let pos = line_index.position_at(comment_start);
            let len_chars = u32::try_from(
                source[comment_start as usize..comment_end as usize]
                    .chars()
                    .count(),
            )
            .unwrap_or(0);
            entries.push((pos.line, pos.character, len_chars, TokenKind::Comment, 0));
            // Skip past the comment line.
            byte_pos = comment_end;
            line_start = false;
            continue;
        }
        line_start = false;
        byte_pos += len;
    }
}

/// Push a single token into the entries list, computing
/// (line, column, length-in-chars, kind).
fn push_token(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    kind: TokenKind,
    modifiers: u32,
    entries: &mut Vec<Entry>,
) {
    let span = tok.span;
    let len_bytes = span.end() - span.start();
    if len_bytes == 0 {
        return;
    }
    let pos = line_index.position_at(span.start());
    let text = source
        .get(span.start() as usize..span.end() as usize)
        .unwrap_or("");
    // Skip multi-line tokens — LSP encoding wants per-line
    // entries; multi-line tokens would need splitting.
    // For the minimal rich port, drop them.
    if text.contains('\n') {
        return;
    }
    let len_chars = u32::try_from(text.chars().count()).unwrap_or(0);
    entries.push((pos.line, pos.character, len_chars, kind, modifiers));
}

/// Encode entries into the LSP packed integer stream:
/// `[deltaLine, deltaCol, length, type, modifiers]` per token.
fn encode_entries(entries: &[Entry]) -> SemanticTokens {
    let mut data: Vec<u32> = Vec::with_capacity(entries.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    for (line, col, len, kind, modifiers) in entries {
        let delta_line = line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            *col
        };
        data.push(delta_line);
        data.push(delta_col);
        data.push(*len);
        data.push(*kind as u32);
        data.push(*modifiers);
        prev_line = *line;
        prev_col = *col;
    }
    SemanticTokens { data }
}

/// Number of packed integers per semantic token
/// (`[deltaLine, deltaCol, length, type, modifiers]`).
const TOKEN_STRIDE: usize = 5;

/// One minimal edit transforming a previous packed token stream
/// into a new one: starting at integer offset `start`, delete
/// `delete_count` integers and splice in `data`.
///
/// All three fields are token-aligned (multiples of
/// [`TOKEN_STRIDE`]) so the edit splits cleanly into whole
/// `SemanticToken`s, which is what the LSP `semanticTokens/full/
/// delta` wire shape requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenEdit {
    /// Integer offset into the previous `data` where the edit
    /// begins.
    pub start: u32,
    /// Number of integers to remove from the previous `data`.
    pub delete_count: u32,
    /// Replacement integers (the changed run of the new stream).
    pub data: Vec<u32>,
}

/// Compute the single minimal edit that turns `old` into `new`
/// by trimming the common leading and trailing tokens.
///
/// Operates at whole-token granularity: a token counts as common
/// only when its entire 5-integer group is identical, so the
/// returned offsets stay token-aligned.  Because the packed
/// encoding is *relative* (each token's delta is measured from
/// its predecessor), any change that shifts a token's position
/// perturbs its 5-tuple and pulls it into the replacement run —
/// so a prefix/suffix diff on the encoded array is correct
/// without re-deltifying the boundary.
///
/// Returns `None` when the streams are identical.
#[must_use]
pub fn diff(old: &[u32], new: &[u32]) -> Option<TokenEdit> {
    if old == new {
        return None;
    }
    let old_tokens = old.len() / TOKEN_STRIDE;
    let new_tokens = new.len() / TOKEN_STRIDE;
    let token = |buf: &[u32], i: usize| -> [u32; TOKEN_STRIDE] {
        let base = i * TOKEN_STRIDE;
        [
            buf[base],
            buf[base + 1],
            buf[base + 2],
            buf[base + 3],
            buf[base + 4],
        ]
    };
    let max_common = old_tokens.min(new_tokens);
    let mut prefix = 0;
    while prefix < max_common && token(old, prefix) == token(new, prefix) {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < max_common - prefix
        && token(old, old_tokens - 1 - suffix) == token(new, new_tokens - 1 - suffix)
    {
        suffix += 1;
    }
    let start = prefix * TOKEN_STRIDE;
    let delete_count = (old_tokens - prefix - suffix) * TOKEN_STRIDE;
    let data = new[start..(new_tokens - suffix) * TOKEN_STRIDE].to_vec();
    // Token streams are bounded well below `u32::MAX`; on the
    // theoretical overflow, return `None` so the caller falls back to
    // a full token set rather than emitting an invalid edit.
    let (Ok(start), Ok(delete_count)) = (u32::try_from(start), u32::try_from(delete_count)) else {
        return None;
    };
    Some(TokenEdit {
        start,
        delete_count,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    fn kinds(src: &str, dialect: &str, registry: &CommandRegistry) -> Vec<u32> {
        full(src, dialect, registry)
            .data
            .chunks(5)
            .map(|c| c[3])
            .collect()
    }

    #[test]
    fn legend_includes_regexp_and_event() {
        let types = legend_token_types();
        assert_eq!(types[TokenKind::Regexp as usize], "regexp");
        assert_eq!(types[TokenKind::Event as usize], "event");
    }

    #[test]
    fn regexp_pattern_classified_as_regexp() {
        // `regexp {abc} $s` — the `{abc}` pattern argument is `regexp`,
        // not `string`.
        let ks = kinds("regexp {abc} $s\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Regexp as u32)),
            "expected a regexp token; got {ks:?}"
        );
        // `regsub -all {x+} $s y out` — option-skip finds the pattern.
        let ks = kinds("regsub -all {x+} $s y out\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::Regexp as u32)),
            "expected a regexp token after -all; got {ks:?}"
        );
    }

    #[test]
    fn event_name_classified_as_event() {
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        let ks = kinds(
            "when HTTP_REQUEST {\n  set x 1\n}\n",
            "f5-irules",
            &registry,
        );
        assert!(
            ks.contains(&(TokenKind::Event as u32)),
            "expected an event token; got {ks:?}"
        );
    }

    #[test]
    fn regex_pattern_subtokenised_into_components() {
        // `(a+)+` → group `(`, literal `a`, quantifier `+`, group `)`,
        // quantifier `+`.
        let ks = kinds("regexp {(a+)+} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
        // The whole-pattern `regexp` kind is replaced by sub-tokens, but
        // the literal `a` run is still `regexp`.
        assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
    }

    #[test]
    fn regex_char_class_and_anchor_subtokens() {
        let ks = kinds("regexp {^[0-9]+$} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpAnchor as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::RegexpQuantifier as u32)), "{ks:?}");
    }

    #[test]
    fn regex_alternation_and_escape_subtokens() {
        let ks = kinds("regexp {a\\d|b} $s\n", "tcl", &reg());
        assert!(
            ks.contains(&(TokenKind::RegexpAlternation as u32)),
            "{ks:?}"
        );
        // `\d` is an ARE class shortcut → char class.
        assert!(ks.contains(&(TokenKind::RegexpCharClass as u32)), "{ks:?}");
    }

    #[test]
    fn regex_without_metachars_stays_single_regexp() {
        // `abc` has no metacharacters → one `regexp` token, no sub-tokens.
        let ks = kinds("regexp {abc} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::Regexp as u32)), "{ks:?}");
        assert!(!ks.contains(&(TokenKind::RegexpGroup as u32)), "{ks:?}");
        assert!(
            !ks.contains(&(TokenKind::RegexpQuantifier as u32)),
            "{ks:?}"
        );
    }

    #[test]
    fn classify_regex_component_maps_each_kind() {
        assert_eq!(classify_regex_component("("), TokenKind::RegexpGroup);
        assert_eq!(classify_regex_component("(?:"), TokenKind::RegexpGroup);
        assert_eq!(
            classify_regex_component("[a-z]"),
            TokenKind::RegexpCharClass
        );
        assert_eq!(classify_regex_component("\\d"), TokenKind::RegexpCharClass);
        assert_eq!(classify_regex_component("."), TokenKind::RegexpCharClass);
        assert_eq!(classify_regex_component("+"), TokenKind::RegexpQuantifier);
        assert_eq!(
            classify_regex_component("{2,3}"),
            TokenKind::RegexpQuantifier
        );
        assert_eq!(classify_regex_component("^"), TokenKind::RegexpAnchor);
        assert_eq!(classify_regex_component("\\b"), TokenKind::RegexpAnchor);
        assert_eq!(classify_regex_component("\\n"), TokenKind::RegexpEscape);
        assert_eq!(classify_regex_component("\\3"), TokenKind::RegexpBackref);
        assert_eq!(classify_regex_component("|"), TokenKind::RegexpAlternation);
    }

    #[test]
    fn sprintf_format_spec_subtokens() {
        // `format {%d}` → `%` percent, `d` spec.
        let ks = kinds("format {%d} $n\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn sprintf_flags_and_width_subtokens() {
        // `%-5.2f` → percent, `-` flag, `5` width, `.` flag, `2` width,
        // `f` spec.
        let ks = kinds("format {%-5.2f} $x\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::FormatFlag as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatWidth as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn scan_format_arg_subtokenised() {
        // `scan`'s format string is arg 2.
        let ks = kinds("scan $s {%d} a\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::FormatSpec as u32)), "{ks:?}");
    }

    #[test]
    fn format_without_specifiers_stays_string() {
        let ks = kinds("format {plain} $x\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::FormatPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn clock_format_subtokens() {
        // `clock format $t -format {%Y-%m-%d}` → %/letter pairs.
        let ks = kinds("clock format $t -format {%Y-%m-%d}\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
    }

    #[test]
    fn clock_locale_modifier_subtoken() {
        // `%Ey` → percent, `E` modifier, `y` spec.
        let ks = kinds("clock scan $s -format {%Ey}\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::ClockModifier as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::ClockSpec as u32)), "{ks:?}");
    }

    #[test]
    fn clock_format_without_specifiers_stays_string() {
        let ks = kinds("clock format $t -format {plain}\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::ClockPercent as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::String as u32)), "{ks:?}");
    }

    #[test]
    fn binary_format_spec_and_count_subtokens() {
        // `binary format a3 $d` (arg 2) → spec `a`, count `3`.
        let ks = kinds("binary format a3 $d\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::BinaryCount as u32)), "{ks:?}");
    }

    #[test]
    fn binary_scan_signed_modifier_and_star() {
        // `binary scan $d su r` (arg 3) → spec `s`, modifier `u`.
        let ks = kinds("binary scan $d su r\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
        // `c*` → spec `c`, `*` flag.
        let ks = kinds("binary format c* $l\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    }

    #[test]
    fn binary_signed_modifier_suppressed_in_tcl84() {
        // The `u`/`s` modifier is 8.5+, so under tcl8.4 the `u` is not a
        // binaryFlag (no signed/unsigned modifier).
        let ks = kinds("binary scan $d su r\n", "tcl8.4", &reg());
        assert!(ks.contains(&(TokenKind::BinarySpec as u32)), "{ks:?}");
        assert!(!ks.contains(&(TokenKind::BinaryFlag as u32)), "{ks:?}");
    }

    #[test]
    fn is_event_name_matches_event_shape() {
        assert!(is_event_name("HTTP_REQUEST"));
        assert!(is_event_name("CLIENT_ACCEPTED"));
        assert!(!is_event_name("lowercase"));
        assert!(!is_event_name("X")); // single char — needs 2+
    }

    #[test]
    fn semantic_tokens_are_dialect_aware_via_expand_syntax() {
        // SYNC-MAY19-dialect-contextvar strip 5: the provider re-segments
        // under the document dialect.  In `foo {*}$x`, on 8.5+ the `{*}`
        // is the expansion operator (consumed — not a highlighted word),
        // but on 8.4 it is a literal braced string `{*}`, which adds an
        // extra `string` token.  So the packed token stream is longer on
        // 8.4.  Before strip 5 the provider always lexed `{*}` as
        // expansion regardless of dialect.
        let src = "foo {*}$x\n";
        let on_90 = full(src, "tcl9.0", &reg()).data;
        let on_84 = full(src, "tcl8.4", &reg()).data;
        assert!(
            on_84.len() > on_90.len(),
            "8.4 keeps `{{*}}` as a highlighted string token (longer stream): \
             8.4={} 9.0={}",
            on_84.len(),
            on_90.len(),
        );
    }

    #[test]
    fn full_returns_non_empty_data_for_simple_proc() {
        let s = full("proc foo {} {}\n", "tcl", &reg());
        // Should have at least: `proc` (keyword), `foo`
        // (function), `{}` (string), `{}` (string).
        assert!(!s.data.is_empty(), "{:?}", s.data);
        // 5 ints per token.
        assert_eq!(s.data.len() % 5, 0);
    }

    #[test]
    fn diff_returns_none_for_identical_streams() {
        let a = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
        assert_eq!(diff(&a, &a), None);
    }

    #[test]
    fn diff_isolates_a_single_changed_token() {
        // Three tokens; only the middle one's type changes.
        let old = vec![
            0, 0, 4, 0, 0, /**/ 0, 5, 3, 1, 0, /**/ 1, 0, 2, 2, 0,
        ];
        let new = vec![
            0, 0, 4, 0, 0, /**/ 0, 5, 3, 4, 0, /**/ 1, 0, 2, 2, 0,
        ];
        let edit = diff(&old, &new).expect("an edit");
        // Skip the first token (5 ints), replace exactly one token.
        assert_eq!(edit.start, 5);
        assert_eq!(edit.delete_count, 5);
        assert_eq!(edit.data, vec![0, 5, 3, 4, 0]);
    }

    #[test]
    fn diff_handles_appended_token() {
        let old = vec![0, 0, 4, 0, 0];
        let new = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
        let edit = diff(&old, &new).expect("an edit");
        // Nothing deleted; one token appended after the prefix.
        assert_eq!(edit.start, 5);
        assert_eq!(edit.delete_count, 0);
        assert_eq!(edit.data, vec![0, 5, 3, 1, 0]);
    }

    #[test]
    fn diff_handles_removed_token() {
        let old = vec![0, 0, 4, 0, 0, 0, 5, 3, 1, 0];
        let new = vec![0, 0, 4, 0, 0];
        let edit = diff(&old, &new).expect("an edit");
        // One trailing token removed, nothing spliced in.
        assert_eq!(edit.start, 5);
        assert_eq!(edit.delete_count, 5);
        assert!(edit.data.is_empty());
    }

    #[test]
    fn legend_has_expected_entries() {
        let types = legend_token_types();
        assert_eq!(types[TokenKind::Keyword as usize], "keyword");
        assert_eq!(types[TokenKind::Function as usize], "function");
        assert_eq!(types[TokenKind::Variable as usize], "variable");
        assert_eq!(types[TokenKind::String as usize], "string");
        assert_eq!(types[TokenKind::Number as usize], "number");
        assert_eq!(types[TokenKind::Comment as usize], "comment");
        assert_eq!(types[TokenKind::Namespace as usize], "namespace");
    }

    #[test]
    fn legend_modifiers_match_python_order() {
        // Order is load-bearing: `defaultLibrary` must be bit index 3.
        let mods = legend_token_modifiers();
        assert_eq!(
            mods,
            vec!["declaration", "definition", "readonly", "defaultLibrary"]
        );
        assert_eq!(MOD_DEFAULT_LIBRARY, 1 << 3);
    }

    #[test]
    fn builtin_command_head_gets_default_library_modifier() {
        // `puts` is a registry built-in classified as `function`, so its
        // head token carries the `defaultLibrary` modifier (bit 3 = 8).
        let s = full("puts hi\n", "tcl", &reg());
        assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
        assert_eq!(s.data[4], MOD_DEFAULT_LIBRARY, "{:?}", s.data);
    }

    #[test]
    fn user_proc_head_has_no_default_library_modifier() {
        // A user-defined command isn't in the registry → `function`
        // with no modifier.
        let s = full("my_custom_cmd 1 2\n", "tcl", &reg());
        assert_eq!(s.data[3], TokenKind::Function as u32, "{:?}", s.data);
        assert_eq!(s.data[4], 0, "{:?}", s.data);
    }

    #[test]
    fn keyword_head_has_no_default_library_modifier() {
        // `if` is a language keyword, not a `function` — no defaultLibrary.
        let s = full("if {1} { puts hi }\n", "tcl", &reg());
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
        assert_eq!(s.data[4], 0, "{:?}", s.data);
    }

    #[test]
    fn keywords_classified_as_keyword() {
        let s = full("if {1} { puts hi }\n", "tcl", &reg());
        // First token's type index should be 0 (Keyword) for `if`.
        // The encoded data: [deltaLine, deltaCol, length, type, modifiers].
        assert_eq!(s.data[3], TokenKind::Keyword as u32, "{:?}", s.data);
    }

    #[test]
    fn comments_classified_as_comment() {
        let s = full("# this is a comment\nset x 1\n", "tcl", &reg());
        // The first token should be the comment.
        assert_eq!(s.data[3], TokenKind::Comment as u32, "{:?}", s.data);
    }

    #[test]
    fn variables_classified_as_variable() {
        let s = full("set $x 1\n", "tcl", &reg());
        // The `$x` token kind should be Variable.
        let kinds: Vec<u32> = s.data.chunks(5).map(|c| c[3]).collect();
        assert!(
            kinds.contains(&(TokenKind::Variable as u32)),
            "expected Variable in kinds; got {kinds:?}",
        );
    }

    #[test]
    fn is_number_literal_recognises_integers_and_floats() {
        assert!(is_number_literal("42"));
        assert!(is_number_literal("-7"));
        assert!(is_number_literal("3.14"));
        assert!(is_number_literal("0xff"));
        assert!(is_number_literal("0b1010"));
        assert!(!is_number_literal("abc"));
        assert!(!is_number_literal(""));
        assert!(!is_number_literal("1.2.3"));
    }

    #[test]
    fn empty_source_returns_empty_data() {
        assert!(full("", "tcl", &reg()).data.is_empty());
    }

    #[test]
    fn classify_command_head_picks_namespace_for_qualified() {
        assert_eq!(
            classify_command_head("::myns::greet", &reg()),
            TokenKind::Namespace,
        );
        assert_eq!(classify_command_head("greet", &reg()), TokenKind::Function);
        assert_eq!(classify_command_head("if", &reg()), TokenKind::Keyword);
    }

    // -- S-semantic-tokens-rich: range variant -----------------------

    #[test]
    fn range_filters_tokens_outside_window() {
        // Three commands on three lines.  Range covers only
        // line 1 — the line-0 and line-2 tokens should drop.
        let src = "set a 1\nset b 2\nset c 3\n";
        let full_data = full(src, "tcl", &reg());
        let line1_only = range(
            src,
            "tcl",
            crate::definition::LspRange {
                start_line: 1,
                start_character: 0,
                end_line: 1,
                end_character: 10,
            },
            &reg(),
        );
        // Each tcl line emits at least one classified token.
        // The range result must be strictly smaller than the
        // full result.
        assert!(line1_only.data.len() < full_data.data.len());
        assert!(line1_only.data.len() % 5 == 0);
        assert!(!line1_only.data.is_empty(), "{:?}", line1_only.data);
    }

    #[test]
    fn range_keeps_entire_document_when_range_covers_it() {
        let src = "proc foo {} { puts hi }\n";
        let full_data = full(src, "tcl", &reg());
        let wide = range(
            src,
            "tcl",
            crate::definition::LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 99,
                end_character: 0,
            },
            &reg(),
        );
        assert_eq!(wide.data, full_data.data);
    }

    #[test]
    fn range_excludes_token_at_exact_end_position() {
        // Regression for PR #454 Codex review: LSP ranges are
        // half-open [start, end), so a token starting exactly
        // at `end` is OUTSIDE the range.
        let src = "set a 1\nset b 2\n";
        // Range whose end exactly coincides with line 1, col 0
        // (the `set` of the second command).  That token should
        // not appear in the range result.
        let r = range(
            src,
            "tcl",
            crate::definition::LspRange {
                start_line: 0,
                start_character: 0,
                end_line: 1,
                end_character: 0,
            },
            &reg(),
        );
        // The full document has at least one line-1 token at col
        // 0 (the `set` of `set b 2`).  The half-open range must
        // exclude it; the range data must therefore be strictly
        // shorter than the full data.
        let full_data = full(src, "tcl", &reg());
        assert!(
            r.data.len() < full_data.data.len(),
            "range data {} should drop the line-1 token; full data {}",
            r.data.len(),
            full_data.data.len(),
        );
    }
}
