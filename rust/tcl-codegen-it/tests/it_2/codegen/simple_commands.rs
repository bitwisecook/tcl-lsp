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

//! The literal and local tables and the simplest statements: `set`, `puts`, `incr`, `return` and quoted literals.

use super::*;

// LiteralTable / LocalVarTable

#[test]
fn literal_table_intern_new() {
    crate::install_surfaces();
    let mut lt = LiteralTable::new();
    assert_eq!(lt.intern("hello"), 0);
    assert_eq!(lt.intern("world"), 1);
    assert_eq!(lt.len(), 2);
}

#[test]
fn literal_table_intern_dedup() {
    crate::install_surfaces();
    let mut lt = LiteralTable::new();
    assert_eq!(lt.intern("x"), 0);
    assert_eq!(lt.intern("x"), 0);
    assert_eq!(lt.len(), 1);
}

#[test]
fn literal_table_entries() {
    crate::install_surfaces();
    let mut lt = LiteralTable::new();
    lt.intern("a");
    lt.intern("b");
    assert_eq!(lt.entries(), &["a", "b"]);
}

#[test]
fn local_var_table_params_first() {
    crate::install_surfaces();
    let mut lvt = LocalVarTable::new(&["x", "y"]);
    assert_eq!(lvt.intern("x"), 0);
    assert_eq!(lvt.intern("y"), 1);
    assert_eq!(lvt.intern("z"), 2);
    assert_eq!(lvt.len(), 3);
}

#[test]
fn local_var_table_entries() {
    crate::install_surfaces();
    let mut lvt = LocalVarTable::new(&["a"]);
    lvt.intern("b");
    assert_eq!(lvt.entries(), &["a", "b"]);
}

// Simple codegen

#[test]
fn set_const() {
    crate::install_surfaces();
    let fa = top_asm("set x 1");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::PUSH1));
    // Top-level uses stack-based ops; procs use storeScalar1.
    assert!(ops.contains(&Op::STORE_STK));
    // Last command's result stays on TOS for done.
    assert_eq!(*ops.last().unwrap(), Op::DONE);
}

#[test]
fn puts_invoke() {
    crate::install_surfaces();
    let fa = top_asm("puts hello");
    assert!(opcodes(&fa).contains(&Op::INVOKE_STK1));
    assert!(fa.literals.entries().iter().any(|l| l == "puts"));
}

#[test]
fn incr_default() {
    crate::install_surfaces();
    // Top-level uses stack-based incr.
    assert!(ops_of("set i 0; incr i").contains(&Op::INCR_STK_IMM));
}

#[test]
fn incr_immediate() {
    crate::install_surfaces();
    assert!(ops_of("set i 0; incr i 5").contains(&Op::INCR_STK_IMM));
}

#[test]
fn proc_return_folds_to_done() {
    crate::install_surfaces();
    let reg = registry();
    let ir = lower_to_ir("proc foo {} { return 42 }", &reg);
    let cfg = build_cfg(&ir, false);
    let module = codegen_module(&cfg, &ir, &reg).expect("original source namespace");
    assert!(fixture_id_if_present(&module.procedures, "::foo").is_some());
    // Tail-position return is folded to done (matching tclsh).
    assert!(
        opcodes(
            module
                .procedures
                .get(&fixture_id(&module.procedures, "::foo"))
                .expect("authored fixture body")
        )
        .contains(&Op::DONE)
    );
}

#[test]
fn done_at_end() {
    crate::install_surfaces();
    let fa = top_asm("set x 1");
    assert_eq!(fa.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn quoted_close_brace_is_string_literal_issue_130() {
    crate::install_surfaces();
    // `append x "}"` must intern the bare `}` character as a literal (not
    // treat it as a delimiter).
    let fa = top_asm("append x \"}\"");
    let lits = fa.literals.entries();
    assert!(
        lits.iter().any(|l| l == "}"),
        "expected '}}' literal, got {lits:?}"
    );
    // Rust emits PUSH + generic invoke for top-level `append` (no APPEND_STK
    // specialisation at script scope) — assert the literal push path, not a
    // stray syntax-error path.
    assert!(opcodes(&fa).contains(&Op::PUSH1));
}

#[test]
fn quoted_close_bracket_is_string_literal() {
    crate::install_surfaces();
    // Mirror: `puts "]"` must intern `]` as a literal.
    let fa = top_asm("puts \"]\"");
    let lits = fa.literals.entries();
    assert!(
        lits.iter().any(|l| l == "]"),
        "expected ']' literal, got {lits:?}"
    );
}

// Backslash substitution

#[test]
fn backslash_in_set_value() {
    crate::install_surfaces();
    let fa = top_asm("set x \"line1\\nline2\"");
    assert!(opcodes(&fa).contains(&Op::DONE));
}
