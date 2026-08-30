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

//! Residual-coverage tests for the Tcl **formatting** subsystem:
//!
//! * `src/formatting/engine.rs` — the token-aware reformatter (lowest-coverage
//!   file in the crate): UTF-8 reconstruction, command-substitution rebuild,
//!   switch-body pattern/body layout, `&&`/`||` expression wrapping, long-line
//!   backslash splitting (bare-word *and* inside-quoted-string), commented-out
//!   code splitting, blank-line policy, the `for` special case, and every
//!   `FormatterConfig` toggle.
//! * `src/formatting/config.rs` — the `Tabs` indent branch of `make_indent`.
//! * `src/formatting/mod.rs` — `formatting` / `formatting_with` edit
//!   construction, `range_formatting` (prefix-depth walk, EOF clamp, CRLF
//!   finalisation, no-op detection), and `brace_delta`.
//!
//! ## Proof discipline
//!
//! A Tcl formatter must be **layout-only**: the BEFORE and AFTER scripts must
//! mean the same thing to the interpreter. Every test here that asserts
//! *semantic equivalence* was verified against real C-Tcl — `tclsh8.6` and
//! `tclsh9.0` via `scripts/dev/tclsh_check.sh` — and cites the ground truth in
//! a `// tclsh-proof:` comment. Tests asserting purely *textual* choices
//! (exact indent width, where a `}` lands, which column a chunk starts) are
//! structural and need no interpreter proof; where it was cheap to also prove
//! the reflow inert, those carry a proof too.
//!
//! ## Bug found
//!
//! `fmt_long_quoted_string_split_must_preserve_string_BUG` documents a real
//! semantics-changing defect: when `split_long_line` wraps a long line by
//! breaking at a space **inside a double-quoted string**, the inserted
//! `\<newline>` + continuation indent adds an extra space to the string's
//! value (Tcl collapses `\<nl>+ws` to one space but keeps the pre-existing
//! space). That test asserts the CORRECT (equivalence) behaviour and is left
//! failing on purpose. See its body for the tclsh proof.

use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::formatting::{
    FormatterConfig, IndentStyle, format_tcl, formatting, formatting_with, range_formatting,
};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

fn reg() -> &'static CommandRegistry {
    static_context_for("tcl8.6").commands()
}

fn fmt(src: &str) -> String {
    format_tcl(src, &FormatterConfig::default(), reg())
}

fn fmt_with(src: &str, cfg: &FormatterConfig) -> String {
    format_tcl(src, cfg, reg())
}

// UTF-8 reconstruction — `utf8_len` 2/3/4-byte lead-byte branches and the
// `normalise_backslash_newline` UTF-8 copy path (engine.rs 83-93, 73-77).

#[test]
fn fmt_preserves_multibyte_utf8_words() {
    // 2-byte (é, U+00E9), 3-byte (中, U+4E2D), and 4-byte (😀, U+1F600) chars
    // round-trip byte-for-byte through token reconstruction; the formatter
    // must not corrupt them when walking the source by UTF-8 char length.
    // tclsh-proof: `puts é` / `puts 中文` / `puts 😀` print the same bytes
    // before and after (no reflow occurs on these short lines, so equivalence
    // is trivially the identity — the point is the byte-exact round-trip).
    assert_eq!(fmt("puts \u{00e9}\n"), "puts \u{00e9}\n");
    assert_eq!(fmt("puts \u{4e2d}\u{6587}\n"), "puts \u{4e2d}\u{6587}\n");
    assert_eq!(fmt("puts \u{1f600}\n"), "puts \u{1f600}\n");
}

#[test]
fn fmt_collapses_backslash_newline_around_multibyte_chars() {
    // The `\<newline>` continuation collapse runs the UTF-8 copy loop
    // (`utf8_len`) over the non-continuation bytes; 2-byte (é), 3-byte (中) and
    // 4-byte (😀) chars straddling the copy must each stay intact while the
    // continuation in the `[cmd]` collapses to a single space (the lexer joins
    // it anyway, so this is semantics-preserving).
    // tclsh-proof (8.6 + 9.0):
    //   `set y [string cat é \<nl>  x]; puts $y` → `éx`  (== `string cat é x`)
    //   `set y [string cat 中 \<nl>  x]; puts $y` → `中x` (== `string cat 中 x`)
    //   `set y [string cat 😀 \<nl> x]; puts $y` → `😀x` (== `string cat 😀 x`)
    assert_eq!(
        fmt("set y [string cat \u{00e9} \\\n  x]\n"),
        "set y [string cat \u{00e9} x]\n",
    );
    assert_eq!(
        fmt("set y [string cat \u{4e2d} \\\n  x]\n"),
        "set y [string cat \u{4e2d} x]\n",
    );
    assert_eq!(
        fmt("set y [string cat \u{1f600} \\\n  x]\n"),
        "set y [string cat \u{1f600} x]\n",
    );
}

