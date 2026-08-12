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

//! Per-opcode parity tests for the variable/array/dict/introspection,
//! coroutine and `TclOO` instructions the compiler does not (yet) emit, plus
//! the iRules dialect operators it does.
//!
//! Each test hand-assembles a [`FunctionAsm`] — the artifact `tcl-compiler`'s
//! `codegen_module` produces — and runs it through [`Vm::run_function`], so the
//! opcode under test is exercised directly rather than through whatever
//! sequence codegen happens to choose. Expectations are pinned to
//! `tclExecute.c`'s `TEBCresume` cases (stack order, result value, error
//! message) for Tcl 9.0.4; the iRules operators, which have no C counterpart,
//! are pinned against the core commands they are defined in terms of.
//!
//! The opcodes that only mean something *inside* a coroutine or a method call
//! (`yield`, `yieldToInvoke`, `tclooSelf`, `tclooNext`, …) run through
//! [`vm_with_seeded_proc`], which installs a hand-built stream as a real proc's
//! compiled body so a `coroutine`/method call reaches it.

use std::collections::HashMap;

use tcl_bytecode::{
    FunctionAsm, Instruction, LiteralTable, LocalVarTable, ModuleAsm, Op, Operand, bytecode_imm,
    layout,
};
use tcl_vm::{Code, Completion, Value, Vm};

/// A hand-built instruction stream plus its literal pool and LVT.
#[derive(Default)]
struct Asm {
    lits: LiteralTable,
    lvt: LocalVarTable,
    instrs: Vec<Instruction>,
}

impl Asm {
    fn new() -> Self {
        Self::default()
    }

    /// Push a literal string. Marked verbatim (as codegen marks braced words)
    /// so the VM pushes it exactly as interned instead of re-substituting it.
    fn push(&mut self, text: &str) -> &mut Self {
        let idx = self.lits.intern(text);
        let mut instr = Instruction::new(Op::PUSH1, vec![Operand::Imm(bytecode_imm(idx))]);
        instr.push_verbatim = true;
        self.instrs.push(instr);
        self
    }

    /// Intern `name` in the LVT and return its slot, for an LVT-operand opcode.
    fn slot(&mut self, name: &str) -> i32 {
        bytecode_imm(self.lvt.intern(name))
    }

    /// Emit `op` with the given operands.
    fn op(&mut self, op: Op, operands: &[i32]) -> &mut Self {
        self.instrs.push(Instruction::new(
            op,
            operands.iter().copied().map(Operand::Imm).collect(),
        ));
        self
    }

    /// Finish the function, assigning byte offsets the way `codegen_module`
    /// does (`resolve_layout`) so the executor's offset→index map is valid.
    fn build(mut self) -> FunctionAsm {
        let labels = HashMap::new();
        let resolved = layout::resolve_layout(&mut self.instrs, &labels);
        FunctionAsm {
            name: "test".into(),
            literals: self.lits,
            lvt: self.lvt,
            instructions: self.instrs,
            labels: resolved,
            loop_targets: HashMap::new(),
            body_base_line: 0,
            error_regions: Vec::new(),
        }
    }
}

/// Run a hand-built function on `vm`.
fn run(vm: &mut Vm, asm: Asm) -> Completion<Value> {
    vm.run_function(&asm.build())
}

/// Run a hand-built function on a fresh `Vm`, returning `(vm, completion)`.
fn run_fresh(asm: Asm) -> (Vm, Completion<Value>) {
    let mut vm = Vm::new();
    let c = run(&mut vm, asm);
    (vm, c)
}

/// The result string of a completion that must have succeeded.
fn ok_str(c: &Completion<Value>) -> String {
    assert_eq!(
        c.code,
        Code::Ok,
        "expected success, got {}",
        c.result.to_str()
    );
    c.result.to_str().to_string()
}

/// The message of a completion that must have failed.
fn err_str(c: &Completion<Value>) -> String {
    assert_eq!(
        c.code,
        Code::Error,
        "expected an error, got {}",
        c.result.to_str()
    );
    c.result.to_str().to_string()
}

// -- loadScalarStk / storeScalarStk ------------------------------------------

/// `storeScalarStk` writes the named scalar and leaves the value on the stack;
/// `loadScalarStk` reads it back. Both share C's `INST_STORE_STK` /
/// `INST_LOAD_STK` cases, so they behave exactly like the general forms.
#[test]
fn load_store_scalar_stk_round_trip() {
    let mut a = Asm::new();
    a.push("x").push("42").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]).push("x").op(Op::LOAD_SCALAR_STK, &[]);
    let (vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "42");
    assert_eq!(
        vm.get_var("x").map(|v| v.to_str().to_string()),
        Some("42".into())
    );
}

/// Reading an unset scalar through `loadScalarStk` raises C's read miss.
#[test]
fn load_scalar_stk_missing_variable_errors() {
    let mut a = Asm::new();
    a.push("nope").op(Op::LOAD_SCALAR_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't read \"nope\": no such variable");
}

// -- loadArray4 / storeArray4 ------------------------------------------------

/// `storeArray4`/`loadArray4` are the 4-byte-slot forms of the `*Array1` pair:
/// the array is the LVT slot, the element key is on the stack.
#[test]
fn load_store_array4_round_trip() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v1").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]).push("k").op(Op::LOAD_ARRAY4, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "v1");
}

/// A missing element reports the array-specific read miss.
#[test]
fn load_array4_missing_element_errors() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[])
        .push("other")
        .op(Op::LOAD_ARRAY4, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't read \"arr(other)\": no such element in array"
    );
}

// -- the incr family --------------------------------------------------------

/// The stack-form increments: `incrScalarStk` takes the amount from the stack,
/// `incrScalarStkImm` from its 1-byte operand. Both push the new value.
#[test]
fn incr_scalar_stk_forms() {
    let mut a = Asm::new();
    a.push("n").push("7").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("n").push("5").op(Op::INCR_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("n").op(Op::INCR_SCALAR_STK_IMM, &[-2]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "10");
}

/// A non-integer increment reports the canonical coercion error.
#[test]
fn incr_scalar_stk_bad_amount_errors() {
    let mut a = Asm::new();
    a.push("n").push("abc").op(Op::INCR_SCALAR_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "expected integer but got \"abc\"");
}

/// The array increments name the element `base(key)`: `incrArray1`/
/// `incrArray1Imm` take the array from their LVT slot, `incrArrayStk` from the
/// stack (`name key amount`, amount on top).
#[test]
fn incr_array_forms() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("1").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]);
    // incrArray1: key on the stack, then the amount.
    a.push("k").push("4").op(Op::INCR_ARRAY1, &[slot]);
    a.op(Op::POP, &[]);
    // incrArray1Imm: key on the stack, amount in operand 1.
    a.push("k").op(Op::INCR_ARRAY1_IMM, &[slot, 5]);
    a.op(Op::POP, &[]);
    // incrArrayStk: array name, key, amount.
    a.push("arr")
        .push("k")
        .push("10")
        .op(Op::INCR_ARRAY_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "20");
}

/// An unset element starts from 0, exactly as `incr` does.
#[test]
fn incr_array_stk_creates_from_zero() {
    let mut a = Asm::new();
    a.push("arr")
        .push("fresh")
        .push("3")
        .op(Op::INCR_ARRAY_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "3");
}

// -- appendStk / lappendStk / appendArrayStk / lappendArrayStk --------------

