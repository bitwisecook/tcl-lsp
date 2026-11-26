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

//! Load, store, `incr`, `append` and `lappend` opcodes in their stack, array and list forms.

use super::*;

// `loadScalarStk` / `storeScalarStk`.

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
    let (_vm, c) = run_fresh(a);
    assert_eq!(err_str(&c), "can't read \"nope\": no such variable");
}

// `loadArray4` / `storeArray4`.

/// `storeArray4`/`loadArray4` are the 4-byte-slot forms of the `*Array1` pair:
/// the array is the LVT slot, the element key is on the stack.
#[test]
fn load_store_array4_round_trip() {
    let mut a = Asm::new();
    let slot = a.slot("arr");
    a.push("k").push("v1").op(Op::STORE_ARRAY4, &[slot]);
    a.op(Op::POP, &[]).push("k").op(Op::LOAD_ARRAY4, &[slot]);
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(
        err_str(&c),
        "can't read \"arr(other)\": no such element in array"
    );
}

// The incr family.

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
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "10");
}

/// A non-integer increment reports the canonical coercion error.
#[test]
fn incr_scalar_stk_bad_amount_errors() {
    let mut a = Asm::new();
    a.push("n").push("abc").op(Op::INCR_SCALAR_STK, &[]);
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "3");
}

// `appendStk` / `lappendStk` / `appendArrayStk` / `lappendArrayStk`.

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
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "{p q}");
}

// The lappendList family.

/// `lappendListStk` appends *each* element of the popped list (`lappend v
/// {*}$list`), unlike `lappendStk`.
#[test]
fn lappend_list_stk_appends_each_element() {
    let mut a = Asm::new();
    a.push("l").push("a").op(Op::LAPPEND_STK, &[]);
    a.op(Op::POP, &[]);
    a.push("l").push("b c").op(Op::LAPPEND_LIST_STK, &[]);
    let (_vm, c) = run_fresh(a);
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
    let (_vm, c) = run_fresh(a);
    assert_eq!(ok_str(&c), "a b c d");
}

fn record_lappend_trace(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let op = args.last().unwrap().to_str();
    let op = match op.as_ref() {
        "r" => "read",
        "w" => "write",
        op => op,
    };
    let seen = vm.get_var("::seen").unwrap().to_str();
    vm.set_var("::seen", Value::string(format!("{seen}{op} ")))
        .unwrap();
    if op == "read" {
        let action = vm.get_var("::action").unwrap().to_str();
        let name = if args[1].to_str().is_empty() {
            "x"
        } else {
            "a"
        };
        if action.as_ref() == "error" {
            return Completion::new(Code::Error, Value::string("READ_FAILURE"), Value::empty());
        }
        if action.as_ref() == "retarget" {
            let result = vm.invoke_command(
                "upvar",
                &[Value::string("0"), Value::string("new"), Value::string("x")],
            );
            assert!(result.code.is_ok(), "{result:?}");
        }
        if action.as_ref() == "unset" || action.as_ref() == "recreate" {
            vm.unset_var(name);
        }
        if action.as_ref() == "recreate" {
            let name = if name == "a" { "a(k)" } else { "x" };
            vm.set_var(name, Value::string("REPLACED")).unwrap();
        }
    }
    Completion::new(Code::Ok, Value::empty(), Value::empty())
}

#[test]
fn lappend_list_retains_the_original_cell_when_read_trace_retargets_alias() {
    let mut vm = tcl_vm::test_support::native_vm();
    vm.set_compiler(Box::new(compiler::svc()));
    vm.register("record", record_lappend_trace);
    for (name, value) in [
        ("old", "OLD"),
        ("new", "NEW"),
        ("seen", ""),
        ("action", "retarget"),
    ] {
        vm.set_var(name, Value::string(value)).unwrap();
    }
    let link = vm.invoke_command(
        "upvar",
        &[Value::string("0"), Value::string("old"), Value::string("x")],
    );
    assert!(link.code.is_ok(), "{link:?}");
    let trace = vm.invoke_command(
        "trace",
        &[
            Value::string("add"),
            Value::string("variable"),
            Value::string("old"),
            Value::string("read write"),
            Value::string("record"),
        ],
    );
    assert!(trace.code.is_ok(), "{trace:?}");
    let mut asm = Asm::new();
    asm.push("x").push("Z").op(Op::LAPPEND_LIST_STK, &[]);
    assert_eq!(ok_str(&run(&mut vm, asm)), "OLD Z");
    assert_eq!(vm.get_var("old").unwrap().to_str().as_ref(), "OLD Z");
    assert_eq!(vm.get_var("x").unwrap().to_str().as_ref(), "NEW");
    assert_eq!(vm.get_var("seen").unwrap().to_str().as_ref(), "read write ");
}

