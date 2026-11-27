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

//! Kernel-target codegen: the verifier *model* over emitted XDP / socket-filter
//! objects, structural cross-checks of the map/BTF/relocation ELF against the
//! system toolchain (`readelf`, `llvm-objdump`), and network-byte-order packet
//! fixtures executed under the userspace simulator.
//!
//! These tests are rootless: the structural ones skip gracefully when a tool is
//! unavailable, and the verifier-model / byte-order ones need no kernel at all.
//! Genuine kernel `bpf()` load and `BPF_PROG_TEST_RUN` acceptance lives in
//! `tests/kernel_load.rs`, gated behind `#[ignore]`.

use std::path::PathBuf;
use std::process::Command;

use bpf_tcl::run::run_socket_filter;
use bpf_tcl_codegen::ebpf::{
    EbpfObject, TargetAbi, emit_program, emit_program_for_target, verify, write_object,
};
use bpf_tcl_ir::compile_module;

fn kernel_obj(src: &str, target: TargetAbi) -> EbpfObject {
    let m = compile_module(src).expect("compiles");
    emit_program_for_target(&m.programs[0].program, target).expect("emits for the kernel")
}

fn rbpf_obj(src: &str) -> EbpfObject {
    let m = compile_module(src).expect("compiles");
    emit_program(&m.programs[0].program).expect("emits")
}

fn has_tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn write_tmp(tag: &str, bytes: &[u8]) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("bpftcl-k-{tag}-{}.o", std::process::id()));
    std::fs::write(&p, bytes).expect("write temp object");
    p
}

// The map-counting XDP source used across several structural tests: a per-CPU
// packet counter plus a bounds-checked TCP destination-port drop.
const XDP_MAP_FILTER: &str = "when XDP {\n\
    map hits array 4 8 4\n\
    setbuf p ctx\n\
    setint k 0\n\
    map_get n hits {$k}\n\
    setint n1 {$n + 1}\n\
    map_set hits {$k} {$n1}\n\
    load16 dport p 36 be\n\
    if {$dport == 22} { drop }\n\
    pass\n}\n";

#[test]
fn verifier_model_accepts_xdp_map_filter() {
    let obj = kernel_obj(XDP_MAP_FILTER, TargetAbi::KernelXdp);
    assert_eq!(verify(&obj), Vec::new(), "the model must accept the object");
    // Two map-fd loads (one for the get, one for the set), each relocated.
    assert_eq!(obj.relocations.len(), 2);
}

#[test]
fn verifier_model_accepts_socket_filter_length_verdict() {
    // A socket filter that accepts a byte count read from the packet.
    let obj = kernel_obj(
        "when SOCKET_FILTER {\n\
         setbuf p ctx\n\
         pktlen len p\n\
         if {$len < 20} { drop }\n\
         accept {$len}\n}\n",
        TargetAbi::KernelSocketFilter,
    );
    assert_eq!(verify(&obj), Vec::new());
}

