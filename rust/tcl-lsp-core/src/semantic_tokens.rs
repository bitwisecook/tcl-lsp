//! Semantic-tokens provider.
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
//!   `ClockSpec` / `ClockModifier`), `binary format` / `scan`
//!   field strings (`BinarySpec` / `BinaryCount` / `BinaryFlag`), and
//!   `regsub` replacement backrefs (`\1` → `Number`, `\&` → `Operator`).
//! * **Object** — BIG-IP object names (pools, data groups, virtuals,
//!   nodes, …) referenced from iRules code, under the `f5-irules`
//!   dialect (see [`crate::irules_object_refs`]).
//!
//! The legend is exposed via [`legend_token_types`] and
//! [`legend_token_modifiers`] so the server advertises it in
//! the LSP `initialize` capabilities response.
//!
//! Additional variants:
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
//! Not handled (a separate document-mode feature, not a
//! per-argument sub-token slice): BIG-IP **config-file** mode
//! (`is_bigip_conf`) — partition paths (`/Common/…`), IPv4 /
//! route-domain / port literals in `.conf` text — and **APL**
//! embedded-Tcl detection.  These run on whole documents of a
//! different type, not on Tcl/iRules command arguments.  The
//! iRules object-reference highlighting (the code-relevant half
//! of the BIG-IP taxonomy) is handled.

use rustc_hash::FxHashMap;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LineIndex, Token, TokenType};

use crate::definition::utf16_len;
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
    /// Operator — the `regsub` whole-match replacement backref `\&`.
    Operator = 26,
    /// BIG-IP object name referenced from iRules code (pool, data group,
    /// virtual, node, …).
    Object = 27,
    /// A recognised `-option` switch on a command (`regexp -nocase`).
    Decorator = 28,
    /// A backslash escape sequence inside a string/bareword (`\n`, `\t`, …).
    Escape = 29,
}

/// `binary format`/`scan` specifier letters.
const BINARY_FORMAT_SPECIFIERS: &[u8] = b"aAbBhHcsSiInwWmrRfdxX@t";

/// Integer specifiers that accept a `u`/`s` signed/unsigned modifier
/// (Tcl 8.5+).
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
        "operator",
        "object",
        "decorator",
        "escape",
    ]
}

/// Token-modifiers part of the legend.  Order is fixed and must
/// align with the `1 << index` bits in [`MOD_DEFAULT_LIBRARY`] etc.
#[must_use]
pub fn legend_token_modifiers() -> Vec<&'static str> {
    vec!["declaration", "definition", "readonly", "defaultLibrary"]
}

/// `defaultLibrary` modifier bit (legend index 3) — set on a
/// command head that resolves to a registry built-in.
const MOD_DEFAULT_LIBRARY: u32 = 1 << 3;

/// `definition` modifier bit (legend index 1) — set on the name token of a
/// `proc` definition.
const MOD_DEFINITION: u32 = 1 << 1;

/// Sub-keywords highlighted as `keyword` that are **not** standalone
/// commands, so they have no `CommandSpec` to carry the
/// `LANGUAGE_KEYWORD` trait: clause keywords of `if`/`try`/`switch`
/// and `TclOO` definition-/method-context words.  The standalone
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

/// Classify a command-head token name: a name is a `keyword`
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
    } else if is_operator_command(name) {
        // A bare operator used as a command head (`+ 3 4`, `tcl::mathop`
        // style).
        TokenKind::Operator
    } else if name.contains("::") {
        TokenKind::Namespace
    } else {
        TokenKind::Function
    }
}

/// `true` when `name` is one of the recognised operator command heads
/// (`+ - * / > >= < <= == !=`).
fn is_operator_command(name: &str) -> bool {
    matches!(
        name,
        "+" | "-" | "*" | "/" | ">" | ">=" | "<" | "<=" | "==" | "!="
    )
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
        // Half-open interval per LSP `Range` semantics: start is
        // inclusive, end is exclusive.
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
    /// Sub-tokenise the token as a `regsub` replacement spec (`\1`-`\9`
    /// → number, `\&` → operator); falls back to the default
    /// classification when it has no backreferences.
    RegsubReplace,
    /// Recurse into a braced command-body argument (`ArgRole::Body`),
    /// re-segmenting its inner script so nested commands / vars / strings
    /// are tokenised rather than emitted as one opaque `string`.
    BodyScript,
    /// Recurse into a braced expression argument (`ArgRole::Expr`),
    /// tokenising it via the expression sub-lexer (variables / numbers /
    /// operators / functions / nested `[cmd]` substitutions).
    ExprScript,
    /// A recognised `-option` switch → `Decorator`.
    Decorator,
    /// A known subcommand word (arg index 1) → `Keyword` + `defaultLibrary`.
    SubcommandKeyword,
    /// The name argument of a `proc` definition → `Function` + `definition`.
    ProcNameDef,
    /// The braced case-list argument of `switch -regexp … { pat body … }`:
    /// the pattern elements are sub-tokenised as regexes and the body
    /// elements recursed as scripts.
    SwitchRegexpCaseList,
    /// A structural keyword word at an argument position (`if`'s
    /// `then`/`elseif`/`else`, `try`'s `on`/`trap`/`finally`), carried
    /// by `ArgRole::Keyword` → highlighted as `Keyword` rather than a
    /// string.
    KeywordArg,
}

/// The inner content (delimiters stripped via `content_offset`) of a
/// braced/quoted literal token, plus its absolute byte start, or `None`
/// for a non-literal token / out-of-bounds span.  Shared by the
/// sub-language scanners.
fn subspec_content(source: &str, tok: Token) -> Option<(usize, &str)> {
    if !matches!(tok.kind, TokenType::Str | TokenType::Esc) {
        return None;
    }
    let cstart = tok.span.start() as usize + tok.content_offset as usize;
    let cend = (tok.span.end() as usize).min(source.len());
    source.get(cstart..cend).map(|inner| (cstart, inner))
}

#[derive(Clone, Copy)]
struct TokenPositionContext<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
}

/// Emit the literal run `inner[run..end]` (absolute start `cstart + run`)
/// as `kind`, when non-empty.  The inter-construct filler for the
/// sub-language scanners.
fn flush_run(
    pos: TokenPositionContext<'_>,
    cstart: usize,
    inner: &str,
    run: usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > run {
        push_subtoken(
            pos.source,
            pos.line_index,
            cstart + run,
            &inner[run..end],
            kind,
            entries,
        );
    }
}

/// Sub-tokenise a `regsub` replacement spec: `\&` → `Operator`,
/// `\0`-`\9` → `Number`, literal runs → `String`.  Returns `false` when
/// there are no backreferences.  A direct backslash scan (no regex).
fn push_regsub_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let next = bytes.get(i + 1).copied();
        if bytes[i] == b'\\' && next.is_some_and(|b| b.is_ascii_digit() || b == b'&') {
            flush_run(pos, cstart, inner, run, i, TokenKind::String, entries);
            // `\&` → operator (whole match); `\0`-`\9` → number (capture).
            let kind = if next == Some(b'&') {
                TokenKind::Operator
            } else {
                TokenKind::Number
            };
            push_subtoken(
                source,
                line_index,
                cstart + i,
                &inner[i..i + 2],
                kind,
                entries,
            );
            emitted = true;
            i += 2;
            run = i;
        } else {
            i += 1;
        }
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// Per-command argument-token classification overrides, keyed by the
/// representative token's start offset.  Two registry-driven cases:
///
/// * a `regexp` / `regsub` regex-pattern argument (the spec's
///   `pattern_type == Regex`, option-skipped first positional) →
///   [`ArgOverride::RegexPattern`] (sub-tokenised into ARE components);
/// * a `when EVENT` event-name argument → [`TokenKind::Event`].
///
/// `arg_texts` holds the command's argument words (`seg.texts[1..]`, head
/// excluded) borrowed as `&[&str]`.  The caller builds it once and shares it
/// with the registry-role and OO-body override passes, so the hot path makes
/// only a single bridging allocation per command.
fn special_arg_kinds(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    inside_oo_body: bool,
    arg_texts: &[&str],
) -> FxHashMap<u32, ArgOverride> {
    let mut overrides = FxHashMap::default();
    let head = &seg.texts[0];

    // `when EVENT` — the literal event-name argument.
    if head == "when"
        && let (Some(tok), Some(text)) = (seg.argv.get(1), seg.texts.get(1))
        && matches!(tok.kind, TokenType::Esc)
        && is_event_name(text)
    {
        overrides.insert(tok.span.start(), ArgOverride::Kind(TokenKind::Event));
    }

    insert_regex_overrides(seg, registry, &mut overrides);
    insert_format_overrides(seg, &mut overrides);

    // `proc NAME …` — the name argument is a function definition.
    if head == "proc"
        && let Some(tok) = seg.argv.get(1)
    {
        overrides
            .entry(tok.span.start())
            .or_insert(ArgOverride::ProcNameDef);
    }

    insert_option_and_subcommand_overrides(seg, registry, &mut overrides);
    insert_switch_regexp_override(seg, &mut overrides);
    insert_role_overrides(seg, registry, arg_texts, &mut overrides);
    insert_oo_body_overrides(seg, inside_oo_body, arg_texts, &mut overrides);

    overrides
}

