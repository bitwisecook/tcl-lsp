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

//! WASM emission from NLIR — the native tier's emitter (plan §7 row P3).
//!
//! The emitter consumes a [`NativeFunction`] and nothing else: no command
//! name, no source span, no compatibility text. It structurises the NLIR
//! control-flow graph into WASM `block`/`loop`/`if` with the dominator-tree
//! algorithm of Ramsey's *Beyond Relooper* (every merge node gets a `block`
//! opened by its immediate dominator, every loop header a `loop`, every other
//! node is emitted inline where its one forward predecessor branches to it),
//! so the executable IR's completion switches become ordinary structured
//! branches.
//!
//! # Values and ownership
//!
//! Every NLIR value is one WASM local of its machine type. A boxed value is an
//! *owned* reference: a definition first releases the local's previous
//! occupant (so a loop never leaks its earlier iterations' values) and the
//! function epilogue releases every boxed local null-safely. Adopting runtime
//! calls receive a freshly retained reference, so a value stays owned by its
//! local for as long as the function runs.
//!
//! # Statements and completions
//!
//! Each statement starts by clearing its completion code and runs inside one
//! `block`; an operation that fails writes the completion (a Tcl error, or
//! the abrupt triple a nested invocation produced) and branches to the end of
//! that block, abandoning the rest of the statement. The block terminator then
//! dispatches the completion exactly as the executable IR prescribed.
//!
//! # Scratch memory
//!
//! One transient call frame per activation, allocated through the runtime,
//! holds the typed-value out slots, one completion triple, and the argv array
//! the generic and operator intrinsics read.

use std::collections::{BTreeMap, HashSet};

use tcl_core_types::Code as CompletionCode;
use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, NATIVE_PROC_STATUS_RAN, WASM32_COMPLETION_CODE_OFFSET,
    WASM32_COMPLETION_OPTIONS_OFFSET, WASM32_COMPLETION_RESULT_OFFSET, WASM32_POINTER_BYTES,
};
use tcl_syntax::expr::{BinOp, UnaryOp};

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use crate::native_lowering::ProcEntryDecline;
use crate::native_lowering::cells::CellPlace;
use crate::native_lowering::ir::{
    CmpOp, CompareKind, DoubleOp, EntryProtocol, IfElseResult, IntOp, NativeBlockId,
    NativeFunction, NativeOp, NativeStatement, NativeTerminator, NativeType, NativeValueId,
};
use crate::native_lowering::representation::int_op;

/// Block type byte for a structured op yielding no value.
const BLOCK_VOID: u8 = 0x40;

/// Frame layout: the typed out slot, the boolean out slot, the completion
/// triple, then the argv array.
const FRAME_SCRATCH_I64: i64 = 0;
const FRAME_SCRATCH_I32: i64 = 8;
const FRAME_COMPLETION: i64 = 12;
const FRAME_ARGV: i64 = 24;
const FRAME_ALIGN: i64 = 8;

/// The runtime imports the native tier calls.
#[derive(Clone, Copy)]
pub(super) struct NativeImports {
    activation_enter: u32,
    activation_leave: u32,
    frame_push: u32,
    frame_pop: u32,
    call_frame_alloc: u32,
    call_frame_free: u32,
    new_owned_string: u32,
    obj_new_string: u32,
    obj_retain: u32,
    obj_release: u32,
    eval_code: u32,
    invoke_argv: u32,
    puts: u32,
    var_get: u32,
    var_get_element: u32,
    var_set: u32,
    var_set_element: u32,
    var_incr: u32,
    var_update: u32,
    word_concat: u32,
    value_new_wide_int: u32,
    value_new_double: u32,
    value_new_bool: u32,
    value_get_wide_int: u32,
    value_get_double: u32,
    value_get_bool: u32,
    value_try_wide_int: u32,
    value_try_double: u32,
    expr_eval: u32,
    mathop: u32,
    mathfunc: u32,
    proc_define_native: u32,
    log_command: u32,
    return_state: u32,
}

/// Declare every import the native tier uses.
pub(super) fn add_native_imports(
    wasm: &mut WasmModule,
    add: &mut impl FnMut(&mut WasmModule, CodegenAbiImportId) -> u32,
) -> NativeImports {
    NativeImports {
        activation_enter: add(wasm, CodegenAbiImportId::ActivationEnter),
        activation_leave: add(wasm, CodegenAbiImportId::ActivationLeave),
        frame_push: add(wasm, CodegenAbiImportId::FramePush),
        frame_pop: add(wasm, CodegenAbiImportId::FramePop),
        call_frame_alloc: add(wasm, CodegenAbiImportId::CallFrameAlloc),
        call_frame_free: add(wasm, CodegenAbiImportId::CallFrameFree),
        new_owned_string: add(wasm, CodegenAbiImportId::NewOwnedString),
        obj_new_string: add(wasm, CodegenAbiImportId::ObjectNewString),
        obj_retain: add(wasm, CodegenAbiImportId::ObjectRetain),
        obj_release: add(wasm, CodegenAbiImportId::ObjectRelease),
        eval_code: add(wasm, CodegenAbiImportId::EvalCode),
        invoke_argv: add(wasm, CodegenAbiImportId::InvokeArgv),
        puts: add(wasm, CodegenAbiImportId::Puts),
        var_get: add(wasm, CodegenAbiImportId::VarGet),
        var_get_element: add(wasm, CodegenAbiImportId::VarGetElement),
        var_set: add(wasm, CodegenAbiImportId::VarSet),
        var_set_element: add(wasm, CodegenAbiImportId::VarSetElement),
        var_incr: add(wasm, CodegenAbiImportId::VarIncr),
        var_update: add(wasm, CodegenAbiImportId::VarUpdate),
        word_concat: add(wasm, CodegenAbiImportId::WordConcat),
        value_new_wide_int: add(wasm, CodegenAbiImportId::ValueNewWideInt),
        value_new_double: add(wasm, CodegenAbiImportId::ValueNewDouble),
        value_new_bool: add(wasm, CodegenAbiImportId::ValueNewBool),
        value_get_wide_int: add(wasm, CodegenAbiImportId::ValueGetWideInt),
        value_get_double: add(wasm, CodegenAbiImportId::ValueGetDouble),
        value_get_bool: add(wasm, CodegenAbiImportId::ValueGetBool),
        value_try_wide_int: add(wasm, CodegenAbiImportId::ValueTryWideInt),
        value_try_double: add(wasm, CodegenAbiImportId::ValueTryDouble),
        expr_eval: add(wasm, CodegenAbiImportId::ExprEval),
        mathop: add(wasm, CodegenAbiImportId::MathOp),
        mathfunc: add(wasm, CodegenAbiImportId::MathFunc),
        proc_define_native: add(wasm, CodegenAbiImportId::ProcDefineNative),
        log_command: add(wasm, CodegenAbiImportId::LogCommand),
        return_state: add(wasm, CodegenAbiImportId::ReturnState),
    }
}

/// The module's window in the runtime's shared function table: which compiled
/// procedure bodies it installed there, and the module global holding the base
/// index `table.grow` returned for the window.
///
/// A `proc` statement whose body this module compiled reads its entry as
/// `base + slot`; every other definition passes `0`, the ABI's "source body
/// only" spelling.
#[derive(Clone, Copy)]
pub(super) struct EntryTable<'a> {
    /// Slot ordinal within the window, per bound procedure.
    pub(super) slots: &'a BTreeMap<String, u32>,
    /// Index of the module global holding the window's base.
    pub(super) base_global: u32,
}

impl EntryTable<'_> {
    fn slot(self, qualified_name: &str) -> Option<u32> {
        self.slots.get(qualified_name).copied()
    }
}

/// The module's constant pool, shared with the legacy emitter.
pub(super) struct ConstantPool<'a> {
    pub(super) data: &'a mut Vec<WasmData>,
    pub(super) offset: &'a mut i64,
}

