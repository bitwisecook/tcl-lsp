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

//! The invocations recorded inside substitutions, expressions and body words.

use super::*;

// -- body-walk and nested-cmd recovery
//
// Verify ``when EVENT { body }`` recurses (the registry
// ``arg_role_resolver`` records BODY at the last index)
// and that braced expr args (``Str`` tokens) have their
// outer braces unwrapped before the nested-``[cmd]`` scan
// (otherwise the scanner skips the entire braced region
// opaquely).

#[test]
fn compound_cmd_word_descends_substitution_fragments_only() {
    // A command substitution that is the *first* fragment of a
    // compound word (`[foo]bar`, `[foo]$x`) merges with the trailing
    // literal into one argv token spanning the whole word.  The
    // nested-invocation recorder must descend only the `[…]`
    // fragment(s) — not the merged span — so it records the real
    // inner head (`foo`) rather than a bogus one (`foo]bar`).
    type Inv = (&'static str, u32, u32);
    let cases: &[(&str, &[Inv])] = &[
        ("puts [foo]bar\n", &[("foo", 6, 9)]),
        ("set x [foo]$suffix\n", &[("foo", 7, 10)]),
        // Substitution as the command head of a compound word.
        ("[foo]bar hi\n", &[("foo", 1, 4)]),
        // Two substitutions in one word — both must be found.
        ("puts [foo]bar[baz]\n", &[("foo", 6, 9), ("baz", 14, 17)]),
        // `;`-separated commands inside a compound substitution.
        ("puts [aa; bb]cc\n", &[("aa", 6, 8), ("bb", 10, 12)]),
    ];
    for (src, expected) in cases {
        let mut a = Analyser::new();
        let r = a.analyse(src, "f5-irules");
        let got: Vec<(String, u32, u32)> = r
            .command_invocations
            .iter()
            .map(|c| (c.name.clone(), c.range.start(), c.range.end()))
            .collect();
        for &(name, start, end) in *expected {
            assert!(
                got.iter()
                    .any(|(n, s, e)| n == name && *s == start && *e == end),
                "src {src:?}: expected ({name:?}, {start}, {end}) in {got:?}",
            );
        }
        // No over-read: a *descended* substitution head must never
        // carry the literal suffix after the matching `]` (`foo]bar`,
        // `foo]${suffix}`).  A whole-word head recorded verbatim by
        // `process_command` (`[foo]bar`) legitimately starts with `[`
        // and is excluded.
        assert!(
            !got.iter()
                .any(|(n, _, _)| n.contains(']') && !n.starts_with('[')),
            "src {src:?}: a descended head leaked the literal suffix (over-read): {got:?}",
        );
    }
}

#[test]
fn analyse_when_body_records_inner_command_invocations() {
    // ``when HTTP_REQUEST { body }`` — ``call`` and the
    // target ``myhelper`` should appear in
    // ``command_invocations`` from the body recursion.
    let mut a = Analyser::new();
    let r = a.analyse(
        "proc myhelper {} {}\nwhen HTTP_REQUEST { call myhelper }\n",
        "f5-irules",
    );
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"call"), "got {names:?}");
    assert!(names.contains(&"myhelper"), "got {names:?}");
}

#[test]
fn analyse_braced_expr_arg_records_inner_substitution() {
    // ``if { [HTTP::uri] eq "/foo" } { ... }`` — the
    // ``[HTTP::uri]`` substitution inside the braced expr
    // arg must surface in ``command_invocations``.  Without
    // the ``Str`` unwrap, the nested-cmd scanner sees the
    // outer ``{`` and skips the entire braced region
    // opaquely.
    let mut a = Analyser::new();
    let r = a.analyse("if { [HTTP::uri] eq \"/foo\" } { puts ok }\n", "f5-irules");
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"HTTP::uri"), "got {names:?}");
}

