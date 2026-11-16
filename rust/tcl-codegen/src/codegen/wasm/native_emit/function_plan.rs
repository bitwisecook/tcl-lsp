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

//! What is fixed per function before emission: its locals, its largest argv, where each statement's result comes from and whether it may be a procedure entry.

use super::super::ir::ValType;
use super::{FIXED_LOCALS, LOCAL_SCRATCH_OBJ_A, LOCAL_SCRATCH_OBJ_B};
use tcl_compiler::native_lowering::ProcEntryDecline;
use tcl_compiler::native_lowering::ir::{
    NativeFunction, NativeOp, NativeStatement, NativeTerminator, NativeType, NativeValueId,
};

/// Every local an emitted function allocates: the fixed scratch slots, one per
/// NLIR value, and a `(code, result, options)` triple per completion.
pub(super) struct LocalPlan {
    pub(super) types: Vec<ValType>,
    pub(super) names: Vec<String>,
    pub(super) value_locals: Vec<u64>,
    pub(super) owned_locals: Vec<u64>,
    pub(super) completion_base: u64,
}

pub(super) fn function_locals(function: &NativeFunction) -> LocalPlan {
    let mut locals_types: Vec<ValType> = vec![
        ValType::I32, // frame
        ValType::I32, // exit code
        ValType::I32,
        ValType::I32,
        ValType::I64,
        ValType::I64,
        ValType::I64,
        ValType::F64,
        ValType::F64,
        ValType::I32,
        ValType::I32,
    ];
    let mut local_names = vec![
        "$frame".to_owned(),
        "$exit_code".to_owned(),
        "$s32a".to_owned(),
        "$s32b".to_owned(),
        "$s64a".to_owned(),
        "$s64b".to_owned(),
        "$s64c".to_owned(),
        "$f64a".to_owned(),
        "$f64b".to_owned(),
        "$obja".to_owned(),
        "$objb".to_owned(),
    ];
    let mut value_locals = Vec::with_capacity(function.values.len());
    let mut owned_locals = vec![LOCAL_SCRATCH_OBJ_A, LOCAL_SCRATCH_OBJ_B];
    for (index, value) in function.values.iter().enumerate() {
        let local = FIXED_LOCALS + u64::try_from(index).unwrap_or(u64::MAX);
        value_locals.push(local);
        locals_types.push(match value.ty {
            NativeType::I64 => ValType::I64,
            NativeType::F64 => ValType::F64,
            NativeType::Bool | NativeType::Obj => ValType::I32,
        });
        local_names.push(format!("$v{index}"));
        if value.ty == NativeType::Obj {
            owned_locals.push(local);
        }
    }
    let completion_base = FIXED_LOCALS + u64::try_from(function.values.len()).unwrap_or(u64::MAX);
    for index in 0..function.completion_count {
        locals_types.extend([ValType::I32; 3]);
        local_names.push(format!("$c{index}_code"));
        local_names.push(format!("$c{index}_result"));
        local_names.push(format!("$c{index}_options"));
        let base = completion_base + 3 * u64::try_from(index).unwrap_or(u64::MAX);
        owned_locals.push(base + 1);
        owned_locals.push(base + 2);
    }
    LocalPlan {
        types: locals_types,
        names: local_names,
        value_locals,
        owned_locals,
        completion_base,
    }
}

/// Whether a lowered body may be bound as a procedure's native entry, and the
/// reason when it may not.
///
/// A proc entry's completion **is** the procedure's answer, so every
/// completion the body returns on its own normal edge has to carry the Tcl
/// result of the command that produced it. Most operations do — a generic
/// invocation adopts the runtime's whole triple, an evaluated source rung
/// leaves the runtime's own result, a fixed completion is explicit — and the
/// two that do not, `set` and `incr`, the emitter materialises from the value
/// they stored ([`Emitter::materialise_result`]). What is left is
/// `append`/`lappend`, whose new cell value the runtime does not hand back,
/// and a structured region (`if` with no `else`, say), whose completion the
/// executable IR produces with no result at all. Binding one of those would
/// answer with the empty string where Tcl answers with a value, so the
/// definition keeps its source body instead.
pub(in super::super) fn proc_entry_decline(function: &NativeFunction) -> Option<ProcEntryDecline> {
    for block in &function.blocks {
        let NativeTerminator::Return(completion) = &block.terminator else {
            continue;
        };
        // A `Return` in a block that did not produce the completion is an
        // abrupt edge: the statement abandoned itself, so the runtime's own
        // result (the error message it set) or the completion triple it
        // adopted already stands.
        let Some(last) = block.statements.last() else {
            continue;
        };
        if last.completion != *completion {
            continue;
        }
        if result_source(last).is_none() {
            return Some(ProcEntryDecline::UndeterminedResult);
        }
    }
    None
}