// Command-substitution `[...]` reconstruction with an embedded continuation
// (engine.rs reconstruct_raw Cmd arm, line 99/103).

#[test]
fn fmt_collapses_continuation_in_command_substitution() {
    // A `\<newline>` inside `[...]` is collapsed to one space — the lexer joins
    // it regardless, so this is semantics-preserving and keeps the formatter
    // idempotent.
    // tclsh-proof:
    //   `set y [string cat a \<nl>  b c]; puts $y` → `abc` (8.6 + 9.0)
    //   `set y [string cat a b c]; puts $y`        → `abc` (8.6 + 9.0)
    assert_eq!(
        fmt("set y [string cat a \\\n  b c]\n"),
        "set y [string cat a b c]\n",
    );
}

// switch-body layout — `format_switch_body` (engine.rs 403-495): raw bodies,
// `-` fall-through, braced bodies, empty braced body, dangling final pattern.

#[test]
fn fmt_switch_raw_unbraced_bodies_reindented() {
    // Pattern + bare (unbraced) body words are re-emitted on one indented line
    // (the `else` arm of the body classification). Structural reindent only.
    assert_eq!(
        fmt("switch $x {\n  a body1\n  b body2\n}\n"),
        "switch $x {\n    a body1\n    b body2\n}\n",
    );
}

#[test]
fn fmt_switch_multitoken_raw_body_reindented() {
    // A raw (unbraced) switch body that lexes to MULTIPLE tokens — here
    // `$y(z)` (a variable with an array index) — accumulates onto the current
    // element (the `cur.as_mut()` multi-token append arm of `format_switch_body`)
    // and is re-emitted verbatim on the pattern line. Structural reindent only:
    // the reflow does not change which token is the body, so behaviour is
    // identical (and identically erroneous — `$y(z)` is evaluated as a command)
    // before and after per tclsh 8.6 + 9.0.
    assert_eq!(
        fmt("switch $x {\n  a $y(z)\n}\n"),
        "switch $x {\n    a $y(z)\n}\n",
    );
}

#[test]
fn fmt_switch_dash_fallthrough_and_braced_body() {
    // `a -` keeps the fall-through marker on the pattern line; the next
    // pattern's braced body expands. Exercises the `body.text == "-"` and
    // `body.is_braced` arms together.
    // tclsh-proof (behaviour identical before/after the reflow):
    //   `switch a { a - b {puts two} default {puts d} }` → `two` (8.6 + 9.0)
    //   same with the expanded brace layout                → `two` (8.6 + 9.0)
    assert_eq!(
        fmt("switch $x {\n  a -\n  b {puts 2}\n}\n"),
        "switch $x {\n    a -\n    b {\n        puts 2\n    }\n}\n",
    );
}

#[test]
fn fmt_switch_empty_braced_body_stays_inline() {
    // An empty `{}` body collapses to `{}` on the pattern line (the
    // `formatted.trim().is_empty()` arm of `format_switch_body`).
    assert_eq!(
        fmt("switch -- $x {\n  a {}\n}\n"),
        "switch -- $x {\n    a {}\n}\n",
    );
}

#[test]
fn fmt_switch_dangling_final_pattern_emitted_alone() {
    // A trailing element with no following body hits the `else` (odd-count)
    // branch: the pattern is emitted on its own indented line.
    assert_eq!(
        fmt("switch $x {\n  a {puts 1}\n  default\n}\n"),
        "switch $x {\n    a {\n        puts 1\n    }\n    default\n}\n",
    );
}

// Expression wrapping — `find_expr_break_points` + `wrap_braced_expr`
// (engine.rs 501-570): split only top-level `&&`/`||`, never inside
// `() {} [] ""` or after a `\` escape.