#[test]
fn analyse_records_every_command_in_a_substitution() {
    // A command substitution can hold more than one command (`;`- /
    // newline-separated), and substitutions nest.  The CST descent
    // records every inner command head (`set x [foo; bar]` ->
    // {foo, bar, set}).
    // A head is recorded in `word_piece` form for every command,
    // incl. a `$var` head (`${var}`), a `"quoted"` head (unquoted),
    // and a compound head (`set x [$cmd a; bar]` -> {${cmd}, bar,
    // set}).
    let cases: &[(&str, &[&str])] = &[
        ("set x [foo; bar]\n", &["set", "foo", "bar"]),
        ("puts [a; b; c]\n", &["puts", "a", "b", "c"]),
        ("set x [foo [bar; baz]]\n", &["set", "foo", "bar", "baz"]),
        ("set x [foo $y; bar]\n", &["set", "foo", "bar"]),
        ("set x [$cmd a; bar]\n", &["set", "${cmd}", "bar"]),
        ("set x [foo; $y arg]\n", &["set", "foo", "${y}"]),
        ("set x [\"q\" a; bar]\n", &["set", "q", "bar"]),
        // A control-flow command inside a substitution has body
        // arguments whose commands are inner invocations too
        // (descend_command resolves them via the registry).
        // `[if {$c} {puts hi}]` -> {if, puts}.
        ("set x [if {$c} {puts hi}]\n", &["set", "if", "puts"]),
        (
            "set x [foreach a $l {log $a}]\n",
            &["set", "foreach", "log"],
        ),
        ("set x [eval {one; two}]\n", &["set", "eval", "one", "two"]),
        // A command substitution inside an *expr* argument of a
        // command nested in a substitution is an invocation too
        // (collect_expr_substitutions). `[if {[check]} {fwd}]`
        // -> {if, check, fwd}; `[expr {[bar] + 1}]` -> {expr, bar}.
        (
            "set x [if {[check]} {fwd}]\n",
            &["set", "if", "check", "fwd"],
        ),
        ("set x [expr {[bar] + 1}]\n", &["set", "expr", "bar"]),
        (
            "set x [while {[cond]} {act}]\n",
            &["set", "while", "cond", "act"],
        ),
        // argv_texts[0] is recorded for every command, incl. a
        // `[subst]` head (recorded *and* descended) and a `{braced}`
        // head (its inner text).
        ("set x [[gen] arg]\n", &["set", "[gen]", "gen"]),
        ("puts [[a] [b]]\n", &["puts", "[a]", "a", "b"]),
    ];
    for (src, expected) in cases {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        for want in *expected {
            assert!(
                names.contains(want),
                "{src:?}: missing {want:?}, got {names:?}"
            );
        }
    }
}

#[test]
fn analyse_records_switch_arm_bodies_not_patterns() {
    // The `switch … {pat body …}` list-form arg is a Tcl *list*,
    // not a script.  The arm *bodies* are scripts (their commands
    // are invocations), but the *patterns* are not — descending
    // the whole list as a script would mis-record a pattern
    // (`a`/`b`) as a command head.  Parse the pairs and descend
    // each body.  A `default` keyword / `-` fall-through is a
    // pattern, not a body.  Result: {cmd1, cmd2, set, switch}.
    let mut a = Analyser::new();
    let r = a.analyse("set x [switch $v {a {cmd1} b {cmd2}}]\n", "tcl8.6");
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert!(names.contains(&"switch"), "got {names:?}");
    assert!(names.contains(&"cmd1"), "arm body missing: {names:?}");
    assert!(names.contains(&"cmd2"), "arm body missing: {names:?}");
    assert!(
        !names.contains(&"a"),
        "pattern recorded as command: {names:?}"
    );
    assert!(
        !names.contains(&"b"),
        "pattern recorded as command: {names:?}"
    );
}

#[test]
fn analyse_braced_data_word_is_not_over_recorded() {
    // A `[...]` inside a braced *data* word is literal (braces
    // suppress substitution), so it must not be recorded — only an
    // *expr* arg's substitutions are.  And a command whose name is
    // itself a substitution (`[x] hi`) must still be descended (the
    // head token is iterated too).
    let cases: &[(&str, &[&str], &[&str])] = &[
        // (source, must-contain, must-NOT-contain)
        ("set x {[noeval]}\n", &["set"], &["noeval"]),
        ("set d {literal data}\n", &["set"], &["data", "literal"]),
        ("proc p {} {[x] hi}\n", &["proc", "x"], &[]),
        ("if {[chk]} {puts ok}\n", &["if", "chk", "puts"], &[]),
    ];
    for (src, want, unwant) in cases {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl8.6");
        let names: Vec<&str> = r
            .command_invocations
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        for w in *want {
            assert!(names.contains(w), "{src:?}: missing {w:?}, got {names:?}");
        }
        for u in *unwant {
            assert!(
                !names.contains(u),
                "{src:?}: over-recorded {u:?}, got {names:?}"
            );
        }
    }
}
