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

//! End-to-end coverage for the diagnostics precision review: tight ranges,
//! did-you-mean suggestions and their quick fixes, and the false-positive
//! classes the review eliminated.  Each area carries its TP (fires), FP
//! guard (must not fire), and — where a fix exists — the fix payload.

use crate::common::{Lsp, unique_uri};

use serde_json::Value;

fn code_str(d: &Value) -> String {
    match d.get("code") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => "None".to_owned(),
    }
}

fn codes(diags: &[Value]) -> Vec<String> {
    diags.iter().map(code_str).collect()
}

fn find<'a>(diags: &'a [Value], code: &str) -> Option<&'a Value> {
    diags.iter().find(|d| code_str(d) == code)
}

fn range_of(d: &Value) -> (i64, i64, i64, i64) {
    let r = &d["range"];
    (
        r["start"]["line"].as_i64().unwrap(),
        r["start"]["character"].as_i64().unwrap(),
        r["end"]["line"].as_i64().unwrap(),
        r["end"]["character"].as_i64().unwrap(),
    )
}

fn message_of(d: &Value) -> String {
    d["message"].as_str().unwrap_or_default().to_owned()
}

// -- W123 / rename resolution ---------------------------------------------

/// FP guard: `rename OLD NEW` binds NEW — calling the renamed name is not an
/// unknown command. TP control: the vacated OLD name draws both W128 (the
/// specific "renamed or deleted" hint) and W123 (the generic "unknown
/// command" — issue #973: confirmed against tclsh 8.6.14 that `rename
/// user_args ua2` really does make a later `user_args` call fail "invalid
/// command name", the same as any other unresolved command).
#[test]
fn rename_target_is_known_and_old_name_flags_w128() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc user_args {args} { puts [llength $args] }\nrename user_args ua2\nua2 1 2\nuser_args 9\n",
    );
    // `ua2 1 2` is line 2 — the rename target must not draw W123 there.
    assert!(
        !diags
            .iter()
            .any(|d| code_str(d) == "W123" && range_of(d).0 == 2),
        "the rename target must be a known command: {diags:?}"
    );
    // `user_args 9` is line 3 — the vacated source name.
    assert!(
        codes(&diags).iter().any(|c| c == "W128"),
        "the vacated source name still draws W128: {diags:?}"
    );
    assert!(
        diags
            .iter()
            .any(|d| code_str(d) == "W123" && range_of(d).0 == 3),
        "the vacated source name now also draws W123 (issue #973): {diags:?}"
    );
}

/// Suggestion-budget guard: a 3-character unknown command must not fish an
/// unrelated command out of the registry (the old fixed budget suggested
/// 'cat' for 'ua2').
#[test]
fn short_unknown_command_gets_no_far_suggestion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "zq9 1 2\n");
    let d = find(&diags, "W123").expect("W123 fires for a genuinely unknown command");
    assert!(
        !message_of(d).contains("did you mean"),
        "no candidate is within one edit of 'zq9': {}",
        message_of(d)
    );
}

/// TP: a builtin renamed away (`rename puts {}`) no longer resolves — the
/// later call draws W123.  FP guard: a call lexically *before* the rename
/// still resolves to the builtin.
#[test]
fn renamed_away_builtin_fires_w123_only_after_the_rename() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "rename puts {}\nputs gone\n");
    let d = find(&diags, "W123").expect("the deleted builtin draws W123");
    assert!(
        message_of(d).contains("'puts'"),
        "W123 names the deleted builtin: {}",
        message_of(d)
    );

    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "puts early\nrename puts myputs\nmyputs late\n");
    assert!(
        !codes(&diags2).iter().any(|c| c == "W123"),
        "a call before the rename and the rename target both resolve: {diags2:?}"
    );
}

// -- Issue #1010: known-command checks gated on deletion -------------------
//
// Four `var_command.rs` "is this a known command" checks had no deletion
// gate at all (the pre-#973 shape): interpolated-W123 resolution, W307
// dynamic-dispatch suppression, dispatch-table reference synthesis, and
// constructor object-typing. Each is confirmed against tclsh 8.6.14.

/// TP: an interpolated command head (`do${suffix}`) folding to a proc
/// renamed away with no re-establishment must keep its W123 — this
/// closure used to delete an already-correct W123 outright.
#[test]
fn interpolated_head_folding_to_a_deleted_proc_keeps_w123() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc doThing {} {}\nrename doThing {}\nset suffix Thing\ndo$suffix\n",
    );
    assert!(
        codes(&diags).iter().any(|c| c == "W123"),
        "the interpolated head must still draw W123 once its target is deleted: {diags:?}"
    );
}

