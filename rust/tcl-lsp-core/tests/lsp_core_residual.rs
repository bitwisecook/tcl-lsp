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

//! Residual-coverage port tests for four `tcl-lsp-core` files that the
//! existing `*_port.rs` suites leave in the low-80s%:
//!
//! * `src/formatting/engine.rs` — indentation, continuation, comment, switch,
//!   and config edges.
//! * `src/refactor/inline_variable.rs` — single/multi-use inline,
//!   `$`-preservation, and decline paths.
//! * `src/semantic_tokens.rs` — the less-common token kinds and modifiers,
//!   delta encoding, and the range variant.
//! * `src/minify.rs` — continuation, brace/quote preservation, expr, and
//!   decline edges.
//!
//! This file targets branches the sibling ports (`semantic_tokens.rs`,
//! `lsp_edit_workspace.rs`, `code_actions_depth.rs`, …) do not
//! already exercise — it does not duplicate them.
//!
//! ## Proof discipline
//!
//! * **Minify / format are semantics-preserving transforms.** Where a script
//!   is minified or formatted, the BEFORE and AFTER forms were run through
//!   `scripts/dev/tclsh_check.sh` against real C-Tcl (tclsh **8.6** and
//!   **9.0**) and compute the same value (or are both `info complete`). Each
//!   such test cites the `// tclsh:` ground truth.
//! * **Inline-variable preserves the program's value** — the inlined program
//!   was run in tclsh 8.6 + 9.0 and prints the same thing; cited per test.
//! * **Token kinds / formatting whitespace are editor presentation.** tclsh
//!   has no opinion on which span is highlighted `keyword` vs `string`, so
//!   those are asserted **structurally** against the encoder's contract (the
//!   `legend_token_types` order), not against C-Tcl.

use tcl_compiler::analyser::Analyser;
use tcl_lexer::LineIndex;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::formatting::{FormatterConfig, format_tcl};
use tcl_lsp_core::minify::minify_tcl;
use tcl_lsp_core::refactor::inline_variable;
use tcl_lsp_core::semantic_tokens::{diff, full, legend_token_types, range};
use tcl_registry::{CommandRegistry, registry_for_dialect};

fn reg() -> &'static CommandRegistry {
    registry_for_dialect("tcl8.6")
}

// ===========================================================================
// formatting/engine.rs — layout edges (whitespace only; tclsh-cited where the
// reflow could plausibly change meaning).
// ===========================================================================

fn fmt(src: &str) -> String {
    format_tcl(src, &FormatterConfig::default(), reg())
}

fn fmt_with(src: &str, cfg: &FormatterConfig) -> String {
    format_tcl(src, cfg, reg())
}

#[test]
fn fmt_collapses_backslash_continuation_in_command_subst() {
    // A `\<newline>` continuation inside `[…]` is collapsed to a single
    // space — semantics-preserving (the lexer joins the continuation anyway).
    // tclsh (8.6 + 9.0): both
    //   `set x [string cat a \<nl>    b]; puts $x`   and
    //   `set x [string cat a b]; puts $x`            print `ab`.
    assert_eq!(
        fmt("set x [string cat a \\\n    b]\n"),
        "set x [string cat a b]\n",
    );
}

#[test]
fn fmt_for_special_case_inlines_init_and_next_expands_body() {
    // The formatter's `for` override: init (`set i 0`) and next (`incr i`)
    // stay inline; only the 4th arg (the loop body) expands K&R.
    // tclsh (8.6 + 9.0): the body is unchanged code, so
    //   `for {set i 0} {$i < 3} {incr i} { append out $i }` accumulates `012`
    // before and after; the formatted block is `info complete` → 1.
    assert_eq!(
        fmt("for {set i 0} {$i < 3} {incr i} {\nappend out $i\n}\n"),
        "for {set i 0} {$i < 3} {incr i} {\n    append out $i\n}\n",
    );
}

#[test]
fn fmt_switch_empty_and_dash_fallthrough_bodies() {
    // switch body formatting: an empty `{}` body stays inline `{}`; a `-`
    // fall-through keeps the bare dash; a real braced body expands.
    // tclsh (8.6 + 9.0): pure layout — the whole switch is `info complete`
    // before and after, and `switch a {a {} b {puts 2}}` / the dash form run
    // identically.
    assert_eq!(
        fmt("switch $x {\na {}\nb {\nputs 2\n}\n}\n"),
        "switch $x {\n    a {}\n    b {\n        puts 2\n    }\n}\n",
    );
    assert_eq!(
        fmt("switch $x {\na -\nb {\nputs 2\n}\n}\n"),
        "switch $x {\n    a -\n    b {\n        puts 2\n    }\n}\n",
    );
}

