//! A minimal ELF relocatable-object writer for our eBPF programs.
//!
//! Produces a standards-valid `ET_REL` / `EM_BPF` ELF64 (little-endian) the way
//! `clang -target bpf` does — written by hand, no LLVM — so a program can be
//! inspected and disassembled with the usual tools (`readelf`, `llvm-objdump`).
//! The object carries:
//!   * a program section named for the program type (`xdp` / `socket`, the
//!     libbpf `SEC(...)` conventions) holding the instruction bytes,
//!   * a `license` section (`"GPL"`),
//!   * a `.symtab`/`.strtab` with a section symbol plus a global `STT_FUNC`
//!     symbol for the program entry, and
//!   * a `.shstrtab`.
//!
//! Scope: the instruction stream is our eBPF encoded for the `rbpf` execution
//! ABI (the `data`/`data_end` metadata-buffer prologue, by-value map helpers),
//! so the object is structurally a real BPF ELF but not yet wired to the kernel
//! ctx/map ABI. Maps therefore need relocations against a `maps` section under
//! the kernel ABI, which is a follow-up; for now a program that declares maps is
//! rejected by the ELF writer. A `TargetAbi::Kernel` codegen flavor (correct ctx
//! offsets + map ABI) is the next increment and makes the object load under
//! libbpf.

use bpf_tcl_ir::ir::ProgType;

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
const SHF_ALLOC: u64 = 0x2;
const SHF_EXECINSTR: u64 = 0x4;

// Symbol binding / type.
const STB_GLOBAL: u8 = 1;
const STT_FUNC: u8 = 2;
const STT_SECTION: u8 = 3;

// Fixed sizes (ELF64).
const EHDR_SIZE: u64 = 64;
const SHDR_SIZE: usize = 64;
const SYM_SIZE: usize = 24;
/// Index of the first global symbol (NULL + the section symbol are local).
const FIRST_GLOBAL_SYM: u32 = 2;

// Section indices in the object we emit (NULL, prog, license, symtab, strtab,
// shstrtab).
const SEC_PROG: u16 = 1;
const SEC_STRTAB: u16 = 4;
const SEC_SHSTRTAB: u16 = 5;
const SEC_COUNT: u16 = 6;

/// An error from the ELF writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElfError {
    /// The program declares maps, which need the kernel map ABI + relocations
    /// (a follow-up increment); the ELF writer can't yet emit them.
    MapsUnsupported,
}

impl std::fmt::Display for ElfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ElfError::MapsUnsupported => write!(
                f,
                "ELF output does not yet support maps (needs the kernel map ABI)"
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

/// The computed offset/size of one laid-out section (file offset, byte size).
type SecLoc = (u64, u64);

/// Everything `section_headers` needs: the five section-name offsets plus each
/// data section's laid-out location.
struct Layout {
    names: [u32; 5],
    prog: SecLoc,
    license: SecLoc,
    symtab: SecLoc,
    strtab: SecLoc,
    shstrtab: SecLoc,
}

/// Write `obj` as an `EM_BPF` `ET_REL` ELF64 object, with the program entry
/// exported as `prog_name`.
///
/// # Errors
/// [`ElfError::MapsUnsupported`] if the program declares any map.
pub fn write_object(obj: &EbpfObject, prog_name: &str) -> Result<Vec<u8>, ElfError> {
    if !obj.maps.is_empty() {
        return Err(ElfError::MapsUnsupported);
    }

    // Section-header name table and the program-symbol name table.
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
    let symtab = build_symtab(strtab.add(prog_name), prog_size);

    // Lay out section data after the ELF header, tracking each section's offset.
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

    let secs = section_headers(&Layout {
        names,
        prog: (prog_off, prog_size),
        license: (license_off, 4),
        symtab: (symtab_off, symtab.len() as u64),
        strtab: (strtab_off, strtab.buf.len() as u64),
        shstrtab: (shstrtab_off, shstr.buf.len() as u64),
    });

    let cap = usize::try_from(shoff).unwrap_or(0) + usize::from(SEC_COUNT) * SHDR_SIZE;
    let mut out = Vec::with_capacity(cap);
    write_ehdr(&mut out, shoff);
    out.extend_from_slice(&data);
    for s in &secs {
        s.encode(&mut out);
    }
    Ok(out)
}

/// Build the symbol table bytes: NULL, the program section symbol (local), then
/// the global `STT_FUNC` for the entry.
fn build_symtab(prog_sym_name: u32, prog_size: u64) -> Vec<u8> {
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

/// Build the six section headers (NULL, prog, license, .symtab, .strtab,
/// .shstrtab) from the laid-out [`Layout`].
fn section_headers(l: &Layout) -> [Shdr; 6] {
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
            link: u32::from(SEC_STRTAB), // strings live in .strtab
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

/// Pad `v` with zero bytes up to a multiple of `to`.
fn align(v: &mut Vec<u8>, to: usize) {
    while !v.len().is_multiple_of(to) {
        v.push(0);
    }
}

/// Write the 64-byte ELF header.
fn write_ehdr(out: &mut Vec<u8>, shoff: u64) {
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
    out.extend_from_slice(&SEC_COUNT.to_le_bytes()); // e_shnum
    out.extend_from_slice(&SEC_SHSTRTAB.to_le_bytes()); // e_shstrndx
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
    use crate::ebpf::emit::emit_program;
    use bpf_tcl_ir::compile_module;

    fn obj_for(src: &str) -> EbpfObject {
        let m = compile_module(src).expect("compiles");
        emit_program(&m.programs[0].program).expect("emits")
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

        // Walk the section headers to find the symbol table.
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

        // Symbol #2 is the global FUNC for the program entry.
        let s2 = off + 2 * SYM_SIZE;
        assert_eq!(o[s2 + 4], (STB_GLOBAL << 4) | STT_FUNC); // st_info
        assert_eq!(le16(&o, s2 + 6), SEC_PROG); // st_shndx
        assert_eq!(le64(&o, s2 + 16), raw_len); // st_size == program bytes
    }

    #[test]
    fn maps_are_rejected() {
        let src = "when SOCKET_FILTER { map m hash 8 8 8\n map_set m {0} {1}\n accept }\n";
        let err = write_object(&obj_for(src), "socket_filter").unwrap_err();
        assert_eq!(err, ElfError::MapsUnsupported);
    }

    #[test]
    fn section_count_and_shstrndx_consistent() {
        let o = write_object(&obj_for("when XDP { drop }\n"), "xdp").unwrap();
        // e_shoff points within the file, and there are exactly SEC_COUNT headers.
        let shoff = le64(&o, 40);
        assert_eq!(o.len(), shoff + usize::from(SEC_COUNT) * 64);
    }
}
