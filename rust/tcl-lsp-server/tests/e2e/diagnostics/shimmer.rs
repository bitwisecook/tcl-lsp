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

//! Shimmer and shared-value diagnostics (S100, S101, S103): their ranges, the bodies they reach and the `# noqa` that silences them.

use super::*;

#[test]
fn shimmer_committed_value_with_lindex_is_s100_info_with_tight_range() {
    // A *committed* Dict (from `[dict create]`) read as a list genuinely
    // shimmers — a pure string literal would promote for free. The range
    // must cover only the `$x` argument, not the whole `lindex $x 0` call.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [dict create a 1 b 2]\nlindex $x 0\n");
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert_eq!(s100.len(), 1, "expected exactly one S100: {diags:?}");
    let range = &s100[0]["range"];
    assert_eq!(range["start"]["line"], 1);
    assert_eq!(range["start"]["character"], 7, "must start at '$x'");
    assert_eq!(range["end"]["line"], 1);
    assert_eq!(range["end"]["character"], 9, "must end after '$x'");
    assert_eq!(
        s100[0].get("severity").and_then(Value::as_i64),
        Some(3), // Information
        "S100 is informational: {:?}",
        s100[0]
    );
}

#[test]
fn clean_list_used_with_lindex_has_no_s100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [list 1 2 3]\nlindex $x 0\n");
    assert!(!has_code(&diags, "S100"), "unexpected S100: {diags:?}");
}

#[test]
fn issue_940_braced_list_literal_in_foreach_has_no_s100() {
    // A braced list literal is a pure string that `foreach` parses into a
    // list once, for free — no shimmer. Includes the `{}` empty-list case.
    let mut lsp = Lsp::tcl();
    for src in [
        "set fontSizes {10.0 12.0 16.0 24.0}\nforeach size $fontSizes { puts $size }\n",
        "set empty {}\nforeach x $empty { puts $x }\n",
        "set a 1\nforeach b $a { puts $b }\n",
        "set d {a 1 b 2}\ndict for {k v} $d { puts \"$k=$v\" }\n",
    ] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(
            !has_code(&diags, "S100") && !has_code(&diags, "S101"),
            "issue #940: pure list literal must not shimmer for {src:?}: {diags:?}"
        );
    }
}

#[test]
fn issue_940_committed_container_in_foreach_still_fires_s100() {
    // TP control: the check must not blanket-silence genuine shimmers — a
    // committed dict read as a list still fires.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set d [dict create a 1 b 2]\nforeach x $d { puts $x }\n",
    );
    assert!(
        has_code(&diags, "S100"),
        "committed dict in foreach must still shimmer: {diags:?}"
    );
}

#[test]
fn element_tracking_committed_lindex_retrieval_is_silent() {
    // container elements are shared objects, so a
    // committed numeric element retrieved by `lindex` is genuinely numeric —
    // arithmetic on it is not a shimmer.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set l [list [expr {2**20}] other]\nset first [lindex $l 0]\nset n [expr {$first + 1}]\nputs $n\n",
    );
    assert!(
        !has_code(&diags, "S100") && !has_code(&diags, "S101"),
        "a committed numeric element is numeric — no shimmer: {diags:?}"
    );
}

#[test]
fn union_lattice_three_way_merge_reports_every_type() {
    // a three-way differently-typed merge stays a tracked union rather
    // than collapsing to a silent OVERDEFINED; the phi message names every
    // member.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc f {c} {\n  if {$c == 1} { set x [list 1] } elseif {$c == 2} { set x [dict create a 1] } else { set x [expr {1+1}] }\n  return $x\n}\n",
    );
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert!(
        s100.iter().any(|d| {
            let m = d["message"].as_str().unwrap_or("");
            m.contains("merges") && m.contains("list") && m.contains("dict")
        }),
        "the 3-way merge message must name the merging types: {s100:?}"
    );
}

#[test]
fn bignum_values_raise_no_false_diagnostics() {
    // beyond-wide integers classify on the tower (`Bignum`, coarse Int)
    // and fold exactly — a bignum literal chain must publish no shimmer /
    // type diagnostics. (The exact fold value is pinned at the compiler
    // tier: `fp_sh_23_bignum_folds_are_exact` and the tcl_expr_eval oracle
    // corpus.)
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set big [expr {2**64}]\nset next [expr {$big + 1}]\nincr next\nputs $next\n",
    );
    assert!(
        !has_code(&diags, "S100") && !has_code(&diags, "S101"),
        "exact bignum arithmetic is not a shimmer: {diags:?}"
    );
}

#[test]
fn commit_dataflow_second_conversion_fires_s100_with_committed_from_type() {
    // First-use commit: `expr` commits the numeric intrep on a pure literal;
    // the later `lindex` genuinely re-represents it (oracle: rep int → list).
    // The message names the *committed* from-type, not the def-time shape.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set v 5\nexpr {$v + 1}\nlindex $v 0\n");
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert!(
        s100.iter().any(|d| {
            d["message"]
                .as_str()
                .is_some_and(|m| m.contains("numeric intrep") && m.contains("expects list"))
        }),
        "expected a numeric→list second-conversion S100: {diags:?}"
    );
}