#[test]
fn fmt_try_on_clause_keyword_is_structural() {
    // `on` is a `try` clause keyword (not a command), kept on the `}` line.
    // tclsh (8.6 + 9.0): layout only; the try block is `info complete` in
    // both forms.
    assert_eq!(
        fmt("try {\nfoo\n} on error {e} {\nbar\n}\n"),
        "try {\n    foo\n} on error {e} {\n    bar\n}\n",
    );
}

#[test]
fn fmt_param_list_with_nested_default_braces_normalised() {
    // A parameter list with a `{b 1}` default is normalised (single internal
    // spaces) without losing the nested braces — `normalise_param_list`'s
    // brace-nesting branch. tclsh (8.6 + 9.0): identical proc signature; the
    // proc definition is `info complete` either way.
    assert_eq!(
        fmt("proc f {a    {b 1}   c} {\nreturn\n}\n"),
        "proc f {a {b 1} c} {\n    return\n}\n",
    );
}

#[test]
fn fmt_tab_indented_body_reindented_to_spaces() {
    // A tab-indented body is re-indented to the configured space width.
    // tclsh (8.6 + 9.0): pure indentation — `proc f {} {set x 1}` runs the
    // same and the body is `info complete`.
    assert_eq!(
        fmt("proc f {} {\n\tset x 1\n}\n"),
        "proc f {} {\n    set x 1\n}\n",
    );
}

#[test]
fn fmt_comment_hash_spacing_honours_config() {
    // `space_after_comment_hash = false` trims *all* surrounding whitespace
    // after `#`; the default (`true`) keeps the leading run and trims only the
    // trailing whitespace. Comments are inert text — no tclsh proof needed (a
    // `#…` line is a no-op command).
    let no_space = FormatterConfig {
        space_after_comment_hash: false,
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("#   hi\n", &no_space), "#hi\n");
    // Default: leading whitespace preserved, trailing trimmed.
    assert_eq!(fmt("#   hi\n"), "#   hi\n");
    assert_eq!(fmt("# hi   \n"), "# hi\n");
}

#[test]
fn fmt_commented_out_code_collapses_continuation() {
    // A commented-out command (no space after `#`) is treated as code: its
    // `\<newline>` continuation collapses. Still a comment line at runtime, so
    // no behavioural change — structural assertion only.
    assert_eq!(fmt("#set x [foo \\\n   bar]\n"), "#set x [foo bar]\n");
}

#[test]
fn fmt_crlf_line_ending_config_applied() {
    // `line_ending = "\r\n"` rewrites every newline. Layout only.
    let crlf = FormatterConfig {
        line_ending: "\r\n".to_owned(),
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("puts hi\n", &crlf), "puts hi\r\n");
}

#[test]
fn fmt_expand_single_line_bodies_config() {
    // `expand_single_line_bodies = true` forces an inline body onto its own
    // line. tclsh (8.6 + 9.0): `proc f {} { set x 1 }` and the expanded form
    // are the same command — both `info complete`, both define `f` identically.
    let expand = FormatterConfig {
        expand_single_line_bodies: true,
        ..FormatterConfig::default()
    };
    assert_eq!(
        fmt_with("proc f {} { set x 1 }\n", &expand),
        "proc f {} {\n    set x 1\n}\n",
    );
}

#[test]
fn fmt_enforce_braced_variables_rewrites_refs() {
    // `enforce_braced_variables` rewrites `$x` → `${x}` everywhere.
    // tclsh (8.6 + 9.0): `set x hi; puts $x` and `set x hi; puts ${x}` both
    // print `hi` — `$x` and `${x}` are the same reference.
    let braced = FormatterConfig {
        enforce_braced_variables: true,
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("puts $x\n", &braced), "puts ${x}\n");
}

