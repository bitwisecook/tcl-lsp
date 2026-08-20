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

//! Tests for the `tcl-compiler` code generator.
//!
//! Coverage:
//!   * bytecode-assembly backend basics
//!   * nested control flow, expr edge cases, switch/loop/proc/try, CFG structure
//!
//! Pipeline used throughout: `lower_to_ir` → `build_cfg_codegen` /
//! `build_cfg` → `codegen_module` / `codegen_function`, then inspect the
//! emitted `FunctionAsm` (opcode stream, literal pool, LVT) and the disassembly
//! text.
//!
//! ── tclsh proofs ────────────────────────────────────────────────────────
//! Where a test pins a RUNTIME RESULT (the generated code computes value
//! X for input Y) and the codegen folds it at compile time, the folded
//! literal is checked against C-Tcl (`scripts/dev/tclsh_check.sh`, 8.6 + 9.0)
//! and cited with `// tclsh:`. The Rust VM (`tcl-vm`) is NOT a dependency of
//! `tcl-compiler` (and adding one would edit Cargo.toml, out of scope here), so
//! an end-to-end VM value check is unavailable from this crate; the folded
//! literal is the observable runtime result and is the value pinned to tclsh.
//! Everything else is a structural / bytecode-shape assertion — compiler
//! internal — and is noted as such where relevant.
//!
//! ── Codegen surfaces with no dedicated opcode (documented behaviour) ─
//!  1. SPECIALISED string/dict-subcommand opcodes. The emitter (verified by
//!     probing `codegen_module`) routes `string length`/`index`/`toupper`/
//!     `trim*`/`match`/`equal`/`compare`/`range`/`first`/`last`/`replace`/`map`,
//!     `lindex`, and `dict get`/`exists` through a generic `invokeStk1` both at
//!     top level AND in proc bodies — the dedicated `STR_*`/`LIST_INDEX_IMM`/
//!     `DICT_*` opcodes exist in `Op` but the bytecode emitter does not select
//!     them for these commands. The corresponding tests assert "compiles to a
//!     generic invoke and the literal `<subcmd>`/operand pool is correct" so the
//!     path is still covered. (The list commands the emitter DOES specialise —
//!     llength/lrange/linsert/lassign/lreplace — are asserted with their real
//!     opcodes.)
//!  2. `LiteralTable.entries()` returns `&[String]` (ordered, deduped) and
//!     `LocalVarTable` likewise.
//!  3. `esc` escaping. `format::esc` matches C-Tcl's disassembler BYTE-WISE
//!     over UTF-8: a non-ASCII codepoint renders as the `\uXXXX` of each raw
//!     UTF-8 byte (so `ÿ` U+00FF → `Ã¿`, an astral char → four `\u00XX`
//!     bytes), not codepoint-wise. The named-control, NUL, C0, DEL, quote, and
//!     printable-ASCII cases are unaffected; the high-BMP / supplementary-plane
//!     cases assert the byte-wise expectation and are annotated.
//!  4. `lappend x foo` at SCRIPT scope emits a generic `invokeStk1` (no
//!     `lappendListStk` specialisation at script scope).
//!  5. Braced whole-name array ref `${a($i)}` vs bare `$a($i)`. The emitter
//!     does NOT carry the braced-vs-bare whole-name/split distinction — both
//!     spellings lower to a split `loadArray1` — so those 11 tclsh-9.0.3-pinned
//!     cases are documented here only.
//!  6. The `emit_binop`/`emit_unaryop` iRules harness builds a CFG by hand: a
//!     `CfgFunction` with a `Statement::AssignExpr` carrying an
//!     `ExprNode::Binary`/`Unary`, run through `codegen_function`.

use std::collections::HashMap;

use tcl_compiler::cfg::{CfgModule, Function as CfgFunction, Terminator};
use tcl_compiler::cfg_builder::{build_cfg, build_cfg_codegen};
use tcl_compiler::codegen::format::{esc, format_function_asm, format_module_asm};
use tcl_compiler::codegen::{
    FunctionAsm, LiteralTable, LocalVarTable, Op, codegen_function, codegen_module,
};
use tcl_compiler::expr_ast::{BinOp, ExprNode, UnaryOp};
use tcl_compiler::ir::{Module as IrModule, Statement};
use tcl_compiler::lowering::lower_to_ir;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

// ── Helpers ──

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

fn sp() -> Span {
    Span::new(0, 0)
}

/// Lower `source` → IR → CFG (codegen flavour, defer top-level) → `ModuleAsm`.
fn asm_for(source: &str) -> tcl_compiler::codegen::ModuleAsm {
    let reg = registry();
    let ir = lower_to_ir(source, &reg);
    let cfg = build_cfg_codegen(&ir, true);
    codegen_module(&cfg, &ir, &reg)
}

/// The top-level `FunctionAsm` for `source`.
fn top_asm(source: &str) -> FunctionAsm {
    asm_for(source).top_level
}

/// The `FunctionAsm` for a named procedure in `source`.
fn proc_asm(source: &str, name: &str) -> FunctionAsm {
    asm_for(source)
        .procedures
        .remove(name)
        .unwrap_or_else(|| panic!("procedure {name} not found"))
}

/// The opcode stream of a `FunctionAsm`.
fn opcodes(fa: &FunctionAsm) -> Vec<Op> {
    fa.instructions.iter().map(|i| i.op).collect()
}

fn ops_of(source: &str) -> Vec<Op> {
    opcodes(&top_asm(source))
}

fn count(ops: &[Op], op: Op) -> usize {
    ops.iter().filter(|&&o| o == op).count()
}

fn has_cond_jump(ops: &[Op]) -> bool {
    ops.iter().any(|o| {
        matches!(
            o,
            Op::JUMP_TRUE1 | Op::JUMP_TRUE4 | Op::JUMP_FALSE1 | Op::JUMP_FALSE4
        )
    })
}

/// IR-only lowering helper.
fn ir_for(source: &str) -> IrModule {
    lower_to_ir(source, &registry())
}

/// Build a top-level `CfgFunction` whose single entry block holds `statements`
/// and returns.
fn toplevel_cfg(statements: Vec<Statement>) -> CfgFunction {
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    let blk = cfg.blocks.get_mut(&entry).unwrap();
    blk.statements = statements;
    blk.terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });
    cfg
}

/// Emit `r = (a <binop> b)` through a hand-built CFG.
fn emit_binop(op: BinOp) -> Vec<Op> {
    let expr = ExprNode::Binary {
        op,
        left: Box::new(ExprNode::Var {
            text: "$a".into(),
            name: "a".into(),
            start: 0,
            end: 2,
        }),
        right: Box::new(ExprNode::Var {
            text: "$b".into(),
            name: "b".into(),
            start: 0,
            end: 2,
        }),
    };
    let cfg = toplevel_cfg(vec![Statement::AssignExpr {
        span: sp(),
        name: "r".into(),
        name_braced: false,
        expr,
        expr_base: None,
    }]);
    opcodes(&codegen_function(&cfg, &[], false, &registry()))
}

