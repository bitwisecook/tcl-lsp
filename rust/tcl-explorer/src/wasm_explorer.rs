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

//! Rich per-instruction explorer view for the WASM codegen output.
//!
//! Built over the
//! [`WasmModule`](tcl_compiler::codegen::wasm::WasmModule) the canonical
//! compiler produces. It resolves, per instruction:
//!
//! * `call` target names (imports and internal functions),
//! * `br` / `br_if` targets (the opening/closing of the enclosing structured
//!   control-flow construct),
//! * a `range` for click-to-source navigation,
//! * the explorer label the emitter attached (`foreach`, `if`, `catch body`, …),
//! * stable block-pairing indices (`openIdx` / `elseIdx` / `endIdx`) so the web
//!   GUI can cross-link a branch to the construct it targets and lay out edges.
//!
//! The shape grows automatically richer as the emitter emits more real
//! instructions: today most leaf commands are a single `call` to the imported
//! `tcl_eval`, so the per-instruction graph is sparse, but the structure
//! (control flow, branch pairing, ranges) is already present and correct.
//!
//! Lives in `tcl-explorer` rather than `tcl-compiler` so it consumes the WASM IR
//! without coupling the codegen crate to the explorer's JSON shape.

use serde_json::{Map, Value, json};
use tcl_compiler::codegen::wasm::{ValType, WasmFunction, WasmModule, WasmOp};
use tcl_lexer::{LineIndex, Span};

use crate::formatters::range_dict;

/// The `block`/`loop`/`if` "void" block-type byte (no result).
const BLOCK_VOID: u8 = 0x40;

