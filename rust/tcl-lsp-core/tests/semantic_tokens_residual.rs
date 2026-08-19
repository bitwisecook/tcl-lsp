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

//! Residual-coverage tests for `tcl-lsp-core/src/semantic_tokens.rs`.
//!
//! The sibling suites (`semantic_tokens.rs`, `lsp_core_residual.rs`)
//! cover the common token kinds; this file drives the *less-common
//! classification branches* the baseline (~88.6% region coverage) leaves cold:
//!
//! * ARE regex sub-tokenisation edges — lookaround / non-capturing groups,
//!   embedded `(?flags)`, brace quantifiers, bracket expressions with a leading
//!   `^`/`]`, hex escapes (`\xHH` / `\uHHHH` / `\UHHHH…`), lazy quantifiers,
//!   two-char anchors, and the unterminated-class fallback.
//! * The `switch -regexp { pat body … }` braced case list
//!   (`collect_switch_regexp_case_list`) — pattern sub-tokenisation, the
//!   literal `default`, and body recursion.
//! * `format`/`scan` `%`-spec edges — positional `%n$`, `*` width / precision,
//!   length modifiers, and the malformed-precision string fallback.
//! * `clock` field strings — locale modifier (`%OS`) and the trailing bare `%`.
//! * `binary` field strings — whitespace skip, unrecognised-specifier skip, and
//!   the `tcl8.4` modifier suppression.
//! * `regsub` replacement specs, namespace-qualified command / proc names,
//!   bareword backslash-escape sub-tokens, quoted structural-keyword args, the
//!   `expr` nested-`[cmd]` / string / boolean arms, and the `f5-irules`
//!   object-reference overlay (single-line vs multi-line bodies).
//!
//! ## Proof discipline
//!
//! Which span an editor paints `keyword` vs `string` vs `regexp` is LSP
//! presentation — tclsh has no opinion — so those facts are asserted
//! **structurally** against the encoder contract (`legend_token_types` order).
//! Where an assertion *also* encodes a Tcl-language fact (a word is a real
//! builtin command / subcommand, a `switch -regexp` case list parses, a regsub
//! backref is meaningful), it is backed by `scripts/dev/tclsh_check.sh` against
//! tclsh **8.6** + **9.0** and cited with a `// tclsh-proof:` comment.

use std::collections::HashSet;

use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::semantic_tokens::{full, legend_token_types, range};
use tcl_registry::{ArgRole, Arity, CommandRegistry, CommandSpec, Traits, registry_for_dialect};

fn reg() -> &'static CommandRegistry {
    registry_for_dialect("tcl8.6")
}

/// An iRules-enabled registry (loads the BIG-IP / iRules dialect on top of the
/// default command set) for the `f5-irules` object-overlay tests.
fn irules_registry() -> CommandRegistry {
    let mut r = CommandRegistry::build_default();
    r.load_dialect(tcl_dialect::DialectSet::IRULES);
    r
}

/// One decoded token: absolute `(line, character, length, type-name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tok {
    line: u32,
    character: u32,
    length: u32,
    ttype: String,
}

/// Decode the packed LSP delta stream into absolute tokens, using an explicit
/// registry (so the iRules cases can pass their dialect-loaded registry).
fn decode_with(source: &str, dialect: &str, registry: &CommandRegistry) -> Vec<Tok> {
    let st = full(source, tcl_dialect::DialectProfile::by_name(dialect), registry);
    assert_eq!(st.data.len() % 5, 0, "stream must be 5-int aligned");
    let legend = legend_token_types();
    let mut out = Vec::new();
    let (mut line, mut character) = (0u32, 0u32);
    for c in st.data.chunks(5) {
        let (dl, dc, length, ty) = (c[0], c[1], c[2], c[3]);
        if dl > 0 {
            line += dl;
            character = dc;
        } else {
            character += dc;
        }
        out.push(Tok {
            line,
            character,
            length,
            ttype: legend[ty as usize].to_owned(),
        });
    }
    out
}

/// Decode under the default `tcl8.6` registry.
fn decode(source: &str, dialect: &str) -> Vec<Tok> {
    decode_with(source, dialect, reg())
}

/// The set of token-type names present in the decoded stream.
fn type_set(source: &str, dialect: &str) -> HashSet<String> {
    decode(source, dialect)
        .into_iter()
        .map(|t| t.ttype)
        .collect()
}

/// All decoded tokens of a given type-name.
fn of_type<'a>(toks: &'a [Tok], ty: &str) -> Vec<&'a Tok> {
    toks.iter().filter(|t| t.ttype == ty).collect()
}

/// The packed-stream type index (legend position) of a token-type name.
fn legend_idx(ty: &str) -> u32 {
    u32::try_from(legend_token_types().iter().position(|t| *t == ty).unwrap()).unwrap()
}

// ===========================================================================
// ARE regex sub-tokenisation — `scan_are_token` / `scan_are_class` /
// `scan_are_brace_quant` / `scan_are_escape` branches reached via `regexp`.
//
// Tcl-language facts asserted here: `regexp` is a real builtin and each pattern
// is a valid ARE that compiles. tclsh-proof: `[llength [info commands regexp]]`
// == 1 and `regexp {<pat>} <str>` runs without error (8.6 + 9.0).
// ===========================================================================

#[test]
fn st_regex_non_capturing_group_is_one_group_token() {
    // `(?:` opens a non-capturing group — the scanner consumes all three bytes
    // as a single `regexpGroup` token (the lookaround/non-capture arm of
    // `scan_are_token`), distinct from a bare `(`.
    // tclsh-proof: regexp {(?:ab)?c} works — `(?:…)` is valid ARE syntax.
    //   tclsh8.6/9.0: `regexp {(?:ab)?c} abc` -> 1
    let toks = decode("regexp {(?:ab)?} $s\n", "tcl8.6");
    let groups = of_type(&toks, "regexpGroup");
    assert!(
        groups.iter().any(|t| t.length == 3),
        "`(?:` should be one length-3 group token; got {toks:?}",
    );
}

#[test]
fn st_regex_embedded_flags_group() {
    // `(?i)` is an embedded-flags directive — the scanner walks the flag
    // letters to the closing `)` and emits the whole `(?i)` as one
    // `regexpGroup` (the closed-flag-group arm).
    // tclsh-proof: tclsh8.6/9.0 `regexp {(?i)ABC} abc` -> 1 (case-insensitive).
    let toks = decode("regexp {(?i)abc} $s\n", "tcl8.6");
    let groups = of_type(&toks, "regexpGroup");
    assert!(
        groups.iter().any(|t| t.length == 4),
        "`(?i)` should be one length-4 group token; got {toks:?}",
    );
}

#[test]
fn st_regex_embedded_flags_unclosed_is_just_open_paren() {
    // `(?ix` with no closing `)` is NOT a flag group — the scanner falls back
    // to emitting just the `(` (length 1), the `else` arm of the embedded-flags
    // branch.
    // tclsh-proof: `(?i)` etc. are valid; here the structural fact is the
    // fallback length, asserted against the encoder.
    let toks = decode("regexp {(?ixabc} $s\n", "tcl8.6");
    let groups = of_type(&toks, "regexpGroup");
    assert!(
        groups.iter().all(|t| t.length == 1) && !groups.is_empty(),
        "unclosed `(?…` should fall back to a length-1 `(`; got {toks:?}",
    );
}