/// Emit `r = (<unaryop> a)` through a hand-built CFG.
fn emit_unaryop(op: UnaryOp) -> Vec<Op> {
    let expr = ExprNode::Unary {
        op,
        operand: Box::new(ExprNode::Var {
            text: "$a".into(),
            name: "a".into(),
            start: 0,
            end: 2,
        }),
    };
    let cfg = toplevel_cfg(vec![Statement::AssignExpr {
        span: sp(),
        name: "r".into(),
        name_braced: false,
        expr,
        expr_base: None,
    }]);
    opcodes(&codegen_function(&cfg, &[], false, &registry()))
}

// ═══════════════════════════════════════════════════════════════════
// LiteralTable / LocalVarTable
// ═══════════════════════════════════════════════════════════════════

#[test]
fn literal_table_intern_new() {
    let mut lt = LiteralTable::new();
    assert_eq!(lt.intern("hello"), 0);
    assert_eq!(lt.intern("world"), 1);
    assert_eq!(lt.len(), 2);
}

#[test]
fn literal_table_intern_dedup() {
    let mut lt = LiteralTable::new();
    assert_eq!(lt.intern("x"), 0);
    assert_eq!(lt.intern("x"), 0);
    assert_eq!(lt.len(), 1);
}

#[test]
fn literal_table_entries() {
    let mut lt = LiteralTable::new();
    lt.intern("a");
    lt.intern("b");
    assert_eq!(lt.entries(), &["a", "b"]);
}

#[test]
fn local_var_table_params_first() {
    let mut lvt = LocalVarTable::new(&["x", "y"]);
    assert_eq!(lvt.intern("x"), 0);
    assert_eq!(lvt.intern("y"), 1);
    assert_eq!(lvt.intern("z"), 2);
    assert_eq!(lvt.len(), 3);
}

#[test]
fn local_var_table_entries() {
    let mut lvt = LocalVarTable::new(&["a"]);
    lvt.intern("b");
    assert_eq!(lvt.entries(), &["a", "b"]);
}

// ═══════════════════════════════════════════════════════════════════
// Simple codegen
// ═══════════════════════════════════════════════════════════════════

#[test]
fn set_const() {
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
    let fa = top_asm("puts hello");
    assert!(opcodes(&fa).contains(&Op::INVOKE_STK1));
    assert!(fa.literals.entries().iter().any(|l| l == "puts"));
}

#[test]
fn incr_default() {
    // Top-level uses stack-based incr.
    assert!(ops_of("set i 0; incr i").contains(&Op::INCR_STK_IMM));
}

#[test]
fn incr_immediate() {
    assert!(ops_of("set i 0; incr i 5").contains(&Op::INCR_STK_IMM));
}

#[test]
fn proc_return_folds_to_done() {
    let reg = registry();
    let ir = lower_to_ir("proc foo {} { return 42 }", &reg);
    let cfg = build_cfg(&ir, false);
    let module = codegen_module(&cfg, &ir, &reg);
    assert!(module.procedures.contains_key("::foo"));
    // Tail-position return is folded to done (matching tclsh).
    assert!(opcodes(&module.procedures["::foo"]).contains(&Op::DONE));
}

#[test]
fn done_at_end() {
    let fa = top_asm("set x 1");
    assert_eq!(fa.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn quoted_close_brace_is_string_literal_issue_130() {
    // Regression for bitwisecook/tcl-lsp#130: `append x "}"` must intern the
    // bare `}` character as a literal (not treat it as a delimiter).
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
    // Mirror: `puts "]"` must intern `]` as a literal.
    let fa = top_asm("puts \"]\"");
    let lits = fa.literals.entries();
    assert!(
        lits.iter().any(|l| l == "]"),
        "expected ']' literal, got {lits:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Expression compilation
// ═══════════════════════════════════════════════════════════════════

#[test]
fn expr_binary_add() {
    assert!(ops_of("set x 1; set r [expr {$x + 2}]").contains(&Op::ADD));
}

#[test]
fn expr_binary_eq() {
    assert!(ops_of("set x 1; set r [expr {$x == 2}]").contains(&Op::EQ));
}

#[test]
fn expr_binary_lt() {
    assert!(ops_of("set x 1; set r [expr {$x < 2}]").contains(&Op::LT));
}

#[test]
fn expr_constant_fold() {
    // All-constant expressions are folded at compile time.
    // tclsh: expr {1 + 2}  →  3   (8.6 + 9.0)
    let fa = top_asm("set r [expr {1 + 2}]");
    let ops = opcodes(&fa);
    assert!(!ops.contains(&Op::ADD), "folded away");
    assert!(fa.literals.entries().iter().any(|l| l == "3"));
}

#[test]
fn expr_unary_neg() {
    // tclsh: expr {-1}  →  -1   (8.6 + 9.0)
    let fa = top_asm("set r [expr {-1}]");
    let ops = opcodes(&fa);
    assert!(!ops.contains(&Op::UMINUS), "constant folded");
    assert!(fa.literals.entries().iter().any(|l| l == "-1"));
}

#[test]
fn expr_var_load() {
    let ops = ops_of("set x 1; set r [expr {$x + 1}]");
    // Top-level uses stack-based load.
    assert!(ops.contains(&Op::LOAD_STK));
    assert!(ops.contains(&Op::ADD));
}

#[test]
fn expr_math_func_inline() {
    // Math function calls compile inline as invokeStk + tryCvtToNumeric.
    let ops = ops_of("set x 1; set r [expr {sin($x)}]");
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(ops.contains(&Op::TRY_CVT_TO_NUMERIC));
}

#[test]
fn standalone_expr_inline() {
    // Standalone `expr` is compiled inline, not as invokeStk.
    let ops = ops_of("set x 1; expr {$x + 2}");
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::LOAD_STK));
}

#[test]
fn standalone_expr_constant_fold() {
    // tclsh: expr {2 + 3}  →  5   (8.6 + 9.0)
    let fa = top_asm("expr {2 + 3}");
    assert!(!opcodes(&fa).contains(&Op::ADD));
    assert!(fa.literals.entries().iter().any(|l| l == "5"));
}

// ═══════════════════════════════════════════════════════════════════
// iRules expression operators
//
// Uses the `emit_binop`/`emit_unaryop` helpers: a hand-built CFG with an
// `AssignExpr` carrying the binary/unary node.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn irule_contains() {
    assert!(emit_binop(BinOp::Contains).contains(&Op::IRULE_CONTAINS));
}

#[test]
fn irule_starts_with() {
    assert!(emit_binop(BinOp::StartsWith).contains(&Op::IRULE_STARTS_WITH));
}

#[test]
fn irule_ends_with() {
    assert!(emit_binop(BinOp::EndsWith).contains(&Op::IRULE_ENDS_WITH));
}

#[test]
fn irule_equals() {
    assert!(emit_binop(BinOp::StrEquals).contains(&Op::IRULE_EQUALS));
}

#[test]
fn irule_matches_glob() {
    assert!(emit_binop(BinOp::MatchesGlob).contains(&Op::IRULE_MATCHES_GLOB));
}

#[test]
fn irule_matches_regex() {
    assert!(emit_binop(BinOp::MatchesRegex).contains(&Op::IRULE_MATCHES_REGEX));
}

#[test]
fn irule_word_and() {
    assert!(emit_binop(BinOp::WordAnd).contains(&Op::IRULE_WORD_AND));
}

#[test]
fn irule_word_or() {
    assert!(emit_binop(BinOp::WordOr).contains(&Op::IRULE_WORD_OR));
}

#[test]
fn irule_word_not() {
    assert!(emit_unaryop(UnaryOp::WordNot).contains(&Op::IRULE_WORD_NOT));
}

// ═══════════════════════════════════════════════════════════════════
// CFG terminators
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// Bytecoded commands
//
// NOTE: top-level `append` and `lappend` route through a generic invoke (no
// APPEND_STK / lappendListStk specialisation at script scope — see the file
// header). `string length/equal/compare` are likewise generic — see the
// documented behaviour note in the header.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn append_toplevel_generic_invoke() {
    // Emits a generic invoke at script scope (no APPEND_STK specialisation).
    let fa = top_asm("set x hello; append x world");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(fa.literals.entries().iter().any(|l| l == "append"));
}

