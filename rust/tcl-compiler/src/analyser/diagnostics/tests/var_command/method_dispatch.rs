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

//! W308 and the `TclOO` form of E001: objects typed by constructors, factories, renames and aliases, and calls with an unknown or missing method.

use super::*;

#[test]
fn analyse_w307_suppressed_for_known_class_constructor_chain() {
    // ``[Dog new] bark`` — ``Dog`` is a user class so
    // ``new`` returns an Object whose class is ``Dog``.
    // The W307 cmd-sub suppression should kick in.  Since
    // ``bark`` is declared on ``Dog``, no W308 either.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} { return woof } }\n[Dog new] bark",
        "tcl8.6",
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "W307 must not fire for [KnownClass new] method chain; got {:?}",
        r.diagnostics,
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "W308 must not fire when method is declared on the class; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn analyse_w308_emitted_for_unknown_method_on_known_class_constructor() {
    // ``[Dog new] fly`` — ``fly`` isn't declared on ``Dog``.
    // W307 is suppressed (constructor returns Object) but
    // W308 fires for the missing method.
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create Dog { method bark {} { return woof } }\n[Dog new] fly",
        "tcl8.6",
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "W308 expected for unknown method on known class; got {:?}",
        r.diagnostics,
    );
}

// `emit_cmd_command_diagnostics` must not type `[Cls new] method` as
// `Object(Cls)` when `Cls` is renamed or deleted with no later
// re-establishment. In C Tcl 8.6.14 the constructor call itself fails with
// "invalid command name", so an "unknown method" W308 would wrongly imply
// `Cls` exists. The dispatch falls back to the conservative "non-literal,
// cannot statically analyse" (W307), leaving W123 on `Cls` as the primary
// diagnostic.
//
// `harvest_constructor_object_types` (the `set x [Cls new]`
// variable-assignment sibling of this check) behaves the same way. The
// independent source in `type_infer.rs`'s `constructor_object_type`, which
// types `x` as `Object(Cls)` for that shape via the SSA type lattice, is
// gated in `aggregate_object_types`, where both sources are unioned and the
// analyser's own deletion facts are in scope (see the
// `w308_*_issue_1013_*` cases below).

#[test]
fn w308_tp_issue_1010_deleted_class_constructor_falls_back_to_w307() {
    let src =
        "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\n[Dog new] fly";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W308"),
        "a deleted class must not draw the misleading 'unknown method' W308; got {codes:?}"
    );
    assert!(
        codes.contains("W123"),
        "the deleted class itself must draw W123 as the real diagnostic; got {codes:?}"
    );
}

#[test]
fn w308_tp_issue_1013_deleted_class_set_var_constructor_draws_no_w308() {
    // `type_infer.rs`'s `constructor_object_type` is a second, independent
    // source of `Object(Dog)` typing for the `set x [Dog new]` shape,
    // reached through the SSA type lattice rather than
    // `harvest_constructor_object_types`. It reads an unfiltered "known
    // classes" set with no deletion awareness, so the deletion gate has to
    // sit where the two sources are unioned, or this shape draws the
    // misleading "unknown method" W308.
    // tclsh8.6 and 9.0 both fail the constructor first: `invalid command
    // name "Dog"` — `x` is never assigned an object at all.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\nproc foo {} {\n    set x [Dog new]\n    $x fly\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W308"),
        "a deleted class must not draw the misleading 'unknown method' W308; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_fp_issue_1013_reestablished_class_set_var_constructor_still_flags() {
    // FP guard for the same gate: a class deleted and then re-established
    // under the same name is live again at file end (the fresh definition
    // postdates the deletion), so `x` must still type as `Object(Dog)` and
    // the unknown method must still be flagged.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\noo::class create Dog { method bark {} { return woof } }\nproc foo {} {\n    set x [Dog new]\n    $x fly\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a re-established class must still type its constructor result; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_issue_1013_live_class_set_var_constructor_still_flags() {
    // TN control: with no deletion anywhere, the ordinary `set x [Dog new]`
    // shape must be entirely unaffected by the gate.
    let src = "oo::class create Dog { method bark {} { return woof } }\nproc foo {} {\n    set x [Dog new]\n    $x fly\n}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a live class must still validate its methods; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_copy_propagated_handle_with_a_trailing_rename_still_flags() {
    // TP restored by moving the class-liveness gate from file end to the
    // dispatch site. `foo` runs while `Dog` is alive; the rename is the last
    // line of the file and cannot affect a call that already happened.
    //
    // The file-end gate dropped `x`'s `Object(::Dog)` type outright, so the
    // dispatch lost its W308 *and* picked up a spurious W307 in its place.
    //
    // Oracle (tclsh8.6): exits 1 with
    // `unknown method "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nproc foo {} {\n    set y [Dog new]\n    set x $y\n    $x fly\n}\nfoo\nrename Dog {}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "the class is alive at the dispatch, so the unknown method must flag; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_class_renamed_to_a_name_keeps_typing_its_existing_objects() {
    // `rename Dog Cat` deletes the command *name* `Dog` but not the class —
    // the object in `d` is still a Dog and still rejects `fly`.
    //
    // Oracle (tclsh8.6 and tclsh9.0): after `set d [Dog new]` and `rename
    // Dog Cat`, `$d bark` returns `woof` while `$d fly` fails with `unknown
    // method "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nset d [Dog new]\nrename Dog Cat\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a class renamed to a name is re-established, not deleted; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w123_tp_a_class_renamed_away_still_flags_its_old_name() {
    // The paired guard: making the *class* survive a rename must not make
    // the vacated *command name* resolve. Oracle (tclsh8.6): after
    // `rename Dog Cat`, `Dog new`
    // fails with `invalid command name "Dog"`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nDog new\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    // W128 owns a call to a name this file renamed away.
    assert!(
        codes.contains("W128"),
        "the vacated name must still be unbound; got {codes:?}",
    );
}

