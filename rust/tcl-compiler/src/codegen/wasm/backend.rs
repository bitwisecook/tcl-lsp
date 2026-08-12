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
//! memory). The runtime side of that ABI is the leak-tested eval surface in
//! `runtime/rust/src/codegen_abi.rs` (`tcl_eval_code`, `tcl_expr_bool`, the
//! object and direct-operation helpers); an emitted module runs against it through the
//! shared-memory dynamic link (`__memory_base` relocation), which the standalone
//! `wasm_execute` test exercises with a stub provider.

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, IntrinsicId, SemanticOperationId, TclType};
use tcl_syntax::expr::{BinOp, ExprNode};

use crate::codegen::cmd_subst::{is_pure_cmd_subst, parse_cmd_parts};
use crate::codegen::emit::Emit;
use crate::codegen::structured;
use crate::command_binding::{
    Binding, BindingKind, ModuleCommandMutations, analyse_command_binding,
    scan_module_command_mutations,
};
use crate::common_aot_plan::semantic_operation_binding_is_trusted;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Module, Procedure, Statement};
use crate::mixed_region_plan::GuardedSelectionEvidence;
use crate::registry_invocation::{RegistryInvocationResolution, resolve_command_tokens};
use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, CodegenAbiValueType, WASM32_CODEGEN_DATA_START, WASM32_COMPLETION_ALIGN,
    WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET, WASM32_COMPLETION_SIZE, WASM32_POINTER_BYTES,
};

use super::pipeline::{WasmCompileOptions, WasmNativeI64AddSelection};
use super::semantic_plan::WasmGenericInvokePlan;

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
const TCL_BREAK: i64 = 3;
const TCL_CONTINUE: i64 = 4;

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
    /// `(ptr, len) -> obj` — box a data-section string as a `TclObj`.
    obj_new_string: u32,
    /// `(script_obj) -> i32` — evaluate a leaf command and return its **completion
    /// code** (`0` ok … `4` continue, or a `return -code N`); the result stays the
    /// interp's own. The emitted control flow branches on the code so abrupt
    /// completion propagates. Adopts (frees) its argument, so
    /// there is no result reference for the emitter to release.
    eval_code: u32,
    /// `(expr_obj) -> i32` — evaluate a condition to a boolean.
    expr_bool: u32,
    aot: Option<AotImports>,
}

/// Runtime ABI imports used by the semantic prebuilt-argv mode of the same
/// module emitter.
#[derive(Clone, Copy)]
struct SemanticImports {
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
/// Keeping these out of [`AotImports`] preserves the legacy/general WASM
/// import surface when native proof selection is disabled.
#[derive(Clone, Copy)]
struct NativeI64AddImports {
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
}

#[derive(Default)]
struct FunctionFacts {
    operations: HashMap<(u32, u32), SemanticOperationId>,
    direct_assignments: HashSet<(u32, u32)>,
    direct_calls: HashMap<(u32, u32, String), String>,
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
    /// Stack of open loops; the last is the innermost (the `break`/`continue`
    /// target, since Tcl has no labelled break).
    loops: Vec<LoopFrame>,
    code_local: u64,
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
            loops: Vec::new(),
            code_local: 0,
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
            loops: Vec::new(),
            code_local: 0,
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

    /// Intern `text` into the data section, returning its `(offset, len)`.
    fn intern(&mut self, text: &str) -> (i64, i64) {
        let offset = self.data_offset;
        let len = i64::try_from(text.len()).unwrap_or(i64::MAX);
        self.data.push(WasmData {
            offset,
            data: text.as_bytes().to_vec(),
        });
        self.data_offset += len;
        (offset, len)
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

    fn call(&mut self, func_idx: u32) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Call,
            leb128_unsigned(u64::from(func_idx)),
        ));
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

