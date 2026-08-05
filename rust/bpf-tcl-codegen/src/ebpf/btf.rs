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

//! A minimal BTF (BPF Type Format) writer for libbpf-compatible BTF-defined
//! maps.
//!
//! libbpf reads a map's kind, key/value sizes, capacity, and flags from a BTF
//! description of a struct variable in the `.maps` section. Each map is a
//! variable of an anonymous struct whose members are pointers to arrays whose
//! element counts encode the numeric properties:
//!
//! ```c
//! struct {
//!     int (*type)[BPF_MAP_TYPE_HASH];   // map type number
//!     int (*key_size)[K];               // key size in bytes
//!     int (*value_size)[V];             // value size in bytes
//!     int (*max_entries)[M];            // capacity
//! } name SEC(".maps");
//! ```
//!
//! This is the array-encoded (`key_size` / `value_size`) form of libbpf's
//! BTF-defined map definition, chosen so we do not need to synthesise precise
//! key/value BTF types — the sizes are all libbpf needs for integer maps.
//!
//! The `.maps` section itself is zero-filled; libbpf takes the definition
//! entirely from this BTF, driven by a global object symbol per map (see the
//! ELF writer).

use bpf_tcl_ir::MapConcurrency;
use bpf_tcl_ir::ir::{MapDef, MapKind};

/// Bytes occupied by one map's struct variable in the `.maps` section (four
/// 8-byte pointer members).
pub const MAP_STRUCT_SIZE: u32 = 32;

// BTF header.
const BTF_MAGIC: u16 = 0xeb9f;
const BTF_VERSION: u8 = 1;
const BTF_HDR_LEN: u32 = 24;

// BTF type kinds (`info` bits 24..=28).
const KIND_INT: u32 = 1;
const KIND_PTR: u32 = 2;
const KIND_ARRAY: u32 = 3;
const KIND_STRUCT: u32 = 4;
const KIND_VAR: u32 = 14;
const KIND_DATASEC: u32 = 15;

/// `BTF_INT_SIGNED` in the top byte of the int encoding word.
const INT_SIGNED: u32 = 1 << 24;

/// `BTF_VAR_GLOBAL_ALLOCATED` linkage.
const VAR_GLOBAL: u32 = 1;

/// `BPF_MAP_TYPE_*` numbers used by BTF-defined maps.
const MAP_TYPE_HASH: u32 = 1;
const MAP_TYPE_ARRAY: u32 = 2;
const MAP_TYPE_PERCPU_HASH: u32 = 5;
const MAP_TYPE_PERCPU_ARRAY: u32 = 6;

/// The `BPF_MAP_TYPE_*` number for a declared map.
fn map_type_number(def: &MapDef) -> u32 {
    match (def.kind, def.concurrency) {
        (MapKind::Hash, MapConcurrency::Shared) => MAP_TYPE_HASH,
        (MapKind::Array, MapConcurrency::Shared) => MAP_TYPE_ARRAY,
        (MapKind::Hash, MapConcurrency::PerCpu) => MAP_TYPE_PERCPU_HASH,
        (MapKind::Array, MapConcurrency::PerCpu) => MAP_TYPE_PERCPU_ARRAY,
    }
}

/// A growable BTF string table (NUL-led, NUL-terminated entries).
struct BtfStrings {
    buf: Vec<u8>,
}

impl BtfStrings {
    fn new() -> Self {
        Self { buf: vec![0] }
    }

    fn add(&mut self, s: &str) -> u32 {
        let off = u32::try_from(self.buf.len()).unwrap_or(0);
        self.buf.extend_from_slice(s.as_bytes());
        self.buf.push(0);
        off
    }
}

/// A BTF type-section accumulator that tracks the next type id.
struct BtfTypes {
    buf: Vec<u8>,
    next_id: u32,
}

impl BtfTypes {
    fn new() -> Self {
        // Type id 0 is the implicit `void`; real ids start at 1.
        Self {
            buf: Vec::new(),
            next_id: 1,
        }
    }