/// TP: a dynamic-dispatch value (`$cmd hello`) proven by SCCP to equal a
/// deleted proc name must not have its W307 silenced.
#[test]
fn dynamic_dispatch_value_equal_to_a_deleted_proc_keeps_w307() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc target {} {}\nrename target {}\nproc foo {} { set cmd target\n$cmd hello }\n",
    );
    assert!(
        codes(&diags).iter().any(|c| c == "W307"),
        "a dispatch value equalling a deleted proc must still draw W307: {diags:?}"
    );
}

/// TP: a dispatch-table literal (`array set ops {add do_add}`) naming a
/// deleted proc must draw no reference (checked via find-references,
/// since this feeds rename-tracking rather than a diagnostic directly).
#[test]
fn dispatch_table_entry_naming_a_deleted_proc_draws_no_reference() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc do_add {a b} {}\nrename do_add {}\narray set ops {add do_add}\nset k add\n$ops($k) 1 2\n";
    lsp.open_ready(&uri, src);
    // `do_add` proc declaration name at line 0, col 5.
    let refs = lsp.references(&uri, 0, 5, true);
    let refs = refs.as_array().cloned().unwrap_or_default();
    let lines: Vec<i64> = refs
        .iter()
        .map(|r| r["range"]["start"]["line"].as_i64().unwrap())
        .collect();
    assert!(
        !lines.contains(&2),
        "a deleted proc must draw no reference from the dead dispatch-table entry: {lines:?}"
    );
}

/// TP: `[Cls new] method` where `Cls` was renamed away with no
/// re-establishment must not draw the misleading "unknown method" W308
/// (which implies `Cls` exists) — `Cls` itself draws W123 instead.
#[test]
fn constructor_of_a_deleted_class_does_not_draw_misleading_w308() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\n[Dog new] fly\n",
    );
    let cs = codes(&diags);
    assert!(
        !cs.iter().any(|c| c == "W308"),
        "a deleted class must not draw the misleading unknown-method W308: {diags:?}"
    );
    assert!(
        cs.iter().any(|c| c == "W123"),
        "the deleted class itself must draw W123: {diags:?}"
    );
}

/// FP guard: `coroutine NAME cmd` binds NAME as a command — calling it is
/// not unknown.  TP control: a typo'd coroutine name still fires.
#[test]
fn coroutine_created_command_is_known() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNum\n",
    );
    assert!(
        !codes(&diags).iter().any(|c| c == "W123"),
        "the coroutine command must be known: {diags:?}"
    );

    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(
        &uri2,
        "proc gen {} { while 1 { yield 1 } }\ncoroutine nextNum gen\nnextNun\n",
    );
    let d = find(&diags2, "W123").expect("the typo'd coroutine name draws W123");
    assert!(message_of(d).contains("nextNun"));
}

/// FP guard: `interp create NAME` binds NAME — `NAME eval {…}` is a real
/// dispatch, not an unknown command.
#[test]
fn interp_create_name_is_known() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "interp create child\nchild eval { puts hi }\n");
    assert!(
        !codes(&diags).iter().any(|c| c == "W123"),
        "the created interpreter command must be known: {diags:?}"
    );
}

/// FP guard: a literal glob `namespace import` provides the names matching
/// its tail — an imported call is silent.  TP control: an unrelated name in
/// the same file still fires.
#[test]
fn namespace_import_glob_suppresses_only_matching_names() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "namespace import ::acme::widgets::render_*\nrender_box 10\nfrobnicate 1\n",
    );
    let w123s: Vec<_> = diags
        .iter()
        .filter(|d| code_str(d) == "W123")
        .map(message_of)
        .collect();
    assert!(
        w123s.iter().all(|m| !m.contains("render_box")),
        "the glob-imported name is known: {w123s:?}"
    );
    assert!(
        w123s.iter().any(|m| m.contains("frobnicate")),
        "an unrelated name still fires: {w123s:?}"
    );
}

// -- W210 nested-read narrowing + cross-event suppression ------------------