#[test]
fn w123_tp_a_rename_in_a_dead_branch_is_not_an_unconditional_deletion() {
    // A `rename` nested in a control-flow body may never run, so it is not
    // evidence the command is gone. The analyser proves this very branch
    // dead in the same run (it emits I230 on it) yet honoured the deletion,
    // flagging a command that is demonstrably still callable and — through
    // the shared liveness facts — withdrawing the W308 as well.
    //
    // Oracle (tclsh8.6): `Dog new` succeeds and
    // `$d fly` fails with `unknown method "fly": must be bark or destroy`.
    let src = "oo::class create Dog {\n    method bark {} { return woof }\n}\nif {0} { rename Dog {} }\nset d [Dog new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W123"),
        "the branch never runs, so `Dog` is still bound; got {:?}",
        r.diagnostics,
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "and the live class must still validate its methods; got {:?}",
        r.diagnostics,
    );
    assert!(
        codes.contains("I230"),
        "the dead branch itself is still reported; got {codes:?}",
    );
}

#[test]
fn w308_tp_a_renamed_class_name_still_types_its_constructor_1049() {
    // TP — `rename Dog Cat` moves the class *command*; the class itself is
    // unchanged, so `Cat new` builds a Dog and `$d fly` is still an unknown
    // method.
    //
    // Oracle (tclsh8.6.14 and tclsh9.0.4): `oo::class create Dog { method
    // bark {} { return woof } }; rename Dog Cat; set d [Cat new]; $d fly`
    // fails with `unknown method "fly": must be bark or destroy`, while
    // `$d bark` returns `woof`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nset d [Cat new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a class reached through a rename still validates its methods; got {:?}",
        r.diagnostics,
    );
    // The object is typed, so the dispatch is not an unanalysable one.
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a typed dispatch must not also draw W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_a_renamed_class_still_accepts_its_real_methods_1049() {
    // TN — the same shape with a method the class really has draws nothing.
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `$d bark` → `woof`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nset d [Cat new]\n$d bark\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert_eq!(
        r.diagnostics.len(),
        0,
        "a valid method on a renamed class is silent; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_a_constructor_written_before_its_rename_does_not_type_1049() {
    // TN — the order gate. At top level the rename has not run yet when
    // `[Cat new]` executes, so `Cat` is simply an unknown command there and
    // nothing may be typed from it. The paired guard for
    // `w123_tp_a_class_renamed_away_still_flags_its_old_name`, from the
    // other side of the rename.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; Cat new`
    // — before any `rename` — fails with `invalid command name "Cat"`.
    // W128 owns the unbound call to a name this file later renames.
    // W123 owns names that the file never establishes.
    let src = "oo::class create Dog { method bark {} { return woof } }\nCat new\nset d [Cat new]\nrename Dog Cat\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let codes: HashSet<String> = r.diagnostics.iter().map(|d| d.code.to_string()).collect();
    assert!(
        !codes.contains("W308"),
        "nothing may be typed from a name the rename has not created yet; got {:?}",
        r.diagnostics,
    );
    assert!(
        codes.contains("W128"),
        "and the out-of-order use is still reported; got {codes:?}",
    );
}

#[test]
fn w308_tp_a_chain_of_renames_still_reaches_the_class_1049() {
    // TP — `rename Dog Cat; rename Cat Kitten` leaves the class reachable
    // under the last name; the hop walk follows the chain (bounded).
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; rename
    // Dog Cat; rename Cat Kitten; set d [Kitten new]` succeeds, and
    // `$d fly` fails `unknown method "fly"`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\nrename Cat Kitten\nset d [Kitten new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a chained rename still reaches the class; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_an_interp_alias_of_a_class_types_its_constructor_1049() {
    // TP — `interp alias {} Cat {} Dog` makes `Cat` dispatch to the class
    // command, so `Cat new` builds a Dog exactly as `Dog new` does.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; interp
    // alias {} Cat {} Dog; set d [Cat new]` returns an `::oo::Obj*` handle.
    let src = "oo::class create Dog { method bark {} { return woof } }\ninterp alias {} Cat {} Dog\nset d [Cat new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "an aliased class command still types its constructor; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_an_alias_with_a_prepended_create_still_makes_its_class_1049() {
    // `interp alias {} Cat {} Dog create` prepends a word, so `Cat new` is
    // `Dog create new`: an object command named `new`, of class `Dog`.
    //
    // tclsh 8.6.18 and 9.0.4:
    //   oo::class create Dog { method bark {} { return woof } }
    //   interp alias {} Cat {} Dog create
    //   set d [Cat new]            ;# d is ::new, [info object class $d] is ::Dog
    //   $d fly                     ;# unknown method "fly": must be bark or destroy
    let src = "oo::class create Dog { method bark {} { return woof } }\ninterp alias {} Cat {} Dog create\nset d [Cat new]\n$d fly\n";
    let r = Analyser::new().analyse(src, "tcl");
    let w308 = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W308)
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(w308.len(), 1, "{:?}", r.diagnostics);
    assert!(
        w308[0].contains("'fly'") && w308[0].contains("::Dog"),
        "{w308:?}"
    );
}

#[test]
fn w308_tp_an_alias_to_a_renamed_class_name_reaches_the_class_1049() {
    // TP — the alias's target is not a directly-live class but a *rename
    // destination*: `Cat` resolves onward to the class still keyed `::Dog`.
    // The alias hop must keep walking the chain and judge liveness where it
    // terminates, not demand a class at the hop itself.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `oo::class create Dog {…}; rename
    // Dog Cat; interp alias {} Pup {} Cat; set d [Pup new]` — `$d bark` →
    // `woof`, `$d fly` fails `unknown method "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\ninterp alias {} Pup {} Cat\nset d [Pup new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "an alias to a rename destination still reaches the class; got {:?}",
        r.diagnostics,
    );
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W307),
        "a typed dispatch must not also draw W307; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_a_chain_of_aliases_reaches_the_class_1049() {
    // TP — an alias whose target is another alias resolves hop by hop, the
    // way Tcl re-resolves each name at invocation.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `interp alias {} Cat {} Dog; interp
    // alias {} Pup {} Cat; set d [Pup new]; $d fly` fails `unknown method
    // "fly": must be bark or destroy`.
    let src = "oo::class create Dog { method bark {} { return woof } }\ninterp alias {} Cat {} Dog\ninterp alias {} Pup {} Cat\nset d [Pup new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a chain of aliases still reaches the class; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tn_an_alias_to_the_vacated_source_name_stays_untyped_1049() {
    // TN — the guard the chain walk must not lose: an alias pointing at a
    // rename *source* lands on a vacated name. The class survives under
    // `::Cat`, but the alias re-resolves `Dog` by name at each call and
    // finds nothing — the lenient rename-source rule that keeps existing
    // objects typed must not resurrect the name for an alias.
    //
    // Oracle (tclsh8.6.14 / tclsh9.0.4): `rename Dog Cat; interp alias {}
    // Pup {} Dog; Pup new` fails `invalid command name "Dog"`.
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\ninterp alias {} Pup {} Dog\nset d [Pup new]\n$d fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W308),
        "an alias to a vacated name resolves to nothing and must not type; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_tp_a_renamed_class_types_a_direct_constructor_dispatch_1049() {
    // TP — the same fix at the direct-dispatch site (`[Cat new] fly`), which
    // types the constructor result inline rather than through a variable.
    let src =
        "oo::class create Dog { method bark {} { return woof } }\nrename Dog Cat\n[Cat new] fly\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a direct dispatch on a renamed constructor still validates; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_fp_issue_1010_reestablished_class_constructor_still_flags_unknown_method() {
    let src = "oo::class create Dog { method bark {} { return woof } }\nrename Dog {}\noo::class create Dog { method bark {} { return woof } }\n[Dog new] fly";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == DiagCode::W308 && d.message.contains("fly")),
        "a class re-established after deletion must still resolve its constructor normally; got {:?}",
        r.diagnostics,
    );
}

