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

//! Procedures: parameters and locals, calls between procedures, and the two top-level shapes a unit carries.

use super::*;

// Procedures

#[test]
fn proc_params_in_lvt() {
    let fa = proc_asm("proc add {a b} { expr {$a + $b} }", "::add");
    let entries = fa.lvt.entries();
    assert_eq!(entries[0], "a");
    assert_eq!(entries[1], "b");
}

#[test]
fn proc_uses_invoke() {
    // `puts $name` → push "puts", load name, invokeStk.
    let fa = proc_asm("proc greet {name} { puts $name }", "::greet");
    assert!(opcodes(&fa).contains(&Op::INVOKE_STK1));
}

#[test]
fn multiple_procs() {
    let ma = asm_for(
        "proc add {a b} { expr {$a + $b} }\nproc sub {a b} { expr {$a - $b} }\nproc mul {a b} { expr {$a * $b} }",
    );
    assert!(fixture_id_if_present(&ma.procedures, "::add").is_some());
    assert!(fixture_id_if_present(&ma.procedures, "::sub").is_some());
    assert!(fixture_id_if_present(&ma.procedures, "::mul").is_some());
    assert!(
        opcodes(
            ma.procedures
                .get(&fixture_id(&ma.procedures, "::add"))
                .expect("authored fixture body")
        )
        .contains(&Op::ADD)
    );
    assert!(
        opcodes(
            ma.procedures
                .get(&fixture_id(&ma.procedures, "::sub"))
                .expect("authored fixture body")
        )
        .contains(&Op::SUB)
    );
    assert!(
        opcodes(
            ma.procedures
                .get(&fixture_id(&ma.procedures, "::mul"))
                .expect("authored fixture body")
        )
        .contains(&Op::MULT)
    );
}

#[test]
fn proc_calling_proc() {
    let ma = asm_for("proc helper {x} { expr {$x * 2} }\nproc caller {x} { helper $x }");
    assert!(
        opcodes(
            ma.procedures
                .get(&fixture_id(&ma.procedures, "::caller"))
                .expect("authored fixture body")
        )
        .contains(&Op::INVOKE_STK1)
    );
}

#[test]
fn proc_with_many_params() {
    let fa = proc_asm(
        "proc many {a b c d e f g h} { return [expr {$a + $b + $c + $d + $e + $f + $g + $h}] }",
        "::many",
    );
    assert_eq!(
        &fa.lvt.entries()[..8],
        &["a", "b", "c", "d", "e", "f", "g", "h"]
    );
}

#[test]
fn proc_local_vs_param() {
    let fa = proc_asm(
        "proc compute {x y} { set temp [expr {$x + $y}]; set result [expr {$temp * 2}]; return $result }",
        "::compute",
    );
    let entries = fa.lvt.entries();
    assert_eq!(entries[0], "x");
    assert_eq!(entries[1], "y");
    let temp_idx = entries.iter().position(|e| e == "temp").unwrap();
    let result_idx = entries.iter().position(|e| e == "result").unwrap();
    assert!(temp_idx > 1);
    assert!(result_idx > 1);
}

#[test]
fn namespace_eval_proc_qualified() {
    let ir = ir_for("namespace eval ::myns {\n    proc foo {x} { expr {$x + 1} }\n}\n");
    assert!(fixture_id_if_present(&ir.procedures, "::myns::foo").is_some());
}

#[test]
fn dynamic_proc_name_not_registered() {
    let ir = ir_for("proc $name {x} { return $x }");
    assert_eq!(ir.procedures.len(), 0);
}

#[test]
fn proc_with_default_args() {
    let ir = ir_for("proc greet {name {greeting \"hello\"}} { return \"$greeting, $name\" }");
    assert!(fixture_id_if_present(&ir.procedures, "::greet").is_some());
}

/// A procedure's assembly records the body word it was compiled from, so a
/// consumer keyed by *name* can tell it apart from another `proc` of the same
/// name — the two arms of an `if`, a redefinition, a second `eval`.
#[test]
fn a_procedure_records_the_body_word_it_was_compiled_from() {
    let m = asm_for("proc p {} {return A}\nproc q {} {}\n");
    assert_eq!(
        m.procedures
            .get(&fixture_id(&m.procedures, "::p"))
            .expect("authored fixture body")
            .proc_body_src
            .as_deref(),
        Some("return A")
    );
    assert_eq!(
        m.procedures
            .get(&fixture_id(&m.procedures, "::q"))
            .expect("authored fixture body")
            .proc_body_src
            .as_deref(),
        Some("")
    );
    assert_eq!(m.top_level.proc_body_src, None);
    assert_eq!(m.top_level_body.proc_body_src, None);
}