#[test]
fn fmt_expr_wrap_preserves_parens_and_quotes() {
    // Force a wrap (line > max_line_length). `($aaaa && $bbbb)` and
    // `($dddd && "e && f")` must each stay a single chunk — the parens (depth)
    // and the quoted `&&` are skipped; only top-level `||` breaks.
    // tclsh-proof (the multi-line expr evaluates identically):
    //   `if {($a && $b) || 0 || ($a && 1)} {...}` → YES (8.6 + 9.0)
    //   same condition wrapped over lines at `||`  → YES (8.6 + 9.0)
    let input = "if {($aaaa && $bbbb) || $cccccccccccccccccccccc || ($dddd && \"e && f\") || $gggggggggggggggggggggggg || $hhhhhhhhhhhhhhhhhhhhhhhh || $iiiiiiiiiiiiiiiiiiiiiiii} {\nputs hi\n}\n";
    let expected = "if {\n    ($aaaa && $bbbb)\n    || $cccccccccccccccccccccc\n    || ($dddd && \"e && f\")\n    || $gggggggggggggggggggggggg\n    || $hhhhhhhhhhhhhhhhhhhhhhhh\n    || $iiiiiiiiiiiiiiiiiiiiiiii\n} {\n    puts hi\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn fmt_expr_wrap_skips_operators_in_brackets_and_braces() {
    // `[string match {a||b} $x]` carries `||` inside both `[]` and `{}`; both
    // must be skipped so the wrap only breaks at the two top-level `||`.
    let input = "if {[string match {a||b} $xxxxxxxxxxxxxxxxxxxxxxxxxxxx] || $yyyyyyyyyyyyyyyyyyyyyyyyyyyyyy || $zzzzzzzzzzzzzzzzzzzzzzzzzzzzzz || $wwwwwwwwwwwwwwwwwwwwwwwwwww} {\nputs ok\n}\n";
    let expected = "if {\n    [string match {a||b} $xxxxxxxxxxxxxxxxxxxxxxxxxxxx]\n    || $yyyyyyyyyyyyyyyyyyyyyyyyyyyyyy\n    || $zzzzzzzzzzzzzzzzzzzzzzzzzzzzzz\n    || $wwwwwwwwwwwwwwwwwwwwwwwwwww\n} {\n    puts ok\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn fmt_expr_wrap_skips_quoted_operand_in_while() {
    // `$x eq "z"` — the `"z"` toggles quote state; the only break points are
    // the top-level `||`. Drives the `while` (Expr-role arg) path and the
    // in-quotes skip of `find_expr_break_points`.
    let input = "while {$aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa eq \"z\" || $bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb || $cccccccccccccccccccccccccccccc || $dddddddddddddddddddddddddddddd} {\nbreak\n}\n";
    let expected = "while {\n    $aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa eq \"z\"\n    || $bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\n    || $cccccccccccccccccccccccccccccc\n    || $dddddddddddddddddddddddddddddd\n} {\n    break\n}\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn fmt_short_expr_is_not_wrapped() {
    // The complementary negative: an `&&`/`||` expr that fits stays on one line
    // (the `find_expr_break_points` non-empty-but-fits path returns early).
    assert_eq!(
        fmt("if {$a && $b} {\nputs hi\n}\n"),
        "if {$a && $b} {\n    puts hi\n}\n"
    );
}

// Long-line backslash splitting — `split_long_line` / `greedy_split` /
// `find_splittable_spaces` (engine.rs 602-795): bare-word splitting (safe) and
// the commented-out-code split path (`split_commented_code`).

#[test]
fn fmt_long_word_line_splits_with_backslash_continuation() {
    // A long bare-word command line is split at a top-level space with a `\`
    // continuation and a continuation-indented remainder. SAFE: between bare
    // words a space is a word separator, and Tcl collapses `\<nl>+ws` back to a
    // single separator, so the argument vector is unchanged.
    // tclsh-proof:
    //   `proc p {args} {return [llength $args]:[lindex $args end]}` then
    //   `p argumentone ... argumentnine` (one line)  → `9:argumentnine` (8.6+9.0)
    //   the same call split at the last space with `\<nl>    ` → `9:argumentnine`
    let input = "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight argumentnine\n";
    let expected = "mycommand argumentone argumenttwo argumentthree argumentfour argumentfive argumentsix argumentseven argumenteight \\\n    argumentnine\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn fmt_long_commented_out_code_splits() {
    // A long comment whose text after `#` is itself code (`#mycommand …`, no
    // space after `#`) is split via `split_commented_code`: the first segment
    // is re-prefixed with `#`, the rest indented. It stays a comment — inert,
    // so no semantics change regardless of layout.
    let input = "#mycommand aaaaaaaaaa bbbbbbbbbb cccccccccc dddddddddd eeeeeeeeee ffffffffff gggggggggg hhhhhhhhhh iiiiiiiiii jjjjjjjjjj kkkkkkkkkk\n";
    let expected = "#mycommand aaaaaaaaaa bbbbbbbbbb cccccccccc dddddddddd eeeeeeeeee ffffffffff gggggggggg hhhhhhhhhh iiiiiiiiii \\\n    jjjjjjjjjj kkkkkkkkkk\n";
    assert_eq!(fmt(input), expected);
}

