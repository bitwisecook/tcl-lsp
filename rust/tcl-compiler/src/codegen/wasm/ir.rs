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

//! WASM IR: value types, opcodes, per-function / per-module containers, and
//! the binary (`to_bytes`) + WAT (`to_wat`) serialisers.
//!
//! Byte counts and indices are bounded by the source size, so the
//! usize/u64 conversions never truncate in practice — mirror the allow set
//! `codegen::asm` uses for its layout arithmetic.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

use tcl_lexer::Span;

use super::encoding::{
    decode_leb128_signed, decode_leb128_unsigned, encode_string, encode_vector, leb128_signed,
    leb128_unsigned,
};

/// WASM magic header (`\0asm`).
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D];
/// WASM binary version 1.
const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];
/// Block type for a structured op that yields no value (`block`/`loop`/`if`).
const BLOCK_VOID: u8 = 0x40;

/// WASM value types (their binary encoding bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ValType {
    /// 32-bit integer.
    I32 = 0x7F,
    /// 64-bit integer.
    I64 = 0x7E,
    /// 32-bit float.
    F32 = 0x7D,
    /// 64-bit float.
    F64 = 0x7C,
}

impl ValType {
    /// The WAT type keyword (`i32`, `i64`, …).
    #[must_use]
    pub fn wat_name(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }
}

/// WASM section identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum SectionId {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
}

/// WASM instruction opcodes — the subset this backend emits. Values are the
/// canonical WebAssembly MVP opcode bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum WasmOp {
    Unreachable = 0x00,
    Nop = 0x01,
    Block = 0x02,
    Loop = 0x03,
    If = 0x04,
    Else = 0x05,
    End = 0x0B,
    Br = 0x0C,
    BrIf = 0x0D,
    BrTable = 0x0E,
    Return = 0x0F,
    Call = 0x10,
    Drop = 0x1A,
    Select = 0x1B,
    LocalGet = 0x20,
    LocalSet = 0x21,
    LocalTee = 0x22,
    GlobalGet = 0x23,
    GlobalSet = 0x24,
    I32Load = 0x28,
    I64Load = 0x29,
    I32Store = 0x36,
    I64Store = 0x37,
    MemorySize = 0x3F,
    MemoryGrow = 0x40,
    I32Const = 0x41,
    I64Const = 0x42,
    F32Const = 0x43,
    F64Const = 0x44,
    I32Eqz = 0x45,
    I32Eq = 0x46,
    I32Ne = 0x47,
    I32LtS = 0x48,
    I32GtS = 0x4A,
    I32LeS = 0x4C,
    I32GeS = 0x4E,
    I64Eqz = 0x50,
    I64Eq = 0x51,
    I64Ne = 0x52,
    I64LtS = 0x53,
    I64GtS = 0x55,
    I64LeS = 0x57,
    I64GeS = 0x59,
    I32Add = 0x6A,
    I32Sub = 0x6B,
    I32Mul = 0x6C,
    I32DivS = 0x6D,
    I32RemS = 0x6F,
    I32And = 0x71,
    I32Or = 0x72,
    I32Xor = 0x73,
    I32Shl = 0x74,
    I32ShrS = 0x75,
    I64Add = 0x7C,
    I64Sub = 0x7D,
    I64Mul = 0x7E,
    I64DivS = 0x7F,
    I64RemS = 0x81,
    I64And = 0x83,
    I64Or = 0x84,
    I64Xor = 0x85,
    I64Shl = 0x86,
    I64ShrS = 0x87,
    I32WrapI64 = 0xA7,
    I64ExtendI32S = 0xAC,
}

