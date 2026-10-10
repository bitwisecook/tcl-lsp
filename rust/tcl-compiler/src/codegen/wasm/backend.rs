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

//! The sole WASM module emitter behind the canonical compilation pipeline.
//!
//! Backend selection changes the typed input plan, not the emitter. A selected
//! prebuilt-argv semantic invocation and general structured Tcl lowering both
//! enter [`emit_wasm`], share one module-construction implementation, and
//! target the same runtime ABI.
//!
//! The analysis-aware tier emits binding-proven assignments, procedure calls,
//! arithmetic, and registry-selected commands over owned `TclObj` pointers.
//! Anything outside that tier is boxed from its CST-derived source span and
//! evaluated by the runtime (`tcl_eval_code`); control flow
//! is **structured** WASM (`if`/`else`; `block`/`loop` with `br`/`br_if` for
//! loops + `break`/`continue`/`return`), and the code a leaf command returns is
//! honoured — an `error`/`return` unwinds, a `break`/`continue` re-enters the
//! loop. This produces a *structurally valid* module
//! (validated with `wasmtime compile`) against the `"tcl"` import ABI the WASM
//! runtime provides (values are i32 `*mut TclObj` pointers into shared linear
//! memory). The runtime ABI is defined by the owning evaluation and completion
//! exports in `runtime/rust/src/codegen_abi.rs` and `codegen_native.rs`;
//! an emitted module runs against it through the
//! shared-memory dynamic link (`__memory_base` relocation), which the standalone
//! `wasm_execute` test exercises with a stub provider.

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{
    GlobalInit, ValType, WasmData, WasmFunction, WasmGlobal, WasmInstruction, WasmModule, WasmOp,
};
use std::collections::{BTreeMap, HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, SemanticOperationId, TclType};
use tcl_syntax::expr::{BinOp, ExprNode};

use crate::codegen::emit::Emit;
use crate::codegen::structured;
use crate::command_binding::{
    Binding, BindingKind, CommandBinding, ModuleCommandMutations, analyse_command_binding,
};
use crate::common_aot_plan::{
    original_direct_expression_body_operations, semantic_operation_binding_is_trusted,
};
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Module, Procedure, Statement};
use crate::mixed_region_plan::GuardedSelectionEvidence;
use crate::native_lowering::cells::{CellPlace, cell_place};
use crate::native_lowering::{
    FunctionDecline, FunctionReport, LoweringInput, NativeBinding, NativeTierReport,
    ProcEntryDecline, lower_function,
};
use crate::registry_invocation::{
    InvocationMetadataContext, RegistryInvocationResolution,
    resolve_command_tokens_with_metadata_context,
};
use crate::semantic_optimisation::SemanticOptimisationConfig;
use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, CodegenAbiValueType, WASM32_CODEGEN_DATA_START, WASM32_COMPLETION_ALIGN,
    WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET, WASM32_COMPLETION_SIZE, WASM32_POINTER_BYTES,
};

use super::leaf_invoke::{
    WasmInvokeNode, WasmLeafInvokeDecline, WasmLeafInvokePlan, WasmWordPlan, select_leaf_invocation,
};
use super::native_emit::{self, ConstantPool, NativeImports};
use super::pipeline::{WasmCompileOptions, WasmNativeI64AddSelection};
use super::semantic_plan::WasmGenericInvokePlan;
use crate::backend_registry::SelectionFacts;

/// Block type byte for a structured op (`block`/`loop`/`if`) yielding no value.
const BLOCK_VOID: u8 = 0x40;

const SEMANTIC_FRAME_LOCAL: u64 = 0;
const SEMANTIC_COMPLETION_LOCAL: u64 = 1;
const SEMANTIC_CODE_LOCAL: u64 = 2;
const SEMANTIC_RESULT_LOCAL: u64 = 3;
const SEMANTIC_OPTIONS_LOCAL: u64 = 4;
const GUARDED_TOKEN_LOCAL: u64 = 5;
const GUARDED_STATUS_LOCAL: u64 = 6;
const SEMANTIC_WORD_LOCAL_START: usize = 7;

fn semantic_word_local(index: usize) -> u64 {
    u64::try_from(SEMANTIC_WORD_LOCAL_START + index).expect("word local fits u64")
}

/// The completion-code scratch local (index 0). Every emitted function declares
/// exactly one `i32` local so [`WasmEmitter::emit_completion_dispatch`] can stash
/// the code a leaf command returns and test it more than once.
/// Tcl completion codes the dispatch tests (`TCL_BREAK` / `TCL_CONTINUE`); the
/// others (`TCL_ERROR` = 1, `TCL_RETURN` = 2, or a `return -code N`) are handled
/// as "any non-`OK` code" (see [`WasmEmitter::emit_completion_dispatch`]).
const TCL_ERROR: i64 = 1;
const TCL_BREAK: i64 = 3;
const TCL_CONTINUE: i64 = 4;

/// Byte offset of one owned-object slot inside a leaf statement's call frame.
fn slot_offset(slot: usize) -> i64 {
    i64::try_from(slot).unwrap_or(i64::MAX) * i64::from(WASM32_POINTER_BYTES)
}

/// Byte offset of one completion record relative to the frame's completion base.
fn completion_offset(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX) * WASM32_COMPLETION_SIZE
}

/// Default linear-memory base for the emitted module's constant pool: the
/// **reserved region** the whole-program runtime leaves free.
///
/// In the shared-memory link the emitted module and the runtime share one linear
/// memory. The runtime's shadow stack occupies the bottom of memory (it grows
/// **down** from its top), so the emitted module cannot place its data at offset
/// 0 — a deep eval's stack would overwrite it. The runtime is therefore built
/// with its data/heap pushed above a reserved gap (`wasm-ld --global-base`),
/// leaving `[RESERVED_DATA_BASE, runtime data)` free; the emitter relocates its
/// constant pool into that gap. `0x10_0000` (1 MiB) is the runtime's default
/// shadow-stack top, so the gap begins there.
pub const RESERVED_DATA_BASE: i64 = WASM32_CODEGEN_DATA_START;

/// Indices of the `"tcl"` host imports the emitted module calls.
#[derive(Clone, Copy)]
struct Imports {
    host_refusal_pending: u32,
    /// `(ptr, len) -> obj` — box a data-section string as a `TclObj`.
    obj_new_string: u32,
    /// `(script_obj) -> i32` — evaluate a leaf command and return its **completion
    /// code** (`0` ok … `4` continue, or a `return -code N`); the result stays the
    /// interp's own. The emitted control flow branches on the code so abrupt
    /// completion propagates. Adopts (frees) its argument, so
    /// there is no result reference for the emitter to release.
    eval_code: u32,
    /// Completion-bearing expression evaluation with caller-owned source.
    condition: ConditionImports,
    aot: Option<AotImports>,
}

/// The general tier owns condition source and completion storage until the
/// single cleanup block settles Host or Guest completion.
#[derive(Clone, Copy)]
struct ConditionImports {
    frame_alloc: u32,
    frame_free: u32,
    string_owned: u32,
    expr_bool_eval: u32,
    completion_release: u32,
    object_release: u32,
}

const CONDITION_SOURCE_OFFSET: i64 = 0;
const CONDITION_COMPLETION_OFFSET: i64 = WASM32_POINTER_BYTES as i64;
const CONDITION_TRUTH_OFFSET: i64 = CONDITION_COMPLETION_OFFSET + WASM32_COMPLETION_SIZE as i64;
const CONDITION_FRAME_BYTES: i64 = CONDITION_TRUTH_OFFSET + WASM32_POINTER_BYTES as i64;

/// Runtime ABI imports used by the semantic prebuilt-argv mode of the same
/// module emitter.
#[derive(Clone, Copy)]
struct SemanticImports {
    host_refusal_pending: u32,
    frame_alloc: u32,
    frame_free: u32,
    string_owned: u32,
    invoke_argv: u32,
    completion_release: u32,
    object_retain: u32,
    object_release: u32,
}

/// Additional ABI imports for a semantic invocation whose common plan carries
/// a runtime-issued guarded intrinsic proof.
#[derive(Clone, Copy)]
struct GuardedIntrinsicImports {
    semantic: SemanticImports,
    guard_prepare: u32,
    guard_check: u32,
    guard_release: u32,
    invoke_intrinsic_argv: u32,
}

/// Imports used exclusively by a selected native i64-to-boxed boundary.
///
/// Keeping these out of [`AotImports`] leaves the general WASM import surface
/// unchanged when native proof selection is disabled.
#[derive(Clone, Copy)]
struct NativeI64AddImports {
    host_refusal_pending: u32,
    value_new_wide_int: u32,
    puts: u32,
}

#[derive(Clone, Copy)]
enum EmitterImports {
    General(Imports),
    Semantic(SemanticImports),
    GuardedIntrinsic(GuardedIntrinsicImports),
}

#[derive(Clone, Copy)]
struct SemanticCallFrameLayout {
    completion_offset: i32,
    bytes: i32,
}

impl SemanticCallFrameLayout {
    fn validated(argc: usize) -> Self {
        let argc = i32::try_from(argc).expect("semantic plan validated argc");
        let completion_offset = argc
            .checked_mul(WASM32_POINTER_BYTES)
            .expect("semantic plan validated argv size");
        let bytes = completion_offset
            .checked_add(WASM32_COMPLETION_SIZE)
            .expect("semantic plan validated frame size");
        Self {
            completion_offset,
            bytes,
        }
    }
}

#[derive(Clone, Copy)]
struct AotImports {
    value_new_string: u32,
    frame_push: u32,
    frame_pop: u32,
    local_bind: u32,
    local_set: u32,
    local_get: u32,
    var_set: u32,
    var_get: u32,
    expr_add: u32,
    puts: u32,
    proc_register: u32,
    argv: ArgvImports,
}

/// The runtime ABI the general tier's normal leaf-command path calls: evaluate
/// the words, hand the runtime a complete argv, release everything.
#[derive(Clone, Copy, Default)]
struct ArgvImports {
    frame_alloc: u32,
    frame_free: u32,
    string_owned: u32,
    invoke_argv: u32,
    object_release: u32,
    var_get: u32,
    var_get_element: u32,
    word_concat: u32,
}

#[derive(Default)]
struct FunctionFacts {
    operations: HashMap<(u32, u32), SemanticOperationId>,
    direct_assignments: HashMap<(u32, u32), String>,
    direct_calls: HashMap<(u32, u32, String), String>,
    /// Selected prebuilt-argv plans, keyed by leaf-statement span.
    leaf_invocations: HashMap<(u32, u32), WasmLeafInvokePlan>,
    /// Typed reasons a leaf statement stayed on the source-span eval fallback.
    leaf_declines: HashMap<(u32, u32), WasmLeafInvokeDecline>,
    /// Spans of `proc` definitions that wrote every word out, so this tier may
    /// register the definition itself instead of leaving it to the runtime's
    /// own `proc`. Decided while planning, where the statement's
    /// structured words are still to hand.
    literal_proc_definitions: HashSet<(u32, u32)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FunctionMode {
    Top,
    FallbackProc,
    DirectProc,
}

/// One open structured loop, recording the control-frame indices a
/// `break`/`continue`/back-edge branches to. Frame indices count from the
/// outermost open frame (0); the relative `br` depth is derived from the
/// current frame depth at the branch site (see [`WasmEmitter::rel_depth`]).
struct LoopFrame {
    /// The break-scope `block` — `br` here exits the loop.
    break_block: u32,
    /// The `loop` — `br` here re-tests (the back-edge target).
    back_edge: u32,
    /// The continue-scope `block` — `br` here runs the step then re-tests.
    continue_block: u32,
}

/// Collects a function body + data section as the structured walk drives it.
struct WasmEmitter {
    imports: EmitterImports,
    body: Vec<WasmInstruction>,
    data: Vec<WasmData>,
    data_offset: i64,
    /// Number of currently open control frames (`block`/`loop`/`if`).
    ctrl_depth: u32,
    invocation_abort: Option<u32>,
    /// Stack of open loops; the last is the innermost (the `break`/`continue`
    /// target, since Tcl has no labelled break).
    loops: Vec<LoopFrame>,
    code_local: u64,
    /// The transient call-frame pointer local, immediately after `code_local`.
    frame_local: u64,
    mode: FunctionMode,
    local_slots: HashMap<String, u32>,
    proc_indices: HashMap<String, u32>,
    procedure_arity: HashMap<String, usize>,
    direct_procs: HashSet<String>,
    procedures_by_span: HashMap<(u32, u32), Procedure>,
    facts: FunctionFacts,
}

impl WasmEmitter {
    fn for_semantic_invoke(imports: SemanticImports, data_offset: i64) -> Self {
        Self {
            imports: EmitterImports::Semantic(imports),
            body: Vec::new(),
            data: Vec::new(),
            data_offset,
            ctrl_depth: 0,
            invocation_abort: None,
            loops: Vec::new(),
            code_local: 0,
            frame_local: 1,
            mode: FunctionMode::Top,
            local_slots: HashMap::new(),
            proc_indices: HashMap::new(),
            procedure_arity: HashMap::new(),
            direct_procs: HashSet::new(),
            procedures_by_span: HashMap::new(),
            facts: FunctionFacts::default(),
        }
    }

    fn for_guarded_intrinsic_invoke(imports: GuardedIntrinsicImports, data_offset: i64) -> Self {
        Self {
            imports: EmitterImports::GuardedIntrinsic(imports),
            body: Vec::new(),
            data: Vec::new(),
            data_offset,
            ctrl_depth: 0,
            invocation_abort: None,
            loops: Vec::new(),
            code_local: 0,
            // Same local layout as `for_semantic_invoke`: slot 0 holds the
            // completion code, slot 1 the transient call frame.
            frame_local: 1,
            mode: FunctionMode::Top,
            local_slots: HashMap::new(),
            proc_indices: HashMap::new(),
            procedure_arity: HashMap::new(),
            direct_procs: HashSet::new(),
            procedures_by_span: HashMap::new(),
            facts: FunctionFacts::default(),
        }
    }

    fn general_imports(&self) -> Imports {
        match self.imports {
            EmitterImports::General(imports) => imports,
            EmitterImports::Semantic(_) | EmitterImports::GuardedIntrinsic(_) => {
                unreachable!("general lowering in semantic mode")
            }
        }
    }

    fn semantic_imports(&self) -> SemanticImports {
        match self.imports {
            EmitterImports::Semantic(imports) => imports,
            EmitterImports::GuardedIntrinsic(imports) => imports.semantic,
            EmitterImports::General(_) => unreachable!("semantic lowering in general mode"),
        }
    }

