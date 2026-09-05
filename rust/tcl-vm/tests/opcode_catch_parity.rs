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

//! C Tcl 9.0.4 parity for the compiled-`catch` exception machinery:
//! `beginCatch4` ranges (via the out-of-band `catch_target`), the
//! `pushResult`/`pushReturnCode`/`pushReturnOpts` epilogue reads,
//! `returnCodeBranch`, and the C-ordered `returnStk`.
//!
//! The inline `catch` shape only fires for a value-position `catch` in a proc
//! body (`InlineCodegenHookId::Catch`), so the compiled cases run the AOT
//! `module.procedures` body directly through `run_function` — the same
//! bytecode the explorer disassembles. Hand-built `FunctionAsm` cases pin the
//! opcode-level contracts.

use std::collections::HashMap;

use tcl_bytecode::layout::resolve_layout;
use tcl_bytecode::{FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op, Operand};
use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::lowering::lower_to_ir_for_bytecode;
use tcl_registry::CommandRegistry;
use tcl_vm::{Code, Vm};

/// A `tcl-compiler`-backed compile service (the CLI embedders' shape), for
/// tests whose procs / dynamic bodies compile at runtime.
struct Svc(CommandRegistry);

impl tcl_vm::CompileService for Svc {
    type Module = tcl_bytecode::ModuleAsm;
    fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, tcl_vm::CompileError> {
        let ir = lower_to_ir_for_bytecode(src, &self.0);
        let cfg = build_cfg_codegen(&ir, false);
        Ok(codegen_module(&cfg, &ir, &self.0))
    }
}

/// Compile `src` and return the AOT body of proc `qname` (e.g. `::p`).
fn compiled_proc_body(src: &str, qname: &str) -> FunctionAsm {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir_for_bytecode(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, &registry);
    module
        .procedures
        .get(qname)
        .unwrap_or_else(|| panic!("{qname} not compiled — got {:?}", module.procedures.keys()))
        .clone()
}

/// Run a proc body compiled by [`compiled_proc_body`] on a fresh VM.
fn run_body(src: &str, qname: &str) -> (Code, String) {
    let asm = compiled_proc_body(src, qname);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    (c.code, c.result.to_str().to_string())
}

/// Assemble a hand-built function: resolve label offsets, done.
fn assemble(
    mut instrs: Vec<Instruction>,
    labels: &HashMap<String, usize>,
    literals: LiteralTable,
) -> FunctionAsm {
    let label_offsets = resolve_layout(&mut instrs, labels);
    FunctionAsm {
        name: "test".into(),
        literals,
        lvt: LocalVarTable::new(&[]),
        instructions: instrs,
        labels: label_offsets,
        loop_targets: HashMap::new(),
        body_base_line: 0,
        proc_body_src: None,
        error_regions: Vec::new(),
    }
}

fn push_lit(lits: &mut LiteralTable, s: &str) -> Instruction {
    let idx = i32::try_from(lits.intern(s)).unwrap();
    let mut i = Instruction::new(Op::PUSH1, vec![Operand::Imm(idx)]);
    i.push_verbatim = true;
    i
}

// -- compiled inline catch: the error path used to be unexecutable
//    (`pushReturnCode` had no arm; `beginCatch4` was inert) --

/// 3-arg `catch` over an erroring body binds code 1, the message, and an
/// options dict carrying `-code`/`-errorcode`/`-errorinfo` (catch-3.1-ish).
#[test]
fn inline_catch_error_path_binds_result_and_opts() {
    let (code, result) = run_body(
        "proc p {} {\n  set r [catch {nosuchcmd} msg opts]\n  list $r $msg [dict get $opts -code] [dict get $opts -errorcode]\n}",
        "::p",
    );
    assert_eq!(code, Code::Ok, "catch must absorb the error: {result}");
    assert_eq!(
        result,
        "1 {invalid command name \"nosuchcmd\"} 1 {TCL LOOKUP COMMAND nosuchcmd}"
    );
}

