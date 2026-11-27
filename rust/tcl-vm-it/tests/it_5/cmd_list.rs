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

//! `lappend` and the grouped `lsort` CLI captures, run through the release compiler.

mod native_original_group_command_controls;

mod tests {
    /// A no-value `lappend` reads as `Tcl_ObjGetVar2(..., 0)`: a missing
    /// variable leaves no error code, while a reached read that fails leaves
    /// C8.6+'s value-read code. Tcl 9.1 compiles the direct `lappend missing`
    /// as a reached read. Rows from tclsh 8.5.19, 8.6.18, 9.0.4 and 9.1.0.
    #[test]
    fn empty_lappend_leaves_only_a_reached_read_error_code() {
        const SCRIPT: &str = r#"
set ::errorCode X
set c lappend
array set A {k v}
upvar 0 nosuch link
namespace eval ns {variable v}
proc code o {expr {[dict exists $o -errorcode] ? [dict get $o -errorcode] : "-"}}
proc p {} {set c lappend; list [catch {$c loc} m o] [code $o]}
list \
    [list top [catch {lappend missing} m o] [code $o]] \
    [list dyn [catch {$c plain} m o] [code $o]] \
    [list elem [catch {$c A(nk)} m o] [code $o]] \
    [list noarr [catch {$c B(nk)} m o] [code $o]] \
    [list link [catch {$c link} m o] [code $o]] \
    [list nsvar [catch {$c ns::v} m o] [code $o]] \
    [list proc [p]] $::errorCode
"#;
        let read = "{TCL READ VARNAME}";
        let expected = |top: &str, reached: &str| {
            format!(
                "{{top 0 {top}}} {{dyn 0 -}} {{elem 0 {reached}}} {{noarr 0 -}} \
                 {{link 0 {reached}}} {{nsvar 0 {reached}}} {{proc {{0 -}}}} X"
            )
        };
        for (release, wanted) in [
            ("tcl8.5", expected("-", "-")),
            ("tcl8.6", expected("-", read)),
            ("tcl9.0", expected("-", read)),
            ("tcl9.1", expected(read, read)),
        ] {
            let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(release));
            let result = vm.eval_source(SCRIPT).unwrap();
            assert_eq!(result.code, tcl_runtime_api::Code::Ok, "{release}");
            assert_eq!(result.result.to_str().as_ref(), wanted, "{release}");
        }
    }
}