    fn guarded_intrinsic_imports(&self) -> GuardedIntrinsicImports {
        match self.imports {
            EmitterImports::GuardedIntrinsic(imports) => imports,
            EmitterImports::General(_) | EmitterImports::Semantic(_) => {
                unreachable!("guarded intrinsic lowering outside guarded semantic mode")
            }
        }
    }

    /// Intern the complete original bytes, including NUL and opaque byte units.
    fn intern_bytes(&mut self, bytes: &[u8]) -> (i64, i64) {
        let offset = self.data_offset;
        let len = i64::try_from(bytes.len()).unwrap_or(i64::MAX);
        self.data.push(WasmData {
            offset,
            data: bytes.to_vec(),
        });
        self.data_offset += len;
        (offset, len)
    }

    fn intern(&mut self, text: &str) -> (i64, i64) {
        self.intern_bytes(text.as_bytes())
    }

    fn push(&mut self, op: WasmOp) {
        self.body.push(WasmInstruction::new(op));
    }

    fn push_i32(&mut self, n: i64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::I32Const,
            leb128_signed(n),
        ));
    }

    fn push_i64(&mut self, n: i64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::I64Const,
            leb128_signed(n),
        ));
    }

    fn raw_call(&mut self, func_idx: u32) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Call,
            leb128_unsigned(u64::from(func_idx)),
        ));
    }

    fn call(&mut self, func_idx: u32) {
        self.raw_call(func_idx);
        let EmitterImports::General(imports) = self.imports else {
            return;
        };
        if !general_import_id(imports, func_idx)
            .is_none_or(CodegenAbiImportId::requires_host_refusal_check)
        {
            return;
        }
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        if let Some(abort) = self.invocation_abort {
            self.br(abort);
        } else {
            self.emit_general_refusal_return(imports);
        }
        self.close_frame();
    }

    fn emit_general_refusal_return(&mut self, imports: Imports) {
        if self.mode == FunctionMode::DirectProc
            && let Some(aot) = imports.aot
        {
            self.raw_call(aot.frame_pop);
        }
        if self.mode == FunctionMode::DirectProc {
            self.push_i32(0);
        }
        self.push(WasmOp::Return);
    }

    fn emit_semantic_refusal_check(&mut self, argc: usize, guarded: bool) {
        let imports = self.semantic_imports();
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        if guarded {
            self.local_get(GUARDED_TOKEN_LOCAL);
            self.raw_call(self.guarded_intrinsic_imports().guard_release);
        }
        for index in 0..argc {
            let local = if guarded {
                semantic_word_local(index)
            } else {
                u64::try_from(5 + index).expect("word local fits u64")
            };
            self.local_get(local);
            self.raw_call(imports.object_release);
        }
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.raw_call(imports.frame_free);
        self.push(WasmOp::Drop);
        // These are transport placeholders; the retained refusal governs the result.
        for _ in 0..3 {
            self.push_i32(0);
        }
        self.push(WasmOp::Return);
        self.close_frame();
    }

    /// Open a structured frame (`block`/`loop`/`if`, void type), returning its
    /// index in the open-frame stack.
    fn open_frame(&mut self, op: WasmOp) -> u32 {
        self.body
            .push(WasmInstruction::with_operands(op, vec![BLOCK_VOID]));
        let idx = self.ctrl_depth;
        self.ctrl_depth += 1;
        idx
    }

    /// Close the innermost structured frame (`end`).
    fn close_frame(&mut self) {
        self.push(WasmOp::End);
        self.ctrl_depth = self.ctrl_depth.saturating_sub(1);
    }

    /// The relative `br` depth from the current point to the frame at `idx`
    /// (innermost open frame = 0).
    fn rel_depth(&self, idx: u32) -> u32 {
        self.ctrl_depth.saturating_sub(1).saturating_sub(idx)
    }

    fn br(&mut self, idx: u32) {
        let d = self.rel_depth(idx);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Br,
            leb128_unsigned(u64::from(d)),
        ));
    }

    fn br_if(&mut self, idx: u32) {
        let d = self.rel_depth(idx);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::BrIf,
            leb128_unsigned(u64::from(d)),
        ));
    }

    /// Box original bytes as a Tcl object without a text projection.
    fn box_bytes(&mut self, bytes: &[u8]) {
        let (offset, len) = self.intern_bytes(bytes);
        self.push_i32(offset);
        self.push_i32(len);
        self.call(self.general_imports().obj_new_string);
    }

    fn box_text(&mut self, text: &str) {
        self.box_bytes(text.as_bytes());
    }

    fn push_text_pair(&mut self, text: &str) {
        let (offset, len) = self.intern(text);
        self.push_i32(offset);
        self.push_i32(len);
    }

    fn box_value(&mut self, text: &str) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        self.push_text_pair(text);
        self.call(aot.value_new_string);
        true
    }

    /// A direct procedure's expression reads only its formal parameters
    /// (`direct_expr_supported`), so every name is a frame slot. Nothing else
    /// reaches this: `emit_expr_value` runs solely from the `DirectProc`
    /// `Statement::Return` arm.
    fn emit_var_get(&mut self, name: &str) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        if self.mode != FunctionMode::DirectProc {
            return false;
        }
        let Some(slot) = self.local_slots.get(name).copied() else {
            return false;
        };
        self.push_i32(i64::from(slot));
        self.call(aot.local_get);
        true
    }

    fn emit_expr_value(&mut self, expr: &ExprNode) -> bool {
        match expr {
            ExprNode::Var { name, .. } => self.emit_var_get(name),
            ExprNode::Literal { text, .. } => self.box_value(text),
            ExprNode::Binary {
                op: BinOp::Add,
                left,
                right,
            } => {
                if !self.emit_expr_value(left) || !self.emit_expr_value(right) {
                    return false;
                }
                let Some(aot) = self.general_imports().aot else {
                    return false;
                };
                self.call(aot.expr_add);
                true
            }
            _ => false,
        }
    }

    fn emit_proc_prelude(&mut self, proc: &Procedure) {
        let Some(aot) = self.general_imports().aot else {
            return;
        };
        self.call(aot.frame_push);
        for (param_idx, name) in proc.params.iter().enumerate() {
            self.push_i32(i64::try_from(param_idx).unwrap_or(i64::MAX));
            self.push_text_pair(name);
            self.local_get(u64::try_from(param_idx).unwrap_or(u64::MAX));
            self.call(aot.local_bind);
            self.push(WasmOp::Drop);
        }
    }

    /// Run one emission alternative, rolling the body back if it declines.
    ///
    /// A declining `try_*` emitter is not necessarily a no-op: it may append
    /// part of its sequence before reaching a word it cannot prove. Anything
    /// that falls through to another alternative must therefore undo the
    /// partial work first, or the abandoned prefix stays in the body with its
    /// operands stranded on the operand stack.
    fn attempt(&mut self, emit: impl FnOnce(&mut Self) -> bool) -> bool {
        let body_len = self.body.len();
        let data_len = self.data.len();
        let data_offset = self.data_offset;
        if emit(self) {
            true
        } else {
            self.body.truncate(body_len);
            self.data.truncate(data_len);
            self.data_offset = data_offset;
            false
        }
    }

    fn try_emit_typed_statement(&mut self, statement: &Statement) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        match statement {
            Statement::AssignConst { span, name, .. }
            | Statement::AssignValue { span, name, .. } => {
                let Some(value) = self.facts.direct_assignments.get(&span_key(*span)).cloned()
                else {
                    return false;
                };
                if self.mode == FunctionMode::DirectProc {
                    let Some(slot) = self.local_slots.get(name).copied() else {
                        return false;
                    };
                    self.push_i32(i64::from(slot));
                    if !self.box_value(&value) {
                        return false;
                    }
                    self.call(aot.local_set);
                } else if self.mode == FunctionMode::Top {
                    self.push_text_pair(name);
                    if !self.box_value(&value) {
                        return false;
                    }
                    self.call(aot.var_set);
                } else {
                    return false;
                }
                self.emit_completion_dispatch();
                true
            }
            Statement::Return {
                expr: Some(expr), ..
            } if self.mode == FunctionMode::DirectProc => {
                if !self.emit_expr_value(expr) {
                    return false;
                }
                self.call(aot.frame_pop);
                self.push(WasmOp::Return);
                true
            }
            Statement::Call { span, .. } => {
                // Each alternative must be transactional. A `try_*` may emit
                // part of its sequence before hitting a word it cannot prove
                // and declining — `try_emit_direct_operation` boxes leading
                // arguments before it reaches, say, an `$arr(key)` it does not
                // handle. `emit_typed_statement` only rolls back when the whole
                // chain declines, so without this a decline followed by a
                // successful fallback would leave the half-emitted prefix in
                // the body and its operands stranded on the stack, producing a
                // module wasmtime rejects with "values remaining on stack at
                // end of block".
                self.attempt(|emitter| emitter.try_emit_direct_operation(*span))
                    || self.attempt(|emitter| emitter.try_emit_leaf_invocation(*span))
            }
            _ => false,
        }
    }

    /// The narrow direct specialisations that beat generic argv invocation
    /// when the binding lattice proves the command is still its builtin.
    fn try_emit_direct_operation(&mut self, span: Span) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        let Some(operation) = self.facts.operations.get(&span_key(span)).copied() else {
            return false;
        };
        match operation {
            SemanticOperationId::StructuredLowering(LoweringHookId::Proc) => {
                // Only a definition whose words were all written out. Otherwise
                // `Procedure` holds the *written* word while the body was
                // compiled from a value this tier materialised, and registering
                // that word reports the wrong `info body` and leaves any later
                // run of the source body evaluating a substitution in the
                // procedure's own frame. The generic invocation
                // below hands the word to the runtime's `proc`, which evaluates
                // it at the call site as Tcl does.
                if !self
                    .facts
                    .literal_proc_definitions
                    .contains(&span_key(span))
                {
                    return false;
                }
                let Some(proc) = self.procedures_by_span.get(&span_key(span)).cloned() else {
                    return false;
                };
                let Some(body) = proc.body_source.as_deref() else {
                    return false;
                };
                self.push_text_pair(&proc.qualified_name);
                self.push_text_pair(&proc.params_raw);
                self.push_text_pair(body);
                self.call(aot.proc_register);
                self.emit_completion_dispatch();
                true
            }
            // There is no `puts` fast path reparsing compatibility text: the
            // leaf-invocation path below evaluates its words structurally, and
            // the native tier owns the channel-write intrinsic.
            SemanticOperationId::Invoke
            | SemanticOperationId::Intrinsic(_)
            | SemanticOperationId::StructuredLowering(_) => false,
        }
    }

    /// The general tier's normal leaf-command path: evaluate every word into a
    /// transient call frame, dispatch the complete argv through the runtime's
    /// ordinary command resolution, and release everything before the
    /// completion is allowed to branch anywhere.
    ///
    /// The whole statement sits inside one `block`. Word evaluation branches to
    /// its end on an abrupt completion, so there is exactly one cleanup path:
    /// every object slot was nulled up front and is released null-safely, then
    /// the frame is freed. Only after that does the completion dispatch run and
    /// possibly `br`/`return` out of the statement, so no exit path can leak.
    fn try_emit_leaf_invocation(&mut self, span: Span) -> bool {
        // A direct procedure returns a value, so the completion dispatch's bare
        // `return` is not well typed there. Its body is a single proven
        // expression return, so no leaf call reaches this in practice; the
        // guard keeps that a property of the code rather than of the caller.
        if self.mode == FunctionMode::DirectProc {
            return false;
        }
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        let Some(plan) = self.facts.leaf_invocations.get(&span_key(span)).cloned() else {
            return false;
        };
        let argv = aot.argv;

        self.push_i32(i64::from(plan.frame_bytes));
        self.push_i32(i64::from(plan.frame_align));
        self.call(argv.frame_alloc);
        self.local_set(self.frame_local);
        for slot in 0..plan.object_slots {
            self.local_get(self.frame_local);
            self.push_i32(0);
            self.store_i32(slot_offset(slot));
        }
        self.push_i32(0);
        self.local_set(self.code_local);

        let abort = self.open_frame(WasmOp::Block);
        self.invocation_abort = Some(abort);
        self.emit_invoke_node(&plan.root, &plan, argv, abort, true);
        self.invocation_abort = None;
        self.close_frame();

        for slot in 0..plan.object_slots {
            self.local_get(self.frame_local);
            self.load_i32(slot_offset(slot));
            self.call(argv.object_release);
        }
        self.local_get(self.frame_local);
        self.call(argv.frame_free);
        self.push(WasmOp::Drop);

        let imports = self.general_imports();
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        self.dispatch_stashed_code();
        true
    }

    /// Emit one invocation: its words, the argv dispatch, and adoption of the
    /// completion's owned result and options into frame slots the single
    /// cleanup path releases.
    ///
    /// A nested invocation additionally propagates an abrupt completion to the
    /// statement's `abort` block; the outermost one leaves its code in
    /// `code_local` for the completion dispatch after cleanup.
    fn emit_invoke_node(
        &mut self,
        node: &WasmInvokeNode,
        plan: &WasmLeafInvokePlan,
        argv: ArgvImports,
        abort: u32,
        outermost: bool,
    ) {
        for word in &node.words {
            self.emit_word_plan(word, plan, argv, abort);
        }
        let completion = plan.completion_base + completion_offset(node.completion);

        self.local_get(self.frame_local);
        self.push_i32(slot_offset(node.argv_slot));
        self.push(WasmOp::I32Add);
        self.push_i32(i64::try_from(node.words.len()).unwrap_or(i64::MAX));
        self.local_get(self.frame_local);
        self.push_i32(i64::from(completion));
        self.push(WasmOp::I32Add);
        self.call(argv.invoke_argv);
        self.push(WasmOp::Drop);

        self.local_get(self.frame_local);
        self.load_i32(i64::from(completion + WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(self.code_local);
        self.adopt_completion_handle(
            completion + WASM32_COMPLETION_RESULT_OFFSET,
            node.result_slot,
        );
        self.adopt_completion_handle(
            completion + WASM32_COMPLETION_OPTIONS_OFFSET,
            node.options_slot,
        );

        if !outermost {
            self.local_get(self.code_local);
            self.open_frame(WasmOp::If);
            self.br(abort);
            self.close_frame();
        }
    }

    /// Move one owned completion handle into the frame slot the statement's
    /// cleanup path releases.
    fn adopt_completion_handle(&mut self, completion_field: i32, slot: usize) {
        self.local_get(self.frame_local);
        self.local_get(self.frame_local);
        self.load_i32(i64::from(completion_field));
        self.store_i32(slot_offset(slot));
    }

    fn emit_word_plan(
        &mut self,
        word: &WasmWordPlan,
        plan: &WasmLeafInvokePlan,
        argv: ArgvImports,
        abort: u32,
    ) {
        match word {
            WasmWordPlan::Literal { slot, text } => {
                self.local_get(self.frame_local);
                self.push_text_pair(text);
                self.call(argv.string_owned);
                self.store_i32(slot_offset(*slot));
            }
            WasmWordPlan::Scalar { slot, name } => {
                self.local_get(self.frame_local);
                self.push_text_pair(name);
                self.call(argv.var_get);
                self.store_i32(slot_offset(*slot));
                self.emit_word_error_guard(*slot, abort);
            }
            WasmWordPlan::Element { slot, name, key } => {
                self.local_get(self.frame_local);
                self.push_text_pair(name);
                self.push_text_pair(key);
                self.call(argv.var_get_element);
                self.store_i32(slot_offset(*slot));
                self.emit_word_error_guard(*slot, abort);
            }
            WasmWordPlan::Invoke(node) => self.emit_invoke_node(node, plan, argv, abort, false),
            WasmWordPlan::Concat {
                slot,
                parts_slot,
                parts,
            } => {
                for part in parts {
                    self.emit_word_plan(part, plan, argv, abort);
                }
                self.local_get(self.frame_local);
                self.local_get(self.frame_local);
                self.push_i32(slot_offset(*parts_slot));
                self.push(WasmOp::I32Add);
                self.push_i32(i64::try_from(parts.len()).unwrap_or(i64::MAX));
                self.call(argv.word_concat);
                self.store_i32(slot_offset(*slot));
                self.emit_word_error_guard(*slot, abort);
            }
        }
    }

    /// A null word means the runtime reported a Tcl error while evaluating it
    /// (a missing variable, a failed read trace). Record `TCL_ERROR` and leave
    /// through the statement's single cleanup path.
    fn emit_word_error_guard(&mut self, slot: usize, abort: u32) {
        self.local_get(self.frame_local);
        self.load_i32(slot_offset(slot));
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.push_i32(TCL_ERROR);
        self.local_set(self.code_local);
        self.br(abort);
        self.close_frame();
    }

    fn local_get(&mut self, idx: u64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalGet,
            leb128_unsigned(idx),
        ));
    }

    fn local_set(&mut self, idx: u64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalSet,
            leb128_unsigned(idx),
        ));
    }

    fn local_tee(&mut self, idx: u64) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::LocalTee,
            leb128_unsigned(idx),
        ));
    }

    fn store_i32(&mut self, offset: i64) {
        let mut operands = leb128_unsigned(2);
        operands.extend(leb128_unsigned(
            u64::try_from(offset).expect("non-negative semantic frame offset"),
        ));
        self.body
            .push(WasmInstruction::with_operands(WasmOp::I32Store, operands));
    }

    fn load_i32(&mut self, offset: i64) {
        let mut operands = leb128_unsigned(2);
        operands.extend(leb128_unsigned(
            u64::try_from(offset).expect("non-negative completion offset"),
        ));
        self.body
            .push(WasmInstruction::with_operands(WasmOp::I32Load, operands));
    }

    /// Emit the selected semantic invocation through this emitter's shared
    /// instruction and data builders.
    fn finish_semantic_invoke(&mut self, plan: &WasmGenericInvokePlan) -> WasmFunction {
        const FRAME_LOCAL: u64 = 0;
        const COMPLETION_LOCAL: u64 = 1;
        const CODE_LOCAL: u64 = 2;
        const RESULT_LOCAL: u64 = 3;
        const OPTIONS_LOCAL: u64 = 4;
        let word_local = |index: usize| u64::try_from(5 + index).expect("word local fits u64");
        let imports = self.semantic_imports();
        let layout = SemanticCallFrameLayout::validated(plan.argv_literals.len());
        let literals = plan
            .argv_literals
            .iter()
            .map(|literal| self.intern(literal))
            .collect::<Vec<_>>();

        self.push_i32(i64::from(layout.bytes));
        self.push_i32(i64::from(WASM32_COMPLETION_ALIGN));
        self.call(imports.frame_alloc);
        self.local_set(FRAME_LOCAL);

        for (index, (offset, length)) in literals.iter().copied().enumerate() {
            self.push_i32(offset);
            self.push_i32(length);
            self.call(imports.string_owned);
            self.local_set(word_local(index));
            self.local_get(FRAME_LOCAL);
            self.local_get(word_local(index));
            self.store_i32(i64::try_from(index * 4).expect("argv offset fits i64"));
        }

        self.local_get(FRAME_LOCAL);
        self.push_i32(i64::try_from(plan.argv_literals.len()).expect("validated argc"));
        self.local_get(FRAME_LOCAL);
        self.push_i32(i64::from(layout.completion_offset));
        self.push(WasmOp::I32Add);
        self.local_tee(COMPLETION_LOCAL);
        self.call(imports.invoke_argv);
        self.push(WasmOp::Drop);
        self.emit_semantic_refusal_check(plan.argv_literals.len(), false);

        self.local_get(COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(CODE_LOCAL);
        self.local_get(COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.call(imports.object_retain);
        self.local_set(RESULT_LOCAL);
        self.local_get(COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(imports.object_retain);
        self.local_set(OPTIONS_LOCAL);
        self.local_get(COMPLETION_LOCAL);
        self.call(imports.completion_release);

        for index in 0..literals.len() {
            self.local_get(word_local(index));
            self.call(imports.object_release);
        }
        self.local_get(FRAME_LOCAL);
        self.call(imports.frame_free);
        self.push(WasmOp::Drop);

        self.local_get(CODE_LOCAL);
        self.local_get(RESULT_LOCAL);
        self.local_get(OPTIONS_LOCAL);
        self.push(WasmOp::Return);

        let mut local_names = vec![
            "$frame".to_string(),
            "$completion".to_string(),
            "$code".to_string(),
            "$result".to_string(),
            "$options".to_string(),
        ];
        local_names.extend((0..literals.len()).map(|index| format!("$word{index}")));
        WasmFunction {
            name: plan.function_name.clone(),
            params: Vec::new(),
            results: vec![ValType::I32, ValType::I32, ValType::I32],
            locals: vec![ValType::I32; 5 + literals.len()],
            body: std::mem::take(&mut self.body),
            local_names,
            exported: true,
            source_range: None,
            kind: "semantic-generic-invoke".to_string(),
        }
    }

    /// Emit a guarded intrinsic attempt over the semantic plan's sole,
    /// already-materialised argv. Any runtime decline executes the exact
    /// generic argv path; no source words are re-evaluated or replayed.
    fn finish_guarded_intrinsic_invoke(
        &mut self,
        plan: &WasmGenericInvokePlan,
        evidence: &GuardedSelectionEvidence,
    ) -> WasmFunction {
        let semantic = self.semantic_imports();
        let guarded = self.guarded_intrinsic_imports();
        assert_eq!(plan.operation, evidence.operation());
        assert_eq!(evidence.guarded_plan().fast(), &plan.operation);
        self.emit_guarded_argv_frame(plan, semantic);
        self.emit_guarded_intrinsic_dispatch(plan, evidence, guarded);
        self.emit_guarded_intrinsic_completion_return(plan, semantic);
        self.guarded_intrinsic_function(plan)
    }

    fn emit_guarded_argv_frame(&mut self, plan: &WasmGenericInvokePlan, imports: SemanticImports) {
        let layout = SemanticCallFrameLayout::validated(plan.argv_literals.len());
        let literals = plan
            .argv_literals
            .iter()
            .map(|literal| self.intern(literal))
            .collect::<Vec<_>>();
        self.push_i32(i64::from(layout.bytes));
        self.push_i32(i64::from(WASM32_COMPLETION_ALIGN));
        self.call(imports.frame_alloc);
        self.local_set(SEMANTIC_FRAME_LOCAL);
        for (index, (offset, length)) in literals.iter().copied().enumerate() {
            self.push_i32(offset);
            self.push_i32(length);
            self.call(imports.string_owned);
            self.local_set(semantic_word_local(index));
            self.local_get(SEMANTIC_FRAME_LOCAL);
            self.local_get(semantic_word_local(index));
            self.store_i32(i64::try_from(index * 4).expect("argv offset fits i64"));
        }
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::from(layout.completion_offset));
        self.push(WasmOp::I32Add);
        self.local_set(SEMANTIC_COMPLETION_LOCAL);
    }

    fn emit_guarded_intrinsic_dispatch(
        &mut self,
        plan: &WasmGenericInvokePlan,
        evidence: &GuardedSelectionEvidence,
        imports: GuardedIntrinsicImports,
    ) {
        let SemanticOperationId::Intrinsic(intrinsic) = evidence.operation() else {
            unreachable!("guarded intrinsic evidence must retain an intrinsic operation");
        };
        let guard = evidence.guarded_plan().guard();
        let identity = guard.expected_identity();
        let argc = plan.argv_literals.len();
        self.push_i32(i64::from(intrinsic.stable_id()));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.push_i32(i64::from(identity.namespace()));
        self.push_i64(i64::try_from(identity.value()).expect("guard identity fits i64"));
        self.push_i32(i64::from(guard.domains().bits()));
        self.call(imports.guard_prepare);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.emit_semantic_refusal_check(argc, true);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.push(WasmOp::I64Eqz);
        self.open_frame(WasmOp::If);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.push(WasmOp::Else);
        self.emit_guarded_token_path(intrinsic.stable_id(), argc, imports);
        self.close_frame();
    }

    fn emit_guarded_token_path(
        &mut self,
        intrinsic: u32,
        argc: usize,
        imports: GuardedIntrinsicImports,
    ) {
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.push_i32(i64::from(intrinsic));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.call(imports.guard_check);
        self.emit_semantic_refusal_check(argc, true);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push_i64(0);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.push(WasmOp::Else);
        self.push_i32(i64::from(intrinsic));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.call(imports.invoke_intrinsic_argv);
        self.local_set(GUARDED_STATUS_LOCAL);
        self.emit_semantic_refusal_check(argc, true);
        self.local_get(GUARDED_STATUS_LOCAL);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push_i64(0);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.push(WasmOp::Else);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push_i64(0);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.close_frame();
        self.close_frame();
    }

    fn emit_guarded_intrinsic_completion_return(
        &mut self,
        plan: &WasmGenericInvokePlan,
        imports: SemanticImports,
    ) {
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(SEMANTIC_CODE_LOCAL);
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.call(imports.object_retain);
        self.local_set(SEMANTIC_RESULT_LOCAL);
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(imports.object_retain);
        self.local_set(SEMANTIC_OPTIONS_LOCAL);
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.call(imports.completion_release);
        for index in 0..plan.argv_literals.len() {
            self.local_get(semantic_word_local(index));
            self.call(imports.object_release);
        }
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.call(imports.frame_free);
        self.push(WasmOp::Drop);
        self.local_get(SEMANTIC_CODE_LOCAL);
        self.local_get(SEMANTIC_RESULT_LOCAL);
        self.local_get(SEMANTIC_OPTIONS_LOCAL);
        self.push(WasmOp::Return);
    }

    fn guarded_intrinsic_function(&mut self, plan: &WasmGenericInvokePlan) -> WasmFunction {
        let mut local_names = vec![
            "$frame".to_string(),
            "$completion".to_string(),
            "$code".to_string(),
            "$result".to_string(),
            "$options".to_string(),
            "$guard_token".to_string(),
            "$intrinsic_status".to_string(),
        ];
        local_names.extend((0..plan.argv_literals.len()).map(|index| format!("$word{index}")));
        let mut locals = vec![ValType::I32; 5];
        locals.push(ValType::I64);
        locals.push(ValType::I32);
        locals.extend(std::iter::repeat_n(ValType::I32, plan.argv_literals.len()));
        WasmFunction {
            name: plan.function_name.clone(),
            params: Vec::new(),
            results: vec![ValType::I32, ValType::I32, ValType::I32],
            locals,
            body: std::mem::take(&mut self.body),
            local_names,
            exported: true,
            source_range: None,
            kind: "semantic-guarded-intrinsic-invoke".to_string(),
        }
    }

    /// Call the generic ABI over the exact argv and completion frame already
    /// owned by this semantic emission.
    fn emit_generic_argv_invoke(&mut self, argc: usize, frame_local: u64, completion_local: u64) {
        self.local_get(frame_local);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.local_get(completion_local);
        self.call(self.semantic_imports().invoke_argv);
        self.push(WasmOp::Drop);
        self.emit_semantic_refusal_check(argc, true);
    }

    /// Honour the completion code a leaf command's [`tcl_eval_code`] left on the
    /// stack — the AOT realisation of "stop the script on the
    /// first non-`OK` command" loop (`eval_script_mode`), so abrupt completion
    /// propagates through compiled `if`/`while`/`for` instead of being swallowed.
    ///
    /// Inside a loop, `break` (3) / `continue` (4) re-enter that loop's structural
    /// scopes (identical to a literal `break`/`continue`, so a *dynamic* one — a
    /// called command that completes `break` — behaves the same). Any other
    /// non-`OK` code (error, return, a `return -code N`, or a break/continue with
    /// no enclosing loop) unwinds the function with `return`. `OK` (0) falls
    /// through to the next statement.
    fn emit_completion_dispatch(&mut self) {
        // Stash the code; the dispatch reads it up to three times.
        self.local_set(self.code_local);
        self.dispatch_stashed_code();
    }

    /// [`Self::emit_completion_dispatch`] for a code already in `code_local`.
    fn dispatch_stashed_code(&mut self) {
        let enclosing = self
            .loops
            .last()
            .map(|frame| (frame.break_block, frame.continue_block));
        self.dispatch_stashed_code_in_loop(enclosing);
    }

    fn dispatch_stashed_code_in_loop(&mut self, enclosing: Option<(u32, u32)>) {
        if let Some((break_block, continue_block)) = enclosing {
            self.emit_code_eq_branch(TCL_BREAK, break_block);
            self.emit_code_eq_branch(TCL_CONTINUE, continue_block);
        }
        self.local_get(self.code_local);
        self.open_frame(WasmOp::If);
        // Only a DirectProc has the frame emitted by emit_proc_prelude.
        self.emit_general_refusal_return(self.general_imports());
        self.close_frame();
    }

    /// Evaluate through the existing raw-evaluation/result/truth owner once,
    /// then release the source and completion before any structural edge.
    fn emit_condition_expression(&mut self, text: &str, loop_header: bool) {
        let imports = self.general_imports();
        let condition = imports.condition;
        let status_local = self.frame_local + 1;
        let truth_local = self.frame_local + 2;
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        self.push_i32(CONDITION_FRAME_BYTES);
        self.push_i32(i64::from(WASM32_COMPLETION_ALIGN));
        self.call(condition.frame_alloc);
        self.local_set(self.frame_local);
        self.local_get(self.frame_local);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        for offset in (0..CONDITION_FRAME_BYTES).step_by(WASM32_POINTER_BYTES as usize) {
            self.local_get(self.frame_local);
            self.push_i32(0);
            self.store_i32(offset);
        }
        self.push_i32(0);
        self.local_set(self.code_local);
        self.push_i32(0);
        self.local_set(status_local);
        self.push_i32(0);
        self.local_set(truth_local);

        let abort = self.open_frame(WasmOp::Block);
        self.invocation_abort = Some(abort);
        self.local_get(self.frame_local);
        self.push_text_pair(text);
        self.call(condition.string_owned);
        self.store_i32(CONDITION_SOURCE_OFFSET);
        self.local_get(self.frame_local);
        self.load_i32(CONDITION_SOURCE_OFFSET);
        self.local_get(self.frame_local);
        self.push_i32(CONDITION_COMPLETION_OFFSET);
        self.push(WasmOp::I32Add);
        self.local_get(self.frame_local);
        self.push_i32(CONDITION_TRUTH_OFFSET);
        self.push(WasmOp::I32Add);
        self.call(condition.expr_bool_eval);
        self.local_set(status_local);
        self.local_get(status_local);
        self.open_frame(WasmOp::If);
        self.br(abort);
        self.close_frame();
        self.local_get(self.frame_local);
        self.load_i32(CONDITION_COMPLETION_OFFSET + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(self.code_local);
        self.local_get(self.code_local);
        self.open_frame(WasmOp::If);
        self.br(abort);
        self.close_frame();
        self.local_get(self.frame_local);
        self.load_i32(CONDITION_TRUTH_OFFSET);
        self.local_set(truth_local);
        self.invocation_abort = None;
        self.close_frame();

        self.local_get(self.frame_local);
        self.load_i32(CONDITION_SOURCE_OFFSET);
        self.call(condition.object_release);
        self.local_get(self.frame_local);
        self.push_i32(CONDITION_COMPLETION_OFFSET);
        self.push(WasmOp::I32Add);
        self.call(condition.completion_release);
        self.local_get(self.frame_local);
        self.call(condition.frame_free);
        self.push(WasmOp::Drop);
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        self.local_get(status_local);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();

        // A loop test is outside its own body completion handler. Its failure
        // belongs to the caller of the whole while/for, possibly an outer loop.
        let enclosing = if loop_header {
            self.loops.iter().rev().nth(1)
        } else {
            self.loops.last()
        }
        .map(|frame| (frame.break_block, frame.continue_block));
        self.dispatch_stashed_code_in_loop(enclosing);
        self.local_get(truth_local);
    }

    /// `if (code == want) br <target>` — a guarded structural branch the
    /// completion dispatch uses for `break`/`continue`. The `if` frame is opened
    /// so the `br` depth is computed with it in place (crossing it, plus any
    /// enclosing `if`s, back to the loop scope).
    fn emit_code_eq_branch(&mut self, want: i64, target: u32) {
        self.local_get(self.code_local);
        self.push_i32(want);
        self.push(WasmOp::I32Eq);
        self.open_frame(WasmOp::If);
        self.br(target);
        self.close_frame();
    }

    /// Close the function the walk just finished — emit its terminal `end`, take
    /// its instruction stream, and reset the per-function state for the next one.
    /// The constant pool (`data`/`data_offset`) is module-global and persists
    /// across functions: every function's strings share one pool in the shared
    /// linear memory, at distinct offsets.
    ///
    /// The terminal `end` is emitted unconditionally — a body ending in a loop's
    /// own `end` would otherwise leave `encode_body` to mistake that for the
    /// function `end` and leave a frame open.
    fn finish_function(
        &mut self,
        name: &str,
        kind: &str,
        proc: Option<&Procedure>,
    ) -> WasmFunction {
        let direct = self.mode == FunctionMode::DirectProc;
        if direct {
            self.box_value("");
            if let Some(aot) = self.general_imports().aot {
                self.call(aot.frame_pop);
            }
        }
        self.push(WasmOp::End);
        self.ctrl_depth = 0;
        self.loops.clear();
        let params = if direct {
            vec![ValType::I32; proc.map_or(0, |p| p.params.len())]
        } else {
            Vec::new()
        };
        let mut local_names = if direct {
            proc.map_or_else(Vec::new, |p| {
                p.params.iter().map(|name| format!("${name}")).collect()
            })
        } else {
            Vec::new()
        };
        local_names.push("$code".to_string());
        local_names.push("$frame".to_string());
        local_names.push("$condition_status".to_string());
        local_names.push("$condition_truth".to_string());
        WasmFunction {
            name: name.to_string(),
            params,
            results: if direct {
                vec![ValType::I32]
            } else {
                Vec::new()
            },
            // Completion code, transient frame, condition transport and truth.
            locals: vec![ValType::I32; 4],
            body: std::mem::take(&mut self.body),
            local_names,
            exported: true,
            source_range: proc.map(|p| p.span),
            kind: kind.to_string(),
        }
    }
}

impl Emit for WasmEmitter {
    fn refuse_native_compilation_admission(&mut self) {
        self.push(WasmOp::Unreachable);
    }

    fn emit_typed_statement(&mut self, statement: &Statement, _source: &str) -> bool {
        self.attempt(|emitter| emitter.try_emit_typed_statement(statement))
    }

    fn emit_command_image(&mut self, source: &tcl_lexer::SourceImage) {
        // code = tcl_eval_code(box(text)); then honour an abrupt completion code
        // (error/return unwinds, break/continue re-enters the loop) instead of
        // swallowing it — the top-level result stays the interp's own result.
        self.box_bytes(source.bytes());
        self.call(self.general_imports().eval_code);
        self.emit_completion_dispatch();
    }

    fn begin_if(&mut self, cond_text: &str) {
        self.emit_condition_expression(cond_text, false);
        self.open_frame(WasmOp::If);
    }

    fn begin_else(&mut self) {
        // `else` stays in the same `if` frame — no depth change.
        self.push(WasmOp::Else);
    }

    fn end_if(&mut self) {
        self.close_frame();
    }

    fn begin_loop(&mut self) {
        // block (break scope) ⊃ loop (retest / back-edge). The continue scope
        // opens in `begin_loop_body`, after the guard.
        let break_block = self.open_frame(WasmOp::Block);
        let back_edge = self.open_frame(WasmOp::Loop);
        self.loops.push(LoopFrame {
            break_block,
            back_edge,
            continue_block: back_edge, // provisional; set in begin_loop_body
        });
    }

    fn loop_test(&mut self, cond_text: Option<&str>) {
        if let Some(cond) = cond_text {
            self.emit_condition_expression(cond, true);
            self.push(WasmOp::I32Eqz);
            if let Some(frame) = self.loops.last() {
                let brk = frame.break_block;
                self.br_if(brk);
            }
        }
    }

    fn begin_loop_body(&mut self) {
        let continue_block = self.open_frame(WasmOp::Block);
        if let Some(frame) = self.loops.last_mut() {
            frame.continue_block = continue_block;
        }
    }

    fn end_loop_body(&mut self) {
        // Close the continue scope: a `continue` (and the body's fall-through)
        // lands here, then runs any step and the back-edge.
        self.close_frame();
    }

    fn end_loop(&mut self) {
        if let Some(frame) = self.loops.last() {
            let back_edge = frame.back_edge;
            self.br(back_edge); // back-edge: re-test
        }
        self.close_frame(); // close loop
        self.close_frame(); // close break scope
        self.loops.pop();
    }

    fn emit_break(&mut self) {
        if let Some(frame) = self.loops.last() {
            let idx = frame.break_block;
            self.br(idx);
        }
    }

    fn emit_continue(&mut self) {
        if let Some(frame) = self.loops.last() {
            let idx = frame.continue_block;
            self.br(idx);
        }
    }

    fn emit_return(&mut self) {
        self.push(WasmOp::Return);
    }
}

fn span_key(span: Span) -> (u32, u32) {
    (span.start(), span.end())
}

fn span_command_key(span: Span, command: &str) -> (u32, u32, String) {
    (span.start(), span.end(), command.to_string())
}

fn direct_expr_supported(
    expr: &ExprNode,
    params: &HashSet<&str>,
    config: tcl_lexer::LexerConfig,
) -> bool {
    match expr {
        ExprNode::Var { text, name, .. } => {
            matches!(
                crate::native_lowering::cells::variable_reference_place(text, config),
                Ok(crate::native_lowering::cells::CellPlace::Named { name: actual })
                    if actual == *name && params.contains(actual.as_str())
            )
        }
        ExprNode::Literal { .. } => true,
        ExprNode::Binary {
            op: BinOp::Add,
            left,
            right,
        } => {
            direct_expr_supported(left, params, config)
                && direct_expr_supported(right, params, config)
        }
        _ => false,
    }
}

fn direct_proc_eligible(
    module: &Module,
    proc: &Procedure,
    unit: &FunctionUnit,
    registry: &CommandRegistry,
    mutations: &ModuleCommandMutations,
) -> bool {
    let Some(operations) = original_direct_expression_body_operations(
        registry,
        unit.invocation_metadata_context_for_module(registry, module),
        proc,
        unit.source_lexer_config(),
    ) else {
        return false;
    };
    if crate::native_compilation_admission::script_requires_admission(&proc.body)
        || proc.namespace_scoped
        || proc
            .qualified_name
            .strip_prefix("::")
            .is_some_and(|name| name.contains("::"))
        || module.redefined_procedures.contains(&proc.qualified_name)
        || unit.complexity_guarded
        || operations
            .into_iter()
            .any(|operation| !semantic_operation_binding_is_trusted(registry, mutations, operation))
        || !matches!(
            unit.return_type.tcl_type(),
            Some(TclType::Int | TclType::Double | TclType::Numeric)
        )
    {
        return false;
    }
    let Some(arguments) =
        crate::var_escape::original_slots::original_procedure_argument_slots(module, proc)
    else {
        return false;
    };
    // This emitter's string-keyed ABI is admitted only after exact native
    // declaration bytes agree with every retained parameter position.
    if arguments.names().len() != proc.params.len()
        || !arguments
            .names()
            .iter()
            .zip(&proc.params)
            .all(|(original, name)| original.try_utf8().ok() == Some(name.as_str()))
    {
        return false;
    }
    let [
        Statement::Return {
            expr: Some(expr), ..
        },
    ] = proc.body.statements.as_slice()
    else {
        return false;
    };
    let params: HashSet<&str> = proc.params.iter().map(String::as_str).collect();
    direct_expr_supported(expr, &params, unit.source_lexer_config())
}

fn initial_procedure_bindings(module: &Module, is_top: bool) -> Vec<(String, Binding)> {
    if is_top {
        return Vec::new();
    }
    module
        .procedures
        .keys()
        .map(|name| {
            (
                name.clone(),
                Binding {
                    kind: BindingKind::Proc,
                    target: Some(name.clone()),
                },
            )
        })
        .collect()
}

fn function_facts(
    unit: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    mutations: &ModuleCommandMutations,
    is_top: bool,
) -> FunctionFacts {
    let initial = initial_procedure_bindings(module, is_top);
    let context = unit.invocation_metadata_context_for_module(registry, module);
    let bindings = analyse_command_binding(&unit.cfg, registry, &initial);
    let mut facts = FunctionFacts::default();
    let mut planned_spans: HashSet<(u32, u32)> = HashSet::new();
    for block in unit.cfg.reverse_postorder() {
        let Some(cfg_block) = unit.cfg.blocks.get(&block) else {
            continue;
        };
        for (stmt_idx, statement) in cfg_block.statements.iter().enumerate() {
            // An analysis marker shares its host's span and is no command to
            // plan: counted as a second sighting of that span it would send
            // the host back to the source-span fallback.
            if crate::ssa::is_effect_marker(statement) || !statement.is_executable_invocation() {
                continue;
            }
            if let Statement::AssignConst {
                span,
                name,
                name_braced,
                ..
            } | Statement::AssignValue {
                span,
                name,
                name_braced,
                ..
            } = statement
                && (*name_braced || !crate::naming::is_dynamic_word(name))
                // `tcl_codegen_var_set` stores under the exact name, so an
                // array-element target — as opposed to a scalar whose name
                // merely contains parentheses — keeps the source-span
                // fallback. `cell_place` owns that reading for every backend.
                && matches!(
                    cell_place(name, *name_braced),
                    Some(CellPlace::Named { .. })
                )
                && assignment_operation_is_trusted(
                    unit,
                    context,
                    registry,
                    mutations,
                    &bindings,
                    (block, stmt_idx),
                    statement,
                )
                && let Some(value) = original_assignment_literal(unit, module, registry, statement)
            {
                facts.direct_assignments.insert(span_key(*span), value);
            }
            let (Statement::Call {
                command, tokens, ..
            }
            | Statement::Barrier {
                command, tokens, ..
            }) = statement
            else {
                continue;
            };
            if let Some(tokens) = tokens
                && let Some(context) = context
                && let Ok(RegistryInvocationResolution::Resolved(invocation)) =
                    resolve_command_tokens_with_metadata_context(registry, Some(context), tokens)
                && bindings.is_original_builtin_at(block, stmt_idx, command)
                && mutations.trusts(invocation.canonical_command.trim_start_matches("::"))
            {
                record_operation(
                    &mut facts,
                    module,
                    registry,
                    Some(context),
                    statement.span(),
                    tokens,
                );
            }
            // Generic prebuilt-argv invocation is the normal leaf-command path
            // and needs no binding proof: the runtime resolves the command head
            // exactly as interpreted Tcl does, so a rename, alias, ensemble, or
            // trace stays honoured. Only the *words* have to be provable.
            if let Statement::Call { tokens, .. } = statement {
                plan_leaf_statement(
                    &mut facts,
                    &mut planned_spans,
                    registry,
                    context,
                    unit.source_lexer_config(),
                    statement.span(),
                    tokens.as_ref(),
                );
            }
            if is_top {
                record_direct_calls(
                    &mut facts,
                    module,
                    mutations,
                    &bindings,
                    (block, stmt_idx),
                    statement,
                );
            }
        }
    }
    facts
}

/// The unchanged value operand uses its original point grammar, including
/// upstream AssignValue escape preservation. This supplies no store authority.
fn original_assignment_literal(
    unit: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    statement: &Statement,
) -> Option<String> {
    let value = match statement {
        Statement::AssignConst { value, .. } | Statement::AssignValue { value, .. } => value,
        _ => return None,
    };
    let tokens = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &unit.cfg.command_binding_sites,
        statement,
    )?;
    if tokens.argv_texts.get(2) != Some(value) {
        return None;
    }
    crate::type_infer::original_literal_argument_contents(registry, module, tokens, 1)
        .map(|literal| literal.value().to_owned())
}

/// A consumed assignment head keeps its original source selection and words.
/// Catalogue membership and flow binding alone cannot recreate that carrier.
fn assignment_operation_is_trusted(
    unit: &FunctionUnit,
    context: Option<InvocationMetadataContext<'_>>,
    registry: &CommandRegistry,
    mutations: &ModuleCommandMutations,
    bindings: &CommandBinding<'_>,
    site: (crate::cfg::BlockId, usize),
    statement: &Statement,
) -> bool {
    let Some(context) = context else {
        return false;
    };
    let Some(tokens) = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
        &unit.cfg.command_binding_sites,
        statement,
    )
    .filter(|tokens| {
        tokens.synthetic.is_none()
            && tokens.words_align_with_argv_text()
            && tokens
                .source_binding
                .as_ref()
                .and_then(crate::command_binding::SourceInvocationBinding::proved_execution_target)
                .is_some()
    }) else {
        return false;
    };
    let Ok(RegistryInvocationResolution::Resolved(invocation)) =
        resolve_command_tokens_with_metadata_context(registry, Some(context), tokens)
    else {
        return false;
    };
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        return false;
    };
    let Some(written_head) = tokens.argv_texts.first() else {
        return false;
    };
    invocation.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && effective.words.len() == 3
        && effective.words.len() == tokens.words().len()
        && !effective.origins.iter().any(|origin| {
            matches!(
                origin,
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
            )
        })
        && bindings.is_original_builtin_at(site.0, site.1, written_head)
        && mutations.trusts(invocation.canonical_command.trim_start_matches("::"))
}