#[test]
fn st_regex_brace_quantifier_is_quantifier_token() {
    // `{2,3}` is a brace quantifier — `scan_are_brace_quant` consumes
    // `{`digits`,`digits`}` as one `regexpQuantifier`.
    // tclsh-proof: tclsh8.6/9.0 `regexp {a{2,3}} aaa` -> 1.
    let toks = decode("regexp {a{2,3}} $s\n", "tcl8.6");
    let quant = of_type(&toks, "regexpQuantifier");
    assert!(
        quant.iter().any(|t| t.length == 5),
        "`{{2,3}}` should be one length-5 quantifier; got {toks:?}",
    );
}

#[test]
fn st_regex_bare_brace_without_digits_is_not_a_quantifier() {
    // `{x}` has no leading digit → `scan_are_brace_quant` returns None and the
    // `{` is a literal regexp char (no quantifier token emitted).
    let toks = decode("regexp {a{x}b} $s\n", "tcl8.6");
    assert!(
        of_type(&toks, "regexpQuantifier").is_empty(),
        "`{{x}}` is not a quantifier; got {toks:?}",
    );
    // The literal run still surfaces as `regexp`.
    assert!(type_set("regexp {a{x}b} $s\n", "tcl8.6").contains("regexp"));
}

#[test]
fn st_regex_bracket_class_with_leading_caret_and_bracket() {
    // `[^]a]` is a negated class whose first member is a literal `]`
    // (`scan_are_class` skips the optional `^` then the optional leading `]`).
    // The whole 5-byte class is one `regexpCharClass`.
    // tclsh-proof: tclsh8.6/9.0 `regexp {[^]a]} b` -> 1 (b is neither `]`/`a`).
    let toks = decode("regexp {[^]a]} $s\n", "tcl8.6");
    let classes = of_type(&toks, "regexpCharClass");
    assert!(
        classes.iter().any(|t| t.length == 5),
        "`[^]a]` should be one length-5 char-class; got {toks:?}",
    );
}

#[test]
fn st_regex_unterminated_class_falls_back_to_plain_regexp() {
    // An unterminated `[` (no closing `]`) makes `scan_are_class` return None;
    // the bytes after the `(` collapse into a single literal `regexp` run with
    // no char-class token.
    let toks = decode("regexp {(unterminated[} $s\n", "tcl8.6");
    assert!(
        of_type(&toks, "regexpCharClass").is_empty(),
        "unterminated `[` must not produce a char-class token; got {toks:?}",
    );
    assert!(
        of_type(&toks, "regexp").iter().any(|t| t.length > 1),
        "the unterminated tail should be one literal regexp run; got {toks:?}",
    );
}

#[test]
fn st_regex_hex_and_unicode_escapes_are_escape_tokens() {
    // `\xff` (2 hex) and `é` (4 hex) are hex escapes — `scan_are_escape`
    // consumes the `\x`/`\u` plus the hex digits as `regexpEscape`.
    // tclsh-proof: tclsh8.6/9.0 `regexp {é} é` -> 1.
    let toks = decode("regexp {\\xff\\u00e9} $s\n", "tcl8.6");
    let esc = of_type(&toks, "regexpEscape");
    assert!(
        esc.iter().any(|t| t.length == 4),
        "`\\xff` should be a length-4 escape; got {toks:?}",
    );
    assert!(
        esc.iter().any(|t| t.length == 6),
        "`\\u00e9` should be a length-6 escape; got {toks:?}",
    );
}

#[test]
fn st_regex_backslash_x_without_hexdigit_is_not_a_token() {
    // `\xZ` has no hex digit after `\x`, so the hex-escape arm requires
    // `j > i + 2` and returns None → no escape token from that `\x`.
    let toks = decode("regexp {a\\xZb} $s\n", "tcl8.6");
    // `\x` with no hex digit yields no `regexpEscape` for it (the `Z`/literal
    // run stays plain regexp). There should be no length-4 hex escape.
    assert!(
        of_type(&toks, "regexpEscape").iter().all(|t| t.length != 4),
        "`\\xZ` is not a 4-char hex escape; got {toks:?}",
    );
}

#[test]
fn st_regex_lazy_quantifiers_consume_trailing_question_mark() {
    // `*?` / `+?` / `??` are lazy quantifiers — `scan_are_token` extends the
    // quantifier by one byte when the next char is `?` (the `i + 2` arm).
    // tclsh-proof: tclsh8.6/9.0 `regexp {a+?} aaa` -> 1.
    let toks = decode("regexp {a*?b+?} $s\n", "tcl8.6");
    let quant = of_type(&toks, "regexpQuantifier");
    assert!(
        quant.iter().filter(|t| t.length == 2).count() >= 2,
        "both `*?` and `+?` should be length-2 lazy quantifiers; got {toks:?}",
    );
}

#[test]
fn st_regex_two_char_anchors_classified_as_anchor() {
    // `\A` (start) and `\Z` (end) are two-char anchors — `scan_are_escape`
    // returns a 2-byte token and `classify_regex_component` maps `A`/`Z` to
    // `regexpAnchor`.
    // tclsh-proof: tclsh8.6/9.0 `regexp {\Afoo\Z} foo` -> 1.
    let toks = decode("regexp {\\Afoo\\Z} $s\n", "tcl8.6");
    let anchors = of_type(&toks, "regexpAnchor");
    assert_eq!(
        anchors.iter().filter(|t| t.length == 2).count(),
        2,
        "`\\A` and `\\Z` should both be length-2 anchors; got {toks:?}",
    );
}

#[test]
fn st_regex_escaped_metachar_is_escape_not_metachar() {
    // `\.` is an escaped literal dot — `scan_are_escape` consumes 2 bytes and
    // `classify_regex_component`'s catch-all maps it to `regexpEscape` (NOT the
    // `.`-as-char-class metachar).
    let toks = decode("regexp {a\\.b} $s\n", "tcl8.6");
    assert!(
        of_type(&toks, "regexpEscape").iter().any(|t| t.length == 2),
        "`\\.` should be a length-2 escape token; got {toks:?}",
    );
}

// ===========================================================================
// switch -regexp braced case list — `collect_switch_case_list` (regexp mode).
//
// tclsh-proof: the case list parses and dispatches by regex. tclsh8.6/9.0:
//   `set x abc; switch -regexp $x {{^a(b)} {set r 1} default {set r 2}}; set r`
//   -> 1
// ===========================================================================