impl WasmOp {
    /// The WAT mnemonic (`i64.add`, `br_if`, `local.get`, …).
    #[must_use]
    pub fn wat_name(self) -> &'static str {
        match self {
            Self::Unreachable => "unreachable",
            Self::Nop => "nop",
            Self::Block => "block",
            Self::Loop => "loop",
            Self::If => "if",
            Self::Else => "else",
            Self::End => "end",
            Self::Br => "br",
            Self::BrIf => "br_if",
            Self::BrTable => "br_table",
            Self::Return => "return",
            Self::Call => "call",
            Self::Drop => "drop",
            Self::Select => "select",
            Self::LocalGet => "local.get",
            Self::LocalSet => "local.set",
            Self::LocalTee => "local.tee",
            Self::GlobalGet => "global.get",
            Self::GlobalSet => "global.set",
            Self::I32Load => "i32.load",
            Self::I64Load => "i64.load",
            Self::I32Store => "i32.store",
            Self::I64Store => "i64.store",
            Self::MemorySize => "memory.size",
            Self::MemoryGrow => "memory.grow",
            Self::I32Const => "i32.const",
            Self::I64Const => "i64.const",
            Self::F32Const => "f32.const",
            Self::F64Const => "f64.const",
            Self::I32Eqz => "i32.eqz",
            Self::I32Eq => "i32.eq",
            Self::I32Ne => "i32.ne",
            Self::I32LtS => "i32.lt_s",
            Self::I32GtS => "i32.gt_s",
            Self::I32LeS => "i32.le_s",
            Self::I32GeS => "i32.ge_s",
            Self::I64Eqz => "i64.eqz",
            Self::I64Eq => "i64.eq",
            Self::I64Ne => "i64.ne",
            Self::I64LtS => "i64.lt_s",
            Self::I64GtS => "i64.gt_s",
            Self::I64LeS => "i64.le_s",
            Self::I64GeS => "i64.ge_s",
            Self::I32Add => "i32.add",
            Self::I32Sub => "i32.sub",
            Self::I32Mul => "i32.mul",
            Self::I32DivS => "i32.div_s",
            Self::I32RemS => "i32.rem_s",
            Self::I32And => "i32.and",
            Self::I32Or => "i32.or",
            Self::I32Xor => "i32.xor",
            Self::I32Shl => "i32.shl",
            Self::I32ShrS => "i32.shr_s",
            Self::I64Add => "i64.add",
            Self::I64Sub => "i64.sub",
            Self::I64Mul => "i64.mul",
            Self::I64DivS => "i64.div_s",
            Self::I64RemS => "i64.rem_s",
            Self::I64And => "i64.and",
            Self::I64Or => "i64.or",
            Self::I64Xor => "i64.xor",
            Self::I64Shl => "i64.shl",
            Self::I64ShrS => "i64.shr_s",
            Self::I32WrapI64 => "i32.wrap_i64",
            Self::I64ExtendI32S => "i64.extend_i32_s",
        }
    }

    /// True for the structured opens that increase WAT indent.
    #[must_use]
    fn is_block_open(self) -> bool {
        matches!(self, Self::Block | Self::Loop | Self::If)
    }
}

/// A single WASM instruction: an opcode plus its already-encoded operand
/// bytes. `range` / `label` are explorer-only metadata — the binary encoder
/// and WAT formatter ignore them.
#[derive(Debug, Clone)]
pub struct WasmInstruction {
    /// Opcode.
    pub op: WasmOp,
    /// Pre-encoded operand bytes (LEB128 indices, block types, …).
    pub operands: Vec<u8>,
    /// Originating Tcl source span, for explorer click-to-source.
    pub range: Option<Span>,
    /// Free-form explorer hint on a structural op (`foreach`, `if`, …).
    pub label: Option<String>,
}

impl WasmInstruction {
    /// A bare instruction with no operands or metadata.
    #[must_use]
    pub fn new(op: WasmOp) -> Self {
        Self {
            op,
            operands: Vec::new(),
            range: None,
            label: None,
        }
    }

    /// An instruction with operand bytes.
    #[must_use]
    pub fn with_operands(op: WasmOp, operands: Vec<u8>) -> Self {
        Self {
            op,
            operands,
            range: None,
            label: None,
        }
    }