    fn info(kind: u32, vlen: u32) -> u32 {
        (kind << 24) | (vlen & 0xffff)
    }

    fn push_word(&mut self, w: u32) {
        self.buf.extend_from_slice(&w.to_le_bytes());
    }

    fn header(&mut self, name_off: u32, info: u32, size_or_type: u32) {
        self.push_word(name_off);
        self.push_word(info);
        self.push_word(size_or_type);
    }

    /// A signed 32-bit `int`.
    fn int(&mut self, name_off: u32) -> u32 {
        let id = self.next_id;
        self.header(name_off, Self::info(KIND_INT, 0), 4);
        self.push_word(INT_SIGNED | 32);
        self.next_id += 1;
        id
    }

    /// `int[nelems]` (element and index type both `int_id`).
    fn array(&mut self, int_id: u32, nelems: u32) -> u32 {
        let id = self.next_id;
        self.header(0, Self::info(KIND_ARRAY, 0), 0);
        self.push_word(int_id); // element type
        self.push_word(int_id); // index type
        self.push_word(nelems);
        self.next_id += 1;
        id
    }

    /// `type *` (pointer to type id `to`).
    fn ptr(&mut self, to: u32) -> u32 {
        let id = self.next_id;
        self.header(0, Self::info(KIND_PTR, 0), to);
        self.next_id += 1;
        id
    }

    /// An anonymous struct with the given `(name_off, member_type)` members.
    fn struct_type(&mut self, members: &[(u32, u32)]) -> u32 {
        let id = self.next_id;
        let vlen = u32::try_from(members.len()).unwrap_or(0);
        self.header(0, Self::info(KIND_STRUCT, vlen), MAP_STRUCT_SIZE);
        let mut bit_off = 0u32;
        for (name_off, ty) in members {
            self.push_word(*name_off);
            self.push_word(*ty);
            self.push_word(bit_off);
            bit_off += 64; // one 8-byte pointer per member
        }
        self.next_id += 1;
        id
    }

    /// A global `VAR` of type id `ty`.
    fn var(&mut self, name_off: u32, ty: u32) -> u32 {
        let id = self.next_id;
        self.header(name_off, Self::info(KIND_VAR, 0), ty);
        self.push_word(VAR_GLOBAL);
        self.next_id += 1;
        id
    }

    /// A `DATASEC` listing `(var_id, offset, size)` entries.
    fn datasec(&mut self, name_off: u32, total_size: u32, vars: &[(u32, u32, u32)]) -> u32 {
        let id = self.next_id;
        let vlen = u32::try_from(vars.len()).unwrap_or(0);
        self.header(name_off, Self::info(KIND_DATASEC, vlen), total_size);
        for (var_id, offset, size) in vars {
            self.push_word(*var_id);
            self.push_word(*offset);
            self.push_word(*size);
        }
        self.next_id += 1;
        id
    }
}