#[test]
fn st_switch_regexp_case_list_subtokenises_patterns_and_recurses_bodies() {
    let src = "switch -regexp $x {\n  {^a(b)} {puts 1}\n  default {puts 2}\n}\n";
    let toks = decode(src, "tcl8.6");
    let kinds: HashSet<&str> = toks.iter().map(|t| t.ttype.as_str()).collect();
    // The pattern `^a(b)` sub-tokenises: `^` anchor, `(` group, `+`/literal.
    assert!(kinds.contains("regexpAnchor"), "{toks:?}");
    assert!(kinds.contains("regexpGroup"), "{toks:?}");
    // Bodies are recursed as scripts → `puts` is a function head on each body
    // line (lines 1 and 2).
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "body 1 should recurse to a function head; got {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.line == 2 && t.ttype == "function"),
        "body 2 (after `default`) should recurse; got {toks:?}",
    );
}

#[test]
fn st_switch_regexp_default_keyword_is_not_a_regex() {
    // `default` matches no text — it is the fall-through arm, so it is a
    // `keyword`, never sub-tokenised as a regex.  The clause-list walk takes it
    // from the registry's `CaseListSpec::keyword_patterns`, the same mechanism
    // that makes Expect's `timeout` / `eof` / `full_buffer` keywords.
    let src = "switch -regexp $x {\n  {^z} {puts 1}\n  default {puts 2}\n}\n";
    let toks = decode(src, "tcl8.6");
    let default_tok = toks
        .iter()
        .find(|t| t.line == 2 && t.character == 2 && t.length == 7)
        .expect("the `default` word token");
    assert_eq!(
        default_tok.ttype, "keyword",
        "`default` is the fall-through keyword; got {toks:?}"
    );
}

#[test]
fn st_switch_regexp_literal_pattern_without_metachars_is_plain_regexp() {
    // A pattern element with no metacharacters (`abc`) takes the
    // `!push_regex_subtokens` fallback in the case-list walk → one `regexp`
    // token covering the whole literal.
    let src = "switch -regexp $x {\n  abc {puts 1}\n}\n";
    let toks = decode(src, "tcl8.6");
    assert!(
        toks.iter()
            .any(|t| t.line == 1 && t.ttype == "regexp" && t.length == 3),
        "literal `abc` pattern should be one length-3 regexp; got {toks:?}",
    );
}

// ===========================================================================
// switch (plain, non-regexp) braced case list — `collect_switch_case_list`
// (exact / glob mode).  Regression for #758: the whole `{ pat body … }` list
// used to be walked as one opaque body, so the case bodies got no tokens.
//
// tclsh-proof: the case list parses and dispatches. tclsh8.6/9.0:
//   `set x b; switch $x {a {set r 1} b {set r 2} default {set r 3}}; set r`
//   -> 2
// ===========================================================================

#[test]
fn st_switch_plain_case_list_recurses_bodies_without_regex_tokens() {
    let src = "switch $x {\n  a {puts 1}\n  default {puts 2}\n}\n";
    let toks = decode(src, "tcl8.6");
    // Bodies are recursed as scripts → `puts` is a function head on each body
    // line (lines 1 and 2).
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "body 1 should recurse to a function head; got {toks:?}",
    );
    assert!(
        toks.iter().any(|t| t.line == 2 && t.ttype == "function"),
        "body 2 (after `default`) should recurse; got {toks:?}",
    );
    // Plain (non-regexp) mode: patterns are literals, not regexes, so no
    // regex sub-token types appear anywhere.
    let kinds: HashSet<&str> = toks.iter().map(|t| t.ttype.as_str()).collect();
    for re in ["regexpAnchor", "regexpGroup", "regexpQuantifier"] {
        assert!(
            !kinds.contains(re),
            "unexpected `{re}` in plain switch; got {toks:?}"
        );
    }
    // The literal pattern `a` is classified as an ordinary word, not a regexp.
    assert!(
        toks.iter()
            .any(|t| t.line == 1 && t.ttype != "regexp" && t.character == 2 && t.length == 1),
        "pattern `a` should be a plain literal token; got {toks:?}",
    );
}

#[test]
fn st_switch_case_list_is_list_parsed_not_script_segmented() {
    // A `switch` case list is a Tcl *list*, not a script: `;` is an ordinary
    // pattern element, not a command separator.  The case-list walk uses the
    // list grammar (`find_element`), so the `;` pattern's body is still paired
    // and recursed. tclsh-proof: tclsh8.6/9.0
    //   `set x {;}; switch -- $x {; {set r semi} default {set r def}}; set r`
    //   -> semi
    let src = "switch -- $x {\n  ; {puts semi}\n  default {puts def}\n}\n";
    let toks = decode(src, "tcl8.6");
    // The `;` pattern element is emitted (as a literal), not dropped.
    assert!(
        toks.iter()
            .any(|t| t.line == 1 && t.character == 2 && t.length == 1),
        "the `;` pattern element should be tokenised, not dropped; got {toks:?}",
    );
    // Its body is recursed → `puts` on line 1 is a function head.
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "the `;` case body should recurse to a function head; got {toks:?}",
    );
}

#[test]
fn st_switch_hash_pattern_is_not_a_comment_and_has_no_overlap() {
    // `#` at the start of a case-list line is a pattern element, not a comment
    // (Tcl's "comments don't work in switch" gotcha).  It must be tokenised as
    // code, its body recursed, and — crucially — the naive comment scanner must
    // not also emit an overlapping comment token over the same line.
    // tclsh-proof: tclsh8.6/9.0
    //   `set x {#}; switch -- $x {# {set r hash} default {set r def}}; set r`
    //   -> hash
    let src = "switch -- $x {\n  # {puts hash}\n  default {puts def}\n}\n";
    let toks = decode(src, "tcl8.6");
    // The `#` case body is recursed → `puts` is a function head on line 1.
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "the `#` case body should recurse to a function head; got {toks:?}",
    );
    // No `comment` token overlays the `#` pattern line.
    assert!(
        !toks.iter().any(|t| t.line == 1 && t.ttype == "comment"),
        "`#` in a switch case list must not be a comment; got {toks:?}",
    );
    // Tokens on line 1 are strictly non-overlapping (no comment/code overlap).
    let mut line1: Vec<&Tok> = toks.iter().filter(|t| t.line == 1).collect();
    line1.sort_by_key(|t| t.character);
    for pair in line1.windows(2) {
        assert!(
            pair[0].character + pair[0].length <= pair[1].character,
            "overlapping tokens on the `#` line: {:?} and {:?}",
            pair[0],
            pair[1],
        );
    }
}

// ===========================================================================
// format / scan %-spec edges — `parse_sprintf_cuts`.
//
// `format` / `scan` are real builtins. tclsh-proof:
//   `[llength [info commands format]]` + `[... scan]` == 2 (8.6 + 9.0).
// The %-decomposition is presentation; assert structurally.
// ===========================================================================

#[test]
fn st_sprintf_positional_argument_specifier() {
    // `%2$d` is a positional conversion: `%`, the position digits `2`
    // (`formatWidth`), the `$` separator (`formatPercent`), then the `d` type.
    let toks = decode("format {%2$d} a b\n", "tcl8.6");
    // Two `formatPercent` tokens: the `%` and the `$` separator.
    assert_eq!(
        of_type(&toks, "formatPercent").len(),
        2,
        "positional spec emits `%` and `$` as formatPercent; got {toks:?}",
    );
    assert!(
        of_type(&toks, "formatWidth").iter().any(|t| t.length == 1),
        "{toks:?}"
    );
    assert!(!of_type(&toks, "formatSpec").is_empty(), "{toks:?}");
}