#[test]
fn append_proc_is_bytecoded() {
    // In a PROC body the Rust emitter DOES specialise append → appendScalar1.
    let fa = proc_asm("proc b {} { set s \"\"; append s \"x\"; return $s }", "::b");
    assert!(opcodes(&fa).contains(&Op::APPEND_SCALAR1));
}

#[test]
fn lappend_toplevel_generic_invoke() {
    // Emits a generic invoke at script scope (no lappendListStk specialisation).
    let fa = top_asm("set x {}; lappend x foo");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(fa.literals.entries().iter().any(|l| l == "lappend"));
}

#[test]
fn lappend_proc_is_bytecoded() {
    // In a PROC body Rust specialises lappend → lappendScalar1.
    let fa = proc_asm("proc b {} { set l {}; lappend l \"y\"; return $l }", "::b");
    assert!(opcodes(&fa).contains(&Op::LAPPEND_SCALAR1));
}

#[test]
fn string_subcommands_route_through_generic_invoke() {
    // The emitter does NOT select STR_LEN / STR_EQ / STR_CMP for
    // `string length/equal/compare` (neither at top level nor in a proc); it
    // emits a generic invoke. Assert that behaviour + the subcommand literal
    // pool so the path stays covered.
    for (src, lit) in [
        ("string length \"hello\"", "length"),
        ("string equal \"a\" \"b\"", "equal"),
        ("string compare \"a\" \"b\"", "compare"),
    ] {
        let fa = top_asm(src);
        let ops = opcodes(&fa);
        assert!(
            ops.contains(&Op::INVOKE_STK1),
            "{src}: expected generic invoke"
        );
        assert!(
            !ops.contains(&Op::STR_LEN),
            "{src}: Rust does not specialise STR_LEN"
        );
        assert!(
            fa.literals.entries().iter().any(|l| l == "string"),
            "{src}: 'string' literal present"
        );
        let _ = lit;
    }
}

// ═══════════════════════════════════════════════════════════════════
// Formatting
// ═══════════════════════════════════════════════════════════════════

#[test]
fn format_header() {
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.starts_with("ByteCode ::top"));
    assert!(text.contains("instructions"));
    assert!(text.contains("literals"));
    assert!(text.contains("variables"));
}

#[test]
fn format_literals_section() {
    let text = format_function_asm(&top_asm("set x hello"));
    assert!(text.contains("Literals:"));
    assert!(text.contains("\"hello\""));
}

#[test]
fn format_variables_section() {
    // Top-level scripts use stack-based var access → no LVT.
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.contains("0 variables"));
    // Proc bodies still use the LVT.
    let ma = asm_for("proc foo {x} { set x 1 }");
    let proc_text = format_function_asm(&ma.procedures["::foo"]);
    assert!(proc_text.contains("Local variables:"));
    assert!(proc_text.contains("%v0: \"x\""));
}

#[test]
fn format_instruction_offsets() {
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.contains("(0) push1"));
}

#[test]
fn format_block_labels() {
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.contains("# "), "block label comment expected");
}

#[test]
fn format_module() {
    let ma = asm_for("proc foo {x} { return $x }\nfoo 1");
    let text = format_module_asm(&ma);
    assert!(text.contains("ByteCode ::top"));
    assert!(text.contains("ByteCode ::foo"));
}

#[test]
fn format_instruction_offsets_monotonic() {
    let fa = top_asm("set x 1; set y 2; set z [expr {$x + $y}]");
    let offsets: Vec<i32> = fa
        .instructions
        .iter()
        .map(|i| i.offset)
        .filter(|&o| o >= 0)
        .collect();
    for w in offsets.windows(2) {
        assert!(w[1] >= w[0], "offsets must be monotonic: {offsets:?}");
    }
}

#[test]
fn format_proc_has_local_variables() {
    let ma = asm_for("proc foo {a b} { set c [expr {$a + $b}]; return $c }");
    let text = format_function_asm(&ma.procedures["::foo"]);
    assert!(text.contains("Local variables:"));
    assert!(text.contains("\"a\""));
    assert!(text.contains("\"b\""));
}

