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

//! Command token identity: retained descendant tokens across an
//! activation-delayed parent deletion (#1755), and colon-edge names whose
//! rendered spellings collide (#1778).

use tcl_dialect::TclVersion;
use tcl_runtime::interp::{Code, Interp};

const SOURCE: &str = r"set answer [namespace eval ::N {
    namespace eval C {proc q {} {return OLDQ}}
    proc p {} {
        namespace delete ::N
        namespace eval ::N::C {proc q {} {return NEWQ}}
        list [C::q] [::N::C::q]
    }
    p
}]
list $answer [::N::C::q]";

/// Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: from 8.5 both
/// references keep the retained old child token while the deleting activation
/// is live; 8.4 re-resolves the absolute spelling through the recreation.
#[test]
fn compiled_descendant_command_survives_activation_delayed_parent_deletion() {
    for (version, expected) in [
        (TclVersion::V8_4, "{OLDQ NEWQ} NEWQ"),
        (TclVersion::V8_5, "{OLDQ OLDQ} NEWQ"),
        (TclVersion::V8_6, "{OLDQ OLDQ} NEWQ"),
        (TclVersion::V9_0, "{OLDQ OLDQ} NEWQ"),
        (TclVersion::V9_1, "{OLDQ OLDQ} NEWQ"),
    ] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let code = interp.eval_str(SOURCE.as_bytes());
        let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
        assert_eq!((code, result.as_str()), (Code::Ok, expected), "{version:?}");
    }
}

#[test]
fn colon_edge_command_tokens_stay_distinct() {
    // #1778. `a:`/`p`, `a`/`:p` and `a` -> `:` -> `p` are three distinct
    // commands whose rendered names collide. Measured on tclsh 8.4.20,
    // 8.5.19, 8.6.18, 9.0.4 and 9.1.0: before 8.6 a non-global `:p` cannot be
    // created; from 8.6 each relative spelling reaches its own token, the
    // absolute `::a:::::p` resolves to none of them, and deleting `a:` leaves
    // the other two.
    let src = r#"set r {}
foreach s {
    {namespace eval a: {proc p {} {return from-a-colon}}}
    {namespace eval a {proc :p {} {return from-colon-p}}}
    {namespace eval a {namespace eval : {proc p {} {return nested}}}}
    {namespace eval a: {p}}
    {namespace eval a {:p}}
    {namespace eval a {namespace eval : {p}}}
    {::a:::::p}
    {namespace delete a:}
    {namespace eval a {:p}}
    {namespace eval a {namespace eval : {p}}}
} {
    lappend r [list [catch $s m] [expr {[string match {*invalid command*} $m] ? "invalid" : $m}]]
}
set r"#;
    let legacy = r#"{0 {}} {1 {can't create procedure ":p" in non-global namespace with name starting with ":"}} {0 {}} {1 invalid} {1 invalid} {1 invalid} {0 nested} {0 {}} {1 invalid} {1 invalid}"#;
    let modern = r"{0 {}} {0 {}} {0 {}} {0 from-a-colon} {0 from-colon-p} {0 nested} {1 invalid} {0 {}} {0 from-colon-p} {0 nested}";
    for (version, expected) in [
        (TclVersion::V8_4, legacy),
        (TclVersion::V8_5, legacy),
        (TclVersion::V8_6, modern),
        (TclVersion::V9_0, modern),
        (TclVersion::V9_1, modern),
    ] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let code = interp.eval_str(src.as_bytes());
        let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
        assert_eq!((code, result.as_str()), (Code::Ok, expected), "{version:?}");
    }
}

#[test]
fn colon_edge_rename_trace_and_recreation_keep_their_own_tokens() {
    // #1778. Renaming `a`/`:p` leaves `a:`/`p` and its trace alone; deleting
    // `a:` fires only that command's trace, and recreating `a:`/`p` does not
    // disturb the renamed `a`/`:q`. tclsh 8.6.18, 9.0.4 and 9.1.0 agree;
    // 8.4 and 8.5 cannot create the non-global `:p` at all.
    let src = r#"set r {}
set ::log {}
proc rec {args} {lappend ::log [lindex $args end]}
foreach s {
    {namespace eval a: {proc p {} {return A}}}
    {namespace eval a {proc :p {} {return B}}}
    {namespace eval a: {trace add command p delete rec}}
    {namespace eval a {rename :p :q}}
    {namespace eval a: {p}}
    {namespace eval a {:q}}
    {namespace eval a {:p}}
    {namespace delete a:}
    {set ::log}
    {namespace eval a: {proc p {} {return C}}}
    {namespace eval a: {p}}
    {namespace eval a {:q}}
} {
    lappend r [list [catch $s m] [expr {[string match {*invalid command*} $m] ? "invalid" : $m}]]
}
set r"#;
    for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let code = interp.eval_str(src.as_bytes());
        let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
        assert_eq!(
            (code, result.as_str()),
            (
                Code::Ok,
                r"{0 {}} {0 {}} {0 {}} {0 {}} {0 A} {0 B} {1 invalid} {0 {}} {0 delete} {0 {}} {0 C} {0 B}"
            ),
            "{version:?}"
        );
    }
}