/// The ok path still binds code 0 and the body's result.
#[test]
fn inline_catch_ok_path() {
    let (code, result) = run_body(
        "proc p {} {\n  set r [catch {list a b} msg]\n  list $r $msg\n}",
        "::p",
    );
    assert_eq!(code, Code::Ok, "{result}");
    assert_eq!(result, "0 {a b}");
}

/// `catch {return x}` reports `TCL_RETURN` (2) — the range absorbs the carried
/// return before the proc boundary sees it (C catch-4.x family).
#[test]
fn inline_catch_absorbs_return() {
    let (code, result) = run_body(
        "proc p {} {\n  set r [catch {return x} msg]\n  list $r $msg\n}",
        "::p",
    );
    assert_eq!(code, Code::Ok, "{result}");
    assert_eq!(result, "2 x");
}

/// A `break`/`continue` returned by the body is caught as 3 / 4.
#[test]
fn inline_catch_absorbs_break_and_continue() {
    let (code, result) = run_body(
        "proc p {} {\n  set b [catch {break} m1]\n  set c [catch {continue} m2]\n  list $b $c\n}",
        "::p",
    );
    assert_eq!(code, Code::Ok, "{result}");
    assert_eq!(result, "3 4");
}

/// An erroring *proc call* inside the body is absorbed on the unwind path
/// (the child activation delivers its error into the caller's live range).
#[test]
fn inline_catch_absorbs_child_proc_error() {
    let (code, result) = run_body(
        "proc boom {} {error kaboom}\nproc p {} {\n  set r [catch {boom} msg]\n  list $r $msg\n}",
        "::p",
    );
    // `boom` must exist when `p`'s body runs: define it on the same VM first.
    // run_body compiles both procs but only runs ::p — so instead assemble the
    // pair through the module top level below.
    let _ = (code, result);
    let registry = CommandRegistry::build_default();
    let src =
        "proc boom {} {error kaboom}\nproc p {} {\n  set r [catch {boom} msg]\n  list $r $msg\n}";
    let ir = lower_to_ir_for_bytecode(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, &registry);
    let mut vm = Vm::new();
    // Defining the procs needs a compile service for their (dynamic) bodies —
    // route through the compiler exactly like the CLI embedders.
    vm.set_compiler(Box::new(Svc(CommandRegistry::build_default())));
    assert!(vm.run_module(&module).code.is_ok());
    // Now run the AOT inline-catch body with `boom` registered.
    let body = module.procedures.get("::p").expect("::p");
    let c = vm.run_function(body);
    assert_eq!(c.code, Code::Ok, "{}", c.result.to_str());
    assert_eq!(&*c.result.to_str(), "1 kaboom");
}

/// Nested hand-built ranges: the inner range fires and is popped by its own
/// `endCatch`; an error raised *after* it (still inside the outer range) is
/// absorbed by the outer handler — and an error raised inside the outer
/// *handler* is NOT re-absorbed (the armed range no longer catches, per C's
/// pc-containment).
#[test]
fn nested_ranges_fire_inner_then_outer_not_handler() {
    let mut lits = LiteralTable::new();
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut instrs: Vec<Instruction> = Vec::new();
    let begin = |instrs: &mut Vec<Instruction>, range: i32, target: &str| {
        let mut i = Instruction::new(Op::BEGIN_CATCH4, vec![Operand::Imm(range)]);
        i.catch_target = Some(target.to_string());
        instrs.push(i);
    };
    // outer range opens
    begin(&mut instrs, 0, "outer_handler");
    // inner range opens, raises, handles, closes
    begin(&mut instrs, 1, "inner_handler");
    instrs.push(push_lit(&mut lits, "inner-boom"));
    instrs.push(push_lit(&mut lits, ""));
    instrs.push(Instruction::new(
        Op::RETURN_IMM,
        vec![Operand::Imm(1), Operand::Imm(0)],
    ));
    labels.insert("inner_handler".into(), instrs.len());
    instrs.push(Instruction::new(Op::PUSH_RESULT, vec![]));
    instrs.push(Instruction::new(Op::POP, vec![]));
    instrs.push(Instruction::new(Op::END_CATCH, vec![]));
    // back in the outer range: raise again → outer handler
    instrs.push(push_lit(&mut lits, "outer-boom"));
    instrs.push(push_lit(&mut lits, ""));
    instrs.push(Instruction::new(
        Op::RETURN_IMM,
        vec![Operand::Imm(1), Operand::Imm(0)],
    ));
    labels.insert("outer_handler".into(), instrs.len());
    instrs.push(Instruction::new(Op::PUSH_RESULT, vec![]));
    instrs.push(Instruction::new(Op::POP, vec![]));
    // an error inside the outer HANDLER must escape (range is armed)
    instrs.push(push_lit(&mut lits, "handler-boom"));
    instrs.push(push_lit(&mut lits, ""));
    instrs.push(Instruction::new(
        Op::RETURN_IMM,
        vec![Operand::Imm(1), Operand::Imm(0)],
    ));
    instrs.push(Instruction::new(Op::END_CATCH, vec![]));
    instrs.push(Instruction::new(Op::DONE, vec![]));
    let asm = assemble(instrs, &labels, lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Error, "handler error must escape");
    assert_eq!(&*c.result.to_str(), "handler-boom");
}

