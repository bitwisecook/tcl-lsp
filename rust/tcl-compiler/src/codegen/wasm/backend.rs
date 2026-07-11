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

//! The greenfield WASM codegen backend — the first consumer of the [`Emit`] seam
//! and the structured [`structured`](crate::codegen::structured) driver.
//!
//! The current tier is **eval-fallback**: every leaf command is boxed as a Tcl
//! string in the module's data section and evaluated by the runtime at run time
//! (`tcl_eval_code`, which reports the command's completion code); control flow
//! is **structured** WASM (`if`/`else`; `block`/`loop` with `br`/`br_if` for
//! loops + `break`/`continue`/`return`), and the code a leaf command returns is
//! honoured — an `error`/`return` unwinds, a `break`/`continue` re-enters the
//! loop (`RUST_ISSUE_010`). This produces a *structurally valid* module
//! (validated with `wasmtime compile`) against the `"tcl"` import ABI the WASM
//! runtime provides (values are i32 `*mut TclObj` pointers into shared linear
//! memory). The runtime side of that ABI is the leak-tested eval surface in
//! `runtime/rust/src/codegen_abi.rs` (`tcl_eval_code`, `tcl_expr_bool`, the
//! obj new/release helpers); an emitted module runs against it through the
//! shared-memory dynamic link (`__memory_base` relocation), which the standalone
//! `wasm_execute` test exercises with a stub provider.

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use crate::codegen::emit::Emit;
use crate::codegen::structured;
use crate::ir::{Module, Procedure};

/// Block type byte for a structured op (`block`/`loop`/`if`) yielding no value.
const BLOCK_VOID: u8 = 0x40;

/// The completion-code scratch local (index 0). Every emitted function declares
/// exactly one `i32` local so [`WasmEmitter::emit_completion_dispatch`] can stash
/// the code a leaf command returns and test it more than once.
const CODE_LOCAL: u64 = 0;

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
pub const RESERVED_DATA_BASE: i64 = 0x10_0000;

/// Indices of the `"tcl"` host imports the emitted module calls.
struct Imports {
    /// `(ptr, len) -> obj` — box a data-section string as a `TclObj`.
    obj_new_string: u32,
    /// `(script_obj) -> i32` — evaluate a leaf command and return its **completion
    /// code** (`0` ok … `4` continue, or a `return -code N`); the result stays the
    /// interp's own. The emitted control flow branches on the code so abrupt
    /// completion propagates (`RUST_ISSUE_010`). Adopts (frees) its argument, so
    /// there is no result reference for the emitter to release.
    eval_code: u32,
    /// `(expr_obj) -> i32` — evaluate a condition to a boolean.
    expr_bool: u32,
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
    imports: Imports,
    body: Vec<WasmInstruction>,
    data: Vec<WasmData>,
    data_offset: i64,
    /// Number of currently open control frames (`block`/`loop`/`if`).
    ctrl_depth: u32,
    /// Stack of open loops; the last is the innermost (the `break`/`continue`
    /// target, since Tcl has no labelled break).
    loops: Vec<LoopFrame>,
}

impl WasmEmitter {
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
        self.call(self.imports.obj_new_string);
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

    /// Honour the completion code a leaf command's [`tcl_eval_code`] left on the
    /// stack — the AOT realisation of the tree-walker's "stop the script on the
    /// first non-`OK` command" loop (`eval_script_mode`), so abrupt completion
    /// propagates through compiled `if`/`while`/`for` instead of being swallowed
    /// (`RUST_ISSUE_010`).
    ///
    /// Inside a loop, `break` (3) / `continue` (4) re-enter that loop's structural
    /// scopes (identical to a literal `break`/`continue`, so a *dynamic* one — a
    /// called command that completes `break` — behaves the same). Any other
    /// non-`OK` code (error, return, a `return -code N`, or a break/continue with
    /// no enclosing loop) unwinds the function with `return`. `OK` (0) falls
    /// through to the next statement.
    fn emit_completion_dispatch(&mut self) {
        // Stash the code; the dispatch reads it up to three times.
        self.local_set(CODE_LOCAL);

        // In a loop, codes 3/4 are a structural break/continue of *this* loop.
        if let Some(frame) = self.loops.last() {
            let break_block = frame.break_block;
            let continue_block = frame.continue_block;
            self.emit_code_eq_branch(TCL_BREAK, break_block);
            self.emit_code_eq_branch(TCL_CONTINUE, continue_block);
        }

        // Any remaining non-`OK` code (error/return/other, or break/continue with
        // no enclosing loop) unwinds the function.
        self.local_get(CODE_LOCAL);
        self.open_frame(WasmOp::If); // if (code != 0)
        self.push(WasmOp::Return);
        self.close_frame();
    }