#[test]
fn st_sprintf_digits_not_followed_by_dollar_are_width_not_position() {
    // `%12d` — the digits are NOT followed by `$`, so the positional branch
    // resets (`j = pos_start`) and the digits become the width run.
    let toks = decode("format {%12d} $x\n", "tcl8.6");
    // Exactly one formatPercent (the `%`), and a width run of length 2.
    assert_eq!(of_type(&toks, "formatPercent").len(), 1, "{toks:?}");
    assert!(
        of_type(&toks, "formatWidth").iter().any(|t| t.length == 2),
        "`12` should be a length-2 width; got {toks:?}",
    );
}

#[test]
fn st_sprintf_star_width_and_precision_are_flags() {
    // `%*.*f` — a `*` width and a `*` precision are variable (the
    // `digit_or_flag` → `FormatFlag` arm), and the `.` separator is a flag too.
    let toks = decode("format {%*.*f} $w $p $x\n", "tcl8.6");
    // At least three formatFlag tokens: `*` width, `.` sep, `*` precision.
    assert!(
        of_type(&toks, "formatFlag").len() >= 3,
        "`*`/`.`/`*` are flags; got {toks:?}",
    );
    assert!(!of_type(&toks, "formatSpec").is_empty(), "{toks:?}");
}

#[test]
fn st_sprintf_length_modifier_is_a_flag() {
    // `%ld` — the `l` length modifier is consumed as a `formatFlag` before the
    // `d` type (the `[hlLzq]` arm).
    let toks = decode("format {%ld} $x\n", "tcl8.6");
    assert!(
        of_type(&toks, "formatFlag").iter().any(|t| t.length == 1),
        "`l` length modifier should be a flag; got {toks:?}",
    );
    assert!(!of_type(&toks, "formatSpec").is_empty(), "{toks:?}");
}

#[test]
fn st_sprintf_malformed_precision_separator_stays_plain_string() {
    // `%5,3d` uses `,` (not `.`) as a separator — `parse_sprintf_cuts` only
    // matches a literal `.` for precision, so the width run ends at `5`, the
    // `,` is not a length modifier, and `d` after `,` is not at the cut
    // position → the whole spec is rejected (returns None) and the field stays
    // one `string` token (no formatPercent emitted).
    let toks = decode("format {%5,3d} $x\n", "tcl8.6");
    assert!(
        of_type(&toks, "formatPercent").is_empty(),
        "malformed `%5,3d` should not decompose; got {toks:?}",
    );
    assert!(
        type_set("format {%5,3d} $x\n", "tcl8.6").contains("string"),
        "{toks:?}"
    );
}

#[test]
fn st_sprintf_percent_without_type_is_literal() {
    // A trailing bare `%` with no conversion type → `parse_sprintf_cuts`
    // returns None, the `%` is a literal, and no formatPercent is emitted.
    let toks = decode("format {abc%} $x\n", "tcl8.6");
    assert!(
        of_type(&toks, "formatPercent").is_empty(),
        "a type-less `%` is literal; got {toks:?}",
    );
}

// ===========================================================================
// clock field strings — `push_clock_subtokens`.
//
// `clock format`/`scan` are real builtins. tclsh-proof:
//   `string length [clock format 0 -format {%Y-%m-%d} -gmt 1]` == 10 (8.6/9.0).
// ===========================================================================

#[test]
fn st_clock_locale_modifier_then_spec() {
    // `%OS` — the `O` locale modifier precedes the `S` spec, so it is emitted
    // as `clockModifier` between the `%` (clockPercent) and `S` (clockSpec).
    let toks = decode("clock scan $s -format {%OS}\n", "tcl8.6");
    assert!(!of_type(&toks, "clockPercent").is_empty(), "{toks:?}");
    assert!(!of_type(&toks, "clockModifier").is_empty(), "{toks:?}");
    assert!(!of_type(&toks, "clockSpec").is_empty(), "{toks:?}");
}

#[test]
fn st_clock_modifier_letter_alone_is_not_a_conversion() {
    // `%E` with no conversion letter after it is a dangling modifier, so the
    // shared clock grammar leaves the whole field as a string.
    let toks = decode("clock format $t -format {%E}\n", "tcl8.6");
    assert!(of_type(&toks, "clockSpec").is_empty(), "{toks:?}");
    assert!(of_type(&toks, "clockPercent").is_empty(), "{toks:?}");
    assert!(
        of_type(&toks, "clockModifier").is_empty(),
        "a lone `%E` has no modifier; got {toks:?}",
    );
}

#[test]
fn st_clock_trailing_percent_not_a_spec_stays_string() {
    // `100%` — the `%` is not followed by a spec letter, so the loop just
    // advances (`i += 1`) and emits nothing; the field decodes as a `string`.
    let toks = decode("clock format $t -format {100%}\n", "tcl8.6");
    assert!(
        of_type(&toks, "clockPercent").is_empty(),
        "trailing bare `%` is not a clock spec; got {toks:?}",
    );
    assert!(type_set("clock format $t -format {100%}\n", "tcl8.6").contains("string"));
}

// ===========================================================================
// binary field strings — `push_binary_subtokens`.
//
// `binary format`/`scan` are real builtins. tclsh-proof:
//   `binary scan [binary format a3 hello] a3 r; set r` -> hel (8.6 + 9.0).
// ===========================================================================

#[test]
fn st_binary_whitespace_inside_field_string_is_skipped() {
    // `{  i2 }` — leading/trailing whitespace in the field string is skipped
    // (the `is_ascii_whitespace` arm); `i` is the spec, `2` the count.
    let toks = decode("binary format {  i2 } $d\n", "tcl8.6");
    assert!(!of_type(&toks, "binarySpec").is_empty(), "{toks:?}");
    assert!(!of_type(&toks, "binaryCount").is_empty(), "{toks:?}");
}

#[test]
fn st_binary_unrecognised_specifier_letter_is_skipped() {
    // `i2z` — `i` is a valid binary specifier and `2` a count, but `z` is NOT
    // in `BINARY_FORMAT_SPECIFIERS`, so it is skipped (the
    // `!BINARY_FORMAT_SPECIFIERS.contains` arm) and produces no token. The
    // valid `i` spec and `2` count still surface, proving the skip resumes the
    // scan rather than aborting.
    // tclsh-proof: `i` is a real binary spec. tclsh8.6/9.0:
    //   `binary scan [binary format i1 65] i1 r; set r` -> 65
    let toks = decode("binary format i2z $d\n", "tcl8.6");
    assert!(
        of_type(&toks, "binarySpec").iter().any(|t| t.length == 1),
        "`i` spec present; got {toks:?}",
    );
    assert!(
        of_type(&toks, "binaryCount").iter().any(|t| t.length == 1),
        "`2` count present; got {toks:?}",
    );
    // The trailing unrecognised `z` adds no further spec — exactly one spec.
    assert_eq!(
        of_type(&toks, "binarySpec").len(),
        1,
        "the unrecognised `z` must be skipped, not emitted; got {toks:?}",
    );
}