impl ConstantPool<'_> {
    fn intern(&mut self, text: &str) -> (i64, i64) {
        let offset = *self.offset;
        let len = i64::try_from(text.len()).unwrap_or(i64::MAX);
        self.data.push(WasmData {
            offset,
            data: text.as_bytes().to_vec(),
        });
        *self.offset += len;
        (offset, len)
    }
}

/// One open structured label the emitter can branch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Label {
    /// The function's exit block.
    Exit,
    /// The current statement's abort block.
    Abort,
    /// A `block` whose end is the entry of an NLIR merge node.
    Block(usize),
    /// A `loop` whose start is an NLIR loop header.
    Loop(usize),
    /// A label nothing branches to by NLIR identity (`if`, an inner block).
    Plain,
}

/// The structural facts the relooper needs.
struct Shape {
    rpo_index: Vec<usize>,
    successors: Vec<Vec<usize>>,
    /// Dominator-tree children that are merge nodes, sorted by RPO index
    /// descending, per node.
    merge_children: Vec<Vec<usize>>,
    loop_headers: HashSet<usize>,
    merge_nodes: HashSet<usize>,
}

impl Shape {
    #[allow(clippy::too_many_lines)]
    fn of(function: &NativeFunction) -> Self {
        let count = function.blocks.len();
        let successors: Vec<Vec<usize>> = function
            .blocks
            .iter()
            .map(|block| match &block.terminator {
                NativeTerminator::Goto(target) => vec![target.index()],
                NativeTerminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => vec![then_target.index(), else_target.index()],
                NativeTerminator::CompletionSwitch { cases, default, .. } => {
                    let mut targets: Vec<usize> =
                        cases.iter().map(|(_, target)| target.index()).collect();
                    targets.push(default.index());
                    targets
                }
                NativeTerminator::Return(_) => Vec::new(),
            })
            .collect();
        // Reverse post-order from the entry.
        let entry = function.entry.index();
        let mut state = vec![0u8; count];
        let mut post = Vec::with_capacity(count);
        let mut stack: Vec<(usize, usize)> = vec![(entry, 0)];
        state[entry] = 1;
        while let Some((block, next)) = stack.last_mut() {
            let block = *block;
            if *next < successors[block].len() {
                let target = successors[block][*next];
                *next += 1;
                if state[target] == 0 {
                    state[target] = 1;
                    stack.push((target, 0));
                }
            } else {
                state[block] = 2;
                post.push(block);
                stack.pop();
            }
        }
        post.reverse();
        let order = post;
        let mut rpo_index = vec![usize::MAX; count];
        for (position, block) in order.iter().enumerate() {
            rpo_index[*block] = position;
        }
        let mut predecessors: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (from, targets) in successors.iter().enumerate() {
            if rpo_index[from] == usize::MAX {
                continue;
            }
            let mut seen = HashSet::new();
            for target in targets {
                if seen.insert(*target) {
                    predecessors[*target].push(from);
                }
            }
        }
        // Loop headers and merge nodes.
        let mut loop_headers = HashSet::new();
        let mut merge_nodes = HashSet::new();
        for block in &order {
            let forward = predecessors[*block]
                .iter()
                .filter(|pred| rpo_index[**pred] < rpo_index[*block])
                .count();
            let backward = predecessors[*block].len() - forward;
            if backward > 0 {
                loop_headers.insert(*block);
            }
            if forward > 1 {
                merge_nodes.insert(*block);
            }
        }
        // Immediate dominators (Cooper–Harvey–Kennedy).
        let mut idom = vec![usize::MAX; count];
        idom[entry] = entry;
        let mut changed = true;
        while changed {
            changed = false;
            for block in order.iter().skip(1) {
                let mut new_idom = usize::MAX;
                for pred in &predecessors[*block] {
                    if idom[*pred] == usize::MAX {
                        continue;
                    }
                    new_idom = if new_idom == usize::MAX {
                        *pred
                    } else {
                        intersect(&idom, &rpo_index, *pred, new_idom)
                    };
                }
                if new_idom != usize::MAX && idom[*block] != new_idom {
                    idom[*block] = new_idom;
                    changed = true;
                }
            }
        }
        let mut merge_children: Vec<Vec<usize>> = vec![Vec::new(); count];
        for block in &order {
            if *block == entry || idom[*block] == usize::MAX {
                continue;
            }
            if merge_nodes.contains(block) {
                merge_children[idom[*block]].push(*block);
            }
        }
        for children in &mut merge_children {
            children.sort_by(|a, b| rpo_index[*b].cmp(&rpo_index[*a]));
        }
        Self {
            rpo_index,
            successors,
            merge_children,
            loop_headers,
            merge_nodes,
        }
    }
}

fn intersect(idom: &[usize], rpo: &[usize], mut a: usize, mut b: usize) -> usize {
    while a != b {
        while rpo[a] > rpo[b] {
            a = idom[a];
        }
        while rpo[b] > rpo[a] {
            b = idom[b];
        }
    }
    a
}

/// Parameter slots of a proc entry, which occupy the low local indices and so
/// shift every local the emitter allocates (see [`Emitter::local_base`]).
const PARAM_ARGV: u64 = 0;
const PARAM_ARGC: u64 = 1;
const PARAM_OUT: u64 = 2;
const PROC_ENTRY_PARAMS: u64 = 3;

/// The Tcl completion code an `errorInfo` frame is logged for.
const TCL_ERROR: i64 = 1;

/// The `-level` a plain `return` records: one enclosing boundary consumes it.
const PLAIN_RETURN_LEVEL: i64 = 1;

/// Local slots reserved before the NLIR values.
const LOCAL_FRAME: u64 = 0;
const LOCAL_EXIT_CODE: u64 = 1;
const LOCAL_SCRATCH_I32: u64 = 2;
const LOCAL_SCRATCH_I32_B: u64 = 3;
const LOCAL_SCRATCH_I64_A: u64 = 4;
const LOCAL_SCRATCH_I64_B: u64 = 5;
const LOCAL_SCRATCH_I64_C: u64 = 6;
const LOCAL_SCRATCH_F64_A: u64 = 7;
const LOCAL_SCRATCH_F64_B: u64 = 8;
const LOCAL_SCRATCH_OBJ_A: u64 = 9;
const LOCAL_SCRATCH_OBJ_B: u64 = 10;
const FIXED_LOCALS: u64 = 11;

struct Emitter<'a, 'p> {
    imports: NativeImports,
    function: &'a NativeFunction,
    /// The module's table window, read by a `proc` definition statement.
    table: EntryTable<'a>,
    /// How many parameter slots sit below the emitter's own locals: the one
    /// place the proc-entry signature costs anything, because every local the
    /// emitter names is an index past the parameters.
    local_base: u64,
    pool: ConstantPool<'p>,
    body: Vec<WasmInstruction>,
    labels: Vec<Label>,
    shape: Shape,
    value_locals: Vec<u64>,
    completion_base: u64,
    argv_slots: i64,
    emitted: HashSet<usize>,
    /// Owned boxed locals to release in the epilogue.
    owned_locals: Vec<u64>,
    /// Scratch boxed locals that hold a temporary the current operation
    /// boxed; released at the end of that operation.
    scratch_owned: Vec<u64>,
    /// The completions a `Return` terminator reads. Only these need their Tcl
    /// result materialised in a proc entry; every other completion is
    /// consumed by a completion switch, which reads only the code.
    returned: HashSet<crate::executable_ir::CompletionId>,
}

