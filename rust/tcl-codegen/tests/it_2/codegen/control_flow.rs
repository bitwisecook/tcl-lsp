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

//! Branches, `switch`, loop exits, `catch` and `try`, and conditions that hold command substitutions.

use super::*;

// CFG terminators

#[test]
fn if_branch_has_cond_jump() {
    let ops = ops_of("if {$x} {set a 1} else {set a 2}");
    assert!(has_cond_jump(&ops));
}

#[test]
fn fallthrough_elimination() {
    // A simple set needs no jumps at all.
    let ops = ops_of("set x 1");
    assert_eq!(count(&ops, Op::JUMP1) + count(&ops, Op::JUMP4), 0);
}

// Nested control flow

#[test]
fn if_inside_for() {
    let src = "proc f {n} { set total 0; for {set i 0} {$i < $n} {incr i} { if {$i % 2 == 0} { set total [expr {$total + $i}] } else { set total [expr {$total - $i}] } }; return $total }";
    // tclsh: f 5  →  2   (0-1+2-3+4)   (9.0)
    let ops = opcodes(&proc_asm(src, "::f"));
    assert!(
        has_cond_jump(&ops),
        "for header + if branch need conditional jumps"
    );
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::SUB));
    assert!(ops.contains(&Op::MOD));
    assert!(ops.contains(&Op::DONE));
}

#[test]
fn for_inside_if() {
    let src = "proc g {mode n} { if {$mode eq \"sum\"} { set r 0; for {set i 1} {$i <= $n} {incr i} { set r [expr {$r + $i}] } } elseif {$mode eq \"prod\"} { set r 1; for {set i 1} {$i <= $n} {incr i} { set r [expr {$r * $i}] } } else { set r -1 }; return $r }";
    let ops = opcodes(&proc_asm(src, "::g"));
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::MULT));
    assert!(count(&ops, Op::LE) >= 2, "two for-loop condition tests");
}

#[test]
fn deeply_nested_if() {
    let src = "proc classify {x y z} { if {$x > 0} { if {$y > 0} { if {$z > 0} { set r \"all_pos\" } else { set r \"z_neg\" } } else { set r \"y_neg\" } } else { set r \"x_neg\" }; return $r }";
    let ops = opcodes(&proc_asm(src, "::classify"));
    let cond = count(&ops, Op::JUMP_TRUE1)
        + count(&ops, Op::JUMP_TRUE4)
        + count(&ops, Op::JUMP_FALSE1)
        + count(&ops, Op::JUMP_FALSE4);
    assert!(cond >= 3);
}

#[test]
fn nested_for_loops() {
    let src = "proc matrix_fill {rows cols} { for {set i 0} {$i < $rows} {incr i} { for {set j 0} {$j < $cols} {incr j} { set val [expr {$i * $cols + $j}] } } }";
    let ops = opcodes(&proc_asm(src, "::matrix_fill"));
    assert!(count(&ops, Op::LT) >= 2);
    assert!(ops.contains(&Op::MULT));
    assert!(ops.contains(&Op::ADD));
}

#[test]
fn foreach_inside_for() {
    let src = "proc search {lists n} { for {set i 0} {$i < $n} {incr i} { foreach item $lists { if {$item eq \"found\"} { return $item } } }; return \"not_found\" }";
    let ops = opcodes(&proc_asm(src, "::search"));
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::FOREACH_START) || ops.contains(&Op::INVOKE_STK1));
    assert!(ops.contains(&Op::STR_EQ));
}

// Switch edge cases

#[test]
fn switch_many_arms() {
    let arms: String = (0..10)
        .map(|i| format!("v{i} {{ set r {i} }}"))
        .collect::<Vec<_>>()
        .join(" ");
    let src = format!(
        "proc big_switch {{x}} {{ switch -exact -- $x {{ {arms} default {{ set r -1 }} }}; return $r }}"
    );
    let ops = opcodes(&proc_asm(&src, "::big_switch"));
    assert!(ops.contains(&Op::DONE));
    let cond = count(&ops, Op::JUMP_TRUE1)
        + count(&ops, Op::JUMP_TRUE4)
        + count(&ops, Op::JUMP_FALSE1)
        + count(&ops, Op::JUMP_FALSE4);
    let has_jump_table = ops.contains(&Op::JUMP_TABLE);
    assert!(
        cond >= 10 || has_jump_table,
        "10 arms → conditional branches or a jump table"
    );
}

#[test]
fn switch_with_return_in_arms() {
    let src = "proc dispatch {cmd} { switch -exact -- $cmd { add { return 1 } sub { return 2 } mul { return 3 } default { return 0 } } }";
    let ops = opcodes(&proc_asm(src, "::dispatch"));
    assert!(ops.contains(&Op::JUMP_TABLE) || has_cond_jump(&ops));
}

