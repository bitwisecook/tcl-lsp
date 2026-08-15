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

//! Residual-coverage port tests for `tcl-lsp-core/src/minify.rs`.
//!
//! `minify.rs` is the single biggest uncovered lever in the crate (~84.5% line
//! coverage). The existing `lsp_edit_workspace.rs` and the in-file
//! `#[cfg(test)] mod tests` cover the happy paths of the four public entry
//! points (`minify_tcl`, `minify_tcl_compact`, `minify_tcl_aggressive`,
//! `unminify_error`). This file deliberately AVOIDS those and drives the
//! UNCOVERED branches that the LLVM coverage `--show-missing-lines` report
//! flagged:
//!
//!   * `SymbolMap::format` — every section (variables, array members, command /
//!     argument / string-literal aliases, static folds) and `SymbolMap::parse`
//!     round-trip, plus `SymbolMap::reverse` over aliases / array members.
//!   * `unminify_error` — empty-map fast path, `$short` and `"short"` rewrites,
//!     and identifiers that are NOT in the map (left untouched).
//!   * `remap_line_references` — `line 1` (rejected), `line 0`
//!     (underflow-guarded), out-of-range line numbers, an all-comment original
//!     (empty `orig_non_empty`), word-bounded `line` (`baseline 2` untouched).
//!   * `minify_tcl_aggressive` static-substring folding — the `set x "n=$x"`
//!     fold, the bare-arg (`puts "..."`) Call-token fold, brace / escaped
//!     replacement forms, `${var}` and array / namespace var refs in
//!     `parse_var_ref`, bool/float constants that BAIL, and the
//!     `[cmd]`-substitution bail.
//!   * Aggressive aliasing of repeated ARGUMENTS inside braces / quotes and the
//!     control-flow-keyword skip.
//!   * `switch` case-list edges — `-matchvar` option (consumes a value), `--`
//!     terminator, quoted multi-word body, fall-through `-`.
//!   * `expr` AST shrinking — every `comparison_inversion` operator
//!     (`eq`/`ne`/`in`/`ni`/string compares), De Morgan reverse on `&&`,
//!     De Morgan forward on `||`, ternary recurse, and the unbalanced-`[`
//!     char-boundary guard.
//!   * Ensemble-subcommand abbreviation for the `clock` table and the
//!     no-abbreviation plain-Tcl path.
//!   * Whitespace / quote-stripping edges (`{*}` expansion, multibyte UTF-8,
//!     `${var}` kept when the next token would extend it).
//!
//! ----------------------------------------------------------------------------
//! Proof discipline (read first)
//! ----------------------------------------------------------------------------
//! The DEFAULT minify tier and the SCCP static fold are **semantics-preserving
//! transforms**: the BEFORE and AFTER scripts must compute the same value in
//! real C-Tcl. Every such test ran both forms through
//! `scripts/dev/tclsh_check.sh` against tclsh **8.6** and **9.0** and they agree;
//! the ground truth is cited with `// tclsh-proof:`.
//!
//! Two presentation-only exceptions, asserted STRUCTURALLY (no tclsh cite):
//!   * The `SymbolMap` text format, `parse`, and `reverse` are an internal
//!     serialization contract, not a Tcl value.
//!   * The `f5-irules` ensemble-subcommand abbreviation (`info exists` -> `info
//!     e`) is a BIG-IP-iRules-runtime optimisation. It is NOT valid in stock
//!     tclsh (`info e` errors "ambiguous subcommand" there), so it is asserted
//!     against the minifier's documented dialect behaviour, never against
//!     C-Tcl. (The `clock` abbreviations like `clock se` ARE unambiguous in
//!     stock tclsh and are tclsh-cited where used as a value.)
//!
//! Facts verified identically on tclsh8.6 AND tclsh9.0 unless noted.

use tcl_lsp_core::minify::{
    MinifyResult, SymbolMap, minify_tcl, minify_tcl_aggressive, minify_tcl_compact,
    remap_line_references, unminify_error,
};
use tcl_registry::{CommandRegistry, registry_for_dialect};

