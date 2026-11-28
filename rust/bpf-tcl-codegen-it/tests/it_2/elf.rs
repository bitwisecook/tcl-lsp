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

use bpf_tcl_codegen::ebpf::elf::*;
use bpf_tcl_codegen::ebpf::emit::EbpfObject;
use tcl_core_types::ArtifactStringError;

// ELF64 values (the System V ABI and its eBPF supplement).
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_REL: u16 = 1;
const EM_BPF: u16 = 247;
const SHT_SYMTAB: u32 = 2;
const SHT_REL: u32 = 9;
const STB_GLOBAL: u8 = 1;
const STT_FUNC: u8 = 2;
const SYM_SIZE: usize = 24;
const REL_SIZE: usize = 16;

// The object's section layout: without maps, NULL, prog, license, symtab,
// strtab and shstrtab; with maps, NULL, prog, .maps, .BTF, .rel<prog>,
// license, .symtab, .strtab and .shstrtab.
const SEC_PROG: u16 = 1;
const SEC_SHSTRTAB: u16 = 5;
const SEC_COUNT: u16 = 6;
const M_PROG: u16 = 1;
const M_SHSTRTAB: u16 = 8;
const M_SEC_COUNT: u16 = 9;
use bpf_tcl_codegen::ebpf::emit::{TargetAbi, emit_program, emit_program_for_target};
use bpf_tcl_ir::compile_module;

fn obj_for(src: &str) -> EbpfObject {
    let m = compile_module(src).expect("compiles");
    emit_program(&m.programs[0].program).expect("emits")
}

fn kernel_obj(src: &str, target: TargetAbi) -> EbpfObject {
    let m = compile_module(src).expect("compiles");
    emit_program_for_target(&m.programs[0].program, target).expect("emits")
}

fn le16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn le64(b: &[u8], o: usize) -> usize {
    usize::try_from(u64::from_le_bytes(b[o..o + 8].try_into().unwrap())).unwrap()
}
fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn direct_program_name_ingress_refuses_nul_in_both_layouts() {
    crate::install_surfaces();
    let objects = [
        obj_for("when XDP { drop }\n"),
        kernel_obj(
            "when XDP { map m hash 8 8 16\n pass }\n",
            TargetAbi::KernelXdp,
        ),
    ];
    for obj in objects {
        assert!(write_object(&obj, "entry").is_ok());
        assert_eq!(
            write_object(&obj, "entry\0suffix"),
            Err(ElfError::ArtifactString(ArtifactStringError::InteriorNul {
                position: 5
            }))
        );
    }
}

#[test]
fn direct_map_name_ingress_refuses_nul_and_keeps_target_error_precedence() {
    crate::install_surfaces();
    let src = "when XDP { map m hash 8 8 16\n pass }\n";
    let mut obj = kernel_obj(src, TargetAbi::KernelXdp);
    assert!(write_object(&obj, "entry").is_ok());
    obj.maps[0].name = "m\0suffix".into();
    assert_eq!(
        write_object(&obj, "entry"),
        Err(ElfError::ArtifactString(ArtifactStringError::InteriorNul {
            position: 1
        }))
    );
    let mut rbpf = obj_for(src);
    rbpf.maps[0].name = "m\0suffix".into();
    assert_eq!(
        write_object(&rbpf, "entry\0suffix"),
        Err(ElfError::MapsNeedKernelTarget)
    );
}

#[test]
fn header_is_em_bpf_rel_elf64_le() {
    crate::install_surfaces();
    let o = write_object(&obj_for("when XDP { drop }\n"), "xdp").unwrap();
    assert_eq!(&o[0..4], &ELF_MAGIC);
    assert_eq!(o[4], ELFCLASS64);
    assert_eq!(o[5], ELFDATA2LSB);
    assert_eq!(le16(&o, 16), ET_REL);
    assert_eq!(le16(&o, 18), EM_BPF);
    assert_eq!(le16(&o, 60), SEC_COUNT); // e_shnum
    assert_eq!(le16(&o, 62), SEC_SHSTRTAB); // e_shstrndx
}

#[test]
fn xdp_program_section_is_named_xdp() {
    crate::install_surfaces();
    let o = write_object(&obj_for("when XDP { drop }\n"), "xdp").unwrap();
    assert!(contains(&o, b"xdp\0"));
    assert!(contains(&o, b"license\0"));
    assert!(contains(&o, b".symtab\0"));
}

