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

//! Builtins a release hides stay hidden through rename, children, imports and aliases; `interp hide` and `expose` collisions keep bindings and traces.

use super::*;

/// A release-hidden builtin must stay hidden when Tcl code tries to move it
/// out of the registry's namespace.  `rename` and `interp hide` both remove a
/// command table entry before restoring it under another spelling; that
/// removal must consult the same surface gate as ordinary dispatch.
#[test]
fn hidden_builtin_cannot_escape_through_rename_or_hide() {
    let src = concat!(
        "puts \"direct=[catch {lassign {a b} x y} msg];$msg\"\n",
        "puts \"rename=[catch {rename lassign la} msg];$msg\"\n",
        "puts \"alias=[catch {la {a b} x y} msg];$msg\"\n",
        "puts \"hide=[catch {interp hide {} lassign} msg];$msg\"\n",
        "puts \"expose=[catch {interp expose {} lassign la} msg];$msg\"\n",
        "puts \"after-hide=[catch {la {a b} x y} msg];$msg\"\n",
        "puts \"vars=[info exists x],[info exists y]\"\n",
    );
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        concat!(
            "direct=1;invalid command name \"lassign\"\n",
            "rename=1;can't rename \"lassign\": command doesn't exist\n",
            "alias=1;invalid command name \"la\"\n",
            "hide=1;unknown command \"lassign\"\n",
            "expose=1;unknown hidden command \"lassign\"\n",
            "after-hide=1;invalid command name \"la\"\n",
            "vars=0,0",
        )
    );
    assert_eq!(
        vm_output(src, TclVersion::V9_0),
        concat!(
            "direct=0;\n",
            "rename=0;\n",
            "alias=0;\n",
            "hide=1;unknown command \"lassign\"\n",
            "expose=1;exposed command \"la\" already exists\n",
            "after-hide=0;\n",
            "vars=1,1",
        )
    );
}

/// Every public command-table mutation and indirection preserves the release
/// surface. Child-interpreter hide must not mutate the parked table directly,
/// and import must not clone a builtin without its final source identity —
/// either would expose `lassign` to an emulated 8.4 VM.
#[test]
fn hidden_builtin_stays_hidden_through_children_imports_and_aliases() {
    let src = concat!(
        "interp create child\n",
        "namespace export lassign\n",
        "namespace eval imported {namespace import ::lassign}\n",
        "puts \"same-hide=[catch {interp hide {} lassign la} m];$m\"\n",
        "puts \"same-hidden=[info commands la]\"\n",
        "puts \"same-expose=[catch {interp expose {} la lassign2} m];$m\"\n",
        "puts \"same-call=[catch {lassign2 {a b} x} m];$m\"\n",
        "puts \"child-hide=[catch {interp hide child lassign la} m];$m\"\n",
        "puts \"child-hidden=[child hidden]\"\n",
        "puts \"child-expose=[catch {interp expose child la lassign2} m];$m\"\n",
        "puts \"child-call=[catch {child eval {lassign2 {a b} x}} m];$m\"\n",
        "interp create child_command\n",
        "puts \"child-command-hide=[catch {child_command hide lassign} m];$m\"\n",
        "puts \"child-command-hidden=[child_command hidden]\"\n",
        "puts \"child-command-expose=[catch {child_command expose lassign} m];$m\"\n",
        "puts \"child-command-call=[catch {child_command eval {lassign {a b} x}} m];$m\"\n",
        "puts \"import-visible=[namespace eval imported {info commands lassign}]\"\n",
        "puts \"import-origin=[catch {namespace eval imported {namespace origin lassign}} m];$m\"\n",
        "puts \"import-call=[catch {namespace eval imported {lassign {a b} x}} m];$m\"\n",
        "interp alias {} alias {} lassign\n",
        "puts \"alias-call=[catch {alias {a b} x} m];$m\"\n",
    );
    assert_eq!(
        vm_output(src, TclVersion::V8_4),
        concat!(
            "same-hide=1;unknown command \"lassign\"\n",
            "same-hidden=\n",
            "same-expose=1;unknown hidden command \"la\"\n",
            "same-call=1;invalid command name \"lassign2\"\n",
            "child-hide=1;unknown command \"lassign\"\n",
            "child-hidden=\n",
            "child-expose=1;unknown hidden command \"la\"\n",
            "child-call=1;invalid command name \"lassign2\"\n",
            "child-command-hide=1;unknown command \"lassign\"\n",
            "child-command-hidden=\n",
            "child-command-expose=1;unknown hidden command \"lassign\"\n",
            "child-command-call=1;invalid command name \"lassign\"\n",
            "import-visible=\n",
            "import-origin=1;invalid command name \"lassign\"\n",
            "import-call=1;invalid command name \"lassign\"\n",
            "alias-call=1;invalid command name \"lassign\"",
        )
    );
    let want_newer = concat!(
        "same-hide=0;\n",
        "same-hidden=\n",
        "same-expose=0;\n",
        "same-call=0;b\n",
        "child-hide=0;\n",
        "child-hidden=la\n",
        "child-expose=0;\n",
        "child-call=0;b\n",
        "child-command-hide=0;\n",
        "child-command-hidden=lassign\n",
        "child-command-expose=0;\n",
        "child-command-call=0;b\n",
        "import-visible=lassign\n",
        "import-origin=0;::lassign2\n",
        "import-call=0;b\n",
        "alias-call=1;invalid command name \"lassign\"",
    );
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        assert_eq!(vm_output(src, version), want_newer, "[{version:?}] surface");
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want_newer, "[{env}] real Tcl oracle");
        }
    }
}