fn empty_lappend_asm(op: Op) -> Asm {
    let mut asm = Asm::new();
    let slot = asm.slot(
        if matches!(op, Op::LAPPEND_LIST_ARRAY | Op::LAPPEND_LIST_ARRAY_STK) {
            "a"
        } else {
            "x"
        },
    );
    match op {
        Op::LAPPEND_LIST => {
            asm.push("").op(op, &[slot]);
        }
        Op::LAPPEND_LIST_STK => {
            asm.push("x").push("").op(op, &[]);
        }
        Op::LAPPEND_LIST_ARRAY => {
            asm.push("k").push("").op(op, &[slot]);
        }
        Op::LAPPEND_LIST_ARRAY_STK => {
            asm.push("a").push("k").push("").op(op, &[]);
        }
        _ => unreachable!(),
    }
    asm
}

#[test]
fn empty_lappend_list_forms_preserve_read_only_values_and_captured_cells() {
    let profiles = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];
    let ops = [
        Op::LAPPEND_LIST,
        Op::LAPPEND_LIST_STK,
        Op::LAPPEND_LIST_ARRAY,
        Op::LAPPEND_LIST_ARRAY_STK,
    ];
    for profile in profiles {
        for op in ops {
            for initial in [None, Some(" A  B "), Some("{")] {
                for action in ["", "unset", "recreate", "error"] {
                    check_empty_lappend(profile, op, initial, action);
                }
            }
        }
    }
}

fn check_empty_lappend(profile: &str, op: Op, initial: Option<&str>, action: &str) {
    let mut vm = tcl_vm::test_support::native_vm_for_profile(
        tcl_dialect::DialectProfile::find(profile).unwrap(),
        Box::new(std::io::stdout()),
    );
    vm.set_compiler(Box::new(compiler::svc()));
    vm.register("record", record_lappend_trace);
    vm.set_var("seen", Value::empty()).unwrap();
    vm.set_var("action", Value::string(action)).unwrap();
    let array = matches!(op, Op::LAPPEND_LIST_ARRAY | Op::LAPPEND_LIST_ARRAY_STK);
    let name = if array { "a(k)" } else { "x" };
    if let Some(initial) = initial {
        vm.set_var(name, Value::string(initial)).unwrap();
    }
    let words = if profile == "tcl8.4" {
        vec!["variable", name, "rw", "record"]
    } else {
        vec!["add", "variable", name, "read write", "record"]
    };
    let trace = vm.invoke_command(
        "trace",
        &words.into_iter().map(Value::string).collect::<Vec<_>>(),
    );
    assert!(trace.code.is_ok(), "{profile} {op:?}: {trace:?}");
    let result = run(&mut vm, empty_lappend_asm(op));
    if array && matches!(action, "unset" | "recreate") {
        assert_eq!(
            err_str(&result),
            "can't set \"a(k)\": upvar refers to element in deleted array"
        );
    } else if initial == Some("{") && action.is_empty() {
        assert_eq!(err_str(&result), "unmatched open brace in list");
    } else {
        let expected = if action == "recreate" {
            "REPLACED"
        } else if matches!(action, "unset" | "error") {
            ""
        } else {
            initial.unwrap_or("")
        };
        assert_eq!(ok_str(&result), expected, "{profile} {op:?} {action}");
    }
    let expected = if (initial.is_none() && action.is_empty()) || action == "error" {
        "read write "
    } else {
        "read "
    };
    assert_eq!(
        vm.get_var("seen").unwrap().to_str().as_ref(),
        expected,
        "{profile} {op:?} {initial:?} {action}"
    );
}