    /// `if (code == want) br <target>` — a guarded structural branch the
    /// completion dispatch uses for `break`/`continue`. The `if` frame is opened
    /// so the `br` depth is computed with it in place (crossing it, plus any
    /// enclosing `if`s, back to the loop scope).
    fn emit_code_eq_branch(&mut self, want: i64, target: u32) {
        self.local_get(CODE_LOCAL);
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
    fn finish_function(&mut self, name: &str, kind: &str) -> WasmFunction {
        self.push(WasmOp::End);
        self.ctrl_depth = 0;
        self.loops.clear();
        WasmFunction {
            name: name.to_string(),
            params: Vec::new(),
            results: Vec::new(),
            // One i32 scratch local (index [`CODE_LOCAL`]) for the completion-code
            // dispatch; declared uniformly (harmless when a body has no commands).
            locals: vec![ValType::I32],
            body: std::mem::take(&mut self.body),
            local_names: vec!["$code".to_string()],
            exported: true,
            source_range: None,
            kind: kind.to_string(),
        }
    }
}

impl Emit for WasmEmitter {
    fn emit_command(&mut self, source_text: &str) {
        // code = tcl_eval_code(box(text)); then honour an abrupt completion code
        // (error/return unwinds, break/continue re-enters the loop) instead of
        // swallowing it — the top-level result stays the interp's own result.
        self.box_text(source_text);
        self.call(self.imports.eval_code);
        self.emit_completion_dispatch();
    }