#[test]
fn commit_dataflow_loop_rethunk_fires_s101_each_target() {
    // A pure literal read as two distinct intreps inside a loop re-thunks
    // per iteration (oracle: list ↔ dict every pass) — both reads are S101.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set l {a 1 b 2}\nforeach i {1 2 3} {\n    llength $l\n    dict size $l\n}\n",
    );
    let s101: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S101"))
        .collect();
    assert!(
        s101.len() >= 2,
        "expected two per-iteration re-thunk S101s: {diags:?}"
    );
}

#[test]
fn commit_dataflow_path_dependent_merge_message() {
    // Two branch arms commit two different intreps; a post-merge use matching
    // neither pays on both paths — the message names the possibilities.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc f {c} {\n  set a {1 2}\n  if {$c} { llength $a } else { dict size $a }\n  string length $a\n}\n",
    );
    assert!(
        diags.iter().any(|d| {
            code_str(d).as_deref() == Some("S100")
                && d["message"]
                    .as_str()
                    .is_some_and(|m| m.contains("path-dependent"))
        }),
        "expected a path-dependent merge S100: {diags:?}"
    );
}

#[test]
fn ordering_compare_on_strings_has_no_s100() {
    // `$s < "banana"` compares two strings — Tcl stays on the string path and
    // never coerces `$s` to a number, so its intrep is untouched (verified on
    // tclsh 8.6/9.0). No S100 (was a false positive before the ordering ops
    // were gated the same way as `==`/`!=`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set s [string trim hello]\nexpr {$s < \"banana\"}\n");
    assert!(!has_code(&diags, "S100"), "unexpected S100: {diags:?}");
    assert!(!has_code(&diags, "S101"), "unexpected S101: {diags:?}");
}

#[test]
fn ordering_compare_string_vs_numeric_literal_stays_silent() {
    // FP guard: a numeric literal on one side does NOT force the other
    // operand onto the numeric path — C Tcl probes each operand's OWN value
    // (`GetNumberFromObj` per operand), and "hello" cannot parse, so the
    // comparison string-compares and `s` keeps its `string` intrep
    // (tclsh-verified; see FP-SH-20's addendum). The old "TP control"
    // asserting S100 here locked in a refuted claim.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set s [string trim hello]\nexpr {$s <= 5}\n");
    assert!(!has_code(&diags, "S100"), "unexpected S100: {diags:?}");
    assert!(!has_code(&diags, "S101"), "unexpected S101: {diags:?}");
}

#[test]
fn shimmer_fires_inside_tcloo_method_body() {
    // TclOO method bodies must get shimmer coverage (the compiler-checks
    // aggregator walks method/body units, not just the top level and
    // procedures). Exactly one, not two: `tcl-lsp-db::proc_taint_solve`
    // (the live server's memoised path) has its own top-up loop for
    // methods/body units, independent of `compiler_checks.rs`'s direct
    // path — if both the top-up loop *and* the main loop covered
    // methods/body units, every shimmer diagnostic inside one would be
    // double-emitted. Rooted builtins make the shimmer independent of
    // receiver-namespace command shadows.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {\n    method m {} {\n        ::set x hello\n        ::incr x\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert_eq!(
        s100.len(),
        1,
        "expected exactly one S100 inside a TclOO method body: {diags:?}"
    );
}

#[test]
fn shimmer_fires_inside_namespace_eval_body() {
    // Same double-count guard as `shimmer_fires_inside_tcloo_method_body`,
    // for the other synthetic body-unit kind.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ns {\n    set x hello\n    incr x\n}\n";
    let diags = lsp.open_ready(&uri, src);
    let s100: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S100"))
        .collect();
    assert_eq!(
        s100.len(),
        1,
        "expected exactly one S100 inside a namespace eval body: {diags:?}"
    );
}

#[test]
fn no_shimmer_for_tcloo_instance_variable_linked_via_my_variable() {
    // (FP guard) `my variable count` links an object-instance variable whose
    // true intrep depends on another method's last write, not the nominal
    // return type of `my`. Before the registry fix (`oo_my.rs`'s `variable`
    // subcommand), this spuriously claimed a String->Int shimmer.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Counter {\n    method bump {} {\n        my variable count\n        incr count\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !diags.iter().any(|d| {
            matches!(code_str(d).as_deref(), Some("S100" | "S101"))
                && d.get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .contains("'count'")
        }),
        "'my variable'-linked instance var must not spuriously shimmer: {diags:?}"
    );
}

/// (Regression guard) `my` dispatches to an arbitrary `TclOO` method name — the
/// registry only recognises `variable` (for the scope-alias trait) and must
/// keep `allow_unknown_subcommands` set, or every other `my <method>` call
/// (the overwhelmingly common form) would falsely draw W001.
#[test]
fn my_arbitrary_method_call_does_not_draw_w001() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Counter {\n    method bump {} {\n        my touch\n    }\n    method touch {} {}\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(!has_code(&diags, "W001"), "unexpected W001: {diags:?}");
}

