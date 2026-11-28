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
//! Bytecode-shape tests take the unit a live Tcl 8.6 interpreter compiles
//! (`vm_compiled`): a module lowered without an interpreter proves no command
//! and emits plain dispatch. IR and CFG shape tests use `lower_to_ir` →
//! `build_cfg`, and the hand-built CFG tests `codegen_function`.
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
//!  1. `LiteralTable.entries()` returns exact native byte entries (ordered, deduped) and
//!     `LocalVarTable` likewise.
//!  2. `esc` escaping. `format::esc` matches C-Tcl's disassembler BYTE-WISE
//!     over UTF-8: a non-ASCII codepoint renders as the `\uXXXX` of each raw
//!     UTF-8 byte (so `ÿ` U+00FF → `Ã¿`, an astral char → four `\u00XX`
//!     bytes), not codepoint-wise. The named-control, NUL, C0, DEL, quote, and
//!     printable-ASCII cases are unaffected; the high-BMP / supplementary-plane
//!     cases assert the byte-wise expectation and are annotated.
//!  3. Braced whole-name array ref `${a($i)}` vs bare `$a($i)`. The emitter
//!     does NOT carry the braced-vs-bare whole-name/split distinction — both
//!     spellings lower to a split `loadArray1` — so those 11 tclsh-9.0.3-pinned
//!     cases are documented here only.
//!  4. The `emit_binop`/`emit_unaryop` iRules harness builds a CFG by hand: a
//!     `CfgFunction` with a `Statement::AssignExpr` carrying an
//!     `ExprNode::Binary`/`Unary`, run through `codegen_function`.

use crate::body_identity;
use body_identity::{fixture_id, fixture_id_if_present};

use std::collections::HashMap;

use tcl_codegen::codegen::format::{esc, format_function_asm, format_module_asm};
use tcl_codegen::codegen::{
    FunctionAsm, LiteralTable, LocalVarTable, Op, Operand, codegen_function, codegen_module,
};
use tcl_compiler::cfg::{CfgModule, Function as CfgFunction, Terminator};
use tcl_compiler::cfg_builder::{build_cfg, build_cfg_codegen};
use tcl_compiler::expr_ast::{BinOp, ExprNode, UnaryOp};
use tcl_compiler::ir::{Module as IrModule, Statement};
use tcl_compiler::lowering::lower_to_ir;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

mod bytecoded_commands;
mod control_flow;
mod disassembly_format;
mod end_to_end;
mod expressions;
mod fold_trust;
mod loops;
mod lowering_and_cfg;
mod procedures;
mod simple_commands;

// Helpers.

fn registry() -> CommandRegistry {
    CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn sp() -> Span {
    Span::new(0, 0)
}

/// Lower `source` → IR → CFG (codegen flavour, defer top-level) → `ModuleAsm`.
fn asm_for(source: &str) -> tcl_codegen::codegen::ModuleAsm {
    let reg = registry();
    let ir = lower_to_ir(source, &reg);
    let cfg = build_cfg_codegen(&ir, true);
    codegen_module(&cfg, &ir, &reg).expect("original source namespace")
}

/// The release these fixtures compile under.
const RELEASE: &str = "tcl8.6";

/// The top-level function a Tcl 8.6 interpreter compiles for `source`.
fn top_asm(source: &str) -> FunctionAsm {
    crate::vm_compiled::top_level(RELEASE, source)
}

/// The body a Tcl 8.6 interpreter compiles for procedure `name` of `source`.
fn proc_asm(source: &str, name: &str) -> FunctionAsm {
    crate::vm_compiled::procedure(RELEASE, source, name)
}

/// Whether the pool holds the folded integer `value`. A release keeps a
/// constant expression's result as a number that has no string yet, so the
/// entry is read by its numeric payload.
fn folds_to_integer(fa: &FunctionAsm, value: i64) -> bool {
    use tcl_bytecode::{NativeExpressionNumberLiteral, NativeLiteralAllocation};
    fa.literals.entries().iter().any(|literal| {
        matches!(
            literal.allocation(),
            NativeLiteralAllocation::PrivateExpressionNumber {
                value: NativeExpressionNumberLiteral::Integer(folded),
                ..
            } if *folded == value
        )
    })
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
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
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
        command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
        expr_base: None,
        fallback_value: "[expr {$a + $b}]".into(),
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
        command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
        expr_base: None,
        fallback_value: "[expr {-$a}]".into(),
    }]);
    opcodes(&codegen_function(&cfg, &[], false, &registry()))
}

// Dict commands
//
// Rust emits VERIFY_DICT for the `dict create` subject normalisation but the
// `dict get`/`dict exists` lookup itself routes through a generic invoke
// (no DICT_GET/DICT_EXISTS opcode selected) — asserted accordingly.