#[test]
fn fmt_short_commented_code_not_split() {
    // The complementary negative for `split_commented_code`: a short
    // commented-out command (under max_line_length) is left on one line, but
    // its internal `\<newline>` continuation is still collapsed (comment is
    // inert).
    assert_eq!(fmt("#set x [a \\\n b]\n"), "#set x [a b]\n");
}

// REGRESSION (was a bug, now fixed) — a long line that can only be split INSIDE
// a double-quoted string must be left over-length rather than broken, because
// breaking there changes the string's value. The old `split_long_line` fell
// back to `find_quoted_string_spaces` and inserted `\<newline>` + indent at a
// space that is *string data*, adding an extra space; the fix removes that
// fallback (engine.rs `split_long_line`).

#[test]
fn fmt_long_quoted_string_is_left_unsplit_preserving_value() {
    // A single long `"…"` string argument, too long to split anywhere except
    // inside the quotes, must NOT be broken: inside a `"…"`, Tcl collapses
    // `\<nl>+whitespace` to ONE space but keeps the space that already preceded
    // the break, so a `\<newline>` split would make the string GAIN a space and
    // change the program's data.
    //
    // tclsh-proof (8.6 + 9.0), minimal — the *would-be* broken form differs:
    //   set a "x y"; set b "x \<nl>    y";
    //   [string length $a]=3  [string length $b]=4  [string equal $a $b]=0
    //
    // Fixed behaviour: the splitter does not break inside a double-quoted
    // string, so the line is emitted over-length with its single-space
    // separators intact (layout is best effort; the data must never change).
    let input = "set s \"aaaaaaaaaa bbbbbbbbbb cccccccccc dddddddddd eeeeeeeeee ffffffffff gggggggggg hhhhhhhhhh iiiiiiiiii jjjjjjjjjj kkkkkkkkkk\"\n";
    let out = fmt(input);
    // The string literal's run of token-separating spaces must remain single
    // spaces: no `"…  …"` double space may appear inside the quoted region.
    let dq_start = out.find('"').expect("opening quote present");
    let quoted_region = &out[dq_start..];
    assert!(
        !quoted_region.contains("  "),
        "formatter injected an extra space into the quoted string \
         (semantics change — string value altered).\noutput: {out:?}",
    );
}

// `for` special case (engine.rs 294-299): only the body (arg 4) expands;
// init/cond/next stay inline. Plus the `is_braced` and arity guards.

#[test]
fn fmt_for_expands_only_the_body() {
    // The formatter-specific `for` override: init `{set i 0}`, cond `{$i < 10}`
    // and next `{incr i}` remain inline; only the 4th braced arg (the body)
    // expands into a K&R block.
    // tclsh-proof (loop runs identically): both forms of
    //   `for {set i 0} {$i < 3} {incr i} {append out $i}` print `012` (8.6+9.0).
    assert_eq!(
        fmt("for {set i 0} {$i < 10} {incr i} {\nputs $i\n}\n"),
        "for {set i 0} {$i < 10} {incr i} {\n    puts $i\n}\n",
    );
}

#[test]
fn fmt_for_with_unbraced_body_is_left_inline() {
    // When the would-be body arg is NOT braced, the `is_braced` guard fails and
    // nothing expands — the command is emitted verbatim (modulo trailing NL).
    assert_eq!(
        fmt("for {set i 0} {$i<3} {incr i} foo\n"),
        "for {set i 0} {$i<3} {incr i} foo\n"
    );
}

#[test]
fn fmt_for_with_too_few_args_falls_through() {
    // `for {} {} {}` has only 3 post-name args (< 4), so the special case is
    // skipped and it goes through the generic registry path without panicking.
    assert_eq!(fmt("for {} {} {}\n"), "for {} {} {}\n");
}

// Body inline vs expand — `body_can_be_inline` / `append_body_no_space`
// (engine.rs 830-887) and the NEVER_INLINE_BODY trait.

#[test]
fn fmt_short_catch_body_stays_inline() {
    // `catch` is not NEVER_INLINE_BODY and the body is short and brace-free, so
    // it renders as `{ … }` on one line (the inline arm).
    // tclsh-proof: `catch {set x 1}` and `catch { set x 1 }` both leave x==1;
    //   `catch {set x 1}; puts $x` → `1` and `catch { set x 1 }; puts $x` → `1`.
    assert_eq!(fmt("catch {set x 1}\n"), "catch { set x 1 }\n");
}

#[test]
fn fmt_never_inline_body_dict_for_always_expands() {
    // `dict for` carries NEVER_INLINE_BODY, so even a tiny body expands.
    // tclsh-proof (iteration identical):
    //   `set d {a 1}; dict for {k v} $d {puts $k:$v}` → `a:1` (8.6 + 9.0)
    //   same with the body expanded over lines        → `a:1` (8.6 + 9.0)
    assert_eq!(
        fmt("dict for {k v} $d {puts $k}\n"),
        "dict for {k v} $d {\n    puts $k\n}\n",
    );
}

