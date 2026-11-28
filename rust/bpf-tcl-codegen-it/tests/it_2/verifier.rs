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

use bpf_tcl_codegen::ebpf::emit::emit_program_for_target;
use bpf_tcl_codegen::ebpf::emit::{EbpfObject, TargetAbi};
use bpf_tcl_codegen::ebpf::insn::{JMP, R7};
use bpf_tcl_codegen::ebpf::verifier::*;

// eBPF opcode fields (the kernel's instruction-set document).
const CLASS_MASK: u8 = 0x07;
const JMP_X: u8 = 0x08;
use bpf_tcl_ir::compile_module;

fn kernel(src: &str, target: TargetAbi) -> EbpfObject {
    let m = compile_module(src).expect("compiles");
    emit_program_for_target(&m.programs[0].program, target).expect("emits")
}

#[test]
fn verdict_only_xdp_verifies() {
    crate::install_surfaces();
    let obj = kernel("when XDP { pass }\n", TargetAbi::KernelXdp);
    assert_eq!(verify(&obj), Vec::new());
}

#[test]
fn xdp_packet_read_verifies() {
    crate::install_surfaces();
    let obj = kernel(
        "when XDP { setbuf p ctx\n load16 dport p 36\n if {$dport == 22} { drop }\n pass }\n",
        TargetAbi::KernelXdp,
    );
    assert_eq!(verify(&obj), Vec::new());
}

#[test]
fn socket_packet_read_verifies() {
    crate::install_surfaces();
    let obj = kernel(
        "when SOCKET_FILTER { setbuf p ctx\n load16 v p 2\n accept {$v} }\n",
        TargetAbi::KernelSocketFilter,
    );
    assert_eq!(verify(&obj), Vec::new());
}

#[test]
fn map_program_verifies_with_relocations() {
    crate::install_surfaces();
    let obj = kernel(
        "when XDP {\n map hits array 4 8 4\n setint k 0\n map_get n hits {$k}\n\
         setint n1 {$n + 1}\n map_set hits {$k} {$n1}\n pass }\n",
        TargetAbi::KernelXdp,
    );
    assert_eq!(verify(&obj), Vec::new());
}

#[test]
fn a_packet_load_without_a_proof_is_flagged() {
    crate::install_surfaces();
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
    crate::install_surfaces();
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