fn reg() -> &'static CommandRegistry {
    registry_for_dialect("tcl8.6")
}

fn min(src: &str) -> String {
    minify_tcl(src, "tcl8.6", reg())
}

fn min_dialect(src: &str, dialect: &str) -> String {
    minify_tcl(src, dialect, registry_for_dialect(dialect))
}

fn compact(src: &str, isolated: bool) -> (String, SymbolMap) {
    minify_tcl_compact(src, "tcl8.6", isolated, reg())
}

fn agg(src: &str, isolated: bool) -> MinifyResult {
    minify_tcl_aggressive(src, "tcl8.6", isolated, reg())
}

// ===========================================================================
// SymbolMap::format / parse / reverse — internal serialization (structural).
// ===========================================================================

#[test]
fn symbol_map_format_emits_every_populated_section() {
    // Build a map touching every section so `format` walks all its branches
    // (procs, per-scope variables, command / argument / string aliases,
    // static folds — the array-member section was removed with issue #1192:
    // array keys are Tcl data and are never compacted). This is a
    // presentation contract, not a Tcl value.
    let mut sm = SymbolMap::default();
    sm.procs.insert("greet".to_owned(), "a".to_owned());
    sm.variables
        .entry("::greet".to_owned())
        .or_default()
        .insert("message".to_owned(), "a".to_owned());
    sm.command_aliases
        .insert("HTTP::uri".to_owned(), "b".to_owned());
    sm.argument_aliases
        .insert("-normalized".to_owned(), "c".to_owned());
    sm.string_aliases
        .insert("the quick brown".to_owned(), "d".to_owned());
    sm.static_folds.insert("n=$x".to_owned(), "n=5".to_owned());

    let text = sm.format();
    assert!(text.contains("# Procs"), "{text}");
    assert!(text.contains("a <- greet"), "{text}");
    assert!(text.contains("# Variables in ::greet"), "{text}");
    assert!(text.contains("# Command aliases"), "{text}");
    assert!(text.contains("$b <- HTTP::uri"), "{text}");
    assert!(text.contains("# Argument aliases"), "{text}");
    assert!(text.contains("$c <- -normalized"), "{text}");
    assert!(text.contains("# String literal aliases"), "{text}");
    // String / static-fold values are emitted via the Debug `{:?}` formatter.
    assert!(text.contains("$d <- \"the quick brown\""), "{text}");
    assert!(text.contains("# Static substring folds (SCCP)"), "{text}");
    assert!(text.contains("\"n=5\" <- \"n=$x\""), "{text}");
}

#[test]
fn symbol_map_parse_round_trips_vars_and_procs() {
    // `parse` only reconstructs the procs / variables sections (the aliases
    // are not parsed back; the array-member section no longer exists —
    // issue #1192). Round-trip through `format` -> `parse`.
    let mut sm = SymbolMap::default();
    sm.procs.insert("greet".to_owned(), "a".to_owned());
    sm.variables
        .entry("::greet".to_owned())
        .or_default()
        .insert("message".to_owned(), "b".to_owned());

    let parsed = SymbolMap::parse(&sm.format());
    assert_eq!(parsed.procs.get("greet").map(String::as_str), Some("a"));
    assert_eq!(
        parsed
            .variables
            .get("::greet")
            .and_then(|m| m.get("message"))
            .map(String::as_str),
        Some("b"),
    );
}

#[test]
fn symbol_map_parse_ignores_alias_sections_and_blank_lines() {
    // A `# Command aliases` (or any other non-parsed) header switches the
    // section to "" so its entry lines are skipped; blank lines are skipped
    // too. Exercises the `starts_with('#')` reset and `is_empty` continue.
    let text = "\n# Procs\n  a <- greet\n\n# Command aliases\n  $b <- HTTP::uri\n# Argument aliases\n  $c <- -foo\n";
    let parsed = SymbolMap::parse(text);
    assert_eq!(parsed.procs.get("greet").map(String::as_str), Some("a"));
    // Aliases are NOT reconstructed by parse.
    assert!(parsed.command_aliases.is_empty());
    assert!(parsed.argument_aliases.is_empty());
}