#[test]
fn st_binary_orphan_digits_are_not_a_count() {
    // Tcl rejects the space in `a2 99`; only the `2` attached to the valid
    // `a` field is a count. Orphan trailing digits are plain string content.
    let toks = decode("binary format {a2 99} $d\n", "tcl8.6");
    let counts = of_type(&toks, "binaryCount");
    assert_eq!(counts.len(), 1, "only `a2` has a count; got {toks:?}");
    assert_eq!(counts[0].length, 1, "the count is `2`; got {toks:?}");
}

#[test]
fn st_binary_signed_modifier_present_in_86_absent_in_84() {
    // `su` — under 8.6 the `u` after the integer specifier `s` is a
    // `binaryFlag` (8.5+ feature); under 8.4 the modifier is suppressed
    // (`allow_mod == false`), so no binaryFlag.
    let on_86 = type_set("binary scan $d su r\n", "tcl8.6");
    assert!(
        on_86.contains("binaryFlag"),
        "8.6 keeps the `u` modifier: {on_86:?}"
    );
    let on_84 = type_set("binary scan $d su r\n", "tcl8.4");
    assert!(
        !on_84.contains("binaryFlag"),
        "8.4 suppresses the `u` modifier: {on_84:?}"
    );
}

#[test]
fn st_binary_modifier_only_after_integer_specifier() {
    // `au` — `a` is NOT an integer specifier, so the `u` is not consumed as a
    // signed/unsigned modifier (the `BINARY_INT_SPECIFIERS.contains` guard).
    // `u` is itself not a binary specifier letter either, so it is skipped.
    let toks = decode("binary format au $d\n", "tcl8.6");
    assert!(
        of_type(&toks, "binaryFlag").is_empty(),
        "`u` after non-integer `a` is not a flag; got {toks:?}",
    );
}

// ===========================================================================
// regsub replacement spec — `push_regsub_subtokens`.
//
// tclsh-proof: regsub backrefs are meaningful. tclsh8.6/9.0:
//   `regsub {(b)} abc {[\1\&]} out; set out` -> a[b&]c
// ===========================================================================

#[test]
fn st_regsub_replacement_backref_and_whole_match() {
    // `{\1-\&}` — `\1` is a capture backref (`number`), `\&` the whole match
    // (`operator`), and the literal `-` between them a `string` run.
    let toks = decode("regsub {(a)} $s {\\1-\\&} out\n", "tcl8.6");
    assert!(
        of_type(&toks, "number").iter().any(|t| t.length == 2),
        "{toks:?}"
    );
    assert!(
        of_type(&toks, "operator").iter().any(|t| t.length == 2),
        "{toks:?}"
    );
    assert!(
        !of_type(&toks, "string").is_empty(),
        "the `-` run is a string; got {toks:?}"
    );
}

#[test]
fn st_regsub_replacement_without_backrefs_is_string() {
    // A replacement with no `\digit`/`\&` (`{plain text}`) → the
    // `push_regsub_subtokens` `!emitted` early return; the word stays a
    // `string` (no operator / number sub-tokens).
    let toks = decode("regsub {a} $s {plain text} out\n", "tcl8.6");
    assert!(of_type(&toks, "operator").is_empty(), "{toks:?}");
    assert!(of_type(&toks, "number").is_empty(), "{toks:?}");
    assert!(!of_type(&toks, "string").is_empty(), "{toks:?}");
}

// ===========================================================================
// Namespace-qualified command / proc names — `emit_command_head`,
// `classify_arg_token`.
// ===========================================================================

#[test]
fn st_namespaced_builtin_head_tail_is_function_with_default_library() {
    // `::set` — a fully-qualified reference to the builtin `set`. The head
    // splits into a `namespace` prefix (`::`) + a `function` tail (`set`), and
    // because the *full* name resolves to a registry builtin the tail carries
    // the `defaultLibrary` modifier (the `kind == Function && registry.get(..)`
    // arm of `emit_command_head`).
    // tclsh-proof: `::set` is the global `set`. tclsh8.6/9.0:
    //   `::set x 7; set x` -> 7
    let st = full("::set x 1\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg());
    let ns_idx = legend_idx("namespace");
    let fn_idx = legend_idx("function");
    // Token 0: `::` namespace prefix (len 2). Token 1: `set` function (len 3).
    assert_eq!(st.data[3], ns_idx, "prefix is namespace: {:?}", st.data);
    assert_eq!(st.data[2], 2, "prefix is `::` (len 2): {:?}", st.data);
    assert_eq!(st.data[8], fn_idx, "tail is function: {:?}", st.data);
    assert_eq!(
        st.data[9],
        1 << 3,
        "tail carries defaultLibrary: {:?}",
        st.data
    );
}

#[test]
fn st_namespaced_user_proc_name_definition_is_function() {
    // `proc ::ns::fn {} {}` — the proc-name argument is namespace-qualified.
    // `classify_arg_token`/`ProcNameDef` classifies the whole `::ns::fn` name
    // as a single `function` token (the proc-name override takes the start
    // offset; the name is not split here).
    let toks = decode("proc ::ns::fn {} {}\n", "tcl8.6");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "function" && t.length == 8 && t.character == 5),
        "the proc name `::ns::fn` (len 8) should be a function; got {toks:?}",
    );
}

#[test]
fn st_namespaced_bareword_argument_classified_as_namespace() {
    // A `::`-qualified bareword *argument* (not a head, not a proc name) →
    // `classify_arg_token`'s `text.contains("::")` arm → `namespace`.
    let toks = decode("puts ::foo::bar\n", "tcl8.6");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "namespace" && t.length == 10),
        "`::foo::bar` arg should be a length-10 namespace; got {toks:?}",
    );
}

// ===========================================================================
// expr sub-tokens — `collect_expr` arms (Command / String / Bool / function).
//
// `expr` math functions are real. tclsh-proof: `expr {abs(-5)}` -> 5 (8.6/9.0).
// ===========================================================================

#[test]
fn st_expr_nested_command_substitution_recurses() {
    // `[llength $a]` inside an `expr` — the `ExprTokenType::Command` arm strips
    // the `[`/`]` and recurses into the inner script, so `llength` surfaces as
    // a `function` head and `$a` as a `variable`.
    let kinds = type_set("set y [expr {[llength $a] + 1}]\n", "tcl8.6");
    assert!(
        kinds.contains("function"),
        "nested [llength] head; got {kinds:?}"
    );
    assert!(kinds.contains("variable"), "nested $a; got {kinds:?}");
    assert!(kinds.contains("operator"), "the `+`; got {kinds:?}");
    assert!(kinds.contains("number"), "the `1`; got {kinds:?}");
}