#[test]
fn fmt_body_too_long_to_inline_expands() {
    // A single-command body whose inline form would exceed goal_line_length is
    // forced to expand (the `inline_len <= goal_line_length` check fails).
    let long = "catch {set xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx 1}\n";
    let expected = "catch {\n    set xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx 1\n}\n";
    assert_eq!(fmt(long), expected);
}

#[test]
fn fmt_word_arg_collapsing_to_empty_is_dropped() {
    // A trailing `\<newline>` continuation that joins onto a blank line yields
    // an all-whitespace argument; `append_word_arg` returns `false` (the
    // collapse-to-empty path) and the arg is dropped, leaving `set x`.
    // tclsh-proof: `set x \<nl>` is an incomplete command on its own; here the
    //   formatter normalises the dangling continuation to nothing — both
    //   `set x` (after) and the original join produce the same parse: a `set`
    //   with a single name word and no value (an error if executed, identically
    //   so in 8.6 + 9.0).
    assert_eq!(fmt("set x \\\n\n"), "set x\n");
}

// Parameter lists & braced vars — `normalise_param_list` (engine.rs 134-168)
// and `enforce_braced_variables` reconstruction.

#[test]
fn fmt_param_list_with_nested_default_braces_normalised() {
    // Param list `{a {b 1} {c {x y}}}` — interior whitespace normalised to
    // single spaces while the nested `{…}` defaults are kept whole (the `{`
    // brace-matching arm of `normalise_param_list`).
    // tclsh-proof (defaults behave identically):
    //   `proc f {a {b 1} {c {x y}}} {return "$a-$b-$c"}; f Q` → `Q-1-x y` (8.6+9.0)
    //   same proc with the body expanded over lines           → `Q-1-x y`
    assert_eq!(
        fmt("proc f {a {b 1} {c {x y}}} {\nreturn\n}\n"),
        "proc f {a {b 1} {c {x y}}} {\n    return\n}\n",
    );
}

#[test]
fn fmt_enforce_braced_variables_rewrites_dollar_refs() {
    // With `enforce_braced_variables`, `$x` → `${x}` in word position.
    // tclsh-proof: `set x 7; puts $x` → `7` and `set x 7; puts ${x}` → `7`
    //   (8.6 + 9.0) — `${x}` is an exact synonym for `$x`.
    let cfg = FormatterConfig {
        enforce_braced_variables: true,
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("puts $x\n", &cfg), "puts ${x}\n");
}

// `{*}`-expansion identity skip (engine.rs 278-285).

#[test]
fn fmt_expand_command_word_is_not_misclassified() {
    // When the command WORD is `{*}$cmd`, the command's identity is dynamic, so
    // `identify_body_args` returns early and no body classification is applied;
    // the line is emitted verbatim.
    assert_eq!(fmt("{*}$cmd arg1 arg2\n"), "{*}$cmd arg1 arg2\n");
}

// Comment normalisation edges — `format_comment` (engine.rs 348-366).

#[test]
fn fmt_bare_hash_comment_preserved() {
    // A lone `#` (empty after the hash) returns `#` (the `comment_text == "#"`
    // / empty-`after_hashes` arms).
    assert_eq!(fmt("#\n"), "#\n");
}

#[test]
fn fmt_hashes_only_comment_preserved() {
    // `###` with nothing after the hashes returns the hashes verbatim (the
    // `after_hashes.is_empty()` → `"#".repeat(n)` arm).
    assert_eq!(fmt("###\n"), "###\n");
}

#[test]
fn fmt_comment_hash_spacing_toggle() {
    // `space_after_comment_hash == false` trims surrounding whitespace; the
    // default (true) keeps a single leading space and trims the trailing.
    let no_space = FormatterConfig {
        space_after_comment_hash: false,
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("#   hi   \n", &no_space), "#hi\n");
    assert_eq!(fmt("#   hi   \n"), "#   hi\n");
}

// Blank-line policy & trailing comments — `compute_blank_lines` (engine.rs
// 371-390) and `format_body`'s trailing-comment loop (1118-1124).

#[test]
fn fmt_blank_lines_between_proc_and_block() {
    // proc→non-proc and non-proc→proc both insert `blank_lines_between_blocks`
    // (default 1), exercising both halves of the `prev/current == "proc"` arm.
    assert_eq!(
        fmt("proc a {} {\nreturn\n}\nset x 1\n"),
        "proc a {} {\n    return\n}\n\nset x 1\n",
    );
    assert_eq!(
        fmt("set x 1\nproc a {} {\nreturn\n}\n"),
        "set x 1\n\nproc a {} {\n    return\n}\n",
    );
}