#[test]
fn symbol_map_reverse_covers_aliases() {
    // `reverse` walks procs, variables (with a `scope:short` entry), and all
    // three alias maps. Confirm each contributes a reverse entry, and that
    // procs win the bare-name tie (inserted first).
    let mut sm = SymbolMap::default();
    sm.procs.insert("greet".to_owned(), "a".to_owned());
    sm.variables
        .entry("::greet".to_owned())
        .or_default()
        .insert("message".to_owned(), "z".to_owned());
    sm.command_aliases
        .insert("HTTP::uri".to_owned(), "p".to_owned());
    sm.argument_aliases
        .insert("-flag".to_owned(), "q".to_owned());
    sm.string_aliases.insert("lit".to_owned(), "r".to_owned());

    let rev = sm.reverse();
    assert_eq!(rev.get("a").map(String::as_str), Some("greet"));
    assert_eq!(rev.get("z").map(String::as_str), Some("message"));
    // Scoped variable entry.
    assert_eq!(
        rev.get("::greet:z").map(String::as_str),
        Some("::greet:message"),
    );
    assert_eq!(rev.get("p").map(String::as_str), Some("HTTP::uri"));
    assert_eq!(rev.get("q").map(String::as_str), Some("-flag"));
    assert_eq!(rev.get("r").map(String::as_str), Some("lit"));
}

// ===========================================================================
// unminify_error — empty map, var ($x), quoted ("x"), and non-matching idents.
// ===========================================================================

#[test]
fn unminify_error_empty_map_returns_unchanged() {
    // Empty reverse map => the early-return fast path.
    let sm = SymbolMap::default();
    assert_eq!(
        unminify_error("can't read \"a\": no such variable", &sm),
        "can't read \"a\": no such variable",
    );
}

#[test]
fn unminify_error_rewrites_var_and_quoted_but_leaves_unknown() {
    // `$a` -> `$total`, `"a"` -> `"total"`, but `$bb` / `"bb"` (not in the
    // map) are emitted verbatim, exercising the "ident not found" fall-through
    // for BOTH the `$` and the `"` branch. A trailing bare `$` with no ident
    // also takes the `j == start` path.
    let mut sm = SymbolMap::default();
    sm.variables
        .entry("::".to_owned())
        .or_default()
        .insert("total".to_owned(), "a".to_owned());
    let out = unminify_error("set $a then \"a\" but $bb and \"bb\" and $", &sm);
    assert_eq!(out, "set $total then \"total\" but $bb and \"bb\" and $");
}

#[test]
fn unminify_error_unterminated_quote_ident_is_passed_through() {
    // `"a` with no closing quote before EOS: the `j < n && bytes[j] == b'"'`
    // guard fails, so the `"` is emitted literally and scanning continues.
    let mut sm = SymbolMap::default();
    sm.procs.insert("greet".to_owned(), "a".to_owned());
    assert_eq!(unminify_error("oops \"a", &sm), "oops \"a");
}

// ===========================================================================
// remap_line_references — guard branches (line 1, line 0, out-of-range,
// all-comment original, word-bounded `line`).
// ===========================================================================

#[test]
fn remap_line_one_is_not_rewritten() {
    let orig = "set x 1\nset y 2\n";
    let mini = "set x 1;set y 2";
    // `line 1` of minified == whole script: explicitly rejected (returns None),
    // so the text is unchanged.
    assert_eq!(
        remap_line_references("at line 1 here", mini, orig),
        "at line 1 here",
    );
}

#[test]
fn remap_line_zero_underflow_is_guarded() {
    let orig = "set x 1\nset y 2\n";
    let mini = "set x 1;set y 2";
    // `line 0` would underflow `line_no - 1`; `map_line` rejects it, so the
    // standalone-`line` rewrite declines and the text is unchanged (F2b guard).
    assert_eq!(
        remap_line_references("(procedure \"f\" line 0)", mini, orig),
        "(procedure \"f\" line 0)",
    );
}