    /// The opcode byte followed by its operand bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![self.op as u8];
        out.extend_from_slice(&self.operands);
        out
    }
}

/// A compiled WASM function.
#[derive(Debug, Clone)]
pub struct WasmFunction {
    /// Function name (`::top`, `::proc`, …).
    pub name: String,
    /// Parameter types.
    pub params: Vec<ValType>,
    /// Result types.
    pub results: Vec<ValType>,
    /// Local-variable types (beyond the params).
    pub locals: Vec<ValType>,
    /// Instruction stream.
    pub body: Vec<WasmInstruction>,
    /// Names for params + locals, for WAT rendering.
    pub local_names: Vec<String>,
    /// Whether the function is exported.
    pub exported: bool,
    /// Source span of the originating proc definition.
    pub source_range: Option<Span>,
    /// `"top"` | `"proc"` | `"method"`.
    pub kind: String,
}

impl WasmFunction {
    /// Encode the function body for the code section: run-length-compressed
    /// local declarations, the instruction stream, and a trailing `end`.
    #[must_use]
    fn encode_body(&self) -> Vec<u8> {
        // Group consecutive same-type locals (count, type).
        let mut local_groups: Vec<Vec<u8>> = Vec::new();
        if let Some(&first) = self.locals.first() {
            let mut current = first;
            let mut count: u64 = 1;
            for &lt in &self.locals[1..] {
                if lt == current {
                    count += 1;
                } else {
                    let mut g = leb128_unsigned(count);
                    g.push(current as u8);
                    local_groups.push(g);
                    current = lt;
                    count = 1;
                }
            }
            let mut g = leb128_unsigned(count);
            g.push(current as u8);
            local_groups.push(g);
        }

        let mut func_body = encode_vector(&local_groups);
        for instr in &self.body {
            func_body.extend(instr.encode());
        }
        // Ensure a trailing END.
        if self.body.last().map(|i| i.op) != Some(WasmOp::End) {
            func_body.push(WasmOp::End as u8);
        }

        let mut out = leb128_unsigned(func_body.len() as u64);
        out.extend(func_body);
        out
    }
}

/// An imported host function.
#[derive(Debug, Clone)]
pub struct WasmImport {
    /// Import module name (e.g. `"tcl"`).
    pub module: String,
    /// Import field name (e.g. `"tcl_obj_get_int"`).
    pub name: String,
    /// Index into the module's type section.
    pub type_idx: usize,
}

/// A data segment (string constant pool entry).
#[derive(Debug, Clone)]
pub struct WasmData {
    /// Linear-memory offset.
    pub offset: i64,
    /// Raw bytes.
    pub data: Vec<u8>,
}

/// A complete WASM module.
#[derive(Debug, Clone, Default)]
pub struct WasmModule {
    /// Defined functions, in declaration order.
    pub functions: Vec<WasmFunction>,
    /// Imported host functions (occupy function indices `0..imports.len()`).
    pub imports: Vec<WasmImport>,
    /// Data segments.
    pub data_segments: Vec<WasmData>,
    /// Linear-memory size in 64 KiB pages.
    pub memory_pages: u64,
    /// Import linear memory from the runtime rather than defining our own.
    pub import_memory: bool,
    /// Interned function signatures `(params, results)`, in type-index order.
    types: Vec<(Vec<ValType>, Vec<ValType>)>,
}