#[test]
fn w308_transitive_alias_of_constructor_object() {
    // `set b $a` copies the OBJECT(Dog) type through the lattice, so the
    // unknown method on `$b` is validated (W308).
    let src = "oo::class create Dog { method bark {} {return woof} }\n\
                   set a [Dog new]\nset b $a\n$b fly";
    assert_eq!(w30x_codes(src), vec!["W308".to_string()]);
}

#[test]
fn w308_transitive_alias_known_method_silent() {
    let src = "oo::class create Dog { method bark {} {return woof} }\n\
                   set a [Dog new]\nset b $a\n$b bark";
    assert_eq!(w30x_codes(src), [] as [std::string::String; 0]);
}

#[test]
fn w308_constructor_create_named_and_auto() {
    // `Dog create rex` / `Dog create %AUTO%` are constructor spellings too.
    for src in [
        "oo::class create Dog { method bark {} {return woof} }\nset a [Dog create rex]\n$a fly",
        "oo::class create Dog { method bark {} {return woof} }\nset a [Dog create %AUTO%]\n$a fly",
    ] {
        assert_eq!(w30x_codes(src), vec!["W308".to_string()], "src: {src}");
    }
}

#[test]
fn w308_namespace_scoped_class_constructor() {
    // Class defined in a namespace; the relative constructor head inside
    // that namespace resolves to the qualified class (OBJECT(::ns::Dog)).
    let qualified = "namespace eval ns { oo::class create Dog { method bark {} {return woof} } }\n\
                         set a [ns::Dog new]\n$a fly";
    assert_eq!(w30x_codes(qualified), vec!["W308".to_string()]);
    let relative = "namespace eval ns {\n oo::class create Dog { method bark {} {return woof} }\n \
                        proc mk {} { set a [Dog new]; $a fly }\n}";
    assert_eq!(w30x_codes(relative), vec!["W308".to_string()]);
}