/// Build the `.BTF` section bytes describing every map in `maps`.
///
/// Returns an empty vector when there are no maps (no BTF section is emitted in
/// that case).
#[must_use]
pub fn build_btf(maps: &[MapDef]) -> Vec<u8> {
    if maps.is_empty() {
        return Vec::new();
    }

    let mut strings = BtfStrings::new();
    let mut types = BtfTypes::new();

    let int_name = strings.add("int");
    let type_name = strings.add("type");
    let key_size_name = strings.add("key_size");
    let value_size_name = strings.add("value_size");
    let max_entries_name = strings.add("max_entries");
    let maps_sec_name = strings.add(".maps");

    let int_id = types.int(int_name);

    // A distinct `int (*)[N]` pointer per array length used by any member.
    let mut ptr_for_len: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    let mut ptr_to_array = |types: &mut BtfTypes, nelems: u32| -> u32 {
        *ptr_for_len.entry(nelems).or_insert_with(|| {
            let arr = types.array(int_id, nelems.max(1));
            types.ptr(arr)
        })
    };

    // A struct + var per map, gathering the section var-info as we go.
    let mut var_infos: Vec<(u32, u32, u32)> = Vec::with_capacity(maps.len());
    for (i, def) in maps.iter().enumerate() {
        let type_ptr = ptr_to_array(&mut types, map_type_number(def));
        let key_ptr = ptr_to_array(&mut types, def.key_size);
        let value_ptr = ptr_to_array(&mut types, def.value_size);
        let max_ptr = ptr_to_array(&mut types, def.max_entries);
        let struct_id = types.struct_type(&[
            (type_name, type_ptr),
            (key_size_name, key_ptr),
            (value_size_name, value_ptr),
            (max_entries_name, max_ptr),
        ]);
        let var_name = strings.add(&def.name);
        let var_id = types.var(var_name, struct_id);
        let offset = u32::try_from(i).unwrap_or(0) * MAP_STRUCT_SIZE;
        var_infos.push((var_id, offset, MAP_STRUCT_SIZE));
    }

    let total = u32::try_from(maps.len()).unwrap_or(0) * MAP_STRUCT_SIZE;
    types.datasec(maps_sec_name, total, &var_infos);

    encode_btf(&types.buf, &strings.buf)
}

/// Prepend the BTF header to the type and string sections.
fn encode_btf(type_bytes: &[u8], str_bytes: &[u8]) -> Vec<u8> {
    let type_len = u32::try_from(type_bytes.len()).unwrap_or(0);
    let str_len = u32::try_from(str_bytes.len()).unwrap_or(0);
    let mut out = Vec::with_capacity(BTF_HDR_LEN as usize + type_bytes.len() + str_bytes.len());
    out.extend_from_slice(&BTF_MAGIC.to_le_bytes());
    out.push(BTF_VERSION);
    out.push(0); // flags
    out.extend_from_slice(&BTF_HDR_LEN.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // type_off
    out.extend_from_slice(&type_len.to_le_bytes());
    out.extend_from_slice(&type_len.to_le_bytes()); // str_off (immediately after types)
    out.extend_from_slice(&str_len.to_le_bytes());
    out.extend_from_slice(type_bytes);
    out.extend_from_slice(str_bytes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpf_tcl_ir::compile_module;

    fn maps_of(src: &str) -> Vec<MapDef> {
        compile_module(src).expect("compiles").programs[0]
            .program
            .maps
            .clone()
    }

    #[test]
    fn empty_maps_produce_no_btf() {
        assert!(build_btf(&[]).is_empty());
    }

    #[test]
    fn btf_header_is_well_formed() {
        let maps = maps_of("when XDP { map m hash 8 8 16\n pass }\n");
        let btf = build_btf(&maps);
        assert!(btf.len() > BTF_HDR_LEN as usize);
        assert_eq!(u16::from_le_bytes([btf[0], btf[1]]), BTF_MAGIC);
        assert_eq!(btf[2], BTF_VERSION);
        let hdr_len = u32::from_le_bytes(btf[4..8].try_into().unwrap());
        assert_eq!(hdr_len, BTF_HDR_LEN);
        // The map name appears in the string section.
        assert!(btf.windows(2).any(|w| w == b"m\0"));
        assert!(
            btf.windows(6).any(|w| w == b".maps\0"),
            "the datasec name is present"
        );
    }

    #[test]
    fn btf_encodes_type_and_str_lengths_consistently() {
        let maps = maps_of("when XDP { map a array 4 8 4\n map b hash 2 4 8\n pass }\n");
        let btf = build_btf(&maps);
        let type_len = u32::from_le_bytes(btf[12..16].try_into().unwrap()) as usize;
        let str_off = u32::from_le_bytes(btf[16..20].try_into().unwrap()) as usize;
        let str_len = u32::from_le_bytes(btf[20..24].try_into().unwrap()) as usize;
        // Header + types + strings accounts for the whole buffer.
        assert_eq!(str_off, type_len);
        assert_eq!(btf.len(), BTF_HDR_LEN as usize + type_len + str_len);
    }
}