/// Record every procedure this call site provably binds to, so the emitter may
/// call the generated function instead of dispatching through the runtime.
fn record_direct_calls(
    facts: &mut FunctionFacts,
    module: &Module,
    mutations: &ModuleCommandMutations,
    bindings: &CommandBinding<'_>,
    site: (crate::cfg::BlockId, usize),
    statement: &Statement,
) {
    let (block, stmt_idx) = site;
    for proc in module.procedures.values() {
        if proc.span.start() >= statement.span().start() {
            continue;
        }
        for written in [proc.name.as_str(), proc.qualified_name.as_str()] {
            let binding = bindings.binding_at(block, stmt_idx, written);
            if binding.kind == BindingKind::Proc
                && binding.target.as_deref() == Some(proc.qualified_name.as_str())
                && mutations.trusts_proc_binding(&proc.qualified_name)
                && !module.has_dynamic_trace
                && !module.traced_commands.contains(
                    proc.qualified_name
                        .strip_prefix("::")
                        .unwrap_or(&proc.qualified_name),
                )
            {
                facts.direct_calls.insert(
                    span_command_key(statement.span(), written),
                    proc.qualified_name.clone(),
                );
            }
        }
    }
}

/// Select and record the prebuilt-argv plan for one leaf `Statement::Call`.
///
/// A span is planned at most once. The control-flow graph can carry a
/// synthetic header call over the same source range as the statement the
/// structured walk emits (iterator headers do exactly that), and the two need
/// not describe the same argv, so a second sighting of a span poisons it back
/// to the source-span eval fallback rather than risk emitting the wrong argv.
fn plan_leaf_statement(
    facts: &mut FunctionFacts,
    planned_spans: &mut HashSet<(u32, u32)>,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
    span: Span,
    tokens: Option<&crate::ir::CommandTokens>,
) {
    let key = span_key(span);
    if !planned_spans.insert(key) {
        facts.leaf_invocations.remove(&key);
        facts
            .leaf_declines
            .insert(key, WasmLeafInvokeDecline::DuplicateStatementSpan);
        return;
    }
    let Some(tokens) = tokens.filter(|tokens| tokens.words_align_with_argv_text()) else {
        facts
            .leaf_declines
            .insert(key, WasmLeafInvokeDecline::MissingCommandTokens);
        return;
    };
    // Missing actual metadata keeps generic argv planning, without entering
    // the resolver's independently supported standalone catalogue mode.
    let resolution = context.map(|context| {
        resolve_command_tokens_with_metadata_context(registry, Some(context), tokens)
    });
    let resolved = match &resolution {
        Some(Ok(RegistryInvocationResolution::Resolved(invocation))) => Some(invocation.as_ref()),
        _ => None,
    };
    let operation = resolved.map_or(SemanticOperationId::Invoke, |facts| facts.operation);
    let selection_facts = resolved.map_or_else(SelectionFacts::unavailable, |facts| {
        SelectionFacts::from_invocation(facts)
    });
    match select_leaf_invocation(tokens.words(), operation, selection_facts, config) {
        Ok(plan) => {
            facts.leaf_invocations.insert(key, plan);
        }
        Err(decline) => {
            facts.leaf_declines.insert(
                key,
                super::leaf_invoke::word_decline(&decline)
                    .unwrap_or(WasmLeafInvokeDecline::MissingCommandTokens),
            );
        }
    }
}