#[test]
fn socket_filter_section_is_named_socket() {
    crate::install_surfaces();
    let o = write_object(&obj_for("when SOCKET_FILTER { drop }\n"), "socket_filter").unwrap();
    assert!(contains(&o, b"socket\0"));
    assert!(contains(&o, b"socket_filter\0")); // the exported FUNC symbol
}

#[test]
fn func_symbol_describes_the_program() {
    crate::install_surfaces();
    let obj = obj_for("when XDP { drop }\n");
    let raw_len = obj.raw.len();
    let o = write_object(&obj, "xdp").unwrap();

    let shoff = le64(&o, 40);
    let mut symtab: Option<(usize, usize)> = None;
    for i in 0..usize::from(SEC_COUNT) {
        let sh = shoff + i * 64;
        if le32(&o, sh + 4) == SHT_SYMTAB {
            symtab = Some((le64(&o, sh + 24), le64(&o, sh + 32)));
        }
    }
    let (off, size) = symtab.expect("symtab present");
    assert_eq!(size, 3 * SYM_SIZE);

    let s2 = off + 2 * SYM_SIZE;
    assert_eq!(o[s2 + 4], (STB_GLOBAL << 4) | STT_FUNC);
    assert_eq!(le16(&o, s2 + 6), SEC_PROG);
    assert_eq!(le64(&o, s2 + 16), raw_len);
}

#[test]
fn rbpf_maps_need_a_kernel_target() {
    crate::install_surfaces();
    let src = "when SOCKET_FILTER { map m hash 8 8 8\n map_set m {0} {1}\n accept }\n";
    let err = write_object(&obj_for(src), "socket_filter").unwrap_err();
    assert_eq!(err, ElfError::MapsNeedKernelTarget);
}

#[test]
fn section_count_and_shstrndx_consistent() {
    crate::install_surfaces();
    let o = write_object(&obj_for("when XDP { drop }\n"), "xdp").unwrap();
    let shoff = le64(&o, 40);
    assert_eq!(o.len(), shoff + usize::from(SEC_COUNT) * 64);
}

#[test]
fn kernel_map_object_has_maps_btf_and_reloc_sections() {
    crate::install_surfaces();
    let obj = kernel_obj(
        "when XDP {\n map hits array 4 8 4\n setint k 0\n map_get n hits {$k}\n pass }\n",
        TargetAbi::KernelXdp,
    );
    let o = write_object(&obj, "xdp").unwrap();
    assert_eq!(le16(&o, 60), M_SEC_COUNT); // e_shnum
    assert_eq!(le16(&o, 62), M_SHSTRTAB);
    assert!(contains(&o, b".maps\0"));
    assert!(contains(&o, b".BTF\0"));
    assert!(contains(&o, b".relxdp\0"));
    assert!(contains(&o, b"hits\0"));

    // Walk to the SHT_REL section and confirm one entry pointing at the
    // program's map-fd load and the map's global object symbol.
    let shoff = le64(&o, 40);
    let mut rel: Option<(usize, usize, u32)> = None;
    for i in 0..usize::from(M_SEC_COUNT) {
        let sh = shoff + i * 64;
        if le32(&o, sh + 4) == SHT_REL {
            rel = Some((le64(&o, sh + 24), le64(&o, sh + 32), le32(&o, sh + 44)));
        }
    }
    let (off, size, info_sec) = rel.expect("rel section present");
    assert_eq!(size, REL_SIZE, "exactly one relocation");
    assert_eq!(info_sec, u32::from(M_PROG), "rel targets the prog section");
    let r_offset = le64(&o, off);
    assert_eq!(
        r_offset,
        obj.relocations[0].insn_index as usize * 8,
        "reloc offset is the map-fd load byte offset"
    );
    let sym = le32(&o, off + 12); // high word of r_info = symbol index
    assert_eq!(sym, 3, "references the first global (map) symbol");
}

#[test]
fn kernel_map_free_object_uses_the_compact_layout() {
    crate::install_surfaces();
    // A kernel object with no maps still uses the six-section layout.
    let obj = kernel_obj("when XDP { pass }\n", TargetAbi::KernelXdp);
    let o = write_object(&obj, "xdp").unwrap();
    assert_eq!(le16(&o, 60), SEC_COUNT);
}
