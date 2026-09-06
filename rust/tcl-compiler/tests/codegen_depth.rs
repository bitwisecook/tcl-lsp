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

//! Depth coverage for the `tcl-compiler` codegen *lowering* modules that the
//! existing `tests/codegen.rs` leaves thin:
//!
//! - `src/codegen/backend.rs` — the agnostic `Backend` trait + the
//!   `BytecodeBackend` impl (0% before).
//! - `src/codegen/control_flow.rs` — catch/try lowering: `emit_catch_*`,
//!   `emit_try_on_error_inline`, `emit_try_finally_inline`,
//!   `detect_const_expr_error`.
//! - `src/codegen/cmd_subst.rs` — command substitutions in every position
//!   (arg / expr / nested `[a [b]]` / condition / list),
//!   `try_list_expand_concat`, `try_inline_list_with_break_continue`, `{*}`
//!   expansion, and the multi-command `EVAL_STK` fallback.
//! - `src/codegen/emitter/try_blocks.rs` — the try/finally CFG-chain detection
//!   walked end-to-end by the emitter.
//! - `codegen/values.rs` / `statements.rs` headroom via the same snippets.
//!
//! ## Method
//!
//! Unlike `codegen.rs` (which calls the free `codegen_module` /
//! `codegen_function`), this suite drives codegen **through the `Backend`
//! trait** — `BytecodeBackend::lower_module` / `lower_function` — which is what
//! exercises `backend.rs`. A handful of the deepest lowering helpers
//! (`emit_try_on_error_inline`, `try_list_expand_concat`,
//! `try_inline_list_with_break_continue`) are also driven directly on a
//! hand-built `CodegenCtx` so the branch is hit even where the surface-syntax
//! path that reaches it is narrow.
//!
//! ## Proof split
//!
//! Opcode-shape and block-structure assertions are **compiler-internal** —
//! they describe how the emitter lays out bytecode, not a Tcl runtime value —
//! and are asserted structurally (this is the bulk of the file; the `tcl-vm`
//! crate is not a `tcl-compiler` dependency, so no VM execution is available
//! here and the emitted shape is the thing under test).
//!
//! Where a snippet computes an **observable value** that the emitter folds at
//! compile time, the folded literal is the observable result and is pinned to
//! C-Tcl (`scripts/dev/tclsh_check.sh`, 8.6 + 9.0) with a `// tclsh:` comment.
//!
//! Does NOT duplicate `codegen.rs`: that suite covers the main emitter
//! path and the `if`/`for`/`while`/`switch`/`proc` happy paths via the free
//! functions; this one covers the *lowering branches* those tests skip, and the
//! `Backend`-trait entry points they never touch.

use tcl_compiler::cfg::{CfgModule, Function as CfgFunction, Terminator};
use tcl_compiler::cfg_builder::{build_cfg, build_cfg_codegen};
use tcl_compiler::codegen::{
    Backend, BytecodeBackend, CodegenCtx, FunctionAsm, ModuleAsm, Op, Operand,
};
use tcl_compiler::ir::{Module as IrModule, Procedure, Statement};
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

// ── Helpers ────────────────────────────────────────────────────────────────

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// Lower `source` → IR → CFG (codegen flavour, defer top-level) → `ModuleAsm`,
/// **through the `BytecodeBackend` trait** (`lower_module`). This is the
/// `backend.rs` entry point, distinct from `codegen.rs`'s free-function
/// `codegen_module`.
fn module_via_backend(source: &str) -> (ModuleAsm, IrModule) {
    let reg = registry();
    let ir = lower_to_ir(source, &reg);
    let cfg = build_cfg_codegen(&ir, true);
    let mut backend = BytecodeBackend;
    let asm = backend.lower_module(&cfg, &ir, &reg);
    (asm, ir)
}

/// As [`module_via_backend`] but with `defer_top_level = false`, so top-level
/// loops/switch/try are inlined into native block structure (the shape the
/// proc bodies also get).
fn module_inline_via_backend(source: &str) -> ModuleAsm {
    let reg = registry();
    let ir = lower_to_ir(source, &reg);
    let cfg = build_cfg(&ir, false);
    let mut backend = BytecodeBackend;
    backend.lower_module(&cfg, &ir, &reg)
}

/// The `FunctionAsm` for a named procedure, lowered through the backend.
fn proc_via_backend(source: &str, qname: &str) -> FunctionAsm {
    module_inline_via_backend(source)
        .procedures
        .remove(qname)
        .unwrap_or_else(|| panic!("procedure {qname} not found"))
}

fn opcodes(fa: &FunctionAsm) -> Vec<Op> {
    fa.instructions.iter().map(|i| i.op).collect()
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

/// A `CodegenCtx` for driving lowering helpers directly (proc flavour).
fn ctx_with_params<'r>(reg: &'r CommandRegistry, params: &[&str]) -> CodegenCtx<'r> {
    CodegenCtx::new(true, params, reg)
}