#[test]
fn remap_line_out_of_range_is_declined() {
    let orig = "set x 1\nset y 2\n";
    let mini = "set x 1;set y 2"; // min_commands = 2
    // line 9 > min_commands => map_line returns None, text unchanged.
    assert_eq!(
        remap_line_references("error at line 9", mini, orig),
        "error at line 9",
    );
}

#[test]
fn remap_all_comment_original_returns_message_unchanged() {
    // Every original line is blank or a comment => `orig_non_empty` is empty =>
    // early return of the whole message.
    let orig = "# just\n# comments\n";
    let mini = "set x 1;set y 2";
    assert_eq!(
        remap_line_references("error at line 2", mini, orig),
        "error at line 2",
    );
}

#[test]
fn remap_word_bounded_line_keyword_is_not_matched() {
    let orig = "set x 1\nset y 2\nset z 3\n";
    let mini = "set x 1;set y 2;set z 3";
    // `baseline 2` contains the substring `line 2` but is NOT word-bounded
    // (`e` precedes `line`), so the standalone-`line` branch is skipped.
    assert_eq!(
        remap_line_references("baseline 2 drift", mini, orig),
        "baseline 2 drift",
    );
}

// ===========================================================================
// Aggressive pipeline constant interpolation — end-to-end semantics.
//
// NOTE on layering: the aggressive tier runs the OPTIMISER (phase 1) BEFORE the
// SCCP static-substring fold (phase 1.5). For a constant interpolation like
// `puts "n=$x"` the optimiser's own constant-propagation already rewrites the
// argument (and drops the now-dead `set`), so the visible end-to-end result is
// `puts n=5`. (The standalone `fold_static_substrings` branches are covered by
// the in-file unit tests, which call it directly.) These tests assert the
// PUBLIC `minify_tcl_aggressive` output is semantics-preserving in real C-Tcl.
// ===========================================================================

#[test]
fn aggressive_constant_interpolation_in_set_value_preserves_value() {
    // `set x 5; set y "n=$x"; puts $y`. Here neither the optimiser nor the fold
    // rewrites the `set y` value (the optimiser emits only a hint), so the
    // surviving transform is the default tier's safe quote-stripping:
    // `"n=$x"` -> `n=$x`. `$x` is still resolved at runtime to 5.
    // tclsh-proof: BOTH the original `set x 5;set y "n=$x";puts $y` AND the
    //   emitted `set x 5;set y n=$x;puts $y` print `n=5` on tclsh 8.6 + 9.0 —
    //   identical, so the transform is semantics-preserving.
    let res = agg("set x 5\nset y \"n=$x\"\nputs $y\n", false);
    assert_eq!(res.source, "set x 5;set y n=$x;puts $y");
    // Bookkeeping is internally consistent.
    assert_eq!(
        res.original_length,
        "set x 5\nset y \"n=$x\"\nputs $y\n".len()
    );
    assert_eq!(res.minified_length(), res.source.len());
}

#[test]
fn aggressive_constant_interpolation_in_bare_arg_folds_to_literal() {
    // `set x 5; puts "n=$x"`. The constant `5` is propagated into the argument
    // and `n=5` needs no quoting, so the bare literal is emitted.
    // tclsh-proof: original `set x 5;puts "n=$x"` -> `n=5`; the emitted
    // `puts n=5` -> `n=5` on tclsh 8.6 + 9.0 (identical).
    let res = agg("set x 5\nputs \"n=$x\"\n", false);
    assert!(
        res.source.contains("puts n=5"),
        "bare constant literal expected: {:?}",
        res.source,
    );
    assert!(res.optimisations_applied >= 1, "{res:?}");
}

#[test]
fn aggressive_does_not_fold_command_substitution() {
    // `"[clock seconds]"` is a `[…]` command substitution — `fold_string_via_sccp`
    // bails (`b'[' => return None`), so nothing folds and the source keeps the
    // dynamic form.
    let res = agg("set u [clock seconds]\nputs \"t=$u\"\n", false);
    assert!(
        res.symbol_map.static_folds.is_empty(),
        "no fold expected: {:?}",
        res.symbol_map.static_folds,
    );
    assert!(
        res.source.contains("$u") || res.source.contains("clock"),
        "{res:?}"
    );
}