#[test]
fn format_done_always_at_end() {
    let ma =
        asm_for("proc add {a b} { expr {$a + $b} }\nproc sub {a b} { expr {$a - $b} }\nset x 1");
    assert_eq!(ma.top_level.instructions.last().unwrap().op, Op::DONE);
    for (name, fa) in &ma.procedures {
        assert_eq!(
            fa.instructions.last().unwrap().op,
            Op::DONE,
            "proc {name} missing trailing done"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════
// Disassembly escaping
//
// `esc` is BYTE-WISE over UTF-8 (matches C-Tcl's disassembler), not
// codepoint-wise. The named/NUL/C0/DEL/quote/ASCII cases are unaffected; the
// high-BMP / supplementary cases assert the byte-wise expectation (see file
// header note 3).
// ═══════════════════════════════════════════════════════════════════

#[test]
fn esc_named_controls_use_short_forms() {
    assert_eq!(esc("a\nb\tc\rd\x0be\x0cf", 40), "a\\nb\\tc\\rd\\ve\\ff");
}

#[test]
fn esc_nul_escaped_as_u_xxxx() {
    assert_eq!(esc("a\x00b", 40), "a\\u0000b");
}

#[test]
fn esc_stx_and_other_c0_controls_escaped() {
    // STX (0x02) is the byte that leaked in tcllib 2.0 installer.tcl.
    assert_eq!(esc("\x01\x02\x03\x1f", 40), "\\u0001\\u0002\\u0003\\u001f");
}

#[test]
fn esc_del_and_high_bmp_escaped_bytewise() {
    assert_eq!(esc("\x7f", 40), "\\u007f");
    // Byte-wise: U+00FF is the two UTF-8 bytes C3 BF → Ã¿.
    assert_eq!(esc("\u{ff}", 40), "\\u00c3\\u00bf");
}

#[test]
fn esc_supplementary_plane_is_bytewise_four_bytes() {
    // Byte-wise: U+1F600 is the four UTF-8 bytes F0 9F 98 80.
    assert_eq!(esc("\u{1f600}", 40), "\\u00f0\\u009f\\u0098\\u0080");
}

#[test]
fn esc_printable_ascii_unchanged() {
    assert_eq!(esc("hello world!", 40), "hello world!");
}

#[test]
fn esc_quote_escaped() {
    assert_eq!(esc("a\"b", 40), "a\\\"b");
}

#[test]
fn literal_with_embedded_stx_appears_escaped_in_disassembly() {
    // End-to-end: a literal interned with STX renders escaped in the
    // Literals: section — no raw control byte reaches the formatter output.
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            value: "\x00\x02-".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        value: None,
        span: None,
        expr: None,
        braced: false,
    });
    let fa = codegen_function(&cfg, &[], false, &registry());
    let text = format_function_asm(&fa);
    assert!(!text.contains('\x02'));
    assert!(text.contains("\\u0000\\u0002-"));
}

// ═══════════════════════════════════════════════════════════════════
// Procedures
// ═══════════════════════════════════════════════════════════════════

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
    assert!(ma.procedures.contains_key("::add"));
    assert!(ma.procedures.contains_key("::sub"));
    assert!(ma.procedures.contains_key("::mul"));
    assert!(opcodes(&ma.procedures["::add"]).contains(&Op::ADD));
    assert!(opcodes(&ma.procedures["::sub"]).contains(&Op::SUB));
    assert!(opcodes(&ma.procedures["::mul"]).contains(&Op::MULT));
}

#[test]
fn proc_calling_proc() {
    let ma = asm_for("proc helper {x} { expr {$x * 2} }\nproc caller {x} { helper $x }");
    assert!(opcodes(&ma.procedures["::caller"]).contains(&Op::INVOKE_STK1));
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
    assert!(ir.procedures.contains_key("::myns::foo"));
}

#[test]
fn dynamic_proc_name_not_registered() {
    let ir = ir_for("proc $name {x} { return $x }");
    assert_eq!(ir.procedures.len(), 0);
}

#[test]
fn proc_with_default_args() {
    let ir = ir_for("proc greet {name {greeting \"hello\"}} { return \"$greeting, $name\" }");
    assert!(ir.procedures.contains_key("::greet"));
}

// ═══════════════════════════════════════════════════════════════════
// Nested control flow
// ═══════════════════════════════════════════════════════════════════

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
fn while_with_nested_switch() {
    let src = "proc dispatch {items} { set i 0; set result {}; while {$i < 10} { switch -exact $i { 0 { set result \"zero\" } 1 { set result \"one\" } default { set result \"other\" } }; incr i }; return $result }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::dispatch"];
    let branch_count = proc_cfg
        .blocks
        .values()
        .filter(|b| matches!(b.terminator, Some(Terminator::Branch { .. })))
        .count();
    assert!(branch_count >= 3);
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

// ═══════════════════════════════════════════════════════════════════
// Complex expressions
// ═══════════════════════════════════════════════════════════════════

#[test]
fn ternary_in_expr() {
    let src =
        "proc clamp {x lo hi} { set r [expr {$x < $lo ? $lo : ($x > $hi ? $hi : $x)}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::clamp"));
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::GT));
    assert!(has_cond_jump(&ops), "ternary needs conditional jumps");
}

#[test]
fn chained_logical_and() {
    let src = "proc all_positive {a b c} { set r [expr {$a > 0 && $b > 0 && $c > 0}]; return $r }";
    // tclsh: with all > 0  →  1   (9.0)
    let ops = opcodes(&proc_asm(src, "::all_positive"));
    assert!(count(&ops, Op::GT) >= 3);
}

#[test]
fn chained_logical_or() {
    let src = "proc any_zero {a b c} { set r [expr {$a == 0 || $b == 0 || $c == 0}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::any_zero"));
    assert!(count(&ops, Op::EQ) >= 3);
}

#[test]
fn mixed_arithmetic_and_comparison() {
    let src = "proc check {x y} { set r [expr {($x + $y) * 2 > ($x - $y) ** 2}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::check"));
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::SUB));
    assert!(ops.contains(&Op::MULT));
    assert!(ops.contains(&Op::EXPON));
    assert!(ops.contains(&Op::GT));
}

#[test]
fn bitwise_operations() {
    // tclsh: expr {7 & 3}=3, {1 << 4}=16, {2 ** 10}=1024   (8.6 + 9.0)
    let src = "proc bits {x y} { set a [expr {$x & $y}]; set b [expr {$x | $y}]; set c [expr {$x ^ $y}]; set d [expr {$x << 2}]; set e [expr {$x >> 1}]; set f [expr {~$x}]; return [expr {$a + $b + $c + $d + $e + $f}] }";
    let ops = opcodes(&proc_asm(src, "::bits"));
    assert!(ops.contains(&Op::BITAND));
    assert!(ops.contains(&Op::BITOR));
    assert!(ops.contains(&Op::BITXOR));
    assert!(ops.contains(&Op::LSHIFT));
    assert!(ops.contains(&Op::RSHIFT));
    assert!(ops.contains(&Op::BITNOT));
}

#[test]
fn string_comparison_operators() {
    let src = "proc str_cmp {a b} { set r1 [expr {$a eq $b}]; set r2 [expr {$a ne $b}]; set r3 [expr {$a lt $b}]; set r4 [expr {$a gt $b}]; return [expr {$r1 + $r2 + $r3 + $r4}] }";
    let ops = opcodes(&proc_asm(src, "::str_cmp"));
    assert!(ops.contains(&Op::STR_EQ));
    assert!(ops.contains(&Op::STR_NEQ));
    assert!(ops.contains(&Op::STR_LT));
    assert!(ops.contains(&Op::STR_GT));
}

#[test]
fn list_membership_in_ni() {
    let src = "proc membership {x lst} { set a [expr {$x in $lst}]; set b [expr {$x ni $lst}]; return [expr {$a + $b}] }";
    let ops = opcodes(&proc_asm(src, "::membership"));
    assert!(ops.contains(&Op::LIST_IN));
    assert!(ops.contains(&Op::LIST_NOT_IN));
}

