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

//! Structured Tcl bytecode disassembly for the explorer `asm` view.
//!
//! Each function entry carries the instruction
//! stream with resolved jump targets, label anchors, jump tables, and the
//! flat-text `text` snippet (reused from `codegen::format::format_function_asm`).
//!
//! **Per-instruction `range`:** the emitter stamps each
//! [`Instruction`](tcl_compiler::codegen::Instruction) with the byte
//! `source_span` of the construct it lowered from (statement, branch
//! condition, return value); this serialiser maps that span to a
//! line:col `range` plus a 1-based `sourceLine` for click-to-source.
//! Synthetic instructions with no direct source (loop-result pushes,
//! fallthrough jumps, padding NOPs) keep `range: null` / `sourceLine: 0`.
//! The bytecode itself is the Rust codegen's (tracked against tclsh by
//! the bytecode-compare gate), and this view's serialisation is pinned by a
//! Rust test.

// Byte offsets and instruction indices are bounded by the source size, so
// the usize/i32 conversions never truncate in practice — mirror the same
// allow set `codegen::format` uses for its layout arithmetic.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;

use serde_json::{Value, json};

use tcl_compiler::codegen::format::{format_function_asm, instruction_operand_text};
use tcl_compiler::codegen::{
    FunctionAsm, ModuleAsm, Op, Operand, codegen_module_with_command_mutations,
};
use tcl_compiler::ir::Module;
use tcl_lexer::LineIndex;
// The pack-carrying registry, not the plain one: since the EDA vendor
// libraries became `.tclspec` loadables they exist nowhere else, so an
// explorer on the plain registry reports every `synth_design` unknown while
// the diagnostic on the same line resolves it.
use tcl_spectcl::bundled::active_registry_for_dialect as registry_for_dialect;

use crate::ExplorerResult;
use crate::formatters::range_dict;

/// Serialise the `asm` view: a structured bytecode disassembly per
/// function. Runs `codegen_module`, formats each function entry, then wires
/// per-entry `sourceRange` from the IR.
#[must_use]
pub fn serialise_asm(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry_held = registry_for_dialect(&result.dialect);
    let registry = &*registry_held;
    // Codegen lowers the plain (analysis-only-transform-free) CFG so the emitted
    // bytecode is byte-identical to the unannotated source: the analysis CFG
    // (`unit.cfg_module`) carries `faithful_exceptions` transforms (tailcall /
    // all-exit-switch terminators, opaque-switch loop-jump edges,
    // guaranteed-iteration loop rotation) that must not reach codegen.  Rebuild
    // a `faithful_exceptions`-off CFG from the same IR for the disassembly view.
    let codegen_cfg = tcl_compiler::cfg_builder::build_cfg_codegen_with_registry(
        &result.unit.ir_module,
        false,
        registry,
    );
    let module: ModuleAsm = codegen_module_with_command_mutations(
        &codegen_cfg,
        &result.unit.ir_module,
        registry,
        &result.unit.command_mutations,
    );

    let mut entries: Vec<Value> = Vec::new();
    entries.push(function_explorer(
        &module.top_level,
        &module.top_level.name,
        "top",
        top_source_range(&result.unit.ir_module, li, source),
        li,
        source,
    ));

    let mut qnames: Vec<&String> = module.procedures.keys().collect();
    qnames.sort();
    for qname in qnames {
        let asm = &module.procedures[qname];
        let src_range = result
            .unit
            .ir_module
            .procedures
            .get(qname)
            .map_or(Value::Null, |p| range_dict(p.span, li, source));
        entries.push(function_explorer(asm, qname, "proc", src_range, li, source));
    }
    Value::Array(entries)
}

/// The `::top` source range: from the first to the last top-level statement.
fn top_source_range(module: &Module, li: &LineIndex, source: &str) -> Value {
    let stmts = &module.top_level.statements;
    match (stmts.first(), stmts.last()) {
        (Some(first), Some(last)) => {
            let span = tcl_lexer::Span::new(first.span().start(), last.span().end());
            range_dict(span, li, source)
        }
        _ => Value::Null,
    }
}

/// The `(idx, offset)` a label resolves to. `idx` is `None` when the label
/// points past the last real instruction (end-of-proc synthetic anchor).
fn resolve_label(
    label: &str,
    labels: &HashMap<String, usize>,
    offset_to_idx: &HashMap<i32, usize>,
) -> (Option<usize>, i32) {
    let Some(&off) = labels.get(label) else {
        return (None, 0);
    };
    let off = off as i32;
    (offset_to_idx.get(&off).copied(), off)
}

