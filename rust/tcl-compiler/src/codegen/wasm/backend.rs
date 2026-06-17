//! The greenfield WASM codegen backend — the first consumer of the [`Emit`] seam
//! and the structured [`cfg_walk`] driver.
//!
//! Stage 1 is the **eval-fallback tier**: every leaf command is boxed as a Tcl
//! string in the module's data section and evaluated by the runtime at run time
//! (`tcl_eval`); control flow is **structured** WASM (`if`/`else`). This produces
//! a *structurally valid* module (validated with `wasmtime validate`) against the
//! `"tcl"` import ABI the WASM runtime provides (values are i32 `*mut TclObj`
//! pointers into shared linear memory). It does not yet *run* — the Rust runtime's
//! wasm32 export surface is still the T1.1 stub (`runtime/rust/capi.rs`); the
//! inline AOT tiers (variable slots, arithmetic, per-command hooks) and the
//! runtime build-out are Stage 2.

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use crate::cfg::CfgModule;
use crate::codegen::cfg_walk;
use crate::codegen::emit::Emit;

/// Indices of the `"tcl"` host imports the emitted module calls.
struct Imports {
    /// `(ptr, len) -> obj` — box a data-section string as a `TclObj`.
    obj_new_string: u32,
    /// `(script_obj) -> result_obj` — the eval fallback.
    eval: u32,
    /// `(obj) -> ()` — drop a result reference.
    obj_release: u32,
    /// `(expr_obj) -> i32` — evaluate a condition to a boolean.
    expr_bool: u32,
}

/// Collects a function body + data section as the CFG walk drives it.
struct WasmEmitter {
    imports: Imports,
    body: Vec<WasmInstruction>,
    data: Vec<WasmData>,
    data_offset: i64,
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
        self.body
            .push(WasmInstruction::with_operands(WasmOp::I32Const, leb128_signed(n)));
    }

    fn call(&mut self, func_idx: u32) {
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Call,
            leb128_unsigned(u64::from(func_idx)),
        ));
    }

    /// Box `text` as a `TclObj`, leaving its i32 pointer on the stack.
    fn box_text(&mut self, text: &str) {
        let (offset, len) = self.intern(text);
        self.push_i32(offset);
        self.push_i32(len);
        self.call(self.imports.obj_new_string);
    }
}

impl Emit for WasmEmitter {
    fn emit_command(&mut self, source_text: &str) {
        // result = tcl_eval(box(text)); release(result)  (top-level result discarded)
        self.box_text(source_text);
        self.call(self.imports.eval);
        self.call(self.imports.obj_release);
    }

    fn begin_if(&mut self, cond_text: &str) {
        // if (tcl_expr_bool(box(cond)))   — void block type (no result)
        self.box_text(cond_text);
        self.call(self.imports.expr_bool);
        self.body
            .push(WasmInstruction::with_operands(WasmOp::If, vec![0x40]));
    }

    fn begin_else(&mut self) {
        self.push(WasmOp::Else);
    }

    fn end_if(&mut self) {
        self.push(WasmOp::End);
    }
}

fn add_tcl_import(m: &mut WasmModule, name: &str, params: &[ValType], results: &[ValType]) -> u32 {
    u32::try_from(m.add_import("tcl", name, params, results)).expect("import index fits in u32")
}

/// Lower a module's top-level script to a WASM module (Stage 1: eval-fallback +
/// structured `if`). `source` is the original Tcl text, sliced for command text.
#[must_use]
pub fn wasm_codegen_module(cfg: &CfgModule, source: &str) -> WasmModule {
    let mut module = WasmModule::new();
    let imports = Imports {
        obj_new_string: add_tcl_import(
            &mut module,
            "tcl_obj_new_string",
            &[ValType::I32, ValType::I32],
            &[ValType::I32],
        ),
        eval: add_tcl_import(&mut module, "tcl_eval", &[ValType::I32], &[ValType::I32]),
        obj_release: add_tcl_import(&mut module, "tcl_obj_release", &[ValType::I32], &[]),
        expr_bool: add_tcl_import(&mut module, "tcl_expr_bool", &[ValType::I32], &[ValType::I32]),
    };

    let mut emitter = WasmEmitter {
        imports,
        body: Vec::new(),
        data: Vec::new(),
        data_offset: 0,
    };
    cfg_walk::walk(&mut emitter, &cfg.top_level, source);
    // The function body is an implicit block: emit its terminal `end` explicitly.
    // (Doing so unconditionally also closes any trailing structured region — a
    // body ending in an `if`'s `end` would otherwise be mistaken by
    // `encode_body` for the function end and left with an open frame.)
    emitter.push(WasmOp::End);

    module.data_segments = emitter.data;
    module.functions.push(WasmFunction {
        name: "::top".to_string(),
        params: Vec::new(),
        results: Vec::new(),
        locals: Vec::new(),
        body: emitter.body,
        local_names: Vec::new(),
        exported: true,
        source_range: None,
        kind: "top".to_string(),
    });
    module
}
