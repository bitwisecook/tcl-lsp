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

//! `for`, `while`, `foreach` and `dict for`: clause edge cases, iterator groups and frozen conditions.

use super::*;

// For loop edge cases

#[test]
fn empty_init_clause() {
    let src = "proc count_down {i} { for {} {$i > 0} {incr i -1} { puts $i } }";
    let ops = opcodes(&proc_asm(src, "::count_down"));
    assert!(count(&ops, Op::NOP) >= 3, "empty init clause → 3 NOPs");
}

#[test]
fn empty_step_clause() {
    let src = "proc manual_step {} { for {set i 0} {$i < 10} {} { incr i 2 } }";
    let ops = opcodes(&proc_asm(src, "::manual_step"));
    assert!(count(&ops, Op::NOP) >= 3);
}

#[test]
fn for_with_complex_init() {
    let src = "proc multi_init {} { for {set i 0; set j 10} {$i < $j} {incr i; incr j -1} { puts \"$i $j\" } }";
    let ops = opcodes(&proc_asm(src, "::multi_init"));
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::INCR_SCALAR1_IMM));
}

#[test]
fn a_for_init_command_starts_one_command_in_every_release() {
    // tclsh8.5, 8.6, 9.0 and 9.1, `tcl::unsupported::disassemble proc p`: the
    // only startCommand of this body is the `for`'s, with a count of 1
    // (8.6: `startCommand +29 1 # next cmd at pc 34, 1 cmds start here`).
    let source = "proc p {} { set warmup 1; for {missing_command} {0} {} {} }";
    for release in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let asm = crate::vm_compiled::procedure(release, source, "::p");
        let counts: Vec<_> = asm
            .instructions
            .iter()
            .filter(|instruction| instruction.op == Op::START_CMD)
            .map(|instruction| instruction.operands.get(1).cloned())
            .collect();
        assert!(!counts.is_empty(), "{release}: {:?}", asm.instructions);
        assert!(
            counts.iter().all(|count| *count == Some(Operand::Imm(1))),
            "{release}: {counts:?}"
        );
    }
}

#[test]
fn for_loop_registered_in_loop_nodes() {
    let src = "proc loop {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::loop"))
        .expect("authored fixture body");
    assert_eq!(
        proc_cfg.loop_nodes.len(),
        1,
        "the single for loop registers one loop node"
    );
}

// While loop edge cases

#[test]
fn while_constant_true() {
    let src = "proc spin {} { while {1} { break } }";
    let ops = opcodes(&proc_asm(src, "::spin"));
    assert!(ops.contains(&Op::JUMP4), "break");
}

#[test]
fn while_complex_condition() {
    let src = "proc bounded {x y limit} { while {$x < $limit && $y < $limit} { incr x; incr y }; return [expr {$x + $y}] }";
    let ops = opcodes(&proc_asm(src, "::bounded"));
    assert!(ops.contains(&Op::LT));
}

// Foreach edge cases

#[test]
fn foreach_multi_var() {
    let src = "proc pairs {lst} { set result {}; foreach {k v} $lst { append result \"$k=$v \" }; return $result }";
    let ops = opcodes(&proc_asm(src, "::pairs"));
    assert!(ops.contains(&Op::FOREACH_START));
    assert!(ops.contains(&Op::FOREACH_STEP));
}

#[test]
fn foreach_creates_loop_cfg() {
    let src = "proc iter {lst} { foreach item $lst { puts $item } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::iter"))
        .expect("authored fixture body");
    let names = proc_cfg.block_names();
    assert!(names.iter().any(|n| n.contains("foreach_header")));
    assert!(names.iter().any(|n| n.contains("foreach_body")));
    assert!(names.iter().any(|n| n.contains("foreach_end")));
}

#[test]
fn dict_for_lowers_to_loop_cfg_for_analysis_but_barriers_for_codegen() {
    // `dict for`'s body runs in the caller's frame, so the *analysis* CFG
    // (`build_cfg`, faithful_exceptions on) flattens it into a foreach-style
    // header→body→end loop — like `foreach_creates_loop_cfg` above — so the
    // body's reads/writes and its own control flow are first-class SSA — a
    // read nested inside an `if` in the body is not lost. The *codegen* CFG
    // (`build_cfg_codegen`, faithful off) keeps the opaque
    // `::tcl::dict::for` barrier the inline `emit_dict_for` compiles, so the
    // emitted bytecode stays byte-identical to C Tcl. Mirrors `array for`.
    let src = "proc dict_iter {d} { dict for {k v} $d { puts \"$k: $v\" } }";
    let ir = ir_for(src);

    // Analysis CFG: real loop blocks, no dict-for barrier.
    let analysis = build_cfg(&ir, false);
    let a_cfg = analysis
        .procedures
        .get(&fixture_id(&analysis.procedures, "::dict_iter"))
        .expect("authored fixture body");
    let names = a_cfg.block_names();
    assert!(
        names.iter().any(|n| n.contains("foreach_header")),
        "analysis CFG should flatten dict for into a loop; got {names:?}"
    );
    assert!(names.iter().any(|n| n.contains("foreach_body")));
    assert!(names.iter().any(|n| n.contains("foreach_end")));
    assert!(
        !a_cfg.blocks.values().any(|b| b.statements.iter().any(
            |s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("dict for"))
        )),
        "analysis CFG should not keep a dict-for barrier"
    );

    // Codegen CFG: the opaque `::tcl::dict::for` barrier is preserved.
    let codegen = build_cfg_codegen(&ir, false);
    let c_cfg = codegen
        .procedures
        .get(&fixture_id(&codegen.procedures, "::dict_iter"))
        .expect("authored fixture body");
    let has_barrier = c_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("dict for")))
    });
    assert!(has_barrier, "codegen CFG must keep the dict-for barrier");
}

