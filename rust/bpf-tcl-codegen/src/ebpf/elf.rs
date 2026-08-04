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

//! A minimal ELF relocatable-object writer for our eBPF programs.
//!
//! Produces a standards-valid `ET_REL` / `EM_BPF` ELF64 (little-endian) the way
//! `clang -target bpf` does — written by hand, no LLVM — so a program can be
//! inspected and disassembled with the usual tools (`readelf`, `llvm-objdump`)
//! and loaded by libbpf.
//!
//! Two layouts:
//!
//!   * **map-free** — a program section named for the program type
//!     (`xdp` / `socket`), a `license` section (`"GPL"`), a `.symtab`/`.strtab`
//!     with a section symbol and a global `STT_FUNC` program symbol, and a
//!     `.shstrtab`. This is the six-section object the audited kernel-XDP slice
//!     already loaded.
//!   * **with maps** (kernel targets only) — additionally a BTF-defined `.maps`
//!     section, a `.BTF` section describing each map, a `.rel<prog>` relocation
//!     section patching each pseudo map-fd `ld_imm64`, a `.maps` section symbol,
//!     and a global `STT_OBJECT` symbol per map. libbpf resolves each relocation
//!     to the created map's file descriptor at load time.

use bpf_tcl_ir::ir::ProgType;

use crate::ebpf::btf::{MAP_STRUCT_SIZE, build_btf};
use crate::ebpf::emit::EbpfObject;

// ELF identification / header constants.
const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ET_REL: u16 = 1;
/// `EM_BPF` — the eBPF machine type.
const EM_BPF: u16 = 247;

// Section header types / flags.
const SHT_PROGBITS: u32 = 1;
const SHT_SYMTAB: u32 = 2;
const SHT_STRTAB: u32 = 3;
const SHT_REL: u32 = 9;
const SHF_WRITE: u64 = 0x1;
const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;

// Symbol binding / type.
const STB_GLOBAL: u8 = 1;
const STT_OBJECT: u8 = 1;
const STT_FUNC: u8 = 2;
const STT_SECTION: u8 = 3;

/// `R_BPF_64_64` — the relocation the loader applies to a pseudo map-fd
/// `ld_imm64`, rewriting its immediate to the created map's file descriptor.
const R_BPF_64_64: u32 = 1;

// Fixed sizes (ELF64).
const EHDR_SIZE: u64 = 64;
const SHDR_SIZE: usize = 64;
const SYM_SIZE: usize = 24;
const REL_SIZE: usize = 16;

// Map-free section indices (NULL, prog, license, symtab, strtab, shstrtab).
const SEC_PROG: u16 = 1;
const SEC_STRTAB: u16 = 4;
const SEC_SHSTRTAB: u16 = 5;
const SEC_COUNT: u16 = 6;
/// Index of the first global symbol in the map-free layout.
const FIRST_GLOBAL_SYM: u32 = 2;

/// An error from the ELF writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElfError {
    /// A program declaring maps was emitted for the userspace `rbpf` simulator
    /// ABI, which has no kernel map file descriptors. Emit maps with a kernel
    /// target (`--target kernel-xdp` / `kernel-socket`) instead.
    MapsNeedKernelTarget,
}

impl std::fmt::Display for ElfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfError::MapsNeedKernelTarget => write!(
                f,
                "maps in an ELF object need a kernel target (use --target kernel-xdp or kernel-socket)"
            ),
        }
    }
}

impl std::error::Error for ElfError {}

/// A growable string table (NUL-led, NUL-terminated entries).
struct StrTab {
    buf: Vec<u8>,
}

impl StrTab {
    fn new() -> Self {
        // Index 0 is the empty string by convention.
        Self { buf: vec![0] }
    }

    fn add(&mut self, s: &str) -> u32 {
        let off = u32::try_from(self.buf.len()).expect("string table < 4 GiB");
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
        off
    }
}

/// One ELF symbol-table entry, ready to encode.
struct Sym {
    name: u32,
    info: u8,
    shndx: u16,
    value: u64,
    size: u64,
}

impl Sym {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.name.to_le_bytes());
        out.push(self.info);
        out.push(0); // st_other
        out.extend_from_slice(&self.shndx.to_le_bytes());
        out.extend_from_slice(&self.value.to_le_bytes());
        out.extend_from_slice(&self.size.to_le_bytes());
    }
}

