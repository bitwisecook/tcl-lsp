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

//! Issues 1078, 1142, 1237, 1260 and 1266: a braced word is a literal name or plain data, not a read.

use super::*;

// A brace-quoted `$`-bearing name is a *literal* variable name
//
// Oracle, tclsh 9.0.4 and 8.6.14, byte-identical:
//
//   set {$n} v ; set {$n}            -> v
//   info exists {$n} / info exists n -> 1 / 0     (two distinct variables)
//   set n other ; set {$n}           -> v         (unaffected)
//   set {$m} 1 ; set m               -> can't read "m": no such variable
//   set ${$n}                        -> can't read "v"  (`${$n}` read `$n`)
//   set {a b} v2 ; set x ${a b}      -> v2
//   set i 5 ; set {arr($i)} 1
//     array names arr                -> {$i}      (the literal key)
//     info exists arr(5)             -> 0
//   unset {$n} ; info exists {$n} / n -> 0 / 1
//
// Acceptance measurements for this family
// (`docs/design/compiler/ssa-construction.md`).

/// The **true positive** a def landing on the wrong variable would miss:
/// `set {$n} 1; puts $n` must warn, because tclsh errors
/// `can't read "n": no such variable`.
#[test]
fn issue_1078_braced_write_does_not_define_the_plain_name() {
    let src = "proc f {} { set {$n} 1; puts $n }\n";
    assert!(
        fires(src, D, "W210"),
        "#1078 TP: `set {{$n}} 1` defines `$n`, so the bare `$n` read is still \
read-before-set (tclsh: `can't read \"n\"`); emitted: {:?}",
        codes(src, D)
    );
}

/// The **false positive** to avoid: a lone `set {$n} 1` reads nothing at all,
/// so recording a read of `n` fires `W210 Variable 'n' is read before it is
/// set` on code that never mentions `n`.
#[test]
fn issue_1078_braced_write_alone_is_not_a_read_of_the_plain_name() {
    let src = "proc f {} { set {$n} 1 }\n";
    assert!(
        !fires(src, D, "W210"),
        "#1078 FP: `set {{$n}} 1` substitutes nothing and reads nothing; \
emitted: {:?}",
        codes(src, D)
    );
    assert!(
        fires(src, D, "W211"),
        "#1078: the write is a genuine unused-variable hint; emitted: {:?}",
        codes(src, D)
    );
    let unused: Vec<String> = crate::analyser::Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W211")
        .map(|d| d.message.clone())
        .collect();
    assert!(
        unused.iter().any(|m| m.contains("'$n'")),
        "#1078: the hint must name `$n`, not `n`; got {unused:?}"
    );
}

/// The **false `W220`** to avoid.  The braced verdict must match the
/// identically-shaped plain-named control exactly: `[set …]` really does read
/// the second store, so it draws no `W220` in either spelling, and the first
/// store — overwritten before anything reads it — draws the one `W220` in
/// both (tclsh 8.4.20 to 9.1b0 return 2 with or without it, and O109 deletes
/// it). The SSA records the return word's `[set n]` as a use of the second
/// store's version alone, where a name-level hidden read would silence both
/// stores.
#[test]
fn issue_1078_braced_double_store_matches_the_plain_control() {
    let braced = "proc f {} { set {$n} 1; set {$n} 2; return [set {$n}] }\n";
    let plain = "proc f {} { set n 1; set n 2; return [set n] }\n";
    for src in [braced, plain] {
        let dead_stores: Vec<u32> = crate::analyser::Analyser::new()
            .analyse(src, D)
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W220")
            .map(|d| d.span.start())
            .collect();
        // The finding anchors at the first store's name word.
        let first_store = src.find("set ").expect("the first store") + "set ".len();
        let first_store = u32::try_from(first_store).expect("an offset");
        assert_eq!(
            dead_stores,
            vec![first_store],
            "#1078: `[set …]` reads the second store, so only the first is dead; \
emitted: {:?}",
            codes(src, D)
        );
    }
    assert_eq!(
        codes(braced, D),
        codes(plain, D),
        "#1078: the braced spelling must draw exactly the plain spelling's \
verdicts"
    );
}

