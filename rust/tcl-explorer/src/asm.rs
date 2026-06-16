//! Structured Tcl bytecode disassembly for the explorer `asm` view.
//!
//! Port of `compiler/codegen/bytecode/format.py::format_function_explorer`
//! / `format_module_explorer`. Each function entry carries the instruction
//! stream with resolved jump targets, label anchors, jump tables, and the
//! flat-text `text` snippet (reused from `codegen::format::format_function_asm`).
//!
//! **Per-instruction `range` is `null`:** the Rust bytecode `Instruction`
//! carries only a `source_line`, not a source span, so click-to-source is
//! at function granularity (the entry's `sourceRange`) until a span is
//! plumbed through the emitter. The bytecode itself is the Rust codegen's
//! (tracked against tclsh by the bytecode-compare gate), so this view is
//! `_NO_PARITY` versus the Python serialiser and pinned by a Rust test.

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
use tcl_compiler::codegen::{FunctionAsm, ModuleAsm, Op, Operand, codegen_module};
use tcl_compiler::ir::Module;
use tcl_lexer::LineIndex;
use tcl_registry::registry_for_dialect;

use crate::ExplorerResult;
use crate::formatters::range_dict;

/// Serialise the `asm` view: a structured bytecode disassembly per
/// function. Mirrors `_serialise_asm` (`codegen_module` +
/// `format_module_explorer`, then wiring per-entry `sourceRange` from the IR).
#[must_use]
pub fn serialise_asm(result: &ExplorerResult, li: &LineIndex, source: &str) -> Value {
    let registry = registry_for_dialect(&result.dialect);
    let module: ModuleAsm =
        codegen_module(&result.unit.cfg_module, &result.unit.ir_module, registry);

    let mut entries: Vec<Value> = Vec::new();
    entries.push(function_explorer(
        &module.top_level,
        &module.top_level.name,
        "top",
        top_source_range(&result.unit.ir_module, li, source),
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
        entries.push(function_explorer(asm, qname, "proc", src_range));
    }
    Value::Array(entries)
}

/// The `::top` source range: from the first to the last top-level statement
/// (mirrors `_serialise_asm`'s `top_range`).
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

/// Build one function's explorer entry (mirrors `format_function_explorer`
/// composed with the `sourceRange` wiring of `_serialise_asm`).
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
fn function_explorer(asm: &FunctionAsm, name: &str, kind: &str, source_range: Value) -> Value {
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
            "range": Value::Null,
            "sourceLine": instr.source_line,
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
/// preferring the label row at the target offset. Mirrors the Python
/// `_retarget` pass.
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