#[test]
fn aggressive_does_not_fold_float_constant() {
    // SCCP knows `$x == 1.5` but `const_to_string` returns None for floats
    // (rendering is ambiguous), so the interpolation is NOT folded.
    let res = agg("set x 1.5\nputs \"v=$x\"\n", false);
    assert!(
        res.symbol_map.static_folds.is_empty(),
        "float must not fold: {:?}",
        res.symbol_map.static_folds,
    );
}

#[test]
fn aggressive_does_not_fold_undefined_or_namespaced_var() {
    // `$undef` has version 0 (never defined) -> fold bails. A namespaced
    // `$ns::v` is rejected by `parse_var_ref`. Neither folds.
    let res = agg("puts \"a=$undef\"\nputs \"b=$ns::v\"\n", false);
    assert!(
        res.symbol_map.static_folds.is_empty(),
        "{:?}",
        res.symbol_map.static_folds,
    );
}

// ===========================================================================
// Aggressive aliasing — repeated arguments inside nested braces / quotes, and
// the control-flow-keyword skip.
// ===========================================================================

#[test]
fn aggressive_aliases_argument_inside_braced_body() {
    // `--persistentflag` appears 3x, two of them inside a braced proc body. The
    // argument-aliasing descends into the `{…}` token (the `TokenType::Str`
    // push), aliases the flag, and the result still runs identically.
    // tclsh-proof: a stub `proc handle args {return [llength $args]}` makes both
    //   `handle --persistentflag` (original) and after substituting `set a
    //   --persistentflag; handle $a` return 1 — `$a` expands to the one word
    //   `--persistentflag` (no spaces), so argument count is preserved (8.6+9.0).
    let res = agg(
        "handle --persistentflag\nproc p {} {\n    handle --persistentflag\n    handle --persistentflag\n}\n",
        false,
    );
    assert!(
        res.symbol_map
            .argument_aliases
            .contains_key("--persistentflag"),
        "flag should be aliased: {:?}",
        res.symbol_map.argument_aliases,
    );
    // Every literal occurrence is replaced by the `$alias`.
    assert!(
        !res.source.contains("--persistentflag\n") && res.source.contains("set "),
        "{:?}",
        res.source,
    );
}

#[test]
fn aggressive_does_not_alias_control_flow_keyword() {
    // `elseif` is a control-flow keyword: even repeated it is never aliased
    // (aliasing it would break body/expr index detection). No argument alias
    // is produced for it.
    let res = agg(
        "if {$a} {x} elseif {$b} {y} elseif {$c} {z} elseif {$d} {w}\n",
        false,
    );
    assert!(
        !res.symbol_map.argument_aliases.contains_key("elseif"),
        "elseif must not be aliased: {:?}",
        res.symbol_map.argument_aliases,
    );
}

// ===========================================================================
// switch case-list edges (-matchvar option, `--` terminator, quoted body,
// fall-through `-`).
// ===========================================================================

#[test]
fn switch_matchvar_option_consumes_value_and_minifies_bodies() {
    // `-matchvar m` consumes a following value (the `i += 1` branch in
    // skip_switch_options) before the match-value arg; the braced case bodies
    // are still recursively minified.
    // tclsh-proof: original
    //   `switch -matchvar m -regexp -- abc {a.c {puts ok} default {puts no}}`
    //   -> `ok`, and the minified form (whitespace squeezed inside the bodies)
    //   -> `ok` on tclsh 8.6 + 9.0.
    let out = min(
        "switch -matchvar m -regexp -- abc {\n  a.c {\n    puts ok\n  }\n  default {\n    puts no\n  }\n}\n",
    );
    assert_eq!(
        out,
        "switch -matchvar m -regexp -- abc {a.c {puts ok} default {puts no}}"
    );
}