/// Record the semantic operation a proven statement resolves to, and — for a
/// `proc` definition — whether this tier may register it itself.
///
/// The literal-definition verdict is decided here rather than at emit time
/// because this is where the statement's structured words are still to hand;
/// the emitter sees only spans and the surviving [`crate::ir::Procedure`].
fn record_operation(
    facts: &mut FunctionFacts,
    module: &Module,
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    span: Span,
    tokens: &crate::ir::CommandTokens,
) {
    let Some(context) = context else {
        return;
    };
    let Ok(RegistryInvocationResolution::Resolved(invocation)) =
        resolve_command_tokens_with_metadata_context(registry, Some(context), tokens)
    else {
        return;
    };
    // Compatibility codegen retains written argv. Registry positional hooks
    // require the argument correspondence proved by the shared projection.
    let Some(effective) = crate::registry_invocation::effective_command_words(tokens) else {
        return;
    };
    if effective.words.len() != tokens.words().len()
        || effective.origins.iter().any(|origin| {
            matches!(
                origin,
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
            )
        })
    {
        return;
    }
    facts
        .operations
        .insert(span_key(span), invocation.operation);
    if invocation.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Proc)
        && proc_definition_is_written_out(module, span, tokens)
    {
        facts.literal_proc_definitions.insert(span_key(span));
    }
}