/// `appendStk` string-appends and `lappendStk` list-appends to the variable
/// named on the stack, each leaving the new value on the stack.
#[test]
fn append_and_lappend_stk() {
    let mut a = Asm::new();
    a.push("s").push("ab").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("s").push("cd").op(Op::APPEND_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("l").push("a b").op(Op::LAPPEND_STK, &[]);
    let (vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "{a b}");
    assert_eq!(
        vm.get_var("s").map(|v| v.to_str().to_string()),
        Some("abcd".into())
    );
}

/// The `*ArrayStk` forms take `name key value` and append to the element.
#[test]
fn append_and_lappend_array_stk() {
    let mut a = Asm::new();
    a.push("arr")
        .push("k")
        .push("x")
        .op(Op::APPEND_ARRAY_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("arr")
        .push("k")
        .push("y")
        .op(Op::APPEND_ARRAY_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("arr")
        .push("li")
        .push("p q")
        .op(Op::LAPPEND_ARRAY_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("arr")
        .push("li")
        .push("r")
        .op(Op::LAPPEND_ARRAY_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("arr").push("k").op(Op::LOAD_ARRAY_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "xy");
}

/// `lappendArrayStk` appends one element (the whole value), not its elements.
#[test]
fn lappend_array_stk_appends_one_element() {
    let mut a = Asm::new();
    a.push("arr")
        .push("li")
        .push("p q")
        .op(Op::LAPPEND_ARRAY_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "{p q}");
}

// -- the lappendList family -------------------------------------------------

/// `lappendListStk` appends *each* element of the popped list (`lappend v
/// {*}$list`), unlike `lappendStk`.
#[test]
fn lappend_list_stk_appends_each_element() {
    let mut a = Asm::new();
    a.push("l").push("a").op(Op::LAPPEND_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("l").push("b c").op(Op::LAPPEND_LIST_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a b c");
}

/// An unparsable list value errors before any write (C checks the list first).
#[test]
fn lappend_list_stk_rejects_invalid_list() {
    let mut a = Asm::new();
    a.push("l").push("a {b").op(Op::LAPPEND_LIST_STK, &[]);
    let (vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "unmatched open brace in list");
    assert!(vm.get_var("l").is_none(), "the variable must be untouched");
}

/// `lappendListArray` (array from the LVT slot) and `lappendListArrayStk`
/// (array name from the stack) both append each element to the element's list.
#[test]
fn lappend_list_array_forms() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("a b").op(Op::LAPPEND_LIST_ARRAY, &[slot]);
    a.op(Op::POP, &[]);
    a.push("arr")
        .push("k")
        .push("c d")
        .op(Op::LAPPEND_LIST_ARRAY_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a b c d");
}

// -- existArray / existArrayStk --------------------------------------------

/// The array existence tests push 1/0 and never error — not for a missing
/// element, and not for a wholly missing array (C `INST_EXIST_ARRAY`).
#[test]
fn exist_array_forms_never_error() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("k").op(Op::EXIST_ARRAY, &[slot]);
    a.push("missing").op(Op::EXIST_ARRAY, &[slot]);
    a.push("arr").push("k").op(Op::EXIST_ARRAY_STK, &[]);
    a.push("no_such_array")
        .push("k")
        .op(Op::EXIST_ARRAY_STK, &[]);
    a.op(Op::LIST, &[4]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1 0 1 0");
}

/// `existArray` fires the element's read traces *before* testing existence, as
/// C does (`INST_EXIST_ARRAY` looks the element up with `TCL_TRACE_READS`), and
/// still reports 0 for a missing element.
#[test]
fn exist_array_fires_read_traces() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    // `;#` swallows the trailing `name elem op` words the trace appends.
    let setup = "set arr(seed) 1\ntrace add variable arr read {lappend ::fired 1;#}";
    assert!(vm.eval_source(setup).expect("compiles").code.is_ok());
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("missing").op(Op::EXIST_ARRAY, &[slot]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "0", "a missing element still reports 0");
    assert_eq!(
        vm.get_var("fired").map(|v| v.to_str().to_string()),
        Some("1".into()),
        "the read trace must have fired"
    );
}

// -- unsetArrayStk ---------------------------------------------------------

/// With the complain flag clear, unsetting a missing element is silent; the
/// present element is still removed.
#[test]
fn unset_array_stk_nocomplain() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("arr").push("gone").op(Op::UNSET_ARRAY_STK, &[0]);
    a.push("arr").push("k").op(Op::UNSET_ARRAY_STK, &[0]);
    a.push("k").op(Op::EXIST_ARRAY, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "0");
}

/// With the flag set (C's `TCL_LEAVE_ERR_MSG`), a missing element errors with
/// the three-way miss message `unset a(k)` reports (set-old-7.4/7.5).
#[test]
fn unset_array_stk_complain_flag_errors() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("arr").push("gone").op(Op::UNSET_ARRAY_STK, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't unset \"arr(gone)\": no such element in array"
    );

    // A scalar target and a wholly missing variable take the other two arms.
    let mut a = Asm::new();
    a.push("s").push("v").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("s").push("k").op(Op::UNSET_ARRAY_STK, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"s(k)\": variable isn't array");

    let mut a = Asm::new();
    a.push("nope").push("k").op(Op::UNSET_ARRAY_STK, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"nope(k)\": no such variable");
}

// -- arrayExistsStk / arrayMakeImm / arrayMakeStk --------------------------

/// `arrayMakeStk` materialises an empty array (so `arrayExistsStk` flips from
/// 0 to 1) and is a no-op on an existing array.
#[test]
fn array_make_stk_creates_empty_array() {
    let mut a = Asm::new();
    a.push("arr").op(Op::ARRAY_EXISTS_STK, &[]);
    a.push("arr").op(Op::ARRAY_MAKE_STK, &[]);
    a.push("arr").op(Op::ARRAY_EXISTS_STK, &[]);
    a.push("arr").op(Op::ARRAY_MAKE_STK, &[]); // idempotent
    a.push("arr").op(Op::ARRAY_EXISTS_STK, &[]);
    a.op(Op::LIST, &[3]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "0 1 1");
}

/// `arrayMakeImm` does the same for an LVT slot, touching no stack.
#[test]
fn array_make_imm_creates_empty_array() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.op(Op::ARRAY_MAKE_IMM, &[slot]);
    a.op(Op::ARRAY_EXISTS_IMM, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1");
}

/// Over a scalar, both forms raise C's `array set` error
/// (`TclObjVarErrMsg(… "array set", "variable isn't array")`).
#[test]
fn array_make_over_scalar_errors() {
    let mut a = Asm::new();
    a.push("s").push("v").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]).push("s").op(Op::ARRAY_MAKE_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't array set \"s\": variable isn't array");

    // An array *element* name is equally rejected.
    let mut a = Asm::new();
    a.push("arr(k)").op(Op::ARRAY_MAKE_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't array set \"arr(k)\": variable isn't array"
    );

    // The `Imm` form names the variable its LVT slot holds (C fills part1Ptr
    // in from `localName(varFramePtr, index)`).
    let mut a = Asm::new();
    let slot = a.slot("s");
    a.push("v").op(Op::STORE_SCALAR1, &[slot]);
    a.op(Op::POP, &[]).op(Op::ARRAY_MAKE_IMM, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't array set \"s\": variable isn't array");
}

// -- variable -------------------------------------------------------------

/// `variable` links the LVT slot to the namespace variable named on the stack,
/// so a later write through the local lands on the qualified cell — the
/// `variable` command's linking, driven by the opcode.
#[test]
fn variable_links_local_to_namespace_var() {
    let mut a = Asm::new();
    let slot = a.slot("v");
    a.push("ns::v").op(Op::VARIABLE, &[slot]);
    a.push("42").op(Op::STORE_SCALAR1, &[slot]);
    let (vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "42");
    assert_eq!(
        vm.get_var("ns::v").map(|v| v.to_str().to_string()),
        Some("42".into()),
        "the write must reach the namespace variable"
    );
}

// -- currentNamespace / infoLevelNumber / infoLevelArgs -------------------

/// `currentNamespace` pushes the fully-qualified name (`::` at global scope)
/// and `infoLevelNumber` the call depth — the same values `namespace current`
/// and `info level` report.
#[test]
fn current_namespace_and_info_level_number() {
    let mut a = Asm::new();
    a.op(Op::CURRENT_NAMESPACE, &[]);
    a.op(Op::INFO_LEVEL_NUM, &[]);
    a.op(Op::LIST, &[2]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), ":: 0");
}

/// `infoLevelArgs` routes through `info level`'s own core, so its errors are
/// the command's: `bad level "N"` for a level with no call, and the integer
/// coercion error for a non-numeric level.
#[test]
fn info_level_args_matches_the_command() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    let cmd_bad_level = vm
        .eval_source("catch {info level 1} m; set m")
        .expect("compiles")
        .result
        .to_str()
        .to_string();
    let mut a = Asm::new();
    a.push("1").op(Op::INFO_LEVEL_ARGS, &[]);
    let c = run(&mut vm, a);
    // At global scope there is no frame 1, for the opcode as for the command.
    assert_eq!(err_str(&c), "bad level \"1\"");
    assert_eq!(cmd_bad_level, err_str(&c));

    let mut a = Asm::new();
    a.push("foo").op(Op::INFO_LEVEL_ARGS, &[]);
    let c = run(&mut vm, a);
    assert_eq!(err_str(&c), "expected integer but got \"foo\"");
}

// -- resolveCmd / originCmd ----------------------------------------------

/// `resolveCmd` pushes the fully-qualified command name, or the empty string
/// when the name resolves to nothing — it never errors (C
/// `INST_RESOLVE_COMMAND`).
#[test]
fn resolve_cmd_pushes_fqn_or_empty() {
    let mut a = Asm::new();
    a.push("set").op(Op::RESOLVE_CMD, &[]);
    a.push("no_such_cmd_xyz").op(Op::RESOLVE_CMD, &[]);
    a.op(Op::LIST, &[2]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "::set {}");
}

/// `originCmd` pushes the origin's fully-qualified name and errors on a name
/// that resolves to no command (C `INST_ORIGIN_COMMAND`).
#[test]
fn origin_cmd_pushes_origin_and_rejects_unknown() {
    let mut a = Asm::new();
    a.push("set").op(Op::ORIGIN_CMD, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "::set");

    let mut a = Asm::new();
    a.push("no_such_cmd_xyz").op(Op::ORIGIN_CMD, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "invalid command name \"no_such_cmd_xyz\"");
}

/// The origin follows the `namespace import` chain, as C's
/// `TclGetOriginalCommand` does.
#[test]
fn origin_cmd_follows_import_chain() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    let setup = "namespace eval src {proc p {} {return 1}; namespace export p}\n\
                 namespace import ::src::p";
    assert!(vm.eval_source(setup).expect("compiles").code.is_ok());
    let mut a = Asm::new();
    a.push("p").op(Op::ORIGIN_CMD, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "::src::p");
}

// -- clockRead -----------------------------------------------------------

/// `clockRead` reads the same host clock `clock clicks`/`clock seconds` do, so
/// the four readings agree with one another (0 = clicks = µs, 1 = µs, 2 = ms,
/// 3 = s).
#[test]
fn clock_read_units_agree() {
    let mut a = Asm::new();
    a.op(Op::CLOCK_READ, &[0]);
    a.op(Op::CLOCK_READ, &[1]);
    a.op(Op::CLOCK_READ, &[2]);
    a.op(Op::CLOCK_READ, &[3]);
    a.op(Op::LIST, &[4]);
    let (_, c) = run_fresh(a);
    let vals: Vec<i64> = ok_str(&c)
        .split_whitespace()
        .map(|s| s.parse::<i64>().expect("wide integer reading"))
        .collect();
    let &[clicks, micros, millis, secs] = vals.as_slice() else {
        panic!("four readings expected, got {vals:?}");
    };
    assert!(secs > 1_600_000_000, "seconds look like a Unix timestamp");
    // Same clock, different scales — allow one unit of drift between reads.
    assert!((micros / 1_000 - millis).abs() <= 1, "µs vs ms: {vals:?}");
    assert!((millis / 1_000 - secs).abs() <= 1, "ms vs s: {vals:?}");
    assert!(
        (clicks / 1_000 - millis).abs() <= 1,
        "clicks vs ms: {vals:?}"
    );
}

// -- dictGetDef ---------------------------------------------------------

/// `dictGetDef` reads the key path like `dictGet`, but a key missing at any
/// depth yields the default instead of erroring (C `INST_DICT_GET_DEF`).
#[test]
fn dict_get_def_reads_or_defaults() {
    // Present single key.
    let mut a = Asm::new();
    a.push("a 1 b 2")
        .push("b")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "2");

    // Missing leaf key → the default.
    let mut a = Asm::new();
    a.push("a 1 b 2")
        .push("z")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "dflt");

    // Nested path, present and then missing at the first level.
    let mut a = Asm::new();
    a.push("x {a 1}")
        .push("x")
        .push("a")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[2]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1");

    let mut a = Asm::new();
    a.push("x {a 1}")
        .push("y")
        .push("a")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[2]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "dflt");
}

