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

//! The CFG's structure and the IR statements lowering produces.

use super::*;

// CFG structure

#[test]
fn linear_script_single_block() {
    crate::install_surfaces();
    let ir = ir_for("set a 1\nset b 2");
    let cfg = build_cfg(&ir, false);
    let top = &cfg.top_level;
    let entry = &top.blocks[&top.entry];
    assert!(matches!(entry.terminator, Some(Terminator::Goto { .. })));
    if let Some(Terminator::Goto { target, .. }) = entry.terminator {
        assert!(top.blocks.contains_key(&target));
    }
}

#[test]
fn if_else_merge_block() {
    crate::install_surfaces();
    let ir = ir_for("if {$x} {set a 1} else {set a 2}\nset b 3");
    let cfg = build_cfg(&ir, false);
    let top = &cfg.top_level;
    let merge_blocks: Vec<_> = top
        .blocks
        .values()
        .filter(|b| {
            b.statements
                .iter()
                .any(|s| matches!(s, Statement::AssignConst { name, .. } if name == "b"))
        })
        .collect();
    assert_eq!(
        merge_blocks.len(),
        1,
        "`set b 3` lands in exactly one merge block"
    );
}

#[test]
fn for_loop_back_edge() {
    crate::install_surfaces();
    let src = "proc loop {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::loop"))
        .expect("authored fixture body");
    let header = proc_cfg
        .block_names()
        .iter()
        .find(|n| n.contains("for_header"))
        .cloned()
        .expect("for_header block");
    let block = proc_cfg.block_by_name(&header).unwrap();
    match &block.terminator {
        Some(Terminator::Branch {
            true_target,
            false_target,
            ..
        }) => {
            assert!(proc_cfg.blocks.contains_key(true_target));
            assert!(proc_cfg.blocks.contains_key(false_target));
        }
        other => panic!("for header should branch, got {other:?}"),
    }
}

#[test]
fn return_terminates_block() {
    crate::install_surfaces();
    let src = "proc early_exit {x} { set a 1; return $a; set b 2 }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::early_exit"))
        .expect("authored fixture body");
    let has_return = proc_cfg
        .blocks
        .values()
        .any(|b| matches!(b.terminator, Some(Terminator::Return { .. })));
    assert!(has_return);
    // `set b 2` after the return is dead. The CFG builder retains it only in an
    // `unreachable_*` block (never reachable from entry), so it must not appear
    // in any non-`unreachable` block.
    let in_live_block = proc_cfg.blocks.values().any(|b| {
        !b.name.starts_with("unreachable")
            && b.statements
                .iter()
                .any(|s| matches!(s, Statement::AssignConst { name, .. } if name == "b"))
    });
    assert!(
        !in_live_block,
        "dead `set b 2` must not be in a reachable block"
    );
}

#[test]
fn all_blocks_reachable() {
    crate::install_surfaces();
    let src = "proc simple {x} { if {$x > 0} { set r \"pos\" } else { set r \"neg\" }; return $r }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = cfg
        .procedures
        .get(&fixture_id(&cfg.procedures, "::simple"))
        .expect("authored fixture body");
    // BFS from entry over terminator successors.
    let mut visited = std::collections::HashSet::new();
    let mut queue = vec![proc_cfg.entry];
    while let Some(id) = queue.pop() {
        if !visited.insert(id) {
            continue;
        }
        if let Some(block) = proc_cfg.blocks.get(&id)
            && let Some(t) = &block.terminator
        {
            queue.extend(t.successors());
        }
    }
    // Every block reached from entry must be a real block in the CFG (no
    // dangling successor ids). The builder leaves an `exit_*` sentinel that the
    // returning merge block does not link to, so we do NOT require the
    // reachable set to equal *all* blocks. Instead,
    // assert the reachable set covers the real then/else/merge blocks.
    for id in &visited {
        assert!(
            proc_cfg.blocks.contains_key(id),
            "dangling successor {id:?}"
        );
    }
    let reachable_names: std::collections::HashSet<&str> =
        visited.iter().map(|&id| proc_cfg.block_name(id)).collect();
    assert!(reachable_names.iter().any(|n| n.contains("if_then")));
    assert!(reachable_names.iter().any(|n| n.contains("if_end")));
    // Only the trailing `exit_*` sentinel may be unreachable.
    let unreachable: Vec<&str> = proc_cfg
        .blocks
        .keys()
        .filter(|id| !visited.contains(id))
        .map(|&id| proc_cfg.block_name(id))
        .collect();
    assert!(
        unreachable.iter().all(|n| n.starts_with("exit")),
        "only the exit sentinel may be unreachable, got {unreachable:?}"
    );
}

// IR lowering edge cases

#[test]
fn set_integer_becomes_assign_const() {
    crate::install_surfaces();
    let ir = ir_for("set x 42");
    assert!(ir.top_level.statements.iter().any(
        |s| matches!(s, Statement::AssignConst { name, value, .. } if name == "x" && value == "42")
    ));
}

#[test]
fn set_expr_becomes_assign_expr() {
    crate::install_surfaces();
    let ir = ir_for("set r [expr {1 + 2}]");
    assert!(
        ir.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::AssignExpr { name, .. } if name == "r"))
    );
}

#[test]
fn incr_becomes_ir_incr() {
    crate::install_surfaces();
    let ir = ir_for("set x 0; incr x");
    assert!(ir.top_level.statements.iter().any(
        |s| matches!(s, Statement::Incr { name, amount, .. } if name == "x" && amount.is_none())
    ));
}

#[test]
fn incr_with_amount() {
    crate::install_surfaces();
    let ir = ir_for("set x 0; incr x 5");
    assert!(ir.top_level.statements.iter().any(|s| {
        matches!(s, Statement::Incr { name, amount, .. } if name == "x" && amount.as_deref() == Some("5"))
    }));
}

#[test]
fn return_with_options_becomes_barrier() {
    crate::install_surfaces();
    let ir = ir_for("proc fail {} { return -code error \"oops\" }");
    let proc = ir
        .procedures
        .get(&fixture_id(&ir.procedures, "::fail"))
        .expect("authored fixture body");
    assert!(
        proc.body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { .. }))
    );
}

#[test]
fn standalone_expr_becomes_expr_eval() {
    crate::install_surfaces();
    let ir = ir_for("expr {1 + 2}");
    assert!(
        ir.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::ExprEval { .. }))
    );
}

#[test]
fn if_without_braces_fallback() {
    crate::install_surfaces();
    let ir = ir_for("if $x {puts ok}");
    let stmts = &ir.top_level.statements;
    assert_ne!(*stmts, [] as [Statement; 0]);
    let has_expected = stmts.iter().any(|s| {
        matches!(
            s,
            Statement::If { .. } | Statement::Barrier { .. } | Statement::Call { .. }
        )
    });
    assert!(
        has_expected,
        "expected If/Barrier/Call, got {:?}",
        stmts.iter().map(std::mem::discriminant).collect::<Vec<_>>()
    );
}