    /// Box `text` as a `TclObj`, leaving its i32 pointer on the stack.
    fn box_text(&mut self, text: &str) {
        let (offset, len) = self.intern(text);
        self.push_i32(offset);
        self.push_i32(len);
        self.call(self.general_imports().obj_new_string);
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

    fn emit_var_get(&mut self, name: &str) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        if self.mode == FunctionMode::DirectProc {
            let Some(slot) = self.local_slots.get(name).copied() else {
                return false;
            };
            self.push_i32(i64::from(slot));
            self.call(aot.local_get);
        } else {
            self.push_text_pair(name);
            self.call(aot.var_get);
        }
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

    fn emit_word_value(&mut self, word: &str, statement: &Statement) -> bool {
        if let Some(name) = simple_var_name(word) {
            return self.emit_var_get(name);
        }
        if !is_pure_cmd_subst(word) {
            return !word.bytes().any(|b| matches!(b, b'$' | b'[' | b'\\')) && self.box_value(word);
        }
        let parts = parse_cmd_parts(word);
        let Some((head, false)) = parts.first() else {
            return false;
        };
        let key = span_command_key(statement.span(), head);
        let Some(target) = self.facts.direct_calls.get(&key).cloned() else {
            return false;
        };
        if !self.direct_procs.contains(&target) {
            return false;
        }
        if self.procedure_arity.get(&target).copied() != Some(parts.len() - 1) {
            return false;
        }
        for (arg, braced) in &parts[1..] {
            if *braced {
                if !self.box_value(arg) {
                    return false;
                }
            } else if !self.emit_word_value(arg, statement) {
                return false;
            }
        }
        let Some(index) = self.proc_indices.get(&target).copied() else {
            return false;
        };
        self.call(index);
        true
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

    fn try_emit_typed_statement(&mut self, statement: &Statement) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        match statement {
            Statement::AssignConst {
                span, name, value, ..
            } => {
                if !self.facts.direct_assignments.contains(&span_key(*span)) {
                    return false;
                }
                if self.mode == FunctionMode::DirectProc {
                    let Some(slot) = self.local_slots.get(name).copied() else {
                        return false;
                    };
                    self.push_i32(i64::from(slot));
                    if !self.box_value(value) {
                        return false;
                    }
                    self.call(aot.local_set);
                } else if self.mode == FunctionMode::Top {
                    self.push_text_pair(name);
                    if !self.box_value(value) {
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
            Statement::Call {
                span, args, tokens, ..
            } => {
                let Some(operation) = self.facts.operations.get(&span_key(*span)).copied() else {
                    return false;
                };
                match operation {
                    SemanticOperationId::StructuredLowering(LoweringHookId::Proc) => {
                        let Some(proc) = self.procedures_by_span.get(&span_key(*span)).cloned()
                        else {
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
                    SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite)
                        if args.len() == 1
                            && !tokens
                                .as_ref()
                                .is_some_and(|tokens| tokens.arg_is_braced_literal(0))
                            && (simple_var_name(&args[0]).is_some()
                                || is_pure_cmd_subst(&args[0])) =>
                    {
                        if !self.emit_word_value(&args[0], statement) {
                            return false;
                        }
                        self.call(aot.puts);
                        self.emit_completion_dispatch();
                        true
                    }
                    SemanticOperationId::Invoke
                    | SemanticOperationId::Intrinsic(_)
                    | SemanticOperationId::StructuredLowering(_) => false,
                }
            }
            _ => false,
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
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.push(WasmOp::Else);
        self.push_i32(i64::from(intrinsic));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.call(imports.invoke_intrinsic_argv);
        self.local_set(GUARDED_STATUS_LOCAL);
        self.local_get(GUARDED_STATUS_LOCAL);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push(WasmOp::Else);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
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

        // In a loop, codes 3/4 are a structural break/continue of *this* loop.
        if let Some(frame) = self.loops.last() {
            let break_block = frame.break_block;
            let continue_block = frame.continue_block;
            self.emit_code_eq_branch(TCL_BREAK, break_block);
            self.emit_code_eq_branch(TCL_CONTINUE, continue_block);
        }

        // Any remaining non-`OK` code (error/return/other, or break/continue with
        // no enclosing loop) unwinds the function.
        self.local_get(self.code_local);
        self.open_frame(WasmOp::If); // if (code != 0)
        self.push(WasmOp::Return);
        self.close_frame();
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
        WasmFunction {
            name: name.to_string(),
            params,
            results: if direct {
                vec![ValType::I32]
            } else {
                Vec::new()
            },
            locals: vec![ValType::I32],
            body: std::mem::take(&mut self.body),
            local_names,
            exported: true,
            source_range: proc.map(|p| p.span),
            kind: kind.to_string(),
        }
    }
}

impl Emit for WasmEmitter {
    fn emit_typed_statement(&mut self, statement: &Statement, _source: &str) -> bool {
        let body_len = self.body.len();
        let data_len = self.data.len();
        let data_offset = self.data_offset;
        if self.try_emit_typed_statement(statement) {
            true
        } else {
            self.body.truncate(body_len);
            self.data.truncate(data_len);
            self.data_offset = data_offset;
            false
        }
    }

    fn emit_command(&mut self, source_text: &str) {
        // code = tcl_eval_code(box(text)); then honour an abrupt completion code
        // (error/return unwinds, break/continue re-enters the loop) instead of
        // swallowing it — the top-level result stays the interp's own result.
        self.box_text(source_text);
        self.call(self.general_imports().eval_code);
        self.emit_completion_dispatch();
    }

    fn begin_if(&mut self, cond_text: &str) {
        // if (tcl_expr_bool(box(cond)))   — void block type (no result)
        self.box_text(cond_text);
        self.call(self.general_imports().expr_bool);
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
            // if (!tcl_expr_bool(box(cond))) br <break>
            self.box_text(cond);
            self.call(self.general_imports().expr_bool);
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

fn simple_var_name(word: &str) -> Option<&str> {
    if let Some(name) = word.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
        return (!name.is_empty()).then_some(name);
    }
    let name = word.strip_prefix('$')?;
    (!name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':')))
    .then_some(name)
}

fn direct_expr_supported(expr: &ExprNode, params: &HashSet<&str>) -> bool {
    match expr {
        ExprNode::Var { name, .. } => params.contains(name.as_str()),
        ExprNode::Literal { .. } => true,
        ExprNode::Binary {
            op: BinOp::Add,
            left,
            right,
        } => direct_expr_supported(left, params) && direct_expr_supported(right, params),
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
    if proc.namespace_scoped
        || proc
            .qualified_name
            .strip_prefix("::")
            .is_some_and(|name| name.contains("::"))
        || module.redefined_procedures.contains(&proc.qualified_name)
        || unit.complexity_guarded
        || !semantic_operation_binding_is_trusted(
            registry,
            mutations,
            SemanticOperationId::StructuredLowering(LoweringHookId::Expr),
        )
        || !semantic_operation_binding_is_trusted(
            registry,
            mutations,
            SemanticOperationId::StructuredLowering(LoweringHookId::Return),
        )
        || !matches!(
            unit.return_type.tcl_type(),
            Some(TclType::Int | TclType::Double | TclType::Numeric)
        )
    {
        return false;
    }
    let raw_params: Vec<&str> = proc.params_raw.split_whitespace().collect();
    if raw_params.len() != proc.params.len()
        || !raw_params
            .iter()
            .zip(&proc.params)
            .all(|(raw, name)| *raw == name)
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
    direct_expr_supported(expr, &params)
}

fn function_facts(
    unit: &FunctionUnit,
    module: &Module,
    registry: &CommandRegistry,
    is_top: bool,
) -> FunctionFacts {
    let mutations = scan_module_command_mutations(module, registry);
    let initial: Vec<(String, Binding)> = if is_top {
        Vec::new()
    } else {
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
    };
    let bindings = analyse_command_binding(&unit.cfg, registry, &initial);
    let mut facts = FunctionFacts::default();
    for block in unit.cfg.reverse_postorder() {
        let Some(cfg_block) = unit.cfg.blocks.get(&block) else {
            continue;
        };
        for (stmt_idx, statement) in cfg_block.statements.iter().enumerate() {
            if let Statement::AssignConst {
                span,
                name,
                name_braced,
                ..
            } = statement
                && (*name_braced || !crate::naming::is_dynamic_word(name))
                && registry
                    .command_names_for_semantic_operation(SemanticOperationId::StructuredLowering(
                        LoweringHookId::Set,
                    ))
                    .any(|command| {
                        bindings.is_original_builtin_at(block, stmt_idx, command)
                            && mutations.trusts(command)
                    })
            {
                facts.direct_assignments.insert(span_key(*span));
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
            let bare = statement
                .canonical_command_or_source()
                .strip_prefix("::")
                .unwrap_or_else(|| statement.canonical_command_or_source());
            if bindings.is_original_builtin_at(block, stmt_idx, command)
                && mutations.trusts(bare)
                && let Some(tokens) = tokens
                && let Ok(RegistryInvocationResolution::Resolved(invocation)) =
                    resolve_command_tokens(registry, unit.semantic_facts.dialect(), tokens)
            {
                facts
                    .operations
                    .insert(span_key(statement.span()), invocation.operation);
            }
            if !is_top {
                continue;
            }
            for proc in module.procedures.values() {
                if is_top && proc.span.start() >= statement.span().start() {
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
    }
    facts
}

const fn abi_value_type(value: CodegenAbiValueType) -> ValType {
    match value {
        CodegenAbiValueType::I32 => ValType::I32,
        CodegenAbiValueType::I64 => ValType::I64,
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

fn add_semantic_imports(wasm: &mut WasmModule) -> SemanticImports {
    SemanticImports {
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
        value_new_wide_int: add_codegen_import(wasm, CodegenAbiImportId::ValueNewWideInt),
        puts: add_codegen_import(wasm, CodegenAbiImportId::Puts),
    }
}

fn add_general_imports(wasm: &mut WasmModule, analysis: bool) -> Imports {
    let mut imports = Imports {
        obj_new_string: add_codegen_import(wasm, CodegenAbiImportId::ObjectNewString),
        eval_code: add_codegen_import(wasm, CodegenAbiImportId::EvalCode),
        expr_bool: add_codegen_import(wasm, CodegenAbiImportId::ExprBool),
        aot: None,
    };
    if analysis {
        imports.aot = Some(add_aot_imports(wasm));
    }
    imports
}

fn add_aot_imports(wasm: &mut WasmModule) -> AotImports {
    AotImports {
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
    }
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
        .filter(|p| !p.namespace_scoped)
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
        let mutations = scan_module_command_mutations(module, registry);
        procs
            .iter()
            .filter(|proc| {
                unit.procedures
                    .get(&proc.qualified_name)
                    .is_some_and(|fu| direct_proc_eligible(module, proc, fu, registry, &mutations))
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
) -> WasmModule {
    let analysis = (matches!(mode, WasmEmissionMode::General)
        && options.analysis_specialisations())
    .then_some((unit, registry));
    codegen(
        &unit.ir_module,
        &unit.source,
        options.data_base,
        options.is_standalone(),
        options.initialise_library(),
        analysis,
        mode,
    )
}

/// Shared implementation for hosted, linked, and standalone packaging.
fn codegen(
    module: &Module,
    source: &str,
    data_base: i64,
    standalone: bool,
    init: bool,
    analysis: Option<(&CompilationUnit, &CommandRegistry)>,
    mode: WasmEmissionMode<'_>,
) -> WasmModule {
    let mut wasm = WasmModule::new();
    if emit_special_mode(&mut wasm, data_base, mode) {
        return wasm;
    }
    let imports = add_general_imports(&mut wasm, analysis.is_some());

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
        function_facts(&unit.top_level, module, registry, true)
    });

    let mut emitter = WasmEmitter {
        imports: EmitterImports::General(imports),
        body: Vec::new(),
        data: Vec::new(),
        data_offset: data_base,
        ctrl_depth: 0,
        loops: Vec::new(),
        code_local: 0,
        mode: FunctionMode::Top,
        local_slots: HashMap::new(),
        proc_indices,
        procedure_arity,
        direct_procs,
        procedures_by_span,
        facts: top_facts,
    };
    // The top-level script.
    structured::walk(&mut emitter, &module.top_level, source);
    let top = emitter.finish_function("::top", "top", None);
    wasm.functions.push(top);

    // Each user-defined proc body becomes its own WASM function, driven through
    // the same structured walk (its body is already lowered IR with absolute
    // source spans). Namespace-scoped procs are created at run time inside
    // `namespace eval`, not at load, so they are skipped — mirroring the bytecode
    // backend (`codegen/emitter/mod.rs`). Emitted in qualified-name order so the
    // module bytes are deterministic (`procedures` is a hash map).
    for proc in procs {
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
        emitter.facts = analysis
            .and_then(|(unit, registry)| {
                unit.procedures
                    .get(&proc.qualified_name)
                    .map(|fu| function_facts(fu, module, registry, false))
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

    wasm
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
        WasmEmissionMode::General => return false,
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
