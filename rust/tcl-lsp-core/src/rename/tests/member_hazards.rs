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

//! Member renames the safety gate refuses or admits: untracked receivers and scoped object commands.

use super::*;

#[test]
fn rename_rewrites_a_method_return_captured_dispatch_site() {
    // `b` is typed only by the object-type lattice's method-return edge,
    // which makes the site a proven `::B` dispatch rather than an
    // untracked receiver: the rename proceeds and rewrites it.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 29).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        29,
        "salute",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "declaration edit missing: {edits:?}");
    assert!(
        lines.contains(&4),
        "the lattice-typed `$b greet` call site must be rewritten: {edits:?}"
    );
    let result = apply_edits(src, &edits);
    assert!(
        result.contains("$b salute") && result.contains("method salute"),
        "applied rename must leave a consistent document:\n{result}"
    );
}

#[test]
fn rename_still_refuses_on_a_genuinely_untracked_receiver() {
    // The refusal gate stays honest: a `$x greet` whose receiver has no
    // binding in either map is still the untracked-receiver
    // hazard, so the rename must refuse (empty edit set) rather than
    // leave the site pointing at a dead name.
    let src = "oo::class create B { method greet {} { return \"hi\" } }\n\
                   proc use {x} { $x greet }\n\
                   set b [B new]\n\
                   $b greet\n";
    let analysis = analyse(src);
    // `x` is a proc parameter — the lattice has no caller to type it
    // from, so it stays untracked.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        28,
        "salute",
        &analysis,
        None,
    );
    assert!(
        edits.is_empty(),
        "an untracked `$x greet` dispatch must keep refusing the rename: {edits:?}"
    );
}