fn ctx_ops(ctx: &CodegenCtx) -> Vec<Op> {
    ctx.instructions.iter().map(|i| i.op).collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// backend.rs — the `Backend` trait + `BytecodeBackend` (was 0%)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn backend_lower_module_names_top_level() {
    // `BytecodeBackend::lower_module` delegates to `codegen_module`; assert the
    // module artifact shape (top-level named `::top`, the proc present).
    let (asm, _) = module_via_backend("proc add {a b} { return [expr {$a + $b}] }\nadd 1 2");
    assert_eq!(asm.top_level.name, "::top");
    assert!(asm.procedures.contains_key("::add"));
    assert!(opcodes(&asm.procedures["::add"]).contains(&Op::ADD));
    // The top-level script ends cleanly with done.
    assert_eq!(asm.top_level.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn backend_lower_module_empty() {
    // Empty source through the trait: a top-level `::top` with just `done`,
    // and no procedures.
    let (asm, _) = module_via_backend("");
    assert_eq!(asm.top_level.name, "::top");
    assert!(asm.procedures.is_empty());
    assert_eq!(asm.top_level.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn backend_associated_types_are_function_and_module_asm() {
    // The trait's associated artifact types are `FunctionAsm` / `ModuleAsm`.
    // Bind them explicitly so the test fails to compile if the impl's
    // associated types ever drift.
    let reg = registry();
    let ir = lower_to_ir("set x 1", &reg);
    let cfg = build_cfg_codegen(&ir, true);
    let mut backend = BytecodeBackend;
    let f: <BytecodeBackend as Backend>::FuncArtifact =
        backend.lower_function(&cfg.top_level, &[], false, &[], &reg);
    let m: <BytecodeBackend as Backend>::ModuleArtifact = backend.lower_module(&cfg, &ir, &reg);
    assert_eq!(f.name, "::top");
    assert_eq!(m.top_level.name, "::top");
}

#[test]
fn backend_lower_function_proc_flavour_uses_lvt() {
    // `lower_function(.., is_proc=true, ..)` selects LVT-based variable access.
    // A returned parameter loads via loadScalar1 and the LVT has it at slot 0.
    let reg = registry();
    let ir = lower_to_ir("proc id {x} { return $x }", &reg);
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::id"];
    let mut backend = BytecodeBackend;
    let fa = backend.lower_function(proc_cfg, &["x"], true, &[], &reg);
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::LOAD_SCALAR1),
        "proc flavour loads via LVT: {ops:?}"
    );
    assert_eq!(fa.lvt.entries()[0], "x");
    assert_eq!(fa.instructions.last().unwrap().op, Op::DONE);
}

#[test]
fn backend_lower_function_script_flavour_uses_stack() {
    // `lower_function(.., is_proc=false, ..)` selects stack-based access
    // (storeStk / loadStk) and leaves the LVT empty.
    let reg = registry();
    let ir = lower_to_ir("set x 1", &reg);
    let cfg = build_cfg(&ir, false);
    let mut backend = BytecodeBackend;
    let fa = backend.lower_function(&cfg.top_level, &[], false, &[], &reg);
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::PUSH1));
    assert!(
        ops.contains(&Op::STORE_STK),
        "script flavour stores via stack: {ops:?}"
    );
    assert_eq!(fa.lvt.entries().len(), 0, "no LVT at script scope");
}

#[test]
fn backend_lower_function_interleaves_proc_defs() {
    // The `proc_defs` parameter interleaves pending proc definitions into the
    // top-level stream (the same job `codegen_module` does). Driving it through
    // the trait with a populated `proc_defs` slice emits the `proc` definition
    // command (a `proc <name> <args> <body>` invoke) inline.
    let reg = registry();
    let ir = lower_to_ir("proc foo {} { return 1 }\nfoo", &reg);
    let cfg = build_cfg_codegen(&ir, true);
    let proc_defs: Vec<Procedure> = ir.procedures.values().cloned().collect();
    assert!(
        !proc_defs.is_empty(),
        "expected a pending proc def to interleave"
    );
    let mut backend = BytecodeBackend;
    let top = backend.lower_function(&cfg.top_level, &[], false, &proc_defs, &reg);
    // The interleaved proc definition pushes the literal `proc` + the proc name
    // and invokes — distinct from the no-proc-defs lowering of the same CFG.
    let lits = top.literals.entries();
    assert!(
        lits.iter().any(|l| l == "proc"),
        "proc-def literal `proc`: {lits:?}"
    );
    assert!(
        lits.iter().any(|l| l == "foo"),
        "proc-def names `foo`: {lits:?}"
    );
    assert!(opcodes(&top).contains(&Op::INVOKE_STK1));
    assert_eq!(top.instructions.last().unwrap().op, Op::DONE);
}

#[test]
#[allow(clippy::default_constructed_unit_structs)] // exercising the Default impl is the point
fn backend_default_and_copy() {
    // `BytecodeBackend` is a zero-sized `Default + Copy` handle; two instances
    // produce identical artifacts for the same input.
    let reg = registry();
    let ir = lower_to_ir("set x 1; set y 2", &reg);
    let cfg = build_cfg_codegen(&ir, true);
    let mut a = BytecodeBackend;
    let mut b = BytecodeBackend::default();
    let asm_a = a.lower_module(&cfg, &ir, &reg);
    let asm_b = b.lower_module(&cfg, &ir, &reg);
    assert_eq!(
        opcodes(&asm_a.top_level),
        opcodes(&asm_b.top_level),
        "the same backend produces a deterministic artifact"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// control_flow.rs — catch lowering (emit_catch_body branches)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn catch_body_break_emits_break_op() {
    // `catch {break}` in a proc body: emit_catch_body's `"break"` arm emits a
    // BREAK opcode inside the catch range.
    let fa = proc_via_backend(
        "proc cb {} { for {set i 0} {$i < 3} {incr i} { set rc [catch {break} e] }; return $i }",
        "::cb",
    );
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::BREAK), "catch {{break}} → BREAK: {ops:?}");
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(ops.contains(&Op::END_CATCH));
}

#[test]
fn catch_body_error_emits_return_imm() {
    // `catch {error "boom"}`: emit_catch_error pushes the message + empty opts
    // and emits RETURN_IMM(code=1, level=0) inside the catch range.
    let fa = proc_via_backend(
        "proc ce {} { set rc [catch {error \"boom\"} e]; return $e }",
        "::ce",
    );
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(
        ops.contains(&Op::RETURN_IMM),
        "catch {{error}} → RETURN_IMM: {ops:?}"
    );
    assert!(
        ops.contains(&Op::PUSH_RESULT),
        "handler path pushes the caught result"
    );
    assert!(fa.literals.entries().iter().any(|l| l == "boom"));
}

