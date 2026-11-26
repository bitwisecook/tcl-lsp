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

//! Variable existence and lifecycle opcodes: `existArray`, `unsetArray*`, `arrayMake*`, `variable` and `const*`.

use super::*;

// `existArray` / `existArrayStk`.

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
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1 0 1 0");
}

/// `existArray` fires the element's read traces *before* testing existence, as
/// C does (`INST_EXIST_ARRAY` looks the element up with `TCL_TRACE_READS`), and
/// still reports 0 for a missing element.
#[test]
fn exist_array_fires_read_traces() {
    let mut vm = tcl_vm::test_support::native_vm();
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

// `unsetArrayStk`.

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
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't unset \"arr(gone)\": no such element in array"
    );

    // A scalar target and a wholly missing variable take the other two arms.
    let mut a = Asm::new();
    a.push("s").push("v").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("s").push("k").op(Op::UNSET_ARRAY_STK, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"s(k)\": variable isn't array");

    let mut a = Asm::new();
    a.push("nope").push("k").op(Op::UNSET_ARRAY_STK, &[1]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"nope(k)\": no such variable");
}

// `arrayExistsStk` / `arrayMakeImm` / `arrayMakeStk`.

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
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "0 1 1");
}

/// `arrayMakeImm` does the same for an LVT slot, touching no stack.
#[test]
fn array_make_imm_creates_empty_array() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.op(Op::ARRAY_MAKE_IMM, &[slot]);
    a.op(Op::ARRAY_EXISTS_IMM, &[slot]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1");
}

/// Over a scalar, both forms raise C's `array set` error
/// (`TclObjVarErrMsg(… "array set", "variable isn't array")`).
#[test]
fn array_make_over_scalar_errors() {
    let mut a = Asm::new();
    a.push("s").push("v").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]).push("s").op(Op::ARRAY_MAKE_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't array set \"s\": variable isn't array");

    // An array *element* name is looked up without creating the element
    // (`createPart2` 0), so a missing one fails the lookup and an existing
    // one is rejected as not an array.
    let mut a = Asm::new();
    a.push("arr(k)").op(Op::ARRAY_MAKE_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't set \"arr(k)\": no such element in array"
    );
    let mut a = Asm::new();
    a.push("arr(k)").push("v").op(Op::STORE_STK, &[]);
    a.op(Op::POP, &[])
        .push("arr(k)")
        .op(Op::ARRAY_MAKE_STK, &[]);
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't array set \"s\": variable isn't array");
}

// Variable.

/// `variable` links the LVT slot to the namespace variable named on the stack,
/// so a later write through the local lands on the qualified cell — the
/// `variable` command's linking, driven by the opcode.
#[test]
fn variable_links_local_to_namespace_var() {
    let mut a = Asm::new();
    let slot = a.slot("v");
    a.push("ns::v").op(Op::VARIABLE, &[slot]);
    a.push("42").op(Op::STORE_SCALAR1, &[slot]);
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    assert_eq!(ok_str(&eval(&mut vm, "namespace eval ns {}")), "");
    let c = run(&mut vm, a);
    assert_eq!(ok_str(&c), "42");
    assert_eq!(
        vm.get_var("ns::v").map(|v| v.to_str().to_string()),
        Some("42".into()),
        "the write must reach the namespace variable"
    );
}

/// `variable` resolves only through a namespace token that already exists.
/// The opcode must not manufacture the missing parent as a side effect of
/// creating its local link.
#[test]
fn variable_rejects_a_missing_parent_namespace() {
    let mut a = Asm::new();
    let slot = a.slot("v");
    a.push("::missing::v").op(Op::VARIABLE, &[slot]);
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    let c = run(&mut vm, a);
    assert_eq!(
        err_str(&c),
        "can't access \"::missing::v\": parent namespace doesn't exist"
    );
    assert_eq!(err_code(&c), "TCL LOOKUP VARNAME ::missing::v");
    assert_eq!(ok_str(&eval(&mut vm, "namespace exists ::missing")), "0");
}

// `constImm` / `constStk`.

