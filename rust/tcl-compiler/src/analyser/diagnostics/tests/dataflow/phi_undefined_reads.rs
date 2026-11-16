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

//! W210 where a merge may leave a name undefined: `return` reads, loop accumulators, `dict with` bodies and matchers that cannot match.

use super::*;

#[test]
fn w210_phi_undef_if_arm_only_def_return() {
    // `v` is defined only when `$x > 0`; the unconditional `return $v`
    // reads it on the no-set path too.
    let got = w210_codes_in_dialect(
        "proc f {x} { if {$x > 0} { set v 1 }\n return $v }",
        "tcl8.6",
    );
    assert!(
        got.iter().any(|m| m.contains("'v'")),
        "phi-from-undef merge read must fire W210; got {got:?}"
    );
}

#[test]
fn w210_phi_undef_switch_no_default_return() {
    let got = w210_codes("proc f {x} { switch $x { a { set v 1 } b { set v 2 } }\n return $v }");
    assert!(
        got.iter().any(|m| m.contains("'v'")),
        "switch-no-default + return must fire W210; got {got:?}"
    );
}

#[test]
fn w210_interproc_dict_with_caller_literal() {
    // A caller passing a literal dict propagates to the callee's
    // `dict with $param` key check (interproc constant propagation).
    // Key present → silent.
    assert_eq!(
        w210_codes("proc f {d} { dict with d { return $missing } }\nf {missing ok}\n").len(),
        0
    );
    // Empty dict → no keys → the read fires.
    assert!(
        w210_codes("proc f {d} { dict with d { return $missing } }\nf {}\n")
            .iter()
            .any(|m| m.contains("'missing'"))
    );
    // Mixed callers → unknown shape → conservatively silent.
    assert_eq!(
        w210_codes("proc f {d} { dict with d { return $missing } }\nf {}\nf {missing X}\n").len(),
        0
    );
}

#[test]
fn w210_provably_no_match_regexp_scan() {
    // Provably-no-match output reads fire.
    assert!(
        w210_codes_in_dialect("proc f {} { scan abc %d n\n puts $n }", "tcl8.6")
            .iter()
            .any(|m| m.contains("'n'"))
    );
    assert!(
        w210_codes_in_dialect("proc f {} { regexp {x} y -> v\n puts $v }", "tcl8.6")
            .iter()
            .any(|m| m.contains("'v'"))
    );
    // Embedded in a negated condition fires on the no-match arm.
    assert!(
        w210_codes_in_dialect(
            "proc f {} { if {![regexp {x} y -> v]} { puts $v } }",
            "tcl8.6"
        )
        .iter()
        .any(|m| m.contains("'v'"))
    );
}

#[test]
fn w210_regexp_expanded_whitespace_pattern_silent() {
    // `-expanded` ignores whitespace, so `{a b}` matches `ab` and writes
    // v — the no-match proof must bail (no false W210).
    assert_eq!(
        w210_codes_in_dialect(
            "proc f {} { regexp -expanded {a b} ab v\n puts $v }",
            "tcl8.6"
        ),
        [] as [std::string::String; 0]
    );
    // A whitespace-free literal under -expanded is still safe → fires.
    assert!(
        w210_codes_in_dialect("proc f {} { regexp -expanded {x} X v\n puts $v }", "tcl8.6")
            .iter()
            .any(|m| m.contains("'v'"))
    );
}