#[test]
fn fmt_max_consecutive_blank_lines_config() {
    // A run of blank lines is clamped to `max_consecutive_blank_lines`.
    // tclsh-proof (blank lines are inert): `set x 1 <blanks> set y 2;
    //   puts [expr {$x+$y}]` → `3` regardless of blank count (8.6 + 9.0).
    let one = FormatterConfig {
        max_consecutive_blank_lines: 1,
        ..FormatterConfig::default()
    };
    assert_eq!(
        fmt_with("set x 1\n\n\n\nset y 2\n", &one),
        "set x 1\n\nset y 2\n"
    );
}

#[test]
fn fmt_trailing_comments_after_blank_run() {
    // Trailing comments (no command after them) are emitted via the trailing
    // loop; a preceding blank run before them is clamped (default max 2 → here
    // the comments attach to nothing so the leading blanks are dropped).
    assert_eq!(
        fmt("set x 1\n\n\n\n# trailing comment\n# second\n"),
        "set x 1\n# trailing comment\n# second\n",
    );
}

// Recursive bodies — namespace / nested control flow (format_body recursion).

#[test]
fn fmt_namespace_eval_body_recurses() {
    // namespace eval body recurses; the inner proc body recurses again.
    // tclsh-proof: `namespace eval foo {proc bar {} {return 42}}; foo::bar`
    //   → `42` (8.6 + 9.0); same with both bodies expanded → `42`.
    assert_eq!(
        fmt("namespace eval foo {\nproc bar {} {\nreturn\n}\n}\n"),
        "namespace eval foo {\n    proc bar {} {\n        return\n    }\n}\n",
    );
}

#[test]
fn fmt_deeply_nested_bodies_indent_by_level() {
    // proc → foreach → if nests three levels; each level adds 4 spaces.
    // tclsh-proof: `proc f {} {foreach x {1 2} {if {$x==2} {return GOT2}}}; f`
    //   → `GOT2` (8.6 + 9.0); the expanded layout evaluates the same.
    assert_eq!(
        fmt("proc f {} {\nforeach x {1 2} {\nif {$x == 2} {\nreturn GOT2\n}\n}\n}\n"),
        "proc f {} {\n    foreach x {1 2} {\n        if {$x == 2} {\n            return GOT2\n        }\n    }\n}\n",
    );
}

// Whole-document config tails — `format_tcl` (engine.rs 1131-1149).

#[test]
fn fmt_ensure_final_newline_disabled() {
    // With `ensure_final_newline == false`, no trailing newline is appended.
    let cfg = FormatterConfig {
        ensure_final_newline: false,
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("set x 1", &cfg), "set x 1");
}

#[test]
fn fmt_trim_trailing_whitespace_disabled_keeps_clean_lines() {
    // With trimming off, the engine still produces clean lines (it never emits
    // trailing whitespace itself), so the dirty input's trailing run is dropped
    // during reconstruction rather than via the trim pass.
    let cfg = FormatterConfig {
        trim_trailing_whitespace: false,
        ..FormatterConfig::default()
    };
    assert_eq!(fmt_with("set x 1   \n", &cfg), "set x 1\n");
}

#[test]
fn fmt_crlf_line_ending_applied() {
    // A non-`\n` `line_ending` is substituted on every newline at the tail.
    let cfg = FormatterConfig {
        line_ending: "\r\n".to_owned(),
        ..FormatterConfig::default()
    };
    assert_eq!(
        fmt_with("proc f {} {\nset x 1\n}\n", &cfg),
        "proc f {} {\r\n    set x 1\r\n}\r\n",
    );
}

#[test]
fn fmt_empty_input_yields_single_newline() {
    // Empty source: `format_body` joins zero lines to `""`, then
    // `ensure_final_newline` appends one `\n`.
    assert_eq!(fmt(""), "\n");
}

#[test]
fn fmt_is_idempotent_on_complex_input() {
    // Idempotency: formatting the formatted output equals the formatted output
    // (the core stability property), across switch + nested bodies + comments.
    let src = "# header\nproc f {a {b 1}} {\nswitch $a {\nx {\nreturn 1\n}\ny -\nz {\nreturn 2\n}\n}\n}\nfor {set i 0} {$i < 3} {incr i} {\nputs $i\n}\n";
    let once = fmt(src);
    let twice = fmt(&once);
    assert_eq!(
        once, twice,
        "format is not idempotent:\nonce:  {once:?}\ntwice: {twice:?}"
    );
}