#[test]
fn st_expr_comment_is_a_comment_token() {
    // A TIP 582 `#` comment inside an `expr` body → the `ExprTokenType::Comment`
    // arm emits a `comment` sub-token, like a script-level comment.
    // tclsh-proof: tclsh9.0 `expr {1 + 2 # note}` -> 3 (expr-62.1's rule).
    let kinds = type_set("set y [expr {1 + 2 # note}]\n", "tcl9.0");
    assert!(
        kinds.contains("comment"),
        "the `# note` comment; got {kinds:?}"
    );
    // The operands either side are still classified — the comment does not
    // swallow the expression.
    assert!(kinds.contains("number"), "the `1`/`2`; got {kinds:?}");
    assert!(kinds.contains("operator"), "the `+`; got {kinds:?}");
    // Under a pre-9.0 dialect `#` is not a comment (TIP 582 is 9.0+), so no
    // comment token appears inside the expression.
    let kinds_86 = type_set("set y [expr {1 + 2 # note}]\n", "tcl8.6");
    assert!(
        !kinds_86.contains("comment"),
        "8.6 has no expr comments; got {kinds_86:?}"
    );
}

#[test]
fn st_expr_string_literal_is_string_token() {
    // A quoted operand inside an `expr` (`"hi"`) → the `ExprTokenType::String`
    // arm emits a `string` sub-token.
    // tclsh-proof: tclsh8.6/9.0 `expr {"hi" eq "hi"}` -> 1.
    let kinds = type_set("set y [expr {\"hi\" eq $a}]\n", "tcl8.6");
    assert!(
        kinds.contains("string"),
        "the `\"hi\"` literal; got {kinds:?}"
    );
    assert!(
        kinds.contains("operator"),
        "the `eq` operator; got {kinds:?}"
    );
}