/// A malformed dictionary still errors — the default only covers *absent*
/// keys (C distinguishes `DICT_PATH_NON_EXISTENT` from a failed conversion).
#[test]
fn dict_get_def_rejects_malformed_dict() {
    let mut a = Asm::new();
    a.push("a 1 b")
        .push("a")
        .push("dflt")
        .op(Op::DICT_GET_DEF, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "missing value to go with key");
}

// -- dictRecombineStk --------------------------------------------------

/// `dictRecombineStk` is `dictRecombineImm` with the dict variable's name on
/// the stack (`varName path state`): it writes each expanded key's local back
/// into the variable, dropping keys whose local was unset.
#[test]
fn dict_recombine_stk_writes_back() {
    let mut a = Asm::new();
    // d = {a 1 b 2}
    a.push("d").push("a 1 b 2").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    // Lay down the recombine's own operands (variable name, then key path),
    // then run the `dict with` prologue, which expands the dict's keys into
    // same-named locals and pushes the recombine state on top.
    a.push("d").push("");
    a.push("a 1 b 2").push("").op(Op::DICT_EXPAND, &[]);
    // Rewrite one expanded local (a net-zero stack effect: store then pop).
    a.push("a").push("99").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.op(Op::DICT_RECOMBINE_STK, &[]);
    a.push("d").op(Op::LOAD_SCALAR_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a 99 b 2");
}

/// A key whose expanded local was unset is removed from the dict, exactly as
/// the `Imm` form (and `dict with`) does.
#[test]
fn dict_recombine_stk_drops_unset_keys() {
    let mut a = Asm::new();
    a.push("d").push("a 1 b 2").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("d").push("");
    a.push("a 1 b 2").push("").op(Op::DICT_EXPAND, &[]);
    a.push("b").op(Op::UNSET_STK, &[0]);
    a.op(Op::DICT_RECOMBINE_STK, &[]);
    a.push("d").op(Op::LOAD_SCALAR_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a 1");
}

// -- constImm / constStk ----------------------------------------------

/// `constImm` defines the LVT slot as a constant; re-defining an existing
/// constant silently drops the value (C's `TclIsVarConstant` early exit).
#[test]
fn const_imm_defines_and_redefines_silently() {
    let mut a = Asm::new();
    let slot = a.slot("k");
    a.push("1").op(Op::CONST_IMM, &[slot]);
    a.push("2").op(Op::CONST_IMM, &[slot]); // silent no-op
    a.op(Op::LOAD_SCALAR1, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1", "the second const must not overwrite");
}

/// `constStk` takes the name from the stack, and both forms reject a name that
/// already holds a normal variable / an array / an array element with C's
/// `can't make constant "n": …` messages.
#[test]
fn const_stk_defines_and_reports_conflicts() {
    let mut a = Asm::new();
    a.push("k").push("v").op(Op::CONST_STK, &[]);
    a.push("k").op(Op::LOAD_SCALAR_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "v");

    // Over an existing scalar.
    let mut a = Asm::new();
    a.push("x").push("1").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("x").push("2").op(Op::CONST_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't make constant \"x\": variable already exists"
    );

    // Over an array.
    let mut a = Asm::new();
    a.push("arr").op(Op::ARRAY_MAKE_STK, &[]);
    a.push("arr").push("2").op(Op::CONST_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't make constant \"arr\": variable is array"
    );

    // An array element name.
    let mut a = Asm::new();
    a.push("arr(k)").push("2").op(Op::CONST_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't make constant \"arr(k)\": name refers to an element in an array"
    );
}

/// A constant cannot then be written through the ordinary store path.
#[test]
fn const_blocks_later_writes() {
    let mut a = Asm::new();
    a.push("k").push("1").op(Op::CONST_STK, &[]);
    a.push("k").push("2").op(Op::STORE_SCALAR_STK, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't set \"k\": variable is a constant");
}

// -- expandDrop ------------------------------------------------------

/// `expandDrop` abandons the innermost expansion, truncating the stack back to
/// the depth its `expandStart` recorded (C `INST_EXPAND_DROP`).
#[test]
fn expand_drop_restores_pre_expansion_depth() {
    let mut a = Asm::new();
    a.push("keep");
    a.op(Op::EXPAND_START, &[]);
    a.push("list").push("a b c").op(Op::EXPAND_STKTOP, &[]);
    a.op(Op::EXPAND_DROP, &[]);
    // Only `keep` is left, so the trailing `list 1` sees exactly it.
    a.op(Op::LIST, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "keep");
}

/// A dropped expansion leaves the marker stack clean, so a following
/// `expandStart`/`invokeExpanded` pair still invokes the right word count.
#[test]
fn expand_drop_leaves_marker_stack_clean() {
    let mut a = Asm::new();
    a.op(Op::EXPAND_START, &[]);
    a.push("string").push("cat x y").op(Op::EXPAND_STKTOP, &[]);
    a.op(Op::EXPAND_DROP, &[]);
    a.op(Op::EXPAND_START, &[]);
    a.push("string").push("cat a b").op(Op::EXPAND_STKTOP, &[]);
    a.op(Op::INVOKE_EXPANDED, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "ab");
}

// -- strmap ----------------------------------------------------------

/// `strmap` is the one-pair, case-sensitive `string map`: the stack is
/// `from to string` with the subject on top (C `INST_STR_MAP`).
#[test]
fn str_map_stack_order_and_mapping() {
    let mut a = Asm::new();
    a.push("ab").push("X").push("abcab").op(Op::STR_MAP, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "XcX");
}

/// The C fast paths: an empty `from`, a `from` longer than the subject, and a
/// whole-string match all behave like the general mapping loop.
#[test]
fn str_map_edge_cases() {
    for (from, to, subject, want) in [
        ("", "X", "abc", "abc"),     // empty key never matches
        ("abcd", "X", "abc", "abc"), // key longer than the subject
        ("abc", "X", "abc", "X"),    // whole-string match
        ("abc", "X", "abd", "abd"),  // same length, no match
        ("a", "", "banana", "bnn"),  // empty replacement deletes
        ("A", "X", "aAa", "aXa"),    // case-sensitive (no -nocase form)
    ] {
        let mut a = Asm::new();
        a.push(from).push(to).push(subject).op(Op::STR_MAP, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "strmap {from} {to} {subject}");
    }
}

/// The opcode and the `string map` command agree on the same one-pair map.
#[test]
fn str_map_matches_string_map_command() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    let cmd = vm
        .eval_source("string map {ab X} abcab")
        .expect("compiles")
        .result
        .to_str()
        .to_string();
    let mut a = Asm::new();
    a.push("ab").push("X").push("abcab").op(Op::STR_MAP, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), cmd);
}

// -- iRules dialect operators ------------------------------------------------

/// The dialect string tests (`contains` / `starts_with` / `ends_with` /
/// `equals`), both outcomes each. The operands sit on the stack subject-first
/// with the needle on top — the order `Op::from_binop` codegen pushes a binary
/// expression's operands in (left, then right), the same as `Op::ADD`'s.
#[test]
fn irule_string_tests_both_outcomes() {
    for (op, subject, operand, want) in [
        (Op::IRULE_CONTAINS, "foobar", "oob", "1"),
        (Op::IRULE_CONTAINS, "foobar", "bop", "0"),
        // Every string contains the empty one.
        (Op::IRULE_CONTAINS, "foobar", "", "1"),
        (Op::IRULE_CONTAINS, "", "x", "0"),
        (Op::IRULE_STARTS_WITH, "foobar", "foo", "1"),
        (Op::IRULE_STARTS_WITH, "foobar", "bar", "0"),
        (Op::IRULE_ENDS_WITH, "foobar", "bar", "1"),
        (Op::IRULE_ENDS_WITH, "foobar", "foo", "0"),
        (Op::IRULE_EQUALS, "foobar", "foobar", "1"),
        // `equals` is the word spelling of `eq`: case-sensitive, and a *string*
        // comparison even when both operands look numeric.
        (Op::IRULE_EQUALS, "foobar", "FOOBAR", "0"),
        (Op::IRULE_EQUALS, "1", "1.0", "0"),
    ] {
        let mut a = Asm::new();
        a.push(subject).push(operand).op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{} {subject} {operand}", op.mnemonic());
    }
}

/// `matches_glob` is a case-sensitive `string match`; `matches_regex` runs the
/// Tcl 9 ARE engine (the `[[:<:]]` word edge proves it is the real engine, not a
/// look-alike), also case-sensitive — neither dialect operator has a `-nocase`
/// form.
#[test]
fn irule_glob_and_regex_matching() {
    for (op, subject, pattern, want) in [
        (Op::IRULE_MATCHES_GLOB, "foobar", "foo*", "1"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "*bar", "1"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "f?obar", "1"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "foo", "0"),
        (Op::IRULE_MATCHES_GLOB, "foobar", "FOO*", "0"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "o+b", "1"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "^foo", "1"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "^bar", "0"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "FOO", "0"),
        (Op::IRULE_MATCHES_REGEX, "foobar", "[[:<:]]bar", "0"),
        (Op::IRULE_MATCHES_REGEX, "foo bar", "[[:<:]]bar", "1"),
    ] {
        let mut a = Asm::new();
        a.push(subject).push(pattern).op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{} {subject} {pattern}", op.mnemonic());
    }
}

/// An uncompilable regex is an error, not a silent `0` — the same engine
/// message the compiled `regexp` opcode reports for the same pattern (both drive
/// the shared ARE core).
#[test]
fn irule_matches_regex_rejects_a_bad_pattern() {
    let mut a = Asm::new();
    a.push("foobar").push("[").op(Op::IRULE_MATCHES_REGEX, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "brackets [] not balanced");

    // `Op::REGEXP` takes its operands the other way round (pattern under the
    // string) and `TCL_REG_ADVANCED` as its flags operand.
    let mut a = Asm::new();
    a.push("[").push("foobar").op(Op::REGEXP, &[3]);
    let (_, regexp_op) = run_fresh(a);
    assert_eq!(err_str(&regexp_op), err_str(&c));
}

/// `and` / `or` reduce Tcl truthiness (the boolean words included) to `1`/`0`.
#[test]
fn irule_word_and_or_truth_table() {
    for (op, left, right, want) in [
        (Op::IRULE_WORD_AND, "1", "1", "1"),
        (Op::IRULE_WORD_AND, "1", "0", "0"),
        (Op::IRULE_WORD_AND, "0", "1", "0"),
        (Op::IRULE_WORD_AND, "0", "0", "0"),
        (Op::IRULE_WORD_AND, "true", "yes", "1"),
        (Op::IRULE_WORD_AND, "2", "-1", "1"),
        (Op::IRULE_WORD_OR, "1", "1", "1"),
        (Op::IRULE_WORD_OR, "1", "0", "1"),
        (Op::IRULE_WORD_OR, "0", "1", "1"),
        (Op::IRULE_WORD_OR, "0", "0", "0"),
        (Op::IRULE_WORD_OR, "no", "off", "0"),
    ] {
        let mut a = Asm::new();
        a.push(left).push(right).op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{} {left} {right}", op.mnemonic());
    }
}

/// The shared tree-walk coerces `and`/`or`'s right operand only when the left
/// does not already decide the result; with both operands on the stack the
/// opcode reproduces that, so a junk right operand errors in exactly the cases
/// the walker treats as an error.
#[test]
fn irule_word_logic_short_circuits_its_coercion() {
    for (op, left, want) in [
        (Op::IRULE_WORD_AND, "0", "0"),
        (Op::IRULE_WORD_OR, "1", "1"),
    ] {
        let mut a = Asm::new();
        a.push(left).push("zzz").op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(
            ok_str(&c),
            want,
            "{} decided by the left operand",
            op.mnemonic()
        );
    }
    for (op, left) in [(Op::IRULE_WORD_AND, "1"), (Op::IRULE_WORD_OR, "0")] {
        let mut a = Asm::new();
        a.push(left).push("zzz").op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(
            err_str(&c),
            "expected boolean value but got \"zzz\"",
            "{} must coerce the right operand",
            op.mnemonic()
        );
    }
    // A junk *left* operand always errors, whatever the operator.
    let mut a = Asm::new();
    a.push("zzz").push("1").op(Op::IRULE_WORD_AND, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "expected boolean value but got \"zzz\"");
}

/// `not` is the word spelling of `!`: it shares the boolean coercion and reports
/// its own spelling in the operand error.
#[test]
fn irule_word_not_negates_truthiness() {
    for (v, want) in [
        ("1", "0"),
        ("0", "1"),
        ("yes", "0"),
        ("off", "1"),
        ("7", "0"),
    ] {
        let mut a = Asm::new();
        a.push(v).op(Op::IRULE_WORD_NOT, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "not {v}");
    }
    let mut a = Asm::new();
    a.push("zzz").op(Op::IRULE_WORD_NOT, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "cannot use non-numeric string \"zzz\" as operand of \"not\""
    );
}

/// Each dialect operator agrees with the core command it is defined in terms of,
/// so the opcodes cannot drift from `[string first]` / `[string equal]` /
/// `[string match]` / `[regexp]`.
#[test]
fn irule_operators_agree_with_the_core_commands() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    for (op, subject, operand, cmd) in [
        (
            Op::IRULE_CONTAINS,
            "foobar",
            "oob",
            "expr {[string first oob foobar] >= 0}",
        ),
        (
            Op::IRULE_CONTAINS,
            "foobar",
            "bop",
            "expr {[string first bop foobar] >= 0}",
        ),
        (
            Op::IRULE_EQUALS,
            "foobar",
            "FOOBAR",
            "string equal foobar FOOBAR",
        ),
        (
            Op::IRULE_EQUALS,
            "foobar",
            "foobar",
            "string equal foobar foobar",
        ),
        (
            Op::IRULE_MATCHES_GLOB,
            "foobar",
            "f?o*",
            "string match f?o* foobar",
        ),
        (
            Op::IRULE_MATCHES_REGEX,
            "foobar",
            "o+b",
            "regexp {o+b} foobar",
        ),
    ] {
        let want = vm
            .eval_source(cmd)
            .expect("compiles")
            .result
            .to_str()
            .to_string();
        let mut a = Asm::new();
        a.push(subject).push(operand).op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), want, "{} vs [{cmd}]", op.mnemonic());
    }
}