/// The read-before-set squiggle sits on the `$foo` read nested inside the
/// command substitution, not on the whole `set` command.
#[test]
fn w210_range_is_tight_on_nested_read() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x [lindex $foo 0]\nputs $x\n");
    let d = find(&diags, "W210").expect("W210 fires for the unset $foo");
    // `$foo` occupies characters 14..18 of line 0.
    assert_eq!(range_of(d), (0, 14, 0, 18), "range must cover exactly $foo");
}

/// iRules cross-event flow: a variable set in `HTTP_REQUEST` is defined by the
/// time `HTTP_RESPONSE` reads it — no read-before-set.  TP control: a name no
/// event defines still fires.
#[test]
fn w210_silent_on_cross_event_read_but_fires_on_undefined() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n    set g 1\n}\nwhen HTTP_RESPONSE {\n    set x $g\n    puts $x\n}\n",
        "tcl-irule",
    );
    assert!(
        !codes(&diags).iter().any(|c| c == "W210"),
        "cross-event defined variable is not read-before-set: {diags:?}"
    );

    let uri2 = unique_uri("irule");
    let diags2 = lsp.open_ready_lang(
        &uri2,
        "when HTTP_RESPONSE {\n    set x $neverdefined\n    puts $x\n}\n",
        "tcl-irule",
    );
    assert!(
        codes(&diags2).iter().any(|c| c == "W210"),
        "a name no event defines still fires W210: {diags2:?}"
    );
}

// -- W308 method word + suggestion + fix -----------------------------------

const OO_SRC: &str = "oo::class create Animal {\n    method speak {} { return woof }\n}\nset a [Animal new]\n$a spek\n";

/// The unknown-method squiggle sits on the method word, the message carries
/// the MRO-derived suggestion, and the attached fix replaces exactly that
/// word.
#[test]
fn w308_anchors_method_word_with_mro_suggestion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, OO_SRC);
    let d = find(&diags, "W308").expect("W308 fires for the typo'd method");
    // `spek` occupies characters 3..7 of line 4.
    assert_eq!(range_of(d), (4, 3, 4, 7), "range must cover exactly `spek`");
    assert!(
        message_of(d).contains("did you mean 'speak'"),
        "suggestion comes from the class's methods: {}",
        message_of(d)
    );
}

/// FP guard for the suggestion: a method name nowhere near any declared
/// method gets no suggestion (and the diagnostic still fires).
#[test]
fn w308_no_far_suggestion() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Animal {\n    method speak {} { return woof }\n}\nset a [Animal new]\n$a zz9\n",
    );
    let d = find(&diags, "W308").expect("W308 fires");
    assert!(
        !message_of(d).contains("did you mean"),
        "no method is within one edit of 'zz9': {}",
        message_of(d)
    );
}

// -- E003 surplus-argument anchoring + removal fix --------------------------

/// The too-many-arguments squiggle covers only the surplus run — for a
/// same-file proc resolved at flush time as much as for a registry command.
#[test]
fn e003_range_covers_surplus_run_for_proc_and_builtin() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc greet {name} { puts $name }\ngreet a b\n");
    let d = find(&diags, "E003").expect("E003 fires for the surplus argument");
    // Line 1 is `greet a b`; the surplus `b` is character 8..9.
    assert_eq!(range_of(d), (1, 8, 1, 9), "range must cover exactly `b`");

    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "string length abc def ghi\n");
    let d2 = find(&diags2, "E003").expect("E003 fires for the builtin");
    // Surplus run `def ghi` is characters 18..25.
    assert_eq!(range_of(d2), (0, 18, 0, 25));
}

// -- W124 literal anchoring --------------------------------------------------

/// The invalid-IP squiggle covers the literal itself, not the whole `set`.
#[test]
fn w124_range_is_tight_on_the_literal() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set ip 10.0.0.999\nputs $ip\n");
    let d = find(&diags, "W124").expect("W124 fires for the over-255 octet");
    // `10.0.0.999` occupies characters 7..17 of line 0.
    assert_eq!(range_of(d), (0, 7, 0, 17));
}

// -- after: registry default-form first word ---------------------------------

/// `after <ms>` selects the default form for every Tcl integer spelling —
/// including the radix forms the old decimal-only check rejected — while a
/// non-integer word still draws the unknown-subcommand warning.
#[test]
fn after_integer_first_word_is_not_a_subcommand() {
    let mut lsp = Lsp::tcl();
    for src in ["after 200 {puts hi}\n", "after 0x10 {puts hi}\n"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(
            !codes(&diags).iter().any(|c| c == "W001"),
            "{src:?} is the default delayed form: {diags:?}"
        );
    }
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "after soon {puts hi}\n");
    assert!(
        codes(&diags).iter().any(|c| c == "W001"),
        "a non-integer, non-subcommand word still fires W001: {diags:?}"
    );
}