#[test]
fn st_expr_math_function_carries_default_library() {
    // A recognised math function inside `expr` (`abs`) is a `function` with the
    // `defaultLibrary` modifier; a user-ish function name would not be. Assert
    // the modifier via the packed stream.
    let st = full("set y [expr {abs($a)}]\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg());
    let fn_idx = legend_idx("function");
    // Find the `abs` function token (length 3) and confirm its modifier bit.
    let mut found = false;
    for c in st.data.chunks(5) {
        if c[3] == fn_idx && c[2] == 3 {
            assert_eq!(
                c[4],
                1 << 3,
                "math fn `abs` carries defaultLibrary: {:?}",
                st.data
            );
            found = true;
        }
    }
    assert!(found, "expected an `abs` function token; got {:?}", st.data);
}

#[test]
fn st_expr_non_braced_argument_falls_back_to_classification() {
    // When the `expr` argument is NOT a braced literal (`expr $e`), `collect_expr`
    // takes the `subspec_content` None branch and classifies the token by its
    // lexer kind — here `$e` is a plain `variable`, not sub-tokenised.
    let toks = decode("expr $e\n", "tcl8.6");
    assert!(
        toks.iter().any(|t| t.ttype == "variable" && t.length == 2),
        "`$e` should classify as a variable; got {toks:?}",
    );
}

// ===========================================================================
// Bareword backslash escapes & structural keyword args — `push_escape_subtokens`,
// `push_keyword_arg`.
// ===========================================================================

#[test]
fn st_bareword_backslash_escape_splits_into_string_and_escape() {
    // `a\tb` as a *bareword* arg (not quoted) → `push_escape_subtokens` splits
    // it into a `string` run (`a`), an `escape` (`\t`), and a `string` run
    // (`b`).
    let toks = decode("puts a\\tb\n", "tcl8.6");
    assert!(
        of_type(&toks, "escape").iter().any(|t| t.length == 2),
        "{toks:?}"
    );
    assert_eq!(
        of_type(&toks, "string").len(),
        2,
        "two literal runs around the escape; got {toks:?}",
    );
}

#[test]
fn st_trailing_backslash_in_bareword_is_not_an_escape() {
    // A bareword ending in a lone `\` (`ab\`) — `push_escape_subtokens`
    // requires `i + 1 < len`, so a trailing backslash is not split out; the
    // word stays a plain string with no escape token.
    let toks = decode("set x ab\\\n", "tcl8.6");
    assert!(
        of_type(&toks, "escape").is_empty(),
        "a trailing lone backslash is not an escape; got {toks:?}",
    );
}

#[test]
fn st_quoted_structural_keyword_arg_trims_delimiters() {
    // A quoted `"on"` clause keyword of `try` — `push_keyword_arg` strips the
    // surrounding quotes and marks just `on` (length 2) as a `keyword`.
    let src = "try {\n foo\n} \"on\" error {e} {\n bar\n}\n";
    let toks = decode(src, "tcl8.6");
    let on_kw = toks
        .iter()
        .find(|t| t.ttype == "keyword" && t.line == 2)
        .expect("an `on` keyword token on the `}` line");
    assert_eq!(
        on_kw.length, 2,
        "quoted `\"on\"` marks just `on`; got {toks:?}"
    );
}

// ===========================================================================
// f5-irules object-reference overlay — `push_object_token`.
//
// The overlay is iRules-only; assert under the iRules registry/dialect.
// ===========================================================================

#[test]
fn st_irules_declaration_body_shape_gates_event_and_proc_definition_tokens() {
    let r = irules_registry();
    let src = concat!(
        "when HTTP_REQUEST bare_body\n",
        "when CLIENT_DATA \"quoted body\"\n",
        "when HTTP_RESPONSE {}\n",
        "proc bare_proc {} return\n",
        "proc quoted_proc {} \"return\"\n",
        "proc valid_proc {} { return }\n",
    );
    let toks = decode_with(src, "f5-irules", &r);
    for line in [0, 1] {
        assert!(
            !toks.iter().any(|t| t.line == line && t.ttype == "event"),
            "non-braced event body must not colour its selector as an event: {toks:?}"
        );
    }
    assert!(
        toks.iter().any(|t| t.line == 2 && t.ttype == "event"),
        "the braced event declaration must remain highlighted: {toks:?}"
    );
    for line in [3, 4] {
        assert!(
            !toks
                .iter()
                .any(|t| t.line == line && t.character == 5 && t.ttype == "function"),
            "non-braced proc body must not colour its name as a definition: {toks:?}"
        );
    }
    assert!(
        toks.iter()
            .any(|t| t.line == 5 && t.character == 5 && t.ttype == "function"),
        "the braced proc declaration must remain highlighted: {toks:?}"
    );
}

#[test]
fn st_irules_declaration_tokens_require_source_top_level() {
    // The registry owns valid declaration shape, but only commands at the
    // iRules file boundary are declarations. A `when` or `proc` nested in a
    // body / command substitution may look shape-valid to the registry while
    // being placement-invalid in an iRule.
    let r = irules_registry();
    let src = concat!(
        "when HTTP_REQUEST {}\n",
        "proc top_level {} {}\n",
        "when HTTP_REQUEST { when CLIENT_DATA {} }\n",
        "proc outer {} { when CLIENT_DATA {} }\n",
        "set ignored [when HTTP_RESPONSE {}]\n",
        "if {1} { proc inner {} {} }\n",
    );
    let toks = decode_with(src, "f5-irules", &r);

    assert!(
        toks.iter()
            .any(|t| t.line == 0 && t.character == 5 && t.ttype == "event"),
        "a valid top-level handler must remain an event: {toks:?}",
    );
    assert!(
        toks.iter()
            .any(|t| t.line == 1 && t.character == 5 && t.ttype == "function"),
        "a valid top-level proc name must remain a definition: {toks:?}",
    );

    for (line, character) in [(2, 25), (3, 21), (4, 18)] {
        let kind = toks
            .iter()
            .find(|t| t.line == line && t.character == character)
            .map(|t| t.ttype.as_str());
        assert_eq!(
            kind,
            Some("string"),
            "nested `when` at {line}:{character} must not receive an event override: {toks:?}",
        );
    }
    assert!(
        !toks
            .iter()
            .any(|t| t.line == 5 && t.character == 14 && t.ttype == "function"),
        "a nested iRules `proc` name must not receive a definition override: {toks:?}",
    );
}

#[test]
fn st_irules_custom_registry_proc_is_a_declaration() {
    // A workspace `.tclspec` pack can add an iRules procedure definer.  The
    // declaration-boundary scan must use that active registry, rather than
    // the process-wide built-in iRules registry, to admit the name override.
    let mut r = irules_registry();
    r.insert(CommandSpec {
        name: "custom_proc",
        traits: Traits::DEFINES_PROCEDURE | Traits::IRULES_TOP_LEVEL_ONLY,
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        ..CommandSpec::DEFAULT
    });
    let src = "custom_proc declared {} { return }\n";
    let toks = decode_with(src, "f5-irules", &r);
    assert!(
        toks.iter()
            .any(|t| t.line == 0 && t.character == 12 && t.ttype == "function"),
        "the custom registry definer's name must be a function definition: {toks:?}",
    );
}

#[test]
fn st_irules_event_fallback_survives_generic_tcl_registry() {
    // `when EVENT` has intentionally been event-coloured in generic Tcl
    // buffers for years.  The active generic registry has no iRules `when`
    // spec, so the declaration-boundary candidate set must retain the
    // built-in iRules fallback as well as workspace extensions.
    let src = "when HTTP_REQUEST {}\n";
    let toks = decode(src, "tcl8.6");
    assert_eq!(
        toks.iter()
            .find(|t| t.line == 0 && t.character == 5)
            .map(|t| t.ttype.as_str()),
        Some("event"),
        "generic Tcl must retain dialect-independent event colouring: {toks:?}",
    );
}

#[test]
fn st_irules_declaration_boundary_tracks_resolved_head_identity() {
    // The shared top-level boundary owner resolves command identity at the
    // source offset: redefining `when` means the later spelling is no longer
    // an iRules event handler, even though its words retain declaration shape.
    let r = irules_registry();
    let src = "proc when {args} {}\nwhen HTTP_REQUEST {}\n";
    let toks = decode_with(src, "f5-irules", &r);
    assert!(
        toks.iter()
            .any(|t| t.line == 0 && t.character == 5 && t.ttype == "function"),
        "the redefining proc name remains a definition: {toks:?}",
    );
    assert_eq!(
        toks.iter()
            .find(|t| t.line == 1 && t.character == 5)
            .map(|t| t.ttype.as_str()),
        Some("string"),
        "a rebound `when` must not receive an event override: {toks:?}",
    );
}

#[test]
fn st_irules_object_ref_in_multiline_body_is_object_token() {
    // `pool /Common/web_pool` inside a multi-line `when` body — the body is
    // tokenised by recursion; the object overlay marks the partitioned pool
    // name as one `object` token.
    let r = irules_registry();
    let src = "when HTTP_REQUEST {\n  pool /Common/web_pool\n}\n";
    let toks = decode_with(src, "f5-irules", &r);
    assert!(
        toks.iter().any(|t| t.ttype == "object" && t.line == 1),
        "the pool name should be an object token; got {toks:?}",
    );
    // The event name is still an `event` token.
    assert!(toks.iter().any(|t| t.ttype == "event"), "{toks:?}");
}

#[test]
fn st_irules_data_group_in_braced_expr_command_is_object_token() {
    // The `class` call is embedded in an expression braced for Tcl's usual
    // substitution discipline. The iRules execution inventory finds its
    // expression-language `[…]` command, and the object overlay preserves the
    // data-group name as an `object` token.
    let r = irules_registry();
    let src = "when HTTP_REQUEST {\n  if {[class match [HTTP::host] equals /Common/host_dg]} { set hit 1 }\n}\n";
    let toks = decode_with(src, "f5-irules", &r);
    assert!(
        toks.iter()
            .any(|t| t.ttype == "object" && t.line == 1 && t.length == 15),
        "the braced-expression data-group must be an object token; got {toks:?}",
    );
}

#[test]
fn st_irules_object_ref_suppresses_overlapping_string_in_single_line_body() {
    // In a single-line body (`when HTTP_REQUEST { pool web_pool }`) the bare
    // `web_pool` would classify as a `string`; the object overlay drops that
    // overlapping `string` entry and emits an `object` token in its place
    // (the `*kind == TokenKind::String` retain branch of `push_object_token`).
    let r = irules_registry();
    let src = "when HTTP_REQUEST { pool web_pool }\n";
    let toks = decode_with(src, "f5-irules", &r);
    let obj = toks
        .iter()
        .find(|t| t.ttype == "object")
        .expect("an object token for `web_pool`");
    assert_eq!(obj.length, 8, "`web_pool` object span; got {toks:?}");
    // No `string` token should remain at the object's column (it was replaced).
    assert!(
        !toks
            .iter()
            .any(|t| t.ttype == "string" && t.character == obj.character),
        "the overlapping string must be suppressed; got {toks:?}",
    );
}

#[test]
fn st_irules_object_overlay_absent_in_plain_tcl() {
    // The object overlay is gated by the canonical iRules profile; under
    // plain `tcl8.6` the same source yields no `object` token.
    let src = "when HTTP_REQUEST {\n  pool web_pool\n}\n";
    assert!(
        !type_set(src, "tcl8.6").contains("object"),
        "object overlay must be iRules-only",
    );
}

// ===========================================================================
// Decorator option switches & subcommand keywords on real specs.
// ===========================================================================

#[test]
fn st_known_option_is_decorator_unknown_stays_string() {
    // `regexp -nocase {p} $s` — `-nocase` is a real `regexp` option → a
    // `decorator`. `puts -foo` — `-foo` is not a `puts` option → it stays a
    // `string` (the `spec.options.iter().any` guard).
    // tclsh-proof: `-nocase` is a documented regexp switch. tclsh8.6/9.0:
    //   `regexp -nocase {ABC} abc` -> 1
    assert!(type_set("regexp -nocase {p} $s\n", "tcl8.6").contains("decorator"));
    let puts = type_set("puts -foo\n", "tcl8.6");
    assert!(
        !puts.contains("decorator"),
        "`-foo` is not a puts option: {puts:?}"
    );
    assert!(puts.contains("string"), "`-foo` stays a string: {puts:?}");
}

#[test]
fn st_subcommand_word_is_keyword_with_default_library() {
    // `string length $s` — `length` is a recognised `string` subcommand →
    // `keyword` + `defaultLibrary` (the `spec.subcommand(..)` arm).
    // tclsh-proof: `string length` is a real subcommand. tclsh8.6/9.0:
    //   `string length hello` -> 5
    let toks = decode("string length $s\n", "tcl8.6");
    let len_tok = toks
        .iter()
        .find(|t| t.character == 7 && t.length == 6)
        .expect("the `length` subcommand token");
    assert_eq!(len_tok.ttype, "keyword", "{toks:?}");
    // Confirm the defaultLibrary modifier on that (2nd) token in the packed stream.
    let st = full("string length $s\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg());
    assert_eq!(
        st.data[9],
        1 << 3,
        "subcommand keyword carries defaultLibrary: {:?}",
        st.data
    );
}

// ===========================================================================
// Range variant & multi-line dropping.
// ===========================================================================

#[test]
fn st_range_variant_drops_tokens_outside_window_and_rebases_delta() {
    // The range variant keeps only tokens whose start falls in `[start, end)`
    // and re-bases the delta encoding on the first survivor.
    let src = "set a 1\nset b 2\nset c 3\n";
    let r = range(
        src,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 2,
            end_character: 0,
        },
        reg(),
    );
    assert_eq!(r.data.len() % 5, 0, "{:?}", r.data);
    assert!(!r.data.is_empty(), "{:?}", r.data);
    // First surviving token is on line 1 → its leading delta-line is 1 (absolute
    // from origin per the LSP range contract).
    assert_eq!(
        r.data[0], 1,
        "first range token delta-line is absolute: {:?}",
        r.data
    );
    // Strictly fewer tokens than the whole document.
    assert!(
        r.data.len() < full(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), reg()).data.len(),
        "{:?}",
        r.data
    );
}

