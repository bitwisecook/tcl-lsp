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

//! `llength` of a value whose cache is a compiled script, expression or regexp
//! converts its string, as C's `Tcl_ListObjLength` does for these sealed core
//! types: `regexp.test`'s `regexp-27.8` takes the length of a word inside the
//! lambda its own `regsub -command` is running.

use crate::common;

use tcl_dialect::TclVersion;

const COMPILED_CACHES: &str = r"set b {expr {1}}; eval $b
set e {1+2}; expr $e
set r {a b}; regexp $r x
puts [list [llength $b] [llength $e] [llength $r]]";

const LAMBDA_SELF_LENGTH: &str =
    r"set t {apply {n {expr {[llength [lindex $::t 1 1 1]] + $n}}}}; puts [{*}$t 5]";

const REGSUB_COMMAND_SELF_LENGTH: &str = r#"set t {apply {n {expr {[llength [lindex $::t 1 1 1]] + $n}}}}
puts [regsub -command -all {\d+} "12=4" $t]"#;

fn matches_original(source: &str, expected: &str, versions: &[TclVersion]) {
    for oracle in
        tcl_test_support::required_tclshs(versions).expect("matching original C interpreters")
    {
        assert_eq!(
            common::oracle_output(&oracle.path, source),
            expected,
            "{}: original",
            oracle.patchlevel
        );
        assert_eq!(
            common::vm_output(source, oracle.version.dialect_profile_name()),
            expected,
            "{}: compiled",
            oracle.patchlevel
        );
    }
}

#[test]
fn compiled_caches_convert_to_a_list() {
    matches_original(COMPILED_CACHES, "2 1 2", &TclVersion::ALL);
}

#[test]
fn a_lambda_reads_the_length_of_its_own_body() {
    let versions = [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ];
    matches_original(LAMBDA_SELF_LENGTH, "13", &versions);
    matches_original(
        REGSUB_COMMAND_SELF_LENGTH,
        "20=12",
        &[TclVersion::V9_0, TclVersion::V9_1],
    );
}