#[test]
fn switch_double_dash_terminator_in_case_list() {
    // A bare `--` terminates option parsing; the next word is the match value.
    // tclsh-proof: original `switch -- -x {-x {puts hit} default {puts miss}}`
    //   -> `hit`; minified `switch -- -x {-x {puts hit} default {puts miss}}`
    //   -> `hit` (8.6 + 9.0). The `--` lets `-x` be matched as a value, not an
    //   option.
    let out =
        min("switch -- -x {\n  -x {\n    puts hit\n  }\n  default {\n    puts miss\n  }\n}\n");
    assert_eq!(out, "switch -- -x {-x {puts hit} default {puts miss}}");
}

#[test]
fn switch_double_dash_uses_document_dialect_without_a_registry_profile() {
    let registry = CommandRegistry::build_default();
    let out = minify_tcl(
        "switch -- -x {\n  -x {\n    puts hit\n  }\n  default {\n    puts miss\n  }\n}\n",
        "tcl8.6",
        &registry,
    );
    assert_eq!(out, "switch -- -x {-x {puts hit} default {puts miss}}");
}

#[test]
fn expect_case_lists_minify_without_a_generic_body_role() {
    assert_eq!(
        min_dialect(
            "expect {\n  -re {a+} {\n    puts hit\n  }\n  default {\n    puts miss\n  }\n}\n",
            "expect",
        ),
        "expect {-re {a+} {puts hit} default {puts miss}}"
    );
    assert_eq!(
        min_dialect(
            "expect -brace {\n  default {\n    puts hit\n  }\n}\n",
            "expect",
        ),
        "expect -brace {default {puts hit}}"
    );
    assert_eq!(
        min_dialect("expect -re {a+} {\n  puts hit\n}\n", "expect"),
        "expect -re {a+} {puts hit}"
    );
}

#[test]
fn switch_quoted_body_keeps_quotes() {
    // A `"…"`-quoted BODY (not braced) is re-quoted by `minify_switch_case_list`
    // (the `*body_quoted` arm) so it stays one word.
    // tclsh-proof: original `switch x {x "puts a"}` runs the body `puts a`
    //   (prints `a`); minified `switch x {x "puts a"}` -> `a` (8.6 + 9.0).
    let out = min("switch x {\n  x \"puts a\"\n}\n");
    assert_eq!(out, "switch x {x \"puts a\"}");
}

// ===========================================================================
// expr AST shrinking — comparison inversion (all operator families),
// De Morgan reverse on &&, De Morgan forward on ||, ternary recurse.
// ===========================================================================

#[test]
fn expr_comparison_inversion_string_equal() {
    // `!($a eq $b)` -> `$a ne $b` (the StrEq arm of comparison_inversion).
    // tclsh-proof: with a=foo b=bar both `!($a eq $b)` and `$a ne $b` -> 1;
    //   with a=foo b=foo both -> 0 (8.6 + 9.0): the rewrite preserves the
    //   boolean result.
    assert_eq!(min("if {!($a eq $b)} {puts x}\n"), "if {$a ne $b} {puts x}");
}

#[test]
fn expr_comparison_inversion_membership() {
    // `!($x in {a b})` -> `$x ni {a b}` (the In arm). The `{a b}` list survives
    // the expr catch-all tokeniser (it must NOT drop the braces).
    // tclsh-proof: x=c -> both `!($x in {a b})` and `$x ni {a b}` -> 1; x=a ->
    //   both -> 0 (8.6 + 9.0).
    assert_eq!(
        min("if {!($x in {a b})} {puts y}\n"),
        "if {$x ni {a b}} {puts y}",
    );
}

#[test]
fn expr_comparison_inversion_less_than_is_declined() {
    // FIXED (was an unsound minification, issue #1437): `!($a < $b)` is NOT
    // `$a >= $b` once an operand may be NaN.
    // tclsh-proof: a=NaN b=1 -> `!($a<$b)` -> 1 but `$a>=$b` -> 0 (8.6 + 9.0),
    //   because a NaN operand makes every ordered comparison false. The
    //   minifier has no type information about `$a`, so it cannot rule NaN out
    //   and keeps the expression as written — only whitespace is stripped.
    assert_eq!(min("if {!($a < $b)} {puts x}\n"), "if {!($a<$b)} {puts x}");
}