/// Mark the script-body arguments of a context-sensitive `TclOO` inner
/// definition command (`method` / `constructor` / `destructor` / `self …`
/// / `property -get/-set` / …) as [`ArgOverride::BodyScript`] so they are
/// recursed into rather than emitted as one opaque `string`.
///
/// Only consulted when `inside_oo_body` — i.e. this segment is a top-level
/// word of an `oo::class create … { … }` / `oo::define … { … }` block —
/// so a same-named user proc at top level is never misclassified.  The
/// outer OO commands themselves carry their body roles in the registry
/// (`arg_indices_for_role`, applied by [`insert_role_overrides`]); this
/// covers only the bare inner sub-keywords, which have no `CommandSpec`.
fn insert_oo_body_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    inside_oo_body: bool,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let head = &seg.texts[0];
    if !inside_oo_body || !crate::oo_body::is_inner_oo_definition_command(head) {
        return;
    }
    // `inner_oo_body_indices` indexes the argument words (excluding the
    // head); `seg.argv[idx + 1]` is the representative token (argv[0] is the
    // head).  Only braced (`Str`) bodies recurse.
    for idx in crate::oo_body::inner_oo_body_indices(head, arg_texts) {
        if let Some(tok) = seg.argv.get(idx + 1)
            && matches!(tok.kind, TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::BodyScript);
        }
    }
}

/// Regex pattern / regsub-replacement overrides for a `pattern_type ==
/// Regex` command (option-skipped first positional, and — for `regsub` —
/// the replacement spec two words later).
fn insert_regex_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let head = &seg.texts[0];
    if !registry
        .get(head)
        .and_then(|s| s.pattern_type)
        .is_some_and(|p| p == tcl_registry::patterns::PatternType::Regex)
    {
        return;
    }
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
    // `regsub … exp string subSpec …` — the replacement spec sits two
    // words after the pattern.
    if head == "regsub"
        && let Some(tok) = seg.argv.get(idx + 3)
    {
        overrides.insert(tok.span.start(), ArgOverride::RegsubReplace);
    }
}

/// Conversion-string overrides for the format families: `format`/`scan`
/// (sprintf), `clock format/scan -format`, and `binary format/scan`.
fn insert_format_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let head = &seg.texts[0];

    // `format FMT …` (arg 1) / `scan STR FMT …` (arg 2) — the conversion
    // string.  Command-name gated (the registry's `format_string_type`
    // field is never populated).
    let fmt_word = match head.as_str() {
        "format" if seg.argv.len() >= 2 => Some(1),
        "scan" if seg.argv.len() >= 3 => Some(2),
        _ => None,
    };
    if let Some(w) = fmt_word
        && let Some(tok) = seg.argv.get(w)
    {
        overrides.insert(tok.span.start(), ArgOverride::SprintfFormat);
    }

    // `clock format/scan … -format FMT` — the `-format` option value.
    if head == "clock"
        && seg.texts.len() >= 3
        && matches!(seg.texts[1].as_str(), "format" | "scan")
        && let Some(i) = (2..seg.texts.len()).find(|&i| seg.texts[i] == "-format")
        && let Some(tok) = seg.argv.get(i + 1)
    {
        overrides.insert(tok.span.start(), ArgOverride::ClockFormat);
    }

    // `binary format FMT …` (arg 2) / `binary scan VAL FMT …` (arg 3).
    if head == "binary" && seg.texts.len() >= 3 {
        let bin_word = match seg.texts[1].as_str() {
            "format" => Some(2),
            "scan" if seg.texts.len() >= 4 => Some(3),
            _ => None,
        };
        if let Some(w) = bin_word
            && let Some(tok) = seg.argv.get(w)
        {
            overrides.insert(tok.span.start(), ArgOverride::BinaryFormat);
        }
    }
}

/// Known `-option` switches → `Decorator` (only real options, so `puts
/// -foo` stays a string); subcommand word at arg index 1 → keyword carrying
/// `defaultLibrary`.  Both consult the command's registry spec.
fn insert_option_and_subcommand_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let head = &seg.texts[0];
    let Some(spec) = registry.get(head) else {
        return;
    };
    for (i, text) in seg.texts.iter().enumerate().skip(1) {
        if text.starts_with('-')
            && spec.options.iter().any(|o| o.name == text.as_str())
            && let Some(tok) = seg.argv.get(i)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
    }
    if let Some(sub_text) = seg.texts.get(1)
        && spec.subcommand(sub_text).is_some()
        && let Some(tok) = seg.argv.get(1)
    {
        overrides
            .entry(tok.span.start())
            .or_insert(ArgOverride::SubcommandKeyword);
    }
}

/// `switch -regexp … { pat body … }` — the braced case list (the final
/// word, when option-skipped past `-regexp`/`--`) carries regex patterns.
/// Tag it so `collect_script` sub-tokenises the patterns as regexes and
/// recurses the bodies, rather than treating the whole list as one opaque
/// body.
fn insert_switch_regexp_override(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if seg.texts[0] != "switch" {
        return;
    }
    let mut i = 1;
    let mut is_regexp = false;
    while i < seg.texts.len() && seg.texts[i].starts_with('-') {
        if seg.texts[i] == "-regexp" {
            is_regexp = true;
        }
        if seg.texts[i] == "--" {
            i += 1;
            break;
        }
        i += 1;
    }
    // Skip the switch value/string argument; the case list is the last
    // word (braced-list form only — the inline `pat body …` form has
    // more than one trailing word).
    let case_idx = i + 1;
    if is_regexp
        && case_idx == seg.texts.len() - 1
        && seg
            .argv
            .get(case_idx)
            .is_some_and(|t| matches!(t.kind, TokenType::Str))
        && let Some(tok) = seg.argv.get(case_idx)
    {
        overrides.insert(tok.span.start(), ArgOverride::SwitchRegexpCaseList);
    }
}

/// Registry-driven role overrides: body / expr braced arguments (recursed
/// into rather than emitted opaque) and structural keyword words.  Added
/// last with `or_insert` so the more specific regex/format overrides win.
fn insert_role_overrides(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    registry: &CommandRegistry,
    arg_texts: &[&str],
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let head = &seg.texts[0];
    // `if {expr} {body}`, `proc n a {body}`, `while {expr} {body}`,
    // `expr {expr}`, … — keyed on each word's representative token
    // (`argv[i + 1]`; `argv[0]` is the head).  Only braced (`Str`) words
    // recurse; non-literal words fall through.
    for (role, ov) in [
        (tcl_registry::ArgRole::Body, ArgOverride::BodyScript),
        (tcl_registry::ArgRole::Expr, ArgOverride::ExprScript),
    ] {
        for i in registry.arg_indices_for_role(head, arg_texts, role) {
            if let Some(tok) = seg.argv.get(i + 1)
                && matches!(tok.kind, TokenType::Str)
            {
                overrides.entry(tok.span.start()).or_insert(ov);
            }
        }
    }

    // Structural keyword words (`if`'s then/elseif/else, `try`'s
    // on/trap/finally) sit at argument positions, not the command-name
    // slot, so the default classifier would render them as strings.  The
    // registry's `Keyword` role marks them; highlight as keywords.  Unlike
    // body/expr these are bare (`Esc`) or quoted (`Str`) literal words, so
    // no `Str`-only guard.
    for i in registry.arg_indices_for_role(head, arg_texts, tcl_registry::ArgRole::Keyword) {
        if let Some(tok) = seg.argv.get(i + 1)
            && matches!(tok.kind, TokenType::Esc | TokenType::Str)
        {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::KeywordArg);
        }
    }
}