/// `$::errorInfo` is published when a compiled range absorbs an error, just
/// as `finish_catch` publishes for the runtime `catch` (set-2.4 analogue).
/// The catch sits in value position so the inline shape fires; the trailing
/// read runs in the same frame.
#[test]
fn inline_catch_publishes_error_info() {
    let registry = CommandRegistry::build_default();
    let src = "proc p {} {\n  set r [catch {nosuchcmd} msg]\n  string match {invalid command name*} $::errorInfo\n}";
    let ir = lower_to_ir_for_bytecode(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let module = codegen_module(&cfg, &ir, &registry);
    let body = module.procedures.get("::p").expect("::p");
    let mut vm = Vm::new();
    let c = vm.run_function(body);
    assert_eq!(c.code, Code::Ok, "{}", c.result.to_str());
    assert_eq!(&*c.result.to_str(), "1");
}

// -- decorative ranges stay inert --

/// A `BEGIN_CATCH4` with no `catch_target` (the C-faithful reference shape
/// for constructs the VM protects via its activation stack) must not absorb.
#[test]
fn decorative_begin_catch_stays_inert() {
    let mut lits = LiteralTable::new();
    let instrs = vec![
        Instruction::new(Op::BEGIN_CATCH4, vec![Operand::Imm(0)]),
        push_lit(&mut lits, "boom"),
        push_lit(&mut lits, ""),
        Instruction::new(Op::RETURN_IMM, vec![Operand::Imm(1), Operand::Imm(0)]),
        Instruction::new(Op::END_CATCH, vec![]),
        Instruction::new(Op::DONE, vec![]),
    ];
    let asm = assemble(instrs, &HashMap::new(), lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "boom");
}

// -- returnCodeBranch --

/// Build `push <code>; returnCodeBranch; 5×jump1 stubs; landing pushes` and
/// return the landing literal — pins the `2*code − 1` byte arithmetic.
fn run_return_code_branch(code_lit: &str) -> (Code, String) {
    let mut lits = LiteralTable::new();
    let mut instrs = vec![
        push_lit(&mut lits, code_lit),
        Instruction::new(Op::RETURN_CODE_BRANCH, vec![]),
    ];
    let mut labels: HashMap<String, usize> = HashMap::new();
    // Five 2-byte jump1 stubs, one per code: error, return, break, continue,
    // other — exactly the table C's `try` compiler lays out.
    for name in ["error", "return", "break", "continue", "other"] {
        instrs.push(Instruction::new(
            Op::JUMP1,
            vec![Operand::Label(format!("land_{name}"))],
        ));
    }
    for name in ["error", "return", "break", "continue", "other"] {
        labels.insert(format!("land_{name}"), instrs.len());
        instrs.push(push_lit(&mut lits, &format!("landed_{name}")));
        instrs.push(Instruction::new(Op::DONE, vec![]));
    }
    let asm = assemble(instrs, &labels, lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    (c.code, c.result.to_str().to_string())
}

#[test]
fn return_code_branch_dispatches_each_code() {
    for (code_lit, want) in [
        ("1", "landed_error"),
        ("2", "landed_return"),
        ("3", "landed_break"),
        ("4", "landed_continue"),
        ("5", "landed_other"),
        ("7", "landed_other"), // out-of-range clamps to the fifth slot
    ] {
        let (code, result) = run_return_code_branch(code_lit);
        assert_eq!(code, Code::Ok, "code {code_lit}: {result}");
        assert_eq!(result, want, "code {code_lit}");
    }
}

/// C panics on `TCL_OK` / non-integer at the top of `returnCodeBranch`; the VM
/// reports a catchable error instead of aborting.
#[test]
fn return_code_branch_rejects_ok_and_junk() {
    let (code, result) = run_return_code_branch("0");
    assert_eq!(code, Code::Error);
    assert_eq!(result, "returnCodeBranch: TOS is TCL_OK");
    let (code, result) = run_return_code_branch("notanint");
    assert_eq!(code, Code::Error);
    assert_eq!(result, "returnCodeBranch: TOS not a return code");
}

// -- returnStk: C stack order (options under result) + options application --

/// `returnStk` with `-code error` in the options raises that error — the
/// options dict drives the completion (C `Tcl_SetReturnOptions`).
#[test]
fn return_stk_applies_code_from_options() {
    let mut lits = LiteralTable::new();
    let instrs = vec![
        push_lit(&mut lits, "-code 1 -level 0"),
        push_lit(&mut lits, "boom"),
        Instruction::new(Op::RETURN_STK, vec![]),
        Instruction::new(Op::DONE, vec![]),
    ];
    let asm = assemble(instrs, &HashMap::new(), lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "boom");
}

/// A `-level 0 -code ok` outcome *continues execution* with the result on the
/// stack — C only routes non-OK codes to `processExceptionReturn`.
#[test]
fn return_stk_ok_outcome_continues() {
    let mut lits = LiteralTable::new();
    let instrs = vec![
        push_lit(&mut lits, "-level 0 -code 0"),
        push_lit(&mut lits, "kept"),
        Instruction::new(Op::RETURN_STK, vec![]),
        // Execution must reach here, with "kept" still on the stack.
        Instruction::new(Op::STR_LEN, vec![]),
        Instruction::new(Op::DONE, vec![]),
    ];
    let asm = assemble(instrs, &HashMap::new(), lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "4", "strlen of the kept result");
}

/// A malformed options dict overrides the result with its own error (C:
/// "an error in the option dictionary overrides the result").
#[test]
fn return_stk_malformed_options_error() {
    let mut lits = LiteralTable::new();
    let instrs = vec![
        push_lit(&mut lits, "odd list here!"),
        push_lit(&mut lits, "ignored"),
        Instruction::new(Op::RETURN_STK, vec![]),
        Instruction::new(Op::DONE, vec![]),
    ];
    let asm = assemble(instrs, &HashMap::new(), lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Error);
    assert_eq!(
        &*c.result.to_str(),
        "expected dict but got \"odd list here!\""
    );
}

/// Plain `returnStk` (empty options) is a level-1 return, absorbed by a proc
/// boundary exactly like the `return` command.
#[test]
fn return_stk_plain_return() {
    let mut lits = LiteralTable::new();
    let instrs = vec![
        push_lit(&mut lits, ""),
        push_lit(&mut lits, "value"),
        Instruction::new(Op::RETURN_STK, vec![]),
        Instruction::new(Op::DONE, vec![]),
    ];
    let asm = assemble(instrs, &HashMap::new(), lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Return, "level 1 raises TCL_RETURN");
    assert_eq!(&*c.result.to_str(), "value");
}

// -- epilogue reads on the fall-through (no catch fired) path --

/// With no absorbed completion, `pushReturnCode` reports 0 and
/// `pushReturnOpts` the ok options dict (C's untouched interp state).
#[test]
fn epilogue_reads_without_catch() {
    let instrs = vec![
        Instruction::new(Op::PUSH_RETURN_CODE, vec![]),
        Instruction::new(Op::PUSH_RETURN_OPTS, vec![]),
        Instruction::new(Op::LIST, vec![Operand::Imm(2)]),
        Instruction::new(Op::DONE, vec![]),
    ];
    let asm = assemble(instrs, &HashMap::new(), LiteralTable::new());
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "0 {-code 0 -level 0}");
}

// -- the epilogue reads must not leak between ranges (PR #1419 review) --

/// A *successful* 3-arg `catch` that follows an erroring one reports its own OK
/// completion, not the earlier error's. Both paths of the compiled epilogue
/// share `pushReturnOpts`, so frame-wide caught state would hand the second
/// catch the first's `-code 1` / `-errorinfo`.
#[test]
fn sequential_catch_does_not_inherit_earlier_error_options() {
    let (code, result) = run_body(
        "proc p {} {\n  set r1 [catch {nosuchcmd} m1 o1]\n  set r2 [catch {list ok} m2 o2]\n  list $r1 $r2 [dict get $o1 -code] [dict get $o2 -code]\n}",
        "::p",
    );
    assert_eq!(code, Code::Ok, "{result}");
    assert_eq!(
        result, "1 0 1 0",
        "the second catch must report -code 0, not the first catch's -code 1"
    );
}

/// The same leak, one level in: a range whose *inner* range fired must still
/// report its own OK completion on the fall-through path. Hand-built because
/// the inline codegen does not nest a value-position `catch` inside a catch
/// body — the inner range here fires, is closed by its own `endCatch`, and the
/// outer epilogue then runs with no absorption of its own.
#[test]
fn outer_range_does_not_inherit_inner_ranges_absorption() {
    let mut lits = LiteralTable::new();
    let mut labels: HashMap<String, usize> = HashMap::new();
    let mut instrs: Vec<Instruction> = Vec::new();
    let begin = |instrs: &mut Vec<Instruction>, range: i32, target: &str| {
        let mut i = Instruction::new(Op::BEGIN_CATCH4, vec![Operand::Imm(range)]);
        i.catch_target = Some(target.to_string());
        instrs.push(i);
    };
    // Outer range opens; an inner range raises, handles and closes.
    begin(&mut instrs, 0, "outer_handler");
    begin(&mut instrs, 1, "inner_handler");
    instrs.push(push_lit(&mut lits, "inner-boom"));
    instrs.push(push_lit(&mut lits, ""));
    instrs.push(Instruction::new(
        Op::RETURN_IMM,
        vec![Operand::Imm(1), Operand::Imm(0)],
    ));
    labels.insert("inner_handler".into(), instrs.len());
    instrs.push(Instruction::new(Op::PUSH_RESULT, vec![]));
    instrs.push(Instruction::new(Op::POP, vec![]));
    instrs.push(Instruction::new(Op::END_CATCH, vec![]));
    // Outer body then succeeds: its epilogue must report code 0, not 1.
    instrs.push(Instruction::new(Op::PUSH_RETURN_CODE, vec![]));
    instrs.push(Instruction::new(Op::END_CATCH, vec![]));
    instrs.push(Instruction::new(Op::DONE, vec![]));
    labels.insert("outer_handler".into(), instrs.len());
    instrs.push(push_lit(&mut lits, "outer-handler-ran"));
    instrs.push(Instruction::new(Op::DONE, vec![]));
    let asm = assemble(instrs, &labels, lits);
    let mut vm = Vm::new();
    let c = vm.run_function(&asm);
    assert_eq!(c.code, Code::Ok, "{}", c.result.to_str());
    assert_eq!(
        &*c.result.to_str(),
        "0",
        "the outer range never fired, so its epilogue reports an OK completion"
    );
}
