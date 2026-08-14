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

//! Op-coverage regression gate for `Vm::tick`'s bytecode dispatch (issue
//! #1411).
//!
//! `Vm::tick`'s `match instr.op { … }` (`tcl-vm/src/exec.rs`) used to end in
//! a wildcard arm: any `Op` variant without a dedicated case fell through to
//! a run-time error string (`"opcode {mnemonic} not implemented in
//! tcl-vm"`) instead of a build failure. #1411 was exactly that gap —
//! `Op::PUSH_RETURN_CODE` had five codegen emission sites
//! (`tcl-compiler/src/codegen/control_flow.rs`) and zero VM dispatch arms —
//! and it was invisible until a compiled `catch`/`try`/`return -code` body
//! hit it at run time.
//!
//! The mechanical fix lives in `exec.rs`, not in this file: the wildcard arm
//! is gone, so `match instr.op` now lists every `Op` variant explicitly.
//! Adding a new `Op` variant (or ever again removing an arm) without a real
//! dispatch case fails to **compile** `tcl-vm` — `rustc`'s own
//! non-exhaustive-match check is the gate, which is strictly stronger than
//! any test this crate could write (a test can be skipped or fail to run; a
//! compile error cannot). This file exists to document the invariant and to
//! exercise, end-to-end, the two `Op` variants (`LAND`/`LOR`) that needed a
//! first dispatch arm to make the match exhaustive.
//!
//! `Op::LAND`/`Op::LOR` are not reachable from current codegen — `&&`/`||`
//! compile to short-circuit jump sequences
//! (`tcl_compiler::codegen::expressions::emit_expr_binary`), never through
//! `Op::from_binop` — but they are enumerable `Op` variants reachable
//! through that same `from_binop` mapping (`BinOp::And`/`BinOp::Or`), so an
//! enum variant with no dispatch arm is precisely the class of bug this
//! gate closes, whether or not codegen happens to emit it today. The VM now
//! gives them real (eager, non-short-circuit) boolean semantics rather than
//! the removed catch-all error.

use std::collections::HashMap;

use tcl_bytecode::{
    FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op, Operand, bytecode_imm, layout,
};
use tcl_vm::{Code, Completion, Value, Vm};

/// A hand-built instruction stream plus its literal pool, mirroring the
/// `Asm` helper in `opcode_c_parity.rs` (kept file-local, per that file's own
/// precedent, rather than shared across test binaries).
#[derive(Default)]
struct Asm {
    lits: LiteralTable,
    instrs: Vec<Instruction>,
}

impl Asm {
    fn new() -> Self {
        Self::default()
    }

    /// Push a literal string, verbatim (as codegen marks braced/constant
    /// words) so the VM pushes it exactly as interned instead of
    /// re-substituting it.
    fn push(&mut self, text: &str) -> &mut Self {
        let idx = self.lits.intern(text);
        let mut instr = Instruction::new(Op::PUSH1, vec![Operand::Imm(bytecode_imm(idx))]);
        instr.push_verbatim = true;
        self.instrs.push(instr);
        self
    }

    fn op(&mut self, op: Op) -> &mut Self {
        self.instrs.push(Instruction::new(op, vec![]));
        self
    }

    fn build(mut self) -> FunctionAsm {
        let labels = HashMap::new();
        let resolved = layout::resolve_layout(&mut self.instrs, &labels);
        FunctionAsm {
            name: "test".into(),
            literals: self.lits,
            lvt: LocalVarTable::new(&[]),
            instructions: self.instrs,
            labels: resolved,
            loop_targets: HashMap::new(),
            body_base_line: 0,
            error_regions: Vec::new(),
        }
    }
}

fn run(asm: Asm) -> Completion<Value> {
    Vm::new().run_function(&asm.build())
}

/// Run `land ( a b )` (`a`/`b` are Tcl boolean literals) and return the
/// completion.
fn land(a: &str, b: &str) -> Completion<Value> {
    let mut asm = Asm::new();
    asm.push(a).push(b).op(Op::LAND);
    run(asm)
}

/// Run `lor ( a b )` and return the completion.
fn lor(a: &str, b: &str) -> Completion<Value> {
    let mut asm = Asm::new();
    asm.push(a).push(b).op(Op::LOR);
    run(asm)
}

/// `Op::LAND` computes eager (non-short-circuit) logical AND over its two
/// popped operands and pushes a Tcl boolean — the full truth table, pinned
/// against ordinary `&&`.
#[test]
fn land_truth_table() {
    for (a, b, want) in [
        ("1", "1", "1"),
        ("1", "0", "0"),
        ("0", "1", "0"),
        ("0", "0", "0"),
    ] {
        let c = land(a, b);
        assert_eq!(
            c.code,
            Code::Ok,
            "land({a}, {b}) errored: {}",
            c.result.to_str()
        );
        assert_eq!(c.result.to_str().as_ref(), want, "land({a}, {b})");
    }
}

/// `Op::LOR` computes eager (non-short-circuit) logical OR — the full truth
/// table, pinned against ordinary `||`.
#[test]
fn lor_truth_table() {
    for (a, b, want) in [
        ("1", "1", "1"),
        ("1", "0", "1"),
        ("0", "1", "1"),
        ("0", "0", "0"),
    ] {
        let c = lor(a, b);
        assert_eq!(
            c.code,
            Code::Ok,
            "lor({a}, {b}) errored: {}",
            c.result.to_str()
        );
        assert_eq!(c.result.to_str().as_ref(), want, "lor({a}, {b})");
    }
}

/// Neither opcode ever reaches the (now-removed) "not implemented" catch-all
/// — the specific regression #1411 warned about. Pinned as an explicit
/// string check, independent of the truth-table assertions above, so a
/// future reintroduction of a wildcard dispatch arm that happens to also
/// break the boolean logic still fails here with the diagnostic string that
/// names the exact failure mode.
#[test]
fn land_lor_never_hit_the_unimplemented_catch_all() {
    for c in [land("1", "1"), lor("0", "0")] {
        assert_eq!(c.code, Code::Ok);
        assert!(
            !c.result.to_str().contains("not implemented in tcl-vm"),
            "dispatch fell through to the catch-all: {}",
            c.result.to_str()
        );
    }
}