// config.rs — `make_indent` Tabs branch (config.rs 115).

#[test]
fn fmt_tab_indent_style() {
    // `IndentStyle::Tabs` indents each level with a single tab (the `Tabs` arm
    // of `make_indent`), independent of `indent_size`.
    let cfg = FormatterConfig {
        indent_style: IndentStyle::Tabs,
        ..FormatterConfig::default()
    };
    assert_eq!(
        fmt_with("proc f {} {\nif {1} {\nset x 1\n}\n}\n", &cfg),
        "proc f {} {\n\tif {1} {\n\t\tset x 1\n\t}\n}\n",
    );
}

// mod.rs — formatting / formatting_with edit construction (mod.rs 61-81),
// brace_delta, and range_formatting (88-208).

#[test]
fn formatting_returns_single_full_document_edit_when_dirty() {
    // `formatting` builds one whole-document `TextEdit` spanning to the
    // computed end position when the document is not already normalised.
    let edits = formatting("set x 1   \n", reg());
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.start_line, 0);
    assert_eq!(edits[0].range.start_character, 0);
    assert_eq!(edits[0].new_text, "set x 1\n");
}

#[test]
fn formatting_returns_no_edit_when_already_normalised() {
    // The no-op path: an already-canonical document yields an empty edit list.
    assert!(formatting("proc f {} {\n    set x 1\n}\n", reg()).is_empty());
}

#[test]
fn formatting_with_honours_explicit_config() {
    // `formatting_with` routes a caller config through to the engine and still
    // produces a whole-document edit.
    let cfg = FormatterConfig {
        indent_size: 2,
        ..FormatterConfig::default()
    };
    let edits = formatting_with("proc f {} {\nset x 1\n}\n", &cfg, reg());
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert!(
        edits[0].new_text.contains("\n  set x 1"),
        "expected 2-space indent; got {:?}",
        edits[0].new_text,
    );
}

#[test]
fn range_formatting_inside_proc_picks_up_prefix_brace_depth() {
    // Range-format only the body line of a proc; the prefix walk must compute
    // depth 1 (one open `{` above) so the line is indented 4 spaces. Exercises
    // the `brace_delta` accumulation + slice formatting.
    let edits = range_formatting(
        "proc foo {} {\nset x 1\n}\n",
        LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 100,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].new_text, "    set x 1\n");
    assert_eq!(edits[0].range.start_line, 1);
    assert_eq!(edits[0].range.end_line, 2);
}

#[test]
fn range_formatting_no_edit_when_slice_already_clean() {
    // A clean slice (already normalised, accounting for the replacement
    // range's trailing newline) yields no edit — the `formatted_slice ==
    // original_with_nl` short-circuit.
    let edits = range_formatting(
        "proc foo {} {\n    set x 1\n}\n",
        LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 100,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert!(edits.is_empty(), "{edits:?}");
}

#[test]
fn range_formatting_clamps_end_past_eof_to_last_line() {
    // An end_line past EOF is clamped; when the slice reaches the document's
    // last line the end anchor is derived via LineIndex (the EOF branch).
    let edits = range_formatting(
        "set x 1   \nset y 2   \n",
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 99,
            end_character: 0,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert!(edits[0].new_text.starts_with("set x 1\n"), "{edits:?}");
    assert!(!edits[0].new_text.contains("   "), "{edits:?}");
}

#[test]
fn range_formatting_last_line_without_trailing_newline_anchors_at_eof() {
    // The final line has no trailing `\n`; the EOF branch must still anchor the
    // edit at the post-final-char position and finalise with a newline.
    let edits = range_formatting(
        "set x 1\nset y 2   ",
        LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 100,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].new_text, "set y 2\n");
    assert_eq!(edits[0].range.start_line, 1);
}

#[test]
fn range_formatting_crlf_finalisation() {
    // `finalise_slice` applies a non-`\n` line_ending to the formatted slice.
    let cfg = FormatterConfig {
        line_ending: "\r\n".to_owned(),
        ..FormatterConfig::default()
    };
    let edits = range_formatting(
        "set x 1   \nset y 2\n",
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 5,
        },
        &cfg,
        reg(),
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].new_text, "set x 1\r\n");
}