/// The conventional `SEC(...)` name for a program type.
fn section_name(prog_type: ProgType) -> &'static str {
    match prog_type {
        ProgType::Xdp => "xdp",
        ProgType::SocketFilter => "socket",
    }
}

/// Write `obj` as an `EM_BPF` `ET_REL` ELF64 object, with the program entry
/// exported as `prog_name`.
///
/// # Errors
/// [`ElfError::MapsNeedKernelTarget`] if a program declaring maps was emitted
/// for the userspace `rbpf` simulator ABI (no kernel map fds exist there).
pub fn write_object(obj: &EbpfObject, prog_name: &str) -> Result<Vec<u8>, ElfError> {
    if obj.maps.is_empty() {
        return Ok(write_map_free(obj, prog_name));
    }
    if !obj.target_abi.is_kernel() {
        return Err(ElfError::MapsNeedKernelTarget);
    }
    Ok(write_with_maps(obj, prog_name))
}

/// The computed offset/size of one laid-out section (file offset, byte size).
type SecLoc = (u64, u64);

/// The map-free six-section layout (unchanged from the original writer, so its
/// section indices and symbol table stay stable for existing consumers).
struct Layout {
    names: [u32; 5],
    prog: SecLoc,
    license: SecLoc,
    symtab: SecLoc,
    strtab: SecLoc,
    shstrtab: SecLoc,
}

fn write_map_free(obj: &EbpfObject, prog_name: &str) -> Vec<u8> {
    let mut shstr = StrTab::new();
    let names = [
        shstr.add(section_name(obj.prog_type)),
        shstr.add("license"),
        shstr.add(".symtab"),
        shstr.add(".strtab"),
        shstr.add(".shstrtab"),
    ];
    let mut strtab = StrTab::new();
    let prog_size = obj.raw.len() as u64;
    let symtab = build_map_free_symtab(strtab.add(prog_name), prog_size);

    let mut data = Vec::new();
    let prog_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&obj.raw);
    let license_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(b"GPL\0");
    align(&mut data, 8);
    let symtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&symtab);
    let strtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&strtab.buf);
    let shstrtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&shstr.buf);
    align(&mut data, 8);
    let shoff = EHDR_SIZE + data.len() as u64;

    let secs = map_free_section_headers(&Layout {
        names,
        prog: (prog_off, prog_size),
        license: (license_off, 4),
        symtab: (symtab_off, symtab.len() as u64),
        strtab: (strtab_off, strtab.buf.len() as u64),
        shstrtab: (shstrtab_off, shstr.buf.len() as u64),
    });

    let cap = usize::try_from(shoff).unwrap_or(0) + usize::from(SEC_COUNT) * SHDR_SIZE;
    let mut out = Vec::with_capacity(cap);
    write_ehdr(&mut out, shoff, SEC_COUNT, SEC_SHSTRTAB);
    out.extend_from_slice(&data);
    for s in &secs {
        s.encode(&mut out);
    }
    out
}

/// Build the map-free symbol table: NULL, the program section symbol (local),
/// then the global `STT_FUNC` for the entry.
fn build_map_free_symtab(prog_sym_name: u32, prog_size: u64) -> Vec<u8> {
    let mut t = Vec::with_capacity(3 * SYM_SIZE);
    Sym {
        name: 0,
        info: 0,
        shndx: 0,
        value: 0,
        size: 0,
    }
    .encode(&mut t);
    Sym {
        name: 0,
        info: STT_SECTION,
        shndx: SEC_PROG,
        value: 0,
        size: 0,
    }
    .encode(&mut t);
    Sym {
        name: prog_sym_name,
        info: (STB_GLOBAL << 4) | STT_FUNC,
        shndx: SEC_PROG,
        value: 0,
        size: prog_size,
    }
    .encode(&mut t);
    t
}

