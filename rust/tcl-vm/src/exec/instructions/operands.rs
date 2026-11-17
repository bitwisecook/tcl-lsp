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

//! Operand decoding and the stack operators the opcode arms share: immediates, indexes, jump targets, and the binary, comparison and unary operators.

use super::super::Frame;
use crate::expr;
use crate::interp::{Vm, err};
use crate::value::Value;
use tcl_bytecode::{FunctionAsm, INDEX_END, INDEX_END_BAND, Instruction, Operand};
use tcl_runtime_api::Completion;
use tcl_syntax::expr::{BinOp, UnaryOp};

pub(in super::super) fn imm0(instr: &Instruction) -> i32 {
    match instr.operands.first() {
        Some(Operand::Imm(n)) => *n,
        _ => 0,
    }
}

pub(in super::super) fn imm_at(instr: &Instruction, idx: usize) -> i32 {
    match instr.operands.get(idx) {
        Some(Operand::Imm(n)) => *n,
        _ => 0,
    }
}

pub(super) fn label0(instr: &Instruction) -> Option<&str> {
    match instr.operands.first() {
        Some(Operand::Label(s)) => Some(s.as_str()),
        _ => None,
    }
}

pub(in super::super) fn pop(f: &mut Frame) -> Value {
    f.stack.pop().unwrap_or_else(Value::empty)
}