impl WasmModule {
    /// A new module that imports its linear memory from the runtime.
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory_pages: 1,
            import_memory: true,
            ..Self::default()
        }
    }

    /// Register an imported host function, returning its function index
    /// (imports occupy indices `0..imports.len()`, before defined functions).
    ///
    /// This is the supported way to add imports: it interns the import's
    /// signature into the type section so the emitted `type_idx` is always
    /// valid. Constructing a [`WasmImport`] by hand and pushing it onto
    /// [`Self::imports`] risks pointing at a type that `to_bytes` never
    /// emits, since the serialiser only interns *defined-function*
    /// signatures.
    pub fn add_import(
        &mut self,
        module: &str,
        name: &str,
        params: &[ValType],
        results: &[ValType],
    ) -> usize {
        let type_idx = self.intern_type(params, results);
        let func_idx = self.imports.len();
        self.imports.push(WasmImport {
            module: module.to_owned(),
            name: name.to_owned(),
            type_idx,
        });
        func_idx
    }

    /// The module's type section — every interned function signature in
    /// type-index order, as `(params, results)` pairs.
    ///
    /// This is what [`Self::to_bytes`] / [`Self::to_wat`] emit, computed
    /// without mutating the module: import signatures are interned eagerly by
    /// [`Self::add_import`], while defined-function signatures are only
    /// interned when the module is serialised, so reading `self.types`
    /// directly would under-report on a module that has not been emitted yet.
    #[must_use]
    pub fn type_section(&self) -> Vec<(Vec<ValType>, Vec<ValType>)> {
        let mut types = self.types.clone();
        for func in &self.functions {
            let sig = (func.params.clone(), func.results.clone());
            if !types.contains(&sig) {
                types.push(sig);
            }
        }
        types
    }

    /// Intern every defined function's signature into the type section.
    ///
    /// Both serialisers need the type section to cover the defined functions
    /// (imports are interned by [`Self::add_import`] at registration time).
    fn intern_function_types(&mut self) {
        let sigs: Vec<(Vec<ValType>, Vec<ValType>)> = self
            .functions
            .iter()
            .map(|f| (f.params.clone(), f.results.clone()))
            .collect();
        for (params, results) in &sigs {
            self.intern_type(params, results);
        }
    }

    /// Intern a function signature, returning its type-section index.
    fn intern_type(&mut self, params: &[ValType], results: &[ValType]) -> usize {
        if let Some(idx) = self
            .types
            .iter()
            .position(|(p, r)| p.as_slice() == params && r.as_slice() == results)
        {
            return idx;
        }
        let idx = self.types.len();
        self.types.push((params.to_vec(), results.to_vec()));
        idx
    }

    /// Prefix a section body with its id and byte length.
    fn make_section(id: SectionId, content: &[u8]) -> Vec<u8> {
        let mut out = vec![id as u8];
        out.extend(leb128_unsigned(content.len() as u64));
        out.extend_from_slice(content);
        out
    }

    /// Serialise to a valid WASM binary.
    #[must_use]
    pub fn to_bytes(&mut self) -> Vec<u8> {
        // Register every defined function's signature first.
        self.intern_function_types();

        let mut sections: Vec<u8> = Vec::new();

        // Type section.
        if !self.types.is_empty() {
            let entries: Vec<Vec<u8>> = self
                .types
                .iter()
                .map(|(params, results)| {
                    let mut e = vec![0x60u8]; // functype
                    let p: Vec<Vec<u8>> = params.iter().map(|&t| vec![t as u8]).collect();
                    let r: Vec<Vec<u8>> = results.iter().map(|&t| vec![t as u8]).collect();
                    e.extend(encode_vector(&p));
                    e.extend(encode_vector(&r));
                    e
                })
                .collect();
            sections.extend(Self::make_section(
                SectionId::Type,
                &encode_vector(&entries),
            ));
        }

        // Import section (host functions + optionally the runtime memory).
        // Emit it whenever there are function imports *or* we import memory:
        // a module that only needs runtime memory (e.g. for data segments)
        // must still declare it, or it references memory 0 undefined and
        // fails validation.
        if !self.imports.is_empty() || self.import_memory {
            let mut entries: Vec<Vec<u8>> = self
                .imports
                .iter()
                .map(|imp| {
                    let mut e = encode_string(&imp.module);
                    e.extend(encode_string(&imp.name));
                    e.push(0x00); // func import
                    e.extend(leb128_unsigned(imp.type_idx as u64));
                    e
                })
                .collect();
            if self.import_memory {
                let mut e = encode_string("tcl");
                e.extend(encode_string("memory"));
                e.push(0x02); // memory import
                e.push(0x00); // limits: no maximum
                e.extend(leb128_unsigned(self.memory_pages));
                entries.push(e);
            }
            sections.extend(Self::make_section(
                SectionId::Import,
                &encode_vector(&entries),
            ));
        }

        // Function section (type index per defined function).
        if !self.functions.is_empty() {
            let indices: Vec<Vec<u8>> = self
                .functions
                .iter()
                .map(|func| {
                    let tidx = self
                        .types
                        .iter()
                        .position(|(p, r)| {
                            p.as_slice() == func.params.as_slice()
                                && r.as_slice() == func.results.as_slice()
                        })
                        .unwrap_or(0);
                    leb128_unsigned(tidx as u64)
                })
                .collect();
            sections.extend(Self::make_section(
                SectionId::Function,
                &encode_vector(&indices),
            ));
        }

        // Memory section (only when we define our own).
        if !self.import_memory {
            let mut mem = leb128_unsigned(1); // one memory
            mem.push(0x00); // no maximum
            mem.extend(leb128_unsigned(self.memory_pages));
            sections.extend(Self::make_section(SectionId::Memory, &mem));
        }

        // Export section.
        let mut exports: Vec<Vec<u8>> = Vec::new();
        if !self.import_memory {
            let mut e = encode_string("memory");
            e.push(0x02); // memory export
            e.extend(leb128_unsigned(0));
            exports.push(e);
        }
        let num_imports = self.imports.len();
        for (i, func) in self.functions.iter().enumerate() {
            if func.exported {
                let mut e = encode_string(&func.name);
                e.push(0x00); // func export
                e.extend(leb128_unsigned((num_imports + i) as u64));
                exports.push(e);
            }
        }
        if !exports.is_empty() {
            sections.extend(Self::make_section(
                SectionId::Export,
                &encode_vector(&exports),
            ));
        }

        // Code section.
        if !self.functions.is_empty() {
            let bodies: Vec<Vec<u8>> = self
                .functions
                .iter()
                .map(WasmFunction::encode_body)
                .collect();
            sections.extend(Self::make_section(SectionId::Code, &encode_vector(&bodies)));
        }

        // Data section.
        if !self.data_segments.is_empty() {
            let entries: Vec<Vec<u8>> = self
                .data_segments
                .iter()
                .map(|seg| {
                    let mut e = vec![0x00u8]; // active, memory 0
                    e.push(WasmOp::I32Const as u8);
                    e.extend(leb128_signed(seg.offset));
                    e.push(WasmOp::End as u8);
                    e.extend(leb128_unsigned(seg.data.len() as u64));
                    e.extend_from_slice(&seg.data);
                    e
                })
                .collect();
            sections.extend(Self::make_section(
                SectionId::Data,
                &encode_vector(&entries),
            ));
        }

        let mut out = Vec::with_capacity(8 + sections.len());
        out.extend_from_slice(&WASM_MAGIC);
        out.extend_from_slice(&WASM_VERSION);
        out.extend(sections);
        out
    }

    /// Render a human-readable WAT representation.
    #[must_use]
    pub fn to_wat(&mut self) -> String {
        self.intern_function_types();

        let mut lines: Vec<String> = vec!["(module".to_owned()];

        for (i, (params, results)) in self.types.iter().enumerate() {
            let p: Vec<String> = params
                .iter()
                .map(|pt| format!("(param {})", pt.wat_name()))
                .collect();
            let r: Vec<String> = results
                .iter()
                .map(|rt| format!("(result {})", rt.wat_name()))
                .collect();
            lines.push(format!(
                "  (type $t{i} (func {} {}))",
                p.join(" "),
                r.join(" ")
            ));
        }

        for imp in &self.imports {
            lines.push(format!(
                "  (import \"{}\" \"{}\" (func $imp_{} (type $t{})))",
                imp.module, imp.name, imp.name, imp.type_idx
            ));
        }

        if self.import_memory {
            lines.push(format!(
                "  (import \"tcl\" \"memory\" (memory {}))",
                self.memory_pages
            ));
        } else {
            lines.push(format!(
                "  (memory (export \"memory\") {})",
                self.memory_pages
            ));
        }

        for func in &self.functions {
            let export = if func.exported {
                format!(" (export \"{}\")", func.name)
            } else {
                String::new()
            };
            let mut sig: Vec<String> = Vec::new();
            for (j, p) in func.params.iter().enumerate() {
                let name = func
                    .local_names
                    .get(j)
                    .cloned()
                    .unwrap_or_else(|| format!("$p{j}"));
                sig.push(format!("(param {name} {})", p.wat_name()));
            }
            for r in &func.results {
                sig.push(format!("(result {})", r.wat_name()));
            }
            lines.push(format!("  (func ${}{export} {}", func.name, sig.join(" ")));

            let param_count = func.params.len();
            for (j, lt) in func.locals.iter().enumerate() {
                let local_idx = param_count + j;
                let name = func
                    .local_names
                    .get(local_idx)
                    .cloned()
                    .unwrap_or_else(|| format!("$l{j}"));
                lines.push(format!("    (local {name} {})", lt.wat_name()));
            }

            let mut indent = 2usize;
            for instr in &func.body {
                lines.push(format_wat_instr(instr, indent));
                if instr.op.is_block_open() {
                    indent += 1;
                } else if instr.op == WasmOp::End {
                    indent = indent.saturating_sub(1).max(2);
                }
            }
            lines.push("  )".to_owned());
        }

        for seg in &self.data_segments {
            let escaped = escape_wat_bytes(&seg.data);
            lines.push(format!("  (data (i32.const {}) \"{escaped}\")", seg.offset));
        }

        lines.push(")".to_owned());
        lines.join("\n")
    }
}