#[test]
fn fmt_force_expr_wrap_at_logical_operators() {
    // A braced `if` expression past `max_line_length` wraps at top-level
    // `&&`/`||` (the `wrap_braced_expr` path). tclsh (8.6 + 9.0): the wrapped
    // multi-line condition is `info complete` and evaluates identically to the
    // single-line form (logical structure unchanged).
    // 7 operands → the first line is 143 chars (> the 120 default), forcing
    // the wrap.
    let out = fmt(
        "if {$aaaaaaaaaa == 1 && $bbbbbbbbbb == 2 && $cccccccccc == 3 && $dddddddddd == 4 && $eeeeeeeeee == 5 && $ffffffffff == 6 && $gggggggggg == 7} {\nputs hi\n}\n",
    );
    assert!(out.contains("\n    && $bbbbbbbbbb == 2"), "{out:?}");
    assert!(out.contains("} {\n    puts hi\n}\n"), "{out:?}");
    // The operands are preserved verbatim (no token dropped / reordered).
    for operand in ["$aaaaaaaaaa == 1", "$gggggggggg == 7"] {
        assert!(out.contains(operand), "missing {operand:?} in {out:?}");
    }
}

#[test]
fn fmt_syntax_error_input_is_not_dropped() {
    // The engine is best-effort: even an unbalanced-brace input round-trips
    // without losing the source words (the lexer still tokenises it). No tclsh
    // proof — the input is deliberately `info complete` → 0.
    let out = fmt("proc f {{ unbalanced\n");
    assert!(out.contains("proc f"), "{out:?}");
    assert!(out.contains("unbalanced"), "{out:?}");
}

#[test]
fn fmt_is_idempotent_on_its_own_output() {
    // Formatting a formatted document is a fixpoint — the idempotence path.
    let src = "proc f {} {\nif {1} {\nset x 1\n}\n}\n";
    let once = fmt(src);
    let twice = fmt(&once);
    assert_eq!(once, twice, "format must be idempotent");
}

#[test]
fn fmt_already_formatted_is_unchanged() {
    // A canonically-formatted document is returned byte-for-byte (the
    // already-formatted no-op path).
    let canonical = "proc f {} {\n    set x 1\n}\n";
    assert_eq!(fmt(canonical), canonical);
}

// ===========================================================================
// refactor/inline_variable.rs — inline a `set x V` into its use(s).
//
// Each positive case pins the applied program text AND cites the tclsh proof
// that BEFORE and AFTER print the same thing. Decline cases return `None`.
// ===========================================================================

fn inline(src: &str, cursor: u32) -> Option<String> {
    let analysis = Analyser::new().analyse(src, "tcl8.6").clone();
    let li = LineIndex::new(src);
    inline_variable(src, cursor, &analysis, reg(), &li).map(|r| r.apply(src))
}

#[test]
fn inline_quoted_value_into_standalone_word_keeps_quotes() {
    // A `"…"` value is kept verbatim (quotes preserved) when the reference is
    // its own word. tclsh (8.6 + 9.0):
    //   `set name "hello world"; puts $name`  →  `hello world`
    //   `puts "hello world"`                  →  `hello world`
    assert_eq!(
        inline("set name \"hello world\"\nputs $name", 0).as_deref(),
        Some("puts \"hello world\""),
    );
}

#[test]
fn inline_into_following_set_rhs() {
    // Inlining into a second command's standalone-word RHS deletes the `set`
    // and substitutes the value. tclsh (8.6 + 9.0):
    //   `set x 99; set y $x; puts $y`  →  `99`
    //   `set y 99; puts $y`            →  `99`
    assert_eq!(inline("set x 99\nset y $x", 0).as_deref(), Some("set y 99"));
}

#[test]
fn inline_braced_value_standalone_preserves_braces() {
    // A `{a b}` value is kept braced when the reference is a standalone word,
    // so it stays one list value. tclsh (8.6 + 9.0):
    //   `set x {a b}; set y $x; puts $y`  →  `a b`
    //   `set y {a b}; puts $y`            →  `a b`
    assert_eq!(
        inline("set x {a b}\nset y $x", 0).as_deref(),
        Some("set y {a b}"),
    );
}

#[test]
fn inline_command_substitution_value_standalone() {
    // A `[list 1 2]` value (command substitution) is spliced verbatim into a
    // standalone-word reference. tclsh (8.6 + 9.0):
    //   `set x [list 1 2]; set y $x; puts $y`  →  `1 2`
    //   `set y [list 1 2]; puts $y`            →  `1 2`
    assert_eq!(
        inline("set x [list 1 2]\nset y $x", 0).as_deref(),
        Some("set y [list 1 2]"),
    );
}