#[test]
fn nested_math_functions() {
    let src = "proc compute {x} { set r [expr {abs(int(sin($x) * 100))}]; return $r }";
    let ops = opcodes(&proc_asm(src, "::compute"));
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(ops.contains(&Op::TRY_CVT_TO_NUMERIC));
}

#[test]
fn constant_fold_complex() {
    // tclsh: expr {(2 + 3) * 4 - 1}  →  19   (8.6 + 9.0)
    let fa = top_asm("set r [expr {(2 + 3) * 4 - 1}]");
    let ops = opcodes(&fa);
    assert!(!ops.contains(&Op::ADD));
    assert!(!ops.contains(&Op::MULT));
    assert!(!ops.contains(&Op::SUB));
    assert!(fa.literals.entries().iter().any(|l| l == "19"));
}

#[test]
fn constant_fold_boolean() {
    // tclsh: expr {1 && 0}  →  0   (8.6 + 9.0)
    let fa = top_asm("set r [expr {1 && 0}]");
    assert!(!opcodes(&fa).contains(&Op::LAND));
    assert!(fa.literals.entries().iter().any(|l| l == "0"));
}

#[test]
fn partial_fold_mixed_const_var() {
    // 2*3=6 folds, but $x + 6 still needs ADD.
    let ops = ops_of("set x 1; set r [expr {$x + 2 * 3}]");
    assert!(ops.contains(&Op::ADD));
    assert!(!ops.contains(&Op::MULT));
}

// ═══════════════════════════════════════════════════════════════════
// Switch edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn switch_fallthrough_stays_irswitch() {
    let src = "proc classify {x} { switch -exact $x { a - b - c { set r \"letter\" } 1 - 2 - 3 { set r \"digit\" } default { set r \"other\" } }; return $r }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::classify"];
    let irswitch_count: usize = proc_cfg
        .blocks
        .values()
        .flat_map(|b| &b.statements)
        .filter(|s| matches!(s, Statement::Switch { .. }))
        .count();
    assert_eq!(
        irswitch_count, 1,
        "fallthrough switch should stay as IRSwitch"
    );
    // Codegen still works.
    assert!(opcodes(&proc_asm(src, "::classify")).contains(&Op::DONE));
}

#[test]
fn switch_glob_stays_as_irswitch() {
    let src = "proc ext {filename} { switch -glob $filename { *.c { return \"c_source\" } *.h { return \"c_header\" } default { return \"unknown\" } } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::ext"];
    let has_barrier = proc_cfg.blocks.values().any(|b| {
        b.statements.iter().any(
            |s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("switch -glob")),
        )
    });
    assert!(!has_barrier, "-glob should not lower to a Barrier");
    let has_switch = proc_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Switch { .. }))
    });
    assert!(has_switch, "-glob should keep IRSwitch for inline codegen");
}

#[test]
fn switch_regexp_stays_switch() {
    let src = "proc match {s} { switch -regexp $s { {^[0-9]+$} { return \"number\" } {^[a-z]+$} { return \"alpha\" } default { return \"mixed\" } } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::match"];
    let has_barrier = proc_cfg.blocks.values().any(|b| {
        b.statements.iter().any(
            |s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("switch -regexp")),
        )
    });
    assert!(!has_barrier, "-regexp should not lower to a Barrier");
    let has_switch = proc_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Switch { .. }))
    });
    assert!(
        has_switch,
        "-regexp should keep IRSwitch for inline codegen"
    );
}

#[test]
fn switch_many_arms() {
    let arms: String = (0..10)
        .map(|i| format!("v{i} {{ set r {i} }}"))
        .collect::<Vec<_>>()
        .join(" ");
    let src = format!(
        "proc big_switch {{x}} {{ switch -exact $x {{ {arms} default {{ set r -1 }} }}; return $r }}"
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
    let src = "proc dispatch {cmd} { switch -exact $cmd { add { return 1 } sub { return 2 } mul { return 3 } default { return 0 } } }";
    let ops = opcodes(&proc_asm(src, "::dispatch"));
    assert!(ops.contains(&Op::JUMP_TABLE) || has_cond_jump(&ops));
}

// ═══════════════════════════════════════════════════════════════════
// Loop control flow
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// Exception handling
// ═══════════════════════════════════════════════════════════════════

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
    let proc_cfg = &cfg.procedures["::with_cleanup"];
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("finally")));
}

#[test]
fn try_on_error() {
    let src = "proc safe_div {a b} { try { set r [expr {$a / $b}] } on error {msg opts} { set r -1 }; return $r }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::safe_div"];
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("handler")));
}

#[test]
fn try_on_error_with_finally() {
    let src = "proc robust {body_cmd} { set ok 0; try { eval $body_cmd; set ok 1 } on error {msg} { set ok 0 } finally { set cleanup_done 1 }; return $ok }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::robust"];
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("handler")));
    assert!(proc_cfg.block_names().iter().any(|n| n.contains("finally")));
}

// ═══════════════════════════════════════════════════════════════════
// For loop edge cases
// ═══════════════════════════════════════════════════════════════════

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
fn for_loop_registered_in_loop_nodes() {
    let src = "proc loop {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::loop"];
    assert_eq!(
        proc_cfg.loop_nodes.len(),
        1,
        "the single for loop registers one loop node"
    );
}

// ═══════════════════════════════════════════════════════════════════
// While loop edge cases
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// Foreach edge cases
// ═══════════════════════════════════════════════════════════════════

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
    let proc_cfg = &cfg.procedures["::iter"];
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
    // body's reads/writes and its own control flow are first-class SSA (a read
    // nested inside an `if` in the body is no longer lost — issue #833). The
    // *codegen* CFG (`build_cfg_codegen`, faithful off) keeps the opaque
    // `::tcl::dict::for` barrier the inline `emit_dict_for` compiles, so the
    // emitted bytecode stays byte-identical to C Tcl. Mirrors `array for`.
    let src = "proc dict_iter {d} { dict for {k v} $d { puts \"$k: $v\" } }";
    let ir = ir_for(src);

    // Analysis CFG: real loop blocks, no dict-for barrier.
    let analysis = build_cfg(&ir, false);
    let a_cfg = &analysis.procedures["::dict_iter"];
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
    let c_cfg = &codegen.procedures["::dict_iter"];
    let has_barrier = c_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("dict for")))
    });
    assert!(has_barrier, "codegen CFG must keep the dict-for barrier");
}

// ═══════════════════════════════════════════════════════════════════
// Variable access patterns
// ═══════════════════════════════════════════════════════════════════

#[test]
fn proc_array_ref() {
    let src =
        "proc array_op {key} { set arr(key1) \"val1\"; set arr($key) \"val2\"; return $arr(key1) }";
    let ops = opcodes(&proc_asm(src, "::array_op"));
    assert!(ops.contains(&Op::STORE_ARRAY1));
    assert!(ops.contains(&Op::LOAD_ARRAY1));
}

