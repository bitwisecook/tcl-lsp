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

//! Commands hidden from view: `interp hide` and `interp expose` failures, and the release gate on the `TclOO` root commands.

use super::*;

#[test]
fn interp_hide_and_expose_failures_keep_typed_tcl_identities() {
    // Exercise the current interpreter, a named child, and the child's own
    // command entry point. Names with spaces also prove that lookup codes are
    // encoded as Tcl lists rather than joined diagnostic strings. Exact Tcl
    // 9.0.4 oracle result.
    assert_eq!(
        run("proc outcome script {
                 set code [catch {uplevel 1 $script} message options]
                 list $code $message [dict get $options -errorcode]
             }
             set out {}
             lappend out [outcome {interp hide {} {not here}}]
             lappend out [outcome {interp expose {} {not hidden}}]
             proc p {} {return P}
             interp hide {} p held
             proc q {} {return Q}
             lappend out [outcome {interp hide {} q held}]
             lappend out [outcome {interp expose {} held q}]
             interp create kid
             lappend out [outcome {interp hide kid {not here}}]
             lappend out [outcome {interp expose kid {not hidden}}]
             kid eval {
                 proc p {} {return P}
                 interp hide {} p held
                 proc q {} {return Q}
             }
             lappend out [outcome {interp hide kid q held}]
             lappend out [outcome {interp expose kid held q}]
             lappend out [outcome {kid hide {not here}}]
             lappend out [outcome {kid expose {not hidden}}]
             kid eval {
                 proc a {} {return A}
                 interp hide {} a
                 proc a {} {return NEW}
             }
             lappend out [outcome {kid hide a}]
             lappend out [outcome {kid expose a}]
             set out"),
        "{1 {unknown command \"not here\"} {TCL LOOKUP COMMAND {not here}}} \
         {1 {unknown hidden command \"not hidden\"} {TCL LOOKUP HIDDENTOKEN {not hidden}}} \
         {1 {hidden command named \"held\" already exists} {TCL HIDE ALREADY_HIDDEN}} \
         {1 {exposed command \"q\" already exists} {TCL EXPOSE COMMAND_EXISTS}} \
         {1 {unknown command \"not here\"} {TCL LOOKUP COMMAND {not here}}} \
         {1 {unknown hidden command \"not hidden\"} {TCL LOOKUP HIDDENTOKEN {not hidden}}} \
         {1 {hidden command named \"held\" already exists} {TCL HIDE ALREADY_HIDDEN}} \
         {1 {exposed command \"q\" already exists} {TCL EXPOSE COMMAND_EXISTS}} \
         {1 {unknown command \"not here\"} {TCL LOOKUP COMMAND {not here}}} \
         {1 {unknown hidden command \"not hidden\"} {TCL LOOKUP HIDDENTOKEN {not hidden}}} \
         {1 {hidden command named \"a\" already exists} {TCL HIDE ALREADY_HIDDEN}} \
         {1 {exposed command \"a\" already exists} {TCL EXPOSE COMMAND_EXISTS}}"
    );
}

#[test]
fn interp_visibility_qualifier_validation_is_shared_by_every_entry_form() {
    // The one-word `$child` shorthand uses that word for both roles. Hide
    // diagnoses the hidden-token role first; expose diagnoses its visible
    // destination first. Explicit forms also validate each distinct role.
    // Exact Tcl 9.0.4 oracle result.
    assert_eq!(
        run("proc outcome script {
                 set code [catch {uplevel 1 $script} message options]
                 list $code $message [dict get $options -errorcode]
             }
             proc p {} {return P}
             namespace eval N {proc p {} {return NP}}
             interp create kid
             kid eval {
                 proc p {} {return P}
                 namespace eval N {proc p {} {return NP}}
             }
             list \
                 [outcome {interp hide {} N::p}] \
                 [outcome {interp expose {} N::p}] \
                 [outcome {interp hide kid N::p}] \
                 [outcome {interp expose kid N::p}] \
                 [outcome {kid hide N::p}] \
                 [outcome {kid expose N::p}] \
                 [outcome {interp hide kid p N::held}] \
                 [outcome {interp expose kid N::held q}] \
                 [outcome {interp hide kid N::p held}] \
                 [outcome {interp expose kid held N::q}]"),
        "{1 {cannot use namespace qualifiers in hidden command token (rename)} {TCL VALUE HIDDENTOKEN}} {1 {cannot expose to a namespace (use expose to toplevel, then rename)} {TCL EXPOSE NON_GLOBAL}} {1 {cannot use namespace qualifiers in hidden command token (rename)} {TCL VALUE HIDDENTOKEN}} {1 {cannot expose to a namespace (use expose to toplevel, then rename)} {TCL EXPOSE NON_GLOBAL}} {1 {cannot use namespace qualifiers in hidden command token (rename)} {TCL VALUE HIDDENTOKEN}} {1 {cannot expose to a namespace (use expose to toplevel, then rename)} {TCL EXPOSE NON_GLOBAL}} {1 {cannot use namespace qualifiers in hidden command token (rename)} {TCL VALUE HIDDENTOKEN}} {1 {unknown hidden command \"N::held\"} {TCL LOOKUP HIDDENTOKEN N::held}} {1 {can only hide global namespace commands (use rename then hide)} {TCL HIDE NON_GLOBAL}} {1 {cannot expose to a namespace (use expose to toplevel, then rename)} {TCL EXPOSE NON_GLOBAL}}"
    );
}

