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
//!     `.shstrtab`. This is the six-section object layout the kernel loads for
//!     XDP programs.
//!   * **with maps** (kernel targets only) — additionally a BTF-defined `.maps`
//!     section, a `.BTF` section describing each map, a `.rel<prog>` relocation
//!     section patching each pseudo map-fd `ld_imm64`, a `.maps` section symbol,
//!     and a global `STT_OBJECT` symbol per map. libbpf resolves each relocation
//!     to the created map's file descriptor at load time.

use bpf_tcl_ir::ir::ProgType;
use tcl_core_types::{ArtifactStringError, ArtifactStringTable};

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
    /// An exact artifact name or string table cannot be represented.
    ArtifactString(ArtifactStringError),
    /// The actual registry rows do not supply one unambiguous section.
    EventSectionUnavailable(ProgType),
}

impl std::fmt::Display for ElfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfError::ArtifactString(error) => error.fmt(f),
            ElfError::EventSectionUnavailable(prog_type) => {
                write!(f, "no unambiguous registry ELF section for {prog_type:?}")
            }
            ElfError::MapsNeedKernelTarget => write!(
                f,
                "maps in an ELF object need a kernel target (use --target kernel-xdp or kernel-socket)"
            ),
        }
    }
}

impl std::error::Error for ElfError {}

impl From<ArtifactStringError> for ElfError {
    fn from(error: ArtifactStringError) -> Self {
        Self::ArtifactString(error)
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

/// Write `obj` as an `EM_BPF` `ET_REL` ELF64 object, with the program entry
/// exported as `prog_name`.
///
/// # Errors
/// [`ElfError::MapsNeedKernelTarget`] if a program declaring maps was emitted
/// for the userspace `rbpf` simulator ABI (no kernel map fds exist there).
/// Refuses unavailable or conflicting registry section metadata and names that
/// cannot be represented exactly in the artifact string tables.
pub fn write_object(obj: &EbpfObject, prog_name: &str) -> Result<Vec<u8>, ElfError> {
    if !obj.maps.is_empty() && !obj.target_abi.is_kernel() {
        return Err(ElfError::MapsNeedKernelTarget);
    }
    let section = bpf_tcl_ir::event::elf_section_for_prog_type(obj.prog_type)
        .ok_or(ElfError::EventSectionUnavailable(obj.prog_type))?;
    if obj.maps.is_empty() {
        write_map_free(obj, prog_name, section)
    } else {
        write_with_maps(obj, prog_name, section)
    }
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

fn write_map_free(obj: &EbpfObject, prog_name: &str, section: &str) -> Result<Vec<u8>, ElfError> {
    let mut shstr = ArtifactStringTable::new();
    let names = [
        shstr.add(section)?,
        shstr.add("license")?,
        shstr.add(".symtab")?,
        shstr.add(".strtab")?,
        shstr.add(".shstrtab")?,
    ];
    let mut strtab = ArtifactStringTable::new();
    let prog_size = obj.raw.len() as u64;
    let symtab = build_map_free_symtab(strtab.add(prog_name)?, prog_size);

    let mut data = Vec::new();
    let prog_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&obj.raw);
    let license_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(b"GPL\0");
    align(&mut data, 8);
    let symtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(&symtab);
    let strtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(strtab.as_bytes());
    let shstrtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(shstr.as_bytes());
    align(&mut data, 8);
    let shoff = EHDR_SIZE + data.len() as u64;

    let secs = map_free_section_headers(&Layout {
        names,
        prog: (prog_off, prog_size),
        license: (license_off, 4),
        symtab: (symtab_off, symtab.len() as u64),
        strtab: (strtab_off, strtab.as_bytes().len() as u64),
        shstrtab: (shstrtab_off, shstr.as_bytes().len() as u64),
    });

    let cap = usize::try_from(shoff).unwrap_or(0) + usize::from(SEC_COUNT) * SHDR_SIZE;
    let mut out = Vec::with_capacity(cap);
    write_ehdr(&mut out, shoff, SEC_COUNT, SEC_SHSTRTAB);
    out.extend_from_slice(&data);
    for s in &secs {
        s.encode(&mut out);
    }
    Ok(out)
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

fn write_with_maps(obj: &EbpfObject, prog_name: &str, section: &str) -> Result<Vec<u8>, ElfError> {
    let n_maps = obj.maps.len();
    let maps_size = u32::try_from(n_maps).unwrap_or(0) * MAP_STRUCT_SIZE;
    let btf = build_btf(&obj.maps)?;

    let mut shstr = ArtifactStringTable::new();
    let rel_name = format!(".rel{section}");
    let names = MapNames {
        prog: shstr.add(section)?,
        maps: shstr.add(".maps")?,
        btf: shstr.add(".BTF")?,
        rel: shstr.add(&rel_name)?,
        license: shstr.add("license")?,
        symtab: shstr.add(".symtab")?,
        strtab: shstr.add(".strtab")?,
        shstrtab: shstr.add(".shstrtab")?,
    };

    // Symbol table: NULL, prog section sym, .maps section sym (locals), then a
    // global object symbol per map, then the global program FUNC.
    let mut strtab = ArtifactStringTable::new();
    let map_sym_base = 3u32; // first global symbol index
    let mut symtab = Vec::new();
    encode_local_syms(&mut symtab);
    let mut map_names: Vec<u32> = Vec::with_capacity(n_maps);
    for def in &obj.maps {
        map_names.push(strtab.add(&def.name)?);
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
        name: strtab.add(prog_name)?,
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
    data.extend_from_slice(strtab.as_bytes());
    let shstrtab_off = EHDR_SIZE + data.len() as u64;
    data.extend_from_slice(shstr.as_bytes());
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
            (strtab_off, strtab.as_bytes().len() as u64),
            (shstrtab_off, shstr.as_bytes().len() as u64),
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
    Ok(out)
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