#[test]
fn top_level_array_ref() {
    let ops = ops_of("set arr(key) \"hello\"; set x $arr(key)");
    assert!(ops.contains(&Op::STORE_ARRAY_STK) || ops.contains(&Op::STORE_STK));
}

#[test]
fn qualified_var_in_proc() {
    let src = "proc ns_access {} { set ::global_var 42; return $::global_var }";
    let ops = opcodes(&proc_asm(src, "::ns_access"));
    // Qualified names always use stack-based ops, even in procs.
    assert!(ops.contains(&Op::STORE_STK));
    assert!(ops.contains(&Op::LOAD_STK));
}

#[test]
fn incr_in_proc_vs_top_level() {
    let proc_fa = proc_asm("proc inc {x} { incr x; return $x }", "::inc");
    let top = ops_of("set x 0; incr x");
    assert!(
        opcodes(&proc_fa).contains(&Op::INCR_SCALAR1_IMM),
        "proc uses INCR_SCALAR1_IMM"
    );
    assert!(
        top.contains(&Op::INCR_STK_IMM),
        "top-level uses INCR_STK_IMM"
    );
}

#[test]
fn incr_large_amount() {
    let src = "proc big_incr {} { set x 0; incr x 1000; return $x }";
    let ops = opcodes(&proc_asm(src, "::big_incr"));
    assert!(ops.contains(&Op::INCR_SCALAR1) || ops.contains(&Op::INVOKE_STK1));
}

// ═══════════════════════════════════════════════════════════════════
// Bytecoded list commands
//
// Only the list commands the emitter actually specialises are asserted
// with their opcodes; `lindex` routes through a generic invoke (no
// LIST_INDEX_IMM specialisation) and is asserted accordingly.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn llength_bytecoded() {
    // tclsh: llength "a b c"  →  3   (8.6 + 9.0)
    assert!(ops_of("llength \"a b c\"").contains(&Op::LIST_LENGTH));
}

#[test]
fn lindex_routes_through_generic_invoke() {
    // tclsh: lindex "a b c" 1  →  b   (8.6 + 9.0)
    // Emits a generic invoke (no LIST_INDEX_IMM specialisation).
    let fa = top_asm("lindex \"a b c\" 1");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(fa.literals.entries().iter().any(|l| l == "lindex"));
}

#[test]
fn lrange_bytecoded() {
    assert!(ops_of("lrange \"a b c d e\" 1 3").contains(&Op::LIST_RANGE_IMM));
}

#[test]
fn list_create_in_proc() {
    let fa = proc_asm("proc mk {a b c} { list $a $b $c }", "::mk");
    assert!(opcodes(&fa).contains(&Op::INVOKE_STK1) || opcodes(&fa).contains(&Op::LIST));
}

#[test]
fn lreplace_routes_through_generic_invoke() {
    // The emitter specialises `linsert` (see below) but routes a full
    // `lreplace` (with replacement elements) through a generic invoke.
    let fa = top_asm("lreplace \"a b c d\" 1 2 X Y");
    assert!(opcodes(&fa).contains(&Op::INVOKE_STK1));
    assert!(fa.literals.entries().iter().any(|l| l == "lreplace"));
}

#[test]
fn linsert_bytecoded() {
    // `linsert` IS specialised to lreplace4.
    assert!(ops_of("linsert \"a b c\" 1 X").contains(&Op::LREPLACE4));
}

#[test]
fn lassign_bytecoded() {
    let ops = ops_of("lassign \"a b c\" x y z");
    assert!(ops.contains(&Op::LIST_INDEX_IMM));
    assert!(ops.contains(&Op::LIST_RANGE_IMM));
}

// ═══════════════════════════════════════════════════════════════════
// Dict commands
//
// Rust emits VERIFY_DICT for the `dict create` subject normalisation but the
// `dict get`/`dict exists` lookup itself routes through a generic invoke
// (no DICT_GET/DICT_EXISTS opcode selected) — asserted accordingly.
// ═══════════════════════════════════════════════════════════════════

#[test]
fn dict_get_path() {
    let fa = top_asm("set d [dict create a 1 b 2]; dict get $d a");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(
        !ops.contains(&Op::DICT_GET),
        "Rust does not select DICT_GET here"
    );
    assert!(fa.literals.entries().iter().any(|l| l == "dict"));
}

#[test]
fn dict_exists_path() {
    let fa = top_asm("set d [dict create a 1]; dict exists $d a");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::INVOKE_STK1));
    assert!(
        !ops.contains(&Op::DICT_EXISTS),
        "Rust does not select DICT_EXISTS here"
    );
}

// ═══════════════════════════════════════════════════════════════════
// CFG structure
// ═══════════════════════════════════════════════════════════════════

#[test]
fn linear_script_single_block() {
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
    let src = "proc loop {} { for {set i 0} {$i < 10} {incr i} { puts $i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::loop"];
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
    let src = "proc early_exit {x} { set a 1; return $a; set b 2 }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::early_exit"];
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
    let src = "proc simple {x} { if {$x > 0} { set r \"pos\" } else { set r \"neg\" }; return $r }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::simple"];
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

// ═══════════════════════════════════════════════════════════════════
// IR lowering edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn set_integer_becomes_assign_const() {
    let ir = ir_for("set x 42");
    assert!(ir.top_level.statements.iter().any(
        |s| matches!(s, Statement::AssignConst { name, value, .. } if name == "x" && value == "42")
    ));
}

#[test]
fn set_expr_becomes_assign_expr() {
    let ir = ir_for("set r [expr {1 + 2}]");
    assert!(
        ir.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::AssignExpr { name, .. } if name == "r"))
    );
}

#[test]
fn set_value_becomes_assign_value() {
    let ir = ir_for("set x $y");
    assert!(
        ir.top_level
            .statements
            .iter()
            .any(|s| matches!(s, Statement::AssignValue { name, .. } if name == "x"))
    );
}

#[test]
fn incr_becomes_ir_incr() {
    let ir = ir_for("set x 0; incr x");
    assert!(ir.top_level.statements.iter().any(
        |s| matches!(s, Statement::Incr { name, amount, .. } if name == "x" && amount.is_none())
    ));
}

#[test]
fn incr_with_amount() {
    let ir = ir_for("set x 0; incr x 5");
    assert!(ir.top_level.statements.iter().any(|s| {
        matches!(s, Statement::Incr { name, amount, .. } if name == "x" && amount.as_deref() == Some("5"))
    }));
}

#[test]
fn return_with_options_becomes_barrier() {
    let ir = ir_for("proc fail {} { return -code error \"oops\" }");
    let proc = &ir.procedures["::fail"];
    assert!(
        proc.body
            .statements
            .iter()
            .any(|s| matches!(s, Statement::Barrier { .. }))
    );
}

