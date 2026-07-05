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

//! Golden cross-check of the ELF object writer against the system toolchain:
//! `readelf` must see a `EM_BPF` relocatable with our sections, and
//! `llvm-objdump` must disassemble exactly the instructions we emitted. These
//! tests skip gracefully when a tool is unavailable, so they never fail a CI
//! host that lacks binutils/LLVM.

use std::path::PathBuf;
use std::process::Command;

use bpf_tcl_codegen::ebpf::{emit_program, write_object};
use bpf_tcl_ir::compile_module;

fn has_tool(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn elf_for(src: &str, sym: &str) -> (Vec<u8>, usize) {
    let m = compile_module(src).expect("compiles");
    let obj = emit_program(&m.programs[0].program).expect("emits");
    let n = obj.insns.len();
    (write_object(&obj, sym).expect("elf"), n)
}

fn write_tmp(tag: &str, bytes: &[u8]) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("bpftcl-{tag}-{}.o", std::process::id()));
    std::fs::write(&p, bytes).expect("write temp object");
    p
}

const PORT_FILTER: &str = "when XDP {\n\
    setbuf p ctx\n\
    pktlen n p\n\
    if {$n < 38} { pass }\n\
    load16 dport p 36\n\
    if {$dport == 22} { drop }\n\
    pass\n}\n";

#[test]
fn readelf_sees_em_bpf_relocatable() {
    if !has_tool("readelf") {
        eprintln!("skip: readelf unavailable");
        return;
    }
    let (bytes, _) = elf_for("when XDP { drop }\n", "xdp");
    let p = write_tmp("readelf-h", &bytes);
    let out = Command::new("readelf").arg("-h").arg(&p).output().unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("BPF"), "machine should be BPF:\n{s}");
    assert!(s.contains("REL"), "type should be REL:\n{s}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn readelf_lists_our_sections_and_symbol() {
    if !has_tool("readelf") {
        eprintln!("skip: readelf unavailable");
        return;
    }
    let (bytes, _) = elf_for("when XDP { drop }\n", "xdp");
    let p = write_tmp("readelf-S", &bytes);
    let secs = Command::new("readelf").arg("-S").arg(&p).output().unwrap();
    let s = String::from_utf8_lossy(&secs.stdout);
    assert!(s.contains("xdp"), "program section present:\n{s}");
    assert!(s.contains("license"), "license section present:\n{s}");

    let syms = Command::new("readelf").arg("-s").arg(&p).output().unwrap();
    let s = String::from_utf8_lossy(&syms.stdout);
    assert!(
        s.contains("FUNC") && s.contains("GLOBAL") && s.contains("xdp"),
        "exported FUNC symbol present:\n{s}"
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn llvm_objdump_disassembles_every_instruction() {
    if !has_tool("llvm-objdump") {
        eprintln!("skip: llvm-objdump unavailable");
        return;
    }
    let (bytes, n) = elf_for(PORT_FILTER, "xdp");
    let p = write_tmp("objdump", &bytes);
    let out = Command::new("llvm-objdump")
        .arg("-d")
        .arg(&p)
        .output()
        .unwrap();
    assert!(out.status.success(), "llvm-objdump should succeed");
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("elf64-bpf"), "recognised as elf64-bpf:\n{s}");
    // Count disassembled instruction lines ("  <hexidx>:\t..").
    let count = s
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.split_once(':').is_some_and(|(idx, _)| {
                !idx.is_empty() && idx.bytes().all(|b| b.is_ascii_hexdigit())
            })
        })
        .count();
    assert_eq!(count, n, "disassembled {count} insns, emitted {n}:\n{s}");
    assert!(s.contains("exit"), "program ends in exit:\n{s}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn matches_clang_reference_machine_and_type() {
    if !has_tool("readelf") || !has_tool("clang") {
        eprintln!("skip: readelf/clang unavailable");
        return;
    }
    // A reference BPF object from the real toolchain.
    let mut cpath = std::env::temp_dir();
    cpath.push(format!("bpftcl-ref-{}.c", std::process::id()));
    let mut opath = std::env::temp_dir();
    opath.push(format!("bpftcl-ref-{}.o", std::process::id()));
    std::fs::write(
        &cpath,
        b"__attribute__((section(\"xdp\"),used)) int p(void*c){return 2;}\n",
    )
    .unwrap();
    let clang = Command::new("clang")
        .args(["-target", "bpf", "-O2", "-c"])
        .arg(&cpath)
        .arg("-o")
        .arg(&opath)
        .output()
        .unwrap();
    if !clang.status.success() {
        eprintln!("skip: clang could not build a bpf object");
        return;
    }
    let ref_hdr = Command::new("readelf")
        .arg("-h")
        .arg(&opath)
        .output()
        .unwrap();
    let ref_s = String::from_utf8_lossy(&ref_hdr.stdout);
    let ref_machine = ref_s.lines().find(|l| l.contains("Machine:")).unwrap_or("");

    let (bytes, _) = elf_for("when XDP { drop }\n", "xdp");
    let p = write_tmp("vs-clang", &bytes);
    let our_hdr = Command::new("readelf").arg("-h").arg(&p).output().unwrap();
    let our_s = String::from_utf8_lossy(&our_hdr.stdout);
    let our_machine = our_s.lines().find(|l| l.contains("Machine:")).unwrap_or("");

    assert_eq!(
        our_machine.trim(),
        ref_machine.trim(),
        "our machine line should match clang's"
    );
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&opath);
}