// Multiple iterator groups in foreach

fn foreach_iter_count(src: &str, proc: &str) -> usize {
    let ir = ir_for(src);
    let p = ir
        .procedures
        .get(&fixture_id(&ir.procedures, proc))
        .expect("authored fixture body");
    p.body
        .statements
        .iter()
        .find_map(|s| {
            if let Statement::Foreach { iterators, .. } = s {
                Some(iterators.len())
            } else {
                None
            }
        })
        .expect("foreach statement")
}

#[test]
fn foreach_two_lists() {
    let src = "proc zip {list1 list2} { set result {}; foreach i $list1 j $list2 { lappend result \"$i:$j\" }; return $result }";
    assert_eq!(foreach_iter_count(src, "::zip"), 2);
    assert!(opcodes(&proc_asm(src, "::zip")).contains(&Op::DONE));
}

#[test]
fn foreach_three_lists() {
    let src = "proc triple {a b c} { set r {}; foreach i $a j $b k $c { lappend r [expr {$i + $j + $k}] }; return $r }";
    assert_eq!(foreach_iter_count(src, "::triple"), 3);
}

#[test]
fn foreach_multi_var_per_group() {
    let src =
        "proc pairs {lst} { set r {}; foreach {k v} $lst { lappend r \"$k=$v\" }; return $r }";
    let ir = ir_for(src);
    let p = ir
        .procedures
        .get(&fixture_id(&ir.procedures, "::pairs"))
        .expect("authored fixture body");
    let fe = p
        .body
        .statements
        .iter()
        .find_map(|s| {
            if let Statement::Foreach { iterators, .. } = s {
                Some(iterators)
            } else {
                None
            }
        })
        .expect("foreach");
    let var_list = &fe[0].vars;
    assert_eq!(var_list.len(), 2);
    assert!(var_list.iter().any(|v| v == "k"));
    assert!(var_list.iter().any(|v| v == "v"));
}

// defer_top_level vs inline_loops

#[test]
fn top_level_foreach_deferred_is_generic_call() {
    // A deferred top-level foreach lowers to a generic `Call` with
    // command "foreach" (not inlined into foreach_* blocks). Assert that
    // shape: a non-inlined `foreach` call at script scope.
    let src = "foreach item {a b c} { puts $item }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, true); // defer_top_level = true
    let top = &cfg.top_level;
    let has_foreach_call = top.blocks.values().any(|b| {
        b.statements.iter().any(|s| {
            matches!(s, Statement::Call { command, .. } if command == "foreach")
                || matches!(s, Statement::Barrier { command, .. } if command == "foreach")
        })
    });
    assert!(
        has_foreach_call,
        "deferred foreach should be a generic call/barrier"
    );
    // And it must NOT have been inlined into foreach_* blocks.
    assert!(
        !top.block_names()
            .iter()
            .any(|n| n.contains("foreach_header")),
        "deferred foreach must not be inlined"
    );
}

#[test]
fn top_level_foreach_inlined() {
    let src = "foreach item {a b c} { puts $item }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false); // defer_top_level = false
    let top = &cfg.top_level;
    assert!(
        top.block_names()
            .iter()
            .any(|n| n.contains("foreach_header"))
    );
}

// Frozen for/while (command-subst conditions)

#[test]
fn frozen_for_cmd_subst_condition() {
    let src = "proc frozen {} { for {set i 0} {[expr {$i < 10}]} {incr i} { puts $i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::frozen"))
        .expect("authored fixture body");
    let has_barrier = proc_cfg.blocks.values().any(|b| {
        b.statements.iter().any(
            |s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("frozen for")),
        )
    });
    assert!(has_barrier);
}

#[test]
fn frozen_while_cmd_subst_condition() {
    let src = "proc frozen_while {} { set i 0; while {[expr {$i < 10}]} { incr i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::frozen_while"))
        .expect("authored fixture body");
    let has_barrier = proc_cfg.blocks.values().any(|b| {
        b.statements.iter().any(
            |s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("frozen while")),
        )
    });
    assert!(has_barrier);
}