/// Build the structured explorer entries for `module`: a synthetic `(module)`
/// header carrying imports, types + data segments, followed by one entry per
/// function with its resolved instruction stream.
#[must_use]
pub fn wasm_to_explorer_json(module: &WasmModule, li: &LineIndex, source: &str) -> Vec<Value> {
    let num_imports = module.imports.len();
    let total_instr: usize = module.functions.iter().map(|f| f.body.len()).sum();

    let module_header = json!({
        "name": "(module)",
        "kind": "module",
        "funcIdx": Value::Null,
        "params": [],
        "results": [],
        "locals": [],
        "sourceRange": Value::Null,
        "instrCount": 0,
        "totalInstrCount": total_instr,
        "functionCount": module.functions.len(),
        "instructions": [],
        "imports": module
            .imports
            .iter()
            .enumerate()
            .map(|(i, imp)| json!({
                "module": imp.module,
                "name": imp.name,
                "typeIdx": imp.type_idx,
                "funcIdx": i,
            }))
            .collect::<Vec<_>>(),
        // The type section as the module would emit it — see the
        // `types` field of docs/design/contracts/wasm-explorer-view.md.
        // Omitting it broke the GUI's module header outright (issue #1182).
        "types": module
            .type_section()
            .iter()
            .enumerate()
            .map(|(index, (params, results))| json!({
                "index": index,
                "params": params.iter().map(|p| p.wat_name()).collect::<Vec<_>>(),
                "results": results.iter().map(|r| r.wat_name()).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "dataSegments": module
            .data_segments
            .iter()
            .map(|seg| json!({ "offset": seg.offset, "size": seg.data.len() }))
            .collect::<Vec<_>>(),
    });

    let mut entries = vec![module_header];
    for (def_idx, func) in module.functions.iter().enumerate() {
        entries.push(function_to_explorer_json(
            module,
            func,
            num_imports + def_idx,
            li,
            source,
        ));
    }
    entries
}

/// A label dict for a WASM function index (imports occupy `0..num_imports`,
/// defined functions follow). closure.
fn func_label(module: &WasmModule, idx: usize) -> Value {
    let num_imports = module.imports.len();
    if idx < num_imports {
        let imp = &module.imports[idx];
        return json!({
            "kind": "import",
            "name": imp.name,
            "module": imp.module,
            "funcIdx": idx,
            "defIdx": Value::Null,
        });
    }
    let def_idx = idx - num_imports;
    if let Some(f) = module.functions.get(def_idx) {
        return json!({
            "kind": f.kind,
            "name": f.name,
            "module": Value::Null,
            "funcIdx": idx,
            "defIdx": def_idx,
        });
    }
    json!({
        "kind": "unknown",
        "name": format!("<func {idx}>"),
        "module": Value::Null,
        "funcIdx": idx,
        "defIdx": Value::Null,
    })
}

/// A paired structured-control-flow construct discovered in a function body.
#[derive(Clone)]
struct Frame {
    op: WasmOp,
    open_idx: usize,
    else_idx: Option<usize>,
    end_idx: Option<usize>,
    label: Option<String>,
}

/// Build the structured explorer view for a single function.
fn function_to_explorer_json(
    module: &WasmModule,
    func: &WasmFunction,
    func_idx: usize,
    li: &LineIndex,
    source: &str,
) -> Value {
    // First pass: pair BLOCK/LOOP/IF opens with their matching ELSE/END.
    let mut stack: Vec<Frame> = Vec::new();
    let mut open_info: std::collections::HashMap<usize, Frame> = std::collections::HashMap::new();
    for (i, instr) in func.body.iter().enumerate() {
        match instr.op {
            WasmOp::Block | WasmOp::Loop | WasmOp::If => stack.push(Frame {
                op: instr.op,
                open_idx: i,
                else_idx: None,
                end_idx: None,
                label: instr.label.clone(),
            }),
            WasmOp::Else => {
                if let Some(f) = stack.last_mut() {
                    f.else_idx = Some(i);
                }
            }
            WasmOp::End => {
                if let Some(mut f) = stack.pop() {
                    f.end_idx = Some(i);
                    open_info.insert(f.open_idx, f);
                }
            }
            _ => {}
        }
    }

    // Second pass: resolve each instruction, tracking indent and the live
    // control stack so branch targets resolve against the enclosing constructs.
    let mut instructions: Vec<Value> = Vec::with_capacity(func.body.len());
    let mut indent: usize = 0;
    let mut ctrl_stack: Vec<Frame> = Vec::new();

    for (i, instr) in func.body.iter().enumerate() {
        let op = instr.op;
        let mnemonic = op.wat_name();
        let mut this_indent = indent;
        let mut block_label: Option<String> = None;
        let mut block_kind: Option<&'static str> = None;

        match op {
            WasmOp::Block | WasmOp::Loop | WasmOp::If => {
                if open_info.contains_key(&i) {
                    block_label = Some(format!("$L{i}"));
                }
                block_kind = Some(match op {
                    WasmOp::Block => "block",
                    WasmOp::Loop => "loop",
                    _ => "if",
                });
                let frame = open_info.get(&i).cloned().unwrap_or(Frame {
                    op,
                    open_idx: i,
                    else_idx: None,
                    end_idx: None,
                    label: None,
                });
                ctrl_stack.push(frame);
                indent += 1;
            }
            WasmOp::Else => {
                this_indent = indent.saturating_sub(1);
            }
            WasmOp::End => {
                ctrl_stack.pop();
                indent = indent.saturating_sub(1);
                this_indent = indent;
            }
            _ => {}
        }

        // Operand decoding + target resolution.
        let d = decode_instruction(op, &instr.operands, mnemonic, module, func, &ctrl_stack);

        instructions.push(json!({
            "idx": i,
            "indent": this_indent,
            "op": mnemonic,
            "opcode": op as u8,
            "operandText": d.operand_text,
            "fullText": d.full_text,
            "range": instr.range.map_or(Value::Null, |s: Span| range_dict(s, li, source)),
            "label": instr.label,
            "callTarget": d.call_target,
            "branchTarget": d.branch_target,
            "blockLabel": block_label,
            "blockKind": block_kind,
            "localIndex": d.local_index,
        }));
    }

    // Attach block-pairing indices so the UI can cross-link opens/else/end.
    attach_pairing(&mut instructions, &open_info);

    assemble_function_json(func, func_idx, instructions, li, source)
}

/// Assemble a function's explorer entry from its resolved instruction list and
/// signature (params / results / locals + source range).
fn assemble_function_json(
    func: &WasmFunction,
    func_idx: usize,
    instructions: Vec<Value>,
    li: &LineIndex,
    source: &str,
) -> Value {
    let param_count = func.params.len();
    let params: Vec<Value> = func
        .params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            json!({
                "name": func.local_names.get(i).cloned().unwrap_or_else(|| format!("$p{i}")),
                "type": p.wat_name(),
            })
        })
        .collect();
    let locals: Vec<Value> = func
        .locals
        .iter()
        .enumerate()
        .map(|(i, lt)| {
            json!({
                "name": func
                    .local_names
                    .get(param_count + i)
                    .cloned()
                    .unwrap_or_else(|| format!("$l{i}")),
                "type": lt.wat_name(),
            })
        })
        .collect();
    json!({
        "name": func.name,
        "kind": func.kind,
        "funcIdx": func_idx,
        "exported": func.exported,
        "params": params,
        "results": func.results.iter().map(|r| r.wat_name()).collect::<Vec<_>>(),
        "locals": locals,
        "sourceRange": func.source_range.map_or(Value::Null, |s| range_dict(s, li, source)),
        "instrCount": func.body.len(),
        "instructions": Value::Array(instructions),
    })
}

/// Decoded display + target metadata for a single instruction.
struct Decoded {
    operand_text: String,
    full_text: String,
    call_target: Value,
    branch_target: Value,
    local_index: Value,
}

/// Decode an instruction's operands into explorer display text and resolved
/// call/branch targets.
fn decode_instruction(
    op: WasmOp,
    operands: &[u8],
    mnemonic: &str,
    module: &WasmModule,
    func: &WasmFunction,
    ctrl_stack: &[Frame],
) -> Decoded {
    let mut d = Decoded {
        operand_text: String::new(),
        full_text: mnemonic.to_owned(),
        call_target: Value::Null,
        branch_target: Value::Null,
        local_index: Value::Null,
    };
    match op {
        WasmOp::LocalGet | WasmOp::LocalSet | WasmOp::LocalTee => {
            if let Some(idx) = decode_index(operands) {
                d.local_index = json!(idx);
                let name = func
                    .local_names
                    .get(idx)
                    .cloned()
                    .unwrap_or_else(|| format!("$l{idx}"));
                d.operand_text = idx.to_string();
                d.full_text = if name == format!("$l{idx}") {
                    format!("{mnemonic} {idx}")
                } else {
                    format!("{mnemonic} {idx} {name}")
                };
            }
        }
        WasmOp::GlobalGet | WasmOp::GlobalSet => {
            if let Some(idx) = decode_index(operands) {
                d.operand_text = idx.to_string();
                d.full_text = format!("{mnemonic} {idx}");
            }
        }
        WasmOp::Call => {
            if let Some(idx) = decode_index(operands) {
                d.operand_text = idx.to_string();
                d.call_target = func_label(module, idx);
                d.full_text = format!("{mnemonic} {idx}");
            }
        }
        WasmOp::Br | WasmOp::BrIf => {
            if let Some(depth) = decode_index(operands) {
                d.operand_text = depth.to_string();
                d.full_text = format!("{mnemonic} {depth}");
                d.branch_target = resolve_branch(ctrl_stack, depth);
            }
        }
        WasmOp::I32Const | WasmOp::I64Const => {
            if !operands.is_empty() {
                let val = decode_leb128_signed(operands);
                d.operand_text = val.to_string();
                d.full_text = format!("{mnemonic} {val}");
            }
        }
        WasmOp::Block | WasmOp::Loop | WasmOp::If => {
            let bt = operands.first().copied().unwrap_or(BLOCK_VOID);
            if bt != BLOCK_VOID {
                d.full_text = format!("{mnemonic} (result {})", valtype_byte_name(bt));
            }
        }
        _ => {}
    }
    d
}

/// Resolve a `br` / `br_if` of `depth` against the live control stack: the
/// `depth`-th frame from the top, or a function return when it escapes.
fn resolve_branch(ctrl_stack: &[Frame], depth: usize) -> Value {
    if depth < ctrl_stack.len() {
        let frame = &ctrl_stack[ctrl_stack.len() - 1 - depth];
        let (target_idx, kind) = if frame.op == WasmOp::Loop {
            (frame.open_idx_as_value(), "loop_header")
        } else if frame.op == WasmOp::Block {
            (json!(frame.end_idx), "block_end")
        } else {
            (json!(frame.end_idx), "if_end")
        };
        json!({
            "depth": depth,
            "targetIdx": target_idx,
            "kind": kind,
            "label": frame.label,
        })
    } else {
        json!({
            "depth": depth,
            "targetIdx": Value::Null,
            "kind": "function_return",
            "label": Value::Null,
        })
    }
}

impl Frame {
    /// `open_idx` as a JSON number (a loop branch targets its header open).
    fn open_idx_as_value(&self) -> Value {
        json!(self.open_idx)
    }
}

/// Post-pass: copy `endIdx`/`elseIdx` onto each open instruction and the
/// matching `openIdx` onto its `else`/`end`.
fn attach_pairing(instructions: &mut [Value], open_info: &std::collections::HashMap<usize, Frame>) {
    // Reverse lookups: end-index → open-index, else-index → open-index.
    let mut end_to_open: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut else_to_open: std::collections::HashMap<usize, usize> =
        std::collections::HashMap::new();
    for frame in open_info.values() {
        if let Some(e) = frame.end_idx {
            end_to_open.insert(e, frame.open_idx);
        }
        if let Some(e) = frame.else_idx {
            else_to_open.insert(e, frame.open_idx);
        }
    }

    let len = instructions.len();
    for i in 0..len {
        let opcode = u8::try_from(
            instructions[i]
                .get("opcode")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        )
        .unwrap_or(0);
        if opcode == WasmOp::Block as u8
            || opcode == WasmOp::Loop as u8
            || opcode == WasmOp::If as u8
        {
            if let Some(frame) = open_info.get(&i) {
                set(&mut instructions[i], "endIdx", json!(frame.end_idx));
                set(&mut instructions[i], "elseIdx", json!(frame.else_idx));
            }
        } else if opcode == WasmOp::End as u8 {
            let Some(&open_idx) = end_to_open.get(&i) else {
                continue;
            };
            set(&mut instructions[i], "openIdx", json!(open_idx));
            if instructions[open_idx].get("endIdx").is_none() {
                set(&mut instructions[open_idx], "endIdx", json!(i));
            }
        } else if opcode == WasmOp::Else as u8 {
            let Some(&open_idx) = else_to_open.get(&i) else {
                continue;
            };
            let end_idx = open_info.get(&open_idx).and_then(|f| f.end_idx);
            set(&mut instructions[i], "openIdx", json!(open_idx));
            set(&mut instructions[i], "endIdx", json!(end_idx));
        }
    }
}

/// Insert a key on a JSON object value (no-op if it isn't an object).
fn set(value: &mut Value, key: &str, v: Value) {
    if let Value::Object(map) = value {
        map.insert(key.to_owned(), v);
    } else {
        let mut map = Map::new();
        map.insert(key.to_owned(), v);
        *value = Value::Object(map);
    }
}

/// The WAT type keyword for a raw block-type byte.
fn valtype_byte_name(byte: u8) -> String {
    match byte {
        x if x == ValType::I32 as u8 => "i32".to_owned(),
        x if x == ValType::I64 as u8 => "i64".to_owned(),
        x if x == ValType::F32 as u8 => "f32".to_owned(),
        x if x == ValType::F64 as u8 => "f64".to_owned(),
        other => format!("unknown({other})"),
    }
}

/// Decode an unsigned LEB128 from the front of `data` (operand bytes are
/// already a single encoded value).
fn decode_leb128_unsigned(data: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0;
    for &byte in data {
        result |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    result
}

/// Decode an unsigned LEB128 index as a `usize`, or `None` if the operand
/// bytes are empty (a bare opcode).
fn decode_index(operands: &[u8]) -> Option<usize> {
    if operands.is_empty() {
        return None;
    }
    usize::try_from(decode_leb128_unsigned(operands)).ok()
}

/// Decode a signed LEB128 from the front of `data`.
fn decode_leb128_signed(data: &[u8]) -> i64 {
    let mut result: i64 = 0;
    let mut shift = 0u32;
    for &byte in data {
        result |= i64::from(byte & 0x7F) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            if byte & 0x40 != 0 && shift < 64 {
                result |= -1i64 << shift;
            }
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_pipeline;

    fn wasm_entries(src: &str) -> Vec<Value> {
        let result = run_pipeline(src, "tcl8.6");
        let registry = tcl_registry::registry_for_dialect(&result.dialect);
        let module = tcl_compiler::codegen::wasm::compile_wasm(
            &result.unit,
            registry,
            tcl_compiler::codegen::wasm::WasmCompileOptions::hosted(),
        );
        let li = LineIndex::new(&result.source);
        wasm_to_explorer_json(&module, &li, &result.source)
    }

    #[test]
    fn module_header_then_functions() {
        let entries = wasm_entries("set x 1\nputs $x");
        assert!(!entries.is_empty());
        assert_eq!(entries[0]["kind"], "module");
        assert_eq!(entries[0]["name"], "(module)");
        // At least the top-level function follows the module header.
        assert!(entries.len() >= 2);
        assert!(entries[1]["instructions"].is_array());
        assert_eq!(
            entries[1]["instrCount"],
            entries[1]["instructions"].as_array().unwrap().len()
        );
    }

    /// Issue #1182: the GUI's `renderWasmModuleHeader` reads
    /// `entry.imports.length`, `entry.types.length` and
    /// `entry.dataSegments.length` unconditionally, so every field the
    /// contract (`docs/design/contracts/wasm-explorer-view.md`) declares on
    /// the `(module)` header must actually be emitted as an array. A missing
    /// `types` threw a `TypeError` mid-render, which left the WASM tab empty
    /// *and* wedged the compile spinner (issue #1183).
    #[test]
    fn module_header_carries_every_contract_field() {
        let entries = wasm_entries("proc add {a b} { expr {$a + $b} }\nputs [add 1 2]");
        let header = &entries[0];
        assert_eq!(header["kind"], "module");
        for field in ["imports", "types", "dataSegments", "instructions"] {
            assert!(
                header[field].is_array(),
                "module header field `{field}` must be an array, got {}",
                header[field]
            );
        }
        for field in ["instrCount", "totalInstrCount", "functionCount"] {
            assert!(
                header[field].is_u64(),
                "module header field `{field}` must be a number, got {}",
                header[field]
            );
        }
        // The header's counters describe the whole module, not the (empty)
        // header body: `instrCount` stays 0 so tab badges don't double-count.
        assert_eq!(header["instrCount"], 0);
        let total: u64 = entries
            .iter()
            .skip(1)
            .map(|f| f["instrCount"].as_u64().unwrap())
            .sum();
        assert_eq!(header["totalInstrCount"].as_u64().unwrap(), total);
        assert_eq!(
            header["functionCount"].as_u64().unwrap(),
            entries.len() as u64 - 1
        );
    }

    #[test]
    fn module_types_cover_imports_and_defined_functions() {
        // Every import's `typeIdx` must index into the emitted `types` list,
        // and each type entry carries WAT type names the GUI can print.
        let entries = wasm_entries("proc add {a b} { expr {$a + $b} }\nputs [add 1 2]");
        let header = &entries[0];
        let types = header["types"].as_array().unwrap();
        assert!(!types.is_empty(), "a module with imports has types");
        for (i, t) in types.iter().enumerate() {
            assert_eq!(t["index"].as_u64().unwrap(), i as u64);
            assert!(t["params"].is_array());
            assert!(t["results"].is_array());
            for name in t["params"]
                .as_array()
                .unwrap()
                .iter()
                .chain(t["results"].as_array().unwrap())
            {
                assert!(
                    ["i32", "i64", "f32", "f64"].contains(&name.as_str().unwrap()),
                    "unexpected WAT type name {name}"
                );
            }
        }
        for imp in header["imports"].as_array().unwrap() {
            let idx = usize::try_from(imp["typeIdx"].as_u64().unwrap()).unwrap();
            assert!(
                idx < types.len(),
                "import {} points at type {idx}, but only {} types are emitted",
                imp["name"],
                types.len()
            );
        }
    }

    #[test]
    fn calls_resolve_a_target() {
        // The compatibility plan boxes unsupported leaf commands as a `call` to an
        // imported host function; every such call must resolve a target label.
        let entries = wasm_entries("set x 1\nputs hello");
        let mut saw_call = false;
        for func in entries.iter().skip(1) {
            for instr in func["instructions"].as_array().unwrap() {
                if instr["op"] == "call" {
                    saw_call = true;
                    assert!(
                        instr["callTarget"].is_object(),
                        "call has no resolved target: {instr}"
                    );
                    assert!(instr["callTarget"]["name"].is_string());
                }
            }
        }
        assert!(saw_call, "no call instruction emitted for a leaf command");
    }

    #[test]
    fn branches_and_blocks_pair_up() {
        // A control construct should produce paired open/end indices and any
        // branch should resolve a structured target (or a function return).
        let entries = wasm_entries("set x 1\nif {$x > 0} {\n  puts yes\n}\n");
        for func in entries.iter().skip(1) {
            for instr in func["instructions"].as_array().unwrap() {
                if instr["op"] == "block" || instr["op"] == "loop" || instr["op"] == "if" {
                    // A paired open carries an endIdx (possibly null only if the
                    // body is genuinely unterminated, which the emitter avoids).
                    assert!(instr.get("endIdx").is_some());
                }
                if instr["op"] == "br" || instr["op"] == "br_if" {
                    assert!(instr["branchTarget"].is_object());
                    assert!(instr["branchTarget"]["kind"].is_string());
                }
            }
        }
    }

    #[test]
    fn leb128_round_trips_small_values() {
        assert_eq!(decode_leb128_unsigned(&[0x00]), 0);
        assert_eq!(decode_leb128_unsigned(&[0x7F]), 127);
        assert_eq!(decode_leb128_unsigned(&[0x80, 0x01]), 128);
        assert_eq!(decode_leb128_signed(&[0x00]), 0);
        assert_eq!(decode_leb128_signed(&[0x7F]), -1);
        assert_eq!(decode_leb128_signed(&[0x3F]), 63);
    }
}