/// Build one function's explorer entry, including its `sourceRange` wiring.
// One flat builder for a function's whole explorer entry; `source_range` is a
// prebuilt JSON `Value` that is moved into the entry (needless-ref would clone).
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
fn function_explorer(
    asm: &FunctionAsm,
    name: &str,
    kind: &str,
    source_range: Value,
    li: &LineIndex,
    source: &str,
) -> Value {
    let instrs = &asm.instructions;

    let mut offset_to_idx: HashMap<i32, usize> = HashMap::new();
    for (i, instr) in instrs.iter().enumerate() {
        if instr.offset >= 0 {
            offset_to_idx.insert(instr.offset, i);
        }
    }
    let end_offset = instrs
        .last()
        .map_or(0, |last| last.offset + i32::from(last.op.size()));

    // Labels keyed by offset; names sorted within an offset for determinism.
    let mut off2labels: HashMap<i32, Vec<&str>> = HashMap::new();
    for (label, &off) in &asm.labels {
        off2labels.entry(off as i32).or_default().push(label);
    }
    for labels in off2labels.values_mut() {
        labels.sort_unstable();
    }

    let mut rows: Vec<Value> = Vec::new();

    for (i, instr) in instrs.iter().enumerate() {
        if let Some(labels) = off2labels.get(&instr.offset) {
            for lbl in labels {
                let idx = rows.len();
                rows.push(json!({
                    "idx": idx,
                    "kind": "label",
                    "label": lbl,
                    "targetIdx": i,
                    "offset": instr.offset,
                }));
            }
        }

        let operand_text = instruction_operand_text(instr, &asm.labels);

        // Structured jump target (a label operand on a jump / startCommand).
        let mut jump_target = Value::Null;
        let mut lvt_ref = Value::Null;
        for (j, operand) in instr.operands.iter().enumerate() {
            match operand {
                Operand::Label(label) if instr.op.is_jump() => {
                    let (tgt_idx, off) = resolve_label(label, &asm.labels, &offset_to_idx);
                    jump_target = json!({
                        "label": label,
                        "offset": off,
                        "targetIdx": tgt_idx,
                        "relative": off - instr.offset,
                    });
                }
                Operand::Label(label) if instr.op == Op::START_CMD && j == 0 => {
                    let (tgt_idx, off) = resolve_label(label, &asm.labels, &offset_to_idx);
                    jump_target = json!({
                        "label": label,
                        "offset": off,
                        "targetIdx": tgt_idx,
                        "relative": off - instr.offset,
                        "kind": "start_cmd_end",
                    });
                }
                Operand::Imm(val) if instr.op.is_lvt_op() && j == 0 => lvt_ref = json!(val),
                Operand::Imm(val)
                    if matches!(instr.op, Op::DICT_SET | Op::DICT_UNSET | Op::DICT_INCR_IMM)
                        && j == 1 =>
                {
                    lvt_ref = json!(val);
                }
                _ => {}
            }
        }

        // Jump-table entries (pattern → label → target index).
        let jump_table = instr.jump_table.as_ref().map(|jt| {
            let mut pairs: Vec<(&String, &String)> = jt.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let entries: Vec<Value> = pairs
                .into_iter()
                .map(|(pattern, label)| {
                    let (tgt_idx, off) = resolve_label(label, &asm.labels, &offset_to_idx);
                    json!({ "pattern": pattern, "label": label, "offset": off, "targetIdx": tgt_idx })
                })
                .collect();
            Value::Array(entries)
        });

        let mnemonic = instr.op.mnemonic();
        let full_text = if operand_text.is_empty() {
            mnemonic.to_owned()
        } else {
            format!("{mnemonic} {operand_text}")
        };
        // Per-op source mapping: the emitter stamps each instruction with
        // the byte span of the construct it lowered from (when known).
        // Map it to a line:col `range` and a 1-based `sourceLine` here;
        // synthetic ops (no span) stay `null` / line 0.
        let (range, source_line) = match instr.source_span {
            Some(span) => (
                range_dict(span, li, source),
                li.position_at(span.start()).line + 1,
            ),
            None => (Value::Null, instr.source_line),
        };
        let idx = rows.len();
        rows.push(json!({
            "idx": idx,
            "kind": "instr",
            "seq": i,
            "op": mnemonic,
            "operandText": operand_text,
            "fullText": full_text,
            "offset": instr.offset,
            "size": instr.op.size(),
            "range": range,
            "sourceLine": source_line,
            "comment": instr.comment,
            "jumpTarget": jump_target,
            "jumpTable": jump_table.unwrap_or(Value::Null),
            "lvtRef": lvt_ref,
        }));
    }

    // Synthetic tail-label rows for labels past the last real instruction.
    let placed: std::collections::HashSet<&str> = off2labels.values().flatten().copied().collect();
    let mut tail: Vec<(&String, usize)> = asm
        .labels
        .iter()
        .filter(|(lbl, off)| !placed.contains(lbl.as_str()) && **off as i32 >= end_offset)
        .map(|(lbl, off)| (lbl, *off))
        .collect();
    tail.sort_by(|a, b| a.0.cmp(b.0));
    for (lbl, off) in tail {
        let idx = rows.len();
        rows.push(json!({
            "idx": idx,
            "kind": "label",
            "label": lbl,
            "targetIdx": idx,
            "offset": off as i32,
        }));
    }

    retarget(&mut rows);

    json!({
        "name": name,
        "kind": kind,
        "instrCount": instrs.len(),
        "byteCount": instrs.iter().map(|i| u32::from(i.op.size())).sum::<u32>(),
        "literals": asm.literals.entries(),
        "locals": asm.lvt.entries(),
        "instructions": rows,
        "text": format_function_asm(asm),
        "sourceRange": source_range,
    })
}