/// Build the six map-free section headers.
fn map_free_section_headers(l: &Layout) -> [Shdr; 6] {
    [
        Shdr::null(),
        Shdr {
            name: l.names[0],
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_EXECINSTR,
            offset: l.prog.0,
            size: l.prog.1,
            link: 0,
            info: 0,
            align: 8,
            entsize: 0,
        },
        Shdr {
            name: l.names[1],
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC,
            offset: l.license.0,
            size: l.license.1,
            link: 0,
            info: 0,
            align: 1,
            entsize: 0,
        },
        Shdr {
            name: l.names[2],
            kind: SHT_SYMTAB,
            flags: 0,
            offset: l.symtab.0,
            size: l.symtab.1,
            link: u32::from(SEC_STRTAB),
            info: FIRST_GLOBAL_SYM,
            align: 8,
            entsize: SYM_SIZE as u64,
        },
        Shdr {
            name: l.names[3],
            kind: SHT_STRTAB,
            flags: 0,
            offset: l.strtab.0,
            size: l.strtab.1,
            link: 0,
            info: 0,
            align: 1,
            entsize: 0,
        },
        Shdr {
            name: l.names[4],
            kind: SHT_STRTAB,
            flags: 0,
            offset: l.shstrtab.0,
            size: l.shstrtab.1,
            link: 0,
            info: 0,
            align: 1,
            entsize: 0,
        },
    ]
}

// Section indices in the map-carrying layout: NULL(0), prog(1), .maps(2),
// .BTF(3), .rel<prog>(4), license(5), .symtab(6), .strtab(7), .shstrtab(8).
const M_PROG: u16 = 1;
const M_MAPS: u16 = 2;
const M_SYMTAB: u16 = 6;
const M_STRTAB: u16 = 7;
const M_SHSTRTAB: u16 = 8;
const M_SEC_COUNT: u16 = 9;

/// The named-section offsets for the map-carrying layout (`.shstrtab` entries).
struct MapNames {
    prog: u32,
    maps: u32,
    btf: u32,
    rel: u32,
    license: u32,
    symtab: u32,
    strtab: u32,
    shstrtab: u32,
}

fn write_with_maps(obj: &EbpfObject, prog_name: &str) -> Vec<u8> {
    let n_maps = obj.maps.len();
    let maps_size = u32::try_from(n_maps).unwrap_or(0) * MAP_STRUCT_SIZE;
    let btf = build_btf(&obj.maps);

    let mut shstr = StrTab::new();
    let rel_name = format!(".rel{}", section_name(obj.prog_type));
    let names = MapNames {
        prog: shstr.add(section_name(obj.prog_type)),
        maps: shstr.add(".maps"),
        btf: shstr.add(".BTF"),
        rel: shstr.add(&rel_name),
        license: shstr.add("license"),
        symtab: shstr.add(".symtab"),
        strtab: shstr.add(".strtab"),
        shstrtab: shstr.add(".shstrtab"),
    };

    // Symbol table: NULL, prog section sym, .maps section sym (locals), then a
    // global object symbol per map, then the global program FUNC.
    let mut strtab = StrTab::new();
    let map_sym_base = 3u32; // first global symbol index
    let mut symtab = Vec::new();
    encode_local_syms(&mut symtab);
    let mut map_names: Vec<u32> = Vec::with_capacity(n_maps);
    for def in &obj.maps {
        map_names.push(strtab.add(&def.name));
    }
    for (i, name) in map_names.iter().enumerate() {
        Sym {
            name: *name,
            info: (STB_GLOBAL << 4) | STT_OBJECT,
            shndx: M_MAPS,
            value: u64::from(u32::try_from(i).unwrap_or(0) * MAP_STRUCT_SIZE),
            size: u64::from(MAP_STRUCT_SIZE),
        }
        .encode(&mut symtab);
    }
    let prog_size = obj.raw.len() as u64;
    Sym {
        name: strtab.add(prog_name),
        info: (STB_GLOBAL << 4) | STT_FUNC,
        shndx: M_PROG,
        value: 0,
        size: prog_size,
    }
    .encode(&mut symtab);

    let relocs = encode_relocs(obj, map_sym_base);

    // Lay out section data after the ELF header.
    let mut data = Vec::new();
    let prog_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&obj.raw);
    align(&mut data, 8);
    let maps_off = EHDR_SIZE + data.len() as u64;
    data.extend(std::iter::repeat_n(0u8, maps_size as usize));
    align(&mut data, 8);
    let btf_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&btf);
    align(&mut data, 8);
    let rel_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&relocs);
    let license_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(b"GPL\0");
    align(&mut data, 8);
    let symtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&symtab);
    let strtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&strtab.buf);
    let shstrtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&shstr.buf);
    align(&mut data, 8);
    let shoff = EHDR_SIZE + data.len() as u64;

    let first_global = map_sym_base;
    let secs = maps_section_headers(
        &names,
        &[
            (prog_off, prog_size),
            (maps_off, u64::from(maps_size)),
            (btf_off, btf.len() as u64),
            (rel_off, relocs.len() as u64),
            (license_off, 4),
            (symtab_off, symtab.len() as u64),
            (strtab_off, strtab.buf.len() as u64),
            (shstrtab_off, shstr.buf.len() as u64),
        ],
        first_global,
    );

    let cap = usize::try_from(shoff).unwrap_or(0) + usize::from(M_SEC_COUNT) * SHDR_SIZE;
    let mut out = Vec::with_capacity(cap);
    write_ehdr(&mut out, shoff, M_SEC_COUNT, M_SHSTRTAB);
    out.extend_from_slice(&data);
    for s in &secs {
        s.encode(&mut out);
    }
    out
}