#[test]
fn catch_body_return_with_code_emits_return_imm() {
    // `catch {return -code error "x"}`: emit_catch_return parses `-code error`
    // and emits RETURN_IMM.
    let fa = proc_via_backend(
        "proc cr {} { set rc [catch {return -code error \"x\"} e]; return $rc }",
        "::cr",
    );
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::RETURN_IMM),
        "catch {{return -code …}} → RETURN_IMM: {ops:?}"
    );
    assert!(ops.contains(&Op::BEGIN_CATCH4));
}

#[test]
fn catch_body_const_div_zero_emits_syntax() {
    // `catch {expr {1/0}}`: detect_const_expr_error fires (constant divide by
    // zero) and emit_catch_body pushes the message + options and emits SYNTAX
    // rather than evaluating the (always-erroring) expression.
    let fa = proc_via_backend(
        "proc s {} { set rc [catch {expr {1/0}} e]; return $rc }",
        "::s",
    );
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::SYNTAX),
        "constant 1/0 in catch → SYNTAX: {ops:?}"
    );
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(fa.literals.entries().iter().any(|l| l == "divide by zero"));
}

/// The `(code, level)` immediates `emit_catch_return` puts on `returnImm`, per
/// C's `TclMergeReturnOptions` → `CompileReturnInternal`
/// (`tclResult.c:866-974`, `tclCompCmdsGR.c:2410`).
///
/// Two normalisations produce the whole table: `-level` defaults to **1** (so a
/// plain `return` is `(0, 1)`, never `(0, 0)` — that pair means "push the result
/// and fall through"), and `-code return` is rewritten to `-code ok -level L+1`,
/// so `TCL_RETURN` never reaches the operand.
///
/// The `break`/`continue`/non-standard rows previously encoded the code *as the
/// level* (`(0, 3)`, `(0, 4)`), which the old compensating VM read back as the
/// right completion by accident; under C semantics they are levels, so they
/// would have unwound N proc frames instead of breaking a loop.
#[test]
fn catch_return_operands_match_c_merge_table() {
    for (body, want) in [
        // plain `return` inside a catch range: C emits `returnImm 0 1`
        // (the `done` fold needs "no enclosing catch").
        ("return", (0, 1)),
        ("return x", (0, 1)),
        // `-code error` → `(1, 1)`.
        ("return -code error x", (1, 1)),
        // `-code return` → merged to `-code ok -level 2` → `(0, 2)`.
        ("return -code return x", (0, 2)),
        ("return -code return -level 3 x", (0, 4)),
        // `-code break`/`-code continue` keep the code on the operand.
        ("return -code break x", (3, 1)),
        ("return -code continue x", (4, 1)),
        // a non-standard code rides the operand too.
        ("return -code 7 x", (7, 1)),
        // an explicit `-level` is used verbatim.
        ("return -level 0 -code error x", (1, 0)),
        ("return -level 2 -code error x", (1, 2)),
        // `return -level 0` alone: C elides the instruction entirely, and
        // `(0, 0)` is the equivalent "push the result and continue".
        ("return -level 0 x", (0, 0)),
    ] {
        let src = format!("proc p {{}} {{ set rc [catch {{{body}}} e]; return $rc }}");
        let fa = proc_via_backend(&src, "::p");
        let ri = fa
            .instructions
            .iter()
            .find(|i| i.op == Op::RETURN_IMM)
            .unwrap_or_else(|| panic!("no returnImm for `{body}`: {:?}", opcodes(&fa)));
        assert_eq!(
            ri.operands,
            vec![Operand::Imm(want.0), Operand::Imm(want.1)],
            "`{body}` must compile to returnImm {} {}",
            want.0,
            want.1
        );
    }
}

/// `error msg` is `returnImm 1 0` — an immediate `TCL_ERROR`, not a level-1
/// return (`TclCompileErrorCmd`, `tclCompCmds.c:2470`). Unchanged by the
/// realignment; pinned here so the two encodings cannot drift together.
#[test]
fn catch_error_operands_are_code_error_level_zero() {
    let fa = proc_via_backend(
        "proc e {} { set rc [catch {error boom} m]; return $rc }",
        "::e",
    );
    let ri = fa
        .instructions
        .iter()
        .find(|i| i.op == Op::RETURN_IMM)
        .expect("error → returnImm");
    assert_eq!(ri.operands, vec![Operand::Imm(1), Operand::Imm(0)]);
}

/// A top-level plain `return` is `returnImm 0 1`, and the proc-body one folds to
/// `done` (`TclCompileReturnCmd`'s `INST_DONE` optimisation).
#[test]
fn plain_return_encodings_top_level_and_proc() {
    let top = module_inline_via_backend("return ok").top_level;
    let ri = top
        .instructions
        .iter()
        .find(|i| i.op == Op::RETURN_IMM)
        .unwrap_or_else(|| panic!("top-level return → returnImm: {:?}", opcodes(&top)));
    assert_eq!(ri.operands, vec![Operand::Imm(0), Operand::Imm(1)]);

    let body = proc_via_backend("proc p {} { return ok }", "::p");
    let ops = opcodes(&body);
    assert!(
        ops.contains(&Op::DONE) && !ops.contains(&Op::RETURN_IMM),
        "a proc's plain return folds to done: {ops:?}"
    );
}

#[test]
fn catch_with_options_var_pushes_return_opts() {
    // 3-arg `catch {…} result opts`: the options-var path emits
    // pushReturnOpts and stores both the options and the result variables.
    let fa = proc_via_backend(
        "proc safe {cmd} { set rc [catch {eval $cmd} result opts]; return $rc }",
        "::safe",
    );
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::PUSH_RETURN_OPTS),
        "3-arg catch → pushReturnOpts: {ops:?}"
    );
    assert!(
        count(&ops, Op::STORE_SCALAR1) >= 2,
        "stores both result and opts"
    );
}