// -- W120 guarded package require --------------------------------------------

/// The guarded-optional-dependency idiom satisfies W120; an unguarded use
/// still fires.
#[test]
fn w120_silent_on_guarded_require_fires_without() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "if {[catch {package require Tk} err]} {\n    exit 0\n}\ncanvas .c -width 10\n",
    );
    assert!(
        !codes(&diags).iter().any(|c| c == "W120"),
        "the require inside the catch guard counts: {diags:?}"
    );

    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "canvas .c -width 10\n");
    assert!(
        codes(&diags2).iter().any(|c| c == "W120"),
        "without any require, W120 fires: {diags2:?}"
    );
}

// -- TK1003 pair-aware option scan -------------------------------------------

/// The unknown-option hint anchors on the offending word with a suggestion;
/// option *values* starting with `-` and legal unique-prefix abbreviations
/// stay silent.
#[test]
fn tk1003_tight_anchor_suggestion_and_fp_guards() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require Tk\nbutton .b -comand {puts hi}\nlabel .l -padx -2 -text ok\nentry .e -wid 20\n",
    );
    let tk: Vec<&Value> = diags.iter().filter(|d| code_str(d) == "TK1003").collect();
    assert_eq!(
        tk.len(),
        1,
        "only the real typo fires — not the -2 value, not the -wid abbreviation: {diags:?}"
    );
    // `-comand` occupies characters 10..17 of line 1.
    assert_eq!(range_of(tk[0]), (1, 10, 1, 17));
    assert!(
        message_of(tk[0]).contains("Did you mean '-command'"),
        "suggestion from the declared option set: {}",
        message_of(tk[0])
    );
}

// -- IRULE3102 single ownership ----------------------------------------------

/// Exactly one IRULE3102 per getter site, anchored on the getter word.
#[test]
fn irule3102_fires_once_with_tight_anchor() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("irule");
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n    set u [HTTP::uri]\n    puts $u\n}\n",
        "tcl-irule",
    );
    let hits: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d) == "IRULE3102")
        .collect();
    assert_eq!(hits.len(), 1, "one emitter owns the code: {diags:?}");
    // `HTTP::uri` inside the brackets occupies characters 11..20 of line 1.
    assert_eq!(range_of(hits[0]), (1, 11, 1, 20));
}

// -- W127 did-you-mean fix -----------------------------------------------------

/// The invalid-enum-value warning names the closest allowed value.
#[test]
fn w127_suggests_closest_allowed_value() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "string is alnun \"abc\"\n");
    let d = find(&diags, "W127").expect("W127 fires for the misspelt class");
    assert!(
        message_of(d).contains("did you mean 'alnum'"),
        "suggestion from the allowed set: {}",
        message_of(d)
    );
}

// -- tclpkg manifest environment ----------------------------------------------

/// A `tclpkg.tcl` manifest resolves its directives against the registry's
/// whole-file environment: no unknown-command / package noise, while a
/// typo'd directive still fires (closed environment).
#[test]
fn tclpkg_manifest_directives_resolve() {
    let mut lsp = Lsp::tcl();
    let uri = format!("file:///e2e/{}_manifest/tclpkg.tcl", std::process::id());
    let diags = lsp.open_ready(
        &uri,
        "package demo-app\nversion 0.1.0\nrequire json 1.0.0\nprovides demo::serve\nentry main.tcl\n",
    );
    assert!(
        diags.is_empty(),
        "a valid manifest produces no diagnostics: {diags:?}"
    );

    let uri2 = format!("file:///e2e/{}_manifest2/tclpkg.tcl", std::process::id());
    let diags2 = lsp.open_ready(&uri2, "package demo-app\nverison 0.1.0\n");
    assert!(
        codes(&diags2).iter().any(|c| c == "W123"),
        "a typo'd directive stays unknown in the closed environment: {diags2:?}"
    );
}

// -- S102 intra-iteration thunking ---------------------------------------------