/// Whether the `proc` statement at `span` wrote every word out, against the
/// [`crate::ir::Procedure`] that survived for it.
///
/// The general tier's half of the rule the native tier applies in
/// `native_lowering::lower::lower_definition`; both call the same predicate so
/// the two cannot drift.
fn proc_definition_is_written_out(
    module: &crate::ir::Module,
    span: Span,
    tokens: &crate::ir::CommandTokens,
) -> bool {
    let Some(procedure) = module
        .procedures
        .values()
        .find(|procedure| procedure.span == span)
    else {
        return false;
    };
    let Some(body_source) = procedure.body_source.as_deref() else {
        return false;
    };
    crate::native_lowering::lower::definition_words_are_written_out(
        tokens.words(),
        &procedure.params_raw,
        body_source,
    )
}

const fn abi_value_type(value: CodegenAbiValueType) -> ValType {
    match value {
        CodegenAbiValueType::I32 => ValType::I32,
        CodegenAbiValueType::I64 => ValType::I64,
        CodegenAbiValueType::F64 => ValType::F64,
    }
}

fn add_codegen_import(module: &mut WasmModule, import: CodegenAbiImportId) -> u32 {
    let descriptor = import.descriptor();
    let parameters = descriptor
        .parameters
        .iter()
        .copied()
        .map(abi_value_type)
        .collect::<Vec<_>>();
    let results = descriptor
        .results
        .iter()
        .copied()
        .map(abi_value_type)
        .collect::<Vec<_>>();
    u32::try_from(module.add_import(descriptor.module, descriptor.name, &parameters, &results))
        .expect("WASM import index fits u32")
}

fn general_import_id(imports: Imports, index: u32) -> Option<CodegenAbiImportId> {
    let mut entries = vec![
        (
            imports.host_refusal_pending,
            CodegenAbiImportId::HostRefusalPending,
        ),
        (imports.obj_new_string, CodegenAbiImportId::ObjectNewString),
        (imports.eval_code, CodegenAbiImportId::EvalCode),
        (
            imports.condition.frame_alloc,
            CodegenAbiImportId::CallFrameAlloc,
        ),
        (
            imports.condition.frame_free,
            CodegenAbiImportId::CallFrameFree,
        ),
        (
            imports.condition.string_owned,
            CodegenAbiImportId::NewOwnedString,
        ),
        (
            imports.condition.expr_bool_eval,
            CodegenAbiImportId::ExprBoolEval,
        ),
        (
            imports.condition.completion_release,
            CodegenAbiImportId::CompletionRelease,
        ),
        (
            imports.condition.object_release,
            CodegenAbiImportId::ObjectRelease,
        ),
    ];
    if let Some(aot) = imports.aot {
        entries.push((aot.value_new_string, CodegenAbiImportId::ValueNewString));
        entries.push((aot.frame_push, CodegenAbiImportId::FramePush));
        entries.push((aot.frame_pop, CodegenAbiImportId::FramePop));
        entries.push((aot.local_bind, CodegenAbiImportId::LocalBind));
        entries.push((aot.local_set, CodegenAbiImportId::LocalSet));
        entries.push((aot.local_get, CodegenAbiImportId::LocalGet));
        entries.push((aot.var_set, CodegenAbiImportId::VarSet));
        entries.push((aot.var_get, CodegenAbiImportId::VarGet));
        entries.push((aot.expr_add, CodegenAbiImportId::ExprAdd));
        entries.push((aot.puts, CodegenAbiImportId::Puts));
        entries.push((aot.proc_register, CodegenAbiImportId::ProcRegister));
        entries.push((aot.argv.frame_alloc, CodegenAbiImportId::CallFrameAlloc));
        entries.push((aot.argv.frame_free, CodegenAbiImportId::CallFrameFree));
        entries.push((aot.argv.string_owned, CodegenAbiImportId::NewOwnedString));
        entries.push((aot.argv.invoke_argv, CodegenAbiImportId::InvokeArgv));
        entries.push((aot.argv.object_release, CodegenAbiImportId::ObjectRelease));
        entries.push((aot.argv.var_get, CodegenAbiImportId::VarGet));
        entries.push((aot.argv.var_get_element, CodegenAbiImportId::VarGetElement));
        entries.push((aot.argv.word_concat, CodegenAbiImportId::WordConcat));
    }
    entries
        .into_iter()
        .find_map(|(candidate, id)| (candidate == index).then_some(id))
}

fn add_semantic_imports(wasm: &mut WasmModule) -> SemanticImports {
    SemanticImports {
        host_refusal_pending: add_codegen_import(wasm, CodegenAbiImportId::HostRefusalPending),
        frame_alloc: add_codegen_import(wasm, CodegenAbiImportId::CallFrameAlloc),
        frame_free: add_codegen_import(wasm, CodegenAbiImportId::CallFrameFree),
        string_owned: add_codegen_import(wasm, CodegenAbiImportId::NewOwnedString),
        invoke_argv: add_codegen_import(wasm, CodegenAbiImportId::InvokeArgv),
        completion_release: add_codegen_import(wasm, CodegenAbiImportId::CompletionRelease),
        object_retain: add_codegen_import(wasm, CodegenAbiImportId::ObjectRetain),
        object_release: add_codegen_import(wasm, CodegenAbiImportId::ObjectRelease),
    }
}

fn add_guarded_intrinsic_imports(wasm: &mut WasmModule) -> GuardedIntrinsicImports {
    GuardedIntrinsicImports {
        semantic: add_semantic_imports(wasm),
        guard_prepare: add_codegen_import(wasm, CodegenAbiImportId::GuardPrepare),
        guard_check: add_codegen_import(wasm, CodegenAbiImportId::GuardCheck),
        guard_release: add_codegen_import(wasm, CodegenAbiImportId::GuardRelease),
        invoke_intrinsic_argv: add_codegen_import(wasm, CodegenAbiImportId::InvokeIntrinsicArgv),
    }
}

fn add_native_i64_add_imports(wasm: &mut WasmModule) -> NativeI64AddImports {
    NativeI64AddImports {
        host_refusal_pending: add_codegen_import(wasm, CodegenAbiImportId::HostRefusalPending),
        value_new_wide_int: add_codegen_import(wasm, CodegenAbiImportId::ValueNewWideInt),
        puts: add_codegen_import(wasm, CodegenAbiImportId::Puts),
    }
}

fn add_general_imports(wasm: &mut WasmModule, analysis: bool) -> Imports {
    let mut imports = Imports {
        host_refusal_pending: add_codegen_import(wasm, CodegenAbiImportId::HostRefusalPending),
        obj_new_string: add_codegen_import(wasm, CodegenAbiImportId::ObjectNewString),
        eval_code: add_codegen_import(wasm, CodegenAbiImportId::EvalCode),
        condition: ConditionImports {
            frame_alloc: add_codegen_import(wasm, CodegenAbiImportId::CallFrameAlloc),
            frame_free: add_codegen_import(wasm, CodegenAbiImportId::CallFrameFree),
            string_owned: add_codegen_import(wasm, CodegenAbiImportId::NewOwnedString),
            expr_bool_eval: add_codegen_import(wasm, CodegenAbiImportId::ExprBoolEval),
            completion_release: add_codegen_import(wasm, CodegenAbiImportId::CompletionRelease),
            object_release: add_codegen_import(wasm, CodegenAbiImportId::ObjectRelease),
        },
        aot: None,
    };
    if analysis {
        imports.aot = Some(add_aot_imports(wasm));
    }
    imports
}

/// The prebuilt-argv surface the general tier's normal leaf-command path uses.
///
/// `var_get` is shared with the direct scalar-read specialisation; everything
/// else is the shared code-generation ABI's own transport surface.
fn add_argv_imports(wasm: &mut WasmModule, var_get: u32) -> ArgvImports {
    ArgvImports {
        frame_alloc: add_codegen_import(wasm, CodegenAbiImportId::CallFrameAlloc),
        frame_free: add_codegen_import(wasm, CodegenAbiImportId::CallFrameFree),
        string_owned: add_codegen_import(wasm, CodegenAbiImportId::NewOwnedString),
        invoke_argv: add_codegen_import(wasm, CodegenAbiImportId::InvokeArgv),
        object_release: add_codegen_import(wasm, CodegenAbiImportId::ObjectRelease),
        var_get,
        var_get_element: add_codegen_import(wasm, CodegenAbiImportId::VarGetElement),
        word_concat: add_codegen_import(wasm, CodegenAbiImportId::WordConcat),
    }
}