/// Emit one lowered function in the shape its
/// [`EntryProtocol`](crate::native_lowering::ir::EntryProtocol) prescribes.
///
/// [`EntryProtocol::Script`] is `() -> ()` and opens with the compiled
/// activation the module's entry point holds. [`EntryProtocol::ProcEntry`] is
/// `(argv, argc, out) -> status` and is **prologue-free**: it takes no
/// activation and pushes no Tcl frame, because `Interp::run_proc` has already
/// pushed the procedure's variable frame and `Interp::run_native_body` holds
/// the activation and the `CmdFrame`. Emitting the script prologue here would
/// push a second, nameless frame at the caller's namespace.
pub(super) fn emit_function(
    name: &str,
    kind: &str,
    function: &NativeFunction,
    imports: NativeImports,
    table: EntryTable<'_>,
    pool: ConstantPool<'_>,
) -> WasmFunction {
    let proc_entry = function.protocol == EntryProtocol::ProcEntry;
    let local_base = if proc_entry { PROC_ENTRY_PARAMS } else { 0 };
    let LocalPlan {
        types: locals_types,
        names: local_names,
        value_locals,
        owned_locals,
        completion_base,
    } = function_locals(function);
    let argv_slots = i64::try_from(argv_slots(function)).unwrap_or(i64::MAX);
    let shape = Shape::of(function);
    let mut emitter = Emitter {
        imports,
        function,
        table,
        local_base,
        pool,
        body: Vec::new(),
        labels: Vec::new(),
        shape,
        value_locals,
        completion_base,
        argv_slots,
        emitted: HashSet::new(),
        owned_locals,
        scratch_owned: Vec::new(),
        returned: function
            .blocks
            .iter()
            .filter_map(|block| match &block.terminator {
                NativeTerminator::Return(completion) => Some(*completion),
                _ => None,
            })
            .collect(),
    };
    emitter.emit_body();
    let (params, results, param_names) = if proc_entry {
        (
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
            vec!["$argv".to_owned(), "$argc".to_owned(), "$out".to_owned()],
        )
    } else {
        (Vec::new(), Vec::new(), Vec::new())
    };
    let mut names = param_names;
    names.extend(local_names);
    WasmFunction {
        name: name.to_owned(),
        params,
        results,
        locals: locals_types,
        body: emitter.body,
        local_names: names,
        exported: true,
        source_range: None,
        kind: kind.to_owned(),
    }
}

/// Every local an emitted function allocates: the fixed scratch slots, one per
/// NLIR value, and a `(code, result, options)` triple per completion.
struct LocalPlan {
    types: Vec<ValType>,
    names: Vec<String>,
    value_locals: Vec<u64>,
    owned_locals: Vec<u64>,
    completion_base: u64,
}