/// `constImm` defines the LVT slot as a constant; re-defining an existing
/// constant silently drops the value (C's `TclIsVarConstant` early exit).
#[test]
fn const_imm_defines_and_redefines_silently() {
    let mut a = Asm::new();
    let slot = a.slot("k");
    a.push("1").op(Op::CONST_IMM, &[slot]);
    a.push("2").op(Op::CONST_IMM, &[slot]); // silent no-op
    a.op(Op::LOAD_SCALAR1, &[slot]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "1", "the second const must not overwrite");
}

/// `constStk` takes the name from the stack, and both forms reject a name that
/// already holds a normal variable or an array with C's
/// `can't make constant "n": …` messages.
#[test]
fn const_stk_defines_and_reports_conflicts() {
    let mut a = Asm::new();
    a.push("k").push("v").op(Op::CONST_STK, &[]);
    a.push("k").op(Op::LOAD_SCALAR_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "v");

    // Over an existing scalar.
    let mut a = Asm::new();
    a.push("x").push("1").op(Op::STORE_SCALAR_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("x").push("2").op(Op::CONST_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't make constant \"x\": variable already exists"
    );

    // Over an array.
    let mut a = Asm::new();
    a.push("arr").op(Op::ARRAY_MAKE_STK, &[]);
    a.push("arr").push("2").op(Op::CONST_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't make constant \"arr\": variable is array"
    );

    // An array element name fails the element lookup first, as
    // `proc q {} {set n arr(k); const $n 2}` does in tclsh 9.0.4.
    let mut a = Asm::new();
    a.push("arr(k)").push("2").op(Op::CONST_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't const \"arr(k)\": no such element in array"
    );
}

/// A constant cannot then be written through the ordinary store path.
#[test]
fn const_blocks_later_writes() {
    let mut a = Asm::new();
    a.push("k").push("1").op(Op::CONST_STK, &[]);
    a.push("k").push("2").op(Op::STORE_SCALAR_STK, &[]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't set \"k\": variable is a constant");
}

// `unsetArray` (LVT slot form).

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
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't unset \"arr(gone)\": no such element in array"
    );

    let mut a = Asm::new();
    let slot = a.slot("s");
    a.push("v").op(Op::STORE_SCALAR4, &[slot]);
    a.op(Op::POP, &[]);
    a.push("k").op(Op::UNSET_ARRAY, &[1, slot]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"s(k)\": variable isn't array");

    let mut a = Asm::new();
    let slot = a.slot("nope");
    a.push("k").op(Op::UNSET_ARRAY, &[1, slot]);
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't unset \"nope(k)\": no such variable");
}

#[test]
fn unset_array_trace_only_miss_kind_precedes_callback_mutation() {
    // Directly exercise unsetArray: Tcl 9.0.4 retains the missing-element
    // lookup result even when its callback deletes or retypes the parent.
    let cases = [
        "unset -nocomplain ::a",
        "unset -nocomplain ::a; set ::a scalar",
    ];
    for action in cases {
        let mut vm = tcl_vm::test_support::native_vm();
        vm.set_compiler(Box::new(compiler::svc()));
        let setup = format!(
            "set fired 0\n\
             proc U {{action args}} {{incr ::fired; uplevel #0 $action}}\n\
             array set a {{}}\n\
             trace add variable a(missing) unset [list U {{{action}}}]\n"
        );
        let completion = vm.eval_source(&setup).expect("setup compiles");
        assert_eq!(completion.code, Code::Ok, "setup failed for {action}");

        let mut asm = Asm::new();
        let slot = asm.slot("a");
        asm.push("missing").op(Op::UNSET_ARRAY, &[1, slot]);
        let completion = run(&mut vm, asm);
        assert_eq!(
            err_str(&completion),
            "can't unset \"a(missing)\": no such element in array",
            "callback action: {action}"
        );
        assert_eq!(
            err_code(&completion),
            "TCL UNSET VARNAME",
            "callback action: {action}"
        );
        assert_eq!(
            vm.get_var("fired").map(|value| value.to_str().to_string()),
            Some("1".to_owned()),
            "callback action: {action}"
        );
    }
}