#[test]
fn st_range_empty_window_yields_no_tokens() {
    // A zero-width range (start == end) keeps nothing (the half-open `pos <
    // end` test can never hold), so the stream is empty.
    let src = "set a 1\nset b 2\n";
    let r = range(
        src,
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 0,
        },
        reg(),
    );
    assert!(
        r.data.is_empty(),
        "empty window yields no tokens; got {:?}",
        r.data
    );
}

#[test]
fn st_multiline_braced_body_is_recursed_not_emitted_whole() {
    // A multi-line braced body's enclosing `string` token spans a newline and
    // is dropped; the recursion emits per-line inner tokens instead. Every
    // emitted token therefore stays on a single line.
    let src = "proc f {} {\n  set x 1\n  set y 2\n}\n";
    let toks = decode(src, "tcl8.6");
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "{toks:?}"
    );
    assert!(
        toks.iter().any(|t| t.line == 2 && t.ttype == "function"),
        "{toks:?}"
    );
    // The body braces never surface as a single multi-line string token.
    assert!(
        !toks
            .iter()
            .any(|t| t.line == 0 && t.ttype == "string" && t.length > 5),
        "no opaque multi-line body string; got {toks:?}",
    );
}

// ===========================================================================
// regexp option-skip to the pattern word — the `-start N` / `--` arms of the
// `pattern_type == Regex` positional-skip loop in `special_arg_kinds`.
// ===========================================================================

#[test]
fn st_regexp_start_option_value_skipped_before_pattern() {
    // `regexp -start 1 {b+} $s` — `-start` takes a value, so the skip loop
    // advances by TWO (`idx += 2`) past both `-start` and its `1` argument; the
    // pattern override then lands on `{b+}`, which sub-tokenises as a regex
    // (the `b` literal + `+` quantifier) rather than a plain string.
    // tclsh-proof: `-start` is a real regexp option. tclsh8.6/9.0:
    //   `regexp -start 1 {b} abcb` -> 1
    let toks = decode("regexp -start 1 {b+} $s\n", "tcl8.6");
    // `-start` is a decorator, `1` its numeric value, and the pattern is a
    // regex (quantifier present) — proving the skip targeted the right word.
    assert!(
        of_type(&toks, "decorator").iter().any(|t| t.length == 6),
        "{toks:?}"
    );
    assert!(
        toks.iter().any(|t| t.ttype == "regexpQuantifier"),
        "the `{{b+}}` pattern after `-start 1` must sub-tokenise as regex; got {toks:?}",
    );
}

#[test]
fn st_regexp_double_dash_ends_options_before_pattern() {
    // `regexp -- {ab} $s` — `--` terminates option parsing; the skip loop steps
    // past it (`idx += 1` at the `args[idx] == "--"` arm) and the override lands
    // on `{ab}`, classified as a `regexp` pattern.
    // tclsh-proof: `--` is the regexp end-of-options marker. tclsh8.6/9.0:
    //   `regexp -- {-x} -x-` -> 1
    let toks = decode("regexp -- {ab} $s\n", "tcl8.6");
    assert!(
        of_type(&toks, "decorator").iter().any(|t| t.length == 2),
        "`--` decorator; got {toks:?}"
    );
    // The `{ab}` pattern (no metacharacters) falls back to a single `regexp`
    // token — proving the `--` skip landed the override on the pattern word
    // rather than leaving it a plain `string`.
    assert!(
        !of_type(&toks, "regexp").is_empty(),
        "the `{{ab}}` pattern after `--` must be a regexp; got {toks:?}",
    );
    assert!(
        of_type(&toks, "string").is_empty(),
        "the pattern word must not be a plain string; got {toks:?}",
    );
}

// ===========================================================================
// binary subcommand that is neither `format` nor `scan` — the `_ => None`
// arm of the `binary` field-word selection (no BinaryFormat override).
// ===========================================================================

#[test]
fn st_binary_non_format_subcommand_has_no_field_override() {
    // `binary encode base64 $d` — the `encode` subcommand is not `format`/`scan`,
    // so the `bin_word` match returns None and no `BinaryFormat` override is
    // installed for it at all (no binarySpec/binaryCount tokens).
    // tclsh-proof: `binary encode base64` is a real subcommand. tclsh8.6/9.0:
    //   `binary encode base64 hello` -> aGVsbG8=
    let toks = decode("binary encode base64 $d\n", "tcl8.6");
    assert!(of_type(&toks, "binarySpec").is_empty(), "{toks:?}");
    assert!(of_type(&toks, "binaryCount").is_empty(), "{toks:?}");
    // `encode` is a recognised subcommand keyword; `base64` is the format-name
    // argument, closed over exactly base64/hex/uuencode
    // (`ENCODE_DECODE_FORMAT_VALUES` in `binary_.rs`) and so classified
    // `enumMember` — a separate, legitimate override from the `BinaryFormat`
    // one this test is otherwise about, not a plain string.
    assert!(
        toks.iter()
            .any(|t| t.ttype == "enumMember" && t.length == 6),
        "`base64` is the closed-set format-name arg, not a plain string; got {toks:?}",
    );
}