/// Sub-tokenise a `binary format`/`scan` field string into its
/// specifiers: digit runs → `BinaryCount`, specifier letters →
/// `BinarySpec`, a `u`/`s` modifier after an integer specifier (Tcl 8.5+)
/// or a trailing `*` → `BinaryFlag`.  Whitespace and unrecognised
/// characters are skipped.  Returns `false` when nothing was emitted.
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
                source,
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
            source,
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
                source,
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
            push_subtoken(
                source,
                line_index,
                cstart + i,
                "*",
                TokenKind::BinaryFlag,
                entries,
            );
            emitted = true;
            i += 1;
        }
    }
    emitted
}

/// Sub-tokenise a `clock format`/`scan` field string into its `%`
/// specifiers (`ClockPercent` + optional `ClockModifier` + `ClockSpec`),
/// literal runs classified as `string`.  Returns `false` when there are
/// no specifiers.
fn push_clock_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        // `%(?:[EO])?<spec>` — an optional `E`/`O` locale modifier only
        // counts when it precedes a spec letter (else the `E`/`O` is
        // itself the spec, as both are in the spec set).
        if bytes[i] == b'%' {
            let mut spec = i + 1;
            let modifier = (matches!(bytes.get(spec), Some(b'E' | b'O'))
                && bytes.get(spec + 1).copied().is_some_and(is_clock_spec))
            .then(|| {
                let m = spec;
                spec += 1;
                m
            });
            if bytes.get(spec).copied().is_some_and(is_clock_spec) {
                flush_run(pos, cstart, inner, run, i, TokenKind::String, entries);
                push_subtoken(
                    source,
                    line_index,
                    cstart + i,
                    "%",
                    TokenKind::ClockPercent,
                    entries,
                );
                if let Some(m) = modifier {
                    push_subtoken(
                        source,
                        line_index,
                        cstart + m,
                        &inner[m..=m],
                        TokenKind::ClockModifier,
                        entries,
                    );
                }
                push_subtoken(
                    source,
                    line_index,
                    cstart + spec,
                    &inner[spec..=spec],
                    TokenKind::ClockSpec,
                    entries,
                );
                emitted = true;
                i = spec + 1;
                run = i;
                continue;
            }
        }
        i += 1;
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// `clock format`/`scan` specifier letters (and `%`).
fn is_clock_spec(b: u8) -> bool {
    matches!(
        b,
        b'a' | b'A'
            | b'b'
            | b'B'
            | b'c'
            | b'C'
            | b'd'
            | b'D'
            | b'e'
            | b'E'
            | b'g'
            | b'G'
            | b'h'
            | b'H'
            | b'I'
            | b'j'
            | b'J'
            | b'k'
            | b'l'
            | b'm'
            | b'M'
            | b'N'
            | b'O'
            | b'p'
            | b'P'
            | b'q'
            | b'Q'
            | b's'
            | b'S'
            | b'u'
            | b'U'
            | b'V'
            | b'w'
            | b'W'
            | b'x'
            | b'X'
            | b'y'
            | b'Y'
            | b'z'
            | b'Z'
            | b'%'
    )
}

/// Sub-tokenise a `format`/`scan` conversion string into its `%`
/// specifier components (`FormatPercent` / `FormatFlag` / `FormatWidth`
/// / `FormatSpec`), with literal runs classified as `string`.  Returns
/// `false` (emitting nothing) when there are no `%` specifiers.
fn push_sprintf_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos_ctx = TokenPositionContext { source, line_index };
    let mut emitted = false;
    let mut run = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(cuts) = parse_sprintf_cuts(bytes, i)
        {
            flush_run(pos_ctx, cstart, inner, run, i, TokenKind::String, entries);
            let mut pos = i;
            for (end, kind) in cuts {
                emit_part(pos_ctx, cstart, inner, &mut pos, end, kind, entries);
            }
            emitted = true;
            i = pos;
            run = i;
            continue;
        }
        i += 1;
    }
    if !emitted {
        return false;
    }
    flush_run(
        pos_ctx,
        cstart,
        inner,
        run,
        inner.len(),
        TokenKind::String,
        entries,
    );
    true
}

/// `format`/`scan` conversion type letters.
fn is_sprintf_type(b: u8) -> bool {
    matches!(
        b,
        b'a' | b'A'
            | b'b'
            | b'B'
            | b'c'
            | b'd'
            | b'i'
            | b'e'
            | b'E'
            | b'f'
            | b'g'
            | b'G'
            | b'o'
            | b's'
            | b'u'
            | b'x'
            | b'X'
            | b'%'
    )
}

/// Parse one `%`-specifier at `b[start]` into its component
/// `(end, kind)` cuts (monotonic ends, consumed in order by
/// [`emit_part`]), or `None` when it isn't a valid conversion (no type
/// letter — the `%` is then a literal).
fn parse_sprintf_cuts(b: &[u8], start: usize) -> Option<Vec<(usize, TokenKind)>> {
    let n = b.len();
    let mut cuts: Vec<(usize, TokenKind)> = Vec::new();
    let mut j = start + 1;
    cuts.push((j, TokenKind::FormatPercent)); // `%`

    // Positional `<digits>$` (or `<digits>\$`).
    let pos_start = j;
    while j < n && b[j].is_ascii_digit() {
        j += 1;
    }
    if j > pos_start {
        let mut k = j;
        if b.get(k) == Some(&b'\\') {
            k += 1;
        }
        if b.get(k) == Some(&b'$') {
            cuts.push((j, TokenKind::FormatWidth)); // position digits
            cuts.push((k + 1, TokenKind::FormatPercent)); // `\`?`$`
            j = k + 1;
        } else {
            j = pos_start; // not positional — the digits are the width
        }
    }

    // Flags `[-+ 0#]*`.
    let flags_start = j;
    while j < n && matches!(b[j], b'-' | b'+' | b' ' | b'0' | b'#') {
        j += 1;
    }
    if j > flags_start {
        cuts.push((j, TokenKind::FormatFlag));
    }

    // Width `*` | digits.
    let width_start = j;
    if b.get(j) == Some(&b'*') {
        j += 1;
    } else {
        while j < n && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    if j > width_start {
        let kind = digit_or_flag(b[width_start]);
        cuts.push((j, kind));
    }

    // Precision `.` then `*` | digits.  The separator is matched as a
    // literal `.` — the actual sprintf precision separator — not any
    // character, so a malformed `%5,3d` stays a plain string rather than
    // being mis-split into width-5 / precision-3 (highlighting only).
    if b.get(j) == Some(&b'.') {
        let value_start = j + 1;
        let mut k = value_start;
        if b.get(k) == Some(&b'*') {
            k += 1;
        } else {
            while k < n && b[k].is_ascii_digit() {
                k += 1;
            }
        }
        cuts.push((value_start, TokenKind::FormatFlag)); // the `.`
        if k > value_start {
            cuts.push((k, digit_or_flag(b[value_start])));
        }
        j = k;
    }

    // Length modifier `[hlLzq]`.
    if j < n && matches!(b[j], b'h' | b'l' | b'L' | b'z' | b'q') {
        j += 1;
        cuts.push((j, TokenKind::FormatFlag));
    }

    // Conversion type — required.
    if j < n && is_sprintf_type(b[j]) {
        cuts.push((j + 1, TokenKind::FormatSpec));
        Some(cuts)
    } else {
        None
    }
}

/// `FormatWidth` for a digit, `FormatFlag` for `*` (variable width/prec).
fn digit_or_flag(first: u8) -> TokenKind {
    if first.is_ascii_digit() {
        TokenKind::FormatWidth
    } else {
        TokenKind::FormatFlag
    }
}

/// Emit `inner[*pos..end]` (absolute offset `cstart + *pos`) as `kind`
/// and advance `*pos`, when non-empty.  The sub-token cursor helper for
/// [`push_sprintf_subtokens`].
fn emit_part(
    pos_ctx: TokenPositionContext<'_>,
    cstart: usize,
    inner: &str,
    pos: &mut usize,
    end: usize,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    if end > *pos {
        push_subtoken(
            pos_ctx.source,
            pos_ctx.line_index,
            cstart + *pos,
            &inner[*pos..end],
            kind,
            entries,
        );
        *pos = end;
    }
}

/// Sub-tokenise a regex pattern token into ARE components (groups,
/// character classes, quantifiers, anchors, escapes, backreferences,
/// alternation), with the literal runs between them classified as
/// `regexp`.  Returns `false` (emitting nothing) when the token isn't a
/// braced/quoted literal or contains no metacharacters — the caller then
/// falls back to a single `regexp` token.
fn push_regex_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, inner)) = subspec_content(source, tok) else {
        return false;
    };
    let bytes = inner.as_bytes();
    let pos_ctx = TokenPositionContext { source, line_index };
    let mut matched_any = false;
    let mut pos = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(end) = scan_are_token(bytes, i) {
            flush_run(pos_ctx, cstart, inner, pos, i, TokenKind::Regexp, entries);
            let kind = classify_regex_component(&inner[i..end]);
            push_subtoken(
                source,
                line_index,
                cstart + i,
                &inner[i..end],
                kind,
                entries,
            );
            matched_any = true;
            i = end;
            pos = i;
        } else {
            i += 1;
        }
    }
    if !matched_any {
        return false;
    }
    if pos < inner.len() {
        push_subtoken(
            source,
            line_index,
            cstart + pos,
            &inner[pos..],
            TokenKind::Regexp,
            entries,
        );
    }
    true
}

