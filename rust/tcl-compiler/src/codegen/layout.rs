//! Instruction layout and jump-size optimisation.
//!
//! Assigns byte offsets to instructions and resolves symbolic labels
//! to concrete offsets.  Ported from `core/compiler/codegen/layout.py`.

use std::collections::HashMap;

use super::{Instruction, Op, Operand};

/// Replace 4-byte jumps with 1-byte jumps when relative offset fits.
///
/// Iterates up to `max_iters` times until no more replacements are
/// possible (each replacement may change offsets enough to enable
/// further replacements).
#[allow(clippy::implicit_hasher)]
pub fn optimise_jumps(instrs: &mut [Instruction], labels: &HashMap<String, usize>, max_iters: u32) {
    let jump4_to_jump1: &[(Op, Op)] = &[
        (Op::JUMP4, Op::JUMP1),
        (Op::JUMP_TRUE4, Op::JUMP_TRUE1),
        (Op::JUMP_FALSE4, Op::JUMP_FALSE1),
    ];

    for _ in 0..max_iters {
        // Assign byte offsets
        let mut offset: i32 = 0;
        for instr in instrs.iter_mut() {
            instr.offset = offset;
            offset += i32::from(instr.op.size());
        }

        // Build label offset map
        let mut label_offsets: HashMap<&str, i32> = HashMap::new();
        for (label, &instr_idx) in labels {
            if instr_idx < instrs.len() {
                label_offsets.insert(label.as_str(), instrs[instr_idx].offset);
            } else {
                label_offsets.insert(label.as_str(), offset);
            }
        }

        let mut changed = false;
        for instr in instrs.iter_mut() {
            let short_op = jump4_to_jump1
                .iter()
                .find(|(long, _)| *long == instr.op)
                .map(|(_, short)| *short);
            let Some(short_op) = short_op else {
                continue;
            };

            let target_off = match instr.operands.first() {
                Some(Operand::Label(label)) => {
                    // Skip certain labels that should stay as jump4
                    if label.starts_with("switch_") || label.starts_with("proc_exit_") {
                        continue;
                    }
                    if matches!(instr.comment.as_str(), "break" | "continue" | "try_on") {
                        continue;
                    }
                    match label_offsets.get(label.as_str()) {
                        Some(&off) => off,
                        None => continue,
                    }
                }
                Some(Operand::Imm(off)) => *off,
                None => continue,
            };

            let rel = target_off - instr.offset;
            if (-128..=127).contains(&rel) {
                instr.op = short_op;
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }
}

/// Assign final byte offsets and return label→offset mapping.
///
/// `implicit_hasher` is allowed because call sites always construct a
/// default `HashMap`; plumbing a hasher parameter through the whole
/// layout pass is noise for no measurable win.
#[allow(clippy::implicit_hasher)]
pub fn resolve_layout(
    instrs: &mut [Instruction],
    labels: &HashMap<String, usize>,
) -> HashMap<String, usize> {
    let mut offset: i32 = 0;
    for instr in instrs.iter_mut() {
        instr.offset = offset;
        offset += i32::from(instr.op.size());
    }

    let mut label_offsets = HashMap::new();
    for (label, &instr_idx) in labels {
        let raw_offset = if instr_idx < instrs.len() {
            instrs[instr_idx].offset
        } else {
            offset
        };
        label_offsets.insert(
            label.clone(),
            usize::try_from(raw_offset).expect("bytecode offsets are non-negative"),
        );
    }
    label_offsets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{Instruction, Op, Operand};

    #[test]
    fn resolve_layout_basic() {
        let mut instrs = vec![
            Instruction::new(Op::PUSH1, vec![Operand::Imm(0)]), // 2 bytes
            Instruction::new(Op::PUSH1, vec![Operand::Imm(1)]), // 2 bytes
            Instruction::new(Op::ADD, vec![]),                  // 1 byte
            Instruction::new(Op::DONE, vec![]),                 // 1 byte
        ];
        let mut labels = HashMap::new();
        labels.insert("end".to_string(), 3usize);

        let offsets = resolve_layout(&mut instrs, &labels);
        assert_eq!(instrs[0].offset, 0);
        assert_eq!(instrs[1].offset, 2);
        assert_eq!(instrs[2].offset, 4);
        assert_eq!(instrs[3].offset, 5);
        assert_eq!(offsets["end"], 5);
    }

    #[test]
    fn resolve_layout_past_end() {
        let mut instrs = vec![
            Instruction::new(Op::DONE, vec![]), // 1 byte
        ];
        let mut labels = HashMap::new();
        labels.insert("after_end".to_string(), 1usize); // past last instruction

        let offsets = resolve_layout(&mut instrs, &labels);
        assert_eq!(offsets["after_end"], 1);
    }

    #[test]
    fn optimise_jumps_shrinks() {
        let mut instrs = vec![
            Instruction::new(Op::PUSH1, vec![Operand::Imm(0)]), // 2
            Instruction::new(Op::JUMP_FALSE4, vec![Operand::Label("L".into())]), // 5
            Instruction::new(Op::PUSH1, vec![Operand::Imm(1)]), // 2
            Instruction::new(Op::DONE, vec![]),                 // 1, offset=9
        ];
        let mut labels = HashMap::new();
        labels.insert("L".to_string(), 3usize);

        optimise_jumps(&mut instrs, &labels, 10);
        // Relative offset is small enough for jump1
        assert_eq!(instrs[1].op, Op::JUMP_FALSE1);
    }

    #[test]
    fn optimise_jumps_preserves_switch_labels() {
        let mut instrs = vec![
            Instruction::new(Op::PUSH1, vec![Operand::Imm(0)]),
            Instruction::new(Op::JUMP4, vec![Operand::Label("switch_arm".into())]),
            Instruction::new(Op::DONE, vec![]),
        ];
        let mut labels = HashMap::new();
        labels.insert("switch_arm".to_string(), 2usize);

        optimise_jumps(&mut instrs, &labels, 10);
        // switch_ labels should NOT be shortened
        assert_eq!(instrs[1].op, Op::JUMP4);
    }
}