#[test]
fn catch_inline_direct_two_arg_shape() {
    // Drive emit_catch_inline directly: 2-arg catch jumps over the handler to a
    // catch_end label and stores the result, leaving the return code on TOS.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    ctx.emit_catch_inline("set x 1", Some("result"), None);
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::BEGIN_CATCH4));
    assert!(ops.contains(&Op::END_CATCH));
    assert!(
        ops.contains(&Op::STORE_SCALAR1),
        "result var stored: {ops:?}"
    );
    assert!(
        ops.contains(&Op::REVERSE),
        "the [result, code] pair is reversed before store"
    );
    // 2-arg form must not push return options (that is the 3-arg path).
    assert!(!ops.contains(&Op::PUSH_RETURN_OPTS));
}

#[test]
fn catch_body_empty_pushes_empty() {
    // emit_catch_body on an empty body pushes the empty literal — the degenerate
    // `catch {}` lowering.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    ctx.emit_catch_body("");
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::PUSH1],
        "empty catch body → single push of empty"
    );
    assert!(ctx.literals.entries().iter().any(String::is_empty));
}

#[test]
fn detect_const_expr_error_modulo_by_zero() {
    // detect_const_expr_error covers `%` by a constant zero as well as `/`.
    use tcl_compiler::codegen::control_flow::detect_const_expr_error;
    use tcl_compiler::expr_ast::{BinOp, ExprNode};
    let node = ExprNode::Binary {
        op: BinOp::Mod,
        left: Box::new(ExprNode::Literal {
            text: "5".into(),
            start: 0,
            end: 1,
        }),
        right: Box::new(ExprNode::Literal {
            text: "0".into(),
            start: 4,
            end: 5,
        }),
    };
    let got = detect_const_expr_error(&node);
    assert!(got.is_some(), "constant `5 % 0` is a compile-time error");
    let (msg, opts) = got.unwrap();
    assert_eq!(msg, "divide by zero");
    assert!(opts.contains("ARITH DIVZERO"));
}

#[test]
fn detect_const_expr_error_non_constant_divisor() {
    // A non-literal divisor (a variable) is not a compile-time error.
    use tcl_compiler::codegen::control_flow::detect_const_expr_error;
    use tcl_compiler::expr_ast::{BinOp, ExprNode};
    let node = ExprNode::Binary {
        op: BinOp::Div,
        left: Box::new(ExprNode::Literal {
            text: "1".into(),
            start: 0,
            end: 1,
        }),
        right: Box::new(ExprNode::Var {
            text: "$d".into(),
            name: "d".into(),
            start: 4,
            end: 6,
        }),
    };
    assert!(detect_const_expr_error(&node).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// control_flow.rs + try_blocks.rs — try/finally lowering, end-to-end
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn try_finally_emits_dual_catch_ranges() {
    // `try { body } finally { cleanup }`: emit_try_finally_inline wraps BOTH the
    // body and the finally in their own beginCatch4/endCatch ranges, converges
    // on pushReturnOpts, and re-raises via returnStk. The try_blocks.rs chain
    // detector (try_body → try_end → try_finally → try_after_finally) is what
    // routes the emitter here.
    let fa = proc_via_backend(
        "proc c {} { try { set x 1 } finally { set x 0 }; return $x }",
        "::c",
    );
    let ops = opcodes(&fa);
    assert!(
        count(&ops, Op::BEGIN_CATCH4) >= 2,
        "try + finally each get a catch range: {ops:?}"
    );
    assert!(count(&ops, Op::END_CATCH) >= 2);
    assert!(
        ops.contains(&Op::PUSH_RETURN_OPTS),
        "finally lowering pushes return opts"
    );
    assert!(
        ops.contains(&Op::RETURN_STK),
        "finally re-raises via returnStk: {ops:?}"
    );
}

#[test]
fn try_finally_during_merge_uses_list_concat() {
    // The finally lowering's `-during` option-merge path lays out the
    // `-during` key push + listConcat for merging the saved options.
    let fa = proc_via_backend(
        "proc c {} { try { set x 1 } finally { set x 0 }; return $x }",
        "::c",
    );
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::LIST_CONCAT),
        "-during merge uses listConcat: {ops:?}"
    );
    assert!(
        fa.literals.entries().iter().any(|l| l == "-during"),
        "the -during option key is interned"
    );
}

#[test]
fn try_finally_cfg_chain_present() {
    // try_blocks::detect_try_finally relies on the CFG having the
    // try_body/try_finally/try_after_finally block names; assert the chain
    // exists so the structural precondition the emitter walks is covered.
    let reg = registry();
    let ir = lower_to_ir(
        "proc c {} { try { set x 1 } finally { set x 0 }; return $x }",
        &reg,
    );
    let cfg = build_cfg(&ir, false);
    let proc_cfg = &cfg.procedures["::c"];
    let names = proc_cfg.block_names();
    assert!(
        names.iter().any(|n| n.starts_with("try_body_")),
        "try_body block: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.contains("finally")),
        "finally block: {names:?}"
    );
}