/// NULL + the two local section symbols (prog, .maps).
fn encode_local_syms(out: &mut Vec<u8>) {
    Sym {
        name: 0,
        info: 0,
        shndx: 0,
        value: 0,
        size: 0,
    }
    .encode(out);
    Sym {
        name: 0,
        info: STT_SECTION,
        shndx: M_PROG,
        value: 0,
        size: 0,
    }
    .encode(out);
    Sym {
        name: 0,
        info: STT_SECTION,
        shndx: M_MAPS,
        value: 0,
        size: 0,
    }
    .encode(out);
}

/// Encode the `SHT_REL` entries: one per pseudo map-fd load, at the load's byte
/// offset, referencing the map's global object symbol.
fn encode_relocs(obj: &EbpfObject, map_sym_base: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(obj.relocations.len() * REL_SIZE);
    for r in &obj.relocations {
        let r_offset = u64::from(r.insn_index) * 8;
        let sym = map_sym_base + r.map_index;
        let r_info = (u64::from(sym) << 32) | u64::from(R_BPF_64_64);
        out.extend_from_slice(&r_offset.to_le_bytes());
        out.extend_from_slice(&r_info.to_le_bytes());
    }
    out
}

fn maps_section_headers(names: &MapNames, locs: &[SecLoc; 8], first_global: u32) -> [Shdr; 9] {
    let [prog, maps, btf, rel, license, symtab, strtab, shstrtab] = *locs;
    [
        Shdr::null(),
        Shdr {
            name: names.prog,
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_EXECINSTR,
            offset: prog.0,
            size: prog.1,
            link: 0,
            info: 0,
            align: 8,
            entsize: 0,
        },
        Shdr {
            name: names.maps,
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC | SHF_WRITE,
            offset: maps.0,
            size: maps.1,
            link: 0,
            info: 0,
            align: 8,
            entsize: 0,
        },
        Shdr {
            name: names.btf,
            kind: SHT_PROGBITS,
            flags: 0,
            offset: btf.0,
            size: btf.1,
            link: 0,
            info: 0,
            align: 4,
            entsize: 0,
        },
        Shdr {
            name: names.rel,
            kind: SHT_REL,
            flags: 0,
            offset: rel.0,
            size: rel.1,
            link: u32::from(M_SYMTAB),
            info: u32::from(M_PROG),
            align: 8,
            entsize: REL_SIZE as u64,
        },
        Shdr {
            name: names.license,
            kind: SHT_PROGBITS,
            flags: SHF_ALLOC,
            offset: license.0,
            size: license.1,
            link: 0,
            info: 0,
            align: 1,
            entsize: 0,
        },
        Shdr {
            name: names.symtab,
            kind: SHT_SYMTAB,
            flags: 0,
            offset: symtab.0,
            size: symtab.1,
            link: u32::from(M_STRTAB),
            info: first_global,
            align: 8,
            entsize: SYM_SIZE as u64,
        },
        Shdr {
            name: names.strtab,
            kind: SHT_STRTAB,
            flags: 0,
            offset: strtab.0,
            size: strtab.1,
            link: 0,
            info: 0,
            align: 1,
            entsize: 0,
        },
        Shdr {
            name: names.shstrtab,
            kind: SHT_STRTAB,
            flags: 0,
            offset: shstrtab.0,
            size: shstrtab.1,
            link: 0,
            info: 0,
            align: 1,
            entsize: 0,
        },
    ]
}