#[test]
fn w210_matchable_regexp_scan_silent() {
    // A matchable / nocase-matchable regexp output is set — no W210.
    assert_eq!(
        w210_codes("proc f {} { regexp -nocase {x} X v\n puts $v }"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        w210_codes("proc f {} { scan 42 %d n\n puts $n }"),
        [] as [std::string::String; 0]
    );
    // The success arm of a positive condition reads a set var.
    assert_eq!(
        w210_codes("proc f {} { if {[regexp {x} y -> v]} { puts $v } }"),
        [] as [std::string::String; 0]
    );
    // An unknown / unsafe switch can't prove no-match → silent.
    assert_eq!(
        w210_codes("proc f {} { regexp -bogus {x} y v\n puts $v }"),
        [] as [std::string::String; 0]
    );
    // `-about` is not such a switch: it names no match variable at all, so
    // `v` is never written and the read is genuine. tclsh 8.4.20 / 8.6.18 /
    // 9.0.4 all fail this program with `can't read "v"` (#2135).
    assert!(
        w210_codes("proc f {} { regexp -about {x} y v\n puts $v }")
            .iter()
            .any(|m| m.contains("'v'"))
    );
}

/// W210 reads the preserve outcome: a match variable a `regexp` leaves
/// untouched holds its prior
/// version (`SccpResult::preserved`), so a read of one no earlier statement
/// set is a read before set, reported at the read — a `return` and a
/// condition's no-match arm included — and a read of one an earlier
/// statement set is not. tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0:
/// `regexp {(x)(y)} zz a b; puts $a` fails with `can't read "a": no such
/// variable` (and `return $b` with `can't read "b"`), the matching subject
/// prints `xy`, and `set a before` first prints `before`, where the private
/// prover reported that read too.
#[test]
fn w210_reads_a_no_match_preserve_outcome() {
    let reads = |src: &str| -> Vec<String> {
        Analyser::new()
            .analyse(src, "tcl8.6")
            .diagnostics
            .iter()
            .filter(|d| d.code == DiagCode::W210)
            .map(|d| src[d.span.as_range()].to_owned())
            .collect()
    };
    assert_eq!(
        reads("proc f {} {\n    regexp {(x)(y)} zz a b\n    puts $a\n}\n"),
        ["$a"]
    );
    // The return pass and the def-use pass each report a `return` read of
    // an undefined version, as they do after an `unset`.
    let returned = reads("proc g {} {\n    regexp {(x)(y)} zz a b\n    return $b\n}\n");
    assert!(
        !returned.is_empty() && returned.iter().all(|read| read.contains("$b")),
        "{returned:?}"
    );
    assert_eq!(
        reads("proc n {} {\n    if {![regexp {x} y -> v]} { puts $v }\n}\n"),
        ["$v"]
    );
    for silent in [
        "proc h {} {\n    regexp {(x)(y)} xy a b\n    puts $a\n}\n",
        "proc k {} {\n    set a before\n    regexp {(x)(y)} zz a b\n    puts $a\n}\n",
        "proc m {} {\n    set v before\n    if {![regexp {x} y -> v]} { puts $v }\n}\n",
    ] {
        assert_eq!(reads(silent), Vec::<String>::new(), "{silent}");
    }
}

#[test]
fn w210_incr_on_uninit_is_silent() {
    // `incr z` initialises z to 0 (Tcl 8.5+) — not read-before-set.
    assert_eq!(
        w210_codes_in_dialect("proc f {} { incr z\n return $z }", "tcl8.6"),
        [] as [std::string::String; 0]
    );
    // A genuine bare read of an unset local still fires.
    assert!(
        w210_codes_in_dialect("proc f {} { puts $z }", "tcl8.6")
            .iter()
            .any(|m| m.contains("'z'"))
    );
}

#[test]
fn w210_phi_undef_use_after_unset_return() {
    let got = w210_codes_in_dialect("proc f {} { set v 1\n unset v\n return $v }", "tcl8.6");
    assert!(
        got.iter().any(|m| m.contains("'v'")),
        "use-after-unset return must fire W210; got {got:?}"
    );
}

/// A name defined only in a loop body is read after the loop. A loop that may
/// run is taken to run, so only a collection that is provably empty warns.
/// tclsh 8.4.20, 8.6.18, 9.0.4 and 9.1.0: over an unknown list the read raises
/// `can't read "r": no such variable` for `{}` and returns `a b` for `{a b}`;
/// over the literal `{a b}` it returns `a b`; over the literal `{}` it raises
/// on every run.
#[test]
fn w210_after_a_loop_follows_what_the_collection_proves() {
    for (header, collection, fires) in [
        ("proc f {items}", "$items", false),
        ("proc f {}", "{a b}", false),
        ("proc f {}", "{}", true),
    ] {
        let src = format!("{header} {{ foreach i {collection} {{ lappend r $i }}\n return $r }}");
        for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
            let got = w210_codes_in_dialect(&src, dialect);
            assert_eq!(
                got.iter().any(|message| message.contains("'r'")),
                fires,
                "{dialect}: {collection}: {got:?}"
            );
        }
    }
    assert!(
        w210_codes_in_dialect(
            "proc f {items} { set r {}; foreach i $items { lappend r $i }; return $r }",
            "tcl8.6"
        )
        .is_empty(),
        "an initial value makes every path safe"
    );
}