#[test]
fn w307_suppressed_for_object_returning_proc_factory() {
    // A proc returning `[Dog new]` is an object factory; a `$o method`
    // dispatch on its result suppresses W307.  The object-type lattice's
    // proc-return edge is a W308 input, so the factory-returned handle
    // also carries its *class*: a known method
    // stays silent and an unknown one is validated as W308 — the same
    // answer hover / go-to-definition give for `o`.
    let known = "oo::class create Dog { method bark {} {return woof} }\n\
                 proc mk {} { return [Dog new] }\nset o [mk]\n$o bark";
    assert_eq!(
        w30x_codes(known).len(),
        0,
        "factory-return dispatch of a declared method must be silent; got {:?}",
        w30x_codes(known)
    );
    let unknown = "oo::class create Dog { method bark {} {return woof} }\n\
                   proc mk {} { return [Dog new] }\nset o [mk]\n$o fly";
    assert_eq!(
        w30x_codes(unknown),
        vec!["W308".to_owned()],
        "an unknown method on the factory-typed handle is W308, not W307"
    );
}

#[test]
fn w307_nested_command_sub_dispatch_counts_for_multidispatch() {
    // `$x` is dispatched once at statement level and once inside a `[…]`
    // command substitution.  The substitution dispatch must be recorded as
    // a var-command site too, so the multi-dispatch (≥2) suppression sees
    // both and stays silent (a single recorded dispatch would fire W307).
    // #2427: a bare undefined command no longer widens the command table; an autoload index entry in view keeps this device unbounded.
    let src = "set ::auto_index(getCmd) {source lib.tcl}\nset x [getCmd]\nputs [$x foo]\n$x foo\n";
    assert_eq!(
        w30x_codes(src).len(),
        0,
        "multi-dispatch (one nested in `[…]`) must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn w308_double_colon_oo_class_constructor() {
    // `::oo::class` is the fully-qualified spelling of `oo::class`; the
    // class must be recognised so the constructor types as OBJECT and the
    // method is validated (W308 for the unknown one, silence for the known).
    let unknown = "::oo::class create ::Dog { method bark {} {return woof} }\n[::Dog new] fly";
    assert_eq!(w30x_codes(unknown), vec!["W308".to_string()]);
    let known = "::oo::class create ::Dog { method bark {} {return woof} }\n[::Dog new] bark";
    assert_eq!(w30x_codes(known), [] as [std::string::String; 0]);
}

// E001 (`TclOO` form): `$obj` invoked with no method word at all.

#[test]
fn tp_e001_bare_tcloo_object_dispatch_requires_method() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // `set o [Dog new]; $o` — tclsh 9.0.4: `wrong # args: should be "o
    // method ?arg ...?"`. The dispatcher checks argument count before any
    // method lookup, so this is unconditional — unlike an unknown *named*
    // method (W308), there is no `unknown`-handler fallback that could
    // save it.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               set o [Dog new]\n$o\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let e001: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::E001)
        .collect();
    assert_eq!(
        e001.len(),
        1,
        "expected one E001 for bare `$o`: {:?}",
        r.diagnostics
    );
    assert_eq!(e001[0].message, "'o' requires a method");
    assert_eq!(e001[0].severity, Severity::Error);
}

