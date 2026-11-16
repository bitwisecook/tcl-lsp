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

//! `find_loop_invariants`: hoistable calls, latch dominance and loop-defined variables.

use super::*;

// Loop-invariant detection.

#[test]
fn find_loop_invariants_detects_hoistable_llength() {
    let registry = CommandRegistry::build_default();
    // header: branch on $i < 10 → body → header (back edge)
    // body: llength $x (invariant w.r.t. loop-defined $i)
    let mut cfg = Function::new("::top", "header");
    let header = cfg.entry;
    let body = cfg.intern_block("body");
    let exit = cfg.intern_block("exit");
    cfg.blocks.insert(body, Block::new("body"));
    cfg.blocks.insert(exit, Block::new("exit"));
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: body,
        false_target: exit,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    // header dominates body and exit.
    ssa.idom.insert(header, None);
    ssa.idom.insert(body, Some(header));
    ssa.idom.insert(exit, Some(header));
    ssa.dominator_tree.insert(header, vec![body, exit]);
    ssa.dominator_tree.insert(body, Vec::new());
    ssa.dominator_tree.insert(exit, Vec::new());

    let h = empty_ssa_block("header");
    let mut b = empty_ssa_block("body");
    // $x's SSA version comes from outside the loop (x@1).
    b.statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(header, h);
    ssa.blocks.insert(body, b);
    ssa.blocks.insert(exit, empty_ssa_block("exit"));

    let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(results.len(), 1);
    // Canonical: loop-invariant code motion is O106 (not O107,
    // which is unreachable dead code).
    assert_eq!(results[0].code, DiagCode::O106);
    assert!(results[0].expression_text.contains("llength"));
}

#[test]
fn find_loop_invariants_skips_branch_that_does_not_dominate_latch() {
    // Loop:  header → cond → {then, latch}; then → latch; latch → header.
    // The invariant `llength $x` sits in `then`, which does NOT dominate
    // the latch (cond can reach the latch directly), so it runs only on
    // some iterations — the latch-dominance gate must suppress O106.
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "header");
    let header = cfg.entry;
    for b in ["cond", "then", "latch", "exit"] {
        let id = cfg.intern_block(b);
        cfg.blocks.insert(id, Block::new(b));
    }
    let cond = id_of(&cfg, "cond");
    let then = id_of(&cfg, "then");
    let latch = id_of(&cfg, "latch");
    let exit = id_of(&cfg, "exit");
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: cond,
        false_target: exit,
        span: None,
        condition_base: None,
    });
    cfg.blocks.get_mut(&cond).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: then,
        false_target: latch,
        span: None,
        condition_base: None,
    });
    cfg.blocks
        .get_mut(&then)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Goto {
        target: latch,
        span: None,
    });
    cfg.blocks.get_mut(&latch).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(header, None);
    ssa.idom.insert(cond, Some(header));
    ssa.idom.insert(then, Some(cond));
    // The latch is reachable from `cond` both directly and via `then`,
    // so its immediate dominator is `cond`, not `then`.
    ssa.idom.insert(latch, Some(cond));
    ssa.idom.insert(exit, Some(header));
    ssa.dominator_tree.insert(header, vec![cond, exit]);
    ssa.dominator_tree.insert(cond, vec![then, latch]);
    for b in [then, latch, exit] {
        ssa.dominator_tree.insert(b, Vec::new());
    }

    ssa.blocks.insert(header, empty_ssa_block("header"));
    ssa.blocks.insert(cond, empty_ssa_block("cond"));
    let mut then_b = empty_ssa_block("then");
    then_b
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(then, then_b);
    ssa.blocks.insert(latch, empty_ssa_block("latch"));
    ssa.blocks.insert(exit, empty_ssa_block("exit"));

    let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(
        results.len(),
        0,
        "invariant behind an in-loop branch must not be hoisted, got {results:?}",
    );
}

#[test]
fn find_loop_invariants_skips_loop_defined_var() {
    let registry = CommandRegistry::build_default();
    // As above, but `$i` is defined inside the loop — llength
    // uses `$i` → not loop-invariant.
    let mut cfg = Function::new("::top", "header");
    let header = cfg.entry;
    let body = cfg.intern_block("body");
    let exit = cfg.intern_block("exit");
    cfg.blocks.insert(body, Block::new("body"));
    cfg.blocks.insert(exit, Block::new("exit"));
    cfg.blocks.get_mut(&header).unwrap().terminator = Some(Terminator::Branch {
        condition: crate::expr_ast::ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        },
        true_target: body,
        false_target: exit,
        span: None,
        condition_base: None,
    });
    let llength_on_i = Statement::Call {
        span: Span::new(0, 0),
        command: "llength".into(),
        canonical_command: None,
        args: vec!["$i".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };
    cfg.blocks
        .get_mut(&body)
        .unwrap()
        .statements
        .push(llength_on_i.clone());
    cfg.blocks.get_mut(&body).unwrap().terminator = Some(Terminator::Goto {
        target: header,
        span: None,
    });
    cfg.blocks.get_mut(&exit).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(header, None);
    ssa.idom.insert(body, Some(header));
    ssa.idom.insert(exit, Some(header));

    let h = empty_ssa_block("header");
    let mut b = empty_ssa_block("body");
    // `$i` defined in body.
    let mut defs = Map::new();
    defs.insert(ssa.intern_var("i"), 1);
    b.statements.push(SsaStatement {
        statement: Statement::AssignConst {
            span: Span::new(0, 0),
            name: "i".into(),
            name_braced: false,
            value: "0".into(),
            value_span: None,
        },
        uses: Map::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    // llength $i — uses map tracks `i`, not `x`.
    let mut uses_i = Map::new();
    uses_i.insert(ssa.intern_var("i"), 1);
    b.statements.push(SsaStatement {
        statement: llength_on_i,
        uses: uses_i,
        defs: Map::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    ssa.blocks.insert(header, h);
    ssa.blocks.insert(body, b);
    ssa.blocks.insert(exit, empty_ssa_block("exit"));

    let results = find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

#[test]
fn find_loop_invariants_no_loops_empty() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    cfg.blocks.get_mut(&cfg.entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.idom.insert(cfg.entry, None);
    ssa.blocks.insert(cfg.entry, empty_ssa_block("entry"));
    assert_eq!(
        find_loop_invariants(&registry, &cfg, &ssa, &all_blocks(&cfg), None),
        [] as [crate::gvn::RedundantComputation; 0]
    );
}