/// A first-iteration read of the accumulator, before its set, is a read
/// before set inside the body: tclsh 8.6.18 raises `can't read "r": no such
/// variable` for every non-empty list.
#[test]
fn w210_first_iteration_read_in_a_loop_body_fires() {
    let inbody = w210_codes_in_dialect(
        "proc f {items} { foreach i $items { puts $r; lappend r $i } }",
        "tcl8.6",
    );
    assert!(
        inbody.iter().any(|m| m.contains("'r'")),
        "first-iteration in-body read before the set must still fire; got {inbody:?}"
    );
}

#[test]
fn w210_c9_loop_accumulator_distinguishes_custom_inputs_from_ordinary_empty_lists() {
    // A custom C9 list can write caller locals from its Length/Elements/Dup
    // callbacks even when no body runs. Unknown formal values cannot prove
    // the accumulator absent after that reached callback opportunity.
    let unknown = w210_codes_in_dialect(
        "proc f {items} { foreach i $items { lappend r $i }; return $r }",
        "tcl9.0",
    );
    assert!(
        unknown.iter().all(|message| !message.contains("'r'")),
        "a custom input retains a callback residual; got {unknown:?}"
    );
    // A literal empty ordinary list has no such callback opportunity, so its
    // zero-iteration path must retain the actual missing accumulator.
    let empty = w210_codes_in_dialect(
        "proc f {} { foreach i {} { lappend r $i }; return $r }",
        "tcl9.0",
    );
    assert!(
        empty.iter().any(|message| message.contains("'r'")),
        "an ordinary empty list leaves the accumulator undefined; got {empty:?}"
    );
}

#[test]
fn w210_no_fire_when_both_merge_arms_define() {
    // Control: every merge predecessor defines `v` — not read-before-set.
    let got = w210_codes("proc f {x} { if {$x > 0} { set v 1 } else { set v 2 }\n return $v }");
    assert_eq!(
        got.len(),
        0,
        "both-arms-defined merge must be silent; got {got:?}"
    );
}

#[test]
fn w210_empty_dict_with_return_fires_but_known_key_silent() {
    // FP-DS-08: empty dict unpacks nothing — `return $missing` fires.
    let empty = w210_codes_in_dialect(
        "proc f {} { set d {}\n dict with d {}\n return $missing }",
        "tcl8.6",
    );
    assert!(
        empty.iter().any(|m| m.contains("'missing'")),
        "empty dict-with return must fire W210; got {empty:?}"
    );
    // Known-key dict unpacks `missing` — silent.
    let known = w210_codes_in_dialect(
        "proc f {} { set d {missing ok}\n dict with d {}\n return $missing }",
        "tcl8.6",
    );
    assert_eq!(
        known.len(),
        0,
        "known-key dict-with return must be silent; got {known:?}"
    );
    // Unknown-shape dict (param) — conservatively silent.
    let unknown =
        w210_codes_in_dialect("proc f {d} { dict with d {}\n return $missing }", "tcl8.6");
    assert_eq!(
        unknown.len(),
        0,
        "unknown dict-with return must be silent; got {unknown:?}"
    );
}

/// `dict with` binds the keys of the value that reaches it, after same-block
/// keyed updates. tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0 raise `can't read
/// "a": no such variable` for the `dict unset` shapes, at top level and in a
/// proc, and print `1` when `dict set d b 2` leaves `a` bound.
#[test]
fn w210_dict_with_reads_the_value_after_keyed_updates() {
    let unbound = [
        "set d {a 1}\ndict unset d a\ndict with d {puts $a}\n",
        "proc p {} { set d {a 1}\n dict unset d a\n dict with d {puts $a} }\n",
        "set d {a 1}\ndict unset d a\ndict set d b 2\ndict with d {puts $a}\n",
    ];
    let bound = [
        "set d {a 1}\ndict set d b 2\ndict with d {puts $a}\n",
        "proc p {} { set d {a 1}\n dict set d b 2\n dict with d {puts $a} }\n",
        "set d {a 1}\ndict set d b 2\ndict unset d b\ndict with d {puts $a}\n",
    ];
    for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for src in unbound {
            let got = w210_codes_in_dialect(src, dialect);
            assert!(
                got.iter().any(|m| m.contains("'a'")),
                "{dialect}: `a` was removed before `dict with`: {src:?}; got {got:?}"
            );
        }
        for src in bound {
            let got = w210_codes_in_dialect(src, dialect);
            assert!(
                got.is_empty(),
                "{dialect}: `a` is still bound: {src:?}; got {got:?}"
            );
        }
    }
}