/// Where the Tcl result of a statement's command is once its operations have
/// run to the end, or `None` when the statement determines no result.
pub(super) fn result_source(statement: &NativeStatement) -> Option<ResultSource> {
    match statement.ops.last() {
        // A generic invocation adopts the runtime's whole completion triple;
        // an evaluated source rung and a proc definition each leave the answer
        // as the runtime's own result, which a null completion result adopts.
        Some(
            NativeOp::Invoke { .. } | NativeOp::EvalSource { .. } | NativeOp::DefineProc { .. },
        ) => Some(ResultSource::Completion),
        Some(NativeOp::Complete { result, .. }) => Some(match result {
            Some(_) => ResultSource::Completion,
            // `return`, `break` and `continue` with no value answer with the
            // empty string.
            None => ResultSource::Empty,
        }),
        // `puts` answers with the empty string.
        Some(NativeOp::Puts { .. }) => Some(ResultSource::Empty),
        // `set` and `incr` answer with the value they stored.
        Some(NativeOp::CellWrite { src, .. }) => Some(ResultSource::Value(*src)),
        Some(NativeOp::CellIncr { dst, .. }) => Some(ResultSource::Value(*dst)),
        _ => None,
    }
}

/// Whether any operation in the statement can complete with a Tcl error, and
/// so needs an `errorInfo` frame logged for its site.
///
/// Written as a list of the operations that provably cannot fail — constants,
/// boxing, and arithmetic the representation lattice already proved in range —
/// so an operation added later is logged rather than silently skipped.
pub(super) fn can_fail(ops: &[NativeOp]) -> bool {
    ops.iter().any(|op| match op {
        NativeOp::ConstInt { .. }
        | NativeOp::ConstDouble { .. }
        | NativeOp::ConstBool { .. }
        | NativeOp::ConstStr { .. }
        | NativeOp::Box { .. }
        | NativeOp::Truth { .. }
        | NativeOp::IntToDouble { .. }
        | NativeOp::BoolToInt { .. }
        | NativeOp::IntBinary { .. }
        | NativeOp::IntNeg { .. }
        | NativeOp::IntBitNot { .. }
        | NativeOp::DoubleNeg { .. }
        | NativeOp::Compare { .. }
        | NativeOp::NotBool { .. }
        | NativeOp::Complete { .. } => false,
        NativeOp::IfElse {
            then_ops, else_ops, ..
        } => can_fail(then_ops) || can_fail(else_ops),
        _ => true,
    })
}

/// Where a statement's Tcl result comes from.
#[derive(Clone, Copy)]
pub(super) enum ResultSource {
    /// The completion's own result slot already holds it.
    Completion,
    /// The empty string.
    Empty,
    /// An NLIR value the emitter has to retain into the completion.
    Value(NativeValueId),
}

/// The largest argv the function's runtime calls need.
pub(super) fn argv_slots(function: &NativeFunction) -> usize {
    fn walk(ops: &[NativeOp], largest: &mut usize) {
        for op in ops {
            let count = match op {
                NativeOp::Invoke { argv }
                | NativeOp::NestedInvoke { argv, .. }
                | NativeOp::DefineProc { argv, .. } => argv.len(),
                NativeOp::MathOp { args, .. } | NativeOp::MathFunc { args, .. } => args.len(),
                NativeOp::Concat { parts, .. } => parts.len(),
                NativeOp::CellAppend { values, .. } => values.len(),
                NativeOp::DynamicBinary { .. }
                | NativeOp::DynamicCompare { .. }
                | NativeOp::DynamicUnary { .. } => 2,
                NativeOp::IfElse {
                    then_ops, else_ops, ..
                } => {
                    walk(then_ops, largest);
                    walk(else_ops, largest);
                    0
                }
                _ => 0,
            };
            *largest = (*largest).max(count);
        }
    }
    let mut largest = function.max_argc.max(2);
    for block in &function.blocks {
        for statement in &block.statements {
            walk(&statement.ops, &mut largest);
        }
    }
    largest
}
