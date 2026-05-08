//! Formatting helpers for bytecode disassembly output.
//!
//! Matches the output format of Tcl's built-in disassembler.
//! Ported from `core/compiler/codegen/format.py`.

use std::collections::HashMap;
use std::fmt::Write;

use super::{str_class_name, FunctionAsm, ModuleAsm, Op, Operand, INDEX_END};

/// Escape a literal value for disassembly comments.
///
/// Matches Tcl's disassembler: control characters and non-ASCII
/// codepoints are escaped; backslashes are NOT doubled.
#[must_use]
pub fn esc(text: &str, limit: usize) -> String {
    let mut parts = String::with_capacity(text.len());
    for ch in text.chars() {
        let cp = ch as u32;
        match ch {
            '"' => parts.push_str("\\\""),
            '\n' => parts.push_str("\\n"),
            '\t' => parts.push_str("\\t"),
            '\r' => parts.push_str("\\r"),
            '\x0b' => parts.push_str("\\v"),
            '\x0c' => parts.push_str("\\f"),
            '\0' => parts.push_str("\\u0000"),
            _ if cp > 0x7E => {
                let _ = write!(parts, "\\u{cp:04x}");
            }
            _ => parts.push(ch),
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
// Long match dispatcher over format-spec shapes.
#[allow(clippy::too_many_lines)]
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

    // Build offset→labels map
    let mut off2labels: HashMap<usize, Vec<&str>> = HashMap::new();
    for (label, &off) in &asm.labels {
        off2labels.entry(off).or_default().push(label.as_str());
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
            match operand {
                Operand::Label(label) if instr.op.is_jump() => {
                    let target = asm.labels.get(label.as_str()).copied().unwrap_or(0);
                    let relative = target as i32 - instr.offset;
                    if relative >= 0 {
                        parts.push(format!("+{relative}"));
                    } else {
                        parts.push(format!("{relative}"));
                    }
                    jump_comment = format!("\t# pc {target}");
                }
                Operand::Label(label) if instr.op == Op::START_CMD && j == 0 => {
                    let target = asm.labels.get(label.as_str()).copied().unwrap_or(0);
                    let relative = target as i32 - instr.offset;
                    if relative >= 0 {
                        parts.push(format!("+{relative}"));
                    } else {
                        parts.push(format!("{relative}"));
                    }
                    let count = match instr.operands.get(1) {
                        Some(Operand::Imm(c)) => *c,
                        _ => 1,
                    };
                    jump_comment = format!("\t# next cmd at pc {target}, {count} cmds start here");
                }
                Operand::Label(label) => {
                    let target = asm.labels.get(label.as_str()).copied().unwrap_or(0);
                    parts.push(format!("pc {target}"));
                }
                Operand::Imm(val) if instr.op.is_lvt_op() && j == 0 => {
                    parts.push(format!("%v{val}"));
                }
                Operand::Imm(val)
                    if matches!(instr.op, Op::DICT_SET | Op::DICT_UNSET | Op::DICT_INCR_IMM)
                        && j == 1 =>
                {
                    parts.push(format!("%v{val}"));
                }
                Operand::Imm(val)
                    if matches!(
                        instr.op,
                        Op::INCR_SCALAR1_IMM
                            | Op::INCR_STK_IMM
                            | Op::INCR_ARRAY_STK_IMM
                            | Op::DICT_INCR_IMM
                            | Op::STR_MATCH
                            | Op::REGEXP
                    ) =>
                {
                    if *val >= 0 {
                        parts.push(format!("+{val}"));
                    } else {
                        parts.push(format!("{val}"));
                    }
                }
                Operand::Imm(val) if matches!(instr.op, Op::RETURN_IMM | Op::SYNTAX) && j == 0 => {
                    if *val >= 0 {
                        parts.push(format!("+{val}"));
                    } else {
                        parts.push(format!("{val}"));
                    }
                }
                Operand::Imm(val) if instr.op == Op::STR_CLASS => {
                    let name = str_class_name(*val as u8).unwrap_or("unknown");
                    parts.push(name.to_string());
                }
                Operand::Imm(val)
                    if matches!(
                        instr.op,
                        Op::LIST_INDEX_IMM | Op::LIST_RANGE_IMM | Op::STR_RANGE_IMM
                    ) && *val <= INDEX_END =>
                {
                    if *val == INDEX_END {
                        parts.push("end".into());
                    } else {
                        parts.push(format!("end{}", val - INDEX_END));
                    }
                }
                Operand::Imm(val) => {
                    parts.push(format!("{val}"));
                }
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

        if instr.op == Op::JUMP_TABLE {
            if let Some(ref jt) = instr.jump_table {
                let entries: Vec<String> = jt
                    .iter()
                    .map(|(pattern, label)| {
                        let target_pc = asm.labels.get(label.as_str()).copied().unwrap_or(0);
                        format!("\"{}\"->pc {target_pc}", esc(pattern, 40))
                    })
                    .collect();
                lines.push(format!("\t\t[{}]", entries.join(", ")));
            }
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
    fn esc_unicode() {
        assert_eq!(esc("\u{00a0}", 40), "\\u00a0");
    }

    #[test]
    fn format_function_asm_basic() {
        use crate::codegen::{FunctionAsm, Instruction, LiteralTable, LocalVarTable, Op, Operand};
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
        };

        let output = format_function_asm(&asm);
        assert!(output.contains("ByteCode test"));
        assert!(output.contains("Literals:"));
        assert!(output.contains("\"hello\""));
        assert!(output.contains("push1"));
        assert!(output.contains("done"));
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
