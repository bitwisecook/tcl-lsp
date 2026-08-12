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

//! Per-opcode parity tests for the variable/array/dict/introspection
//! instructions the compiler does not (yet) emit.
//!
//! Each test hand-assembles a [`FunctionAsm`] — the artifact `tcl-compiler`'s
//! `codegen_module` produces — and runs it through [`Vm::run_function`], so the
//! opcode under test is exercised directly rather than through whatever
//! sequence codegen happens to choose. Expectations are pinned to
//! `tclExecute.c`'s `TEBCresume` cases (stack order, result value, error
//! message) for Tcl 9.0.4.

use std::collections::HashMap;

use tcl_bytecode::{
    FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op, Operand, bytecode_imm, layout,
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