// The availability gate covers the TclOO root object commands

#[test]
fn tcloo_roots_follow_their_introducing_release() {
    // Real tclsh 8.4/8.5 have no TclOO at all.
    for release in ["tcl8.4", "tcl8.5"] {
        assert_eq!(
            run_at("catch {oo::class create C {}} m; set m", release),
            "invalid command name \"oo::class\"",
            "{release}"
        );
        assert_eq!(
            run_at("catch {oo::object new} m; set m", release),
            "invalid command name \"oo::object\"",
            "{release}"
        );
    }
    // 8.6 has oo::class but not TIP 558's oo::configurable.
    assert_eq!(
        run_at("catch {oo::configurable create C {}} m; set m", "tcl8.6"),
        "invalid command name \"oo::configurable\""
    );
    assert_eq!(run_at("oo::class create D {}", "tcl8.6"), "::D");
    assert_eq!(run_at("oo::configurable create C {}", "tcl9.0"), "::C");
}

#[test]
fn a_script_created_object_is_release_invariant() {
    // The gate must consult the registry for the engine-installed roots only:
    // an object command that happens to share a registry name is user code.
    assert_eq!(
        run_at(
            "oo::class create lpop {method m {} {return M}}\n[lpop new] m",
            "tcl8.6"
        ),
        "M"
    );
}

#[test]
fn a_hidden_tcloo_root_cannot_be_renamed_back_into_view() {
    // The removal seam is gated too, so the gate cannot be walked around.
    assert_eq!(
        run_at("catch {rename oo::class fresh} m; set m", "tcl8.4"),
        "can't rename \"oo::class\": command doesn't exist"
    );
}

/// A renamed engine-installed root keeps its **registry identity**, so it is
/// still dated by the name the registry knows rather than by the name it now
/// answers to. Real tclsh cannot switch release mid-session, so this pins the
/// invariant the gate exists to enforce rather than a tclsh transcript: the
/// same command, re-pinned to a release that has no such builtin, must
/// disappear exactly as it would have under its original name.
#[test]
fn a_renamed_tcloo_root_still_gates_by_its_registry_identity() {
    // `oo::configurable` is TIP 558 (9.0); renaming it does not make it an
    // 8.6 command.
    assert_eq!(
        run_flipping(&[
            (
                "tcl9.0",
                "rename oo::configurable myconf\nmyconf create C {}"
            ),
            ("tcl8.6", "catch {myconf create D {}} m; set m"),
        ]),
        "invalid command name \"myconf\""
    );
    // The rename really did work on the release that has the command…
    assert_eq!(
        run_flipping(&[(
            "tcl9.0",
            "rename oo::configurable myconf\nmyconf create C {}"
        )]),
        "::C"
    );
    // …and the vacated name is not left gated: it is the user's to take.
    assert_eq!(
        run_flipping(&[
            ("tcl9.0", "rename oo::configurable myconf"),
            (
                "tcl9.0",
                "oo::class create oo::configurable {method m {} {return MINE}}\n\
                 [oo::configurable new] m"
            ),
        ]),
        "MINE"
    );
    // A script-created object stays release-invariant across the same flip —
    // the control that says the gate is keyed on identity, not on being an
    // object command.
    assert_eq!(
        run_flipping(&[
            (
                "tcl9.0",
                "oo::class create keeper {method m {} {return KEPT}}\nkeeper create k"
            ),
            ("tcl8.6", "k m"),
        ]),
        "KEPT"
    );
}