/// `unset {$n}` destroys the literal `$n` — it neither reads nor destroys `n`,
/// so no `W213 Variable 'n' may not exist`.
#[test]
fn issue_1078_braced_unset_does_not_touch_the_plain_name() {
    let src = "proc f {} { set {$n} 1; unset {$n}; return ok }\n";
    assert!(
        !fires(src, D, "W213"),
        "#1078: `unset {{$n}}` destroys the variable it just set; emitted: {:?}",
        codes(src, D)
    );
    assert!(
        !fires(src, D, "W210"),
        "#1078: …and reads nothing; emitted: {:?}",
        codes(src, D)
    );
}

/// TN control for the family: the *unbraced* substituted spellings must keep
/// behaving as dynamic names — the dynamic-name barrier still has to fire.
/// Same shape as FP-DS-14's control, restated here so the braced-name rules
/// cannot quietly erase the barrier they sit on top of.
#[test]
fn issue_1078_unbraced_substituted_target_still_behaves_dynamically() {
    // `[set $p]` may observe *any* store, so the read barrier blinds the
    // dead-store pass for the whole function.
    let dynamic = "proc f {p} { set b [set $p]\n set a 1\n set a 2\n return \"$a$b\" }\n";
    assert!(
        !fires(dynamic, D, "W220"),
        "#1076 barrier: a computed read target must still blind W220; \
emitted: {:?}",
        codes(dynamic, D)
    );
    // TP control: brace the name and the barrier clears, so the genuine dead
    // store is reported again.
    let braced = "proc f {p} { set b [set {$p}]\n set a 1\n set a 2\n return \"$a$b\" }\n";
    assert!(
        fires(braced, D, "W220"),
        "#1078: a brace-quoted target computes nothing, so the unrelated dead \
store is still reported; emitted: {:?}",
        codes(braced, D)
    );
}

/// The `${…}` **read** form reaches the same cell the brace-quoted write
/// created: `set {$n} 1; return ${$n}` is a complete write/read pair with
/// nothing to report, and it must not be read as touching `n`.
#[test]
fn issue_1078_brace_reference_form_reads_the_literal_name() {
    let src = "proc f {} { set {$n} 1; return ${$n} }\n";
    assert!(
        codes(src, D).is_empty(),
        "#1078: `${{$n}}` reads the `{{$n}}` cell — nothing to report; \
emitted: {:?}",
        codes(src, D)
    );
}

/// A brace-quoted **array element** keeps its literal key, and a space-bearing
/// braced name is an ordinary variable — both must key on their own spelling.
#[test]
fn issue_1078_braced_element_and_spaced_names_key_on_their_own_spelling() {
    let elem = "proc f {} { set {arr($i)} 1; return [set {arr($i)}] }\n";
    assert!(
        codes(elem, D).is_empty(),
        "#1078: `{{arr($i)}}` writes and reads one literal element; emitted: {:?}",
        codes(elem, D)
    );
    let spaced = "proc f {} { set {a b} 1; return [set {a b}] }\n";
    assert!(
        codes(spaced, D).is_empty(),
        "#1078: `{{a b}}` is a legal name written and read; emitted: {:?}",
        codes(spaced, D)
    );
}

// A braced word is data, not a script the call site substitutes.  The SSA
// use is *classified* (`ssa::UseClass`) rather than
// dropped: liveness keeps honouring it (the text may be evaluated later),
// read-before-set does not.
//
// Oracle, tclsh 8.6.14:
//
//   puts {$y}                       -> $y          (y undefined; no error)
//   puts {\n set y 1\n return $y\n} -> the literal three lines
//   proc f {} { return {$y} }; f    -> $y
//   catch {puts $y} m; set m        -> can't read "y": no such variable

