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

//! An in-repo verifier *model*: a structural check of the safety properties the
//! Linux kernel verifier enforces, applied to our emitted kernel objects.
//!
//! This is **not** the kernel verifier — it does not run in a container without
//! privilege, and it does not model register range tracking in full. It is a
//! sound checker of the invariants our own codegen must maintain, so a codegen
//! regression that would make the kernel reject a program is caught in a
//! rootless test:
//!
//! - the program terminates in `exit` (no fall-through off the end);
//! - the context prologue reads `data` / `data_end` at the target's field
//!   offsets and width;
//! - every packet dereference is dominated by a `data_end` bounds proof;
//! - every pseudo map-fd `ld_imm64` has a matching relocation; and
//! - every stack access lies within the 512-byte frame.
//!
//! Genuine kernel-verifier acceptance is asserted separately, behind an
//! `#[ignore]`d privileged test (see `tests/kernel_load.rs`).

use crate::ebpf::emit::{EbpfObject, TargetAbi};
use crate::ebpf::insn::{Insn, PSEUDO_MAP_FD, R1, R6, R7, R10};

/// A property the verifier model found violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// The program does not end in an `exit` instruction.
    NoExit,
    /// The context prologue does not read `data`/`data_end` correctly.
    BadContextPrologue,
    /// A packet load at this instruction index has no dominating `data_end`
    /// bounds proof.
    UnprovenPacketLoad(usize),
    /// A pseudo map-fd load at this instruction index has no relocation.
    MissingRelocation(usize),
    /// A stack access at this instruction index is outside the 512-byte frame.
    StackOutOfRange(usize),
}

const CLASS_MASK: u8 = 0x07;
const LDX: u8 = 0x01;
const STX: u8 = 0x03;
const ST: u8 = 0x02;
const LD: u8 = 0x00;
const JMP: u8 = 0x05;
const DW_MODE: u8 = 0x18; // LD | IMM | DW low bits for a wide load
const EXIT_OP: u8 = 0x95;
const JMP_X: u8 = 0x08; // register-source jump

/// Verify an emitted kernel object against the model. Returns every violation
/// found (empty when the object satisfies the model).
///
/// A non-kernel (`rbpf`) object is not modelled here — the userspace VM has its
/// own load-time checks — so it always verifies clean.
#[must_use]
pub fn verify(obj: &EbpfObject) -> Vec<VerifyError> {
    if !obj.target_abi.is_kernel() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    check_exit(&obj.insns, &mut errors);
    check_prologue(obj, &mut errors);
    check_map_fd_relocations(obj, &mut errors);
    check_stack_accesses(&obj.insns, &mut errors);
    check_packet_loads(&obj.insns, &mut errors);
    errors
}

fn check_exit(insns: &[Insn], errors: &mut Vec<VerifyError>) {
    if insns.last().map(|i| i.op) != Some(EXIT_OP) {
        errors.push(VerifyError::NoExit);
    }
}

/// The prologue (when present) reads `data` into `r6` and `data_end` into `r7`
/// from `r1` at the target's field offsets. A verdict-only program has no
/// prologue and no packet access, so an absent prologue is only an error when
/// the program actually dereferences the packet.
fn check_prologue(obj: &EbpfObject, errors: &mut Vec<VerifyError>) {
    let insns = &obj.insns;
    let uses_packet = insns
        .iter()
        .any(|i| i.op & CLASS_MASK == LDX && i.src == R6);
    if !uses_packet {
        return;
    }
    let (data_off, data_end_off) = match obj.target_abi {
        TargetAbi::KernelXdp => (0i16, 4i16),
        TargetAbi::KernelSocketFilter => (76, 80),
        TargetAbi::RbpfFixedMbuff => return,
    };
    let ok = insns.len() >= 2
        && insns[0].op & CLASS_MASK == LDX
        && insns[0].dst == R6
        && insns[0].src == R1
        && insns[0].off == data_off
        && insns[1].op & CLASS_MASK == LDX
        && insns[1].dst == R7
        && insns[1].src == R1
        && insns[1].off == data_end_off;
    if !ok {
        errors.push(VerifyError::BadContextPrologue);
    }
}

/// Every pseudo map-fd `ld_imm64` (a wide load with `src == PSEUDO_MAP_FD`) must
/// have a relocation recorded at its index.
fn check_map_fd_relocations(obj: &EbpfObject, errors: &mut Vec<VerifyError>) {
    for (i, insn) in obj.insns.iter().enumerate() {
        let is_wide_load = insn.op & CLASS_MASK == LD && insn.op & DW_MODE == DW_MODE;
        if is_wide_load && insn.src == PSEUDO_MAP_FD {
            let has_reloc = obj.relocations.iter().any(|r| r.insn_index as usize == i);
            if !has_reloc {
                errors.push(VerifyError::MissingRelocation(i));
            }
        }
    }
}