/// Recognise one ARE metacharacter construct starting at `b[i]`,
/// returning its exclusive end, or `None` when `b[i]` is a literal
/// character.
fn scan_are_token(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    match b[i] {
        b'(' => {
            if b.get(i + 1) != Some(&b'?') {
                return Some(i + 1); // group open
            }
            // non-capturing / lookaround open: `(?:` `(?=` `(?!` `(?>`
            if let Some(b':' | b'=' | b'!' | b'>') = b.get(i + 2) {
                return Some(i + 3);
            }
            // embedded flags `(?imsx-imsx)`
            let mut j = i + 2;
            while j < len
                && matches!(
                    b[j],
                    b'i' | b'm' | b'n' | b's' | b'x' | b'w' | b'p' | b'q' | b'-'
                )
            {
                j += 1;
            }
            // Closed flag group → the whole `(?…)`; else just `(`.
            if b.get(j) == Some(&b')') {
                Some(j + 1)
            } else {
                Some(i + 1)
            }
        }
        b')' | b'|' | b'^' | b'$' | b'.' => Some(i + 1),
        b'*' | b'+' | b'?' => Some(if b.get(i + 1) == Some(&b'?') {
            i + 2
        } else {
            i + 1
        }),
        b'[' => scan_are_class(b, i),
        b'{' => scan_are_brace_quant(b, i),
        b'\\' if i + 1 < len => scan_are_escape(b, i),
        _ => None,
    }
}

/// Scan a bracket expression `[…]` starting at `b[i] == '['`.
/// `[` optional `^` optional leading `]` then `([^]\\]|\\.)* ]`.
fn scan_are_class(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let mut j = i + 1;
    if b.get(j) == Some(&b'^') {
        j += 1;
    }
    if b.get(j) == Some(&b']') {
        j += 1;
    }
    while j < len && b[j] != b']' {
        j += if b[j] == b'\\' && j + 1 < len { 2 } else { 1 };
    }
    (j < len).then_some(j + 1) // unterminated class → not a token
}

/// Scan a brace quantifier `{n}` / `{n,}` / `{n,m}` at `b[i] == '{'`.
fn scan_are_brace_quant(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let mut j = i + 1;
    let digits = j;
    while j < len && b[j].is_ascii_digit() {
        j += 1;
    }
    if j == digits {
        return None;
    }
    if b.get(j) == Some(&b',') {
        j += 1;
        while j < len && b[j].is_ascii_digit() {
            j += 1;
        }
    }
    (b.get(j) == Some(&b'}')).then_some(j + 1)
}

/// Scan a backslash escape at `b[i] == '\\'` (caller guarantees `i + 1`
/// is in bounds): a two-char class/anchor/backref/escaped-metachar, or a
/// `\xHH` / `\uHHHH` / `\UHHHHHHHH` hex escape.
fn scan_are_escape(b: &[u8], i: usize) -> Option<usize> {
    let len = b.len();
    let esc = b[i + 1];
    match esc {
        // class shortcuts / anchors / backref / escaped metachar /
        // escape sequence — all two characters.
        b'A'
        | b'b'
        | b'B'
        | b'd'
        | b'D'
        | b'm'
        | b'M'
        | b's'
        | b'S'
        | b'w'
        | b'W'
        | b'y'
        | b'Y'
        | b'Z'
        | b'0'..=b'9'
        | b'a'
        | b'e'
        | b'f'
        | b'n'
        | b'r'
        | b't'
        | b'v'
        | b'.'
        | b'*'
        | b'+'
        | b'?'
        | b'('
        | b')'
        | b'{'
        | b'}'
        | b'['
        | b']'
        | b'|'
        | b'^'
        | b'$'
        | b'\\' => Some(i + 2),
        // `\xHH` (1-2 hex), `\uHHHH` (1-4), `\UHHHHHHHH` (1-8).
        b'x' | b'u' | b'U' => {
            let max = match esc {
                b'x' => 2,
                b'u' => 4,
                _ => 8,
            };
            let mut j = i + 2;
            while j < len && j < i + 2 + max && b[j].is_ascii_hexdigit() {
                j += 1;
            }
            // Requires at least one hex digit, else not a token.
            (j > i + 2).then_some(j)
        }
        _ => None, // `\` before an unrecognised char → literal
    }
}

/// Classify a single ARE metacharacter run.
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
/// `text`.  Skips empty runs; a multi-line run is split into one entry per
/// covered line (see [`push_span_entries`]).
fn push_subtoken(
    source: &str,
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    entries: &mut Vec<Entry>,
) {
    push_span_entries(source, line_index, abs_off, text, kind, 0, entries);
}

/// Emit token [`Entry`] values for `text` at absolute byte offset `abs_off`.
///
/// The LSP semantic-tokens encoding cannot represent a single token spanning
/// a newline (each token carries only a length, not an end position), so a
/// multi-line token is split into one entry per covered line, each covering
/// that line's slice of the token.  This keeps multi-line literals — braced
/// (`{…}`) or quoted (`"…"`) strings that span lines (issue #757) — highlighted
/// rather than dropped.  Empty per-line slices (blank lines, the trailing
/// slice after a final newline) are skipped, and the newline / `\r` bytes
/// themselves are never covered.
fn push_span_entries(
    source: &str,
    line_index: &LineIndex,
    abs_off: usize,
    text: &str,
    kind: TokenKind,
    modifiers: u32,
    entries: &mut Vec<Entry>,
) {
    if text.is_empty() {
        return;
    }
    if !text.contains('\n') {
        let pos = line_index.position_at_utf16(u32::try_from(abs_off).unwrap_or(0), source);
        entries.push((
            pos.line,
            pos.character.get(),
            utf16_len(text),
            kind,
            modifiers,
        ));
        return;
    }
    let mut off = 0usize;
    for line in text.split_inclusive('\n') {
        let seg = line.strip_suffix('\n').unwrap_or(line);
        let seg = seg.strip_suffix('\r').unwrap_or(seg);
        if !seg.is_empty() {
            let pos =
                line_index.position_at_utf16(u32::try_from(abs_off + off).unwrap_or(0), source);
            entries.push((
                pos.line,
                pos.character.get(),
                utf16_len(seg),
                kind,
                modifiers,
            ));
        }
        off += line.len();
    }
}

/// Maximum body / expr / command-substitution recursion depth — guards
/// against pathological nesting.
const MAX_TOKEN_RECURSION: u32 = 32;