/// Rewrite each `jumpTarget`/`jumpTable` `targetIdx` from an
/// asm-instruction index (`seq`) to a `result_instructions` index,
/// preferring the label row at the target offset.
fn retarget(rows: &mut [Value]) {
    let mut label_row_by_name: HashMap<String, usize> = HashMap::new();
    let mut seq_to_result: HashMap<u64, usize> = HashMap::new();
    for row in rows.iter() {
        let idx = row["idx"].as_u64().unwrap_or(0) as usize;
        match row["kind"].as_str() {
            Some("label") => {
                if let Some(l) = row["label"].as_str() {
                    label_row_by_name.insert(l.to_owned(), idx);
                }
            }
            Some("instr") => {
                if let Some(seq) = row["seq"].as_u64() {
                    seq_to_result.insert(seq, idx);
                }
            }
            _ => {}
        }
    }

    let fix = |tgt: &mut Value| {
        if let Some(lbl) = tgt.get("label").and_then(Value::as_str)
            && let Some(&ri) = label_row_by_name.get(lbl)
        {
            tgt["targetIdx"] = json!(ri);
            return;
        }
        if let Some(old) = tgt.get("targetIdx").and_then(Value::as_u64)
            && let Some(&ri) = seq_to_result.get(&old)
        {
            tgt["targetIdx"] = json!(ri);
        }
    };

    for row in rows.iter_mut() {
        if row["kind"].as_str() != Some("instr") {
            continue;
        }
        if !row["jumpTarget"].is_null() {
            fix(&mut row["jumpTarget"]);
        }
        if let Some(entries) = row["jumpTable"].as_array_mut() {
            for e in entries {
                fix(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_pipeline;

    /// Collect every `kind == "instr"` row across all asm entries.
    fn instr_rows(asm: &Value) -> Vec<&Value> {
        asm.as_array()
            .expect("asm view is an array")
            .iter()
            .flat_map(|entry| entry["instructions"].as_array().unwrap().iter())
            .filter(|row| row["kind"] == "instr")
            .collect()
    }

    #[test]
    fn instructions_carry_per_op_source_range() {
        // Two statements on two lines: the second `set` lowers on line 2.
        let source = "set x 1\nset y 2";
        let result = run_pipeline(source, "tcl8.6");
        let li = LineIndex::new(source);
        let asm = serialise_asm(&result, &li, source);

        let rows = instr_rows(&asm);
        assert!(!rows.is_empty(), "expected emitted instructions");

        // At least one instruction must now carry a real (non-null) range
        // with a 1-based sourceLine — the per-op span plumbing.
        let mapped: Vec<&Value> = rows
            .iter()
            .filter(|r| !r["range"].is_null())
            .copied()
            .collect();
        assert!(
            !mapped.is_empty(),
            "no instruction carried a source range: {rows:#?}"
        );
        for row in &mapped {
            let range = &row["range"];
            let source_line = row["sourceLine"].as_u64().expect("sourceLine is a number");
            assert!(source_line >= 1, "mapped op has 1-based sourceLine");
            // sourceLine is the 1-based startLine of the range.
            assert_eq!(
                source_line,
                range["startLine"].as_u64().unwrap() + 1,
                "sourceLine matches range startLine+1"
            );
        }

        // The second statement (`set y 2`) starts on line 2 (0-based line
        // 1), so some instruction must map there.
        assert!(
            mapped
                .iter()
                .any(|r| r["range"]["startLine"].as_u64() == Some(1)),
            "expected an op mapped to the second source line"
        );
    }

    #[test]
    fn synthetic_ops_have_null_range() {
        // An empty proc body emits only synthetic instructions (push "" /
        // done) with no source construct — their range stays null.
        let source = "proc p {} {}\n";
        let result = run_pipeline(source, "tcl8.6");
        let li = LineIndex::new(source);
        let asm = serialise_asm(&result, &li, source);

        let proc_entry = asm
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == "::p")
            .expect("proc ::p present");
        for row in proc_entry["instructions"].as_array().unwrap() {
            if row["kind"] == "instr" {
                assert!(
                    row["range"].is_null(),
                    "synthetic op range is null: {row:#?}"
                );
                assert_eq!(row["sourceLine"], json!(0), "synthetic op sourceLine is 0");
            }
        }
    }
}