#[test]
fn tp_e001_bare_tcloo_object_dispatch_named_constructor() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // `Dog create rex` names the instance directly; `$rex` bare still
    // requires a method.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               set rex [Dog create rex]\n$rex\n";
    assert!(
        has_code(src, "tcl8.6", "E001"),
        "bare dispatch on a named-constructor instance should require a method"
    );
}

#[test]
fn tp_e001_bare_dispatch_fires_even_when_class_is_ambiguous() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // Unlike per-method arity (which abstains under ambiguity — see
    // `tcloo_method_arity_abstains_for_ambiguous_receiver_class`), a
    // bare dispatch's "missing method" failure is universal across every
    // `TclOO` instance regardless of which candidate class it resolves
    // to, so ambiguity between two known `TclOO` classes does not
    // suppress it.
    let src = "\
oo::class create A { method same {x} {} }
oo::class create B { method same {x y} {} }
if {$cond} { set f1 [A new] } else { set f1 [B new] }
$f1
";
    assert!(
        has_code(src, "tcl", "E001"),
        "bare dispatch should still fire regardless of which known TclOO class it is: {:?}",
        {
            let mut a = Analyser::new();
            a.analyse(src, "tcl").diagnostics
        }
    );
}

#[test]
fn fp_e001_bare_snit_instance_dispatch_not_flagged() {
    // snit's generated dispatcher proc is a different mechanism this
    // analyser does not model precisely enough to make the same
    // guarantee — must not assume it shares `TclOO`'s unconditional
    // "wrong # args" behaviour on a bare call.
    let src = "snit::type Dog { method bark {} { return woof } }\nDog t\n$t\n";
    assert!(
        !has_code(src, "tcl", "E001"),
        "bare snit instance dispatch must not fire E001 (unmodelled dispatcher)"
    );
}

#[test]
fn tn_e001_bare_dispatch_silent_when_method_present() {
    // Sanity: adding the bare-dispatch check must not leak into the
    // ordinary `$obj method` shape — that path stays W308's job.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               set o [Dog new]\n$o bark\n";
    assert!(!has_code(src, "tcl", "E001"));
}

#[test]
fn tn_e001_bare_dispatch_silent_for_unclassified_variable() {
    // A bare `$x` where `x` is never proven to hold a `TclOO` object
    // takes the ordinary W307 (non-literal command) path, not this
    // `TclOO`-specific E001 — the two must not overlap.
    let src = "proc walk {x} { $x }\n";
    assert!(!has_code(src, "tcl", "E001"));
}