fn add_aot_imports(wasm: &mut WasmModule) -> AotImports {
    let mut imports = AotImports {
        value_new_string: add_codegen_import(wasm, CodegenAbiImportId::ValueNewString),
        frame_push: add_codegen_import(wasm, CodegenAbiImportId::FramePush),
        frame_pop: add_codegen_import(wasm, CodegenAbiImportId::FramePop),
        local_bind: add_codegen_import(wasm, CodegenAbiImportId::LocalBind),
        local_set: add_codegen_import(wasm, CodegenAbiImportId::LocalSet),
        local_get: add_codegen_import(wasm, CodegenAbiImportId::LocalGet),
        var_set: add_codegen_import(wasm, CodegenAbiImportId::VarSet),
        var_get: add_codegen_import(wasm, CodegenAbiImportId::VarGet),
        expr_add: add_codegen_import(wasm, CodegenAbiImportId::ExprAdd),
        puts: add_codegen_import(wasm, CodegenAbiImportId::Puts),
        proc_register: add_codegen_import(wasm, CodegenAbiImportId::ProcRegister),
        argv: ArgvImports::default(),
    };
    imports.argv = add_argv_imports(wasm, imports.var_get);
    imports
}

struct ProcedurePlan<'a> {
    procs: Vec<&'a Procedure>,
    indices: HashMap<String, u32>,
    arity: HashMap<String, usize>,
    by_span: HashMap<(u32, u32), Procedure>,
    direct: HashSet<String>,
}

fn procedure_plan<'a>(
    module: &'a Module,
    analysis: Option<(&CompilationUnit, &CommandRegistry)>,
    top_idx: u32,
) -> ProcedurePlan<'a> {
    let mut procs: Vec<&Procedure> = module
        .procedures
        .values()
        .filter(|p| {
            !p.namespace_scoped
                && !crate::native_compilation_admission::script_requires_admission(&p.body)
        })
        .collect();
    procs.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
    let indices = procs
        .iter()
        .enumerate()
        .map(|(position, proc)| {
            (
                proc.qualified_name.clone(),
                top_idx
                    .saturating_add(1)
                    .saturating_add(u32::try_from(position).unwrap_or(u32::MAX)),
            )
        })
        .collect();
    let arity = procs
        .iter()
        .map(|proc| (proc.qualified_name.clone(), proc.params.len()))
        .collect();
    let by_span = procs
        .iter()
        .map(|proc| (span_key(proc.span), (*proc).clone()))
        .collect();
    let direct = analysis.map_or_else(HashSet::new, |(unit, registry)| {
        procs
            .iter()
            .filter(|proc| {
                unit.procedures.get(&proc.qualified_name).is_some_and(|fu| {
                    direct_proc_eligible(module, proc, fu, registry, &unit.command_mutations)
                })
            })
            .map(|proc| proc.qualified_name.clone())
            .collect()
    });
    ProcedurePlan {
        procs,
        indices,
        arity,
        by_span,
        direct,
    }
}

/// Selected input mode for the single module emitter.
#[derive(Clone, Copy)]
pub(super) enum WasmEmissionMode<'a> {
    /// The genuine host must compile one entire original script at entry.
    /// `None` explicitly refuses admission instead of evaluating empty text.
    RuntimeChunk(Option<&'a crate::command_binding::ExecutedScriptSource>),
    /// Common proofs selected sealed-program native i64 addition with one
    /// registry-proved boxed output boundary.
    NativeI64Add(&'a WasmNativeI64AddSelection),
    /// `BackendRegistry` selected one prebuilt-argv semantic invocation.
    SemanticInvoke(&'a WasmGenericInvokePlan),
    /// Common analysis selected a guarded boxed intrinsic over the semantic
    /// invocation's exact prebuilt argv and generic fallback.
    GuardedIntrinsic {
        /// The sole semantic invocation retaining literal argv ownership.
        plan: &'a WasmGenericInvokePlan,
        /// Common proof and guard request selected for that invocation.
        evidence: &'a GuardedSelectionEvidence,
    },
    /// General structured lowering, retaining a typed semantic decline in the
    /// outer [`super::WasmCompilation`] evidence.
    General,
}

/// Internal emitter behind the canonical [`super::compile_wasm`] pipeline.
///
/// Packaging flags alter relocation and bootstrap only. Direct
/// specialisations may be conservatively disabled for a restricted test host;
/// unsupported statements always fall back inside this emitter.
pub(super) fn emit_wasm(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    options: WasmCompileOptions,
    mode: WasmEmissionMode<'_>,
) -> (WasmModule, NativeTierReport) {
    use crate::native_compilation_admission::{
        NativeCompilationAdmissionPlan, NativeCompilationAdmissionScope, script_admission_plan,
    };
    let mode = match script_admission_plan(
        &unit.ir_module.top_level,
        NativeCompilationAdmissionScope::Script,
    ) {
        NativeCompilationAdmissionPlan::HostScript(source) => {
            WasmEmissionMode::RuntimeChunk(Some(source))
        }
        NativeCompilationAdmissionPlan::RefuseMissingSource => WasmEmissionMode::RuntimeChunk(None),
        NativeCompilationAdmissionPlan::NoRetainedObligation => mode,
        NativeCompilationAdmissionPlan::HostProcedure(_) => unreachable!("selected script scope"),
    };
    let analysis = (matches!(mode, WasmEmissionMode::General)
        && options.analysis_specialisations())
    .then_some((unit, registry));
    let native = (matches!(mode, WasmEmissionMode::General) && options.native_tier_enabled())
        .then_some(NativeTier {
            unit,
            registry,
            config: options.semantic_optimisations(),
        });
    let (mut module, mut report) = codegen(
        &unit.ir_module,
        &unit.source,
        options.data_base,
        options.is_standalone(),
        options.initialise_library(),
        analysis,
        native,
        mode,
    );
    report.enabled = options.native_tier_enabled();
    module.manifest = Some(module_manifest(unit));
    (module, report)
}

/// What the module says about the world it was compiled for: the context of
/// the profile the unit's dialect resolves to and this build's intrinsic
/// table. A WASM site records no pack claim — a guarded fast path is checked
/// against the live command on every call — so the module's packs are none.
fn module_manifest(unit: &CompilationUnit) -> tcl_runtime_api::ArtefactIdentityManifest {
    let profile = unit
        .ir_module
        .resolved_profile()
        .unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl);
    tcl_registry::model::runtime_context_for_profile(profile)
        .identity(&[], tcl_registry::intrinsic_table_hash())
}

/// The native tier's inputs, present only when the pipeline selected it.
#[derive(Clone, Copy)]
struct NativeTier<'a> {
    unit: &'a CompilationUnit,
    registry: &'a CommandRegistry,
    config: SemanticOptimisationConfig,
}

impl NativeTier<'_> {
    /// Lower one function unit through NLIR, or the typed reason it stays on
    /// the legacy structured path.
    fn lower(
        &self,
        function: &FunctionUnit,
        top_level: bool,
        line_origin: u32,
    ) -> Result<(crate::native_lowering::ir::NativeFunction, FunctionReport), FunctionDecline> {
        let facts = &function.semantic_facts;
        let executable = facts
            .executable()
            .function()
            .ok_or(FunctionDecline::NoExecutableFunction)?;
        let hints = std::collections::BTreeMap::new();
        let input = LoweringInput {
            registry: self.registry,
            context: facts.context(),
            function: executable,
            source: &self.unit.source,
            module: &self.unit.ir_module,
            mutations: &self.unit.command_mutations,
            config: self.config,
            escape: None,
            top_level,
            line_origin,
            entry_assumption: facts.dispatch_entry_assumption(),
            type_hints: &hints,
        };
        lower_function(&input)
    }
}

/// One procedure body's native lowering, held until the module's table window
/// is known: `::top` cannot be emitted before the binding set is decided, and
/// a body is only worth lowering once.
enum LoweredProc {
    /// The body lowered; `decline` says why it may nonetheless not be bound
    /// as the procedure's entry.
    Lowered {
        function: crate::native_lowering::ir::NativeFunction,
        report: FunctionReport,
        decline: Option<ProcEntryDecline>,
    },
    /// The tier declined the body outright.
    Declined(FunctionDecline),
}

/// Every procedure a lowered function defines through the native definition
/// shape, which is the exact set that can read a table entry.
fn collect_defined_procs(
    function: &crate::native_lowering::ir::NativeFunction,
    out: &mut HashSet<String>,
) {
    fn walk(ops: &[crate::native_lowering::ir::NativeOp], out: &mut HashSet<String>) {
        use crate::native_lowering::ir::NativeOp;
        for op in ops {
            match op {
                NativeOp::DefineProc { qualified_name, .. } => {
                    out.insert(qualified_name.clone());
                }
                NativeOp::IfElse {
                    then_ops, else_ops, ..
                } => {
                    walk(then_ops, out);
                    walk(else_ops, out);
                }
                _ => {}
            }
        }
    }
    for block in &function.blocks {
        for statement in &block.statements {
            walk(&statement.ops, out);
        }
    }
}

/// `::top`'s table-install prologue: grow the runtime's shared function table
/// by this module's entry count, keep the base in the module's `$table_base`
/// global, and write one `ref.func` per bound procedure.
///
/// Guarded on the global, so a `::top` that runs twice — a host calling it
/// after `_start` already did — installs once and keeps the same indices. A
/// `table.grow` answering `-1` is a runtime linked without
/// `--growable-table`: the module traps at its entry point with a clear
/// backtrace rather than dispatching to whatever sits at slot 0.
fn table_install(
    bound: &BTreeMap<String, u32>,
    indices: &HashMap<String, u32>,
    base_global: u32,
) -> Vec<WasmInstruction> {
    if bound.is_empty() {
        return Vec::new();
    }
    let global =
        |op: WasmOp| WasmInstruction::with_operands(op, leb128_unsigned(u64::from(base_global)));
    let constant =
        |value: i64| WasmInstruction::with_operands(WasmOp::I32Const, leb128_signed(value));
    let mut ops = vec![
        global(WasmOp::GlobalGet),
        constant(0),
        WasmInstruction::new(WasmOp::I32LtS),
        WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
        WasmInstruction::ref_null_func(),
        constant(i64::try_from(bound.len()).unwrap_or(i64::MAX)),
        WasmInstruction::table_grow(0),
        global(WasmOp::GlobalSet),
        global(WasmOp::GlobalGet),
        constant(0),
        WasmInstruction::new(WasmOp::I32LtS),
        WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
        WasmInstruction::new(WasmOp::Unreachable),
        WasmInstruction::new(WasmOp::End),
    ];
    for (name, slot) in bound {
        let Some(index) = indices.get(name).copied() else {
            continue;
        };
        ops.push(global(WasmOp::GlobalGet));
        ops.push(constant(i64::from(*slot)));
        ops.push(WasmInstruction::new(WasmOp::I32Add));
        ops.push(WasmInstruction::ref_func(index));
        ops.push(WasmInstruction::table_set(0));
    }
    ops.push(WasmInstruction::new(WasmOp::End));
    ops
}

/// Host admission executes the whole source before any compiled effects,
/// exports or procedure-table installation can bypass its compiler entry.
fn codegen_runtime_chunk(
    chunk: Option<&crate::command_binding::ExecutedScriptSource>,
    data_base: i64,
    standalone: bool,
    init: bool,
) -> (WasmModule, NativeTierReport) {
    let mut wasm = WasmModule::new();
    let imports = add_general_imports(&mut wasm, false);
    let bootstrap = standalone.then(|| {
        let create = add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeCreateInterp);
        let set_current =
            add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeSetCurrentInterp);
        let library =
            init.then(|| add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeInitLibrary));
        (create, set_current, library)
    });
    let top_index = u32::try_from(wasm.imports.len()).expect("import count fits u32");
    let mut emitter = WasmEmitter {
        imports: EmitterImports::General(imports),
        body: Vec::new(),
        data: Vec::new(),
        data_offset: data_base,
        ctrl_depth: 0,
        invocation_abort: None,
        loops: Vec::new(),
        code_local: 0,
        frame_local: 1,
        mode: FunctionMode::Top,
        local_slots: HashMap::new(),
        proc_indices: HashMap::new(),
        procedure_arity: HashMap::new(),
        direct_procs: HashSet::new(),
        procedures_by_span: HashMap::new(),
        facts: FunctionFacts::default(),
    };
    if let Some(chunk) = chunk {
        emitter.emit_command_image(&chunk.text);
    } else {
        emitter.refuse_native_compilation_admission();
    }
    wasm.functions
        .push(emitter.finish_function("::top", "host-compilation-entry", None));
    wasm.data_segments = emitter.data;
    if let Some((create, set_current, library)) = bootstrap {
        wasm.functions
            .push(start_function(create, set_current, library, top_index));
    }
    let mut report = NativeTierReport::default();
    report.functions.insert(
        "::top".to_owned(),
        FunctionReport::declined(FunctionDecline::NativeCompilationAdmissionRequired),
    );
    (wasm, report)
}