#[test]
fn inline_interpolated_inside_quoted_string_dequotes_value() {
    // When the reference is interpolated inside a larger `"…"` word, a
    // `{a b}` value is *dequoted* to `a b` and spliced — keeping the braces
    // would corrupt the string. tclsh (8.6 + 9.0):
    //   `set g {a b}; puts "x $g y"`  →  `x a b y`
    //   `puts "x a b y"`              →  `x a b y`
    assert_eq!(
        inline("set g {a b}\nputs \"x $g y\"", 0).as_deref(),
        Some("puts \"x a b y\""),
    );
}

#[test]
fn inline_multiline_continuation_value_preserved_verbatim() {
    // A value spanning a `\<newline>` continuation is kept verbatim as a
    // standalone word. tclsh (8.6 + 9.0): both
    //   `set x "a\<nl>b"; puts $x`  and  `puts "a\<nl>b"`  print `a b`.
    assert_eq!(
        inline("set x \"a\\\nb\"\nputs $x", 0).as_deref(),
        Some("puts \"a\\\nb\""),
    );
}

#[test]
fn inline_declines_multiple_uses() {
    // Two reads → ambiguous which to inline → declined (kept). (Mirrors the
    // in-module guard but via the public entry with a distinct fixture.)
    assert!(inline("set x 42\nputs $x\nputs $x", 0).is_none());
}

#[test]
fn inline_declines_when_reassigned_later() {
    // The variable is read once but reassigned afterwards; inlining the first
    // value would be unsafe, so the transform declines.
    assert!(inline("set x 1\nputs $x\nset x 2", 0).is_none());
}

#[test]
fn inline_declines_used_before_definition() {
    // The only reference precedes the `set`, so there is no
    // definition-line-matched single use to inline → declined.
    assert!(inline("puts $x\nset x 1", 9).is_none());
}

#[test]
fn inline_declines_bare_concat_with_whitespace_value() {
    // Interpolated into a *bare* (unquoted) word, a value containing
    // whitespace cannot be spliced without changing word count, so it
    // declines. tclsh (8.6 + 9.0) shows why: `set g "a b"; puts hi$g` prints
    // `hia b` (one arg); a bare `hi a b` splice would be three args — wrong.
    assert!(inline("set g \"a b\"\nputs hi$g", 0).is_none());
}

#[test]
fn inline_bare_concat_without_whitespace_value_succeeds() {
    // The complementary positive: a whitespace-free value *can* splice into a
    // bare concatenation. tclsh (8.6 + 9.0):
    //   `set g xyz; puts hi$g`  →  `hixyz`
    //   `puts hixyz`            →  `hixyz`
    assert_eq!(
        inline("set g xyz\nputs hi$g", 0).as_deref(),
        Some("puts hixyz")
    );
}

#[test]
fn inline_declines_read_form_and_non_set() {
    // `set x` (read form, no value) and a non-`set` command have nothing to
    // inline.
    assert!(inline("set x", 0).is_none());
    assert!(inline("puts \"hello\"", 0).is_none());
}

// ===========================================================================
// semantic_tokens.rs — the less-common token kinds + modifiers, delta
// encoding, and the range variant.
//
// Token-kind classification is editor presentation (tclsh has no opinion), so
// these decode the LSP stream and assert structurally against the
// `legend_token_types` order.
// ===========================================================================

/// One decoded token: absolute `(line, char, length, type-name)`.
#[derive(Debug, Clone)]
struct Tok {
    line: u32,
    character: u32,
    length: u32,
    ttype: String,
}

