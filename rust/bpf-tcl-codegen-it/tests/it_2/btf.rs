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

use bpf_tcl_codegen::ebpf::btf::*;
use bpf_tcl_ir::ir::MapDef;
use tcl_core_types::ArtifactStringError;

// The BTF header's fields (linux/btf.h).
const BTF_MAGIC: u16 = 0xeb9f;
const BTF_VERSION: u8 = 1;
const BTF_HDR_LEN: u32 = 24;
use bpf_tcl_ir::compile_module;

fn maps_of(src: &str) -> Vec<MapDef> {
    compile_module(src).expect("compiles").programs[0]
        .program
        .maps
        .clone()
}

#[test]
fn empty_maps_produce_no_btf() {
    crate::install_surfaces();
    assert_eq!(build_btf(&[]).unwrap(), [] as [u8; 0]);
}

#[test]
fn direct_map_ingress_refuses_nul_without_collapsing_to_the_valid_prefix() {
    crate::install_surfaces();
    let mut maps = maps_of("when XDP { map m hash 8 8 16\n pass }\n");
    assert!(build_btf(&maps).is_ok());
    maps[0].name = "m\0suffix".into();
    assert_eq!(
        build_btf(&maps),
        Err(ArtifactStringError::InteriorNul { position: 1 })
    );
}

#[test]
fn btf_header_is_well_formed() {
    crate::install_surfaces();
    let maps = maps_of("when XDP { map m hash 8 8 16\n pass }\n");
    let btf = build_btf(&maps).unwrap();
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
    crate::install_surfaces();
    let maps = maps_of("when XDP { map a array 4 8 4\n map b hash 2 4 8\n pass }\n");
    let btf = build_btf(&maps).unwrap();
    let type_len = u32::from_le_bytes(btf[12..16].try_into().unwrap()) as usize;
    let str_off = u32::from_le_bytes(btf[16..20].try_into().unwrap()) as usize;
    let str_len = u32::from_le_bytes(btf[20..24].try_into().unwrap()) as usize;
    // Header + types + strings accounts for the whole buffer.
    assert_eq!(str_off, type_len);
    assert_eq!(btf.len(), BTF_HDR_LEN as usize + type_len + str_len);
}
