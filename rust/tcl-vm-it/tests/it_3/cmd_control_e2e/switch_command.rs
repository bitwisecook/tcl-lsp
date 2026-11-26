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

//! `switch` on its runtime path (`cmd_switch.rs`): matching modes, fallthrough, match variables and error paths.

use super::*;

// switch  (cmd_switch.rs, runtime path via -glob/-regexp/-nocase/dynamic)

/// `-glob` matching with a `default` fall-through clause.
#[test]
fn switch_glob_with_default() {
    // tclsh: `glob-a`
    assert_eq!(
        run("switch -glob abc {a* {set r glob-a} b* {set r glob-b} default {set r none}}").1,
        "glob-a",
    );
    // no glob matches -> default runs.
    // tclsh: `none`
    assert_eq!(
        run("switch -glob zzz {a* {set r glob-a} b* {set r glob-b} default {set r none}}").1,
        "none",
    );
}

/// `-regexp` matching, including the `-` fall-through across pattern/body pairs
/// and a trailing `default`.
#[test]
fn switch_regexp_fallthrough_and_default() {
    // The first pattern's `-` body falls through to the next pattern's body.
    // tclsh: `matched`
    assert_eq!(
        run("switch -regexp -- abc123 {[a-z]+ - [0-9]+ {set r matched} default {set r none}}").1,
        "matched",
    );
}

/// `-nocase` exact matching, and combined with `-glob`.
#[test]
fn switch_nocase() {
    // tclsh: `yes`
    assert_eq!(
        run("switch -nocase ABC {abc {set r yes} default {set r no}}").1,
        "yes",
    );
    // -nocase combined with -glob.
    // tclsh: `hit`
    assert_eq!(
        run("switch -nocase -glob ABC {a*c {set r hit} default {set r no}}").1,
        "hit",
    );
}

/// `--` ends option processing so a value starting with `-` is treated as the
/// string, not an option.
#[test]
fn switch_double_dash() {
    // tclsh: `dashx`
    assert_eq!(
        run("switch -- -x {-x {set r dashx} default {set r d}}").1,
        "dashx",
    );
}

/// `-matchvar` captures the regexp match + submatches; `-indexvar` captures the
/// match index pairs (TIP #75). Both are written before the body runs.
#[test]
fn switch_matchvar_indexvar() {
    // -matchvar holds {whole sub1 sub2 ...}.
    // tclsh: `abcXYZ abc XYZ`
    assert_eq!(
        run("switch -regexp -matchvar mv -- abcXYZ {([a-z]+)([A-Z]+) {set mv}}").1,
        "abcXYZ abc XYZ",
    );
    // The matchvar is readable inside the body for indexing a submatch.
    // tclsh: `123`
    assert_eq!(
        run("switch -regexp -matchvar m -- foo123bar {([0-9]+) {lindex $m 1}}").1,
        "123",
    );
    // -indexvar holds the {start end} index pair of the whole match.
    // tclsh: `{2 3}`
    assert_eq!(
        run("switch -regexp -indexvar iv -- abcde {cd {set iv}}").1,
        "{2 3}",
    );
}

/// A multi-step `-` fall-through chain resolves to the next real body.
#[test]
fn switch_multi_dash_fallthrough() {
    // tclsh: `abc`
    assert_eq!(run("switch -glob a {a - b - c {set r abc}}").1, "abc");
}

/// The inline (non-brace-list) pattern/body form: pattern/body words given as
/// separate args rather than a single braced list.
#[test]
fn switch_inline_pairs_form() {
    // tclsh: `ia`
    assert_eq!(
        run("set s switch; $s -glob abc a* {set r ia} b* {set r ib}").1,
        "ia",
    );
}

/// No pattern matches and there is no `default`: `switch` returns empty.
#[test]
fn switch_no_match_returns_empty() {
    // tclsh: `<>`
    assert_eq!(
        run("set r [switch -glob nomatch {a* 1 b* 2}]; subst <$r>").1,
        "<>",
    );
}

/// `switch` options abbreviate like tclsh (`Tcl_GetIndexFromObj`, flags 0):
/// `-gl`/`-e`/`-noc` resolve to their options in both the direct form (the
/// compiler bails to the runtime for a non-exact option word) and the
/// runtime-dispatch form. Probed tclsh 8.6.14.
#[test]
fn switch_option_abbreviation() {
    // tclsh: `switch -gl -- abc {a* {concat ia}}` → `ia`.
    assert_eq!(run("switch -gl -- abc {a* {concat ia}}").1, "ia");
    // tclsh: `-e` → -exact, `-noc` → -nocase.
    assert_eq!(
        run("set s switch; $s -e -- b {a {concat A} b {concat B}}").1,
        "B",
    );
    assert_eq!(
        run("set s switch; $s -noc -- ABC {abc {concat nc}}").1,
        "nc"
    );
}