/// Decode the packed LSP delta stream into absolute tokens (the inverse of
/// the provider's relative encoding).
fn decode(source: &str, dialect: &str) -> Vec<Tok> {
    let registry = registry_for_dialect(dialect);
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

fn type_set(source: &str, dialect: &str) -> std::collections::HashSet<String> {
    decode(source, dialect)
        .into_iter()
        .map(|t| t.ttype)
        .collect()
}

#[test]
fn st_namespace_qualified_head_splits_into_namespace_plus_command() {
    // A `oo::class` head splits into a leading `namespace` token (`oo::`) and
    // a final-segment command token. The split is byte-accurate.
    let toks = decode("oo::class create C\n", "tcl8.6");
    let ns = toks
        .iter()
        .find(|t| t.ttype == "namespace")
        .expect("a namespace prefix token");
    assert_eq!((ns.line, ns.character, ns.length), (0, 0, 4), "{toks:?}");
}

#[test]
fn st_expr_emits_operator_number_and_variable_subtokens() {
    // A braced `expr` body is sub-tokenised: `$a` → variable, `+` → operator,
    // `1` → number, the math function `abs` → function (defaultLibrary).
    let kinds = type_set("set y [expr {abs($a) + 1}]\n", "tcl8.6");
    for k in ["variable", "operator", "number", "function"] {
        assert!(
            kinds.contains(k),
            "expected {k:?} in expr subtokens; got {kinds:?}"
        );
    }
}

#[test]
fn st_expr_boolean_literal_is_keyword() {
    // A boolean literal inside an `expr` (`true`) is classified `keyword`
    // (the `ExprTokenType::Bool` arm).
    let kinds = type_set("set y [expr {$a && true}]\n", "tcl8.6");
    assert!(kinds.contains("keyword"), "{kinds:?}");
}

#[test]
fn st_string_backslash_escape_subtokens() {
    // A bareword/quoted string with a `\t` escape splits into `string` literal
    // runs plus an `escape` sub-token (the `push_escape_subtokens` path).
    let kinds = type_set("puts \"a\\tb\"\n", "tcl8.6");
    assert!(
        kinds.contains("escape"),
        "expected an escape token; got {kinds:?}"
    );
    assert!(kinds.contains("string"), "{kinds:?}");
}

#[test]
fn st_namespaced_bareword_argument_is_namespace() {
    // A `::`-qualified *bareword argument* (not a command head) classifies as
    // `namespace` (the `classify_arg_token` Esc `::` branch).
    let toks = decode("set x ::foo::bar\n", "tcl8.6");
    assert!(
        toks.iter()
            .any(|t| t.ttype == "namespace" && t.length == 10),
        "expected a length-10 namespace arg token; got {toks:?}",
    );
}

#[test]
fn st_proc_name_carries_definition_modifier() {
    // The name argument of `proc` is `function` + the `definition` modifier
    // (legend modifier index 1 → bit 2). Assert via the raw packed stream.
    let st = full("proc myproc {} {}\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg());
    // Tokens: [proc(keyword), myproc(function+definition), {} , {}].
    // The second token (offset 5..10) is the proc name.
    let name = &st.data[5..10];
    let function_idx = u32::try_from(
        legend_token_types()
            .iter()
            .position(|t| *t == "function")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        name[3], function_idx,
        "proc name is a function: {:?}",
        st.data
    );
    assert_eq!(
        name[4],
        1 << 1,
        "proc name carries `definition`: {:?}",
        st.data
    );
}

#[test]
fn st_subcommand_word_is_keyword_with_default_library() {
    // A recognised subcommand word at arg index 1 (`string length`) is
    // `keyword` + `defaultLibrary` (bit 3). Find the `length` token.
    let toks_packed = full("string length $s\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg());
    // Decode positions to find the subcommand word (col 7, len 6).
    let toks = decode("string length $s\n", "tcl8.6");
    let sub = toks
        .iter()
        .find(|t| t.character == 7)
        .expect("the `length` subcommand token");
    assert_eq!(sub.ttype, "keyword", "{toks:?}");
    // Confirm the defaultLibrary modifier on that token in the packed stream.
    // The subcommand is the 2nd token → modifiers at index 5*1+4 = 9.
    assert_eq!(
        toks_packed.data[9],
        1 << 3,
        "subcommand keyword carries defaultLibrary: {:?}",
        toks_packed.data,
    );
}

#[test]
fn st_multiline_tokens_are_dropped_from_stream() {
    // A multi-line braced body's enclosing `string` token is not emitted whole
    // (LSP wants per-line tokens); the recursion emits its inner tokens
    // instead. Assert no emitted token spans a newline by reconstructing each
    // token's covered text length never crossing a line — structurally, every
    // entry stays on one line (the decoder already groups per line).
    let src = "proc f {} {\n  set x 1\n  set y 2\n}\n";
    let toks = decode(src, "tcl8.6");
    // The inner `set`/`x`/`1` and `set`/`y`/`2` tokens must appear on their own
    // lines (1 and 2), proving the body recursion ran rather than emitting one
    // opaque multi-line string.
    assert!(
        toks.iter().any(|t| t.line == 1 && t.ttype == "function"),
        "{toks:?}"
    );
    assert!(
        toks.iter().any(|t| t.line == 2 && t.ttype == "function"),
        "{toks:?}"
    );
}

#[test]
fn st_range_variant_restarts_delta_from_first_surviving_token() {
    // The range variant drops tokens outside the window and re-bases the delta
    // encoding on the first surviving token (its delta-line is absolute).
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
    assert!(!r.data.is_empty(), "{:?}", r.data);
    assert_eq!(r.data.len() % 5, 0);
    // First surviving token sits on line 1 → its leading delta-line is 1
    // (absolute from origin, per the LSP range contract).
    assert_eq!(
        r.data[0], 1,
        "first range token delta-line is absolute: {:?}",
        r.data
    );
    // Strictly fewer tokens than the full document.
    assert!(r.data.len() < full(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), reg()).data.len());
}

#[test]
fn st_diff_appends_a_single_token_via_provider() {
    // Drive `diff` with two real provider streams (not synthetic vectors):
    // appending one trailing argument (`set x 1` → `set x 1 2`) appends exactly
    // one token after the common prefix, deleting nothing. (The in-module tests
    // use hand-built vectors; this exercises the encoder + diff together.)
    // Note: a same-length value edit (`1`→`2`) leaves the stream byte-identical
    // — the encoding carries position/length/type, not the literal — so `diff`
    // returns `None`; we change the token *count* instead.
    let before = full("set x 1\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg()).data;
    let after = full("set x 1 2\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg()).data;
    let edit = diff(&before, &after).expect("a single append edit");
    // Token-aligned: every field is a multiple of 5.
    assert_eq!(edit.start % 5, 0, "{edit:?}");
    assert_eq!(edit.delete_count, 0, "append deletes nothing: {edit:?}");
    assert_eq!(edit.data.len(), 5, "exactly one token appended: {edit:?}");
    // The common prefix is the three unchanged tokens (`set`, `x`, `1`).
    assert_eq!(edit.start, 15, "{edit:?}");
    // A same-length literal edit produces an identical stream → no diff.
    let same_len = full("set x 2\n", tcl_dialect::DialectProfile::by_name("tcl8.6"), reg()).data;
    assert_eq!(
        diff(&before, &same_len),
        None,
        "1->2 leaves the stream unchanged"
    );
}

// ===========================================================================
// minify.rs — continuation / brace-quote preservation / expr / decline edges.
//
// Default-tier minify is semantics-preserving; each pinned output cites the
// tclsh proof that BEFORE and AFTER compute the same value.
// ===========================================================================

fn minc(src: &str) -> String {
    minify_tcl(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), reg())
}

#[test]
fn minify_collapses_backslash_continuation_in_command_subst() {
    // A `\<newline>` continuation inside `[…]` collapses to a single space.
    // tclsh (8.6 + 9.0): both
    //   `set x [string cat a \<nl>    b]; puts $x`  and  the minified
    //   `set x [string cat a b];puts $x`            print `ab`.
    assert_eq!(
        minc("set x [string cat a \\\n    b]\nputs $x\n"),
        "set x [string cat a b];puts $x",
    );
}

#[test]
fn minify_preserves_expr_comma_in_function_call() {
    // The expr tokeniser's catch-all keeps the comma in `atan2($a,$b)` — a
    // naive operator-only tokeniser would drop it and corrupt the call.
    // tclsh (8.6 + 9.0): both
    //   `puts [expr {atan2(1.0,1.0) > 0}]`  and  the minified
    //   `puts [expr {atan2(1.0,1.0)>0}]`    print `1`.
    assert_eq!(
        minc("puts [expr {atan2(1.0,1.0) > 0}]\n"),
        "puts [expr {atan2(1.0,1.0)>0}]",
    );
}

#[test]
fn minify_preserves_expr_membership_braced_list() {
    // `$x ni {a b c}` keeps its braces and the `ni` word-operator spacing.
    // tclsh (8.6 + 9.0): both
    //   `set x b; puts [expr {$x ni {a b c}}]`  and  the minified
    //   `set x b;puts [expr {$x ni {a b c}}]`   print `0`.
    assert_eq!(
        minc("set x b\nputs [expr {$x ni {a b c}}]\n"),
        "set x b;puts [expr {$x ni {a b c}}]",
    );
}

#[test]
fn minify_keeps_braces_in_bare_list_argument() {
    // A bare `{a b}` list argument keeps its braces (dropping them would split
    // it into two words). tclsh (8.6 + 9.0): both
    //   `puts {a b}`  and the minified `puts {a b}`  print `a b`.
    assert_eq!(minc("puts {a b}\n"), "puts {a b}");
}

#[test]
fn minify_preserves_empty_quoted_string() {
    // An empty `""` value cannot drop its quotes (it would vanish).
    // tclsh (8.6 + 9.0): both
    //   `set x ""; puts $x`  and  the minified `set x "";puts $x`
    // print an empty line (length 0).
    assert_eq!(minc("set x \"\"\nputs $x\n"), "set x \"\";puts $x");
}

#[test]
fn minify_keeps_quotes_around_hash_to_avoid_comment() {
    // A `"#…"` value keeps its quotes — dropping them would turn `#notacomment`
    // into a comment in some positions. tclsh (8.6 + 9.0): both
    //   `puts "#notacomment"`  and the minified form  print `#notacomment`.
    assert_eq!(minc("puts \"#notacomment\"\n"), "puts \"#notacomment\"");
}

#[test]
fn minify_drops_redundant_quotes_around_number() {
    // A `"42"` value can safely drop its quotes (no special chars).
    // tclsh (8.6 + 9.0): both `puts "42"` and `puts 42` print `42`.
    assert_eq!(minc("puts \"42\"\n"), "puts 42");
}

#[test]
fn minify_keeps_quotes_when_value_starts_with_brace() {
    // A value whose first char is `{` keeps its quotes (the `can_strip_quotes`
    // leading-`{` guard). tclsh (8.6 + 9.0): both
    //   `puts "{notdropped}"`  and the minified form  print `{notdropped}`.
    assert_eq!(minc("puts \"{notdropped}\"\n"), "puts \"{notdropped}\"");
}

#[test]
fn minify_recurses_into_nested_braced_bodies() {
    // The minifier recurses into nested body args (`while` inside `proc`),
    // stripping the comment and collapsing whitespace at every depth.
    // tclsh (8.6 + 9.0): the proc body is `info complete` and behaves
    // identically; here we pin the exact collapsed form.
    assert_eq!(
        minc("proc f {} {\n  while {$x} {\n    incr x\n  }\n}\n"),
        "proc f {} {while {$x} {incr x}}",
    );
}

#[test]
fn minify_switch_regexp_case_list_preserved() {
    // A `switch -regexp` braced case list keeps its regex pattern and bodies.
    // tclsh (8.6 + 9.0): both
    //   `set x abc; switch -regexp $x {{^a} {puts 1} default {puts 2}}`
    // (original multi-line) and the minified one-line form  print `1`.
    assert_eq!(
        minc("switch -regexp $x {\n  {^a} {puts 1}\n  default {puts 2}\n}\n"),
        "switch -regexp $x {{^a} {puts 1} default {puts 2}}",
    );
}

#[test]
fn minify_expr_comparison_inversion_in_value_context_is_safe() {
    // `!($a == $b)` → `$a != $b` is safe even as a value: both forms yield a
    // boolean. tclsh (8.6 + 9.0):
    //   `set a 1; set b 2; puts [expr {!($a == $b)}]`  →  `1`
    //   `set a 1; set b 2; puts [expr {$a != $b}]`     →  `1`
    assert_eq!(
        minc("set a 1\nset b 2\nputs [expr {!($a == $b)}]\n"),
        "set a 1;set b 2;puts [expr {$a!=$b}]",
    );
}

// ---------------------------------------------------------------------------
// FIXED: minify's double-negation rewrite (`!!x` → `x`) was NOT semantics-
// preserving in an `expr` VALUE context — it dropped the 0/1 boolean coercion.
//
// tclsh 8.6 + 9.0 (via scripts/dev/tclsh_check.sh):
//   `set x 5;set y [expr {!!$x}];puts $y`  ->  1   (!! forces a 0/1 boolean)
//   `set x 5;set y [expr {$x}];puts $y`    ->  5   (bare $x is the raw value)
//
// The fold was only valid where the result is consumed as a condition
// (`if`/`while`), but the minifier processes Expr-role args without that
// context (and `!!x` is unsound even as a subexpression of a boolean expr).
// `shrink_not` no longer applies the `!!x -> x` fold, so `!!$x` is preserved.
#[test]
fn minify_double_negation_in_value_context_must_preserve_value() {
    assert_eq!(
        minc("set x 5\nset y [expr {!!$x}]\nputs $y\n"),
        "set x 5;set y [expr {!!$x}];puts $y",
    );
}