// -- yield / yieldToInvoke / coroName ----------------------------------------

/// Run `body` as the compiled body of a global proc `p`.
///
/// The seam is the pre-compiled-body cache: `cmd_proc` takes the module-supplied
/// assembly for the first definition of a name, so seeding `p` here (under the
/// VM's own registration key — `p`, unrooted) and then defining it from source
/// gives a proc whose body is this hand-built stream. That is the only way to
/// reach an opcode from *inside* a coroutine or a method call, which is where the
/// coroutine and `TclOO` opcodes live.
fn vm_with_seeded_proc(body: Asm) -> Vm {
    let mut procedures = HashMap::new();
    procedures.insert("p".to_string(), body.build());
    let module = ModuleAsm {
        top_level: Asm::new().build(),
        procedures,
    };
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    assert_eq!(vm.run_module(&module).code, Code::Ok, "seeding failed");
    vm.eval_source("proc p {} {this body is replaced by the seeded one}")
        .expect("proc defines");
    vm
}

/// Evaluate `src`, asserting it compiled, and return its completion.
fn eval(vm: &mut Vm, src: &str) -> Completion<Value> {
    vm.eval_source(src).expect("compiles")
}

/// `coroName` pushes the empty string outside a coroutine (C
/// `INST_COROUTINE_NAME` pushes a fresh empty object when there is no
/// `corPtr`) and the running coroutine's fully-qualified command name inside
/// one.
#[test]
fn coro_name_outside_and_inside_a_coroutine() {
    let mut a = Asm::new();
    a.op(Op::CORO_NAME, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "");

    // Inside: the seeded body pushes `coroName` and yields it, so the
    // `coroutine` command's result *is* what the opcode pushed.
    let mut body = Asm::new();
    body.op(Op::CORO_NAME, &[]).op(Op::YIELD, &[]);
    let mut vm = vm_with_seeded_proc(body);
    let c = eval(&mut vm, "coroutine c p");
    assert_eq!(ok_str(&c), "::c");
}