fn function_locals(function: &NativeFunction) -> LocalPlan {
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
pub(super) fn proc_entry_decline(function: &NativeFunction) -> Option<ProcEntryDecline> {
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
fn result_source(statement: &NativeStatement) -> Option<ResultSource> {
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
fn can_fail(ops: &[NativeOp]) -> bool {
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
enum ResultSource {
    /// The completion's own result slot already holds it.
    Completion,
    /// The empty string.
    Empty,
    /// An NLIR value the emitter has to retain into the completion.
    Value(NativeValueId),
}

/// The largest argv the function's runtime calls need.
fn argv_slots(function: &NativeFunction) -> usize {
    fn walk(ops: &[NativeOp], largest: &mut usize) {
        for op in ops {
            let count = match op {
                NativeOp::Invoke { argv } | NativeOp::NestedInvoke { argv, .. } => argv.len(),
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

impl Emitter<'_, '_> {
    // -- instruction helpers --------------------------------------------------

    fn push(&mut self, op: WasmOp) {
        self.body.push(WasmInstruction::new(op));
    }

    fn i32(&mut self, value: i64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::I32Const,
            leb128_signed(value),
        ));
    }

    fn i64(&mut self, value: i64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::I64Const,
            leb128_signed(value),
        ));
    }

    fn f64(&mut self, value: f64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::F64Const,
            value.to_le_bytes().to_vec(),
        ));
    }

    fn call(&mut self, index: u32) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Call,
            leb128_unsigned(u64::from(index)),
        ));
    }

    /// Read one of the emitter's own locals. Every local index the emitter
    /// names is relative to its own first slot, so a proc entry's parameters
    /// shift them here rather than at each of the hundred use sites.
    fn get(&mut self, local: u64) {
        let local = local + self.local_base;
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalGet,
            leb128_unsigned(local),
        ));
    }

    fn set(&mut self, local: u64) {
        let local = local + self.local_base;
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalSet,
            leb128_unsigned(local),
        ));
    }

    fn tee(&mut self, local: u64) {
        let local = local + self.local_base;
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalTee,
            leb128_unsigned(local),
        ));
    }

    /// Read one of the function's parameters, which sit below `local_base`.
    fn param(&mut self, index: u64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalGet,
            leb128_unsigned(index),
        ));
    }

    fn memarg(op: WasmOp, align: u64, offset: i64) -> WasmInstruction {
        let mut operands = leb128_unsigned(align);
        operands.extend(leb128_unsigned(u64::try_from(offset).unwrap_or(0)));
        WasmInstruction::with_operands(op, operands)
    }

    fn load_i32(&mut self, offset: i64) {
        self.body.push(Self::memarg(WasmOp::I32Load, 2, offset));
    }

    fn store_i32(&mut self, offset: i64) {
        self.body.push(Self::memarg(WasmOp::I32Store, 2, offset));
    }

    fn load_i64(&mut self, offset: i64) {
        self.body.push(Self::memarg(WasmOp::I64Load, 3, offset));
    }

    fn load_f64(&mut self, offset: i64) {
        self.body.push(Self::memarg(WasmOp::F64Load, 3, offset));
    }

    fn open(&mut self, op: WasmOp, label: Label) {
        self.body
            .push(WasmInstruction::with_operands(op, vec![BLOCK_VOID]));
        self.labels.push(label);
    }

    fn close(&mut self) {
        self.push(WasmOp::End);
        self.labels.pop();
    }

    fn else_(&mut self) {
        self.push(WasmOp::Else);
    }

    fn depth_of(&self, label: Label) -> u64 {
        let position = self
            .labels
            .iter()
            .rposition(|candidate| *candidate == label)
            .expect("a branch target label is open");
        u64::try_from(self.labels.len() - 1 - position).unwrap_or(u64::MAX)
    }

    fn br(&mut self, label: Label) {
        let depth = self.depth_of(label);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Br,
            leb128_unsigned(depth),
        ));
    }

    fn br_if(&mut self, label: Label) {
        let depth = self.depth_of(label);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::BrIf,
            leb128_unsigned(depth),
        ));
    }

    fn text_pair(&mut self, text: &str) {
        let (offset, len) = self.pool.intern(text);
        self.i32(offset);
        self.i32(len);
    }

    fn local_of(&self, value: NativeValueId) -> u64 {
        self.value_locals[value.0 as usize]
    }

    fn ty(&self, value: NativeValueId) -> NativeType {
        self.function.value(value).ty
    }

    fn code_local(&self, completion: crate::executable_ir::CompletionId) -> u64 {
        self.completion_base + 3 * u64::try_from(completion.index()).unwrap_or(u64::MAX)
    }

    /// Store the owned object on the stack into `local`, releasing the
    /// previous occupant.
    fn set_owned(&mut self, local: u64) {
        self.set(LOCAL_SCRATCH_I32);
        self.get(local);
        self.call(self.imports.obj_release);
        self.get(LOCAL_SCRATCH_I32);
        self.set(local);
    }

    fn set_value_owned(&mut self, value: NativeValueId) {
        let local = self.local_of(value);
        self.set_owned(local);
    }

    /// Push a retained reference to the boxed value for an adopting call.
    fn push_retained(&mut self, value: NativeValueId) {
        self.get(self.local_of(value));
        self.call(self.imports.obj_retain);
    }

    /// Push a *borrowed* boxed handle for `value`, boxing a native value into
    /// a scratch local first.
    fn push_boxed(&mut self, value: NativeValueId) {
        match self.ty(value) {
            NativeType::Obj => self.get(self.local_of(value)),
            NativeType::I64 | NativeType::F64 | NativeType::Bool => {
                let scratch = if self.scratch_owned.contains(&LOCAL_SCRATCH_OBJ_A) {
                    LOCAL_SCRATCH_OBJ_B
                } else {
                    LOCAL_SCRATCH_OBJ_A
                };
                self.box_value(value);
                self.set_owned(scratch);
                self.scratch_owned.push(scratch);
                self.get(scratch);
            }
        }
    }

    /// Release the scratch boxes an operation created.
    fn release_scratch(&mut self) {
        for local in std::mem::take(&mut self.scratch_owned) {
            self.get(local);
            self.call(self.imports.obj_release);
            self.i32(0);
            self.set(local);
        }
    }

    /// Box a native value, leaving the owned handle on the stack.
    fn box_value(&mut self, value: NativeValueId) {
        self.get(self.local_of(value));
        match self.ty(value) {
            NativeType::I64 => self.call(self.imports.value_new_wide_int),
            NativeType::F64 => self.call(self.imports.value_new_double),
            NativeType::Bool => self.call(self.imports.value_new_bool),
            NativeType::Obj => self.call(self.imports.obj_retain),
        }
    }

    fn frame_offset(&mut self, offset: i64) {
        self.get(LOCAL_FRAME);
        if offset != 0 {
            self.i32(offset);
            self.push(WasmOp::I32Add);
        }
    }

    /// Store borrowed handles for `values` into the frame's argv array and
    /// leave the array pointer on the stack.
    fn argv(&mut self, values: &[NativeValueId]) {
        for (index, value) in values.iter().enumerate() {
            self.get(LOCAL_FRAME);
            self.push_boxed(*value);
            self.store_i32(
                FRAME_ARGV + i64::try_from(index).unwrap_or(0) * i64::from(WASM32_POINTER_BYTES),
            );
        }
        self.frame_offset(FRAME_ARGV);
    }

    // -- function body --------------------------------------------------------

    fn emit_body(&mut self) {
        // A script entry holds the activation the eval loop would hold for
        // the script it runs. A proc entry holds none: `run_native_body`
        // already took one for this body, and taking a second would halve the
        // recursion depth Tcl allows.
        if self.proc_entry() {
            // The parameters are the reserved seam for P5's native formal
            // binder; a P5-lite body reads its formals as named cells, which
            // `run_proc` bound before the call.
            let _ = (PARAM_ARGV, PARAM_ARGC);
        } else {
            self.call(self.imports.activation_enter);
            self.open(WasmOp::If, Label::Plain);
            self.push(WasmOp::Return);
            self.close();
        }
        self.i32(FRAME_ARGV + self.argv_slots * i64::from(WASM32_POINTER_BYTES));
        self.i32(FRAME_ALIGN);
        self.call(self.imports.call_frame_alloc);
        self.set(LOCAL_FRAME);
        self.open(WasmOp::Block, Label::Exit);
        let entry = self.function.entry.index();
        self.do_tree(entry);
        self.close();
        for local in self.owned_locals.clone() {
            self.get(local);
            self.call(self.imports.obj_release);
        }
        self.get(LOCAL_FRAME);
        self.call(self.imports.call_frame_free);
        self.push(WasmOp::Drop);
        if self.proc_entry() {
            self.i32(i64::from(NATIVE_PROC_STATUS_RAN));
        } else {
            self.get(LOCAL_EXIT_CODE);
            self.call(self.imports.activation_leave);
        }
        self.push(WasmOp::End);
    }

    const fn proc_entry(&self) -> bool {
        matches!(self.function.protocol, EntryProtocol::ProcEntry)
    }

    // -- structurisation ------------------------------------------------------

    fn do_tree(&mut self, block: usize) {
        if !self.emitted.insert(block) {
            // A reducible CFG never asks for a node twice; fail closed by
            // leaving the function with the exit code already set.
            self.br(Label::Exit);
            return;
        }
        let children = self.shape.merge_children[block].clone();
        if self.shape.loop_headers.contains(&block) {
            self.open(WasmOp::Loop, Label::Loop(block));
            self.node_within(block, &children);
            self.close();
        } else {
            self.node_within(block, &children);
        }
    }

    fn node_within(&mut self, block: usize, children: &[usize]) {
        match children.split_first() {
            None => {
                self.emit_block(block);
            }
            Some((child, rest)) => {
                self.open(WasmOp::Block, Label::Block(*child));
                self.node_within(block, rest);
                self.close();
                self.do_tree(*child);
            }
        }
    }

    fn do_branch(&mut self, target: usize) {
        if self.labels.contains(&Label::Loop(target)) {
            self.br(Label::Loop(target));
        } else if self.labels.contains(&Label::Block(target)) {
            self.br(Label::Block(target));
        } else if self.shape.merge_nodes.contains(&target)
            || self.shape.loop_headers.contains(&target)
        {
            // Not reachable by construction (a merge node's block is opened
            // by its dominator); keep the module valid regardless.
            self.do_tree(target);
        } else {
            self.do_tree(target);
        }
    }

    fn emit_block(&mut self, index: usize) {
        let block = &self.function.blocks[index];
        for statement in &block.statements {
            self.emit_statement(statement);
        }
        match &block.terminator {
            NativeTerminator::Goto(target) => self.do_branch(target.index()),
            NativeTerminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                self.get(self.local_of(*condition));
                self.open(WasmOp::If, Label::Plain);
                self.do_branch(then_target.index());
                self.else_();
                self.do_branch(else_target.index());
                self.close();
            }
            NativeTerminator::CompletionSwitch {
                completion,
                cases,
                default,
            } => {
                let code = self.code_local(*completion);
                let cases = cases.clone();
                let default = default.index();
                self.emit_switch(code, &cases, default);
            }
            NativeTerminator::Return(completion) => {
                if self.proc_entry() {
                    self.write_completion_out(*completion);
                } else {
                    let code = self.code_local(*completion);
                    self.get(code);
                    self.set(LOCAL_EXIT_CODE);
                }
                self.br(Label::Exit);
            }
        }
        let _ = self.shape.rpo_index.len();
        let _ = self.shape.successors.len();
    }

    fn emit_switch(&mut self, code: u64, cases: &[(i32, NativeBlockId)], default: usize) {
        match cases.split_first() {
            None => self.do_branch(default),
            Some(((wanted, target), rest)) => {
                self.get(code);
                self.i32(i64::from(*wanted));
                self.push(WasmOp::I32Eq);
                self.open(WasmOp::If, Label::Plain);
                self.do_branch(target.index());
                self.else_();
                self.emit_switch(code, rest, default);
                self.close();
            }
        }
    }

    // -- statements -----------------------------------------------------------

    fn emit_statement(&mut self, statement: &NativeStatement) {
        let code = self.code_local(statement.completion);
        // A proc entry hands the completions a `Return` reads to the runtime,
        // so those — and only those — have to carry the statement's Tcl
        // result. Every other completion is read by a completion switch,
        // which looks at the code alone.
        let escapes = self.proc_entry() && self.returned.contains(&statement.completion);
        let materialise = escapes.then(|| result_source(statement)).flatten();
        self.i32(0);
        self.set(code);
        if escapes {
            // An edge that abandons the statement must report the runtime's
            // own result — the error message the failing operation set — not
            // the one a previous execution of this same statement left here.
            self.i32(0);
            self.set_owned(code + 1);
        }
        self.open(WasmOp::Block, Label::Abort);
        for op in &statement.ops {
            self.emit_op(op, statement.completion);
        }
        // Reached only when every operation ran: an abandoned statement
        // branched past this to the end of the block.
        if let Some(source) = materialise {
            self.materialise_result(statement.completion, source);
        }
        self.close();
        self.log_error_site(statement);
    }

    /// Put the Tcl result of the statement's command in the completion, for
    /// the operations whose runtime call does not leave it anywhere the
    /// proc-entry epilogue can find it.
    ///
    /// `set foo bar` answers with `bar` and `incr n` with the new value, but
    /// `tcl_codegen_var_set`/`tcl_codegen_var_incr` set no interpreter result:
    /// without this a body ending in one would answer with whatever the
    /// runtime's result happened to be. Only a proc entry needs it — a script
    /// entry's completion result is never read.
    fn materialise_result(
        &mut self,
        completion: crate::executable_ir::CompletionId,
        source: ResultSource,
    ) {
        let code = self.code_local(completion);
        match source {
            // Already where the epilogue looks.
            ResultSource::Completion => {}
            ResultSource::Empty => {
                self.text_pair("");
                self.call(self.imports.new_owned_string);
                self.set(code + 1);
            }
            ResultSource::Value(value) => {
                self.push_retained(value);
                self.set(code + 1);
            }
        }
    }

    /// Log the statement's own `errorInfo` frame when it completed with an
    /// error, because a compiled statement reaches no eval loop that would.
    ///
    /// The runtime owns the `already_logged` protocol, so the innermost
    /// statement of a nest logs and the rest are no-ops — the same dedup C
    /// applies within one bytecode frame. Without this a compiled body's
    /// error carries neither a `while executing "<text>"` frame nor the TIP
    /// 348 `CALL` entry, and `errorLine` never advances.
    fn log_error_site(&mut self, statement: &NativeStatement) {
        let Some(site) = statement.site.as_ref() else {
            return;
        };
        if !can_fail(&statement.ops) {
            return;
        }
        let (line, text) = (i64::from(site.line), site.text.clone());
        let code = self.code_local(statement.completion);
        self.get(code);
        self.i32(TCL_ERROR);
        self.push(WasmOp::I32Eq);
        self.open(WasmOp::If, Label::Plain);
        self.i32(line);
        self.text_pair(&text);
        self.call(self.imports.log_command);
        self.close();
    }

    /// Abandon the statement with an error completion.
    fn fail(&mut self, completion: crate::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.i32(1);
        self.set(code);
        self.release_scratch();
        self.br(Label::Abort);
    }

    /// Abandon the statement with the completion triple the frame holds.
    fn fail_with_frame_completion(&mut self, completion: crate::executable_ir::CompletionId) {
        self.adopt_frame_completion(completion);
        self.release_scratch();
        self.br(Label::Abort);
    }

    /// Move the frame's completion triple into the statement's completion
    /// locals.
    fn adopt_frame_completion(&mut self, completion: crate::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.set(code);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.set_owned(code + 1);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.set_owned(code + 2);
    }

    /// Hand the completion triple to the caller through `out` and give up
    /// ownership of it: the runtime takes the reference on each non-null
    /// pointer, so the locals are nulled and the epilogue's release loop
    /// skips them.
    ///
    /// A null result is not an omission: it means the runtime's own current
    /// result is the body's answer, which is what an evaluated source rung
    /// and every error edge leave behind.
    fn write_completion_out(&mut self, completion: crate::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.param(PARAM_OUT);
        self.get(code);
        self.store_i32(i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.param(PARAM_OUT);
        self.get(code + 1);
        self.store_i32(i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.i32(0);
        self.set(code + 1);
        self.param(PARAM_OUT);
        self.get(code + 2);
        self.store_i32(i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.i32(0);
        self.set(code + 2);
    }

    /// After a completion-writing intrinsic: fail on a non-OK code, else move
    /// the result into `dst` and release the options.
    fn take_frame_result(
        &mut self,
        dst: NativeValueId,
        completion: crate::executable_ir::CompletionId,
    ) {
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.open(WasmOp::If, Label::Plain);
        self.fail_with_frame_completion(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.set_value_owned(dst);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(self.imports.obj_release);
        self.release_scratch();
    }

    #[allow(clippy::too_many_lines)]
    fn emit_op(&mut self, op: &NativeOp, completion: crate::executable_ir::CompletionId) {
        match op {
            NativeOp::ConstInt { dst, value } => {
                self.i64(*value);
                self.set(self.local_of(*dst));
            }
            NativeOp::ConstDouble { dst, value } => {
                self.f64(*value);
                self.set(self.local_of(*dst));
            }
            NativeOp::ConstBool { dst, value } => {
                self.i32(i64::from(*value));
                self.set(self.local_of(*dst));
            }
            NativeOp::ConstStr { dst, text } => {
                self.text_pair(text);
                self.call(self.imports.new_owned_string);
                self.set_value_owned(*dst);
            }
            NativeOp::Box { dst, src } => {
                self.box_value(*src);
                self.set_value_owned(*dst);
            }
            NativeOp::Unbox { dst, src, target } => {
                self.get(self.local_of(*src));
                let (import, offset) = match target {
                    NativeType::I64 => (self.imports.value_get_wide_int, FRAME_SCRATCH_I64),
                    NativeType::F64 => (self.imports.value_get_double, FRAME_SCRATCH_I64),
                    NativeType::Bool => (self.imports.value_get_bool, FRAME_SCRATCH_I32),
                    NativeType::Obj => (self.imports.obj_retain, 0),
                };
                if *target == NativeType::Obj {
                    self.call(import);
                    self.set_value_owned(*dst);
                    return;
                }
                self.frame_offset(offset);
                self.call(import);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_FRAME);
                match target {
                    NativeType::I64 => self.load_i64(offset),
                    NativeType::F64 => self.load_f64(offset),
                    _ => self.load_i32(offset),
                }
                self.set(self.local_of(*dst));
            }
            NativeOp::Truth { dst, src } => {
                self.get(self.local_of(*src));
                match self.ty(*src) {
                    NativeType::I64 => {
                        self.push(WasmOp::I64Eqz);
                        self.push(WasmOp::I32Eqz);
                    }
                    NativeType::F64 => {
                        self.f64(0.0);
                        self.push(WasmOp::F64Ne);
                    }
                    NativeType::Bool | NativeType::Obj => {}
                }
                self.set(self.local_of(*dst));
            }
            NativeOp::IntToDouble { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::F64ConvertI64S);
                self.set(self.local_of(*dst));
            }
            NativeOp::BoolToInt { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::I64ExtendI32U);
                self.set(self.local_of(*dst));
            }
            NativeOp::IntBinary { dst, op, lhs, rhs } => {
                let (a, b) = (self.local_of(*lhs), self.local_of(*rhs));
                let d = self.local_of(*dst);
                self.int_binary(*op, a, b, d);
            }
            NativeOp::IntNeg { dst, src } => {
                self.i64(0);
                self.get(self.local_of(*src));
                self.push(WasmOp::I64Sub);
                self.set(self.local_of(*dst));
            }
            NativeOp::IntBitNot { dst, src } => {
                self.get(self.local_of(*src));
                self.i64(-1);
                self.push(WasmOp::I64Xor);
                self.set(self.local_of(*dst));
            }
            NativeOp::DoubleBinary { dst, op, lhs, rhs } => {
                self.get(self.local_of(*lhs));
                self.get(self.local_of(*rhs));
                self.push(match op {
                    DoubleOp::Add => WasmOp::F64Add,
                    DoubleOp::Sub => WasmOp::F64Sub,
                    DoubleOp::Mul => WasmOp::F64Mul,
                    DoubleOp::Div => WasmOp::F64Div,
                });
                self.set(self.local_of(*dst));
            }
            NativeOp::DoubleNeg { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::F64Neg);
                self.set(self.local_of(*dst));
            }
            NativeOp::Compare {
                dst,
                op,
                kind,
                lhs,
                rhs,
            } => {
                self.get(self.local_of(*lhs));
                self.get(self.local_of(*rhs));
                self.push(compare_op(*op, *kind));
                self.set(self.local_of(*dst));
            }
            NativeOp::NotBool { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::I32Eqz);
                self.set(self.local_of(*dst));
            }
            NativeOp::DynamicBinary {
                dst, op, lhs, rhs, ..
            } => self.dynamic_binary(*dst, *op, *lhs, *rhs, completion),
            NativeOp::DynamicCompare {
                dst, op, lhs, rhs, ..
            } => self.dynamic_compare(*dst, *op, *lhs, *rhs, completion),
            NativeOp::DynamicUnary { dst, op, src } => {
                self.dynamic_unary(*dst, *op, *src, completion);
            }
            NativeOp::MathOp { dst, op, args } => {
                self.text_pair(op);
                self.argv(args);
                self.i32(i64::try_from(args.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.mathop);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::MathFunc { dst, name, args } => {
                self.text_pair(name);
                self.argv(args);
                self.i32(i64::try_from(args.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.mathfunc);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::ExprEval { dst, text } => {
                self.text_pair(text);
                self.call(self.imports.new_owned_string);
                self.set_owned(LOCAL_SCRATCH_OBJ_A);
                self.scratch_owned.push(LOCAL_SCRATCH_OBJ_A);
                self.get(LOCAL_SCRATCH_OBJ_A);
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.expr_eval);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::IfElse {
                condition,
                then_ops,
                else_ops,
                result,
            } => {
                self.get(self.local_of(*condition));
                self.open(WasmOp::If, Label::Plain);
                for op in then_ops {
                    self.emit_op(op, completion);
                }
                if let Some(IfElseResult { dst, then_src, .. }) = result {
                    self.copy_value(*then_src, *dst);
                }
                self.else_();
                for op in else_ops {
                    self.emit_op(op, completion);
                }
                if let Some(IfElseResult { dst, else_src, .. }) = result {
                    self.copy_value(*else_src, *dst);
                }
                self.close();
            }
            NativeOp::CellRead { dst, place, .. } => {
                self.cell_name_args(place);
                match place {
                    CellPlace::Named { .. } => self.call(self.imports.var_get),
                    CellPlace::Element { .. } => self.call(self.imports.var_get_element),
                }
                self.tee(LOCAL_SCRATCH_I32);
                self.push(WasmOp::I32Eqz);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_SCRATCH_I32);
                self.set_value_owned(*dst);
            }
            NativeOp::CellWrite { place, src, .. } => {
                self.cell_name_args(place);
                self.push_retained(*src);
                match place {
                    CellPlace::Named { .. } => self.call(self.imports.var_set),
                    CellPlace::Element { .. } => self.call(self.imports.var_set_element),
                }
                self.fail_on_code(completion);
            }
            NativeOp::CellIncr {
                dst, place, delta, ..
            } => {
                self.text_pair(&place.spelling());
                self.push_boxed(*delta);
                self.call(self.imports.var_incr);
                self.tee(LOCAL_SCRATCH_I32);
                self.push(WasmOp::I32Eqz);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_SCRATCH_I32);
                self.set_value_owned(*dst);
                self.release_scratch();
            }
            NativeOp::CellAppend {
                place,
                values,
                list,
                ..
            } => {
                self.text_pair(&place.spelling());
                self.argv(values);
                self.i32(i64::try_from(values.len()).unwrap_or(0));
                self.i32(i64::from(*list));
                self.call(self.imports.var_update);
                self.fail_on_code(completion);
            }
            NativeOp::Concat { dst, parts } => {
                self.argv(parts);
                self.i32(i64::try_from(parts.len()).unwrap_or(0));
                self.call(self.imports.word_concat);
                self.tee(LOCAL_SCRATCH_I32);
                self.push(WasmOp::I32Eqz);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_SCRATCH_I32);
                self.set_value_owned(*dst);
                self.release_scratch();
            }
            NativeOp::Puts { src } => {
                self.push_retained(*src);
                self.call(self.imports.puts);
                self.set(self.code_local(completion));
            }
            NativeOp::Invoke { argv } => {
                self.argv(argv);
                self.i32(i64::try_from(argv.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.invoke_argv);
                self.push(WasmOp::Drop);
                self.adopt_frame_completion(completion);
                self.release_scratch();
            }
            NativeOp::NestedInvoke { dst, argv } => {
                self.argv(argv);
                self.i32(i64::try_from(argv.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.invoke_argv);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::Complete { code, result } => {
                let code_local = self.code_local(completion);
                // A compiled `return` *is* the `return` command, so it records
                // the pending return state that command records for its plain
                // form. Without it the enclosing procedure's return boundary
                // (`Interp::settle_return`) consumes whatever an earlier
                // `return -level N` left behind, and the call propagates code
                // 2 — or a stale requested code — instead of its value. Only
                // `return` reaches this arm: every option-carrying form keeps
                // the generic invocation, which records the state itself, and
                // `settle_return` ignores every completion code but `Return`.
                if *code == CompletionCode::Return {
                    self.i32(PLAIN_RETURN_LEVEL);
                    self.i32(i64::from(CompletionCode::Ok.as_int()));
                    self.call(self.imports.return_state);
                }
                self.i32(code.as_int());
                self.set(code_local);
                if let Some(result) = result {
                    self.push_retained(*result);
                    self.set_owned(code_local + 1);
                }
            }
            NativeOp::DefineProc {
                qualified_name,
                params_raw,
                body_source,
            } => {
                self.text_pair(qualified_name);
                self.text_pair(params_raw);
                self.text_pair(body_source);
                self.push_entry_index(qualified_name);
                self.call(self.imports.proc_define_native);
                self.fail_on_code(completion);
            }
            NativeOp::EvalSource { text, .. } => {
                self.text_pair(text);
                self.call(self.imports.obj_new_string);
                self.call(self.imports.eval_code);
                self.set(self.code_local(completion));
            }
        }
    }

    /// The completion code an adopting runtime call left on the stack: keep
    /// it as the statement's code and abandon the statement when non-zero.
    fn fail_on_code(&mut self, completion: crate::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.tee(code);
        self.open(WasmOp::If, Label::Plain);
        self.release_scratch();
        self.br(Label::Abort);
        self.close();
        self.release_scratch();
    }

    /// The `entry` argument of a proc definition: the module's own table
    /// index for a body it compiled and installed, or `0` — the ABI's
    /// "source body only" — for one it did not.
    fn push_entry_index(&mut self, qualified_name: &str) {
        match self.table.slot(qualified_name) {
            Some(slot) => {
                self.body.push(WasmInstruction::with_operands(
                    WasmOp::GlobalGet,
                    leb128_unsigned(u64::from(self.table.base_global)),
                ));
                self.i32(i64::from(slot));
                self.push(WasmOp::I32Add);
            }
            None => self.i32(0),
        }
    }

    fn cell_name_args(&mut self, place: &CellPlace) {
        match place {
            CellPlace::Named { name } => self.text_pair(name),
            CellPlace::Element { name, key } => {
                self.text_pair(name);
                self.text_pair(key);
            }
        }
    }

    /// `dst = src` for values of one machine type, retaining a boxed value.
    fn copy_value(&mut self, src: NativeValueId, dst: NativeValueId) {
        if self.ty(dst) == NativeType::Obj {
            self.push_retained(src);
            self.set_value_owned(dst);
        } else {
            self.get(self.local_of(src));
            self.set(self.local_of(dst));
        }
    }

    // -- integer arithmetic ---------------------------------------------------

    /// Native integer arithmetic on locals `a`, `b` into `d`, Tcl rounding.
    fn int_binary(&mut self, op: IntOp, a: u64, b: u64, d: u64) {
        match op {
            IntOp::Add
            | IntOp::Sub
            | IntOp::Mul
            | IntOp::And
            | IntOp::Or
            | IntOp::Xor
            | IntOp::Shl
            | IntOp::Shr => {
                self.get(a);
                self.get(b);
                self.push(match op {
                    IntOp::Add => WasmOp::I64Add,
                    IntOp::Sub => WasmOp::I64Sub,
                    IntOp::Mul => WasmOp::I64Mul,
                    IntOp::And => WasmOp::I64And,
                    IntOp::Or => WasmOp::I64Or,
                    IntOp::Xor => WasmOp::I64Xor,
                    IntOp::Shl => WasmOp::I64Shl,
                    _ => WasmOp::I64ShrS,
                });
                self.set(d);
            }
            IntOp::Div => {
                // q = a / b; if (a % b != 0) && ((a % b < 0) != (b < 0)) { q -= 1 }
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64DivS);
                self.set(d);
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64RemS);
                self.set(LOCAL_SCRATCH_I64_C);
                self.floor_adjust_condition(b);
                self.open(WasmOp::If, Label::Plain);
                self.get(d);
                self.i64(1);
                self.push(WasmOp::I64Sub);
                self.set(d);
                self.close();
            }
            IntOp::Mod => {
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64RemS);
                self.tee(LOCAL_SCRATCH_I64_C);
                self.set(d);
                self.floor_adjust_condition(b);
                self.open(WasmOp::If, Label::Plain);
                self.get(d);
                self.get(b);
                self.push(WasmOp::I64Add);
                self.set(d);
                self.close();
            }
        }
    }

    /// Leaves `(r != 0) && ((r < 0) != (b < 0))` on the stack, with the
    /// remainder in scratch `c`.
    fn floor_adjust_condition(&mut self, b: u64) {
        self.get(LOCAL_SCRATCH_I64_C);
        self.i64(0);
        self.push(WasmOp::I64Ne);
        self.get(LOCAL_SCRATCH_I64_C);
        self.i64(0);
        self.push(WasmOp::I64LtS);
        self.get(b);
        self.i64(0);
        self.push(WasmOp::I64LtS);
        self.push(WasmOp::I32Xor);
        self.push(WasmOp::I32And);
    }

    /// Emit the checked native integer operation on scratch `a`/`b` into
    /// scratch `c`, branching to `slow` when the result leaves `i64` or a
    /// Tcl precondition fails.
    #[allow(clippy::too_many_lines)]
    fn checked_int_op(&mut self, op: IntOp, slow: Label) {
        let (a, b, c) = (
            LOCAL_SCRATCH_I64_A,
            LOCAL_SCRATCH_I64_B,
            LOCAL_SCRATCH_I64_C,
        );
        match op {
            IntOp::Add => {
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Add);
                self.set(c);
                // overflow iff ((a ^ r) & (b ^ r)) < 0
                self.get(a);
                self.get(c);
                self.push(WasmOp::I64Xor);
                self.get(b);
                self.get(c);
                self.push(WasmOp::I64Xor);
                self.push(WasmOp::I64And);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
            }
            IntOp::Sub => {
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Sub);
                self.set(c);
                // overflow iff ((a ^ b) & (a ^ r)) < 0
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Xor);
                self.get(a);
                self.get(c);
                self.push(WasmOp::I64Xor);
                self.push(WasmOp::I64And);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
            }
            IntOp::Mul => {
                // The two products that overflow through division itself.
                self.get(a);
                self.i64(-1);
                self.push(WasmOp::I64Eq);
                self.get(b);
                self.i64(i64::MIN);
                self.push(WasmOp::I64Eq);
                self.push(WasmOp::I32And);
                self.br_if(slow);
                self.get(b);
                self.i64(-1);
                self.push(WasmOp::I64Eq);
                self.get(a);
                self.i64(i64::MIN);
                self.push(WasmOp::I64Eq);
                self.push(WasmOp::I32And);
                self.br_if(slow);
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Mul);
                self.set(c);
                // overflow iff a != 0 && r / a != b
                self.get(a);
                self.i64(0);
                self.push(WasmOp::I64Ne);
                self.open(WasmOp::If, Label::Plain);
                self.get(c);
                self.get(a);
                self.push(WasmOp::I64DivS);
                self.get(b);
                self.push(WasmOp::I64Ne);
                self.open(WasmOp::If, Label::Plain);
                self.br(slow);
                self.close();
                self.close();
            }
            IntOp::Div | IntOp::Mod => {
                self.get(b);
                self.push(WasmOp::I64Eqz);
                self.br_if(slow);
                if op == IntOp::Div {
                    self.get(a);
                    self.i64(i64::MIN);
                    self.push(WasmOp::I64Eq);
                    self.get(b);
                    self.i64(-1);
                    self.push(WasmOp::I64Eq);
                    self.push(WasmOp::I32And);
                    self.br_if(slow);
                } else {
                    // rem_s by -1 is defined (zero); nothing to check.
                }
                self.int_binary(op, a, b, c);
            }
            IntOp::And | IntOp::Or | IntOp::Xor => {
                self.int_binary(op, a, b, c);
            }
            IntOp::Shl => {
                self.get(b);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
                self.get(b);
                self.i64(62);
                self.push(WasmOp::I64GtS);
                self.br_if(slow);
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Shl);
                self.set(c);
                self.get(c);
                self.get(b);
                self.push(WasmOp::I64ShrS);
                self.get(a);
                self.push(WasmOp::I64Ne);
                self.br_if(slow);
            }
            IntOp::Shr => {
                self.get(b);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
                self.get(b);
                self.i64(63);
                self.push(WasmOp::I64GtS);
                self.open(WasmOp::If, Label::Plain);
                self.get(a);
                self.i64(63);
                self.push(WasmOp::I64ShrS);
                self.set(c);
                self.else_();
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64ShrS);
                self.set(c);
                self.close();
            }
        }
    }

    /// `dst = lhs op rhs` over operands of any representation: integer fast
    /// path, double fast path, then the runtime operator.
    fn dynamic_binary(
        &mut self,
        dst: NativeValueId,
        op: BinOp,
        lhs: NativeValueId,
        rhs: NativeValueId,
        completion: crate::executable_ir::CompletionId,
    ) {
        let done = Label::Plain;
        self.open(WasmOp::Block, done);
        let done_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // slow
        let slow_depth = self.labels.len() - 1;
        if let Some(iop) = int_op(op) {
            self.open(WasmOp::Block, Label::Plain); // not int
            let notint_depth = self.labels.len() - 1;
            self.load_as_int_at(lhs, LOCAL_SCRATCH_I64_A, notint_depth);
            self.load_as_int_at(rhs, LOCAL_SCRATCH_I64_B, notint_depth);
            self.checked_int_op_at(iop, slow_depth);
            self.get(LOCAL_SCRATCH_I64_C);
            self.call(self.imports.value_new_wide_int);
            self.set_value_owned(dst);
            self.br_depth(done_depth);
            self.close();
            if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div) {
                self.load_as_double_at(lhs, LOCAL_SCRATCH_F64_A, slow_depth);
                self.load_as_double_at(rhs, LOCAL_SCRATCH_F64_B, slow_depth);
                self.get(LOCAL_SCRATCH_F64_A);
                self.get(LOCAL_SCRATCH_F64_B);
                self.push(match op {
                    BinOp::Add => WasmOp::F64Add,
                    BinOp::Sub => WasmOp::F64Sub,
                    BinOp::Mul => WasmOp::F64Mul,
                    _ => WasmOp::F64Div,
                });
                self.call(self.imports.value_new_double);
                self.set_value_owned(dst);
                self.br_depth(done_depth);
            }
        }
        self.close(); // slow
        self.text_pair(op.spec().spelling);
        self.argv(&[lhs, rhs]);
        self.i32(2);
        self.frame_offset(FRAME_COMPLETION);
        self.call(self.imports.mathop);
        self.push(WasmOp::Drop);
        self.take_frame_result(dst, completion);
        self.close(); // done
    }

    fn dynamic_compare(
        &mut self,
        dst: NativeValueId,
        op: BinOp,
        lhs: NativeValueId,
        rhs: NativeValueId,
        completion: crate::executable_ir::CompletionId,
    ) {
        self.open(WasmOp::Block, Label::Plain); // done
        let done_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // slow
        let slow_depth = self.labels.len() - 1;
        if let Some(cmp) = crate::native_lowering::representation::cmp_op(op) {
            self.open(WasmOp::Block, Label::Plain); // not int
            let notint_depth = self.labels.len() - 1;
            self.load_as_int_at(lhs, LOCAL_SCRATCH_I64_A, notint_depth);
            self.load_as_int_at(rhs, LOCAL_SCRATCH_I64_B, notint_depth);
            self.get(LOCAL_SCRATCH_I64_A);
            self.get(LOCAL_SCRATCH_I64_B);
            self.push(compare_op(cmp, CompareKind::I64));
            self.set(self.local_of(dst));
            self.br_depth(done_depth);
            self.close();
            self.load_as_double_at(lhs, LOCAL_SCRATCH_F64_A, slow_depth);
            self.load_as_double_at(rhs, LOCAL_SCRATCH_F64_B, slow_depth);
            self.get(LOCAL_SCRATCH_F64_A);
            self.get(LOCAL_SCRATCH_F64_B);
            self.push(compare_op(cmp, CompareKind::F64));
            self.set(self.local_of(dst));
            self.br_depth(done_depth);
        }
        self.close(); // slow
        self.text_pair(op.spec().spelling);
        self.argv(&[lhs, rhs]);
        self.i32(2);
        self.frame_offset(FRAME_COMPLETION);
        self.call(self.imports.mathop);
        self.push(WasmOp::Drop);
        // The runtime's answer is a boxed 0/1: read it in boolean context.
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.open(WasmOp::If, Label::Plain);
        self.fail_with_frame_completion(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.set_owned(LOCAL_SCRATCH_OBJ_A);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(self.imports.obj_release);
        self.get(LOCAL_SCRATCH_OBJ_A);
        self.frame_offset(FRAME_SCRATCH_I32);
        self.call(self.imports.value_get_bool);
        self.open(WasmOp::If, Label::Plain);
        self.fail(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_SCRATCH_I32);
        self.set(self.local_of(dst));
        self.release_scratch();
        self.close(); // done
    }

    fn dynamic_unary(
        &mut self,
        dst: NativeValueId,
        op: UnaryOp,
        src: NativeValueId,
        completion: crate::executable_ir::CompletionId,
    ) {
        self.open(WasmOp::Block, Label::Plain); // done
        let done_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // slow
        let slow_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // not int
        let notint_depth = self.labels.len() - 1;
        self.load_as_int_at(src, LOCAL_SCRATCH_I64_A, notint_depth);
        match op {
            UnaryOp::Neg => {
                self.get(LOCAL_SCRATCH_I64_A);
                self.i64(i64::MIN);
                self.push(WasmOp::I64Eq);
                self.br_if_depth(slow_depth);
                self.i64(0);
                self.get(LOCAL_SCRATCH_I64_A);
                self.push(WasmOp::I64Sub);
            }
            UnaryOp::Pos => self.get(LOCAL_SCRATCH_I64_A),
            UnaryOp::BitNot => {
                self.get(LOCAL_SCRATCH_I64_A);
                self.i64(-1);
                self.push(WasmOp::I64Xor);
            }
            UnaryOp::Not | UnaryOp::WordNot => self.br_depth(slow_depth),
        }
        if !matches!(op, UnaryOp::Not | UnaryOp::WordNot) {
            self.call(self.imports.value_new_wide_int);
            self.set_value_owned(dst);
            self.br_depth(done_depth);
        }
        self.close(); // not int
        if matches!(op, UnaryOp::Neg | UnaryOp::Pos) {
            self.load_as_double_at(src, LOCAL_SCRATCH_F64_A, slow_depth);
            self.get(LOCAL_SCRATCH_F64_A);
            if op == UnaryOp::Neg {
                self.push(WasmOp::F64Neg);
            }
            self.call(self.imports.value_new_double);
            self.set_value_owned(dst);
            self.br_depth(done_depth);
        }
        self.close(); // slow
        self.text_pair(op.as_str());
        self.argv(&[src]);
        self.i32(1);
        self.frame_offset(FRAME_COMPLETION);
        self.call(self.imports.mathop);
        self.push(WasmOp::Drop);
        self.take_frame_result(dst, completion);
        self.close(); // done
    }

    // Depth-addressed branches for the anonymous blocks of the dynamic ops.

    fn br_depth(&mut self, label_index: usize) {
        let depth = u64::try_from(self.labels.len() - 1 - label_index).unwrap_or(u64::MAX);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Br,
            leb128_unsigned(depth),
        ));
    }

    fn br_if_depth(&mut self, label_index: usize) {
        let depth = u64::try_from(self.labels.len() - 1 - label_index).unwrap_or(u64::MAX);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::BrIf,
            leb128_unsigned(depth),
        ));
    }

    fn load_as_int_at(&mut self, value: NativeValueId, target: u64, slow_index: usize) {
        match self.ty(value) {
            NativeType::F64 => self.br_depth(slow_index),
            NativeType::Obj => {
                self.get(self.local_of(value));
                self.frame_offset(FRAME_SCRATCH_I64);
                self.call(self.imports.value_try_wide_int);
                self.push(WasmOp::I32Eqz);
                self.br_if_depth(slow_index);
                self.get(LOCAL_FRAME);
                self.load_i64(FRAME_SCRATCH_I64);
                self.set(target);
            }
            NativeType::I64 => {
                self.get(self.local_of(value));
                self.set(target);
            }
            NativeType::Bool => {
                self.get(self.local_of(value));
                self.push(WasmOp::I64ExtendI32U);
                self.set(target);
            }
        }
    }

    fn load_as_double_at(&mut self, value: NativeValueId, target: u64, slow_index: usize) {
        match self.ty(value) {
            NativeType::Obj => {
                self.get(self.local_of(value));
                self.frame_offset(FRAME_SCRATCH_I64);
                self.call(self.imports.value_try_double);
                self.push(WasmOp::I32Eqz);
                self.br_if_depth(slow_index);
                self.get(LOCAL_FRAME);
                self.load_f64(FRAME_SCRATCH_I64);
                self.set(target);
            }
            NativeType::I64 | NativeType::Bool | NativeType::F64 => {
                self.get(self.local_of(value));
                if self.ty(value) == NativeType::Bool {
                    self.push(WasmOp::I64ExtendI32U);
                }
                if self.ty(value) != NativeType::F64 {
                    self.push(WasmOp::F64ConvertI64S);
                }
                self.set(target);
            }
        }
        self.get(target);
        self.get(target);
        self.push(WasmOp::F64Ne);
        self.br_if_depth(slow_index);
    }

    fn checked_int_op_at(&mut self, op: IntOp, slow_index: usize) {
        // `checked_int_op` branches through a `Label`; the dynamic ops
        // address their slow block by index, so give it a unique label.
        let marker = Label::Block(usize::MAX - slow_index);
        self.labels[slow_index] = marker;
        self.checked_int_op(op, marker);
        self.labels[slow_index] = Label::Plain;
    }
}

const fn compare_op(op: CmpOp, kind: CompareKind) -> WasmOp {
    match (kind, op) {
        (CompareKind::I64, CmpOp::Eq) => WasmOp::I64Eq,
        (CompareKind::I64, CmpOp::Ne) => WasmOp::I64Ne,
        (CompareKind::I64, CmpOp::Lt) => WasmOp::I64LtS,
        (CompareKind::I64, CmpOp::Le) => WasmOp::I64LeS,
        (CompareKind::I64, CmpOp::Gt) => WasmOp::I64GtS,
        (CompareKind::I64, CmpOp::Ge) => WasmOp::I64GeS,
        (CompareKind::F64, CmpOp::Eq) => WasmOp::F64Eq,
        (CompareKind::F64, CmpOp::Ne) => WasmOp::F64Ne,
        (CompareKind::F64, CmpOp::Lt) => WasmOp::F64Lt,
        (CompareKind::F64, CmpOp::Le) => WasmOp::F64Le,
        (CompareKind::F64, CmpOp::Gt) => WasmOp::F64Gt,
        (CompareKind::F64, CmpOp::Ge) => WasmOp::F64Ge,
    }
}