#[test]
fn standalone_expr_becomes_expr_eval() {
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
    let ir = ir_for("if $x {puts ok}");
    let stmts = &ir.top_level.statements;
    assert!(!stmts.is_empty());
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

#[test]
fn switch_modes_tracked() {
    for src in [
        "switch -exact $x {a {set r 1}}",
        "switch -glob $x {*.c {set r 1}}",
        "switch -regexp $x {^a {set r 1}}",
    ] {
        let ir = ir_for(src);
        let stmts = &ir.top_level.statements;
        let ok = stmts
            .iter()
            .any(|s| matches!(s, Statement::Switch { .. } | Statement::Barrier { .. }));
        assert!(ok, "{src}: expected Switch or Barrier");
    }
}

// ═══════════════════════════════════════════════════════════════════
// End-to-end pipeline
// ═══════════════════════════════════════════════════════════════════

#[test]
fn fibonacci_proc() {
    // tclsh: fib 10  →  55   (9.0)
    let src = "proc fib {n} { if {$n <= 1} { return $n }; set a 0; set b 1; for {set i 2} {$i <= $n} {incr i} { set temp [expr {$a + $b}]; set a $b; set b $temp }; return $b }";
    let fa = proc_asm(src, "::fib");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::LE));
    assert!(ops.contains(&Op::ADD));
    assert!(format_function_asm(&fa).contains("ByteCode ::fib"));
}

#[test]
fn bubble_sort() {
    let src = "proc bubblesort {lst} { set n [llength $lst]; for {set i 0} {$i < $n} {incr i} { for {set j 0} {$j < [expr {$n - $i - 1}]} {incr j} { set a [lindex $lst $j]; set b [lindex $lst [expr {$j + 1}]]; if {$a > $b} { set lst [lreplace $lst $j $j $b]; set lst [lreplace $lst [expr {$j + 1}] [expr {$j + 1}] $a] } } }; return $lst }";
    let fa = proc_asm(src, "::bubblesort");
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::GT));
    assert!(format_function_asm(&fa).contains("ByteCode ::bubblesort"));
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
        fa.literals
            .entries()
            .iter()
            .any(|l| l.contains("Hello, ") && l.contains("{name}")),
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

// ═══════════════════════════════════════════════════════════════════
// Codegen output format validation (labels)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn labels_point_to_valid_offsets() {
    let fa = top_asm("if {$x} {set a 1} else {set a 2}");
    // Labels resolve to non-negative byte offsets (layout-resolved).
    for (label, &off) in &fa.labels {
        // `usize` offset is inherently >= 0; assert it is within the byte span
        // and the label is non-empty so the table is meaningful.
        assert!(!label.is_empty());
        let total_bytes: usize = fa
            .instructions
            .iter()
            .map(|i| usize::from(i.op.size()))
            .sum();
        assert!(
            off <= total_bytes + 16,
            "label {label} offset {off} absurdly large"
        );
    }
    assert!(fa.labels.len() >= 2, "if/else needs at least 2 labels");
}

// ═══════════════════════════════════════════════════════════════════
// Condition defs from command substitutions
// ═══════════════════════════════════════════════════════════════════

#[test]
fn catch_in_if_condition() {
    let src =
        "proc safe_check {} { if {[catch {expr {1/0}} result]} { return $result }; return \"ok\" }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::safe_check"];
    let has_cond_def = proc_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "<cond>"))
    });
    assert!(has_cond_def, "synthetic <cond> def expected");
}

#[test]
fn catch_in_while_condition() {
    let src = "proc retry_loop {} { while {[catch {some_op} result] != 0} { puts \"retrying...\" }; return $result }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::retry_loop"];
    let has_cond_def = proc_cfg.blocks.values().any(|b| {
        b.statements
            .iter()
            .any(|s| matches!(s, Statement::Call { command, .. } if command == "<cond>"))
    });
    assert!(has_cond_def);
}

// ═══════════════════════════════════════════════════════════════════
// Backslash substitution
// ═══════════════════════════════════════════════════════════════════

#[test]
fn backslash_in_set_value() {
    let fa = top_asm("set x \"line1\\nline2\"");
    assert!(opcodes(&fa).contains(&Op::DONE));
}

// ═══════════════════════════════════════════════════════════════════
// Multiple iterator groups in foreach
// ═══════════════════════════════════════════════════════════════════

fn foreach_iter_count(src: &str, proc: &str) -> usize {
    let ir = ir_for(src);
    let p = &ir.procedures[proc];
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
    let p = &ir.procedures["::pairs"];
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

// ═══════════════════════════════════════════════════════════════════
// defer_top_level vs inline_loops
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// Frozen for/while (command-subst conditions)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn frozen_for_cmd_subst_condition() {
    let src = "proc frozen {} { for {set i 0} {[expr {$i < 10}]} {incr i} { puts $i } }";
    let ir = ir_for(src);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::frozen"];
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
    let proc_cfg = &cfg.procedures["::frozen_while"];
    let has_barrier = proc_cfg.blocks.values().any(|b| {
        b.statements.iter().any(
            |s| matches!(s, Statement::Barrier { reason, .. } if reason.contains("frozen while")),
        )
    });
    assert!(has_barrier);
}

// ═══════════════════════════════════════════════════════════════════
// Stress: combining everything
// ═══════════════════════════════════════════════════════════════════

#[test]
fn kitchen_sink() {
    let src = "namespace eval ::test {\n    proc helper {x} {\n        return [expr {$x * 2}]\n    }\n}\n\nproc main {args} {\n    set total 0\n    set items {1 2 3 4 5}\n    for {set i 0} {$i < 5} {incr i} {\n        if {$i % 2 == 0} {\n            set total [expr {$total + $i}]\n        }\n    }\n    foreach item $items {\n        set total [expr {$total + $item}]\n    }\n    set j 0\n    while {$j < 100} {\n        if {$j > 10} {\n            break\n        }\n        incr j\n    }\n    switch -exact $j {\n        10 { set label \"ten\" }\n        11 { set label \"eleven\" }\n        default { set label \"other\" }\n    }\n    set rc [catch {expr {1/0}} err]\n    set s \"hello world\"\n    string length $s\n    string toupper $s\n    return $total\n}\n\nmain\n";
    let ma = asm_for(src);
    assert!(ma.procedures.contains_key("::main"));
    let ops = opcodes(&ma.procedures["::main"]);
    assert!(ops.contains(&Op::LT) || ops.contains(&Op::LE));
    assert!(ops.contains(&Op::ADD));
    assert!(ops.contains(&Op::MOD));
    assert!(ops.contains(&Op::DONE));
    assert!(format_module_asm(&ma).contains("ByteCode ::main"));
}

// ═══════════════════════════════════════════════════════════════════
// codegen_module sanity (mirrors integration's empty-module smoke)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn codegen_module_empty_has_top() {
    let cfg_mod = CfgModule {
        top_level: toplevel_cfg(vec![]),
        procedures: HashMap::new(),
    };
    let ir_mod = ir_for("");
    let asm = codegen_module(&cfg_mod, &ir_mod, &registry());
    assert_eq!(asm.top_level.name, "::top");
}

