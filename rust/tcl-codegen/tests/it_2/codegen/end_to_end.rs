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

//! Whole programs through the pipeline: recursive procedures, sorting, state machines and a stress script.

use super::*;

// End-to-end pipeline

#[test]
fn fibonacci_proc() {
    // tclsh: fib 10  →  55   (9.0)
    let src = "proc fib {n} { if {$n <= 1} { return $n }; set a 0; set b 1; for {set i 2} {$i <= $n} {incr i} { set temp [expr {$a + $b}]; set a $b; set b $temp }; return $b }";
    let fa = proc_asm(src, "::fib");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::LE));
    assert!(ops.contains(&Op::ADD));
}

#[test]
fn bubble_sort() {
    let src = "proc bubblesort {lst} { set n [llength $lst]; for {set i 0} {$i < $n} {incr i} { for {set j 0} {$j < [expr {$n - $i - 1}]} {incr j} { set a [lindex $lst $j]; set b [lindex $lst [expr {$j + 1}]]; if {$a > $b} { set lst [lreplace $lst $j $j $b]; set lst [lreplace $lst [expr {$j + 1}] [expr {$j + 1}] $a] } } }; return $lst }";
    let fa = proc_asm(src, "::bubblesort");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::GT));
}

#[test]
fn state_machine() {
    let src = "proc state_machine {input} { set state \"start\"; set i 0; while {$state ne \"done\"} { switch -exact $state { start { set state \"process\" } process { if {$i >= 10} { set state \"done\" } else { incr i } } default { set state \"done\" } } }; return $i }";
    let fa = proc_asm(src, "::state_machine");
    let ops = opcodes(&fa);
    assert!(has_cond_jump(&ops));
    assert!(ops.contains(&Op::DONE));
}

#[test]
fn multiproc_module_format() {
    let src =
        "proc add {a b} { expr {$a + $b} }\nproc mul {a b} { expr {$a * $b} }\nadd 1 2\nmul 3 4";
    let text = format_module_asm(&asm_for(src));
    assert!(text.contains("ByteCode ::top"));
    assert!(text.contains("ByteCode ::add"));
    assert!(text.contains("ByteCode ::mul"));
}

#[test]
fn complex_value_interpolation_in_proc() {
    // The emitter pushes the interpolated `"Hello, $name! …"` value as a single
    // literal (runtime word-substitution resolves the `$name`/`$age` markers),
    // then storeScalar1 — no compile-time STR_CONCAT1. Assert the store path +
    // the interpolated literal is interned verbatim.
    let fa = proc_asm(
        "proc greet {name age} { set msg \"Hello, $name! You are $age years old.\"; return $msg }",
        "::greet",
    );
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::STORE_SCALAR1));
    assert!(
        fa.literals.entries().iter().any(|l| {
            l.unicode()
                .is_ok_and(|text| text.contains("Hello, ") && text.contains("{name}"))
        }),
        "interpolated literal interned verbatim, got {:?}",
        fa.literals.entries()
    );
}

#[test]
fn append_lappend_bytecoded() {
    let src = "proc builder {} { set s \"\"; set lst {}; append s \"hello\"; append s \" world\"; lappend lst \"item1\"; lappend lst \"item2\"; return $lst }";
    let ops = opcodes(&proc_asm(src, "::builder"));
    assert!(ops.contains(&Op::APPEND_SCALAR1));
    assert!(ops.contains(&Op::LAPPEND_SCALAR1));
}

#[test]
fn catch_inside_loop() {
    let src = "proc retry {n cmd} { for {set i 0} {$i < $n} {incr i} { set rc [catch {eval $cmd} result]; if {$rc == 0} { return $result } }; return \"\" }";
    let ops = opcodes(&proc_asm(src, "::retry"));
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(ops.contains(&Op::END_CATCH));
    assert!(ops.contains(&Op::LT));
}

#[test]
fn large_literal_table() {
    let mut assigns = String::new();
    for i in 0..50 {
        use std::fmt::Write as _;
        let _ = writeln!(assigns, "    set v{i} {i}");
    }
    let src = format!("proc many_lits {{}} {{\n{assigns}}}\n");
    let fa = proc_asm(&src, "::many_lits");
    assert!(fa.literals.len() >= 50);
}

#[test]
fn deeply_nested_procs_do_not_crash() {
    let src = "proc deep {x} { if {$x > 0} { if {$x > 1} { if {$x > 2} { for {set i 0} {$i < $x} {incr i} { if {$i % 2 == 0} { while {$i < 5} { incr i } } } } } }; return $x }";
    assert!(opcodes(&proc_asm(src, "::deep")).contains(&Op::DONE));
}

// Stress: combining everything

#[test]
fn kitchen_sink() {
    let src = "namespace eval ::test {\n    proc helper {x} {\n        return [expr {$x * 2}]\n    }\n}\n\nproc main {args} {\n    set total 0\n    set items {1 2 3 4 5}\n    for {set i 0} {$i < 5} {incr i} {\n        if {$i % 2 == 0} {\n            set total [expr {$total + $i}]\n        }\n    }\n    foreach item $items {\n        set total [expr {$total + $item}]\n    }\n    set j 0\n    while {$j < 100} {\n        if {$j > 10} {\n            break\n        }\n        incr j\n    }\n    switch -exact $j {\n        10 { set label \"ten\" }\n        11 { set label \"eleven\" }\n        default { set label \"other\" }\n    }\n    set rc [catch {expr {1/0}} err]\n    set s \"hello world\"\n    string length $s\n    string toupper $s\n    return $total\n}\n\nmain\n";
    let ma = asm_for(src);
    assert!(fixture_id_if_present(&ma.procedures, "::main").is_some());
    let ops = opcodes(
        ma.procedures
            .get(&fixture_id(&ma.procedures, "::main"))
            .expect("authored fixture body"),
    );
    assert!(ops.contains(&Op::LT) || ops.contains(&Op::LE));
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::MOD));
    assert!(ops.contains(&Op::DONE));
    assert!(format_module_asm(&ma).contains("ByteCode ::main"));
}

// codegen_module sanity (mirrors integration's empty-module smoke)

#[test]
fn codegen_module_empty_has_top() {
    let cfg_mod = CfgModule {
        top_level: toplevel_cfg(vec![]),
        procedures: HashMap::new(),
    };
    let ir_mod = ir_for("");
    let asm = codegen_module(&cfg_mod, &ir_mod, &registry()).expect("original source namespace");
    assert_eq!(asm.top_level.name, "::top");
}
