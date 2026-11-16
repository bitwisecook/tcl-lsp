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

//! `find_redundancies`: same-block and dominated duplicates, invalidation and sibling branches.

use super::*;

#[test]
fn find_redundancies_detects_same_block_duplicate() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
    entry_blk.statements.push(llength_call());
    entry_blk.statements.push(llength_call());
    entry_blk.terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(cfg.entry, Vec::new());
    let mut ssa_entry = empty_ssa_block("entry");
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(cfg.entry, ssa_entry);

    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].code, DiagCode::O105);
    assert!(results[0].expression_text.contains("llength"));
}

#[test]
fn find_redundancies_ignores_different_ssa_version() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
    entry_blk.statements.push(llength_call());
    entry_blk.statements.push(llength_call());
    entry_blk.terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(cfg.entry, Vec::new());
    let mut ssa_entry = empty_ssa_block("entry");
    // Same expression but different SSA versions of $x → no
    // redundancy.
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(2)));
    ssa.blocks.insert(cfg.entry, ssa_entry);

    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

#[test]
fn find_redundancies_global_write_invalidates_cache() {
    let registry = CommandRegistry::build_default();
    // entry: llength $x; set ::g 1; llength $x
    let mut cfg = Function::new("::top", "entry");
    let entry_blk = cfg.blocks.get_mut(&cfg.entry).unwrap();
    entry_blk.statements.push(llength_call());
    entry_blk.statements.push(Statement::Call {
        span: Span::new(0, 0),
        command: "set".into(),
        canonical_command: None,
        args: vec!["::g".into(), "1".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    });
    entry_blk.statements.push(llength_call());
    entry_blk.terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(cfg.entry, Vec::new());
    let mut ssa_entry = empty_ssa_block("entry");
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa_entry.statements.push(ssa_stmt_for(
        &mut ssa,
        cfg.blocks[&cfg.entry].statements[1].clone(),
        None,
    ));
    ssa_entry
        .statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(cfg.entry, ssa_entry);

    // The global write should invalidate — no redundancy reported.
    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

#[test]
fn find_redundancies_descends_dominator_tree() {
    let registry = CommandRegistry::build_default();
    // entry: llength $x
    // dom_child: llength $x   (should trigger)
    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let child = cfg.intern_block("child");
    cfg.blocks.insert(child, Block::new("child"));
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Goto {
        target: child,
        span: None,
    });
    cfg.blocks
        .get_mut(&child)
        .unwrap()
        .statements
        .push(llength_call());
    cfg.blocks.get_mut(&child).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial(cfg.name.clone(), cfg.entry, cfg.block_names().to_vec());
    ssa.dominator_tree.insert(entry, vec![child]);
    ssa.dominator_tree.insert(child, Vec::new());
    let mut e = empty_ssa_block("entry");
    e.statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    let mut c = empty_ssa_block("child");
    c.statements
        .push(ssa_stmt_for(&mut ssa, llength_call(), Some(1)));
    ssa.blocks.insert(entry, e);
    ssa.blocks.insert(child, c);

    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results.len(), 1);
}

#[test]
fn find_redundancies_empty_for_no_blocks() {
    let registry = CommandRegistry::build_default();
    let mut cfg = Function::new("::top", "entry");
    // Intern a name with no matching block so the SSA entry is absent
    // from `cfg.blocks` and the walk early-returns.
    let nonexistent = cfg.intern_block("nonexistent");
    let ssa = SsaFunction::trivial(cfg.name.clone(), nonexistent, cfg.block_names().to_vec());
    let results = find_redundancies(&registry, &cfg, &ssa, None);
    assert_eq!(results, [] as [crate::gvn::RedundantComputation; 0]);
}

/// Exercise the explicitly legacy CFG-only classifier. Production
/// [`FunctionUnit`](crate::compilation_unit::FunctionUnit) GVN deliberately
/// does not scan embedded source strings when no exact semantic node exists.
fn legacy_redundancies(src: &str) -> Vec<String> {
    use crate::compilation_unit::CompilationUnit;
    let registry = release_registry("tcl8.6");
    let cu =
        CompilationUnit::build_for(src, &registry, false).with_interprocedural(&registry, None);
    let mut out = Vec::new();
    for function in cu.analysable_functions() {
        out.extend(
            find_redundancies(&registry, &function.cfg, &function.ssa, None)
                .into_iter()
                .map(|redundancy| redundancy.expression_text),
        );
    }
    out.sort();
    out
}

#[test]
fn detects_redundant_ensemble_subcommand_cse() {
    // `string length $a` is pure via its *subcommand* purity — the
    // second computation is redundant. Before subcommand-aware
    // side-effect classification this was missed (the ensemble call
    // looked like an unknown-write).
    assert_eq!(
        legacy_redundancies(
            "proc f {a} { set x [string length $a]\n set y [string length $a]\n return [expr {$x + $y}] }"
        ),
        vec!["string length $a".to_string()],
    );
    // Likewise `dict get`.
    assert_eq!(
        legacy_redundancies(
            "proc f {d} { set a [dict get $d k]\n set b [dict get $d k]\n return [list $a $b] }"
        ),
        vec!["dict get $d k".to_string()],
    );
}

#[test]
fn mutating_ensemble_subcommand_is_not_cse() {
    // `dict set` mutates — never a redundant-computation candidate.
    assert_eq!(
        legacy_redundancies("proc f {d} { dict set d k 1\n dict set d k 1\n return $d }").len(),
        0
    );
}

#[test]
fn kill_all_does_not_leak_across_sibling_branches() {
    // A state-writing statement (a global write here) in
    // the then-arm calls `kill_all`. That must NOT collapse the scope
    // stack, or the then-arm's `[llength $lst]` occurrence strands in the
    // never-popped root scope and the else-arm's identical computation is
    // falsely reported redundant. The two arms are mutually exclusive
    // paths — nothing is redundant.
    assert_eq!(
            legacy_redundancies(
                "proc f {c lst} { if {$c} { set ::g 1\n set a [llength $lst] } else { set b [llength $lst] }\n return 1 }"
            ).len(), 0,
            "llength on mutually-exclusive branches must not be flagged redundant",
        );
}

#[test]
fn kill_all_preserves_within_path_detection() {
    // FP-guard for: the in-place clear must not disturb
    // ordinary same-path redundancy detection — two identical pure
    // computations with nothing between them are still redundant.
    assert_eq!(
        legacy_redundancies(
            "proc f {lst} { set a [llength $lst]\n set b [llength $lst]\n return [list $a $b] }"
        ),
        vec!["llength $lst".to_string()],
    );
}