#[test]
fn readelf_accepts_kernel_map_object() {
    if !has_tool("readelf") {
        eprintln!("skip: readelf unavailable");
        return;
    }
    let obj = kernel_obj(XDP_MAP_FILTER, TargetAbi::KernelXdp);
    let elf = write_object(&obj, "xdp").expect("elf");
    let path = write_tmp("readelf-maps", &elf);

    let hdr = Command::new("readelf")
        .arg("-h")
        .arg(&path)
        .output()
        .unwrap();
    let header = String::from_utf8_lossy(&hdr.stdout);
    assert!(
        header.contains("BPF") && header.contains("REL"),
        "EM_BPF REL:\n{header}"
    );

    let secs = Command::new("readelf")
        .arg("-S")
        .arg(&path)
        .output()
        .unwrap();
    let sections = String::from_utf8_lossy(&secs.stdout);
    for needle in [".maps", ".BTF", ".relxdp", "xdp", ".symtab"] {
        assert!(
            sections.contains(needle),
            "section {needle} present:\n{sections}"
        );
    }

    let rel = Command::new("readelf")
        .arg("-r")
        .arg(&path)
        .output()
        .unwrap();
    let relocs = String::from_utf8_lossy(&rel.stdout);
    assert!(
        relocs.contains("R_BPF_64_64"),
        "map relocations present:\n{relocs}"
    );
    assert!(
        relocs.contains("hits"),
        "relocation names the map:\n{relocs}"
    );

    let syms = Command::new("readelf")
        .arg("-s")
        .arg(&path)
        .output()
        .unwrap();
    let symbols = String::from_utf8_lossy(&syms.stdout);
    assert!(
        symbols.contains("OBJECT") && symbols.contains("hits"),
        "map symbol:\n{symbols}"
    );
    assert!(
        symbols.contains("FUNC") && symbols.contains("xdp"),
        "program symbol:\n{symbols}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn llvm_objdump_disassembles_kernel_map_object() {
    if !has_tool("llvm-objdump") {
        eprintln!("skip: llvm-objdump unavailable");
        return;
    }
    let obj = kernel_obj(XDP_MAP_FILTER, TargetAbi::KernelXdp);
    let insn_count = obj.insns.len();
    let elf = write_object(&obj, "xdp").expect("elf");
    let path = write_tmp("objdump-maps", &elf);

    let out = Command::new("llvm-objdump")
        .arg("-dr")
        .arg(&path)
        .output()
        .unwrap();
    assert!(out.status.success(), "llvm-objdump should succeed");
    let dump = String::from_utf8_lossy(&out.stdout);
    assert!(
        dump.contains("elf64-bpf"),
        "recognised as elf64-bpf:\n{dump}"
    );
    // Every emitted instruction disassembles (a wide load counts once but spans
    // two 8-byte slots, so count listed instruction indices).
    let count = dump
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.split_once(':').is_some_and(|(idx, _)| {
                !idx.is_empty() && idx.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        })
        .count();
    assert_eq!(
        count, insn_count,
        "disassembled {count} of {insn_count} instructions"
    );
    assert!(dump.contains("call"), "a helper call is present:\n{dump}");
    assert!(
        dump.contains("hits"),
        "reloc against the map is shown:\n{dump}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn socket_filter_kernel_object_names_socket_section() {
    if !has_tool("readelf") {
        eprintln!("skip: readelf unavailable");
        return;
    }
    let obj = kernel_obj(
        "when SOCKET_FILTER {\n map seen hash 4 4 64\n setint k 0\n\
         map_set seen {$k} {1}\n accept }\n",
        TargetAbi::KernelSocketFilter,
    );
    let elf = write_object(&obj, "flt").expect("elf");
    let path = write_tmp("sock-sec", &elf);
    let secs = Command::new("readelf")
        .arg("-S")
        .arg(&path)
        .output()
        .unwrap();
    let sections = String::from_utf8_lossy(&secs.stdout);
    assert!(
        sections.contains("socket"),
        "socket program section:\n{sections}"
    );
    assert!(
        sections.contains(".relsocket"),
        "socket relocation section:\n{sections}"
    );
    let _ = std::fs::remove_file(&path);
}

// Network-byte-order fixtures. The emitted kernel object cannot run in this
// container, but the shared BPF-IR byte-order lowering (BPF_END) is identical on
// the rbpf and kernel paths, so exercising it under the simulator proves the
// `be` / `le` field interpretation the kernel object inherits.

/// A realistic Ethernet + IPv4 + TCP frame with a chosen TCP destination port.
fn eth_ip_tcp_frame(dst_port: u16) -> Vec<u8> {
    let mut pkt = vec![0u8; 54];
    // Ethernet: EtherType 0x0800 (IPv4) in network order at offset 12.
    pkt[12] = 0x08;
    pkt[13] = 0x00;
    // IPv4 total length (network order) at offset 16: 40 bytes.
    pkt[16] = 0x00;
    pkt[17] = 0x28;
    // TCP destination port (network order) at offset 36.
    pkt[34 + 2] = (dst_port >> 8) as u8;
    pkt[34 + 3] = (dst_port & 0xff) as u8;
    pkt
}

#[test]
fn network_order_16bit_ethertype_matches_fixture() {
    // `load16 … be` reads the big-endian EtherType and yields host-order 0x0800.
    let obj = rbpf_obj(
        "when SOCKET_FILTER {\n\
         setbuf p ctx\n\
         load16 et p 12 be\n\
         if {$et == 2048} { accept 1 }\n\
         drop\n}\n",
    );
    let mut frame = eth_ip_tcp_frame(80);
    assert_eq!(
        run_socket_filter(&obj, &mut frame).unwrap(),
        1,
        "0x0800 == 2048"
    );
}

#[test]
fn network_order_16bit_tcp_port_drop() {
    let obj = rbpf_obj(
        "when SOCKET_FILTER {\n\
         setbuf p ctx\n\
         load16 dport p 36 be\n\
         if {$dport == 22} { drop }\n\
         accept 1\n}\n",
    );
    let mut ssh = eth_ip_tcp_frame(22);
    assert_eq!(
        run_socket_filter(&obj, &mut ssh).unwrap(),
        0,
        "port 22 drops"
    );
    let mut http = eth_ip_tcp_frame(80);
    assert_eq!(
        run_socket_filter(&obj, &mut http).unwrap(),
        1,
        "port 80 accepts"
    );
}

#[test]
fn network_order_32bit_field_roundtrips() {
    // Store a 32-bit network-order value and read it back big-endian.
    let obj = rbpf_obj(
        "when SOCKET_FILTER {\n\
         setbuf p ctx\n\
         load32 word p 12 be\n\
         accept {$word}\n}\n",
    );
    let mut pkt = vec![0u8; 20];
    // Big-endian 0x0800_0000 at offset 12 → host value 0x08000000 = 134217728.
    pkt[12] = 0x08;
    pkt[13] = 0x00;
    pkt[14] = 0x00;
    pkt[15] = 0x00;
    let verdict = run_socket_filter(&obj, &mut pkt).unwrap();
    assert_eq!(verdict & 0xffff_ffff, 0x0800_0000, "big-endian 32-bit read");
}

#[test]
fn little_endian_16bit_differs_from_big_endian() {
    // The same two bytes read `le` vs `be` give byte-swapped host values.
    let be = rbpf_obj("when SOCKET_FILTER { setbuf p ctx\n load16 v p 0 be\n accept {$v} }\n");
    let le = rbpf_obj("when SOCKET_FILTER { setbuf p ctx\n load16 v p 0 le\n accept {$v} }\n");
    let mut pkt = vec![0x12u8, 0x34, 0, 0];
    assert_eq!(run_socket_filter(&be, &mut pkt).unwrap() & 0xffff, 0x1234);
    let mut pkt2 = vec![0x12u8, 0x34, 0, 0];
    assert_eq!(run_socket_filter(&le, &mut pkt2).unwrap() & 0xffff, 0x3412);
}

#[test]
fn rbpf_map_object_still_runs() {
    // The userspace simulator target is untouched by the kernel work.
    let obj = rbpf_obj(
        "when SOCKET_FILTER {\n map m hash 8 8 8\n map_set m {7} {99}\n\
         map_get v m {7}\n accept {$v} }\n",
    );
    let mut pkt = vec![0u8; 8];
    assert_eq!(run_socket_filter(&obj, &mut pkt).unwrap(), 99);
}