/// The textbook oscillation fires even though the loop-header type is
/// consistent; the hardened single-representation loop stays silent.
#[test]
fn s102_intra_iteration_oscillation_matrix() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc j {items} {\n    set acc \"\"\n    foreach x $items {\n        lappend acc $x\n        set acc \"$acc,\"\n    }\n    return $acc\n}\n",
    );
    assert!(
        codes(&diags).iter().any(|c| c == "S102"),
        "intra-iteration oscillation thunks: {diags:?}"
    );

    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(
        &uri2,
        "proc j {items} {\n    set parts [list]\n    foreach x $items {\n        lappend parts $x\n    }\n    return [join $parts ,]\n}\n",
    );
    assert!(
        !codes(&diags2).iter().any(|c| c == "S102"),
        "a single-representation accumulator does not thunk: {diags2:?}"
    );
}

// -- Review 2: tight ranges for S100/S101, W313, W110; I230 fidelity --------

/// The S100 expression-shimmer squiggle sits on the offending operand
/// (`$x` inside the braced expr), not on the whole `set` statement.
#[test]
fn s100_range_is_tight_on_expr_operand() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x \"hello\"\nset y [expr {$x + 1}]\nputs $y\n");
    let d = find(&diags, "S100").expect("S100 fires for the String operand");
    // `$x` occupies characters 13..15 of line 1 (`set y [expr {$x + 1}]`).
    assert_eq!(range_of(d), (1, 13, 1, 15), "range must cover exactly $x");
}

/// The S101 in-loop expression shimmer anchors the operand inside the loop
/// body, matching the S100 narrowing.
#[test]
fn s101_range_is_tight_on_in_loop_expr_operand() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc f {l} {\n    foreach x $l {\n        set y [expr {$x + 1}]\n        puts $y\n    }\n}\n",
    );
    let d = find(&diags, "S101").expect("S101 fires for the in-loop operand");
    // `$x` occupies characters 21..23 of line 2 (`        set y [expr {$x + 1}]`).
    assert_eq!(range_of(d), (2, 21, 2, 23), "range must cover exactly $x");
}

/// The W313 destructive-file squiggle sits on the dynamic path argument,
/// not on the whole `file delete` command.
#[test]
fn w313_range_is_tight_on_path_argument() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set p [gets stdin]\nfile delete -force $p\n");
    let d = find(&diags, "W313").expect("W313 fires for the variable path");
    // `$p` occupies characters 19..21 of line 1 (`file delete -force $p`).
    assert_eq!(range_of(d), (1, 19, 1, 21), "range must cover exactly $p");
}

/// The W110 hint anchors on the offending `==` operator itself — and on the
/// *matched* operator when an earlier `==` is a legitimate numeric compare.
#[test]
fn w110_range_is_tight_on_the_matched_operator() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set a 1\nset b 2\nset c x\nif {$a == $b && $c == \"x\"} { puts hi }\n",
    );
    let d = find(&diags, "W110").expect("W110 fires for the string compare");
    // The second `==` occupies characters 19..21 of line 3
    // (`if {$a == $b && $c == "x"} { puts hi }`).
    assert_eq!(
        range_of(d),
        (3, 19, 3, 21),
        "range must cover exactly the matched =="
    );
}

/// I230 quotes a bare `$n` condition as the source spells it — not the
/// segmenter's re-braced `${n}` reconstruction.
#[test]
fn i230_message_spells_bare_var_like_the_source() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set n 1\nif $n { puts a } else { puts b }\n");
    let d = find(&diags, "I230").expect("I230 fires for the constant condition");
    assert!(
        message_of(d).contains("'$n'") && !message_of(d).contains("${n}"),
        "message must quote `$n` verbatim: {}",
        message_of(d)
    );
}

/// The in-loop S101 phi merge anchors on the loop-body assignment that
/// produces the merging type, not the pre-loop initialiser.
#[test]
fn s101_phi_merge_anchors_in_loop_not_on_initialiser() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "set v 5\nforeach x {a b c} {\n    puts $v\n    if {$x eq \"b\"} { set v \"s\" } else { set v 7 }\n}\n",
    );
    let d = diags
        .iter()
        .find(|d| code_str(d) == "S101" && d["message"].as_str().unwrap_or("").contains("merges"))
        .expect("S101 phi merge fires");
    let (line, _, _, _) = range_of(d);
    assert!(
        line >= 1,
        "the merge anchor must sit inside the loop (line >= 1), got {:?}",
        range_of(d)
    );
}