/// Outside a coroutine both suspend opcodes report C's `INST_YIELD` /
/// `INST_YIELD_TO_INVOKE` message (C also sets errorcode
/// `TCL COROUTINE ILLEGAL_YIELD`), and nothing is left pending afterwards.
#[test]
fn yield_opcodes_outside_a_coroutine_match_c() {
    let mut a = Asm::new();
    a.push("v").op(Op::YIELD, &[]);
    let (mut vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "yield can only be called in a coroutine");

    let mut a = Asm::new();
    a.push("list a").op(Op::YIELD_TO_INVOKE, &[]);
    let c = run(&mut vm, a);
    assert_eq!(err_str(&c), "yieldto can only be called in a coroutine");

    // The rejected requests left no suspend armed: a following push still runs
    // to completion in this same VM.
    let mut a = Asm::new();
    a.push("after");
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "after");
}

/// In a coroutine the `yield` opcode suspends: the yielded value comes out of
/// the resume command, and the value the next resume delivers replaces it on the
/// operand stack (C's `pc++; cleanup = 1; TEBC_YIELD()`), so it becomes the
/// body's result.
#[test]
fn yield_opcode_suspends_and_takes_the_resume_value() {
    let mut body = Asm::new();
    body.push("yielded").op(Op::YIELD, &[]);
    let mut vm = vm_with_seeded_proc(body);
    assert_eq!(ok_str(&eval(&mut vm, "coroutine c p")), "yielded");
    assert_eq!(ok_str(&eval(&mut vm, "c resumed")), "resumed");
    // The body ran off its end, so the coroutine command is gone.
    assert_eq!(ok_str(&eval(&mut vm, "info commands c")), "");
}

