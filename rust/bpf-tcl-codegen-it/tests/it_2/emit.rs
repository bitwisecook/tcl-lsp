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

use bpf_tcl_ir::compile_module;

use bpf_tcl_codegen::ebpf::emit::*;
use bpf_tcl_codegen::ebpf::insn::*;
use bpf_tcl_ir::BpfDiag;

/// The register the emitter keeps the packet end in.
const REND: u8 = R7;

fn emit_src(src: &str) -> EbpfObject {
    let module = compile_module(src).expect("compiles");
    emit_program(&module.programs[0].program).expect("emits")
}

fn emit_kernel(src: &str, target: TargetAbi) -> EbpfObject {
    let module = compile_module(src).expect("compiles");
    emit_program_for_target(&module.programs[0].program, target).expect("emits for the kernel")
}

#[test]
fn accept_all_ends_in_exit() {
    crate::install_surfaces();
    let obj = emit_src("when SOCKET_FILTER { setbuf pkt ctx\n accept }\n");
    assert_eq!(obj.raw.len(), obj.insns.len() * 8);
    // Last instruction is EXIT (0x95).
    assert_eq!(obj.insns.last().unwrap().op, 0x95);
    // Prologue loads data/data_end from the metadata buffer (r1).
    assert_eq!(obj.insns[0], ldx(SZ_DW, R6, R1, 0)); // r6 = data
    assert_eq!(obj.insns[1], ldx(SZ_DW, R7, R1, 8)); // r7 = data_end
}

#[test]
fn kernel_xdp_verdict_only_has_no_context_prologue() {
    crate::install_surfaces();
    let obj = emit_kernel("when XDP { pass }\n", TargetAbi::KernelXdp);
    assert_eq!(obj.target_abi, TargetAbi::KernelXdp);
    // No prologue and no slot zeroing: `mov r1,2; stx; ldx r0; exit`.
    assert_eq!(obj.insns.len(), 4);
    assert_ne!(obj.insns[0], ldx(SZ_DW, R6, R1, 0));
    assert_eq!(obj.insns.last().unwrap().op, 0x95);
}

#[test]
fn kernel_xdp_reads_xdp_md_context_fields() {
    crate::install_surfaces();
    // `struct xdp_md`: data at byte 0, data_end at byte 4, both 32-bit.
    let obj = emit_kernel(
        "when XDP { setbuf p ctx\n load16 dport p 36\n pass }\n",
        TargetAbi::KernelXdp,
    );
    assert_eq!(obj.insns[0], ldx(SZ_W, R6, R1, 0)); // r6 = data (u32)
    assert_eq!(obj.insns[1], ldx(SZ_W, R7, R1, 4)); // r7 = data_end (u32)
}

#[test]
fn kernel_socket_reads_sk_buff_context_fields() {
    crate::install_surfaces();
    // `struct __sk_buff`: data at byte 76, data_end at byte 80.
    let obj = emit_kernel(
        "when SOCKET_FILTER { setbuf p ctx\n load16 dport p 2\n accept }\n",
        TargetAbi::KernelSocketFilter,
    );
    assert_eq!(obj.insns[0], ldx(SZ_W, R6, R1, 76));
    assert_eq!(obj.insns[1], ldx(SZ_W, R7, R1, 80));
}

#[test]
fn kernel_packet_load_has_dominating_bounds_proof() {
    crate::install_surfaces();
    let obj = emit_kernel(
        "when XDP { setbuf p ctx\n load16 dport p 36\n pass }\n",
        TargetAbi::KernelXdp,
    );
    // The load `*(u16*)(r6 + 36)` (op 0x69) must be preceded by a
    // `if r2 <= r7 goto +N` (JLE reg, op 0xbd) comparing against data_end.
    let load_pos = obj
        .insns
        .iter()
        .position(|i| i.op == (SZ_H | 0x61) && i.src == R6 && i.off == 36)
        .expect("a u16 packet load from r6+36");
    let has_proof = obj.insns[..load_pos]
        .iter()
        .any(|i| i.op == (JLE | 0x0d) && i.src == REND);
    assert!(
        has_proof,
        "load must be dominated by a data_end bounds check"
    );
}