/// TP + TN, the object-command half.
///
/// Oracle (tclsh 9.0.4 / 8.6.16, identical): `::a::Factory create rex`
/// and `::b::Widget create rex` bind two different commands; `rex make`
/// inside `::a` prints `a-made` and inside `::b` prints `b-made`, and
/// `::a::rex make` / `::b::rex make` both work from the top level.
/// Renaming `::b::Widget::make` → `produce` and also rewriting `::a`'s
/// call site makes both interpreters fail with
/// `unknown method "produce": must be destroy or make` at `rex produce`
/// inside `::a` — which is exactly what the LSP emitted before this fix.
#[test]
fn tn_object_command_dispatch_is_scoped_to_its_creation_namespace() {
    let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory {\n\
                   \x20       method make {} { return \"a-made\" }\n\
                   \x20   }\n\
                   \x20   Factory create rex\n\
                   \x20   puts [rex make]\n\
                   }\n\
                   namespace eval ::b {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method make {} { return \"b-made\" }\n\
                   \x20   }\n\
                   \x20   Widget create rex\n\
                   \x20   puts [rex make]\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `make` in `::b::Widget`'s declaration.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        9,
        15,
        "produce",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("method produce {} { return \"b-made\" }"),
        "::b's declaration must rename: {applied}"
    );
    assert!(
        applied.contains("method make {} { return \"a-made\" }"),
        "::a's declaration must be untouched: {applied}"
    );
    // `::a`'s own call site must survive verbatim; `::b`'s must rename.
    let a_call = applied
        .split("namespace eval ::b")
        .next()
        .expect("split keeps the ::a half");
    assert!(
        a_call.contains("puts [rex make]"),
        "::a's `rex make` must not be rewritten (issue #981): {applied}"
    );
    assert!(
        applied
            .split("namespace eval ::b")
            .nth(1)
            .is_some_and(|b| b.contains("puts [rex produce]")),
        "::b's own `rex make` must rename (the finding's lost-site half): {applied}"
    );
}

/// FP guard, end of the in-document path: the untracked-receiver shape
/// refuses with a reason rather than emitting the declaration-only edit
/// set that breaks the program on both interpreters.
#[test]
fn fp_rename_refuses_the_untracked_receiver_dispatch() {
    let src = "oo::class create Vector3d {\n\
                   \x20   variable _x\n\
                   \x20   constructor {args} {\n\
                   \x20       set other [lindex $args 0]\n\
                   \x20       set _x [$other X]\n\
                   \x20   }\n\
                   \x20   method X {} { return $_x }\n\
                   }\n";
    let analysis = analyse(src);
    let err = rename_with_diagnosis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        6,
        11,
        "GetX",
        &analysis,
        None,
    )
    .expect_err("idx 79's shape must refuse");
    assert!(err.reason.contains("not tracked"), "{}", err.reason);
    assert!(err.range.is_some(), "the refusal must point at the site");
    // And the plain entry point must produce no edits at all — never a
    // partial set.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            6,
            11,
            "GetX",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

// The gate must not depend on the trigger position.  A real editor's
// "rename symbol" gesture can be invoked from any occurrence, and an
// ungated one returns, from the two positions below, a live
// WorkspaceEdit rewriting only the declaration and the `export` word.
// Applying that edit and running it (tclsh 9.0.4 and 8.6.16, byte
// identical):
//
//   unknown method "X": must be Get, GetX, Y, Z or destroy
//
// at the constructor's own `[$args X]`.  Every position that names the
// same member must therefore reach the same verdict.

/// The exact source the trigger-position tests share — the copy-
/// constructor shape from nico-robert/tomato's `Vector3d.tcl`, with the
/// `export` list the finding's `WorkspaceEdit` also rewrote.
///
/// Line/column map (0-based):
///   4  col 23 — the `X` inside `[$other X]`   (untracked call site)
///   6  col 11 — the `X` of `method X`          (declaration)
///   8  col 11 — the `X` in `export X Get`      (export bareword)
pub(super) const IDX79_HAZARD: &str = "oo::class create Vector3d {\n\
         \x20   variable _x _y\n\
         \x20   constructor {args} {\n\
         \x20       set other [lindex $args 0]\n\
         \x20       set _x [$other X]\n\
         \x20   }\n\
         \x20   method X {} { return $_x }\n\
         \x20   method Get {} { return \"$_x $_y\" }\n\
         \x20   export X Get\n\
         }\n";

/// The same class with **every** receiver tracked: `$v` is bound by
/// `Vector3d new`, so the reference scan rewrites its dispatch and there
/// is no hazard.  The true-negative control for the tests below — an
/// over-broad gate would refuse this too and make rename unusable.
///
/// Line/column map (0-based):
///   1  col 11 — the `X` of `method X`          (declaration)
///   2  col 31 — the `X` inside `[my X]`        (`my` dispatch)
///   3  col 11 — the `X` in `export X Get`      (export bareword)
///   6  col  9 — the `X` inside `[$v X]`        (tracked `$obj` dispatch)
const IDX79_SAFE: &str = "oo::class create Vector3d {\n\
         \x20   method X {} { return 1 }\n\
         \x20   method Get {} { return [my X] }\n\
         \x20   export X Get\n\
         }\n\
         set v [Vector3d new]\n\
         puts [$v X]\n";

/// Every trigger position that reaches the hazardous member refuses —
/// declaration, untracked call site, and `export` bareword alike.
#[test]
fn fp_rename_refuses_the_untracked_receiver_from_every_trigger_position() {
    let analysis = analyse(IDX79_HAZARD);
    for (line, character, what) in [
        (4, 23, "the untracked `[$other X]` call site"),
        (6, 11, "the `method X` declaration"),
        (8, 11, "the `export X Get` bareword"),
    ] {
        let err = rename_with_diagnosis(
            IDX79_HAZARD,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            line,
            character,
            "GetX",
            &analysis,
            None,
        )
        .map_or_else(
            |err| err,
            |edits| panic!("{what} must refuse; it returned {edits:?} instead"),
        );
        assert!(
            err.reason.contains("not tracked"),
            "{what}: the refusal must name the untracked receiver, got {}",
            err.reason
        );
        assert!(
            rename(
                IDX79_HAZARD,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                line,
                character,
                "GetX",
                &analysis,
                None
            )
            .is_empty(),
            "{what} must never yield a partial edit set"
        );
    }
}

/// TN control: with every receiver tracked the rename still succeeds from
/// each of the same trigger positions, and the applied result rewrites
/// the declaration, the dispatches, *and* the export word together —
/// the property that makes the edit safe to run.
#[test]
fn fn_guard_a_fully_tracked_member_renames_from_every_trigger_position() {
    let analysis = analyse(IDX79_SAFE);
    for (line, character, what) in [
        (1, 11, "the `method X` declaration"),
        (2, 31, "the `my X` dispatch"),
        (3, 11, "the `export X Get` bareword"),
        (6, 9, "the tracked `[$v X]` call site"),
    ] {
        let edits = rename_with_diagnosis(
            IDX79_SAFE,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            line,
            character,
            "GetX",
            &analysis,
            None,
        )
        .unwrap_or_else(|err| panic!("{what} must not refuse: {}", err.reason));
        let applied = apply_edits(IDX79_SAFE, &edits);
        assert!(
            applied.contains("method GetX {}"),
            "{what}: declaration must rename: {applied}"
        );
        assert!(
            applied.contains("[my GetX]"),
            "{what}: the `my` dispatch must rename: {applied}"
        );
        assert!(
            applied.contains("export GetX Get"),
            "{what}: the export word must rename: {applied}"
        );
        assert!(
            !applied.contains(" X]") && !applied.contains("method X "),
            "{what}: no occurrence of the old name may survive: {applied}"
        );
    }
}