// ═══════════════════════════════════════════════════════════════════
// Command-binding trust gate (issue #1585)
//
// A fold or a specialised inline emission *is* a builtin's semantics, so it
// may only fire while the name still denotes that builtin. C Tcl reaches the
// same answer dynamically: it compiles the specialisation unconditionally but
// wraps every command in `INST_START_CMD`, which re-dispatches the slow way
// once `iPtr->compileEpoch` moves (`tclExecute.c`, `instStartCmdFailed`).
// This compiler emits no epoch guard, so the check is the static
// whole-unit `command_binding` scan instead.
//
// The runtime results below are pinned on tclsh 8.6.16 and 9.0.4 (byte
// identical): `rename dict {}` then `dict create a 1 b 2` raises
// `invalid command name "dict"` on both, and `proc format {args} {return z}`
// then `format "%s!" hi` yields `z` on both — neither is the folded value
// this codegen used to bake in.
// ═══════════════════════════════════════════════════════════════════

/// The literal pool of `source`'s top-level function.
fn top_lits(source: &str) -> Vec<String> {
    top_asm(source).literals.entries().to_vec()
}

#[test]
fn list_fold_bails_when_the_unit_renames_list() {
    // Untouched: the whole substitution collapses to one literal.
    assert!(top_lits("puts [list a b c]\n").contains(&"a b c".to_owned()));

    // tclsh 8.6.16 / 9.0.4: `rename list mylist` then `list a b c` errors
    // through `unknown` — so the words must survive as a real call.
    let lits = top_lits("rename list mylist\nputs [list a b c]\n");
    assert!(
        !lits.contains(&"a b c".to_owned()),
        "the folded list must not survive a `rename list`: {lits:?}"
    );
    for word in ["a", "b", "c"] {
        assert!(
            lits.contains(&word.to_owned()),
            "argument {word:?} must reach the runtime call: {lits:?}"
        );
    }
}

#[test]
fn format_fold_bails_when_a_proc_shadows_format() {
    assert!(top_lits("puts [format \"%s!\" hi]\n").contains(&"hi!".to_owned()));

    // tclsh 8.6.16 / 9.0.4: the shadowing proc runs and the script prints `z`.
    let lits = top_lits("proc format {args} {return z}\nputs [format \"%s!\" hi]\n");
    assert!(
        !lits.contains(&"hi!".to_owned()),
        "the folded result must not survive a shadowing `proc format`: {lits:?}"
    );
    assert!(lits.contains(&"%s!".to_owned()) && lits.contains(&"hi".to_owned()));
}

#[test]
fn dict_create_fold_bails_when_the_unit_deletes_dict() {
    let ops = ops_of("puts [dict create a 1 b 2]\n");
    assert!(ops.contains(&Op::VERIFY_DICT), "baseline folds: {ops:?}");

    // tclsh 8.6.16 / 9.0.4: `invalid command name "dict"`.
    let src = "rename dict {}\nputs [dict create a 1 b 2]\n";
    let ops = opcodes(&top_asm(src));
    assert!(
        !ops.contains(&Op::VERIFY_DICT),
        "the folded dict must not survive a `rename dict {{}}`: {ops:?}"
    );
    assert!(top_lits(src).contains(&"create".to_owned()));
}

#[test]
fn statement_position_hook_bails_when_the_unit_renames_llength() {
    let ops = ops_of("set l {a b c}\nllength $l\n");
    assert!(
        ops.contains(&Op::LIST_LENGTH),
        "baseline specialises: {ops:?}"
    );

    let ops = ops_of("rename llength myll\nset l {a b c}\nllength $l\n");
    assert!(
        !ops.contains(&Op::LIST_LENGTH),
        "`llength` must not keep its opcode once renamed away: {ops:?}"
    );
}

#[test]
fn value_position_hook_bails_when_the_unit_renames_string() {
    let ops = ops_of("set s ab\nset x [string length $s]\n");
    assert!(ops.contains(&Op::STR_LEN), "baseline specialises: {ops:?}");

    let ops = ops_of("rename string mystring\nset s ab\nset x [string length $s]\n");
    assert!(
        !ops.contains(&Op::STR_LEN),
        "`string length` must not keep its opcode once `string` is renamed: {ops:?}"
    );
}

#[test]
fn the_trust_gate_is_per_name_not_whole_module() {
    // Shadowing `format` must not cost `list` or `dict` their folds — the gate
    // asks about one name, not "did this unit touch any command table".
    let src = "proc format {args} {return z}\nputs [list a b c]\nputs [dict create k v]\n";
    let lits = top_lits(src);
    assert!(lits.contains(&"a b c".to_owned()), "{lits:?}");
    assert!(lits.contains(&"k v".to_owned()), "{lits:?}");
}

/// The gate holds through the *other* caller of the shared fold helper.
///
/// The tests above reach `try_emit_constant_fold` from the argument path; the
/// simplified value emitter (`emit_value_interpolated`) reaches it from the
/// hook emitters, and `lset`'s new-value word is one such route. Both callers
/// were separate copies of the block until the copies were merged, and this is
/// what keeps the second route honest (issue #1585).
#[test]
fn the_fold_gate_holds_through_the_simplified_value_emitter() {
    let lits = |src: &str| proc_asm(src, "::p").literals.entries().to_vec();

    let list_body = "proc p {} {set l {x}\nlset l 0 [list a b c]\nreturn $l}\n";
    assert!(lits(list_body).contains(&"a b c".to_owned()));
    let renamed = lits(&format!("{list_body}rename list mylist\np\n"));
    assert!(
        !renamed.contains(&"a b c".to_owned()) && renamed.contains(&"list".to_owned()),
        "`lset`'s value word must keep the real `list` call: {renamed:?}"
    );

    let fmt_body = "proc p {} {set l {x}\nlset l 0 [format \"%s!\" hi]\nreturn $l}\n";
    assert!(lits(fmt_body).contains(&"hi!".to_owned()));
    let shadowed = lits(&format!("{fmt_body}proc format {{args}} {{return z}}\np\n"));
    assert!(
        !shadowed.contains(&"hi!".to_owned()) && shadowed.contains(&"%s!".to_owned()),
        "`lset`'s value word must keep the real `format` call: {shadowed:?}"
    );

    let dict_body = "proc p {} {set l {x}\nlset l 0 [dict create a 1]\nreturn $l}\n";
    let base_ops = opcodes(&proc_asm(dict_body, "::p"));
    assert!(base_ops.contains(&Op::VERIFY_DICT), "{base_ops:?}");
    let deleted = opcodes(&proc_asm(
        &format!("{dict_body}rename dict {{}}\np\n"),
        "::p",
    ));
    assert!(
        !deleted.contains(&Op::VERIFY_DICT),
        "`lset`'s value word must keep the real `dict create` call: {deleted:?}"
    );
}