/// Emit the command-head token, splitting a namespace-qualified head
/// (`oo::class`, `::set`) into a `namespace` token for the leading
/// `…::` prefix plus a command token for the final segment.  A bare head
/// is emitted whole, carrying `defaultLibrary` when it resolves to a
/// registry built-in.
/// Sub-tokenise the braced case list of `switch -regexp … { pat body … }`.
///
/// The inner script is re-segmented into commands; the words are flattened
/// across all command lines and paired (even index → pattern, odd index →
/// body), since a Tcl `switch` case list is one flat list whose line breaks
/// are insignificant whitespace.  Pattern words (except the literal
/// `default`) are sub-tokenised as regexes; body words are recursed as
/// scripts.
/// Immutable context threaded through the recursive script-tokenisation
/// walk.  Bundling these read-only borrows keeps each recursive helper to a
/// small, focused signature (the mutable `entries` sink and the `depth`
/// guard stay explicit parameters).
#[derive(Clone, Copy)]
struct ScriptCtx<'a> {
    full_source: &'a str,
    dialect: &'a str,
    registry: &'a CommandRegistry,
    line_index: &'a LineIndex,
    /// Whether this script is a `TclOO` definition body (an
    /// `oo::class create … { … }` / `oo::define … { … }` block).  Inside
    /// one, the context-sensitive OO sub-keywords (`method`, `constructor`,
    /// `property`, `self …`, …) carry script bodies that must be recursed
    /// into — see [`crate::oo_body`].  Outside one, a same-named user proc
    /// must not be treated as an OO definition.
    inside_oo_body: bool,
}

fn collect_switch_regexp_case_list(
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    if depth > MAX_TOKEN_RECURSION {
        return;
    }
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        return;
    };
    // Flatten every word across the (possibly multi-line) case list.
    let mut words: Vec<(Token, String)> = Vec::new();
    for seg in segment_commands_with_offset_and_config(
        inner,
        u32::try_from(cstart).unwrap_or(0),
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    ) {
        for (i, t) in seg.argv.iter().enumerate() {
            let text = seg.texts.get(i).cloned().unwrap_or_default();
            words.push((*t, text));
        }
    }
    for (idx, (word_tok, text)) in words.iter().enumerate() {
        if idx % 2 == 0 {
            // Pattern element — regex unless it is the `default` keyword.
            if text == "default" {
                if let Some(kind) = classify_arg_token(*word_tok, full_source) {
                    push_token(line_index, full_source, *word_tok, kind, 0, entries);
                }
            } else if !push_regex_subtokens(line_index, full_source, *word_tok, entries) {
                push_token(
                    line_index,
                    full_source,
                    *word_tok,
                    TokenKind::Regexp,
                    0,
                    entries,
                );
            }
        } else if let Some((bstart, body)) = subspec_content(full_source, *word_tok) {
            // Body element — recurse as a script.
            collect_script(
                ctx,
                body,
                u32::try_from(bstart).unwrap_or(0),
                entries,
                depth + 1,
            );
        } else if let Some(kind) = classify_arg_token(*word_tok, full_source) {
            push_token(line_index, full_source, *word_tok, kind, 0, entries);
        }
    }
}

fn emit_command_head(
    line_index: &LineIndex,
    full_source: &str,
    head_tok: Token,
    head_text: &str,
    registry: &CommandRegistry,
    entries: &mut Vec<Entry>,
) {
    let full_kind = classify_command_head(head_text, registry);
    // Split any `…::name` head (namespace-qualified command or keyword) into a
    // namespace prefix + final-segment command token.
    if head_text.contains("::")
        && let Some(idx) = head_text.rfind("::")
    {
        // Byte length of the `…::` prefix (head_text bytes == span bytes).
        let prefix_len = u32::try_from(idx + 2).unwrap_or(0);
        let start = head_tok.span.start();
        // Namespace prefix token.
        push_token(
            line_index,
            full_source,
            Token {
                span: tcl_lexer::Span::new(start, start + prefix_len),
                ..head_tok
            },
            TokenKind::Namespace,
            0,
            entries,
        );
        // Final-segment command token: keyword when the full name is a
        // language keyword (TclOO `oo::class` etc.), else function;
        // `defaultLibrary` when the full name is a registry built-in.
        let tail = &head_text[idx + 2..];
        let is_keyword = registry.get(head_text).is_some_and(|s| {
            s.traits
                .contains(tcl_registry::prelude::Traits::LANGUAGE_KEYWORD)
        }) || LANGUAGE_KEYWORD_SUB_KEYWORDS.contains(&tail);
        let kind = if is_keyword {
            TokenKind::Keyword
        } else {
            TokenKind::Function
        };
        let mods = if kind == TokenKind::Function && registry.get(head_text).is_some() {
            MOD_DEFAULT_LIBRARY
        } else {
            0
        };
        push_token(
            line_index,
            full_source,
            Token {
                span: tcl_lexer::Span::new(start + prefix_len, head_tok.span.end()),
                ..head_tok
            },
            kind,
            mods,
            entries,
        );
        return;
    }
    let mods = if full_kind == TokenKind::Function && registry.get(head_text).is_some() {
        MOD_DEFAULT_LIBRARY
    } else {
        0
    };
    push_token(line_index, full_source, head_tok, full_kind, mods, entries);
}

/// Segment `text` (anchored at absolute byte `base_offset` within
/// `full_source`) into commands and push a semantic-token [`Entry`] for each
/// token, recursing into braced bodies (`ArgRole::Body`), braced expressions
/// (`ArgRole::Expr`), and `[…]` command substitutions.  Token spans are
/// already absolute (the segmenter shifts them by `base_offset`), so positions
/// and text are resolved against `full_source` + `line_index`.
fn collect_script(
    ctx: ScriptCtx<'_>,
    text: &str,
    base_offset: u32,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    if depth > MAX_TOKEN_RECURSION {
        return;
    }
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let registry = ctx.registry;
    for seg in segment_commands_with_offset_and_config(
        text,
        base_offset,
        tcl_lexer::LexerConfig::for_dialect(ctx.dialect),
    ) {
        if seg.argv.is_empty() {
            continue;
        }
        // Classify the command-head token.  A head that resolves to a registry
        // built-in carries the `defaultLibrary` modifier.
        let head_tok = seg.argv[0];
        let head_text = &seg.texts[0];
        emit_command_head(
            line_index,
            full_source,
            head_tok,
            head_text,
            registry,
            entries,
        );

        // The command's argument words (head excluded), borrowed once as
        // `&[&str]` and shared by every registry-driven pass below — the
        // override builder and the OO-body context check both need it, and
        // the registry API takes `&[&str]`, so building it here keeps the
        // hot path to a single bridging allocation per command.
        let arg_texts: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();

        let overrides = special_arg_kinds(&seg, registry, ctx.inside_oo_body, &arg_texts);

        // The `inside_oo_body` context the recursion into THIS command's
        // body arguments should carry: an outer OO definition body switches
        // it on, an inner OO command (inside an OO body) switches it off,
        // everything else inherits.  `oo::define`/`oo::objdefine` only switch
        // it on for their bare script form, not their member (`method …`)
        // forms — hence the args are consulted.  Command substitutions and
        // expressions always run in ordinary (non-definition) context.
        let next_oo = crate::oo_body::next_inside_oo_body(
            head_text,
            &arg_texts,
            ctx.inside_oo_body,
            registry,
        );
        let body_ctx = ScriptCtx {
            inside_oo_body: next_oo,
            ..ctx
        };

        for tok in &seg.all_tokens {
            if tok.span == head_tok.span {
                continue;
            }
            emit_arg_token(
                ctx,
                body_ctx,
                *tok,
                overrides.get(&tok.span.start()),
                entries,
                depth,
            );
        }
    }
}

/// Emit semantic-token entries for a single non-head argument token,
/// dispatching on its [`ArgOverride`] (or falling back to default
/// classification) and recursing into braced bodies / expressions /
/// command substitutions.  Extracted from [`collect_script`] to keep that
/// function's body small.
/// When `cond` holds, classify `tok` with [`classify_arg_token`] and, if it
/// yields a kind, push a plain token.  Used as the fallback for the
/// sub-tokenising format overrides (sprintf / clock / binary / regsub) when
/// the specialised sub-lexer declined to emit anything.
fn classify_and_push_if(cond: bool, ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>) {
    if cond && let Some(kind) = classify_arg_token(tok, ctx.full_source) {
        push_token(ctx.line_index, ctx.full_source, tok, kind, 0, entries);
    }
}

