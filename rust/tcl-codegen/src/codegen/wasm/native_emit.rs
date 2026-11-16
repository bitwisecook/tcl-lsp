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

//! WASM emission from NLIR — the native tier's emitter (see `wasm-native-lowering-plan.md` §3.3).
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

use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, NATIVE_PROC_STATUS_HOST_REFUSED, NATIVE_PROC_STATUS_RAN,
    WASM32_POINTER_BYTES,
};

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use tcl_compiler::native_lowering::ir::{
    EntryProtocol, NativeBlockId, NativeFunction, NativeTerminator, NativeType, NativeValueId,
};

mod arithmetic;
mod function_plan;
mod operations;
mod shape;

pub(super) use function_plan::proc_entry_decline;
use function_plan::{LocalPlan, argv_slots, function_locals};
use shape::Shape;

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
    native_body_entry_status: u32,
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
    try_cached_incr: u32,
    var_update: u32,
    word_concat: u32,
    value_new_wide_int: u32,
    value_new_double: u32,
    value_new_bool: u32,
    value_get_wide_int: u32,
    value_get_double: u32,
    /// Primitive getter used only for the runtime comparison's canonical 0/1.
    value_get_bool: u32,
    value_get_bool_for_purpose: u32,
    value_get_expression_bool: u32,
    value_try_wide_int: u32,
    value_try_double: u32,
    expr_eval: u32,
    expr_bool_eval: u32,
    mathop: u32,
    mathfunc: u32,
    proc_define_original_argv: u32,
    proc_define_guarded_argv: u32,
    log_command: u32,
    return_state: u32,
    host_refusal_pending: u32,
}

/// Declare every import the native tier uses.
pub(super) fn add_native_imports(
    wasm: &mut WasmModule,
    add: &mut impl FnMut(&mut WasmModule, CodegenAbiImportId) -> u32,
) -> NativeImports {
    NativeImports {
        activation_enter: add(wasm, CodegenAbiImportId::ActivationEnter),
        native_body_entry_status: add(wasm, CodegenAbiImportId::NativeBodyEntryStatus),
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
        try_cached_incr: add(wasm, CodegenAbiImportId::TryCachedIncr),
        var_update: add(wasm, CodegenAbiImportId::VarUpdate),
        word_concat: add(wasm, CodegenAbiImportId::WordConcat),
        value_new_wide_int: add(wasm, CodegenAbiImportId::ValueNewWideInt),
        value_new_double: add(wasm, CodegenAbiImportId::ValueNewDouble),
        value_new_bool: add(wasm, CodegenAbiImportId::ValueNewBool),
        value_get_wide_int: add(wasm, CodegenAbiImportId::ValueGetWideInt),
        value_get_double: add(wasm, CodegenAbiImportId::ValueGetDouble),
        value_get_bool: add(wasm, CodegenAbiImportId::ValueGetBool),
        value_get_bool_for_purpose: add(wasm, CodegenAbiImportId::ValueGetBoolForPurpose),
        value_get_expression_bool: add(wasm, CodegenAbiImportId::ValueGetExpressionBool),
        value_try_wide_int: add(wasm, CodegenAbiImportId::ValueTryWideInt),
        value_try_double: add(wasm, CodegenAbiImportId::ValueTryDouble),
        expr_eval: add(wasm, CodegenAbiImportId::ExprEval),
        expr_bool_eval: add(wasm, CodegenAbiImportId::ExprBoolEval),
        mathop: add(wasm, CodegenAbiImportId::MathOp),
        mathfunc: add(wasm, CodegenAbiImportId::MathFunc),
        proc_define_original_argv: add(wasm, CodegenAbiImportId::ProcDefineOriginalArgv),
        proc_define_guarded_argv: add(wasm, CodegenAbiImportId::ProcDefineGuardedArgv),
        log_command: add(wasm, CodegenAbiImportId::LogCommand),
        return_state: add(wasm, CodegenAbiImportId::ReturnState),
        host_refusal_pending: add(wasm, CodegenAbiImportId::HostRefusalPending),
    }
}

impl NativeImports {
    /// Only transport imports can omit the check. Unlisted imports remain
    /// checked, including future operation imports and compiled proc calls.
    fn requires_host_refusal_check(self, index: u32) -> bool {
        use CodegenAbiImportId as Id;
        let transport = [
            (self.activation_enter, Id::ActivationEnter),
            (self.native_body_entry_status, Id::NativeBodyEntryStatus),
            (self.activation_leave, Id::ActivationLeave),
            (self.frame_push, Id::FramePush),
            (self.frame_pop, Id::FramePop),
            (self.call_frame_alloc, Id::CallFrameAlloc),
            (self.call_frame_free, Id::CallFrameFree),
            (self.new_owned_string, Id::NewOwnedString),
            (self.obj_new_string, Id::ObjectNewString),
            (self.obj_retain, Id::ObjectRetain),
            (self.obj_release, Id::ObjectRelease),
            (self.value_new_wide_int, Id::ValueNewWideInt),
            (self.value_new_double, Id::ValueNewDouble),
            (self.value_new_bool, Id::ValueNewBool),
            (self.log_command, Id::LogCommand),
            (self.return_state, Id::ReturnState),
            (self.host_refusal_pending, Id::HostRefusalPending),
        ];
        transport
            .into_iter()
            .find_map(|(candidate, id)| (candidate == index).then_some(id))
            .is_none_or(CodegenAbiImportId::requires_host_refusal_check)
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
    pub(super) slots: &'a BTreeMap<tcl_compiler::ir::ProcedureId, u32>,
    /// Index of the module global holding the window's base.
    pub(super) base_global: u32,
}

impl EntryTable<'_> {
    fn slot(self, id: &tcl_compiler::ir::ProcedureId) -> Option<u32> {
        self.slots.get(id).copied()
    }
}

