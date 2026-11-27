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
//! types, and commits a list rep. Measured on tclsh 9.0.4; 8.4.20 through
//! 9.1.0 agree.

use std::rc::Rc;

use tcl_host_native::NativeHost;
use tcl_runtime::interp::{Code, Interp};

fn run(script: &str) -> String {
    let mut interp = Interp::with_native_core(
        Rc::new(NativeHost::new()),
        tcl_registry::model::resolve_environment("tcl9.0").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("actual Tcl 9.0 native core bootstrap");
    let code = interp.eval_str(script.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    assert_eq!(code, Code::Ok, "{script} → {result}");
    result
}

#[test]
fn compiled_caches_convert_to_a_list() {
    assert_eq!(
        run(r"set b {expr {1}}; eval $b
            set e {1+2}; expr $e
            set r {a b}; regexp $r x
            list [llength $b] [llength $e] [llength $r] \
                [lindex [tcl::unsupported::representation $b] 3]"),
        "2 1 2 list"
    );
}

#[test]
fn a_lambda_reads_the_length_of_its_own_body() {
    assert_eq!(
        run(
            r#"set t {apply {n {expr {[llength [lindex $::t 1 1 1]] + $n}}}}
            list [{*}$t 5] [regsub -command -all {\d+} "12=4" $t]"#
        ),
        "13 20=12"
    );
}