pub(super) fn ilen(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Resolve a bytecode-immediate index (`N`, or `end`-relative encoded relative
/// to `INDEX_END`) against a length.
///
/// `end-N` encodes as `INDEX_END - N` and `end+N` as `INDEX_END + N`
/// (`end_relative_imm`), so an end-relative index occupies a band *around*
/// `INDEX_END`, not just `<= INDEX_END`. Detecting only `<= INDEX_END` would
/// misread `end+N` as a huge negative plain index, so e.g. `lrange $l
/// end+1 0` and `lrange $l 0 end+1` would disagree with the uncompiled command
/// (lrange-5 battery). The band's half-width `INDEX_END_BAND` sits midway
/// between `INDEX_END` (`-2^30`) and 0: any plausible `end±N` lands inside it,
/// while no realistic literal plain index is that far negative.
pub(in super::super) fn imm_index(imm: i32, len: usize) -> isize {
    let n = isize::try_from(len).unwrap_or(isize::MAX);
    if imm <= INDEX_END + INDEX_END_BAND {
        (n - 1) + (imm as isize - INDEX_END as isize)
    } else {
        imm as isize
    }
}

/// Original end-relative immediate coordinates for the portable string owner.
pub(super) fn encoded_string_index(encoded: i32) -> Value {
    if encoded <= INDEX_END + (1 << 29) {
        let offset = i64::from(encoded) - i64::from(INDEX_END);
        Value::string(if offset == 0 {
            "end".to_owned()
        } else {
            format!("end{offset:+}")
        })
    } else {
        Value::int(i64::from(encoded))
    }
}

/// Resolve a runtime index value, erroring on a non-integer spec (`bad index`)
/// — the validating form C's index-taking opcodes all use, since every one of
/// them routes through `TclGetIntForIndexM` and treats its failure as an error
/// (`INST_STR_INDEX`, `INST_STR_RANGE`, `INST_STR_REPLACE`, `INST_LREPLACE4`);
/// the non-validating `unwrap_or` forms it replaced turned a garbage index into
/// a plausible-looking value.
pub(super) fn checked_index_value(
    vm: &mut Vm,
    v: &Value,
    len: usize,
) -> Result<isize, Completion<Value>> {
    let selected = tcl_cmd_core::index::resolve_value(vm, v, len)
        .map_err(|error| crate::command::completion_from_cmd_error(vm, error))?;
    isize::try_from(selected).map_err(|_| {
        crate::command::completion_from_cmd_error(
            vm,
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("VM original index width")
                .into(),
        )
    })
}

/// List element at signed index, or empty when out of range.
pub(super) fn get_at(items: &[Value], i: isize) -> Value {
    usize::try_from(i)
        .ok()
        .and_then(|x| items.get(x))
        .cloned()
        .unwrap_or_else(Value::empty)
}

/// Sublist `[lo..=hi]` clamped to bounds; empty when the range is empty.
pub(in super::super) fn slice(items: &[Value], lo: isize, hi: isize) -> Value {
    let len = isize::try_from(items.len()).unwrap_or(isize::MAX);
    if hi < 0 || lo >= len {
        return Value::list(Vec::new());
    }
    let lo = usize::try_from(lo.max(0)).unwrap_or(0);
    let hi = usize::try_from(hi.min(len - 1)).unwrap_or(0);
    if lo > hi {
        return Value::list(Vec::new());
    }
    Value::list(items[lo..=hi].to_vec())
}

/// Character index of `needle` in `hay` (`string first`/`last`), or -1.
///
/// An **empty needle is a miss**: both C helpers open with an explicit
/// `if (ln == 0) { /* We don't find empty substrings.  Bizarre! */ goto …End; }`
/// that leaves the result at `-1` (`tclStringObj.c:3853-3858` `TclStringFirst`,
/// `:3948-3956` `TclStringLast`). Rust's `str::find`/`rfind` instead *do* match
/// the empty needle, at 0 and at the haystack length respectively, so
/// `INST_STR_FIND`/`INST_STR_FIND_LAST` returned `0`/`len` where C returns `-1`
/// (and where our own `string first`/`last` builtins already returned `-1`).
pub(in super::super) fn char_find(hay: &str, needle: &str, last: bool) -> i64 {
    if needle.is_empty() {
        return -1;
    }
    let byte = if last {
        hay.rfind(needle)
    } else {
        hay.find(needle)
    };
    byte.map_or(-1, |b| {
        i64::try_from(hay[..b].chars().count()).unwrap_or(-1)
    })
}

pub(super) fn label_to_idx(
    asm: &FunctionAsm,
    off2idx: &rustc_hash::FxHashMap<i32, usize>,
    label: &str,
) -> Option<usize> {
    let off = *asm.labels.get(label)?;
    let off_i32 = i32::try_from(off).ok()?;
    Some(
        off2idx
            .get(&off_i32)
            .copied()
            .unwrap_or(asm.instructions.len()),
    )
}

/// Resolve a jump operand (`Operand::Label`) to a target instruction index.
pub(super) fn jump_target(
    asm: &FunctionAsm,
    off2idx: &rustc_hash::FxHashMap<i32, usize>,
    instr: &Instruction,
) -> Option<usize> {
    label_to_idx(asm, off2idx, label0(instr)?)
}

pub(in super::super) fn bin(
    vm: &mut Vm,
    f: &mut Frame,
    op: BinOp,
) -> Result<(), Completion<Value>> {
    let dialect = vm.native_invocation_dialect();
    let b = pop(f);
    let a = pop(f);
    match expr::with_original_numeric_context(vm, |context| expr::arith_in(context, op, &a, &b)) {
        Ok(v) => {
            f.stack.push(v.with_native_double_format(dialect));
            Ok(())
        }
        // Through `completion_from_tcl_error`, not `err`: C stamps an
        // `-errorcode` on the arithmetic failures (`ARITH DIVZERO`,
        // `ARITH DOMAIN`), and a bare `err` would drop it, leaving the
        // compiled `expr {…}` path reporting `NONE` where the dynamic
        // `expr $e` path reports the real code.
        Err(e) => {
            f.record_failed_operands(vec![a, b]);
            Err(crate::command::completion_from_tcl_error(vm, e))
        }
    }
}

pub(in super::super) fn cmp(
    vm: &mut Vm,
    f: &mut Frame,
    op: BinOp,
) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    if let (Some(left), Some(right)) = (
        a.plain_integer_representation(),
        b.plain_integer_representation(),
    ) && vm.execution_refusal.is_none()
        && vm.authored_numeric_context().is_none()
        && vm.native_invocation_dialect().family() == Some(tcl_dialect::model::Family::Tcl)
    {
        let truth = match op {
            BinOp::Eq => Some(left == right),
            BinOp::Ne => Some(left != right),
            BinOp::Lt => Some(left < right),
            BinOp::Le => Some(left <= right),
            BinOp::Gt => Some(left > right),
            BinOp::Ge => Some(left >= right),
            _ => None,
        };
        if let Some(truth) = truth {
            f.stack.push(expr::native_boolean_result(
                vm.numeric_policy_context(),
                truth,
            ));
            return Ok(());
        }
    }
    match expr::with_original_numeric_context(vm, |context| expr::compare_in(context, op, &a, &b)) {
        Ok(t) => {
            f.stack
                .push(expr::native_boolean_result(vm.numeric_policy_context(), t));
            Ok(())
        }
        Err(e) => {
            f.record_failed_operands(vec![a, b]);
            Err(crate::command::completion_from_tcl_error(vm, e))
        }
    }
}

/// Eager (non-short-circuit) logical AND/OR: pop both operands, coerce each
/// to boolean, and push the boolean result.
///
/// C8.4's native compiler normalises the original left operand through a
/// registered `0`/`1`, then emits a short-circuit jump and eager LAND/LOR.
/// Later compiler recipes use branches around both operands instead.
pub(super) fn land_lor(vm: &mut Vm, f: &mut Frame, is_and: bool) -> Result<(), Completion<Value>> {
    let b = pop(f);
    let a = pop(f);
    let value = expr::logical_value_for_vm(vm, a, &b, is_and)
        .map_err(|error| crate::command::completion_from_tcl_error(vm, error))?;
    f.stack.push(value);
    Ok(())
}

pub(super) fn un(vm: &mut Vm, f: &mut Frame, op: UnaryOp) -> Result<(), Completion<Value>> {
    let dialect = vm.native_invocation_dialect();
    let v = pop(f);
    match expr::unary_for_vm(vm, op, &v) {
        Ok(r) => {
            f.stack.push(r.with_native_double_format(dialect));
            Ok(())
        }
        // Keeps C's `-errorcode` (`ARITH DOMAIN <description>` for an
        // operand-type error); a bare `err` would drop it.
        Err(e) => {
            f.record_failed_operands(vec![v]);
            Err(crate::command::completion_from_tcl_error(vm, e))
        }
    }
}

/// Take the top `imm0(instr)` stack words, deepest first — the operand-counted
/// word list an invocation-shaped opcode (`tclooNext`/`tclooNextClass`) consumes.
/// `mnemonic` names the opcode in the underflow error.
pub(in super::super) fn take_words(
    f: &mut Frame,
    instr: &Instruction,
    mnemonic: &str,
) -> Result<Vec<Value>, Completion<Value>> {
    let count = usize::try_from(imm0(instr)).unwrap_or(0);
    if count == 0 || f.stack.len() < count {
        return Err(err(format!("{mnemonic}: stack underflow")));
    }
    Ok(f.stack.split_off(f.stack.len() - count))
}

/// An iRules-dialect binary operator (`IRULE_*`): the operands are on the stack
/// left-then-right, exactly as [`bin`]/[`cmp`] take them, and the semantics come
/// from the shared [`expr::irule_binary`].
pub(super) fn irule(vm: &mut Vm, f: &mut Frame, op: BinOp) -> Result<(), Completion<Value>> {
    let provider = tcl_registry::native_expression_program::authored_f5_string_predicate_provider(
        vm.expression_evaluation_policy().as_ref(),
    );
    let b = pop(f);
    let a = pop(f);
    match expr::irule_binary(provider, op, &a, &b) {
        Ok(v) => {
            f.stack.push(v);
            Ok(())
        }
        Err(e) => Err(crate::command::completion_from_tcl_error(vm, e)),
    }
}

/// `INST_NUM_TYPE` over the release's numeral grammar. C9 reports the raw
/// `TCL_NUMBER_*` (int 2, big 3, double 4, nan 5). C8.6 refines it for
/// `string is`: a long is `TCL_NUMBER_LONG` (1) only within `±UINT_MAX`,
/// else `TCL_NUMBER_WIDE` (2); a bignum within `±UWIDE_MAX` is wide too.
pub(super) fn native_numeric_type(text: &str, version: tcl_dialect::TclVersion) -> i64 {
    use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};
    let parsed = parse_whole_with(text.trim(), ParseFlags::for_syntax(version.number_syntax()));
    let refined = version < tcl_dialect::TclVersion::V9_0;
    match parsed {
        Some(Number::Int(value)) if refined => {
            if u32::try_from(value.unsigned_abs()).is_ok() {
                1
            } else {
                2
            }
        }
        Some(Number::Int(_)) => 2,
        Some(Number::Big { radix, digits, .. }) => {
            let wide = refined
                && digits
                    .chars()
                    .try_fold(0_u128, |acc, digit| {
                        let value = u128::from(digit.to_digit(radix as u32)?);
                        acc.checked_mul(radix as u128)?.checked_add(value)
                    })
                    .is_some_and(|magnitude| magnitude <= u128::from(u64::MAX));
            if wide { 2 } else { 3 }
        }
        Some(Number::Double(_)) => 4,
        Some(Number::Nan { .. }) => 5,
        None => 0,
    }
}