#[test]
fn range_formatting_brace_delta_ignores_string_braces_in_prefix() {
    // The prefix line `set s "{"` has an unbalanced `{` *inside a string*;
    // `brace_delta` must treat it as depth-neutral, so the formatted body line
    // is at depth 0 (NOT indented). Drives the in-string arm of `brace_delta`.
    let edits = range_formatting(
        "set s \"{ unbalanced\"\nset y 2   \n",
        LspRange {
            start_line: 1,
            start_character: 0,
            end_line: 1,
            end_character: 100,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    // depth 0 → no leading indentation.
    assert_eq!(edits[0].new_text, "set y 2\n");
}

// Line endings: the `auto` default, and the whole-document / range edit
// ranges that must be measured on the CLIENT's EOL model.
//
// The two data-loss cases below are the ones a live-server audit reproduced
// against VS Code's own edit application (clamp each position to the client
// line model, then splice). Both came from `formatting_with` building its
// replace range with the `\n`-only `LineIndex::new`, which cannot see a lone
// `\r` as a line break the way the client does.

#[test]
fn fmt_auto_line_ending_follows_the_document() {
    // `auto` is the default: an LF file stays LF, a CRLF file stays CRLF, and
    // an old-Mac file stays CR. Before this, the formatter rewrote every CRLF
    // document to LF on the first Format Document — a silent whole-file
    // change no one asked for.
    assert_eq!(
        fmt("proc f {} {\nset x 1\n}\n"),
        "proc f {} {\n    set x 1\n}\n"
    );
    assert_eq!(
        fmt("proc f {} {\r\nset x 1\r\n}\r\n"),
        "proc f {} {\r\n    set x 1\r\n}\r\n",
    );
    assert_eq!(
        fmt("proc f {} {\rset x 1\r}\r"),
        "proc f {} {\r    set x 1\r}\r",
    );
    // A single line with no terminator has no evidence either way → LF.
    assert_eq!(fmt("set  x 1"), "set x 1\n");
}

#[test]
fn fmt_explicit_line_ending_still_overrides_auto() {
    let cfg = FormatterConfig {
        line_ending: "\n".to_owned(),
        ..FormatterConfig::default()
    };
    assert_eq!(
        fmt_with("set  x 1\r\nset  y 2\r\n", &cfg),
        "set x 1\nset y 2\n"
    );
}

#[test]
fn formatting_edit_range_spans_a_lone_cr_document() {
    // The client models `a\rb\r` as separate lines; the server must report the
    // end of its whole-document replace range on the SAME model. With the
    // `\n`-only index the end landed at 0:39 (one line, the whole file), so
    // VS Code replaced only line 0 and left the original lines 1..5 behind —
    // the file doubled.
    let src = "proc  p {a  b} {\rif {$a} {\rputs $b\r}\r}\r";
    let edits = formatting(src, reg());
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.start_line, 0);
    assert_eq!(edits[0].range.start_character, 0);
    // 6 client lines (5 terminators + the empty tail), so the range ends at
    // the start of line 5.
    assert_eq!(edits[0].range.end_line, 5);
    assert_eq!(edits[0].range.end_character, 0);
    assert_eq!(
        edits[0].new_text,
        "proc p {a b} {\r    if {$a} {\r        puts $b\r    }\r}\r",
    );
}

#[test]
fn formatting_edit_range_spans_a_mixed_line_ending_document() {
    // Mixed `\r\n` / `\n` / lone `\r`: the client counts 6 lines, the
    // `\n`-only model counted 5, so the edit stopped one line short and the
    // document's last `}` survived the replacement as a stray line.
    let src = "proc  p {a  b} {\r\nif {$a} {\nputs $b\r}\n}\n";
    let edits = formatting(src, reg());
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.end_line, 5);
    assert_eq!(edits[0].range.end_character, 0);
}

#[test]
fn range_formatting_cuts_the_slice_on_the_client_line_model() {
    // Line 2 of an old-Mac document is `puts $b`; on the `\n`-only model the
    // whole file was line 0 and any range request re-formatted everything.
    let src = "proc p {a b} {\rif {$a} {\rputs   $b\r}\r}\r";
    let edits = range_formatting(
        src,
        LspRange {
            start_line: 2,
            start_character: 0,
            end_line: 2,
            end_character: 100,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.start_line, 2);
    assert_eq!(edits[0].range.end_line, 3);
    assert_eq!(edits[0].range.end_character, 0);
    // Two braces deep, and re-emitted with the document's own `\r`.
    assert_eq!(edits[0].new_text, "        puts $b\r");
}

#[test]
fn range_formatting_reports_no_edit_for_an_already_formatted_crlf_slice() {
    // The no-op check compares against the RAW bytes of the same span, so an
    // already-normalised CRLF slice does not produce an edit that merely
    // rewrites its own terminator.
    let edits = range_formatting(
        "set x 1\r\nset y 2\r\n",
        LspRange {
            start_line: 0,
            start_character: 0,
            end_line: 0,
            end_character: 7,
        },
        &FormatterConfig::default(),
        reg(),
    );
    assert!(edits.is_empty(), "{edits:?}");
}