#[test]
fn kernel_map_lookup_emits_pseudo_fd_and_helper_call() {
    crate::install_surfaces();
    let obj = emit_kernel(
        "when XDP {\n map hits array 4 8 4\n setint k 0\n map_get n hits {$k}\n pass }\n",
        TargetAbi::KernelXdp,
    );
    // One relocation for the single map-fd load.
    assert_eq!(obj.relocations.len(), 1);
    assert_eq!(obj.relocations[0].map_index, 0);
    // The relocated instruction is a pseudo ld_imm64 (src = PSEUDO_MAP_FD).
    let reloc_at = obj.relocations[0].insn_index as usize;
    assert_eq!(obj.insns[reloc_at].src, PSEUDO_MAP_FD);
    // A bpf_map_lookup_elem call (helper id 1) is present.
    assert!(
        obj.insns
            .iter()
            .any(|i| i.op == 0x85 && i.imm == FN_MAP_LOOKUP)
    );
}

#[test]
fn kernel_map_update_emits_update_helper() {
    crate::install_surfaces();
    let obj = emit_kernel(
        "when XDP {\n map hits array 4 8 4\n setint k 0\n map_set hits {$k} {7}\n pass }\n",
        TargetAbi::KernelXdp,
    );
    assert!(
        obj.insns
            .iter()
            .any(|i| i.op == 0x85 && i.imm == FN_MAP_UPDATE)
    );
}

#[test]
fn kernel_target_rejects_wrong_program_type() {
    crate::install_surfaces();
    let module = compile_module("when SOCKET_FILTER { accept }\n").expect("compiles");
    let err = emit_program_for_target(&module.programs[0].program, TargetAbi::KernelXdp)
        .expect_err("socket filter is not an XDP program");
    assert_eq!(err.code, BpfDiag::OutOfSubset);
}

#[test]
fn drop_is_mov0_exit() {
    crate::install_surfaces();
    let obj = emit_src("drop\n");
    let n = obj.insns.len();
    assert_eq!(obj.insns[n - 1].op, 0x95); // exit
    assert_eq!(obj.insns[n - 2].dst, R0); // ldx r0, [r10-..]
}

#[test]
fn division_uses_signed_ops() {
    crate::install_surfaces();
    // `/` and `%` on signed Tcl integers must emit the
    // signed eBPF ops (BPF_SDIV / BPF_SMOD = DIV / MOD with off == 1), not
    // the unsigned forms that mangle negative operands.
    let div_op = alu64_reg_off(DIV, R1, R2, 1).op;
    let mod_op = alu64_reg_off(MOD, R1, R2, 1).op;
    let src = "when SOCKET_FILTER {\n\
         setint x {0 - 8}\n\
         setint y {$x / 2}\n\
         setint z {$x % 3}\n\
         accept\n}\n";
    for obj in [
        emit_src(src),
        emit_kernel(src, TargetAbi::KernelSocketFilter),
    ] {
        let div = obj
            .insns
            .iter()
            .find(|i| i.op == div_op)
            .expect("a signed div instruction");
        assert_eq!(div.off, 1, "division must be signed (off==1)");
        let modi = obj
            .insns
            .iter()
            .find(|i| i.op == mod_op)
            .expect("a signed mod instruction");
        assert_eq!(modi.off, 1, "modulo must be signed (off==1)");
    }
}

#[test]
fn port_filter_has_branches() {
    crate::install_surfaces();
    let obj = emit_src(
        "when SOCKET_FILTER {\n\
         setbuf pkt ctx\n\
         pktlen len pkt\n\
         if {$len < 36} { accept }\n\
         load16 dport pkt 36\n\
         if {$dport == 22} { drop }\n\
         accept\n}\n",
    );
    // Contains at least one conditional jump (JNE branch dispatch, 0x55) and
    // a signed compare in a Cmp materialisation (JSLT 0xcd / JEQ 0x1d).
    assert!(obj.insns.iter().any(|i| i.op == (JNE | 0x05)));
    assert_eq!(obj.insns.last().unwrap().op, 0x95);
}