/// Shared implementation for hosted, linked, and standalone packaging.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn codegen(
    module: &Module,
    source: &str,
    data_base: i64,
    standalone: bool,
    init: bool,
    analysis: Option<(&CompilationUnit, &CommandRegistry)>,
    native: Option<NativeTier<'_>>,
    mode: WasmEmissionMode<'_>,
) -> (WasmModule, NativeTierReport) {
    if let WasmEmissionMode::RuntimeChunk(chunk) = mode {
        return codegen_runtime_chunk(chunk, data_base, standalone, init);
    }
    let mut wasm = WasmModule::new();
    let mut report = NativeTierReport {
        enabled: native.is_some(),
        functions: std::collections::BTreeMap::new(),
    };
    if emit_special_mode(&mut wasm, data_base, mode) {
        return (wasm, report);
    }
    let imports = add_general_imports(&mut wasm, analysis.is_some());
    let native_imports: Option<NativeImports> =
        native.map(|_| native_emit::add_native_imports(&mut wasm, &mut add_codegen_import));

    // Standalone: the interp-bootstrap imports `_start` drives. Added after the
    // ABI imports so the ABI indices in `Imports` are unchanged.
    let bootstrap = standalone.then(|| {
        let create = add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeCreateInterp);
        let set_current =
            add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeSetCurrentInterp);
        let init_library =
            init.then(|| add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeInitLibrary));
        (create, set_current, init_library)
    });

    // `::top` is the first *defined* function, so its call index is the import
    // count (imports occupy the low indices). Capture it before emitting bodies.
    let top_idx = u32::try_from(wasm.imports.len()).expect("import count fits in u32");

    let ProcedurePlan {
        procs,
        indices: proc_indices,
        arity: procedure_arity,
        by_span: procedures_by_span,
        direct: direct_procs,
    } = procedure_plan(module, analysis, top_idx);
    let top_facts = analysis.map_or_else(FunctionFacts::default, |(unit, registry)| {
        function_facts(
            &unit.top_level,
            module,
            registry,
            &unit.command_mutations,
            true,
        )
    });

    let mut emitter = WasmEmitter {
        imports: EmitterImports::General(imports),
        body: Vec::new(),
        data: Vec::new(),
        data_offset: data_base,
        ctrl_depth: 0,
        invocation_abort: None,
        loops: Vec::new(),
        code_local: 0,
        frame_local: 1,
        mode: FunctionMode::Top,
        local_slots: HashMap::new(),
        proc_indices,
        procedure_arity,
        direct_procs,
        procedures_by_span,
        facts: top_facts,
    };
    // Every procedure body is lowered *before* `::top` is emitted: which of
    // them lowered decides the module's window in the runtime's shared
    // function table, and `::top` both grows that window and binds each entry
    // to its definition, so it cannot be emitted without knowing.
    let lowered_top = native.map(|tier| tier.lower(&tier.unit.top_level, true, 0));
    let lowered_procs: Vec<Option<LoweredProc>> = procs
        .iter()
        .map(|proc| {
            let tier = native?;
            let unit = tier.unit.procedures.get(&proc.qualified_name)?;
            Some(match tier.lower(unit, false, proc.body_offset) {
                Ok((function, report)) => LoweredProc::Lowered {
                    decline: native_emit::proc_entry_decline(&function),
                    function,
                    report,
                },
                Err(reason) => LoweredProc::Declined(reason),
            })
        })
        .collect();
    // Only a definition statement the lowering actually took natively can
    // carry an entry, and the table window must hold exactly those: a slot
    // nothing binds would import the runtime's table for nothing.
    let mut defined: HashSet<String> = HashSet::new();
    if let Some(Ok((function, _))) = &lowered_top {
        collect_defined_procs(function, &mut defined);
    }
    for lowered in &lowered_procs {
        if let Some(LoweredProc::Lowered { function, .. }) = lowered {
            collect_defined_procs(function, &mut defined);
        }
    }
    // A binding is only reachable through `::top`'s install sequence, so a
    // module whose entry point stayed on the legacy path installs nothing and
    // every definition it emits is source-only.
    let bound: BTreeMap<String, u32> = if matches!(lowered_top, Some(Ok(_))) {
        procs
            .iter()
            .zip(&lowered_procs)
            .filter(|(proc, lowered)| {
                matches!(lowered, Some(LoweredProc::Lowered { decline: None, .. }))
                    && defined.contains(&proc.qualified_name)
            })
            .enumerate()
            .map(|(slot, (proc, _))| {
                (
                    proc.qualified_name.clone(),
                    u32::try_from(slot).unwrap_or(u32::MAX),
                )
            })
            .collect()
    } else {
        BTreeMap::new()
    };
    // The table import is opt-in per module: a runtime linked without
    // `--export-table` cannot satisfy it at all, so a module that installs
    // nothing must not ask for it.
    let table_base_global = u32::try_from(wasm.globals.len()).unwrap_or(u32::MAX);
    if !bound.is_empty() {
        wasm.import_table = true;
        wasm.globals.push(WasmGlobal {
            name: "$table_base".to_owned(),
            mutable: true,
            init: GlobalInit::I32(-1),
        });
        wasm.elem_declared = bound
            .keys()
            .filter_map(|name| emitter.proc_indices.get(name).copied())
            .collect();
        wasm.elem_declared.sort_unstable();
    }
    let table = native_emit::EntryTable {
        slots: &bound,
        base_global: table_base_global,
    };

    // The top-level script: the native tier when it lowers, else the legacy
    // structured walk with the typed reason recorded.
    match (lowered_top, native_imports) {
        (Some(Ok((function, function_report))), Some(native_imports)) => {
            report.functions.insert("::top".to_owned(), function_report);
            let mut top = native_emit::emit_function(
                "::top",
                "native-top",
                &function,
                native_imports,
                table,
                ConstantPool {
                    data: &mut emitter.data,
                    offset: &mut emitter.data_offset,
                },
            );
            let install = table_install(&bound, &emitter.proc_indices, table_base_global);
            top.body.splice(0..0, install);
            wasm.functions.push(top);
        }
        (lowered, _) => {
            if let Some(Err(reason)) = lowered {
                report
                    .functions
                    .insert("::top".to_owned(), FunctionReport::declined(reason));
            }
            structured::walk(&mut emitter, &module.top_level, source);
            let top = emitter.finish_function("::top", "top", None);
            wasm.functions.push(top);
        }
    }

    // Each user-defined proc body becomes its own WASM function, driven through
    // the same structured walk (its body is already lowered IR with absolute
    // source spans). Namespace-scoped procs are created at run time inside
    // `namespace eval`, not at load, so they are skipped — mirroring the bytecode
    // backend (`codegen/emitter/mod.rs`). Emitted in qualified-name order so the
    // module bytes are deterministic (`procedures` is a hash map).
    for (proc, lowered) in procs.iter().zip(lowered_procs) {
        match (lowered, native_imports) {
            (
                Some(LoweredProc::Lowered {
                    function,
                    report: mut function_report,
                    decline,
                }),
                Some(native_imports),
            ) => {
                function_report.binding = match (decline, bound.contains_key(&proc.qualified_name))
                {
                    (None, true) => NativeBinding::BoundNatively,
                    (Some(reason), _) => NativeBinding::SourceOnly(reason),
                    (None, false) => NativeBinding::NotApplicable,
                };
                report
                    .functions
                    .insert(proc.qualified_name.clone(), function_report);
                let func = native_emit::emit_function(
                    &proc.qualified_name,
                    "native-proc",
                    &function,
                    native_imports,
                    table,
                    ConstantPool {
                        data: &mut emitter.data,
                        offset: &mut emitter.data_offset,
                    },
                );
                wasm.functions.push(func);
                continue;
            }
            (Some(LoweredProc::Declined(reason)), _) => {
                report.functions.insert(
                    proc.qualified_name.clone(),
                    FunctionReport::declined(reason),
                );
            }
            _ => {}
        }
        let direct = emitter.direct_procs.contains(&proc.qualified_name);
        emitter.mode = if direct {
            FunctionMode::DirectProc
        } else {
            FunctionMode::FallbackProc
        };
        emitter.local_slots = if direct {
            proc.params
                .iter()
                .enumerate()
                .map(|(slot, name)| (name.clone(), u32::try_from(slot).unwrap_or(u32::MAX)))
                .collect()
        } else {
            HashMap::new()
        };
        emitter.code_local = if direct {
            u64::try_from(proc.params.len()).unwrap_or(u64::MAX)
        } else {
            0
        };
        emitter.frame_local = emitter.code_local.saturating_add(1);
        emitter.facts = analysis
            .and_then(|(unit, registry)| {
                unit.procedures
                    .get(&proc.qualified_name)
                    .map(|fu| function_facts(fu, module, registry, &unit.command_mutations, false))
            })
            .unwrap_or_default();
        if direct {
            emitter.emit_proc_prelude(proc);
        }
        structured::walk(&mut emitter, &proc.body, source);
        let func = emitter.finish_function(&proc.qualified_name, "proc", Some(proc));
        wasm.functions.push(func);
    }

    wasm.data_segments = emitter.data;

    if let Some((create, set_current, init_library)) = bootstrap {
        wasm.functions
            .push(start_function(create, set_current, init_library, top_idx));
    }

    (wasm, report)
}

fn emit_special_mode(wasm: &mut WasmModule, data_base: i64, mode: WasmEmissionMode<'_>) -> bool {
    match mode {
        WasmEmissionMode::NativeI64Add(plan) => {
            emit_native_i64_add(wasm, data_base, plan);
        }
        WasmEmissionMode::SemanticInvoke(plan) => {
            let imports = add_semantic_imports(wasm);
            let mut emitter = WasmEmitter::for_semantic_invoke(imports, data_base);
            wasm.functions.push(emitter.finish_semantic_invoke(plan));
            wasm.memory_pages = required_pages(data_base, &emitter.data);
            wasm.data_segments = emitter.data;
        }
        WasmEmissionMode::GuardedIntrinsic { plan, evidence } => {
            let imports = add_guarded_intrinsic_imports(wasm);
            let mut emitter = WasmEmitter::for_guarded_intrinsic_invoke(imports, data_base);
            wasm.functions
                .push(emitter.finish_guarded_intrinsic_invoke(plan, evidence));
            wasm.memory_pages = required_pages(data_base, &emitter.data);
            wasm.data_segments = emitter.data;
        }
        WasmEmissionMode::General | WasmEmissionMode::RuntimeChunk(_) => return false,
    }
    true
}

fn emit_native_i64_add(wasm: &mut WasmModule, data_base: i64, plan: &WasmNativeI64AddSelection) {
    let imports = add_native_i64_add_imports(wasm);
    let top_index = u32::try_from(wasm.imports.len()).expect("import count fits in u32");
    let add_index = top_index
        .checked_add(1)
        .expect("native function index fits u32");
    wasm.functions.push(WasmFunction {
        name: "::top".to_owned(),
        params: Vec::new(),
        results: Vec::new(),
        locals: vec![ValType::I32],
        body: vec![
            WasmInstruction::with_operands(WasmOp::I64Const, leb128_signed(plan.left)),
            WasmInstruction::with_operands(WasmOp::I64Const, leb128_signed(plan.right)),
            WasmInstruction::with_operands(WasmOp::Call, leb128_unsigned(u64::from(add_index))),
            WasmInstruction::with_operands(
                WasmOp::Call,
                leb128_unsigned(u64::from(imports.value_new_wide_int)),
            ),
            WasmInstruction::with_operands(WasmOp::Call, leb128_unsigned(u64::from(imports.puts))),
            // Match structured lowering: a non-OK `puts` completion stops
            // the top-level script instead of being silently discarded.
            WasmInstruction::with_operands(WasmOp::LocalSet, leb128_unsigned(0)),
            WasmInstruction::with_operands(
                WasmOp::Call,
                leb128_unsigned(u64::from(imports.host_refusal_pending)),
            ),
            WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
            WasmInstruction::new(WasmOp::Return),
            WasmInstruction::new(WasmOp::End),
            WasmInstruction::with_operands(WasmOp::LocalGet, leb128_unsigned(0)),
            WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
            WasmInstruction::new(WasmOp::Return),
            WasmInstruction::new(WasmOp::End),
            WasmInstruction::new(WasmOp::Return),
        ],
        local_names: vec!["$completion_code".to_owned()],
        exported: true,
        source_range: None,
        kind: "native-i64-add-top".to_owned(),
    });
    wasm.functions.push(WasmFunction {
        name: plan.callee.qualified_name.clone(),
        params: vec![ValType::I64, ValType::I64],
        results: vec![ValType::I64],
        locals: Vec::new(),
        body: vec![
            WasmInstruction::with_operands(WasmOp::LocalGet, leb128_unsigned(0)),
            WasmInstruction::with_operands(WasmOp::LocalGet, leb128_unsigned(1)),
            WasmInstruction::new(WasmOp::I64Add),
            WasmInstruction::new(WasmOp::Return),
        ],
        local_names: vec!["$left".to_owned(), "$right".to_owned()],
        // The raw i64 function is an implementation detail of the closed
        // proof. Only the boxed Tcl entry boundary is externally callable.
        exported: false,
        source_range: None,
        kind: "native-i64-add-proc".to_owned(),
    });
    wasm.memory_pages = required_pages(data_base, &[]);
}

fn required_pages(data_base: i64, data: &[WasmData]) -> u64 {
    let end = data.iter().fold(data_base, |largest, segment| {
        let segment_end = segment
            .offset
            .checked_add(i64::try_from(segment.data.len()).expect("literal length fits i64"))
            .expect("semantic plan validated constant-pool bounds");
        largest.max(segment_end)
    });
    let pages = end
        .checked_add(65_535)
        .expect("semantic plan validated page calculation")
        / 65_536;
    u64::try_from(pages.max(1)).expect("validated wasm32 page count")
}