#[test]
fn shimmer_fires_through_interp_alias() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "interp alias {} myindex {} ::lindex\nset x [dict create a 1 b 2]\nmyindex $x 0\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "S100"),
        "expected S100 through an interp alias to a shimmering builtin: {diags:?}"
    );
}

#[test]
fn shimmer_noqa_suppresses_s100() {
    // `# noqa: S100` on the line before the shimmering command must
    // suppress it through the live publishDiagnostics pipeline.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set x [dict create a 1 b 2]\n# noqa: S100\nlindex $x 0\n",
    );
    assert!(
        !has_code(&diags, "S100"),
        "S100 must be suppressed by a preceding '# noqa: S100': {diags:?}"
    );
}

#[test]
fn analyser_noqa_suppresses_an_analyser_code() {
    // The analyser families (`E` / `W` / …) obey the same `# noqa` contract the
    // shimmer and optimiser families do: the analyser records `suppressed_lines`
    // without filtering by it, and the policy step applies it to every
    // finding before the report is published.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // Each read sits in its own procedure: at top level tclsh stops at the
    // first unset read, so the second is never reached.
    let diags = lsp.open_ready(
        &uri,
        "proc a {} {\n    # noqa: W210\n    puts $alpha\n}\nproc b {} {\n    puts $beta\n}\n",
    );
    assert!(
        diags.iter().all(|d| !message(d).contains("alpha")),
        "'# noqa: W210' must silence the W210 on the line it precedes: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| message(d).contains("beta")),
        "the unmarked read must still be reported: {diags:?}"
    );
}

#[test]
fn analyser_noqa_for_an_unrelated_code_does_not_suppress_w210() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# noqa: S100\nputs $alpha\n");
    assert!(
        has_code(&diags, "W210"),
        "a noqa naming another code must leave W210 alone: {diags:?}"
    );
}

#[test]
fn shimmer_noqa_for_unrelated_code_does_not_suppress_s100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set x [dict create a 1 b 2]\n# noqa: W100\nlindex $x 0\n",
    );
    assert!(
        has_code(&diags, "S100"),
        "S100 must still fire when the preceding noqa names an unrelated code: {diags:?}"
    );
}

// S103 (shared-value copy-on-write).

#[test]
fn s103_fires_on_shared_copy_mutation_as_hint_with_tight_range() {
    // The tight pattern: `set b $a`, `lappend b y`, `a` read afterwards —
    // tclsh-verified (8.6.14): b holds the same object as a until the
    // lappend, which duplicates the whole list before appending. The
    // range covers the mutating command; the severity is Hint (4).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [lrepeat 1000 x]\nset b $a\nlappend b y\nputs [llength $a]\n",
    );
    let s103: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("S103"))
        .collect();
    assert_eq!(s103.len(), 1, "expected exactly one S103: {diags:?}");
    let range = &s103[0]["range"];
    assert_eq!(range["start"]["line"], 2);
    assert_eq!(range["start"]["character"], 0);
    assert_eq!(range["end"]["line"], 2);
    assert_eq!(range["end"]["character"], 11, "must cover `lappend b y`");
    assert_eq!(
        s103[0].get("severity").and_then(Value::as_i64),
        Some(4), // Hint
        "S103 is a hint: {:?}",
        s103[0]
    );
}

#[test]
fn s103_silent_for_param_mutation_and_explicit_copy_and_dead_source() {
    let mut lsp = Lsp::tcl();
    // FP guard: mutating a parameter directly is idiomatic (the caller's
    // copy is the normal case) — silent.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {l} {\n    lappend l x\n    return $l\n}\n");
    assert!(!has_code(&diags, "S103"), "param mutation FP: {diags:?}");

    // FP guard: an explicit copy is not a pure-var-ref pair — silent.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [list 1 2 3]\nset b [lrange $a 0 end]\nlappend b y\nputs [llength $a]\n",
    );
    assert!(!has_code(&diags, "S103"), "explicit copy FP: {diags:?}");

    // FP guard: the source is dead past the mutation — silent (the sharing
    // is unwanted; deliberate under-approximation).
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [list 1 2 3]\nset b $a\nlappend b y\nputs [llength $b]\n",
    );
    assert!(!has_code(&diags, "S103"), "dead source FP: {diags:?}");

    // FP guard: array elements conflate onto one SSA symbol — silent.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set arr(src) [list 1 2 3]\nset b $arr(src)\nlappend b y\nputs [llength $arr(src)]\n",
    );
    assert!(!has_code(&diags, "S103"), "array element FP: {diags:?}");
}

#[test]
fn s103_noqa_suppresses_and_shimmer_toggle_disables() {
    // `# noqa: S103` on the preceding line suppresses it through the live
    // pipeline (the shimmer-family suppression path).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a [list 1 2 3]\nset b $a\n# noqa: S103\nlappend b y\nputs [llength $a]\n",
    );
    assert!(
        !has_code(&diags, "S103"),
        "S103 must be suppressed by a preceding '# noqa: S103': {diags:?}"
    );
}