    fn begin_if(&mut self, cond_text: &str) {
        // if (tcl_expr_bool(box(cond)))   — void block type (no result)
        self.box_text(cond_text);
        self.call(self.imports.expr_bool);
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
            self.call(self.imports.expr_bool);
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

fn add_tcl_import(m: &mut WasmModule, name: &str, params: &[ValType], results: &[ValType]) -> u32 {
    u32::try_from(m.add_import("tcl", name, params, results)).expect("import index fits in u32")
}

/// Lower a module's top-level script to a WASM module (eval-fallback tier +
/// structured control flow). `source` is the original Tcl text, sliced for
/// command / expression text. The constant pool is placed at offset 0 — valid
/// for a standalone module (own memory) or one whose host keeps low memory free.
#[must_use]
pub fn wasm_codegen_module(module: &Module, source: &str) -> WasmModule {
    codegen(module, source, 0, false, false)
}

/// As [`wasm_codegen_module`], but relocate the constant pool to `data_base` so
/// it lands in the runtime's reserved region (see [`RESERVED_DATA_BASE`]) when
/// the emitted module shares the runtime's linear memory in the whole-program
/// link. Both the data segments and the `i32.const` offsets the module passes to
/// `tcl_obj_new_string` are based at `data_base`.
#[must_use]
pub fn wasm_codegen_module_based(module: &Module, source: &str, data_base: i64) -> WasmModule {
    codegen(module, source, data_base, false, false)
}

/// As [`wasm_codegen_module_based`], but also emit a WASI-command entry point
/// (`_start`) that bootstraps an interp and runs `::top`, for a **single
/// self-contained binary**: merge the emitted module with the runtime (e.g.
/// `wasm-merge runtime.wasm tcl out.wasm user`), then run the one module with
/// `wasmtime merged.wasm` — no host orchestration. `_start` calls
/// `tcl_runtime_create_interp` + `tcl_runtime_set_current_interp` (imported from
/// `"tcl"`, resolved to the runtime's exports by the merge) then `::top`. The
/// runtime's `.command_export` wrappers run its ctors on first entry, so no
/// separate `_initialize` call is needed.
#[must_use]
pub fn wasm_codegen_module_standalone(module: &Module, source: &str, data_base: i64) -> WasmModule {
    codegen(module, source, data_base, true, false)
}

/// As [`wasm_codegen_module_standalone`], but the emitted `_start` also calls
/// `tcl_runtime_init_library` (between `set_current_interp` and `::top`) to
/// bootstrap the standard library — `source $TCL_LIBRARY/init.tcl`, bringing up
/// `unknown`/auto-load/`package`. With the runtime built with the embedded
/// stdlib (`--features wasm_stdlib`), the merged module runs a compiled script
/// against a fully initialised interpreter — so `package require tcltest` and
/// the rest of the stdlib work from a single `wasmtime merged.wasm`.
#[must_use]
pub fn wasm_codegen_module_standalone_init(
    module: &Module,
    source: &str,
    data_base: i64,
) -> WasmModule {
    codegen(module, source, data_base, true, true)
}

/// Shared codegen for the linkable ([`wasm_codegen_module_based`]) and
/// self-contained ([`wasm_codegen_module_standalone`]) forms. `init` adds the
/// `tcl_runtime_init_library` bootstrap call to a standalone `_start`.
fn codegen(
    module: &Module,
    source: &str,
    data_base: i64,
    standalone: bool,
    init: bool,
) -> WasmModule {
    let mut wasm = WasmModule::new();
    let imports = Imports {
        obj_new_string: add_tcl_import(
            &mut wasm,
            "tcl_obj_new_string",
            &[ValType::I32, ValType::I32],
            &[ValType::I32],
        ),
        eval_code: add_tcl_import(&mut wasm, "tcl_eval_code", &[ValType::I32], &[ValType::I32]),
        expr_bool: add_tcl_import(&mut wasm, "tcl_expr_bool", &[ValType::I32], &[ValType::I32]),
    };

    // Standalone: the interp-bootstrap imports `_start` drives. Added after the
    // ABI imports so the ABI indices in `Imports` are unchanged.
    let bootstrap = standalone.then(|| {
        let create = add_tcl_import(&mut wasm, "tcl_runtime_create_interp", &[], &[ValType::I32]);
        let set_current = add_tcl_import(
            &mut wasm,
            "tcl_runtime_set_current_interp",
            &[ValType::I32],
            &[],
        );
        let init_library = init
            .then(|| add_tcl_import(&mut wasm, "tcl_runtime_init_library", &[], &[ValType::I32]));
        (create, set_current, init_library)
    });

    // `::top` is the first *defined* function, so its call index is the import
    // count (imports occupy the low indices). Capture it before emitting bodies.
    let top_idx = u32::try_from(wasm.imports.len()).expect("import count fits in u32");

    let mut emitter = WasmEmitter {
        imports,
        body: Vec::new(),
        data: Vec::new(),
        data_offset: data_base,
        ctrl_depth: 0,
        loops: Vec::new(),
    };
    // The top-level script.
    structured::walk(&mut emitter, &module.top_level, source);
    let top = emitter.finish_function("::top", "top");
    wasm.functions.push(top);

    // Each user-defined proc body becomes its own WASM function, driven through
    // the same structured walk (its body is already lowered IR with absolute
    // source spans). Namespace-scoped procs are created at run time inside
    // `namespace eval`, not at load, so they are skipped — mirroring the bytecode
    // backend (`codegen/emitter/mod.rs`). Emitted in qualified-name order so the
    // module bytes are deterministic (`procedures` is a hash map).
    let mut procs: Vec<&Procedure> = module
        .procedures
        .values()
        .filter(|p| !p.namespace_scoped)
        .collect();
    procs.sort_by(|a, b| a.qualified_name.cmp(&b.qualified_name));
    for proc in procs {
        structured::walk(&mut emitter, &proc.body, source);
        let func = emitter.finish_function(&proc.qualified_name, "proc");
        wasm.functions.push(func);
    }

    wasm.data_segments = emitter.data;

    if let Some((create, set_current, init_library)) = bootstrap {
        wasm.functions
            .push(start_function(create, set_current, init_library, top_idx));
    }

    wasm
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