#[test]
fn tp_e001_bare_command_substitution_head() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // `[Dog new]` used directly
    // as a command runs `Dog new`, then invokes the produced object with no
    // method word — tclsh 9.0.3/9.0.4 fail with `wrong # args: should be
    // "::oo::Obj… method ?arg ...?"` (`-errorcode {TCL WRONGARGS}`).
    let src = "oo::class create Dog { method bark {} { return woof } }\n[Dog new]\n";
    assert!(
        has_code(src, "tcl8.6", "E001"),
        "a bare `[Dog new]` command-substitution head must fire E001"
    );
}

#[test]
fn tp_e001_bare_cmd_head_named_constructor_and_alias() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // `[Dog create rex]` returns the object command too, and an
    // `interp alias` / `rename` reaching the class is the same constructor
    // (the indirection resolver types it) — both bare uses fail identically.
    let named = "oo::class create Dog { method bark {} { return woof } }\n[Dog create rex]\n";
    assert!(
        has_code(named, "tcl8.6", "E001"),
        "bare `[Dog create rex]` head must fire E001"
    );
    let renamed = "oo::class create Dog { method bark {} { return woof } }\n\
                   rename Dog Cat\n[Cat new]\n";
    assert!(
        has_code(renamed, "tcl8.6", "E001"),
        "bare `[Cat new]` after `rename Dog Cat` must fire E001"
    );
}

#[test]
fn tp_e001_bare_cmd_head_factory_proc_return() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // The proc-return flow: the lattice proves `make` object-returning
    // (`ObjectHandleFacts::returns_object`), so a bare `[make]` is the same
    // zero-word TclOO dispatch failure.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               proc make {} { return [Dog new] }\n\
               [make]\n";
    assert!(
        has_code(src, "tcl", "E001"),
        "bare `[make]` on a proven object-returning factory must fire E001"
    );
}

#[test]
fn tn_e001_cmd_head_with_method_word_stays_silent() {
    // `[Dog new] bark` has a method word — the W308 named-method path, not
    // E001; a valid method draws nothing at all.
    let src = "oo::class create Dog { method bark {} { return woof } }\n[Dog new] bark\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| matches!(d.code, DiagCode::E001 | DiagCode::W307)),
        "`[Dog new] bark` is a well-formed dispatch; got {:?}",
        r.diagnostics
    );
}

#[test]
fn fn_e001_cmd_head_factory_with_method_word_no_w307() {
    // The invariant on the cmd-head shape: the lattice
    // resolves `[make]`'s class, so `[make] bark` must be validated (and
    // stay silent), never draw the W307 abstention warning.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               proc make {} { return [Dog new] }\n\
               [make] bark\n";
    assert_eq!(
        w30x_codes(src).len(),
        0,
        "`[make] bark` resolves through the factory return type; got {:?}",
        w30x_codes(src)
    );
    let bogus = "oo::class create Dog { method bark {} { return woof } }\n\
                 proc make {} { return [Dog new] }\n\
                 [make] fly\n";
    assert!(
        has_code(bogus, "tcl", "W308"),
        "`[make] fly` must be validated against ::Dog and fire W308"
    );
}

#[test]
fn tn_e001_cmd_head_non_object_and_unknown_stay_silent() {
    // A non-object substitution result and a genuinely unknown one must not
    // fire E001 (the unknown one keeps its ordinary W307 abstention story).
    for src in [
        // Known non-object return type.
        "[string length abc]\n",
        // Same-named user proc, no TclOO class in sight.
        "proc Dog {args} { return 1 }\n[Dog new]\n",
        // Dynamic head — nothing provable.
        "set h puts\n[$h hello]\n",
    ] {
        assert!(
            !has_code(src, "tcl", "E001"),
            "`{src}` must not fire the TclOO bare-dispatch E001"
        );
    }
}