/// Hide/expose collisions reject before moving either binding.  The visible
/// destination keeps both command and execution traces, and the hidden token
/// remains invocable, matching Tcl's non-destructive failure contract.
#[test]
fn hide_and_expose_collisions_preserve_bindings_and_traces() {
    let src = concat!(
        "proc a {} {return A}; proc b {} {return B}\n",
        "proc cb args {lappend ::events [lindex $args end]}\n",
        "trace add command b delete cb; trace add execution b enter cb\n",
        "interp hide {} a token\n",
        "puts \"hide=[catch {interp hide {} b token} m];$m\"\n",
        "puts \"expose=[catch {interp expose {} token b} m];$m\"\n",
        "interp expose {} token restored\n",
        "puts \"visible=[b] hidden=[restored] traces=[llength [trace info command b]],[llength [trace info execution b]] events=$::events\"\n",
    );
    let want = concat!(
        "hide=1;hidden command named \"token\" already exists\n",
        "expose=1;exposed command \"b\" already exists\n",
        "visible=B hidden=A traces=1,1 events=enter",
    );
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] hide collision"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

#[test]
fn child_command_hide_and_expose_collisions_propagate_without_mutation() {
    let src = concat!(
        "interp create kid\n",
        "kid eval {proc a {} {return A}; proc cb args {lappend ::events [lindex $args end]}}\n",
        "kid hide a\n",
        "kid eval {proc a {} {return B}; trace add command a delete cb; trace add execution a enter cb}\n",
        "puts \"hide=[catch {kid hide a} m];$m\"\n",
        "puts \"expose=[catch {kid expose a} m];$m\"\n",
        "puts \"state=[kid eval {list [a] [llength [trace info command a]] [llength [trace info execution a]]}]\"\n",
    );
    let want = concat!(
        "hide=1;hidden command named \"a\" already exists\n",
        "expose=1;exposed command \"a\" already exists\n",
        "state=B 1 1",
    );
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] child hide collision"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

/// `interp expose`'s destination is always the exact global command-table
/// key.  A caller may have a same-spelled local command, which Tcl leaves
/// untouched while exposing the hidden binding globally.  Exercise the root,
/// named-child, and child-command (by-id) entry points so each routes through
/// the same collision owner rather than contextual command lookup.
#[test]
fn expose_uses_exact_global_destination_in_every_entry_point() {
    let src = concat!(
        "proc a {} {return root-global}; interp hide {} a held\n",
        "namespace eval n {proc b {} {return root-local}; interp expose {} held b; puts \"root=[b],[::b]\"}\n",
        "interp create named; named eval {proc a {} {return named-global}; interp hide {} a held; namespace eval n {proc b {} {return named-local}; interp expose {} held b; puts \"named=[b],[::b]\"}}\n",
        "interp create byid; byid eval {proc a {} {return byid-global}; interp hide {} a; namespace eval n {proc a {} {return byid-local}}}\n",
        "byid expose a; puts \"byid=[byid eval {namespace eval n {join [list [a] [::a]] ,}}]\"\n",
    );
    let want =
        "root=root-local,root-global\nnamed=named-local,named-global\nbyid=byid-local,byid-global";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] exact expose destination"
        );
    }
    for (env, names) in [
        ("TCLSH86", &["tclsh8.6"][..]),
        ("TCLSH90", &["tclsh9.0"][..]),
    ] {
        if let Some(out) = tclsh_output(env, names, src) {
            assert_eq!(out, want, "[{env}] real Tcl oracle");
        }
    }
}

#[test]
fn hide_expose_moves_traces_without_callbacks() {
    let src = concat!(
        "proc a {} {return A}; proc cb args {lappend ::events [lindex $args end]}\n",
        "trace add command a {rename delete} cb; trace add execution a enter cb\n",
        "interp hide {} a token\n",
        "interp expose {} token moved\n",
        "puts \"state=[moved] [llength [trace info command moved]] [llength [trace info execution moved]] $::events\"\n",
    );
    let want = "state=A 1 1 enter";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] hide trace move"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

#[test]
fn root_invokehidden_dispatches_without_exposing_the_command() {
    let src = concat!(
        "proc a {} {return A}\n",
        "interp hide {} a held\n",
        "puts \"hidden=[interp invokehidden {} held]\"\n",
        "puts \"visible=[llength [info commands held]] listed=[interp hidden {}]\"\n",
    );
    let want = "hidden=A\nvisible=0 listed=held";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] root invokehidden"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}

#[test]
fn child_hide_expose_moves_traces_for_named_and_by_id_forms() {
    let src = concat!(
        "interp create named; interp create byid\n",
        "named eval {proc a {} {return N}; proc cb args {lappend ::events [lindex $args end]}; trace add execution a enter cb}\n",
        "interp hide named a held; puts \"named-hidden=[interp invokehidden named held]\"; interp expose named held moved\n",
        "puts \"named=[named eval {list [moved] [llength [trace info execution moved]] $::events}]\"\n",
        "byid eval {proc a {} {return I}; proc cb args {lappend ::events [lindex $args end]}; trace add execution a enter cb}\n",
        "byid hide a; puts \"byid-hidden=[byid invokehidden a]\"; byid expose a\n",
        "puts \"byid=[byid eval {list [a] [llength [trace info execution a]] $::events}]\"\n",
    );
    let want = "named-hidden=N\nnamed=N 1 {enter enter}\nbyid-hidden=I\nbyid=I 1 {enter enter}";
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        assert_eq!(
            vm_output(src, version),
            want,
            "[{version:?}] child trace move"
        );
    }
    if let Some(out) = tclsh_output("TCLSH90", &["tclsh9.0"], src) {
        assert_eq!(out, want, "[TCLSH90] real Tcl oracle");
    }
}