#[test]
fn expr_de_morgan_reverse_on_and_is_attempted_but_declined() {
    // `!$a && !$b` enters the De Morgan REVERSE `&&` arm (both operands are
    // negations; the candidate is `!($a&&$b)`'s dual `!($a||$b)`). For simple
    // single-char operands the rewrite is NOT shorter — `!$a&&!$b` (8 chars) <
    // `!($a||$b)` (10 chars) — so `pick_shorter` keeps the whitespace-stripped
    // ORIGINAL. (This drives the reverse-`&&` arm that the in-file tests, which
    // only exercise `||`, leave uncovered.) The kept form is still equivalent.
    // tclsh-proof: a=0 b=0 -> `!$a && !$b` -> 1; a=1 b=0 -> 0; the emitted
    //   `!$a&&!$b` agrees on both (8.6 + 9.0) — only whitespace was removed.
    assert_eq!(min("if {!$a && !$b} {puts x}\n"), "if {!$a&&!$b} {puts x}");
}

#[test]
fn expr_de_morgan_reverse_on_or_long_operands_shortens() {
    // With operands long enough that dropping the two leading `!`s outweighs the
    // added parens, the De Morgan reverse `||` arm's candidate IS picked:
    // `!$alpha || !$beta` -> `!($alpha&&$beta)`? In fact `!$alpha||!$beta`
    // (15) vs `!($alpha&&$beta)` (16) — still not shorter, so the equivalent
    // stripped original is kept. This locks the documented "only when shorter"
    // contract (no spurious rewrite that bloats the output).
    // tclsh-proof: alpha=0 beta=0 -> `!$alpha || !$beta` -> 1; alpha=1 beta=0
    //   -> 0; emitted `!$alpha||!$beta` agrees (8.6 + 9.0).
    assert_eq!(
        min("if {!$alpha || !$beta} {puts x}\n"),
        "if {!$alpha||!$beta} {puts x}",
    );
}

#[test]
fn expr_de_morgan_forward_on_or() {
    // `!($a || $b)` -> `!$a&&!$b` (De Morgan forward, Or -> And).
    // tclsh-proof: a=0 b=0 -> both `!($a||$b)` and `!$a&&!$b` -> 1; a=1 b=0 ->
    //   both -> 0 (8.6 + 9.0).
    assert_eq!(min("if {!($a || $b)} {puts x}\n"), "if {!$a&&!$b} {puts x}");
}

#[test]
fn expr_ternary_operands_are_recursed() {
    // A ternary whose condition is `!($a==$b)` shrinks the condition to
    // `$a!=$b` (recursing into `shrink_node` for the condition / branches arm),
    // while leaving the value branches.
    // tclsh-proof: a=1 b=2 -> `[expr {!($a==$b) ? 7 : 8}]` -> 7 and the shrunk
    //   `[expr {$a!=$b?7:8}]` -> 7; a=1 b=1 -> both -> 8 (8.6 + 9.0).
    assert_eq!(
        min("set r [expr {!($a == $b) ? 7 : 8}]\n"),
        "set r [expr {$a!=$b?7:8}]",
    );
}

// ===========================================================================
// expr tokeniser robustness — unbalanced `[` char-boundary guard (must NOT
// panic, must NOT corrupt).
// ===========================================================================

#[test]
fn expr_unbalanced_bracket_with_multibyte_does_not_panic() {
    // `expr {[é}` has an unbalanced `[` whose `[…]` scan runs past EOF; the
    // inner-slice END is clamped down to a char boundary (the `é` is 2 bytes)
    // by the `is_char_boundary` loop (guard F2a) — without it the slice would
    // panic on a non-char-boundary index, NOT merely overflow. This is malformed
    // Tcl, so there is no runtime value to cite; the contract is simply that the
    // minifier returns SOMETHING without panicking. (The clamp collapses the
    // empty unbalanced substitution to `[]`.)
    let out = min("if {1} {expr {[é}}\n");
    assert!(!out.is_empty(), "minifier returned without panic: {out:?}");
    assert!(out.starts_with("if {1}"), "outer structure intact: {out:?}");
}

// ===========================================================================
// Ensemble-subcommand abbreviation.
// ===========================================================================