/// `switch` error paths: bad option, empty body list, odd pattern/body count
/// (both brace-list and inline), a trailing `-` with nothing to fall to, and the
/// misplaced-comment heuristic.
#[test]
fn switch_error_paths() {
    // bad option
    // tclsh: `bad option "-badopt": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --`
    assert_eq!(
        run("set s switch; catch {$s -badopt x {a 1}} m; set m").1,
        "bad option \"-badopt\": must be -exact, -glob, -indexvar, -matchvar, \
         -nocase, -regexp, or --",
    );
    // ambiguous option: a lone `-` prefixes every table entry.
    // tclsh: `ambiguous option "-": must be -exact, -glob, -indexvar, -matchvar, -nocase, -regexp, or --`
    assert_eq!(
        run("set s switch; catch {$s - x {a 1}} m; set m").1,
        "ambiguous option \"-\": must be -exact, -glob, -indexvar, -matchvar, \
         -nocase, -regexp, or --",
    );
    // empty body list
    // tclsh: `wrong # args: should be "switch ?-option ...? string {?pattern body ...? ?default body?}"`
    assert_eq!(
        run("set s switch; catch {$s -glob x {}} m; set m").1,
        "wrong # args: should be \"switch ?-option ...? string \
         {?pattern body ...? ?default body?}\"",
    );
    // odd pattern/body count in the brace-list form
    // tclsh: `extra switch pattern with no body`
    assert_eq!(
        run("set s switch; catch {$s -glob x {a 1 b}} m; set m").1,
        "extra switch pattern with no body",
    );
    // odd pattern/body count in the inline form
    assert_eq!(
        run("set s switch; catch {$s -glob x a 1 b} m; set m").1,
        "extra switch pattern with no body",
    );
    // trailing `-` body with nothing to fall through to
    // tclsh: `no body specified for pattern "c"`
    assert_eq!(
        run("set s switch; catch {$s -glob x {a 1 b 2 c -}} m; set m").1,
        "no body specified for pattern \"c\"",
    );
    // misplaced-comment heuristic: odd count whose first pattern begins with `#`.
    // tclsh: `extra switch pattern with no body, this may be due to a comment ...`
    assert_eq!(
        run("set s switch; catch {$s -glob x {# 1 2}} m; set m").1,
        "extra switch pattern with no body, this may be due to a comment \
         incorrectly placed outside of a switch body - see the \"switch\" \
         documentation",
    );
}

/// A `switch` body that is `return`/`break` propagates that code.
#[test]
fn switch_body_control_propagates() {
    // a return inside a switch body inside a proc returns from the proc.
    // tclsh: `fromswitch`
    assert_eq!(
        run("proc p {} { switch -glob abc {a* { return fromswitch }} }; p").1,
        "fromswitch",
    );
    // a break inside a switch body inside a loop breaks the loop.
    // tclsh: `1`
    assert_eq!(
        run(concat!(
            "set c foreach; set n 0\n",
            "$c x {1 2 3} { switch -glob $x {1 {incr n} * {break}} }\n",
            "set n"
        ))
        .1,
        "1",
    );
}

/// A malformed brace-list body errors (the `rest[0].as_list()` Err arm in
/// `cmd_switch`).
#[test]
fn switch_runtime_bad_body_list() {
    // An unbalanced brace in the body list.
    // tclsh: `unmatched open brace in list`
    let (ok, msg, _) = run("set s switch; $s -glob x \"a 1 \\{\"");
    assert!(!ok);
    assert_eq!(msg, "unmatched open brace in list");
}

/// A `-regexp` switch whose pattern does not compile errors (the
/// `core_switch::select` Err arm in `cmd_switch`). The VM targets the Tcl 9.0
/// wording (8.6 phrases it differently).
#[test]
fn switch_runtime_bad_regexp() {
    // tclsh 9.0: `cannot compile regular expression pattern: invalid quantifier operand`
    // (tclsh 8.6: `couldn't compile regular expression pattern: quantifier operand invalid`)
    let (ok, msg, _) = run("set s switch; $s -regexp -- abc {* {set z 1}}");
    assert!(!ok);
    assert_eq!(
        msg,
        "cannot compile regular expression pattern: invalid quantifier operand",
    );
}

/// A `-matchvar`/`-indexvar` write that fails propagates the set error (the
/// `set_var` Err arm in `cmd_switch`). Writing into a constant (Tcl 9) fails.
#[test]
fn switch_runtime_matchvar_write_failure() {
    // tclsh 9.0: `can't set "c": variable is a constant`
    let (ok, msg, _) =
        run("const c 1; set s switch; $s -regexp -matchvar c -- abc {a.. {set z 1}}");
    assert!(!ok);
    assert_eq!(msg, "can't set \"c\": variable is a constant");
}

/// A hard *parse* error in the chosen `switch` body is mapped to an error (the
/// `eval_source` Err arm in `cmd_switch`).
#[test]
fn switch_runtime_body_parse_error() {
    // tclsh: `extra characters after close-quote`
    let (ok, msg, _) = run("set s switch; $s -glob abc {a* {set \"q\"zzz 1}}");
    assert!(!ok);
    assert_eq!(msg, "extra characters after close-quote");
}