/// Escape raw data-segment bytes for a WAT string literal without changing
/// their value. Printable ASCII stays readable; quotes and backslashes use
/// their named escapes, and every other byte uses WAT's two-digit hex escape.
fn escape_wat_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut escaped = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            b'"' => escaped.push_str("\\\""),
            b'\\' => escaped.push_str("\\\\"),
            0x20..=0x7e => escaped.push(char::from(byte)),
            _ => {
                escaped.push('\\');
                escaped.push(char::from(HEX[usize::from(byte >> 4)]));
                escaped.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    escaped
}

/// The byte offset of a memory instruction's memarg, skipping its alignment
/// exponent. `None` when the operands are not a complete memarg pair.
fn memarg_offset(operands: &[u8]) -> Option<u64> {
    let align_bytes = operands.iter().position(|byte| byte & 0x80 == 0)? + 1;
    let offset = operands.get(align_bytes..)?;
    (!offset.is_empty()).then(|| decode_leb128_unsigned(offset))
}

/// Format one instruction for WAT, indented. Operand-bearing ops decode their
/// LEB128 operand for display; structural opens render their block type.
fn format_wat_instr(instr: &WasmInstruction, indent: usize) -> String {
    let prefix = "    ".repeat(indent);
    let name = instr.op.wat_name();
    match instr.op {
        WasmOp::LocalGet
        | WasmOp::LocalSet
        | WasmOp::LocalTee
        | WasmOp::GlobalGet
        | WasmOp::GlobalSet
        | WasmOp::Call
        | WasmOp::Br
        | WasmOp::BrIf => {
            if instr.operands.is_empty() {
                format!("{prefix}{name}")
            } else {
                format!("{prefix}{name} {}", decode_leb128_unsigned(&instr.operands))
            }
        }
        WasmOp::I32Const | WasmOp::I64Const => {
            if instr.operands.is_empty() {
                format!("{prefix}{name}")
            } else {
                format!("{prefix}{name} {}", decode_leb128_signed(&instr.operands))
            }
        }
        // A memarg is an alignment exponent followed by a byte offset. The
        // emitter always uses the operand's natural alignment, so only the
        // offset carries information worth reading.
        WasmOp::I32Load | WasmOp::I64Load | WasmOp::I32Store | WasmOp::I64Store => {
            match memarg_offset(&instr.operands) {
                Some(offset) => format!("{prefix}{name} offset={offset}"),
                None => format!("{prefix}{name}"),
            }
        }
        WasmOp::Block | WasmOp::Loop | WasmOp::If => {
            // Only the void block type is emitted by this backend.
            if instr.operands.first() == Some(&BLOCK_VOID) || instr.operands.is_empty() {
                format!("{prefix}{name}")
            } else {
                format!("{prefix}{name} (result ...)")
            }
        }
        _ => format!("{prefix}{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_module() -> WasmModule {
        let mut m = WasmModule::new();
        // Host helper with a signature no defined function shares, via the
        // supported path so its type is interned.
        m.add_import("tcl", "tcl_obj_get_int", &[ValType::I32], &[ValType::I64]);
        // A trivial exported `::top` that returns the i64 constant 42.
        m.functions.push(WasmFunction {
            name: "::top".into(),
            params: vec![],
            results: vec![ValType::I64],
            locals: vec![ValType::I64],
            body: vec![
                WasmInstruction::with_operands(WasmOp::I64Const, leb128_signed(42)),
                WasmInstruction::new(WasmOp::Return),
            ],
            local_names: vec!["$x".into()],
            exported: true,
            source_range: None,
            kind: "top".into(),
        });
        m
    }

    #[test]
    fn to_bytes_emits_valid_header_and_sections() {
        let mut m = sample_module();
        let bytes = m.to_bytes();
        // Magic + version.
        assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6D]);
        assert_eq!(&bytes[4..8], &[0x01, 0x00, 0x00, 0x00]);
        // Type (1), Import (2), Function (3), Export (7), Code (10) sections
        // each appear once, in ascending id order.
        let mut seen = Vec::new();
        let mut i = 8;
        while i < bytes.len() {
            let id = bytes[i];
            let (len, adv) = read_uleb(&bytes[i + 1..]);
            seen.push(id);
            i += 1 + adv + len as usize;
        }
        assert_eq!(seen, vec![1, 2, 3, 7, 10], "section ids in order: {seen:?}");
    }

    // Minimal uleb reader for the test (returns value + bytes consumed).
    fn read_uleb(bytes: &[u8]) -> (u64, usize) {
        let mut result = 0u64;
        let mut shift = 0;
        let mut n = 0;
        for &b in bytes {
            result |= u64::from(b & 0x7F) << shift;
            n += 1;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        (result, n)
    }

    #[test]
    fn to_wat_renders_module_function_and_body() {
        let mut m = sample_module();
        let wat = m.to_wat();
        assert!(wat.starts_with("(module"));
        assert!(wat.contains("(import \"tcl\" \"tcl_obj_get_int\""));
        assert!(wat.contains("(import \"tcl\" \"memory\" (memory 1))"));
        assert!(wat.contains("(func $::top (export \"::top\")"));
        assert!(wat.contains("(local $x i64)"));
        assert!(wat.contains("i64.const 42"));
        assert!(wat.contains("return"));
        assert!(wat.trim_end().ends_with(')'));
    }

    #[test]
    fn to_wat_escapes_data_segments_without_changing_bytes() {
        let mut m = WasmModule::new();
        m.data_segments.push(WasmData {
            offset: 17,
            data: vec![
                b'a', b'\n', b'\t', 0, b'"', b'\\', b' ', 0x7f, 0x80, 0xff, b'z',
            ],
        });

        let wat = m.to_wat();
        assert!(
            wat.contains(r#"(data (i32.const 17) "a\0a\09\00\"\\ \7f\80\ffz")"#),
            "{wat}"
        );
        let data_line = wat
            .lines()
            .find(|line| line.contains("(data "))
            .expect("rendered data segment");
        assert!(
            data_line.bytes().all(|byte| !byte.is_ascii_control()),
            "data segment contains an unescaped control byte: {data_line:?}"
        );
    }

    #[test]
    fn encode_body_compresses_locals_and_appends_end() {
        let func = WasmFunction {
            name: "f".into(),
            params: vec![],
            results: vec![],
            locals: vec![ValType::I64, ValType::I64, ValType::I32],
            body: vec![WasmInstruction::new(WasmOp::Nop)],
            local_names: vec![],
            exported: false,
            source_range: None,
            kind: "proc".into(),
        };
        let body = func.encode_body();
        // body length prefix, then locals vector: 2 groups -> count 2.
        let (size, adv) = read_uleb(&body);
        assert_eq!(size as usize, body.len() - adv);
        let locals = &body[adv..];
        assert_eq!(locals[0], 2, "two local groups (i64 x2, i32 x1)");
        // ...count=2, type=i64(0x7E), count=1, type=i32(0x7F)
        assert_eq!(&locals[1..5], &[2, 0x7E, 1, 0x7F]);
        // nop then trailing end.
        assert_eq!(locals[5], WasmOp::Nop as u8);
        assert_eq!(*locals.last().unwrap(), WasmOp::End as u8);
    }

    #[test]
    fn add_import_interns_signature_into_type_section() {
        // The import's signature ([i32] -> [i64]) differs from the only
        // defined function's (() -> [i64]); add_import must intern it so the
        // import points at a real, distinct type index.
        let mut m = sample_module();
        let import_type_idx = m.imports[0].type_idx;
        let _ = m.to_bytes(); // populates the rest of the type table
        assert!(
            import_type_idx < m.types.len(),
            "import type_idx {import_type_idx} must reference an emitted type (have {})",
            m.types.len()
        );
        assert_eq!(
            m.types[import_type_idx],
            (vec![ValType::I32], vec![ValType::I64]),
            "interned import signature must round-trip"
        );
        // Two distinct signatures => two types.
        assert_eq!(m.types.len(), 2);
    }

    #[test]
    fn type_section_matches_the_emitted_type_table() {
        // `type_section()` is the non-mutating view the explorer reads before
        // anything has serialised the module. It must already agree with the
        // table `to_bytes` ends up interning, or the explorer's `(module)`
        // header under-reports the module's types (issue #1182).
        let mut m = sample_module();
        let before = m.type_section();
        assert_eq!(
            before.len(),
            2,
            "import signature + defined-function signature"
        );
        let _ = m.to_bytes();
        assert_eq!(
            before, m.types,
            "type_section() must predict the emitted type section exactly"
        );
        // Idempotent: serialising again neither grows nor reorders the table.
        assert_eq!(m.type_section(), m.types);
    }

    #[test]
    fn memory_only_module_still_declares_imported_memory() {
        // A module with no function imports but import_memory = true (e.g.
        // one that only writes data segments) must still emit the Import
        // section carrying the memory import, else it references memory 0
        // undefined and fails validation.
        let mut m = WasmModule::new();
        m.data_segments.push(WasmData {
            offset: 0,
            data: b"hi".to_vec(),
        });
        let bytes = m.to_bytes();
        let mut i = 8;
        let mut seen = Vec::new();
        while i < bytes.len() {
            let id = bytes[i];
            let (len, adv) = read_uleb(&bytes[i + 1..]);
            seen.push(id);
            i += 1 + adv + len as usize;
        }
        assert!(
            seen.contains(&(SectionId::Import as u8)),
            "import section (with memory) must be present: {seen:?}"
        );
        assert!(
            seen.contains(&(SectionId::Data as u8)),
            "data section must be present: {seen:?}"
        );
        // The WAT view already shows the memory import; the binary must agree.
        assert!(m.to_wat().contains("(import \"tcl\" \"memory\""));
    }
}