fn emit_arg_token(
    ctx: ScriptCtx<'_>,
    body_ctx: ScriptCtx<'_>,
    tok: Token,
    override_kind: Option<&ArgOverride>,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let tok = &tok;
    // Command substitutions / expressions never run in OO definition
    // context, whatever the enclosing command is.
    let plain_ctx = ScriptCtx {
        inside_oo_body: false,
        ..ctx
    };
    match override_kind {
        Some(ArgOverride::RegexPattern) => {
            if !push_regex_subtokens(line_index, full_source, *tok, entries) {
                push_token(line_index, full_source, *tok, TokenKind::Regexp, 0, entries);
            }
        }
        Some(ArgOverride::SprintfFormat) => {
            let emitted = push_sprintf_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::ClockFormat) => {
            let emitted = push_clock_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::BinaryFormat) => {
            let emitted =
                push_binary_subtokens(line_index, full_source, *tok, ctx.dialect, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::RegsubReplace) => {
            let emitted = push_regsub_subtokens(line_index, full_source, *tok, entries);
            classify_and_push_if(!emitted, ctx, *tok, entries);
        }
        Some(ArgOverride::Kind(kind)) => {
            push_token(line_index, full_source, *tok, *kind, 0, entries);
        }
        Some(ArgOverride::Decorator) => {
            push_token(
                line_index,
                full_source,
                *tok,
                TokenKind::Decorator,
                0,
                entries,
            );
        }
        Some(ArgOverride::SubcommandKeyword) => {
            push_token(
                line_index,
                full_source,
                *tok,
                TokenKind::Keyword,
                MOD_DEFAULT_LIBRARY,
                entries,
            );
        }
        Some(ArgOverride::ProcNameDef) => {
            push_token(
                line_index,
                full_source,
                *tok,
                TokenKind::Function,
                MOD_DEFINITION,
                entries,
            );
        }
        Some(ArgOverride::BodyScript) => {
            if let Some((cstart, inner)) = subspec_content(full_source, *tok) {
                // Recurse with the OO-body context computed for this
                // command's bodies (`body_ctx`) so a method / constructor /
                // property-accessor body inside a class definition is walked
                // as ordinary code, while the class body itself stays in OO
                // context.
                collect_script(
                    body_ctx,
                    inner,
                    u32::try_from(cstart).unwrap_or(0),
                    entries,
                    depth + 1,
                );
            } else if let Some(kind) = classify_arg_token(*tok, full_source) {
                push_token(line_index, full_source, *tok, kind, 0, entries);
            }
        }
        Some(ArgOverride::ExprScript) => {
            collect_expr(plain_ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::SwitchRegexpCaseList) => {
            collect_switch_regexp_case_list(body_ctx, *tok, entries, depth + 1);
        }
        Some(ArgOverride::KeywordArg) => {
            push_keyword_arg(line_index, full_source, *tok, entries);
        }
        None => emit_default_arg_token(plain_ctx, *tok, entries, depth),
    }
}

/// Handle an argument token with no [`ArgOverride`]: recurse into a `[…]`
/// command substitution, or classify a plain word (splitting backslash
/// escapes out of string literals).  Extracted from [`emit_arg_token`].
fn emit_default_arg_token(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>, depth: u32) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    if matches!(tok.kind, TokenType::Cmd) {
        // Command substitution `[…]` — recurse into the inner
        // script (delimiters stripped via `content_offset`).
        let cstart = tok.span.start() as usize + tok.content_offset as usize;
        let cend = (tok.span.end() as usize).min(full_source.len());
        if cend > cstart
            && let Some(inner) = full_source.get(cstart..cend)
        {
            collect_script(
                ctx,
                inner,
                u32::try_from(cstart).unwrap_or(0),
                entries,
                depth + 1,
            );
        }
    } else if let Some(kind) = classify_arg_token(tok, full_source) {
        // String / bareword args with backslash escapes split
        // into literal `String` runs + `Escape` sub-tokens.
        if kind == TokenKind::String && push_escape_subtokens(line_index, full_source, tok, entries)
        {
            // emitted as sub-tokens
        } else {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
    }
}

/// Tokenise a braced expression argument via the expression sub-lexer,
/// emitting variable / number / operator / function / string / boolean
/// sub-tokens (math functions carry `defaultLibrary`) and recursing into
/// nested `[cmd]` substitutions.
fn collect_expr(ctx: ScriptCtx<'_>, tok: Token, entries: &mut Vec<Entry>, depth: u32) {
    let full_source = ctx.full_source;
    let line_index = ctx.line_index;
    let Some((cstart, inner)) = subspec_content(full_source, tok) else {
        if let Some(kind) = classify_arg_token(tok, full_source) {
            push_token(line_index, full_source, tok, kind, 0, entries);
        }
        return;
    };
    let math = tcl_lexer::expr_math_functions();
    for et in tcl_lexer::tokenise_expr(inner, Some(ctx.dialect)) {
        use tcl_lexer::ExprTokenType as E;
        let abs_start = cstart + et.start as usize;
        match et.kind {
            E::Command => {
                // `[cmd …]` inside the expression — recurse into the inner
                // script (strip the surrounding `[` / `]`).
                let has_open = et.text.starts_with('[');
                let body = et.text.trim_start_matches('[').trim_end_matches(']');
                collect_script(
                    ctx,
                    body,
                    u32::try_from(abs_start + usize::from(has_open)).unwrap_or(0),
                    entries,
                    depth + 1,
                );
            }
            E::Number => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Number,
                    entries,
                );
            }
            E::Variable => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Variable,
                    entries,
                );
            }
            E::Operator => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Operator,
                    entries,
                );
            }
            E::String => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::String,
                    entries,
                );
            }
            E::Bool => {
                push_subtoken(
                    full_source,
                    line_index,
                    abs_start,
                    &et.text,
                    TokenKind::Keyword,
                    entries,
                );
            }
            E::Function if !et.text.is_empty() && !et.text.contains('\n') => {
                let pos = line_index
                    .position_at_utf16(u32::try_from(abs_start).unwrap_or(0), full_source);
                let mods = if math.contains(et.text.as_str()) {
                    MOD_DEFAULT_LIBRARY
                } else {
                    0
                };
                entries.push((
                    pos.line,
                    pos.character.get(),
                    utf16_len(&et.text),
                    TokenKind::Function,
                    mods,
                ));
            }
            _ => {}
        }
    }
}

/// Walk the segmenter + comment scan and return raw
/// [`Entry`] tuples sorted by position.  Shared by `full` and `range`.
fn collect_entries(source: &str, dialect: &str, registry: &CommandRegistry) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let line_index = LineIndex::new(source);

    // Walk every segmented command (recursing into braced bodies, braced
    // expressions, and `[…]` command substitutions) and classify each token.
    let ctx = ScriptCtx {
        full_source: source,
        dialect,
        registry,
        line_index: &line_index,
        inside_oo_body: false,
    };
    collect_script(ctx, source, 0, &mut entries, 0);

    // Comments aren't in the segmenter's command stream
    // (it strips them).  Scan the source for `#` comments
    // separately.
    push_comment_tokens(source, &line_index, &mut entries);

    // BIG-IP object references (iRules dialect): overlay `object` tokens
    // at recognised pool / data-group / virtual / … name positions.
    // Skipped when an entry already covers the position (e.g. a
    // single-line body's enclosing `string` token) so the token stream
    // never carries overlaps.  Multi-line bodies aren't tokenised by the
    // main walk, so refs inside them surface cleanly.
    if dialect == "f5-irules" {
        for span in crate::irules_object_refs::object_ref_spans(source, registry) {
            push_object_token(source, &line_index, span, &mut entries);
        }
    }

    // Sort by (line, column) so the delta encoding works.
    entries.sort_by_key(|(line, col, _, _, _)| (*line, *col));
    entries
}

/// Push a BIG-IP `object` token for `span`, unless an existing entry on
/// the same line already overlaps its column range (keeps the stream
/// overlap-free).
fn push_object_token(
    source: &str,
    line_index: &LineIndex,
    span: tcl_lexer::Span,
    entries: &mut Vec<Entry>,
) {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    if start.line != end.line {
        return;
    }
    let len = end.character.get().saturating_sub(start.character.get());
    if len == 0 {
        return;
    }
    // An object reference is more specific than the generic bareword
    // `string` classification the (now recursive) body walk produces — drop
    // an overlapping `string` entry and emit the object token instead.  A
    // more specific overlapping kind (keyword / function / variable / …)
    // wins and suppresses the object token.
    let mut other_overlap = false;
    entries.retain(|(l, c, ln, kind, _)| {
        let overlaps = *l == start.line
            && *c < start.character.get() + len
            && start.character.get() < *c + *ln;
        if overlaps {
            if *kind == TokenKind::String {
                return false;
            }
            other_overlap = true;
        }
        true
    });
    if !other_overlap {
        entries.push((start.line, start.character.get(), len, TokenKind::Object, 0));
    }
}