#[test]
fn try_finally_in_loop_still_lowers() {
    // A try/finally nested inside a loop must still produce its dual catch
    // ranges (the loop's break/continue targets do not disturb the try chain).
    let fa = proc_via_backend(
        "proc t {n} { set acc 0; for {set i 0} {$i < $n} {incr i} { try { incr acc $i } finally { set last $i } }; return $acc }",
        "::t",
    );
    let ops = opcodes(&fa);
    assert!(count(&ops, Op::BEGIN_CATCH4) >= 2);
    assert!(ops.contains(&Op::RETURN_STK));
    assert!(
        ops.contains(&Op::LT),
        "the enclosing for-loop condition is still emitted"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// control_flow.rs — try/on error inline (emit_try_on_error_inline)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn try_on_error_inside_catch_full_dispatch() {
    // `try {…} on error {m} {…}` nested inside a `[catch {…}]` body in a proc
    // reaches emit_try_on_error_inline (the only surface path to it). That
    // lowering: two extra beginCatch4 ranges (try body + handler body), a
    // code==1 dispatch (jumpFalse4), a `-during` dict-set merge, and a
    // returnStk re-raise.
    let fa = proc_via_backend(
        "proc p {a b} { set rc [catch {try {expr {$a/$b}} on error {m} {set z 1}} res]; return $rc }",
        "::p",
    );
    let ops = opcodes(&fa);
    assert!(
        count(&ops, Op::BEGIN_CATCH4) >= 3,
        "outer catch + try-body + handler-body catch ranges: {ops:?}"
    );
    assert!(ops.contains(&Op::JUMP_FALSE4), "code==1 dispatch: {ops:?}");
    assert!(
        ops.contains(&Op::DICT_SET),
        "-during merge dict-set: {ops:?}"
    );
    assert!(
        count(&ops, Op::RETURN_STK) >= 2,
        "matched + no-match re-raise: {ops:?}"
    );
}

#[test]
fn try_on_tcl_numeric_error_spellings_use_the_typed_registry_selector() {
    // The registry receives resolved word values, so the braced whitespace
    // form is `" 1 "`. All of these are Tcl numeric spellings for error (1),
    // and must reach the existing typed `Error` path rather than falling back
    // to a compiler-local selector parser or generic invocation.
    for selector in ["+1", "01", "0x1", "{ 1 }"] {
        let source = format!(
            "proc p {{}} {{ set rc [catch {{try {{error boom}} on {selector} {{msg}} {{set handled $msg}}}} result]; return $rc }}"
        );
        let fa = proc_via_backend(&source, "::p");
        let ops = opcodes(&fa);
        assert!(
            count(&ops, Op::BEGIN_CATCH4) >= 3,
            "{selector:?} must select inline try body and handler ranges: {ops:?}"
        );
        assert!(
            ops.contains(&Op::JUMP_FALSE4),
            "{selector:?} must use the typed error-code dispatch: {ops:?}"
        );
    }
}

#[test]
fn try_on_error_inline_direct() {
    // Drive emit_try_on_error_inline directly so the branch is exercised with a
    // minimal, deterministic input. Asserts the full sequence shape.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    let normal = ctx.fresh_label("normal");
    ctx.emit_try_on_error_inline(
        "try",
        &[
            ("expr {1/0}".into(), true),
            ("on".into(), false),
            ("error".into(), false),
            ("msg".into(), false),
            ("set z 1".into(), true),
        ],
        &normal,
    );
    ctx.place_label(&normal);
    let ops = ctx_ops(&ctx);
    assert_eq!(
        count(&ops, Op::BEGIN_CATCH4),
        2,
        "try-body + handler-body ranges: {ops:?}"
    );
    assert_eq!(
        count(&ops, Op::END_CATCH),
        4,
        "two normal exits + two handler entries"
    );
    assert!(ops.contains(&Op::JUMP_FALSE4), "code==1 dispatch");
    assert!(ops.contains(&Op::DICT_SET), "-during merge");
    assert_eq!(
        count(&ops, Op::RETURN_STK),
        2,
        "matched-cleanup + no-match re-raise"
    );
    // The handler variable `msg` and the two `#tempN` slots are interned.
    let lvt = ctx.lvt.entries();
    assert!(lvt.iter().any(|v| v == "msg"), "handler var slot: {lvt:?}");
    assert!(
        lvt.iter().any(|v| v.starts_with("#temp")),
        "temp opts/result slots: {lvt:?}"
    );
}

#[test]
fn try_on_error_handler_var_is_first_slot() {
    // emit_try_on_error_inline interns the handler variable before the temps —
    // it must land in the LVT ahead of the `#tempN` scratch slots.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    let normal = ctx.fresh_label("normal");
    ctx.emit_try_on_error_inline(
        "try",
        &[
            ("list 1 2".into(), true),
            ("on".into(), false),
            ("error".into(), false),
            ("errVar".into(), false),
            ("set z 1".into(), true),
        ],
        &normal,
    );
    let lvt = ctx.lvt.entries();
    let err_idx = lvt
        .iter()
        .position(|v| v == "errVar")
        .expect("errVar in LVT");
    let temp_idx = lvt
        .iter()
        .position(|v| v.starts_with("#temp"))
        .expect("temp in LVT");
    assert!(err_idx < temp_idx, "handler var precedes temps: {lvt:?}");
}

// ═══════════════════════════════════════════════════════════════════════════
// cmd_subst.rs — command substitutions in various positions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn cmd_subst_nested_bracket_in_arg() {
    // Nested `[a [b]]`: `string length` of a `string range` result. Both inline
    // forms fire (STR_LEN over STR_RANGE_IMM), and the inner subst is wrapped in
    // a startCommand in proc context.
    let fa = proc_via_backend(
        "proc p {x} { return [string length [string range $x 0 2]] }",
        "::p",
    );
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::STR_RANGE_IMM),
        "inner string range inlined: {ops:?}"
    );
    assert!(
        ops.contains(&Op::STR_LEN),
        "outer string length inlined: {ops:?}"
    );
    assert!(
        ops.contains(&Op::START_CMD),
        "inner cmd-subst is startCommand-wrapped"
    );
}

#[test]
fn cmd_subst_in_if_condition_is_a_pushed_word() {
    // A command substitution used in an `if` condition is not inlined to
    // STR_LEN: it becomes a pushed `<cond>` word the VM substitutes at run time.
    // No EXPR_STK follows the push — the substituted value *is* the operand, and
    // re-parsing it as an expression is exactly the double evaluation C avoids by
    // compiling the operand with `TclCompileTokens`. (control_flow + cmd_subst.)
    let fa = proc_via_backend(
        "proc p {x} { if {[string length $x] > 0} { return 1 }; return 0 }",
        "::p",
    );
    let ops = opcodes(&fa);
    assert!(
        !ops.contains(&Op::STR_LEN),
        "cmd-subst in cond is not inlined: {ops:?}"
    );
    assert!(
        !ops.contains(&Op::EXPR_STK),
        "the substituted word is not re-parsed as an expression: {ops:?}"
    );
    assert!(has_cond_jump(&ops), "the if still has a conditional jump");
}