/// Stack accesses (base `r10`) must land within `[-512, 0)`.
fn check_stack_accesses(insns: &[Insn], errors: &mut Vec<VerifyError>) {
    for (i, insn) in insns.iter().enumerate() {
        let class = insn.op & CLASS_MASK;
        let base = match class {
            LDX => insn.src,
            STX | ST => insn.dst,
            _ => continue,
        };
        if base == R10 && (insn.off >= 0 || insn.off < -512) {
            errors.push(VerifyError::StackOutOfRange(i));
        }
    }
}

/// Every packet dereference (`ldx` from `r6`, past the prologue) must be
/// dominated by a `data_end` comparison. The emitter places the proof
/// immediately before the load — `if (data + off + width) <= data_end goto` — so
/// the model requires a register-source jump comparing against `r7` in the four
/// instructions preceding the load.
fn check_packet_loads(insns: &[Insn], errors: &mut Vec<VerifyError>) {
    for (i, insn) in insns.iter().enumerate() {
        // The prologue's `r6 = *(u32*)(r1+..)` reads from r1, not r6; a packet
        // load reads from r6.
        if insn.op & CLASS_MASK != LDX || insn.src != R6 {
            continue;
        }
        let window_start = i.saturating_sub(4);
        let proven = insns[window_start..i]
            .iter()
            .any(|w| w.op & CLASS_MASK == JMP && w.op & JMP_X != 0 && w.src == R7);
        if !proven {
            errors.push(VerifyError::UnprovenPacketLoad(i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebpf::emit::emit_program_for_target;
    use bpf_tcl_ir::compile_module;

    fn kernel(src: &str, target: TargetAbi) -> EbpfObject {
        let m = compile_module(src).expect("compiles");
        emit_program_for_target(&m.programs[0].program, target).expect("emits")
    }

    #[test]
    fn verdict_only_xdp_verifies() {
        let obj = kernel("when XDP { pass }\n", TargetAbi::KernelXdp);
        assert_eq!(verify(&obj), Vec::new());
    }

    #[test]
    fn xdp_packet_read_verifies() {
        let obj = kernel(
            "when XDP { setbuf p ctx\n load16 dport p 36\n if {$dport == 22} { drop }\n pass }\n",
            TargetAbi::KernelXdp,
        );
        assert_eq!(verify(&obj), Vec::new());
    }

    #[test]
    fn socket_packet_read_verifies() {
        let obj = kernel(
            "when SOCKET_FILTER { setbuf p ctx\n load16 v p 2\n accept {$v} }\n",
            TargetAbi::KernelSocketFilter,
        );
        assert_eq!(verify(&obj), Vec::new());
    }

    #[test]
    fn map_program_verifies_with_relocations() {
        let obj = kernel(
            "when XDP {\n map hits array 4 8 4\n setint k 0\n map_get n hits {$k}\n\
             setint n1 {$n + 1}\n map_set hits {$k} {$n1}\n pass }\n",
            TargetAbi::KernelXdp,
        );
        assert_eq!(verify(&obj), Vec::new());
    }

    #[test]
    fn a_packet_load_without_a_proof_is_flagged() {
        // Corrupt an emitted object: turn the dominating bounds check into a
        // no-op so the following packet load is left unproven.
        let mut obj = kernel(
            "when XDP { setbuf p ctx\n load16 dport p 36\n pass }\n",
            TargetAbi::KernelXdp,
        );
        // Find the JLE-against-r7 proof and neutralise it.
        for insn in &mut obj.insns {
            if insn.op & CLASS_MASK == JMP && insn.op & JMP_X != 0 && insn.src == R7 {
                insn.op = 0x07; // an ALU op, no longer a data_end compare
                break;
            }
        }
        let errors = verify(&obj);
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, VerifyError::UnprovenPacketLoad(_))),
            "a load with its proof removed must be flagged: {errors:?}"
        );
    }

    #[test]
    fn a_missing_relocation_is_flagged() {
        let mut obj = kernel(
            "when XDP { map hits array 4 8 4\n setint k 0\n map_get n hits {$k}\n pass }\n",
            TargetAbi::KernelXdp,
        );
        obj.relocations.clear();
        assert!(
            verify(&obj)
                .iter()
                .any(|e| matches!(e, VerifyError::MissingRelocation(_)))
        );
    }
}
