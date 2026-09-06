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

//! Formatting helpers for bytecode disassembly output.
//!
//! Matches the output format of Tcl's built-in disassembler.

use std::collections::HashMap;
use std::fmt::Write;
use std::hash::BuildHasher;

use crate::{FunctionAsm, INDEX_END, Instruction, ModuleAsm, Op, Operand, str_class_name};

/// Escape a literal value for disassembly comments.
///
/// Matches Tcl's disassembler: control characters and non-ASCII
/// codepoints are escaped; backslashes are NOT doubled.
#[must_use]
pub fn esc(text: &str, limit: usize) -> String {
    let mut parts = String::with_capacity(text.len());
    // C Tcl's disassembler escapes the literal *byte-wise*: only the named
    // forms `\t \n \r \v \f` (and `"`) are used; every other control byte
    // and every non-ASCII byte renders as `\uXXXX` of the raw byte. So the
    // two UTF-8 bytes of `e-acute` become `\u00c3\u00a9`, an astral char is
    // its four `\u00XX` bytes, and a 0x01 control byte is `\u0001`. Iterating
    // bytes (not chars) reproduces that exactly.
    for &b in text.as_bytes() {
        match b {
            b'"' => parts.push_str("\\\""),
            b'\n' => parts.push_str("\\n"),
            b'\t' => parts.push_str("\\t"),
            b'\r' => parts.push_str("\\r"),
            0x0b => parts.push_str("\\v"),
            0x0c => parts.push_str("\\f"),
            0x20..=0x7e => parts.push(b as char),
            _ => {
                let _ = write!(parts, "\\u{b:04x}");
            }
        }
    }
    if parts.len() > limit {
        if limit <= 3 {
            // Degenerate case: ``limit`` is too small to fit
            // even a single character before the ellipsis.
            // Return ``limit`` dots so we never panic on the
            // ``limit - 3`` subtraction below.
            ".".repeat(limit)
        } else {
            let truncated: String = parts.chars().take(limit - 3).collect();
            format!("{truncated}...")
        }
    } else {
        parts
    }
}

/// Render a [`FunctionAsm`] to disassembly text.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
/// Format a single operand into its disassembly representation.
/// Returns `(part, jump_comment)`.  Extracted from
/// [`format_function_asm`] to keep the dispatcher under threshold.
fn format_operand<S: BuildHasher>(
    operand: &Operand,
    instr: &Instruction,
    j: usize,
    labels: &HashMap<String, usize, S>,
) -> (String, String) {
    match operand {
        Operand::Label(label) if instr.op.is_jump() => {
            let target = labels.get(label.as_str()).copied().unwrap_or(0);
            let relative = target as i32 - instr.offset;
            let part = if relative >= 0 {
                format!("+{relative}")
            } else {
                format!("{relative}")
            };
            (part, format!("\t# pc {target}"))
        }
        Operand::Label(label) if instr.op == Op::START_CMD && j == 0 => {
            let target = labels.get(label.as_str()).copied().unwrap_or(0);
            let relative = target as i32 - instr.offset;
            let part = if relative >= 0 {
                format!("+{relative}")
            } else {
                format!("{relative}")
            };
            let count = match instr.operands.get(1) {
                Some(Operand::Imm(c)) => *c,
                Some(_) | None => 1,
            };
            (
                part,
                format!("\t# next cmd at pc {target}, {count} cmds start here"),
            )
        }
        Operand::Label(label) => {
            let target = labels.get(label.as_str()).copied().unwrap_or(0);
            (format!("pc {target}"), String::new())
        }
        Operand::Imm(val) if instr.op.is_lvt_op() && j == 0 => (format!("%v{val}"), String::new()),
        Operand::Imm(val)
            if matches!(
                instr.op,
                Op::DICT_SET
                    | Op::DICT_UNSET
                    | Op::DICT_INCR_IMM
                    | Op::UNSET_SCALAR
                    | Op::UNSET_ARRAY
            ) && j == 1 =>
        {
            (format!("%v{val}"), String::new())
        }
        Operand::Imm(val)
            if matches!(
                instr.op,
                Op::INCR_SCALAR1_IMM
                    | Op::INCR_ARRAY1_IMM
                    | Op::INCR_STK_IMM
                    | Op::INCR_SCALAR_STK_IMM
                    | Op::INCR_ARRAY_STK_IMM
                    | Op::DICT_INCR_IMM
                    | Op::STR_MATCH
                    | Op::REGEXP
            ) =>
        {
            let part = if *val >= 0 {
                format!("+{val}")
            } else {
                format!("{val}")
            };
            (part, String::new())
        }
        Operand::Imm(val) if matches!(instr.op, Op::RETURN_IMM | Op::SYNTAX) && j == 0 => {
            let part = if *val >= 0 {
                format!("+{val}")
            } else {
                format!("{val}")
            };
            (part, String::new())
        }
        Operand::Imm(val) if instr.op == Op::STR_CLASS => {
            let name = str_class_name(*val as u8).unwrap_or("unknown");
            (name.to_string(), String::new())
        }
        Operand::Imm(val)
            if matches!(
                instr.op,
                Op::LIST_INDEX_IMM | Op::LIST_RANGE_IMM | Op::STR_RANGE_IMM
            ) && *val <= INDEX_END =>
        {
            let part = if *val == INDEX_END {
                "end".into()
            } else {
                format!("end{}", val - INDEX_END)
            };
            (part, String::new())
        }
        Operand::Imm(val) => (format!("{val}"), String::new()),
    }
}