#[test]
fn cmd_subst_info_exists_in_condition() {
    // `info exists` inside an `if` condition is likewise a substituted word, not
    // the inline existScalar form and not a re-parsed expression.
    let fa = proc_via_backend(
        "proc p {x} { if {[info exists x]} { return 1 }; return 0 }",
        "::p",
    );
    let ops = opcodes(&fa);
    assert!(
        !ops.contains(&Op::EXPR_STK),
        "info-exists cond is a pushed word, not a re-parsed expression: {ops:?}"
    );
    assert!(has_cond_jump(&ops), "the if still has a conditional jump");
}

#[test]
fn cmd_subst_in_expr_position_is_a_pushed_word() {
    // A command substitution embedded in an `expr` body (`[llength $l] + 1`) is
    // not separately inlined — it becomes a pushed operand word the VM
    // substitutes, while the outer `+ 1` is real compile-time arithmetic.
    // (cmd_subst + values.)
    let fa = proc_via_backend("proc p {l} { return [expr {[llength $l] + 1}] }", "::p");
    let ops = opcodes(&fa);
    assert!(
        !ops.contains(&Op::EXPR_STK),
        "the substituted operand is not re-parsed as an expression: {ops:?}"
    );
    assert!(
        ops.contains(&Op::ADD),
        "the outer +1 is a real ADD: {ops:?}"
    );
    assert!(
        !ops.contains(&Op::LIST_LENGTH),
        "llength is not inlined in subst position"
    );
}

#[test]
fn cmd_subst_specialised_opcode_in_assignment_rhs() {
    // As an assignment RHS, a specialised command substitution emits its
    // dedicated opcode: `set y [lindex $l 0]` → an immediate list-index. (The
    // specialised inline path of emit_inline_cmd_subst, reached via emit_value.)
    let fa = proc_via_backend("proc p {l} { set y [lindex $l 0]; return $y }", "::p");
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::LIST_INDEX_IMM),
        "lindex RHS → LIST_INDEX_IMM: {ops:?}"
    );
    assert!(
        ops.contains(&Op::STORE_SCALAR1),
        "the result is stored to y"
    );
}

#[test]
fn cmd_subst_in_list_construction() {
    // `[list [expr {…}] [string length …]]`: each list element is itself a
    // command substitution. The `list` inline path emits each arg then `LIST N`.
    let fa = proc_via_backend(
        "proc p {x} { return [list [expr {1 + 2}] [string length $x]] }",
        "::p",
    );
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::LIST), "list construction → LIST: {ops:?}");
    assert!(
        ops.contains(&Op::STR_LEN),
        "the string-length element inlines"
    );
}

#[test]
fn cmd_subst_lindex_multi_index() {
    // `lindex $m 0 1` (multi-index) → LINDEX_MULTI, distinct from the single
    // immediate-index LIST_INDEX_IMM form.
    let fa = proc_via_backend("proc p {m} { return [lindex $m 0 1] }", "::p");
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::LINDEX_MULTI),
        "multi-index lindex → LINDEX_MULTI: {ops:?}"
    );
}

#[test]
fn cmd_subst_lrange_end_index() {
    // `lrange $l 1 end`: the `end` index encodes as an immediate (`<= INDEX_END`)
    // so it stays on the LIST_RANGE_IMM fast path.
    let fa = proc_via_backend("proc q {l} { return [lrange $l 1 end] }", "::q");
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::LIST_RANGE_IMM),
        "lrange .. end → LIST_RANGE_IMM: {ops:?}"
    );
}

#[test]
fn cmd_subst_multi_command_falls_back_to_eval_stk() {
    // A `[...]` body with an internal `;` separator is two commands; it cannot
    // inline as a single call and falls back to runtime EVAL_STK over the braced
    // script. (has_command_separator + emit_inline_cmd_subst.)
    let fa = proc_via_backend("proc m {} { return [set a 1; set b 2] }", "::m");
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::EVAL_STK),
        "multi-command subst → EVAL_STK: {ops:?}"
    );
}

#[test]
fn cmd_subst_expand_form_uses_invoke_expanded() {
    // `[cmd {*}$a {*}$b]` in value position: the `{*}` expansion compiles to
    // expandStart / expandStkTop / invokeExpanded (leaving the result on the
    // stack), rather than a plain invoke.
    let fa = proc_via_backend("proc e {a b} { return [concat {*}$a {*}$b] }", "::e");
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::EXPAND_START),
        "{{*}} expansion → expandStart: {ops:?}"
    );
    assert!(ops.contains(&Op::EXPAND_STKTOP));
    assert!(
        ops.contains(&Op::INVOKE_EXPANDED),
        "→ invokeExpanded: {ops:?}"
    );
}

#[test]
fn try_list_expand_concat_direct_two_arg() {
    // `[list {*}$a {*}$b]` (exactly two `{*}$name` args) is the specialised
    // two-list-concat form: load a, load b, listConcat — driven directly.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &["a", "b"]);
    let matched = ctx.try_list_expand_concat("[list {*}$a {*}$b]");
    assert!(matched, "the exact two-arg form must match");
    let ops = ctx_ops(&ctx);
    assert_eq!(
        ops,
        vec![Op::LOAD_SCALAR1, Op::LOAD_SCALAR1, Op::LIST_CONCAT],
        "load a; load b; listConcat"
    );
}