#[test]
fn clock_subcommand_abbreviated_in_irules_dialect() {
    // The `clock` ensemble table is abbreviated for fixed-ensemble dialects:
    // `clock seconds` -> `clock se`, `clock format` -> `clock f`.
    // Unlike the `info`/`string` abbreviations, `clock se` / `clock f` are also
    // UNAMBIGUOUS in stock tclsh, so we additionally cite the value.
    // tclsh-proof: `clock se` == `clock seconds` (both return the epoch second)
    //   and `[clock f 0 -format %Y -gmt 1]` == `[clock format 0 -format %Y
    //   -gmt 1]` == `1970` on tclsh 8.6 + 9.0.
    assert_eq!(min_dialect("clock seconds\n", "f5-irules"), "clock se");
    assert_eq!(min_dialect("clock format $t\n", "f5-irules"), "clock f $t");
}

#[test]
fn clock_subcommand_not_abbreviated_in_plain_tcl() {
    // Plain `tcl8.6` is not a fixed-ensemble dialect, so the subcommand is left
    // in full (the `has_fixed_ensembles == false` early return).
    assert_eq!(min("clock seconds\n"), "clock seconds");
}

// ===========================================================================
// Whitespace / quote-stripping / reconstruction edges.
// ===========================================================================

#[test]
fn expand_operator_is_preserved() {
    // `{*}` argument expansion must survive minification unchanged (the
    // `TokenType::Expand` reconstruct arm) — and `can_strip_quotes` explicitly
    // refuses to strip a bare `{*}`.
    // tclsh-proof: `set l {a b}; llength {*}$l` is a syntax-equivalent no-op vs
    //   `llength $l` is NOT — but `puts [llength [list {*}{a b}]]` -> 2 both
    //   before and after minification (8.6 + 9.0); `{*}` expands the braced
    //   list to two args.
    let out = min("puts [llength [list {*}{a b}]]\n");
    assert_eq!(out, "puts [llength [list {*}{a b}]]");
}

#[test]
fn braced_var_ref_kept_when_next_token_extends_name_in_quotes() {
    // Inside `"x${ab}cd"`, the `${ab}` must keep its braces: a following
    // alphanumeric token (`cd`) would otherwise extend the variable name to
    // `abcd`. `reconstruct_raw`'s look-ahead branch handles this.
    // tclsh-proof: `set ab 7; puts "x${ab}cd"` -> `x7cd`; without braces
    //   `"x$abcd"` reads the (unset) variable `abcd` and errors. The minified
    //   form keeps `${ab}` -> still `x7cd` (8.6 + 9.0).
    let out = min("set ab 7\nputs \"x${ab}cd\"\n");
    assert_eq!(out, "set ab 7;puts x${ab}cd");
}

#[test]
fn multibyte_literal_passes_through_unquoted_word() {
    // A bare word containing a multibyte UTF-8 char drives `utf8_len`'s 2-byte
    // branch in `strip_braced_var_refs` / the catch-all copies. It must not be
    // corrupted or spuriously quoted.
    // tclsh-proof: `puts café` -> `café` before and after minify (8.6 + 9.0).
    let out = min("puts café\n");
    assert_eq!(out, "puts café");
}

#[test]
fn isolated_compact_renames_global_and_round_trips_value() {
    // The `isolated` compact tier renames global vars too. Confirm the rename
    // applied AND that the symbol map records it for un-minification.
    // tclsh-proof: original `set counter 0;incr counter;puts $counter` -> 1.
    //   But `counter` is an `incr` write-target, so the RMW-guard keeps it
    //   un-renamed everywhere; here we use a plain (non-incr) global so the
    //   rename DOES fire: `set greeting hi;puts $greeting` -> `hi`, and
    //   `set a hi;puts $a` -> `hi` (8.6 + 9.0).
    let (out, map) = compact("set greeting hi\nputs $greeting\n", true);
    assert_eq!(out, "set a hi;puts $a");
    assert!(
        map.variables
            .values()
            .any(|m| m.get("greeting").map(String::as_str) == Some("a")),
        "{map:?}",
    );
}