/// The `yield` opcode keeps the yield-boundary check: reached across a host
/// re-entry (here an `TclOO` method call) it is C Tcl's
/// `cannot yield: C stack busy`, exactly as the builtin is.
#[test]
fn yield_opcode_still_rejects_a_yield_across_a_host_re_entry() {
    let mut body = Asm::new();
    body.push("v").op(Op::YIELD, &[]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, "oo::class create C {method m {} {p}}\nC create o");
    let c = eval(&mut vm, "coroutine c o m");
    assert_eq!(err_str(&c), "cannot yield: C stack busy");
}

/// `yieldToInvoke` parks the coroutine and runs the popped command list in the
/// *resuming* context, whose result the resumer receives; the next resume's
/// arguments come back as a list, replacing the list on the stack.
#[test]
fn yield_to_invoke_runs_the_command_list_in_the_resumer() {
    let mut body = Asm::new();
    body.push("list a b").op(Op::YIELD_TO_INVOKE, &[]);
    let mut vm = vm_with_seeded_proc(body);
    assert_eq!(ok_str(&eval(&mut vm, "coroutine c p")), "a b");
    assert_eq!(ok_str(&eval(&mut vm, "c x y")), "x y");
}

// -- TclOO -------------------------------------------------------------------

/// `tclooIsObject` never errors: `0` for a name that is not an object, `1` for a
/// real one — the test `info object isa object` (the form C compiles to this
/// opcode) performs.
#[test]
fn tcloo_is_object_reports_without_erroring() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    eval(&mut vm, "oo::class create C {}\nC create o");
    for (name, want) in [("o", "1"), ("C", "1"), ("junk", "0"), ("", "0")] {
        let mut a = Asm::new();
        a.push(name).op(Op::TCLOO_IS_OBJECT, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), want, "tclooIsObject {name}");
        let cmd = eval(&mut vm, &format!("info object isa object {{{name}}}"));
        assert_eq!(ok_str(&cmd), want, "info object isa object {name}");
    }
}

/// `tclooClass` / `tclooNamespace` report exactly what `info object class` /
/// `info object namespace` do (the command forms C compiles to them).
#[test]
fn tcloo_class_and_namespace_match_info_object() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    eval(&mut vm, "oo::class create C {}\nC create o");
    for (op, cmd) in [
        (Op::TCLOO_CLASS, "info object class o"),
        (Op::TCLOO_NS, "info object namespace o"),
    ] {
        let want = ok_str(&eval(&mut vm, cmd));
        let mut a = Asm::new();
        a.push("o").op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), want, "{} vs [{cmd}]", op.mnemonic());
    }
    // The class of a class is its metaclass.
    let mut a = Asm::new();
    a.push("C").op(Op::TCLOO_CLASS, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "::oo::class");
}

/// A name that is not an object is the shared lookup error (C's
/// `%s does not refer to an object`), identical to the command form's.
#[test]
fn tcloo_class_and_namespace_reject_a_non_object() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    for (op, cmd) in [
        (Op::TCLOO_CLASS, "info object class junk"),
        (Op::TCLOO_NS, "info object namespace junk"),
    ] {
        let want = err_str(&eval(&mut vm, cmd));
        let mut a = Asm::new();
        a.push("junk").op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(err_str(&c), want, "{} vs [{cmd}]", op.mnemonic());
        assert!(
            want.ends_with("does not refer to an object"),
            "C's lookup message: {want}"
        );
    }
}

/// Outside a method the compiled forms report C's `INST_TCLOO_*` messages — not
/// the `invalid command name` the *command* forms give, since in C `self` /
/// `next` / `nextto` are only resolvable inside an object's namespace.
#[test]
fn tcloo_self_next_and_nextto_outside_a_method() {
    let mut a = Asm::new();
    a.op(Op::TCLOO_SELF, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "self may only be called from inside a method");

    let mut a = Asm::new();
    a.push("next").op(Op::TCLOO_NEXT, &[1]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "next may only be called from inside a method");

    let mut a = Asm::new();
    a.push("nextto").push("C").op(Op::TCLOO_NEXT_CLASS, &[2]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "nextto may only be called from inside a method"
    );
}

/// `tclooSelf` pushes the current object's command name (C's
/// `TclOOObjectName`), the value `self` / `self object` reports.
#[test]
fn tcloo_self_pushes_the_current_object() {
    let mut body = Asm::new();
    body.op(Op::TCLOO_SELF, &[]);
    let mut vm = vm_with_seeded_proc(body);
    // `m` reaches the opcode (through the seeded proc); `n` asks the command.
    eval(
        &mut vm,
        "oo::class create C {\nmethod m {} {p}\nmethod n {} {self object}\n}\nC create o",
    );
    let via_opcode = ok_str(&eval(&mut vm, "o m"));
    assert_eq!(via_opcode, "::o");
    assert_eq!(via_opcode, ok_str(&eval(&mut vm, "o n")));
}

/// `tclooNext` / `tclooNextClass` invoke the next implementation on the method
/// chain through the `next` / `nextto` cores. The operand counts *all* the words
/// C pushes — the command word itself first (C's `skip`), then the arguments,
/// which for `nextto` start with the class.
#[test]
fn tcloo_next_and_next_class_invoke_the_chain() {
    let classes = "oo::class create B {method m {} {return base}}\n\
                   oo::class create D {superclass B\nmethod m {} {p}}\n\
                   D create o";
    let mut body = Asm::new();
    body.push("next").op(Op::TCLOO_NEXT, &[1]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, classes);
    assert_eq!(ok_str(&eval(&mut vm, "o m")), "base");

    let mut body = Asm::new();
    body.push("nextto").push("B").op(Op::TCLOO_NEXT_CLASS, &[2]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, classes);
    assert_eq!(ok_str(&eval(&mut vm, "o m")), "base");
}

/// At the end of the chain `tclooNext` reports the core's
/// `no next <kind> implementation` (C's `INST_TCLOO_NEXT` message for the same
/// state), and a zero word count is rejected rather than read off an empty
/// stack.
#[test]
fn tcloo_next_at_the_end_of_the_chain_and_underflow() {
    let mut body = Asm::new();
    body.push("next").op(Op::TCLOO_NEXT, &[1]);
    let mut vm = vm_with_seeded_proc(body);
    eval(&mut vm, "oo::class create B {method m {} {p}}\nB create o");
    assert_eq!(
        err_str(&eval(&mut vm, "o m")),
        "no next method implementation"
    );

    let mut a = Asm::new();
    a.op(Op::TCLOO_NEXT, &[0]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "tclooNext: stack underflow");
}