#[test]
fn try_list_expand_concat_rejects_one_arg() {
    // A single `{*}$x` (or 3+) is not the two-list form and must not match
    // (it falls through to the generic / expanded path).
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &["a"]);
    assert!(
        !ctx.try_list_expand_concat("[list {*}$a]"),
        "one arg must not match"
    );
    assert!(
        !ctx.try_list_expand_concat("[list {*}$a {*}$b {*}$c]"),
        "three args must not match"
    );
    assert!(ctx.instructions.is_empty(), "a non-match emits nothing");
}

#[test]
fn list_break_continue_inline_jump_with_cleanup() {
    // `[list arg [break] arg]` with a break target in scope compiles the break
    // as an inline jump4 to the loop end, popping the already-pushed list
    // arguments first, then lays out the (dead) `list N`. Driven directly.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    ctx.break_target = Some("loop_break".into());
    let matched = ctx.try_inline_list_with_break_continue("[list 1 [break] 2]");
    assert!(matched, "the [list .. [break] ..] pattern must match");
    let ops = ctx_ops(&ctx);
    assert!(ops.contains(&Op::JUMP4), "break compiles to jump4: {ops:?}");
    assert!(
        ops.contains(&Op::LIST),
        "the trailing (dead) list N is still laid out"
    );
    assert!(
        ops.contains(&Op::POP),
        "earlier list args are popped before the jump"
    );
}

#[test]
fn list_continue_inline_jump() {
    // The `[continue]` variant jumps to the continue target.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    ctx.continue_target = Some("loop_cont".into());
    let matched = ctx.try_inline_list_with_break_continue("[list a [continue] b]");
    assert!(matched);
    let ops = ctx_ops(&ctx);
    assert!(
        ops.contains(&Op::JUMP4),
        "continue compiles to jump4: {ops:?}"
    );
    assert!(ops.contains(&Op::LIST));
}

#[test]
fn list_break_no_target_does_not_match() {
    // Without a break/continue target in scope, the `[break]`/`[continue]`
    // special-case still parses but the jump branch only fires when a target is
    // set; with no target the args are emitted as plain values. Assert it does
    // not emit a spurious jump.
    let reg = registry();
    let mut ctx = ctx_with_params(&reg, &[]);
    // No break_target / continue_target set.
    let matched = ctx.try_inline_list_with_break_continue("[list a [break] b]");
    assert!(matched, "the pattern still matches (it contains [break])");
    let ops = ctx_ops(&ctx);
    assert!(
        !ops.contains(&Op::JUMP4),
        "no target ⇒ no inline jump: {ops:?}"
    );
    assert!(ops.contains(&Op::LIST));
}

// ═══════════════════════════════════════════════════════════════════════════
// control_flow.rs — nested loops, break/continue at depth, return in branches
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn break_at_loop_depth_two_jumps_to_inner_end() {
    // A `break` inside the inner of two nested loops jumps to the *inner* loop
    // end (jump4); both loop conditions are still emitted.
    let fa = proc_via_backend(
        "proc nb {n} { set hits 0; for {set i 0} {$i < $n} {incr i} { for {set j 0} {$j < $n} {incr j} { if {$j == 2} { break }; incr hits } }; return $hits }",
        "::nb",
    );
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::JUMP4), "inner break → jump4: {ops:?}");
    assert!(
        count(&ops, Op::LT) >= 2,
        "both nested loop conditions emitted"
    );
    assert!(has_cond_jump(&ops));
}

#[test]
fn continue_at_loop_depth_two() {
    // A `continue` in the inner loop body lowers to a jump back to the inner
    // loop's continue/step target.
    let fa = proc_via_backend(
        "proc nc {n} { set s 0; for {set i 0} {$i < $n} {incr i} { for {set j 0} {$j < $n} {incr j} { if {$j % 2 == 0} { continue }; incr s } } ; return $s }",
        "::nc",
    );
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::JUMP4), "inner continue → jump: {ops:?}");
    assert!(ops.contains(&Op::MOD), "the mod op is emitted");
}

#[test]
fn return_in_both_if_branches() {
    // `return` in both arms of an if: both produce a proc return (folded to
    // done in tail position) — no fallthrough merge value is needed.
    let fa = proc_via_backend(
        "proc sign {x} { if {$x < 0} { return neg } else { return nonneg } }",
        "::sign",
    );
    let ops = opcodes(&fa);
    assert!(
        has_cond_jump(&ops),
        "the if branch needs a conditional jump: {ops:?}"
    );
    assert!(ops.contains(&Op::DONE) || ops.contains(&Op::RETURN_IMM));
    assert!(fa.literals.entries().iter().any(|l| l == "neg"));
    assert!(fa.literals.entries().iter().any(|l| l == "nonneg"));
}

#[test]
fn if_elseif_else_chain_three_way() {
    // A three-way if/elseif/else chain emits at least two condition tests and
    // lands each arm's value in the shared merge.
    let fa = proc_via_backend(
        "proc bucket {x} { if {$x < 0} { set r lo } elseif {$x == 0} { set r mid } else { set r hi }; return $r }",
        "::bucket",
    );
    let ops = opcodes(&fa);
    let conds = count(&ops, Op::JUMP_FALSE1)
        + count(&ops, Op::JUMP_FALSE4)
        + count(&ops, Op::JUMP_TRUE1)
        + count(&ops, Op::JUMP_TRUE4);
    assert!(conds >= 2, "elseif chain ⇒ ≥2 conditional jumps: {ops:?}");
    assert!(ops.contains(&Op::LT));
    assert!(ops.contains(&Op::EQ));
}

#[test]
fn switch_exact_return_arms_jump_table() {
    // A `switch -exact` whose arms all `return` compiles to a jump table for the
    // dispatch (exact string equality), with each arm body emitted.
    let fa = proc_via_backend(
        "proc dispatch {cmd} { switch -exact $cmd { add { return 1 } sub { return 2 } mul { return 3 } default { return 0 } } }",
        "::dispatch",
    );
    let ops = opcodes(&fa);
    assert!(
        ops.contains(&Op::JUMP_TABLE) || has_cond_jump(&ops),
        "exact switch dispatch → jump table or conditional chain: {ops:?}"
    );
    assert!(ops.contains(&Op::DONE));
}

