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

//! 8.4 refuses a numeric string `strtod` reads only with `ERANGE`: a
//! subnormal is a "floating-point value", an underflow to zero or an
//! overflow a "non-numeric string", a math function finds no numeric value,
//! and `<` compares the strings. Later releases read all three as numbers
//! (#2483). Measured on tclsh 8.4.20 and 8.5.19.

use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_runtime::interp::{Code, Interp};

const SCRIPT: &str = r#"
set out {}
foreach v {1e-320 1e-400 1e400} {
    set x $v
    catch {expr {$x*1}} r; lappend out $r
    catch {expr {sin($x)}} r; lappend out $r
    lappend out [expr {$x < 1}]
}
proc p {x} { catch {expr {$x*1}} r; return $r }
lappend out [p 1e-320] [p 1e400]
join $out \n
"#;

fn run(version: TclVersion) -> String {
    let mut interp = Interp::with_native_core(
        Rc::new(NativeHost::new()),
        tcl_registry::model::resolve_environment(version.dialect_profile_name()).unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .expect("selected native core bootstrap");
    assert_eq!(interp.eval_str(SCRIPT.as_bytes()), Code::Ok);
    String::from_utf8_lossy(&interp.result_bytes()).into_owned()
}

#[test]
fn tcl84_refuses_an_out_of_range_double_string() {
    assert_eq!(
        run(TclVersion::V8_4),
        [
            "can't use floating-point value as operand of \"*\"",
            "argument to math function didn't have numeric value",
            "0",
            "can't use non-numeric string as operand of \"*\"",
            "argument to math function didn't have numeric value",
            "0",
            "can't use non-numeric string as operand of \"*\"",
            "argument to math function didn't have numeric value",
            "0",
            "can't use floating-point value as operand of \"*\"",
            "can't use non-numeric string as operand of \"*\"",
        ]
        .join("\n")
    );
}

#[test]
fn tcl85_reads_an_out_of_range_double_string_as_a_number() {
    assert_eq!(
        run(TclVersion::V8_5),
        [
            "1e-320",
            "1e-320",
            "1",
            "0.0",
            "0.0",
            "1",
            "Inf",
            "domain error: argument not in valid range",
            "0",
            "1e-320",
            "Inf",
        ]
        .join("\n")
    );
}