// -- unsetArray (LVT slot form) --------------------------------------------

/// `unsetArray` honours its flags operand exactly as `unsetArrayStk` does: C
/// reads `flags = TclGetUInt1AtPtr(pc + 1) ? TCL_LEAVE_ERR_MSG : 0`
/// (`tclExecute.c:3814`) and, when it is set and the element is absent,
/// deliberately falls to `slowUnsetArray` (3837-3839, 3851-3862) so
/// `TclLookupArrayElement(…, flags, "unset", …)` reports the miss and
/// `errorInUnset` (3893) raises it. With the flag clear it is silent (the early
/// `NEXT_INST_F(6, 1, 0)` at 3843-3848).
#[test]
fn unset_array_flag_clear_is_silent() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("gone").op(Op::UNSET_ARRAY, &[0, slot]);
    a.push("k").op(Op::UNSET_ARRAY, &[0, slot]);
    a.push("k").op(Op::EXIST_ARRAY, &[slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "0");
}

/// With the flag set, the three-way `tclVar.c` miss text (NOSUCHELEMENT /
/// NEEDARRAY / NOSUCHVAR, `tclVar.c:119-122`) — identical to `unsetArrayStk`'s,
/// which reaches it through `TclObjUnsetVar2` instead.
#[test]
fn unset_array_complain_flag_errors() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("gone").op(Op::UNSET_ARRAY, &[1, slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't unset \"arr(gone)\": no such element in array"
    );

    let mut a = Asm::new();
    let slot = a.slot("s");
    a.push("v").op(Op::STORE_SCALAR4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("k").op(Op::UNSET_ARRAY, &[1, slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"s(k)\": variable isn't array");

    let mut a = Asm::new();
    let slot = a.slot("nope");
    a.push("k").op(Op::UNSET_ARRAY, &[1, slot]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"nope(k)\": no such variable");
}

// -- strfind / strrfind ----------------------------------------------------

/// An **empty needle is a miss**. Both C helpers open with
/// `if (ln == 0) { /* We don't find empty substrings.  Bizarre! */ … }` leaving
/// the result at -1 (`tclStringObj.c:3853-3858` `TclStringFirst`, `:3948-3956`
/// `TclStringLast`), where Rust's `str::find`/`rfind` would answer 0 and the
/// haystack length. Stack order is C's `TclStringFirst(OBJ_UNDER_TOS,
/// OBJ_AT_TOS, …)` — needle under, haystack on top (`tclExecute.c:5555-5567`).
#[test]
fn str_find_empty_needle_is_minus_one() {
    for (op, want) in [(Op::STR_FIND, "-1"), (Op::STR_RFIND, "-1")] {
        let mut a = Asm::new();
        a.push("").push("abc").op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{op:?} with an empty needle");
    }
    // A non-empty needle keeps its character index (first vs last occurrence).
    let mut a = Asm::new();
    a.push("l").push("hello").op(Op::STR_FIND, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "2");
    let mut a = Asm::new();
    a.push("l").push("hello").op(Op::STR_RFIND, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "3");
}

/// The opcodes agree with the `string first`/`string last` commands, which
/// already returned -1 for an empty needle.
#[test]
fn str_find_matches_string_first_last_commands() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    for (sub, op) in [("first", Op::STR_FIND), ("last", Op::STR_RFIND)] {
        let cmd = vm
            .eval_source(&format!("string {sub} \"\" abc"))
            .expect("compiles")
            .result
            .to_str()
            .to_string();
        let mut a = Asm::new();
        a.push("").push("abc").op(op, &[]);
        let c = run(&mut vm, a);
        assert_eq!(ok_str(&c), cmd, "string {sub} vs {op:?}");
    }
}

// -- strupper / strlower / strtitle ---------------------------------------

/// C `INST_STR_UPPER`/`LOWER`/`TITLE` (`tclExecute.c:5284-5334`) call
/// `Tcl_UtfToUpper`/`ToLower`/`ToTitle`, which map **one code point to one code
/// point** through `Tcl_UniCharToUpper` &co (`tclUtf.c:1777-1858`) — Tcl's
/// tables hold only Unicode's *simple* mappings, so `ß` (U+00DF, no simple
/// uppercase) is unchanged and the character count never changes. Rust's
/// `to_uppercase()`/`to_lowercase()` implement *full* mapping and expand
/// (`ß` → `SS`, `İ` → `i` + U+0307), which is what these arms used to do.
#[test]
fn str_case_ops_use_simple_unicode_mapping() {
    for (op, subject, want) in [
        // Expanding full mappings have no C counterpart: character preserved.
        (Op::STR_UPPER, "\u{00DF}", "\u{00DF}"), // ß, not SS
        (Op::STR_LOWER, "\u{0130}", "\u{0130}"), // İ, not i + U+0307
        (Op::STR_TITLE, "\u{00DF}x", "\u{00DF}x"),
        // Ordinary 1:1 mappings still apply.
        (Op::STR_UPPER, "aé\u{0131}", "AÉI"),
        (Op::STR_LOWER, "AÉ", "aé"),
        (Op::STR_TITLE, "hELLO wORLD", "Hello world"),
        // `Tcl_UniCharToTitle` differs from uppercase for the Latin digraphs.
        (Op::STR_TITLE, "\u{01C4}z", "\u{01C5}z"), // DŽ → Dž
    ] {
        let mut a = Asm::new();
        a.push(subject).op(op, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "{op:?} {subject:?}");
    }
}

/// The opcodes and the `string toupper`/`tolower`/`totitle` commands share the
/// simple-mapping helpers, so they cannot drift apart again.
#[test]
fn str_case_ops_match_string_case_commands() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    for (sub, op) in [
        ("toupper", Op::STR_UPPER),
        ("tolower", Op::STR_LOWER),
        ("totitle", Op::STR_TITLE),
    ] {
        for subject in ["\u{00DF}", "\u{0130}", "hELLO wORLD", "\u{01C4}z"] {
            let cmd = vm
                .eval_source(&format!("string {sub} {{{subject}}}"))
                .expect("compiles")
                .result
                .to_str()
                .to_string();
            let mut a = Asm::new();
            a.push(subject).op(op, &[]);
            let c = run(&mut vm, a);
            assert_eq!(ok_str(&c), cmd, "string {sub} {subject:?} vs {op:?}");
        }
    }
}

// -- tryCvtToBoolean -------------------------------------------------------

/// C `INST_TRY_CVT_TO_BOOLEAN` (`tclExecute.c:6404-6414`; table
/// `tclCompile.c:616` — `{"tryCvtToBoolean", 1, +1, 0, {OPERAND_NONE}}`, "Try
/// converting stktop to boolean if possible. **No errors.** Stack: … value =>
/// … value isStrictBool") leaves the value in place and pushes a 0/1 flag, and
/// never raises. Here the flag is consumed by `strEq` against a literal so both
/// the flag *and* the surviving value are observed. The flag is the **strict**
/// acceptor (`TclSetBooleanFromAny` → `ParseBoolean`, `tclObj.c:2100-2280`), so
/// `2` and `1.5` are *not* booleans even though they are truthy in `if`.
#[test]
fn try_cvt_to_boolean_pushes_flag_and_keeps_value() {
    for (subject, want_flag) in [
        ("yes", "1"),
        ("0", "1"),
        ("1", "1"),
        ("true", "1"),
        ("of", "1"), // unique prefix of `off`
        ("foo", "0"),
        ("", "0"),
        ("2", "0"),
        ("1.5", "0"),
        ("o", "0"), // ambiguous prefix (`on`/`off`)
    ] {
        // … value flag => the flag, with the value still underneath.
        let mut a = Asm::new();
        a.push(subject).op(Op::TRY_CVT_TO_BOOLEAN, &[]);
        a.push(want_flag).op(Op::STR_EQ, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), "1", "tryCvtToBoolean {subject:?} flag");

        // Discard the flag and the original value must still be there — the +1
        // stack effect, not the 0 our old pop/re-push arm had.
        let mut a = Asm::new();
        a.push(subject).op(Op::TRY_CVT_TO_BOOLEAN, &[]);
        a.op(Op::POP, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), subject, "tryCvtToBoolean {subject:?} value");
    }
}