/// Sub-tokenise a string / bareword token's backslash escapes (`\n`, `\t`,
/// `\\`, …): literal runs become `String`, each `\X` becomes `Escape`.
/// Returns `false` (emitting nothing) when the token carries no backslash, so
/// the caller falls back to a single `String` token.  Multi-line tokens are
/// left to the caller.
fn push_escape_subtokens(
    line_index: &LineIndex,
    source: &str,
    tok: Token,
    entries: &mut Vec<Entry>,
) -> bool {
    let Some((cstart, text)) = subspec_content(source, tok) else {
        return false;
    };
    if !text.contains('\\') || text.contains('\n') {
        return false;
    }
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut run_start = 0;
    let mut emitted = false;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            if i > run_start {
                push_subtoken(
                    source,
                    line_index,
                    cstart + run_start,
                    &text[run_start..i],
                    TokenKind::String,
                    entries,
                );
            }
            // Minimal `\X` (two chars); richer `\uHHHH` widths aren't handled.
            let esc = &text[i..(i + 2).min(text.len())];
            push_subtoken(
                source,
                line_index,
                cstart + i,
                esc,
                TokenKind::Escape,
                entries,
            );
            emitted = true;
            i += 2;
            run_start = i;
        } else {
            i += 1;
        }
    }
    if emitted && run_start < bytes.len() {
        push_subtoken(
            source,
            line_index,
            cstart + run_start,
            &text[run_start..],
            TokenKind::String,
            entries,
        );
    }
    emitted
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
            // heuristic that missed inner fragments.
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
                // Bareword argument words classify as String, so `puts
                // hello` emits the `hello` string token rather than
                // dropping it.
                Some(TokenKind::String)
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
/// a Comment-kind entry.
fn push_comment_tokens(source: &str, line_index: &LineIndex, entries: &mut Vec<Entry>) {
    let bytes = source.as_bytes();
    let mut line_start = true;
    // Byte offset up to which the rest of an already-emitted comment line is
    // skipped.  Derived from `char_indices` so the cursor never desyncs from
    // the iterator — the previous hand-incremented `byte_pos` drifted past the
    // buffer end on multi-comment files, slicing out of bounds (panic).
    let mut skip_until: usize = 0;
    for (idx, c) in source.char_indices() {
        if idx < skip_until {
            continue;
        }
        if c == '\n' {
            line_start = true;
            continue;
        }
        if c.is_whitespace() {
            continue;
        }
        if line_start && c == '#' {
            // Find the end of the comment, honouring backslash line
            // continuation: a physical line ending in an *odd* run of
            // backslashes (before the newline) continues the comment onto the
            // next physical line, matching Tcl's parser (issue #759).  An even
            // run (e.g. `\\`) is an escaped backslash and terminates the line.
            let mut p = idx;
            loop {
                let content_start = p;
                while p < bytes.len() && bytes[p] != b'\n' {
                    p += 1;
                }
                // Trailing backslashes on this physical line, ignoring a CRLF
                // `\r` immediately before the newline.
                let mut end = p;
                if end > content_start && bytes[end - 1] == b'\r' {
                    end -= 1;
                }
                let mut backslashes = 0usize;
                while end > content_start && bytes[end - 1] == b'\\' {
                    backslashes += 1;
                    end -= 1;
                }
                if backslashes % 2 == 1 && p < bytes.len() {
                    p += 1; // consume the `\n` and continue on the next line
                    continue;
                }
                break;
            }
            let comment_start = u32::try_from(idx).unwrap_or(0);
            let pos = line_index.position_at_utf16(comment_start, source);
            // A `#` is only a Tcl comment in command position.  This naive scan
            // can't see command position, but a physical line already covered by
            // an emitted token is inside a multi-line string / braced literal
            // (whose per-line entries are pushed before this scan), so the `#`
            // there is literal text, not a comment.  Suppress the comment to
            // avoid an overlapping token the LSP client would reject (#757).
            let already_covered = entries.iter().any(|(l, c, ln, _, _)| {
                *l == pos.line && *c <= pos.character.get() && pos.character.get() < *c + *ln
            });
            if !already_covered {
                // Emit one entry per covered line: a continuation comment spans
                // several physical lines and the LSP encoding cannot represent a
                // token crossing a newline.  `push_span_entries` also strips the
                // line-ending `\r` from each segment.
                push_span_entries(
                    source,
                    line_index,
                    idx,
                    &source[idx..p],
                    TokenKind::Comment,
                    0,
                    entries,
                );
            }
            // Skip the remainder of the comment; the terminating `\n` (at `p`)
            // is processed normally and resets `line_start`.
            skip_until = p;
            line_start = false;
            continue;
        }
        line_start = false;
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
    let start = span.start();
    let mut end = span.end();
    // The lexer's empty-content clamp (tcl-lexer `parse_quoted`) extends a
    // quoted `Esc` fragment's span by one byte over the `$` / `[` that
    // introduces the *next* substitution token, so `token_text` stays empty
    // while `span.end` lands on the terminator.  That introducer byte
    // belongs to the following `Var` / `Cmd` token; emitting it here would
    // produce overlapping semantic tokens (e.g. `"$x"` → the opening
    // fragment `"$` overlapping the `$x` variable).  A clamped-empty ESC is
    // recognised by `span_len == content_offset + 1` with a `$` / `[` last
    // byte; trim it back to just its leading delimiter (the opening `"`, or
    // nothing when there is no delimiter, e.g. between adjacent `$a$b`).
    if tok.kind == TokenType::Esc
        && end - start == u32::from(tok.content_offset) + 1
        && let Some(&last) = source.as_bytes().get((end - 1) as usize)
        && (last == b'$' || last == b'[')
    {
        end = start + u32::from(tok.content_offset);
    }
    if end <= start {
        return;
    }
    let text = source.get(start as usize..end as usize).unwrap_or("");
    // The LSP encoding wants per-line entries, so a multi-line token (a braced
    // or quoted string literal spanning lines) is split into one entry per
    // line rather than dropped — see [`push_span_entries`] and issue #757.
    push_span_entries(
        source,
        line_index,
        start as usize,
        text,
        kind,
        modifiers,
        entries,
    );
}

/// Emit a structural keyword word (`if`'s then/elseif/else, `try`'s
/// on/trap/finally) as a `Keyword` token.  Offsets past any leading
/// delimiter so a quoted `"else"` — whose span starts on the opening
/// quote — marks `else` rather than `"els`, and trims the matching
/// trailing delimiter.
fn push_keyword_arg(line_index: &LineIndex, source: &str, tok: Token, entries: &mut Vec<Entry>) {
    if let Some((cstart, inner)) = subspec_content(source, tok) {
        let content = inner.trim_end_matches(['"', '}']);
        if !content.is_empty() {
            push_subtoken(
                source,
                line_index,
                cstart,
                content,
                TokenKind::Keyword,
                entries,
            );
            return;
        }
    }
    push_token(line_index, source, tok, TokenKind::Keyword, 0, entries);
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

    /// Decode the packed stream into absolute `(line, col, len)` triples.
    fn decode(src: &str, dialect: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32)> {
        let st = full(src, dialect, registry);
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for c in st.data.chunks(5) {
            let (dl, dc, len) = (c[0], c[1], c[2]);
            if dl > 0 {
                line += dl;
                col = dc;
            } else {
                col += dc;
            }
            out.push((line, col, len));
        }
        out
    }

    /// Assert no two tokens on the same line overlap (next starts at or
    /// after the previous token's end) — the client "Overlapping semantic
    /// tokens detected" invariant.
    fn assert_non_overlapping(src: &str, registry: &CommandRegistry) {
        let toks = decode(src, "tcl", registry);
        for w in toks.windows(2) {
            let (l0, c0, len0) = w[0];
            let (l1, c1, _) = w[1];
            if l0 == l1 {
                assert!(
                    c1 >= c0 + len0,
                    "overlap on line {l0}: token at col {c1} starts before \
                     previous token end {} (src={src:?}, toks={toks:?})",
                    c0 + len0,
                );
            }
        }
    }

    #[test]
    fn quoted_var_at_string_start_no_overlap() {
        // Regression: the lexer's empty-content clamp made the opening `"`
        // fragment span `"$`, overlapping the `$x` variable token.  The
        // opening fragment must shrink to just the `"`.
        let r = reg();
        assert_non_overlapping("puts \"$x y\"\n", &r);
        assert_non_overlapping("set x 1\nputs \"$x — résumé — 日本語\"\n", &r);
        // Adjacent substitutions: the empty ESC between `$a` and `$b`
        // carries no delimiter, so it must vanish entirely (no zero-area
        // overlap at the `$b`).
        assert_non_overlapping("puts \"$a$b\"\n", &r);
        // Command substitution introducer `[` at string start.
        assert_non_overlapping("puts \"[expr {1+2}] z\"\n", &r);
        // Dense line with several adjacent substitutions/strings.
        assert_non_overlapping("set a 1;set b 2;puts \"$a [expr {$a+$b}] $b\";# tail\n", &r);
    }

    #[test]
    fn quoted_string_opening_fragment_is_single_quote() {
        // `puts "$x y"` — the opening string fragment is exactly the `"`
        // (col 5, len 1), not `"$` (len 2).
        let toks = decode("puts \"$x y\"\n", "tcl", &reg());
        // The opening `"` lands at byte/col 5 on line 0 with length 1.
        assert!(
            toks.contains(&(0, 5, 1)),
            "expected a length-1 string token at col 5, got {toks:?}",
        );
    }

    #[test]
    fn known_option_classified_as_decorator() {
        // `regexp -nocase {pat} $s` — `-nocase` is a real option → decorator.
        let ks = kinds("regexp -nocase {pat} $s\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
        // `puts -foo` — `-foo` is not an option of `puts` → not a decorator.
        let ks = kinds("puts -foo\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::Decorator as u32)), "{ks:?}");
    }

    #[test]
    fn operator_command_head_classified_as_operator() {
        // `+ 3 4` — the operator head is `operator`, not `function`.
        let ks = kinds("+ 3 4\n", "tcl", &reg());
        assert_eq!(ks.first(), Some(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    #[test]
    fn bareword_argument_classified_as_string() {
        // `puts hello` → function head + a `string` token for the bareword
        // arg, not a dropped arg.
        let ks = kinds("puts hello\n", "tcl", &reg());
        assert_eq!(ks.len(), 2, "expected head + arg token; got {ks:?}");
        assert!(
            ks.contains(&(TokenKind::String as u32)),
            "bareword arg not classified as string; got {ks:?}"
        );
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
    fn bigip_object_ref_token_in_irules_body() {
        let mut registry = CommandRegistry::build_default();
        registry.load_dialect(tcl_registry::dialects::DialectSet::IRULES);
        // `pool web_pool` inside a multi-line `when` body → `object`.
        let ks = kinds(
            "when HTTP_REQUEST {\n  pool web_pool\n}\n",
            "f5-irules",
            &registry,
        );
        assert!(ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
    }

    #[test]
    fn bigip_object_ref_not_emitted_in_plain_tcl() {
        // The object overlay is iRules-only.
        let ks = kinds("when HTTP_REQUEST {\n  pool web_pool\n}\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::Object as u32)), "{ks:?}");
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
    fn regsub_replacement_backref_subtokens() {
        // `regsub {a} $s {\1-\&} out` → `\1` number, `\&` operator.
        let ks = kinds("regsub {a} $s {\\1-\\&} out\n", "tcl", &reg());
        assert!(ks.contains(&(TokenKind::Number as u32)), "{ks:?}");
        assert!(ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
    }

    #[test]
    fn regsub_replacement_without_backrefs_stays_string() {
        let ks = kinds("regsub {a} $s {plain} out\n", "tcl", &reg());
        assert!(!ks.contains(&(TokenKind::Operator as u32)), "{ks:?}");
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
        // The provider re-segments
        // under the document dialect.  In `foo {*}$x`, on 8.5+ the `{*}`
        // is the expansion operator (consumed — not a highlighted word),
        // but on 8.4 it is a literal braced string `{*}`, which adds an
        // extra `string` token.  So the packed token stream is longer on
        // 8.4.
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
    fn legend_modifiers_order() {
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

    /// Decode the packed stream into `(line, col, len, kind)` tuples plus
    /// the covered source word (ASCII sources only — byte == utf16).
    fn decode_words(src: &str, registry: &CommandRegistry) -> Vec<(u32, u32, u32, u32, String)> {
        let st = full(src, "tcl", registry);
        let lines: Vec<&str> = src.split('\n').collect();
        let mut line = 0u32;
        let mut col = 0u32;
        let mut out = Vec::new();
        for c in st.data.chunks(5) {
            let (dl, dc, len, kind) = (c[0], c[1], c[2], c[3]);
            if dl > 0 {
                line += dl;
                col = dc;
            } else {
                col += dc;
            }
            let word = lines
                .get(line as usize)
                .and_then(|l| l.get(col as usize..(col + len) as usize))
                .unwrap_or("")
                .to_string();
            out.push((line, col, len, kind, word));
        }
        out
    }

    fn keyword_words(src: &str, registry: &CommandRegistry) -> std::collections::HashSet<String> {
        decode_words(src, registry)
            .into_iter()
            .filter(|(_, _, _, kind, _)| *kind == TokenKind::Keyword as u32)
            .map(|(_, _, _, _, word)| word)
            .collect()
    }

    #[test]
    fn if_else_elseif_are_keywords() {
        // else/elseif structural keywords highlight like `if`.
        let src = "if 1 {\n puts a\n} elseif 2 {\n puts b\n} else {\n puts c\n}";
        let kw = keyword_words(src, &reg());
        for expected in ["if", "elseif", "else"] {
            assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
        }
    }

    #[test]
    fn try_on_finally_are_keywords() {
        // try's on/trap/finally structural keywords highlight as keywords.
        let src = "try {\n set x 1\n} on error {e} {\n puts $e\n} finally {\n puts d\n}";
        let kw = keyword_words(src, &reg());
        for expected in ["try", "on", "finally"] {
            assert!(kw.contains(expected), "missing {expected:?} in {kw:?}");
        }
    }

    #[test]
    fn builtin_name_as_bareword_arg_is_string() {
        // A builtin name used as a plain dict value stays a string, not a
        // keyword — the KEYWORD role is position-aware (if/try only).
        let src = "dict set frame proc \"asasdas asd\"";
        let proc = decode_words(src, &reg())
            .into_iter()
            .find(|(_, _, _, _, word)| word == "proc")
            .expect("a `proc` token");
        assert_eq!(proc.3, TokenKind::String as u32, "{proc:?}");
    }

    #[test]
    fn quoted_structural_keyword_offsets_past_quote() {
        // A quoted `"else"` keyword marks `else`, not `"els`.
        let src = "if 0 {} \"else\" {puts ok}";
        let kw = decode_words(src, &reg())
            .into_iter()
            .find(|(_, col, _, kind, _)| *kind == TokenKind::Keyword as u32 && *col >= 8)
            .expect("a keyword token past the first word");
        assert_eq!(kw.4, "else", "{kw:?}");
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
    fn semantic_token_lengths_use_utf16_code_units() {
        let data = full("# 😀x\n", "tcl", &reg()).data;
        assert_eq!(
            &data[..5],
            &[0, 0, 5, TokenKind::Comment as u32, 0],
            "comment token length must count the emoji as two UTF-16 code units",
        );
    }

    #[test]
    fn many_comment_lines_do_not_drift_out_of_bounds() {
        // Regression: `push_comment_tokens` hand-incremented a byte cursor to
        // the end of each comment line while the `chars()` iterator only
        // advanced one char, so the cursor drifted past the buffer and sliced
        // out of bounds (panic) on files with several comment lines.
        use std::fmt::Write as _;
        let mut src = String::new();
        for i in 0..40 {
            let _ = writeln!(src, "# comment line number {i} with some padding text");
        }
        src.push_str("set x 1\n");
        src.push_str("# trailing comment after code, no final newline");
        let st = full(&src, "tcl", &reg()); // must not panic
        let comments = st
            .data
            .chunks(5)
            .filter(|c| c[3] == TokenKind::Comment as u32)
            .count();
        assert_eq!(comments, 41, "expected one token per comment line");
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

    // range variant

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
        assert!(line1_only.data.len().is_multiple_of(5));
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
        // Regression: LSP ranges are
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