/// Pad `v` with zero bytes up to a multiple of `to`.
fn align(v: &mut Vec<u8>, to: usize) {
    while !v.len().is_multiple_of(to) {
        v.push(0);
    }
}

/// Write the 64-byte ELF header.
fn write_ehdr(out: &mut Vec<u8>, shoff: u64, shnum: u16, shstrndx: u16) {
    out.extend_from_slice(&ELF_MAGIC);
    out.push(ELFCLASS64);
    out.push(ELFDATA2LSB);
    out.push(EV_CURRENT);
    out.extend_from_slice(&[0u8; 9]); // EI_OSABI, EI_ABIVERSION, padding.
    out.extend_from_slice(&ET_REL.to_le_bytes());
    out.extend_from_slice(&EM_BPF.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes()); // e_version
    out.extend_from_slice(&0u64.to_le_bytes()); // e_entry
    out.extend_from_slice(&0u64.to_le_bytes()); // e_phoff
    out.extend_from_slice(&shoff.to_le_bytes()); // e_shoff
    out.extend_from_slice(&0u32.to_le_bytes()); // e_flags
    out.extend_from_slice(&64u16.to_le_bytes()); // e_ehsize = sizeof(Elf64_Ehdr)
    out.extend_from_slice(&0u16.to_le_bytes()); // e_phentsize
    out.extend_from_slice(&0u16.to_le_bytes()); // e_phnum
    out.extend_from_slice(&64u16.to_le_bytes()); // e_shentsize = sizeof(Elf64_Shdr)
    out.extend_from_slice(&shnum.to_le_bytes()); // e_shnum
    out.extend_from_slice(&shstrndx.to_le_bytes()); // e_shstrndx
}

/// One ELF section header, ready to encode.
struct Shdr {
    name: u32,
    kind: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    align: u64,
    entsize: u64,
}

impl Shdr {
    fn null() -> Self {
        Self {
            name: 0,
            kind: 0,
            flags: 0,
            offset: 0,
            size: 0,
            link: 0,
            info: 0,
            align: 0,
            entsize: 0,
        }
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.name.to_le_bytes());
        out.extend_from_slice(&self.kind.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&0u64.to_le_bytes()); // sh_addr
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.size.to_le_bytes());
        out.extend_from_slice(&self.link.to_le_bytes());
        out.extend_from_slice(&self.info.to_le_bytes());
        out.extend_from_slice(&self.align.to_le_bytes());
        out.extend_from_slice(&self.entsize.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebpf::emit::{TargetAbi, emit_program, emit_program_for_target};
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
    fn header_is_em_bpf_rel_elf64_le() {
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
        let o = write_object(&obj_for("when XDP { drop }\n"), "xdp").unwrap();
        assert!(contains(&o, b"xdp\0"));
        assert!(contains(&o, b"license\0"));
        assert!(contains(&o, b".symtab\0"));
    }

    #[test]
    fn socket_filter_section_is_named_socket() {
        let o = write_object(&obj_for("when SOCKET_FILTER { drop }\n"), "socket_filter").unwrap();
        assert!(contains(&o, b"socket\0"));
        assert!(contains(&o, b"socket_filter\0")); // the exported FUNC symbol
    }

    #[test]
    fn func_symbol_describes_the_program() {
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
        let src = "when SOCKET_FILTER { map m hash 8 8 8\n map_set m {0} {1}\n accept }\n";
        let err = write_object(&obj_for(src), "socket_filter").unwrap_err();
        assert_eq!(err, ElfError::MapsNeedKernelTarget);
    }

    #[test]
    fn section_count_and_shstrndx_consistent() {
        let o = write_object(&obj_for("when XDP { drop }\n"), "xdp").unwrap();
        let shoff = le64(&o, 40);
        assert_eq!(o.len(), shoff + usize::from(SEC_COUNT) * 64);
    }

    #[test]
    fn kernel_map_object_has_maps_btf_and_reloc_sections() {
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
        // A kernel object with no maps still uses the six-section layout.
        let obj = kernel_obj("when XDP { pass }\n", TargetAbi::KernelXdp);
        let o = write_object(&obj, "xdp").unwrap();
        assert_eq!(le16(&o, 60), SEC_COUNT);
    }
}