// -- listIndex / lindexMulti ----------------------------------------------

/// C `INST_LIST_INDEX` (`tclExecute.c:4696-4768`) only *fast-paths* an integer
/// index; anything else falls through to `TclLindexList` (4754), so the index
/// word may be a list of indices (nested `lindex`) or empty (the whole list),
/// and a spec that is neither is `bad index …` (4758-4761).
#[test]
fn list_index_implements_tcl_lindex_list() {
    for (list, idx, want) in [
        ("{a b} {c d}", "1 0", "c"),        // nested drill
        ("{a b} {c d}", "", "{a b} {c d}"), // empty index list ⇒ whole list
        ("a b c", "1", "b"),                // plain integer
        ("a b c", "end-1", "b"),            // end-relative
        ("a b c", "9", ""),                 // out of range ⇒ empty
        ("a b c", "-1", ""),                // before the start ⇒ empty
        ("{a b} {c d}", "1 9", ""),         // out of range at depth
    ] {
        let mut a = Asm::new();
        a.push(list).push(idx).op(Op::LIST_INDEX, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "listIndex {list:?} {idx:?}");
    }
}

/// A syntactically bad index is an error, not the empty string.
#[test]
fn list_index_bad_index_errors() {
    for idx in ["foo", "1.5", "end-"] {
        let mut a = Asm::new();
        a.push("a b c").push(idx).op(Op::LIST_INDEX, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(
            err_str(&c),
            format!("bad index \"{idx}\": must be integer?[+-]integer? or end?[+-]integer?"),
            "listIndex bad index {idx:?}"
        );
    }
}

/// `lindexMulti` is C's `TclLindexFlat` (`tclExecute.c:4833-4858`): each operand
/// is *one* index (never re-split as an index list), an out-of-range index gives
/// the empty string, and a malformed one returns `NULL` → `goto gotError`.
#[test]
fn lindex_multi_validates_every_index() {
    let mut a = Asm::new();
    a.push("{a b} {c d}").push("1").push("0");
    a.op(Op::LINDEX_MULTI, &[3]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "c");

    let mut a = Asm::new();
    a.push("{a b} {c d}").push("0").push("9");
    a.op(Op::LINDEX_MULTI, &[3]);
    let (_, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "");

    let mut a = Asm::new();
    a.push("{a b} {c d}").push("0").push("foo");
    a.op(Op::LINDEX_MULTI, &[3]);
    let (_, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "bad index \"foo\": must be integer?[+-]integer? or end?[+-]integer?"
    );
}

/// The compiled forms agree with the `lindex` command, which already
/// implemented `TclLindexList`/`TclLindexFlat` faithfully.
#[test]
fn list_index_matches_lindex_command() {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(compiler::svc()));
    let cmd = vm
        .eval_source("lindex {{a b} {c d}} {1 0}")
        .expect("compiles")
        .result
        .to_str()
        .to_string();
    let mut a = Asm::new();
    a.push("{a b} {c d}").push("1 0").op(Op::LIST_INDEX, &[]);
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), cmd);
}

// -- strindex / strrange ---------------------------------------------------

/// C `INST_STR_INDEX` (`tclExecute.c:5336-5380`) and `INST_STR_RANGE`
/// (`:5382-5406`) both run their indices through `TclGetIntForIndexM` and treat
/// a *parse* failure as `goto gotError`; only the separate range tests
/// (`index < 0 || index >= slength`, and `toIdx == TCL_INDEX_NONE`) produce the
/// empty result. Clamping a bad spec to 0/-1 turned a typo into a
/// plausible-looking substring.
#[test]
fn str_index_and_range_reject_bad_indices() {
    let bad = |spec: &str| {
        format!("bad index \"{spec}\": must be integer?[+-]integer? or end?[+-]integer?")
    };
    let mut a = Asm::new();
    a.push("abc").push("foo").op(Op::STR_INDEX, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("foo"));

    // A list-valued index is not an index spec for the string ops.
    let mut a = Asm::new();
    a.push("abc").push("1 0").op(Op::STR_INDEX, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("1 0"));

    // `first` is validated before `last`, matching C's two sequential
    // `TclGetIntForIndexM` calls.
    let mut a = Asm::new();
    a.push("abcde").push("foo").push("2").op(Op::STR_RANGE, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("foo"));

    let mut a = Asm::new();
    a.push("abcde").push("1").push("bar").op(Op::STR_RANGE, &[]);
    let (_, c) = run_fresh(a);
    assert_eq!(err_str(&c), bad("bar"));
}

/// Valid-but-out-of-range indices keep C's clamp/empty behaviour — no error.
#[test]
fn str_index_and_range_still_clamp_valid_indices() {
    for (idx, want) in [("0", "a"), ("2", "c"), ("end", "c"), ("9", ""), ("-1", "")] {
        let mut a = Asm::new();
        a.push("abc").push(idx).op(Op::STR_INDEX, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "strindex abc {idx}");
    }
    for (from, to, want) in [
        ("1", "3", "bcd"),
        ("0", "end", "abcde"),
        ("-3", "2", "abc"), // negative `first` clamps to 0
        ("0", "end+1", "abcde"),
        ("3", "1", ""),  // reversed range
        ("0", "-1", ""), // `toIdx == TCL_INDEX_NONE`
        ("9", "20", ""),
    ] {
        let mut a = Asm::new();
        a.push("abcde").push(from).push(to).op(Op::STR_RANGE, &[]);
        let (_, c) = run_fresh(a);
        assert_eq!(ok_str(&c), want, "strrange abcde {from} {to}");
    }
}

/// A `tcl-compiler`-backed compile service, so the tests that need real Tcl
/// setup (traces, `namespace import`, command dispatch) can `eval_source`.
mod compiler {
    use tcl_registry::CommandRegistry;
    use tcl_vm::{CompileError, CompileService};

    pub struct Svc {
        registry: CommandRegistry,
    }

    impl CompileService for Svc {
        type Module = tcl_bytecode::ModuleAsm;

        fn compile(&self, src: &str) -> Result<tcl_bytecode::ModuleAsm, CompileError> {
            if let Some(msg) = tcl_compiler::lowering::first_fatal_parse_error(src) {
                return Err(CompileError(msg));
            }
            let ir = tcl_compiler::lowering::lower_to_ir(src, &self.registry);
            let cfg = tcl_compiler::cfg_builder::build_cfg_codegen(&ir, false);
            Ok(tcl_compiler::codegen::codegen_module(
                &cfg,
                &ir,
                &self.registry,
            ))
        }
    }

    pub fn svc() -> Svc {
        Svc {
            registry: CommandRegistry::build_default(),
        }
    }
}