#[test]
fn switch_glob_routes_through_generic_invoke() {
    // A `-glob` switch is not exact-string dispatch, so it must NOT emit a jump
    // table; it routes through a generic switch invoke. (control_flow dead-arm
    // / generic-switch path.)
    let fa = proc_via_backend(
        "proc ext {f} { switch -glob -- $f { *.c { return c } *.h { return h } default { return x } } }",
        "::ext",
    );
    let ops = opcodes(&fa);
    assert!(
        !ops.contains(&Op::JUMP_TABLE),
        "glob switch must not jump-table: {ops:?}"
    );
    assert!(
        ops.contains(&Op::INVOKE_STK1) || ops.contains(&Op::INVOKE_STK4),
        "glob switch → generic invoke: {ops:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// values.rs / statements.rs headroom — folding (tclsh-pinned)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn fold_nested_arithmetic_to_literal() {
    // The all-constant expression folds; the folded literal is the observable
    // runtime value and is pinned to C-Tcl.
    // tclsh: expr {(1+2)*(3+4)}  →  21   (8.6 + 9.0)
    let (asm, _) = module_via_backend("set r [expr {(1+2)*(3+4)}]");
    let fa = &asm.top_level;
    let ops = opcodes(fa);
    assert!(
        !ops.contains(&Op::ADD),
        "constant arithmetic folded away: {ops:?}"
    );
    assert!(!ops.contains(&Op::MULT));
    assert!(
        fa.literals.entries().iter().any(|l| l == "21"),
        "folded literal 21"
    );
}

#[test]
fn fold_exponent_to_literal() {
    // tclsh: expr {2 ** 8}  →  256   (8.6 + 9.0)
    let (asm, _) = module_via_backend("set r [expr {2 ** 8}]");
    let fa = &asm.top_level;
    assert!(!opcodes(fa).contains(&Op::EXPON), "constant ** folded");
    assert!(fa.literals.entries().iter().any(|l| l == "256"));
}

#[test]
fn fold_list_command_to_literal() {
    // `[list a b c]` with literal elements folds to the canonical list string.
    // tclsh: list a b c  →  a b c   (8.6 + 9.0)
    let (asm, _) = module_via_backend("set r [list a b c]");
    let fa = &asm.top_level;
    assert!(
        fa.literals.entries().iter().any(|l| l == "a b c"),
        "folded list literal: {:?}",
        fa.literals.entries()
    );
}

#[test]
fn fold_bitwise_chain_to_literal() {
    // tclsh: expr {7 & 3 | 8}  →  11   (8.6 + 9.0)
    let (asm, _) = module_via_backend("set r [expr {7 & 3 | 8}]");
    let fa = &asm.top_level;
    let ops = opcodes(fa);
    assert!(
        !ops.contains(&Op::BITAND) && !ops.contains(&Op::BITOR),
        "bitwise folded: {ops:?}"
    );
    assert!(fa.literals.entries().iter().any(|l| l == "11"));
}

#[test]
fn fold_ternary_to_literal() {
    // A constant ternary folds to the selected branch value.
    // tclsh: expr {5 > 3 ? 100 : 200}  →  100   (8.6 + 9.0)
    let (asm, _) = module_via_backend("set r [expr {5 > 3 ? 100 : 200}]");
    let fa = &asm.top_level;
    assert!(
        fa.literals.entries().iter().any(|l| l == "100"),
        "folded ternary → 100"
    );
    assert!(
        !opcodes(fa).contains(&Op::GT),
        "the constant comparison folded"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Hand-built CFG through the Backend trait (statements.rs / values.rs surface)
// ═══════════════════════════════════════════════════════════════════════════

/// Build a top-level `CfgFunction` whose entry block holds `statements` and
/// returns — the by-hand CFG idiom shared with the existing suites.
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

#[test]
fn backend_lower_function_handbuilt_assign_const() {
    // A hand-built CFG with a single AssignConst, lowered through the trait,
    // pushes the constant and stores it (stack-based at script scope).
    let reg = registry();
    let cfg = toplevel_cfg(vec![Statement::AssignConst {
        span: tcl_lexer::Span::new(0, 0),
        name: "x".into(),
        name_braced: false,
        value: "42".into(),
        value_span: None,
    }]);
    let mut backend = BytecodeBackend;
    let fa = backend.lower_function(&cfg, &[], false, &[], &reg);
    let ops = opcodes(&fa);
    assert!(ops.contains(&Op::PUSH1));
    assert!(ops.contains(&Op::STORE_STK));
    assert!(fa.literals.entries().iter().any(|l| l == "42"));
}

#[test]
fn backend_lower_module_handbuilt_empty_module() {
    // An empty hand-built CfgModule through the trait yields a `::top` with no
    // procedures — the degenerate module artifact.
    let reg = registry();
    let cfg_mod = CfgModule {
        top_level: toplevel_cfg(vec![]),
        procedures: std::collections::HashMap::new(),
    };
    let ir = lower_to_ir("", &reg);
    let mut backend = BytecodeBackend;
    let asm = backend.lower_module(&cfg_mod, &ir, &reg);
    assert_eq!(asm.top_level.name, "::top");
    assert!(asm.procedures.is_empty());
    // An empty hand-built top-level block ends with `returnImm` (the explicit
    // `Return` terminator with no value); the from-source empty path peephole-
    // folds to `done`. Accept either terminal form.
    let last = asm.top_level.instructions.last().unwrap().op;
    assert!(
        matches!(last, Op::DONE | Op::RETURN_IMM),
        "terminal op was {last:?}"
    );
}
