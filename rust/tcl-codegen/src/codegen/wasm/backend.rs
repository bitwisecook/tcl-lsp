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
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use std::collections::{HashMap, HashSet};

use tcl_registry::CommandRegistry;
use tcl_syntax::expr::{BinOp, ExprNode};

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::ir::Procedure;
use tcl_compiler::mixed_region_plan::GuardedSelectionEvidence;
use tcl_compiler::native_lowering::NativeTierReport;
use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, WASM32_CODEGEN_DATA_START, WASM32_COMPLETION_SIZE, WASM32_POINTER_BYTES,
};

use super::leaf_invoke::{WasmLeafInvokeDecline, WasmLeafInvokePlan};
use super::pipeline::{WasmCompileOptions, WasmNativeI64AddSelection};
use super::semantic_plan::WasmGenericInvokePlan;

mod function_facts;
mod imports;
mod leaf_statements;
mod module_assembly;
mod semantic_invoke;
mod structured_walk;

#[cfg(test)]
use function_facts::{
    OriginalProcedureNames, original_assignment_literal, original_assignment_name,
    procedure_shadow_obligations,
};
use imports::general_import_id;
#[cfg(test)]
use module_assembly::codegen_runtime_chunk;
use module_assembly::{NativeTier, codegen};

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
    direct_assignments: HashMap<(u32, u32), (String, String)>,
    /// Selected prebuilt-argv plans, keyed by leaf-statement span.
    leaf_invocations: HashMap<(u32, u32), WasmLeafInvokePlan>,
    /// Typed reasons a leaf statement stayed on the source-span eval fallback.
    leaf_declines: HashMap<(u32, u32), WasmLeafInvokeDecline>,
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
    proc_indices: HashMap<tcl_compiler::ir::ProcedureId, u32>,
    procedure_arity: HashMap<tcl_compiler::ir::ProcedureId, usize>,
    direct_procs: HashSet<tcl_compiler::ir::ProcedureId>,
    facts: FunctionFacts,
    retained_i64: Option<super::native_add::retained_publication::RetainedI64Publication>,
    retained_i64_import: Option<u32>,
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
            facts: FunctionFacts::default(),
            retained_i64: None,
            retained_i64_import: None,
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
            facts: FunctionFacts::default(),
            retained_i64: None,
            retained_i64_import: None,
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
        let imports = self.general_imports();
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_unbound_parameter_cleanup(proc, 0);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        for (param_idx, name) in proc.params.iter().enumerate() {
            self.push_i32(i64::try_from(param_idx).unwrap_or(i64::MAX));
            self.push_text_pair(name);
            self.local_get(u64::try_from(param_idx).unwrap_or(u64::MAX));
            // The binding ABI consumes this parameter even when it refuses.
            // Later parameters still own their transferred references.
            self.raw_call(aot.local_bind);
            self.local_set(self.code_local);
            self.raw_call(imports.host_refusal_pending);
            self.local_get(self.code_local);
            self.push(WasmOp::I32Or);
            self.open_frame(WasmOp::If);
            self.emit_unbound_parameter_cleanup(proc, param_idx + 1);
            self.emit_general_refusal_return(imports);
            self.close_frame();
        }
    }

    fn emit_unbound_parameter_cleanup(&mut self, proc: &Procedure, first: usize) {
        let Some(aot) = self.general_imports().aot else {
            return;
        };
        for parameter in first..proc.params.len() {
            self.local_get(u64::try_from(parameter).expect("formal index fits u64"));
            self.raw_call(aot.argv.object_release);
        }
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

mod owned_function_names;

/// Selected input mode for the single module emitter.
#[derive(Clone, Copy)]
pub(super) enum WasmEmissionMode<'a> {
    /// The genuine host must compile one entire original script at entry.
    /// `None` explicitly refuses admission instead of evaluating empty text.
    RuntimeChunk(Option<&'a tcl_compiler::command_binding::ExecutedScriptSource>),
    /// Common proofs selected sealed-program native i64 addition with one
    /// registry-proved boxed output boundary.
    NativeI64Add(&'a WasmNativeI64AddSelection),
    /// Preserve all original statements and request a checked body only after actual entry.
    RetainedNativeI64Add(&'a super::native_add::retained_publication::RetainedI64Publication),
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
    use tcl_compiler::native_compilation_admission::{
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
    let analysis = ((matches!(mode, WasmEmissionMode::General)
        && options.analysis_specialisations())
        || matches!(mode, WasmEmissionMode::RetainedNativeI64Add(_)))
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

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "backend/original_bind_tests.rs"]
mod original_bind_tests;