/// A braced data argument reads nothing, so no `W210`.
#[test]
fn issue_1237_braced_data_argument_is_not_a_read() {
    for src in [
        "proc f {} { puts {$y} }\n",
        "proc f {} { puts {\n    set y 1\n    return $y\n} }\n",
        "proc f {} { return {$y} }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1237: a braced word performs no substitution; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// The generic half: an un-hooked / unknown definer's brace-shaped
/// trailing argument carries no role information, so a `$y` in it that the
/// same word `set`s is that body's own local, whichever frame the body ends up
/// running in — never a read-before-set of the enclosing scope. The
/// unknown-command hint stays: abstaining about the *body* says nothing about
/// the *name*.
#[test]
fn issue_1142_unknown_definer_body_is_not_read_before_set() {
    let src = "proc f {} { mydefiner ::foo::bar {optlist} {\n    set y 1\n    return $y\n} }\n";
    assert!(
        !fires(src, D, "W210"),
        "#1142: an un-hooked definer body's own local is not read-before-set; \
emitted: {:?}",
        codes(src, D)
    );
    assert!(
        fires(src, D, "W123"),
        "#1142: the unknown command itself is still reported; emitted: {:?}",
        codes(src, D)
    );
}

/// TN control — a **free** read in an unclassified braced word (one
/// nothing sets) is still a read: the word may be a script, and a wrapper that
/// hands it to an `uplevel`-ing worker runs it in this frame, where tclsh
/// errors on the unset name. Only a registry-*described* command's braced
/// word (`puts {$y}`) is known data.
#[test]
fn issue_1142_unclassified_braced_word_free_read_still_fires() {
    let src = "proc f {} { mywrapper {puts $notset} }\n";
    assert!(
        fires(src, D, "W210"),
        "#1142 TN: an unclassified braced word may run in this frame; \
emitted: {:?}",
        codes(src, D)
    );
}

/// TN control — a **quoted** word really is substituted, so the genuine
/// read-before-set survives.  This is the measurement that separates
/// "classified use" from "stop scanning braced words".
#[test]
fn issue_1237_quoted_word_still_fires_w210() {
    for src in [
        "proc f {} { puts \"$y\" }\n",
        "proc f {} { puts $y }\n",
        "proc f {} { return \"$y\" }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1237 TN: a substituted read must still fire; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// FP guard — a braced word whose role has the callee evaluate it **in this
/// frame** is a real read: `expr`'s expression and `if`'s condition/body both
/// substitute against the caller's variables.
#[test]
fn issue_1237_in_frame_braced_roles_still_read() {
    for src in [
        "proc f {} { expr {$p + $q} }\n",
        "proc f {} { if {$p} { puts hi } }\n",
        "proc f {} { catch {puts $p} m }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1237 FP guard: an Expr/Body-role braced word is evaluated in \
this frame; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// FP guard — the use itself must **survive**, classified: dropping it would
/// resurrect `W211 set but never used` / `W220 never read` on a variable whose
/// only mention is inside a braced word that may be evaluated later (a
/// deferred callback, an `eval`, an unknown definer).  This is the guard rail
/// that rules out the "skip the scan" shortcut, restated for a scalar
/// alongside `w220_braced_literal_arg_is_not_a_read`'s array element.
#[test]
fn issue_1237_quoted_use_still_keeps_the_variable_live() {
    let src = "proc f {} { set x 1; puts {$x} }\n";
    assert!(
        !fires(src, D, "W211") && !fires(src, D, "W220"),
        "#1237: a quoted mention keeps the store live; emitted: {:?}",
        codes(src, D)
    );
    // TP control: with no mention of any kind the dead store is reported.
    let unmentioned = "proc f {} { set x 1; puts {$y} }\n";
    assert!(
        fires(unmentioned, D, "W211") || fires(unmentioned, D, "W220"),
        "#1237 TP: an unmentioned store is still dead; emitted: {:?}",
        codes(unmentioned, D)
    );
}

// A `$var` inside a **braced** `foreach` / `lmap` / `dict for`
// value word is a literal list element, not a substitution.
//
// tclsh-proof (tclsh 8.6.14 — the only interpreter available in this
// container):
//
//   foreach n {a $b c} { puts $n }          -> a / $b / c  (b undefined)
//   dict for {k v} {a $b} { puts "$k=$v" }  -> a=$b        (b undefined)
//
// Braces are the only thing separating the literal spelling from the
// substituting one, and the IR's `list_arg` stores the word's *content*, so
// `ForeachIterator::list_braced` is what the CFG's synthetic loop-header call
// carries the fact on.

/// FP — a braced value word reads nothing, at top level and inside a
/// proc, for every loop that lowers to `Statement::Foreach`.
#[test]
fn issue_1260_braced_loop_value_word_is_not_a_read() {
    for src in [
        "foreach n {a $b c} { }\n",
        "lmap n {a $b c} { }\n",
        "proc p {} { foreach n {a $b c} { } }\n",
        "proc p {} { lmap n {a $b c} { } }\n",
        "dict for {k v} {a $b} { }\n",
        "dict map {k v} {a $b} { }\n",
        // Multi-group: the braced group stays quiet alongside a
        // substituting one whose own read is satisfied.
        "proc p {l} { foreach x {a $b c} y $l { } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1260: a braced loop value word substitutes nothing; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// TP — the substituting spellings of the same word must still fire.
/// This is what separates "carry the braced flag" from "stop scanning the
/// loop's value word".
#[test]
fn issue_1260_substituted_loop_value_word_still_fires() {
    for src in [
        "foreach n \"a $b c\" { }\n",
        "foreach n $b { }\n",
        "lmap n \"a $b c\" { }\n",
        "proc p {} { foreach n \"a $b c\" { } }\n",
        "proc p {} { foreach n $b { } }\n",
        "dict for {k v} $b { }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1260 TP: a substituted loop value word is a real read; {src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// TN — the braced word still keeps the mentioned store *live*: the use
/// is classified (`UseClass::Quoted`), not dropped, so W211/W220 must not
/// resurrect on a variable whose only mention is inside the braced list. Same
/// guard rail as `issue_1237_quoted_use_still_keeps_the_variable_live`.
#[test]
fn issue_1260_braced_loop_value_word_keeps_the_store_live() {
    let src = "proc p {} { set x 1; foreach n {$x} { } }\n";
    assert!(
        !fires(src, D, "W211") && !fires(src, D, "W220"),
        "#1260: a quoted mention keeps the store live; emitted: {:?}",
        codes(src, D)
    );
    // TP control: with no mention of any kind the dead store is reported.
    let unmentioned = "proc p {} { set x 1; foreach n {$y} { } }\n";
    assert!(
        fires(unmentioned, D, "W211") || fires(unmentioned, D, "W220"),
        "#1260 TP: an unmentioned store is still dead; emitted: {:?}",
        codes(unmentioned, D)
    );
}

// A read collapsed out of an **opaque** `switch` arm keeps its `UseClass`.
//
// A non-lowered `switch` (`-glob` / `-regexp`, or `-exact` with a
// fall-through arm) keeps its arms as one opaque CFG statement, so those arm
// bodies are the only scripts that reach SSA un-lowered.  Collecting them as a
// bare name set and merging the lot into `ClassifiedUses::substituted` would
// see every brace-quoted DATA word inside an arm as a *substituted* read and
// draw a false W210 — while the identical spelling outside an arm is quiet,
// because the lowered path classifies it `UseClass::Quoted` and the W210
// emitter skips a quoted use.
//
// The walk threads `ClassifiedUses` end to end, so an arm body is classified
// exactly as the same script is when lowered.
//
// tclsh-proof (tclsh 9.0.4, the interpreter available in this container; the
// issue reports the same on 8.6.14):
//
//   proc f {z} { switch -glob $z { a* { foreach n {a $b c} {puts $n} } } }
//   f abc                                     -> a / $b / c   (b undefined)
//   proc f {z} { switch -glob $z { a* { puts {$b} } } };  f abc -> $b
//   proc f {z} { switch $z { a - b { puts {$q} } } };     f b   -> $q
//   proc f {z} { switch -regexp $z { a.* { puts {$b} } default { puts {$c} } } }
//   f zz                                      -> $c
//   proc f {z} { set x 1; switch -glob $z { a* { foreach n {$x} {puts $n} } } }
//   f abc                                     -> $x   (the literal two chars)
//   proc f {z} { switch -glob $z { a* { puts $b } } }
//   f abc                    -> can't read "b": no such variable

/// FP — a braced data word inside an opaque arm substitutes nothing, in
/// every arm shape that keeps the `switch` opaque and at every nesting depth
/// the collapsed walk descends.
#[test]
fn issue_1266_braced_data_word_in_opaque_switch_arm_is_not_a_read() {
    for src in [
        // The two spellings named on the issue.
        "proc f {z} { switch -glob $z { a* { foreach n {a $b c} {} } } }\n",
        "proc f {z} { switch -glob $z { a* { puts {$b} } } }\n",
        // -regexp keeps it opaque too, and the default body is walked by the
        // same helper as an arm body.
        "proc f {z} { switch -regexp $z { a.* { puts {$b} } } }\n",
        "proc f {z} { switch -regexp $z { a.* { } default { puts {$b} } } }\n",
        // -exact stays opaque when an arm falls through.
        "proc f {z} { switch $z { a - b { puts {$q} } } }\n",
        // Structured statements inside an arm are walked by `reads_in_stmt`,
        // not lowered — each recursion must carry the classification.
        "proc f {z} { switch -glob $z { a* { if {1} { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { if {0} { } else { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { while {[incr i] < 2} { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { for {set i 0} {$i < 1} {incr i} { puts {$b} } } } }\n",
        "proc f {z} { switch -glob $z { a* { catch { puts {$b} } m } } }\n",
        "proc f {z} { switch -glob $z { a* { try { puts {$b} } finally { puts {$c} } } } }\n",
        // A nested opaque switch inside an opaque arm.
        "proc f {z} { switch -glob $z { a* { switch -glob $z { c* { puts {$b} } } } } }\n",
        // Every `Statement::Foreach` value word, including a multi-group call.
        "proc f {z} { switch -glob $z { a* { foreach n {a $b c} m {$d} {} } } }\n",
        "proc f {z} { switch -glob $z { a* { lmap n {a $b c} {} } } }\n",
        // A braced `return` value is literal in an arm exactly as outside one.
        "proc f {z} { switch -glob $z { a* { return {$b} } } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1266: a braced word inside an opaque switch arm substitutes nothing; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// TP — the substituting spellings of the same words inside an opaque
/// arm must still fire. This is what separates "classify the arm's reads"
/// from "stop collecting them".
#[test]
fn issue_1266_substituted_word_in_opaque_switch_arm_still_fires() {
    for src in [
        "proc f {z} { switch -glob $z { a* { puts $b } } }\n",
        "proc f {z} { switch -glob $z { a* { foreach n \"a $b c\" {} } } }\n",
        "proc f {z} { switch -glob $z { a* { foreach n $b {} } } }\n",
        "proc f {z} { switch -glob $z { a* { if {$b} { } } } }\n",
        "proc f {z} { switch -glob $z { a* { return $b } } }\n",
        "proc f {z} { switch -regexp $z { a.* { } default { puts $b } } }\n",
        "proc f {z} { switch $z { a - b { puts $q } } }\n",
    ] {
        assert!(
            fires(src, D, "W210"),
            "#1266 TP: a substituted word inside an opaque switch arm is a real read; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
}

/// TN / guard rail — the classification must be *threaded*, never dropped.
/// Omitting the name from `reads_in_script` instead would lose its
/// **liveness** use as well, resurrecting a false W211/W220 on a store whose
/// only mention is the braced word. Same guard rail as
/// `issue_1237_quoted_use_still_keeps_the_variable_live` and
/// `issue_1260_braced_loop_value_word_keeps_the_store_live`, now inside an
/// opaque arm.
#[test]
fn issue_1266_quoted_arm_mention_keeps_the_store_live() {
    for src in [
        "proc p {z} { set x 1; switch -glob $z { a* { foreach n {$x} {} } } }\n",
        "proc p {z} { set x 1; switch -glob $z { a* { puts {$x} } } }\n",
        "proc p {z} { set x 1; switch -glob $z { a* { if {1} { puts {$x} } } } }\n",
        "proc p {z} { set x 1; switch -regexp $z { a.* { } default { puts {$x} } } }\n",
        "proc p {z} { set x 1; switch $z { a - b { puts {$x} } } }\n",
    ] {
        assert!(
            !fires(src, D, "W211") && !fires(src, D, "W220"),
            "#1266: a quoted mention inside an opaque arm keeps the store live; \
{src} emitted: {:?}",
            codes(src, D)
        );
    }
    // TP control: with no mention of any kind the dead store is still reported.
    for unmentioned in [
        "proc p {z} { set x 1; switch -glob $z { a* { foreach n {$y} {} } } }\n",
        "proc p {z} { set x 1; switch -glob $z { a* { puts {$y} } } }\n",
    ] {
        assert!(
            fires(unmentioned, D, "W211") || fires(unmentioned, D, "W220"),
            "#1266 TP: an unmentioned store is still dead; {unmentioned} emitted: {:?}",
            codes(unmentioned, D)
        );
    }
}

/// FN guard — the whole point is that an opaque arm agrees with the
/// lowered path. A plain `-exact` `switch` with no fall-through lowers its
/// arms into ordinary CFG blocks; the `-glob` spelling of the same body must
/// produce the same verdict, braced and substituted alike.
#[test]
fn issue_1266_opaque_arm_agrees_with_the_lowered_path() {
    for (lowered, opaque) in [
        (
            "proc f {z} { switch $z { a { puts {$b} } } }\n",
            "proc f {z} { switch -glob $z { a* { puts {$b} } } }\n",
        ),
        (
            "proc f {z} { switch $z { a { puts $b } } }\n",
            "proc f {z} { switch -glob $z { a* { puts $b } } }\n",
        ),
        (
            "proc f {z} { switch $z { a { foreach n {a $b c} {} } } }\n",
            "proc f {z} { switch -glob $z { a* { foreach n {a $b c} {} } } }\n",
        ),
        (
            "proc p {z} { set x 1; switch $z { a { foreach n {$x} {} } } }\n",
            "proc p {z} { set x 1; switch -glob $z { a* { foreach n {$x} {} } } }\n",
        ),
    ] {
        for code in ["W210", "W211", "W220"] {
            assert_eq!(
                fires(lowered, D, code),
                fires(opaque, D, code),
                "#1266: {code} must agree between the lowered arm ({lowered}) \
and the opaque one ({opaque}); {:?} vs {:?}",
                codes(lowered, D),
                codes(opaque, D)
            );
        }
    }
}

/// The same collapse covers the arm **patterns**, which the
/// canonical single-braced `{pat body …}` block also carries literally.
///
/// tclsh 9.0.4: `proc f {z} { switch -glob $z { $a* { puts hit } default {
/// puts miss } } }` prints `hit` for `f {$a}` and `miss` for `f zz`, with `a`
/// undefined throughout — the pattern matched the two literal characters
/// `$a`. Supplied as separate words the pattern *does* substitute, and the
/// lowered `-exact` path classifies its patterns this way, so this is the
/// same lowered-vs-opaque parity question.
#[test]
fn issue_1266_braced_switch_pattern_is_not_a_read() {
    for src in [
        "proc f {z} { switch -glob $z { $a* { } } }\n",
        "proc f {z} { switch -regexp $z { $a.* { } } }\n",
        "proc f {z} { switch $z { $a - b { } } }\n",
    ] {
        assert!(
            !fires(src, D, "W210"),
            "#1266: a pattern in the braced arm block is literal; {src} emitted: {:?}",
            codes(src, D)
        );
    }
    // TP — patterns supplied as separate words really do substitute.
    let words = "proc f {z} { switch -glob -- $z $a* { } }\n";
    assert!(
        fires(words, D, "W210"),
        "#1266 TP: a separate-word pattern substitutes; emitted: {:?}",
        codes(words, D)
    );
    // TN guard rail — the quoted pattern still keeps the store live.
    let live = "proc p {z} { set x 1; switch -glob $z { $x* { } } }\n";
    assert!(
        !fires(live, D, "W211") && !fires(live, D, "W220"),
        "#1266: a quoted pattern mention keeps the store live; emitted: {:?}",
        codes(live, D)
    );
}