/// The `dict with` / `dict update` key harvest reads the registry's body
/// plan. A key path descends before the keys bind: `dict with d a
/// {}` over `{a {x 1}}` binds `x`, not `a` (tclsh 8.5 to 9.1 return `1`,
/// and raise `can't read "y"` for a key the path does not hold); a key path
/// over a dictionary the analysis does not know leaves the shape unknown;
/// the qualified spelling is the same plan; and a `dict update` variable is
/// bound only when the dictionary holds its key (`can't read "v"`
/// otherwise).
#[test]
fn w210_dict_body_keys_come_from_the_plan() {
    let path = w210_codes_in_dialect(
        "proc f {} { set d {a {x 1}}\n dict with d a {}\n return $x }",
        "tcl8.6",
    );
    assert!(path.is_empty(), "a key path's keys bind; got {path:?}");
    let beside = w210_codes_in_dialect(
        "proc f {} { set d {a {x 1}}\n dict with d a {}\n return $y }",
        "tcl8.6",
    );
    assert!(
        beside.iter().any(|m| m.contains("'y'")),
        "a key the path does not hold is unbound; got {beside:?}"
    );
    let unknown = w210_codes_in_dialect("proc f {d} { dict with d a {}\n return $x }", "tcl8.6");
    assert!(
        unknown.is_empty(),
        "an unknown dictionary's key path is unknown shape; got {unknown:?}"
    );
    let qualified = w210_codes_in_dialect(
        "proc f {} { set d {a 1}\n ::tcl::dict::with d {}\n return $a }",
        "tcl8.6",
    );
    assert!(
        qualified.is_empty(),
        "the qualified spelling binds; got {qualified:?}"
    );
    let qualified_missing = w210_codes_in_dialect(
        "proc f {} { set d {a 1}\n ::tcl::dict::with d {}\n return $b }",
        "tcl8.6",
    );
    assert!(
        qualified_missing.iter().any(|m| m.contains("'b'")),
        "the qualified spelling binds only its keys; got {qualified_missing:?}"
    );
    let present = w210_codes_in_dialect(
        "proc f {} { set d {k 1}\n dict update d k v {}\n return $v }",
        "tcl8.6",
    );
    assert!(present.is_empty(), "a present key binds; got {present:?}");
    let absent = w210_codes_in_dialect(
        "proc f {} { set d {j 1}\n dict update d k v {}\n return $v }",
        "tcl8.6",
    );
    assert!(
        absent.iter().any(|m| m.contains("'v'")),
        "an absent key leaves its variable unbound; got {absent:?}"
    );
}

#[test]
fn w210_qualified_variable_alias_tail_return_silent() {
    // FP-RBS-04: `variable ${name}::graphAttr` declares the local alias
    // `graphAttr`; the bare tail read is not read-before-set.
    let got =
        w210_codes("proc ::ns::get {name key} { variable ${name}::graphAttr\n return $graphAttr }");
    assert_eq!(
        got.len(),
        0,
        "qualified variable-alias tail read must be silent; got {got:?}"
    );
}

#[test]
fn w210_no_false_fire_on_many_var_scan_return() {
    // The dynamic scan arg-role resolver marks every trailing
    // varName as a write, so `return $a19` is not read-before-set.
    let src = "proc f {} { scan {0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19} \
{%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} \
a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 a15 a16 a17 a18 a19\n return $a19 }";
    assert_eq!(
        w210_codes(src).len(),
        0,
        "20-var scan must not false-fire W210 on the tail var"
    );
}

#[test]
fn w210_no_false_fire_on_many_var_lassign_return() {
    let src = "proc f {l} { lassign $l a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11 a12 a13 a14 \
a15 a16 a17 a18 a19 a20\n return $a20 }";
    assert_eq!(
        w210_codes(src).len(),
        0,
        "21-var lassign must not false-fire W210 on the tail var"
    );
}