/// The `_start` WASI-command entry:
/// `set_current_interp(create_interp()); [init_library();] ::top()`.
/// `finish_function`'s usual trailing `end` is appended here by hand since this
/// body is built directly rather than via the structured walk.
fn start_function(
    create_interp: u32,
    set_current_interp: u32,
    init_library: Option<u32>,
    top_idx: u32,
) -> WasmFunction {
    let call =
        |idx: u32| WasmInstruction::with_operands(WasmOp::Call, leb128_unsigned(u64::from(idx)));
    // create_interp() leaves the interp ptr on the stack; set_current_interp
    // consumes it; the optional init_library() bootstraps the stdlib (its i32
    // status is discarded); ::top runs against the now-current, initialised interp.
    let mut body = vec![call(create_interp), call(set_current_interp)];
    if let Some(init) = init_library {
        body.push(call(init));
        body.push(WasmInstruction::new(WasmOp::Drop));
    }
    body.push(call(top_idx));
    body.push(WasmInstruction::new(WasmOp::End));
    WasmFunction {
        name: "_start".to_string(),
        params: Vec::new(),
        results: Vec::new(),
        locals: Vec::new(),
        body,
        local_names: Vec::new(),
        exported: true,
        source_range: None,
        kind: "start".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plan the top-level statements of `source` under the full analysed tier.
    fn top_level_facts(source: &str) -> FunctionFacts {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let unit = retained_source_unit(
            std::sync::Arc::clone(&context),
            source,
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        function_facts(
            &unit.top_level,
            &unit.ir_module,
            context.commands(),
            &unit.command_mutations,
            true,
        )
    }

    fn retained_source_unit(
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
        source: &str,
        config: tcl_lexer::LexerConfig,
    ) -> crate::environment_ingress::RetainedNativeUnit {
        native_source_unit(
            context,
            source,
            config,
            tcl_dialect::DialectProfile::find("tcl8.6").unwrap(),
        )
    }

    fn native_source_unit(
        context: std::sync::Arc<tcl_registry::model::ContextRegistry>,
        source: &str,
        config: tcl_lexer::LexerConfig,
        profile: &'static tcl_dialect::DialectProfile,
    ) -> crate::environment_ingress::RetainedNativeUnit {
        let (owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = CompilationUnit::build_with_context_registry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&context),
        );
        crate::environment_ingress::RetainedNativeUnit::new(unit, owner)
    }

    fn assignment_facts(
        function: &FunctionUnit,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> FunctionFacts {
        function_facts(
            function,
            &unit.ir_module,
            registry,
            &unit.command_mutations,
            true,
        )
    }

    fn original_assignment_diagnostics(
        function: &FunctionUnit,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> String {
        let metadata = function.invocation_metadata_context_for_module(registry, &unit.ir_module);
        let initial = initial_procedure_bindings(&unit.ir_module, true);
        let bindings = analyse_command_binding(&function.cfg, registry, &initial);
        let mut rows = vec![format!("metadata available: {}", metadata.is_some())];
        for (&block, body) in &function.cfg.blocks {
            for (index, statement) in body.statements.iter().enumerate() {
                if !matches!(
                    statement,
                    Statement::AssignConst { .. } | Statement::AssignValue { .. }
                ) {
                    continue;
                }
                let tokens = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                    &function.cfg.command_binding_sites,
                    statement,
                );
                rows.push(format!(
                    "{block:?}/{index} {statement:?} original carrier: {}",
                    tokens.is_some()
                ));
                if let Some(tokens) = tokens {
                    let target = tokens.source_binding.as_ref().and_then(|binding| {
                        binding
                            .proved_execution_target()
                            .map(|target| (&target.command, target.kind, target.registry_backed))
                    });
                    let handler = tokens.source_binding.as_ref().and_then(|binding| {
                        binding
                            .proved_handler_target()
                            .map(|target| (&target.command, target.kind, target.registry_backed))
                    });
                    let literal =
                        original_assignment_literal(function, &unit.ir_module, registry, statement);
                    let resolution =
                        resolve_command_tokens_with_metadata_context(registry, metadata, tokens)
                            .map(|resolved| match resolved {
                                RegistryInvocationResolution::Resolved(facts) => {
                                    format!("{:?}: {}", facts.operation, facts.canonical_command)
                                }
                                other => format!("{other:?}"),
                            });
                    rows.push(format!(
                        "words={:?}, aligned={}, target={target:?}, handler={handler:?}, literal={literal:?}, resolution={resolution:?}, flow={:?}, effective={:?}",
                        tokens.argv_texts,
                        tokens.words_align_with_argv_text(),
                        bindings.binding_at(block, index, tokens.argv_texts.first().map_or("", String::as_str)),
                        crate::registry_invocation::effective_command_words(tokens),
                    ));
                }
            }
        }
        rows.join("\n")
    }

    #[test]
    fn original_assignment_literals_keep_decoding_separate_from_execution_authority() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let config = tcl_lexer::LexerConfig {
            escapes: tcl_dialect::EscapeSyntax::Tcl84,
            ..tcl_lexer::LexerConfig::for_file_grammar(profile.grammar)
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&context),
            config,
        );
        let unit = CompilationUnit::build_with_analysis_input(
            r"set result \U00000041",
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let statement = unit.top_level.cfg.blocks.values().flat_map(|block| &block.statements)
            .find(|statement| matches!(statement, Statement::AssignValue { name, .. } if name == "result"))
            .expect("upstream bare literal retains AssignValue");
        assert_eq!(
            original_assignment_literal(
                &unit.top_level,
                &unit.ir_module,
                context.commands(),
                statement
            )
            .as_deref(),
            Some("U00000041")
        );
        assert!(unit.ir_module.source_entry.native_entry.is_none());
        assert!(
            assignment_facts(&unit.top_level, &unit, context.commands())
                .direct_assignments
                .is_empty()
        );
        let mut changed = statement.clone();
        let Statement::AssignValue { value, .. } = &mut changed else {
            unreachable!()
        };
        *value = "OTHER".to_owned();
        assert!(
            original_assignment_literal(
                &unit.top_level,
                &unit.ir_module,
                context.commands(),
                &changed
            )
            .is_none()
        );
    }

    #[test]
    fn structural_assignments_require_actual_availability_and_grammar() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Availability selection retains a separate actual command-entry owner;
        // it does not assert successful native compilation or fixture execution.
        use std::sync::Arc;
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut registry = baseline
            .commands()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let mut setter = registry.get("set").unwrap().clone();
        setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(setter);
        let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
        let registry = current.commands();
        let unit = retained_source_unit(
            Arc::clone(&current),
            "set result VALUE",
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let selected = &unit.top_level;
        assert_eq!(
            assignment_facts(selected, &unit, registry)
                .direct_assignments
                .len(),
            1,
            "{}",
            original_assignment_diagnostics(selected, &unit, registry),
        );
        let input = selected.source_metadata_input().unwrap();
        let replace_input = |context| {
            let mut changed = selected.clone();
            changed.source_metadata_input = Some(crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                context,
                input.lexer_config(),
            ));
            changed
        };
        let older = replace_input(Arc::new(
            tcl_registry::model::ingress::static_context_for("tcl8.4")
                .with_command_store(Arc::clone(registry)),
        ));
        assert!(Arc::ptr_eq(
            older
                .source_metadata_input()
                .unwrap()
                .context_registry()
                .commands(),
            registry
        ));
        let mut older_module = unit.ir_module.clone();
        older_module.source_metadata_input = older.source_metadata_input.clone();
        assert!(
            function_facts(
                &older,
                &older_module,
                registry,
                &unit.command_mutations,
                true,
            )
            .direct_assignments
            .is_empty()
        );
        let foreign = replace_input(
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        );
        let mut missing = selected.clone();
        missing.source_metadata_input = None;
        let mut different_grammar = selected.clone();
        different_grammar.source_config.strict_quoting =
            !different_grammar.source_config.strict_quoting;
        let mut missing_module = unit.ir_module.clone();
        missing_module.source_metadata_input = None;
        assert!(
            function_facts(
                selected,
                &missing_module,
                registry,
                &unit.command_mutations,
                true,
            )
            .direct_assignments
            .is_empty()
        );
        for refused in [older, foreign, missing, different_grammar] {
            assert!(
                assignment_facts(&refused, &unit, registry)
                    .direct_assignments
                    .is_empty()
            );
        }
    }

    #[test]
    fn structural_assignments_need_unanimous_original_source_carriers() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let unit = retained_source_unit(
            std::sync::Arc::clone(&context),
            "set result VALUE",
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let registry = context.commands();
        let selected = &unit.top_level;
        assert_eq!(
            assignment_facts(selected, &unit, registry)
                .direct_assignments
                .len(),
            1,
            "{}",
            original_assignment_diagnostics(selected, &unit, registry),
        );
        let mut missing = selected.clone();
        missing.cfg.command_binding_sites.clear();
        assert!(
            assignment_facts(&missing, &unit, registry)
                .direct_assignments
                .is_empty()
        );
        let mut conflicting = selected.clone();
        let mut additional = conflicting
            .cfg
            .command_binding_sites
            .iter()
            .find(|site| site.source_tokens.is_some())
            .unwrap()
            .clone();
        let tokens = additional.source_tokens.as_mut().unwrap();
        tokens.argv_texts[0] = "different".into();
        conflicting.cfg.command_binding_sites.push(additional);
        assert!(
            assignment_facts(&conflicting, &unit, registry)
                .direct_assignments
                .is_empty()
        );
        let mut unqueried = selected.clone();
        for site in &mut unqueried.cfg.command_binding_sites {
            if let Some(tokens) = &mut site.source_tokens {
                tokens.source_binding = None;
            }
        }
        assert!(
            assignment_facts(&unqueried, &unit, registry)
                .direct_assignments
                .is_empty()
        );
    }

    #[test]
    fn assignment_source_metadata_retains_nested_lexer_overlays() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let config = tcl_lexer::LexerConfig {
            escapes: tcl_dialect::EscapeSyntax::Tcl84,
            ..tcl_lexer::LexerConfig::for_dialect("tcl8.6")
        };
        // The original Tcl84 escape overlay is the measured Tcl8.5 lexer
        // grammar. Its actual command entry stays separate from C8.6 availability.
        let unit = native_source_unit(
            std::sync::Arc::clone(&context),
            r"proc p {} {set escaped \U00000041}; p",
            config,
            tcl_dialect::DialectProfile::find("tcl8.5").unwrap(),
        );
        let function = unit.procedures.get("::p").unwrap();
        assert_eq!(function.source_lexer_config(), config.nested().normalized());
        assert_eq!(
            function.source_metadata_input().unwrap().lexer_config(),
            config.nested().normalized()
        );
        let facts = function_facts(
            function,
            &unit.ir_module,
            context.commands(),
            &unit.command_mutations,
            false,
        );
        assert_eq!(facts.direct_assignments.len(), 1);
        let mut stale = function.clone();
        stale.source_config.escapes = tcl_dialect::EscapeSyntax::Tcl86;
        assert!(
            stale
                .invocation_metadata_context(context.commands())
                .is_none()
        );
        let facts = function_facts(
            &stale,
            &unit.ir_module,
            context.commands(),
            &unit.command_mutations,
            false,
        );
        assert!(facts.direct_assignments.is_empty());
    }

    fn only_decline(facts: &FunctionFacts) -> WasmLeafInvokeDecline {
        let mut declines = facts.leaf_declines.values().copied().collect::<Vec<_>>();
        assert_eq!(declines.len(), 1, "expected exactly one declined statement");
        declines.pop().expect("one decline")
    }

    /// The `proc` statement inside `unit`, with the tokens the front end kept.
    fn proc_statement_tokens(unit: &FunctionUnit) -> Vec<(Span, &crate::ir::CommandTokens)> {
        let mut found = Vec::new();
        for block in unit.cfg.reverse_postorder() {
            let Some(cfg_block) = unit.cfg.blocks.get(&block) else {
                continue;
            };
            for statement in &cfg_block.statements {
                if let Statement::Call {
                    span,
                    command,
                    tokens: Some(tokens),
                    ..
                } = statement
                    && command == "proc"
                {
                    found.push((*span, tokens));
                }
            }
        }
        found
    }

    /// A definition whose body word is a substitution must not be registered
    /// by this tier.
    ///
    /// `Procedure` records the *written* word
    /// `${body}` while the body this module compiled came from `return hello`,
    /// so registering it would report that word as `info body`, and any later
    /// run of the source body would evaluate the substitution in `p`'s own
    /// frame, where `body` does not exist.
    ///
    /// Asserted against the rule rather than against the emitted module,
    /// because the emit path is unreachable today: every substituted word in a
    /// definition also defeats the enclosing function's binding proof, so no
    /// `StructuredLowering(Proc)` operation is recorded to gate. That is a
    /// coincidence of two independent proofs, not a design — the point of the
    /// guard is that it already holds when procedure bodies become proven.
    #[test]
    fn host_script_admission_boxes_the_complete_original_byte_image() {
        use crate::command_binding::{ExecutedScriptSource, SourceOriginId};
        use std::sync::Arc;
        let original = tcl_lexer::SourceImage::native(b"set raw \xff\0tail".as_slice());
        let origin = Arc::new(SourceOriginId::authored_image(original.clone()));
        let chunk = ExecutedScriptSource::contiguous_image(origin, original.clone(), 0).unwrap();
        let (module, _) = codegen_runtime_chunk(Some(&chunk), 64, false, false);
        assert_eq!(module.data_segments.len(), 1);
        assert_eq!(module.data_segments[0].offset, 64);
        assert_eq!(module.data_segments[0].data, original.bytes());
    }

    #[test]
    fn a_substituted_definition_body_is_left_to_the_runtimes_own_proc() {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for(
            "proc make {} { set body {return hello} ; proc p {x} $body }\n",
            &registry,
            false,
        );
        let procedure = unit
            .ir_module
            .procedures
            .get("::p")
            .expect("the inner definition survives as a procedure");
        assert_eq!(
            procedure.body_source.as_deref(),
            Some("${body}"),
            "the fixture must record the written word rather than the compiled \
             body, else this proves nothing",
        );

        let enclosing = unit.procedures.get("::make").expect("the enclosing body");
        let definition = proc_statement_tokens(enclosing)
            .into_iter()
            .find(|(span, _)| *span == procedure.span)
            .expect("the inner `proc` statement kept its tokens");
        assert!(
            !proc_definition_is_written_out(&unit.ir_module, definition.0, definition.1),
            "a substituted body word must keep the generic invocation",
        );
    }

    /// The same proof, on a body that *is* written out, still binds — so the
    /// rule is about substitution rather than a blanket refusal to register
    /// anything defined inside a procedure.
    #[test]
    fn a_written_out_definition_body_still_binds() {
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let registry = context.commands();
        // A called original body supplies its reached binding separately from
        // the uncalled declaration preview and from availability metadata.
        let unit = retained_source_unit(
            std::sync::Arc::clone(&context),
            "proc make {} { proc p {x} {return $x} }; make\n",
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let procedure = unit.ir_module.procedures.get("::p").expect("procedure");
        let enclosing = unit.procedures.get("::make").expect("the enclosing body");
        let definition = proc_statement_tokens(enclosing)
            .into_iter()
            .find(|(span, _)| *span == procedure.span)
            .expect("the inner `proc` statement kept its tokens");
        assert!(
            proc_definition_is_written_out(&unit.ir_module, definition.0, definition.1),
            "a written-out definition is exactly what this tier may register",
        );

        // And the reachable end of it: the fact the emitter consults is set.
        let facts = function_facts(
            enclosing,
            &unit.ir_module,
            registry,
            &unit.command_mutations,
            false,
        );
        assert!(
            facts
                .literal_proc_definitions
                .contains(&span_key(procedure.span)),
            "the guard must not refuse a definition it should register: {:?}",
            facts.literal_proc_definitions,
        );
    }

    /// A top-level written-out definition is registered too — the guard is
    /// about the words, not about where the definition sits.
    #[test]
    fn a_top_level_written_out_definition_is_registered() {
        let facts = top_level_facts("proc p {x} {return $x}\n");
        assert_eq!(
            facts.literal_proc_definitions.len(),
            1,
            "{:?}",
            facts.literal_proc_definitions,
        );
    }

    /// A leaf command whose words all compile selects a plan and records no
    /// decline for that statement.
    #[test]
    fn provable_words_select_a_prebuilt_argv_plan() {
        let facts = top_level_facts("string length $value\n");
        assert_eq!(facts.leaf_invocations.len(), 1);
        assert!(facts.leaf_declines.is_empty(), "{:?}", facts.leaf_declines);
    }

    /// A word the emitter cannot prove records the precise typed reason, so the
    /// remaining source-span evaluation stays observable rather than anonymous.
    #[test]
    fn unprovable_words_record_their_typed_decline() {
        for (source, expected) in [
            (
                "string length {*}$args\n",
                WasmLeafInvokeDecline::ArgumentExpansion,
            ),
            (
                "string length a\\tb\n",
                WasmLeafInvokeDecline::BackslashSubstitution,
            ),
            (
                "string length $a($i)\n",
                WasmLeafInvokeDecline::DynamicVariableName,
            ),
        ] {
            let facts = top_level_facts(source);
            assert!(
                facts.leaf_invocations.is_empty(),
                "{source} should not select a plan"
            );
            assert_eq!(only_decline(&facts), expected, "{source}");
        }
    }
}