/// Render a [`FunctionAsm`] to its disassembly string form.
/// Decode an instruction's operands to their display text, the way the
/// flat disassembler does — reused by the compiler explorer's structured
/// `asm` view so it does not re-implement the per-`Op` operand decoding.
#[must_use]
pub fn instruction_operand_text<S: BuildHasher>(
    instr: &Instruction,
    labels: &HashMap<String, usize, S>,
) -> String {
    instr
        .operands
        .iter()
        .enumerate()
        .map(|(j, operand)| format_operand(operand, instr, j, labels).0)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Mirrors C Tcl 9.0.2's `tcl::unsupported::disassemble proc` output.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
pub fn format_function_asm(asm: &FunctionAsm) -> String {
    let mut lines = Vec::new();

    let total_bytes: u32 = asm
        .instructions
        .iter()
        .map(|i| u32::from(i.op.size()))
        .sum();
    lines.push(format!(
        "ByteCode {}, {} instructions, {} bytes, {} literals, {} variables",
        asm.name,
        asm.instructions.len(),
        total_bytes,
        asm.literals.len(),
        asm.lvt.len(),
    ));

    if !asm.literals.is_empty() {
        lines.push("  Literals:".into());
        for (i, lit) in asm.literals.entries().iter().enumerate() {
            lines.push(format!("    {i}: \"{}\"", esc(lit, 40)));
        }
    }

    if !asm.lvt.is_empty() {
        lines.push("  Local variables:".into());
        for (i, var) in asm.lvt.entries().iter().enumerate() {
            lines.push(format!("    %v{i}: \"{var}\""));
        }
    }

    lines.push("  Instructions:".into());

    // Build offset→labels map; names sorted within an offset for determinism.
    let mut off2labels: HashMap<usize, Vec<&str>> = HashMap::new();
    for (label, &off) in &asm.labels {
        off2labels.entry(off).or_default().push(label.as_str());
    }
    for labels in off2labels.values_mut() {
        labels.sort_unstable();
    }

    for instr in &asm.instructions {
        let off = instr.offset as usize;
        if let Some(labels) = off2labels.get(&off) {
            for lbl in labels {
                lines.push(format!("  # {lbl}:"));
            }
        }

        let mut parts = Vec::new();
        let mut jump_comment = String::new();

        for (j, operand) in instr.operands.iter().enumerate() {
            let (part, comment) = format_operand(operand, instr, j, &asm.labels);
            parts.push(part);
            if !comment.is_empty() {
                jump_comment = comment;
            }
        }

        let ops_str = parts.join(" ");
        let spacer = if ops_str.is_empty() { "" } else { " " };
        let comment = if !jump_comment.is_empty() {
            jump_comment
        } else if !instr.comment.is_empty() {
            format!("\t# {}", instr.comment)
        } else {
            String::new()
        };
        lines.push(format!(
            "    ({}) {}{spacer}{ops_str}{comment}",
            instr.offset,
            instr.op.mnemonic()
        ));

        if instr.op == Op::JUMP_TABLE
            && let Some(ref jt) = instr.jump_table
        {
            // `jump_table` is a `HashMap`, so sort by pattern for deterministic
            // output (the table is order-independent at run time).
            let mut sorted: Vec<(&String, &String)> = jt.iter().collect();
            sorted.sort_unstable_by(|a, b| a.0.cmp(b.0));
            let entries: Vec<String> = sorted
                .into_iter()
                .map(|(pattern, label)| {
                    let target_pc = asm.labels.get(label.as_str()).copied().unwrap_or(0);
                    format!("\"{}\"->pc {target_pc}", esc(pattern, 40))
                })
                .collect();
            lines.push(format!("\t\t[{}]", entries.join(", ")));
        }
    }

    lines.join("\n")
}

/// Format an entire [`ModuleAsm`] for display.
#[must_use]
pub fn format_module_asm(module: &ModuleAsm) -> String {
    let mut parts = vec![format_function_asm(&module.top_level)];
    let mut proc_names: Vec<&String> = module.procedures.keys().collect();
    proc_names.sort();
    for name in proc_names {
        parts.push(String::new());
        parts.push(format_function_asm(&module.procedures[name]));
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_basic() {
        assert_eq!(esc("hello", 40), "hello");
    }

    #[test]
    fn esc_control_chars() {
        assert_eq!(esc("a\nb", 40), "a\\nb");
        assert_eq!(esc("a\tb", 40), "a\\tb");
        assert_eq!(esc("a\rb", 40), "a\\rb");
    }

    #[test]
    fn esc_quotes() {
        assert_eq!(esc("a\"b", 40), "a\\\"b");
    }

    #[test]
    fn esc_null() {
        assert_eq!(esc("a\0b", 40), "a\\u0000b");
    }

    #[test]
    fn esc_truncation() {
        let long = "a".repeat(50);
        let result = esc(&long, 40);
        assert!(result.len() <= 40);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn esc_unicode_is_byte_wise() {
        // C Tcl escapes the raw UTF-8 bytes, not the code point: U+00A0 is the
        // two bytes C2 A0, U+00E9 (é) is C3 A9.
        assert_eq!(esc("\u{00a0}", 40), "\\u00c2\\u00a0");
        assert_eq!(esc("é", 40), "\\u00c3\\u00a9");
    }

    #[test]
    fn esc_astral_is_byte_wise() {
        // U+1F600 is the four UTF-8 bytes F0 9F 98 80.
        assert_eq!(esc("\u{1F600}", 40), "\\u00f0\\u009f\\u0098\\u0080");
    }

    #[test]
    fn esc_c0_controls_and_named_forms() {
        // Named: \t \n \r \v \f; every other control byte is \uXXXX.
        assert_eq!(esc("\x0b\x0c", 40), "\\v\\f");
        assert_eq!(
            esc("\x01\x08\x07\x1b\x7f", 40),
            "\\u0001\\u0008\\u0007\\u001b\\u007f"
        );
    }

    #[test]
    fn format_function_asm_basic() {
        use crate::{FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op, Operand};
        use std::collections::HashMap;

        let mut lit = LiteralTable::new();
        lit.intern("hello");

        let instrs = vec![
            Instruction::new(Op::PUSH1, vec![Operand::Imm(0)]),
            Instruction::new(Op::DONE, vec![]),
        ];

        let asm = FunctionAsm {
            name: "test".into(),
            literals: lit,
            lvt: LocalVarTable::new(&[]),
            instructions: instrs,
            labels: HashMap::new(),
            loop_targets: HashMap::new(),
            body_base_line: 0,
            proc_body_src: None,
            error_regions: Vec::new(),
            plain_command_dispatch: false,
            command_bindings: Vec::new(),
            procedure_bindings: Vec::new(),
        };

        let output = format_function_asm(&asm);
        assert!(output.contains("ByteCode test"));
        assert!(output.contains("Literals:"));
        assert!(output.contains("\"hello\""));
        assert!(output.contains("push1"));
        assert!(output.contains("done"));
    }

    /// Operand rendering for the newly executable opcodes: an LVT slot reads
    /// `%vN`, an `incr` immediate carries its sign, and a plain flag/count
    /// operand (`unsetArrayStk`, `clockRead`, `dictGetDef`) is a bare integer —
    /// matching how the existing members of each family render.
    #[test]
    fn format_operand_renders_new_opcodes() {
        use crate::{Instruction, Op, Operand};
        use std::collections::HashMap;

        let labels: HashMap<String, usize> = HashMap::new();
        let text = |op: Op, operands: Vec<Operand>| {
            instruction_operand_text(&Instruction::new(op, operands), &labels)
        };
        assert_eq!(text(Op::LOAD_ARRAY4, vec![Operand::Imm(3)]), "%v3");
        assert_eq!(text(Op::ARRAY_MAKE_IMM, vec![Operand::Imm(0)]), "%v0");
        assert_eq!(text(Op::VARIABLE, vec![Operand::Imm(1)]), "%v1");
        assert_eq!(text(Op::CONST_IMM, vec![Operand::Imm(2)]), "%v2");
        assert_eq!(
            text(Op::INCR_ARRAY1_IMM, vec![Operand::Imm(1), Operand::Imm(-2)]),
            "%v1 -2"
        );
        assert_eq!(text(Op::INCR_SCALAR_STK_IMM, vec![Operand::Imm(1)]), "+1");
        assert_eq!(text(Op::UNSET_ARRAY_STK, vec![Operand::Imm(1)]), "1");
        assert_eq!(text(Op::CLOCK_READ, vec![Operand::Imm(2)]), "2");
        assert_eq!(text(Op::DICT_GET_DEF, vec![Operand::Imm(2)]), "2");
        assert_eq!(text(Op::EXPAND_DROP, vec![]), "");
    }

    #[test]
    fn op_size_values() {
        assert_eq!(Op::PUSH1.size(), 2);
        assert_eq!(Op::PUSH4.size(), 5);
        assert_eq!(Op::POP.size(), 1);
        assert_eq!(Op::ADD.size(), 1);
        assert_eq!(Op::JUMP1.size(), 2);
        assert_eq!(Op::JUMP4.size(), 5);
        assert_eq!(Op::START_CMD.size(), 9);
        assert_eq!(Op::RETURN_IMM.size(), 9);
        assert_eq!(Op::LIST.size(), 5);
        assert_eq!(Op::NOP.size(), 1);
        assert_eq!(Op::DONE.size(), 1);
        assert_eq!(Op::INCR_SCALAR1_IMM.size(), 3);
        assert_eq!(Op::LIST_RANGE_IMM.size(), 9);
        assert_eq!(Op::INVOKE_REPLACE.size(), 6);
    }
}