// `asm_for` builds its CFG without naming a dialect, so it takes the lenient
// `tcl` profile, which declares no release. Before 8.5 `switch` reads every
// leading word that starts with `-` as an option, so a variable subject with no
// `--` before it may be one: the statement stays one generic invoke, where a
// dispatch chain would select the arm the subject spells. `--` ends the options
// and keeps the jump table (`switch_with_return_in_arms`).
#[test]
fn switch_a_release_may_scan_as_an_option_routes_through_generic_invoke() {
    let src = "proc dispatch {cmd} { switch -exact $cmd { add { return 1 } sub { return 2 } mul { return 3 } default { return 0 } } }";
    let ops = opcodes(&proc_asm(src, "::dispatch"));
    assert!(
        !ops.contains(&Op::JUMP_TABLE) && !has_cond_jump(&ops),
        "no dispatch chain: {ops:?}"
    );
    assert!(
        ops.contains(&Op::INVOKE_STK1) || ops.contains(&Op::INVOKE_STK4),
        "one generic invoke: {ops:?}"
    );
}

// Loop control flow

#[test]
fn break_in_while() {
    let src = "proc find_first {lst target} { set i 0; while {$i < 100} { if {$i == $target} { break }; incr i }; return $i }";
    let ops = opcodes(&proc_asm(src, "::find_first"));
    assert!(has_cond_jump(&ops));
    assert!(
        ops.contains(&Op::JUMP4),
        "break compiles to JUMP4 to loop end"
    );
}

#[test]
fn continue_in_for() {
    let src = "proc sum_odd {n} { set total 0; for {set i 0} {$i < $n} {incr i} { if {$i % 2 == 0} { continue }; set total [expr {$total + $i}] }; return $total }";
    let ops = opcodes(&proc_asm(src, "::sum_odd"));
    assert!(ops.contains(&Op::JUMP4), "continue → jump");
}

#[test]
fn return_in_loop() {
    let src = "proc search {lst target} { foreach item $lst { if {$item eq $target} { return $item } }; return \"\" }";
    let ops = opcodes(&proc_asm(src, "::search"));
    assert!(ops.contains(&Op::DONE) || ops.contains(&Op::RETURN_IMM));
}

// Exception handling

#[test]
fn catch_basic() {
    let src = "proc safe {cmd} { set rc [catch {eval $cmd} result]; return $rc }";
    let ops = opcodes(&proc_asm(src, "::safe"));
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(ops.contains(&Op::END_CATCH));
}

#[test]
fn catch_with_options() {
    let src = "proc safe2 {cmd} { set rc [catch {eval $cmd} result opts]; return $rc }";
    let ops = opcodes(&proc_asm(src, "::safe2"));
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(ops.contains(&Op::END_CATCH));
}

#[test]
fn try_finally_block() {
    let src = "proc with_cleanup {} { try { set x 1 } finally { set x 0 }; return $x }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::with_cleanup"))
        .expect("authored fixture body");
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("finally")));
}

#[test]
fn try_on_error() {
    let src = "proc safe_div {a b} { try { set r [expr {$a / $b}] } on error {msg opts} { set r -1 }; return $r }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::safe_div"))
        .expect("authored fixture body");
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("handler")));
}

#[test]
fn try_on_error_with_finally() {
    let src = "proc robust {body_cmd} { set ok 0; try { eval $body_cmd; set ok 1 } on error {msg} { set ok 0 } finally { set cleanup_done 1 }; return $ok }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::robust"))
        .expect("authored fixture body");
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("handler")));
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("finally")));
}

// Condition defs from command substitutions

#[test]
fn catch_in_if_condition() {
    let src =
        "proc safe_check {} { if {[catch {expr {1/0}} result]} { return $result }; return \"ok\" }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::safe_check"))
        .expect("authored fixture body");
    let has_cond_def = proc_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "<cond>"))
    });
    assert!(has_cond_def, "synthetic <cond> def expected");
}

#[test]
fn catch_in_while_condition() {
    // #2427: `lower_to_ir` IR runs in a host-extensible command table, so an
    // undefined `some_op` is an unbounded head that may rebind `catch` before
    // the condition runs again, and the condition then names no write. With
    // it undefined the loop never ends: tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4
    // and 9.1.0 print `retrying...` forever. Defined, `retry_loop` returns
    // the empty string on each of them.
    let src = "proc some_op {} {}\nproc retry_loop {} { while {[catch {some_op} result] != 0} { puts \"retrying...\" }; return $result }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::retry_loop"))
        .expect("authored fixture body");
    let has_cond_def = proc_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "<cond>"))
    });
    assert!(has_cond_def);
}