#[test]
fn tn_e001_abstains_inside_a_child_interpreter_body() {
    // A child interpreter has its own command table: `Dog` does not exist
    // there, so the real failure is `invalid command name "Dog"` in the
    // child — main-interp object semantics must not be asserted (E001
    // abstains, both the cmd-head and the `$var` forms).
    for src in [
        "oo::class create Dog { method bark {} { return woof } }\n\
         interp create sub\n\
         interp eval sub {[Dog new]}\n",
        "oo::class create Dog { method bark {} { return woof } }\n\
         interp create -safe s\n\
         s eval {[Dog new]}\n",
        "oo::class create Dog { method bark {} { return woof } }\n\
         interp create sub\n\
         interp eval sub {set o [Dog new]\n$o}\n",
    ] {
        assert!(
            !has_code(src, "tcl", "E001"),
            "`{src}` runs in a child interpreter; E001 must abstain"
        );
    }
}

#[test]
fn tn_e001_abstains_on_a_deleted_class() {
    // `rename Dog {}` deletes the class command; `[Dog new]` then fails
    // `invalid command name` (W123's story), so no bare-dispatch E001.
    let src = "oo::class create Dog { method bark {} { return woof } }\n\
               rename Dog {}\n\
               [Dog new]\n";
    assert!(
        !has_code(src, "tcl", "E001"),
        "a deleted class cannot produce the object; E001 must abstain"
    );
}

#[test]
fn tn_e001_cmd_head_snit_stays_silent() {
    // snit's generated dispatcher is a different mechanism — same carve-out
    // as the `$var` path (FP-OBJ-05).
    let src = "snit::type Dog { method bark {} { return woof } }\n[Dog create t]\n";
    assert!(
        !has_code(src, "tcl", "E001"),
        "bare snit `[Dog create t]` must not fire the TclOO E001"
    );
}

#[test]
fn e001_cmd_head_matches_var_path_under_tcloo_less_dialect() {
    // Dialect availability is W002's job: under tcl8.4 the `oo::class`
    // definition itself is flagged (`disabled in the active dialect
    // profile`), and the dispatch checks keep modelling the TclOO semantics
    // of the classes the file declares — the `$var` path has always fired
    // E001 here, and the cmd-head path must agree with it rather than
    // silently diverge on the same document.
    let cmd_head = "oo::class create Dog { method bark {} { return woof } }\n[Dog new]\n";
    let var_path = "oo::class create Dog { method bark {} { return woof } }\n\
                    set o [Dog new]\n$o\n";
    assert_eq!(
        has_code(cmd_head, "tcl8.4", "E001"),
        has_code(var_path, "tcl8.4", "E001"),
        "cmd-head and $var bare-dispatch must agree under tcl8.4"
    );
    assert!(
        has_code(cmd_head, "tcl8.4", "W002"),
        "the definition itself carries the dialect-availability diagnostic"
    );
}

#[test]
fn w307_suppressed_for_method_return_captured_handle() {
    // `set b [$a make]` — a handle returned by a
    // `$var`-dispatched method and captured into a variable.  The lattice's
    // method-return edge types `b`, so the re-dispatch draws no W307 (the
    // information hover / go-to-definition already surface).
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
               oo::class create B { method greet {} { ::return \"hi\" } }\n\
               set a [A new]\n\
               set b [$a make]\n\
               $b greet\n";
    assert_eq!(
        w30x_codes(src).len(),
        0,
        "the method-return-captured handle must suppress W307; got {:?}",
        w30x_codes(src)
    );
}

#[test]
fn w308_fires_for_bogus_method_on_method_return_captured_handle() {
    // The positive half of the same edge: once `b` is provably a ::B, an
    // unknown method is W308 — the diagnostic and hover agree on the class.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
               oo::class create B { method greet {} { ::return \"hi\" } }\n\
               set a [A new]\n\
               set b [$a make]\n\
               $b fly\n";
    assert_eq!(
        w30x_codes(src),
        vec!["W308".to_owned()],
        "an unknown method on the lattice-typed handle is W308, not W307"
    );
}

#[test]
fn tp_e001_bare_dispatch_on_method_return_captured_handle() {
    // naming.diagnostics.retained-logical-class-family
    // docs/design/analysis/name-resolution-proofs/diagnostic-retained-logical-class-family.md
    // Genuine Logical object facts and observed family advice; no Native activation grant.
    // The same lattice typing makes a bare `$b` the unconditional TclOO
    // zero-word failure.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
               oo::class create B { method greet {} { ::return \"hi\" } }\n\
               set a [A new]\n\
               set b [$a make]\n\
               $b\n";
    assert!(
        has_code(src, "tcl8.6", "E001"),
        "a bare dispatch on the method-return-typed handle must fire E001"
    );
}