/// The module's constant pool, shared with the structured emitter.
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
    returned: HashSet<tcl_compiler::executable_ir::CompletionId>,
}

/// Emit one lowered function in the shape its
/// [`EntryProtocol`](tcl_compiler::native_lowering::ir::EntryProtocol) prescribes.
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

impl Emitter<'_, '_> {
    // Instruction helpers.

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
        self.call_unchecked(index);
        if self.labels.contains(&Label::Exit) && self.imports.requires_host_refusal_check(index) {
            // A reached host refusal is not a Tcl completion. Branch before
            // adopting out slots or dispatching catch/try, then run the same
            // null-safe ownership cleanup as every other function exit.
            self.call_unchecked(self.imports.host_refusal_pending);
            self.open(WasmOp::If, Label::Plain);
            self.br(Label::Exit);
            self.close();
        }
    }

    fn call_unchecked(&mut self, index: u32) {
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

    fn code_local(&self, completion: tcl_compiler::executable_ir::CompletionId) -> u64 {
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
                let scratch = self.box_scratch(value);
                self.get(scratch);
            }
        }
    }

    fn box_scratch(&mut self, value: NativeValueId) -> u64 {
        let scratch = if self.scratch_owned.contains(&LOCAL_SCRATCH_OBJ_A) {
            LOCAL_SCRATCH_OBJ_B
        } else {
            LOCAL_SCRATCH_OBJ_A
        };
        self.box_value(value);
        self.set_owned(scratch);
        self.scratch_owned.push(scratch);
        scratch
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

    fn try_cached_increment(
        &mut self,
        place: &tcl_compiler::native_lowering::cells::CellPlace,
        delta: NativeValueId,
        candidate: &tcl_compiler::native_lowering::ir::CachedIntegerIncrement,
    ) {
        let boxed_delta = self.box_scratch(delta);
        self.text_pair(&place.spelling());
        self.get(self.local_of(candidate.receiver));
        self.get(boxed_delta);
        self.get(self.local_of(candidate.old));
        self.get(self.local_of(candidate.sum));
        self.frame_offset(FRAME_SCRATCH_I32);
        self.call(self.imports.try_cached_incr);
        self.set(LOCAL_SCRATCH_I32);
        self.get(LOCAL_SCRATCH_I32);
        self.push(WasmOp::I32Eqz);
        self.open(WasmOp::If, Label::Plain);
        self.text_pair(&place.spelling());
        self.get(boxed_delta);
        self.call(self.imports.var_incr);
        self.set(LOCAL_SCRATCH_I32);
        self.else_();
        self.get(LOCAL_SCRATCH_I32);
        self.i32(1);
        self.push(WasmOp::I32Eq);
        self.open(WasmOp::If, Label::Plain);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_SCRATCH_I32);
        self.set(LOCAL_SCRATCH_I32);
        self.else_();
        // Host refusal is handled by `call`; invalid transport has no output.
        self.i32(0);
        self.set(LOCAL_SCRATCH_I32);
        self.close();
        self.close();
        self.get(LOCAL_SCRATCH_I32);
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

    // Function body.

    fn emit_body(&mut self) {
        // A script entry holds the activation the eval loop would hold for
        // the script it runs. A proc entry holds none: `run_native_body`
        // already took one for this body, and taking a second would halve the
        // recursion depth Tcl allows.
        if self.proc_entry() {
            // Decline before allocating a frame or evaluating any body word.
            // The existing caller then runs its ordinary retained source body.
            self.call_unchecked(self.imports.native_body_entry_status);
            self.tee(LOCAL_EXIT_CODE);
            self.open(WasmOp::If, Label::Plain);
            self.get(LOCAL_EXIT_CODE);
            self.push(WasmOp::Return);
            self.close();
            // The body reads its formals from named cells bound by `run_proc`.
            // The argument parameters remain unused.
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
            self.i32(i64::from(NATIVE_PROC_STATUS_HOST_REFUSED));
            self.i32(i64::from(NATIVE_PROC_STATUS_RAN));
            self.call_unchecked(self.imports.host_refusal_pending);
            self.push(WasmOp::Select);
        } else {
            self.get(LOCAL_EXIT_CODE);
            self.call(self.imports.activation_leave);
        }
        self.push(WasmOp::End);
    }

    const fn proc_entry(&self) -> bool {
        matches!(self.function.protocol, EntryProtocol::ProcEntry)
    }

    // Structurisation.

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
}

#[cfg(test)]
mod original_boolean_transport_tests;
