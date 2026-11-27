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

//! Privileged kernel-verifier acceptance tests. **These do not run by default.**
//!
//! Loading a real BPF program calls the `bpf()` syscall, which needs
//! `CAP_BPF`/root and a live Linux kernel — neither is available in the sandbox
//! the rest of the suite runs in, so these are marked `#[ignore]`. Run them by
//! hand on a Linux host with `bpftool` installed:
//!
//! ```text
//! sudo -E cargo test -p bpf-tcl --test kernel_load -- --ignored --nocapture
//! ```
//!
//! Each test:
//!   * emits a kernel object with the real ABI (`--target kernel-xdp` /
//!     `kernel-socket`), writes it under `/tmp`;
//!   * asks `bpftool prog loadall` to submit it to the verifier, capturing the
//!     full verifier log on failure; and
//!   * unconditionally removes every pin and temporary file it created — it
//!     never attaches to a live interface.
//!
//! When `bpftool` is missing or the caller is unprivileged, the test **skips**
//! with a printed reason rather than failing, so `--ignored` on an unsuitable
//! host is harmless. A verifier *rejection* is a real failure.

use std::path::{Path, PathBuf};
use std::process::Command;

use bpf_tcl_codegen::ebpf::{TargetAbi, emit_program_for_target, write_object};
use bpf_tcl_ir::compile_module;

fn tool_present(name: &str) -> bool {
    Command::new(name)
        .arg("version")
        .output()
        .is_ok_and(|o| o.status.success())
}

fn is_root() -> bool {
    // Rootless is the common case in CI; a non-privileged loadall fails cleanly.
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .is_some_and(|s| s.trim() == "0")
}

fn write_object_file(src: &str, target: TargetAbi, sym: &str, tag: &str) -> PathBuf {
    let module = compile_module(src).expect("source compiles");
    let obj = emit_program_for_target(&module.programs[0].program, target).expect("object emits");
    let bytes = write_object(&obj, sym).expect("elf writes");
    let mut p = std::env::temp_dir();
    p.push(format!("bpftcl-load-{tag}-{}.o", std::process::id()));
    std::fs::write(&p, bytes).expect("write object");
    p
}

/// Ask the verifier to accept `obj_path`, pinning under `pin_dir`; returns the
/// verifier log. Cleans up the pin directory and object afterwards.
fn verifier_accepts(obj_path: &Path, pin_dir: &Path) -> Result<String, String> {
    let out = Command::new("bpftool")
        .args(["-d", "prog", "loadall"])
        .arg(obj_path)
        .arg(pin_dir)
        .output()
        .map_err(|e| format!("bpftool failed to spawn: {e}"))?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Best-effort cleanup: remove every pin and the object.
    let _ = std::fs::remove_dir_all(pin_dir);
    let _ = std::fs::remove_file(obj_path);
    if out.status.success() {
        Ok(log)
    } else {
        Err(log)
    }
}

fn run_load_case(src: &str, target: TargetAbi, sym: &str, tag: &str) {
    if !tool_present("bpftool") {
        eprintln!("skip: bpftool unavailable (install linux-tools to run this test)");
        return;
    }
    if !is_root() {
        eprintln!("skip: kernel load needs root/CAP_BPF");
        return;
    }
    let obj = write_object_file(src, target, sym, tag);
    let mut pin = std::env::temp_dir();
    pin.push(format!("bpftcl-pin-{tag}-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&pin);
    match verifier_accepts(&obj, &pin) {
        Ok(log) => eprintln!("verifier accepted {tag}:\n{log}"),
        Err(log) => panic!("verifier rejected {tag}:\n{log}"),
    }
}

#[test]
#[ignore = "needs root + bpftool + a live kernel; run with --ignored on a Linux host"]
fn kernel_xdp_pass_loads() {
    run_load_case(
        "when XDP { pass }\n",
        TargetAbi::KernelXdp,
        "xdp",
        "xdp-pass",
    );
}

#[test]
#[ignore = "needs root + bpftool + a live kernel; run with --ignored on a Linux host"]
fn kernel_xdp_bounded_packet_read_loads() {
    run_load_case(
        "when XDP {\n setbuf p ctx\n load16 dport p 36 be\n\
         if {$dport == 22} { drop }\n pass }\n",
        TargetAbi::KernelXdp,
        "xdp",
        "xdp-portfilter",
    );
}

#[test]
#[ignore = "needs root + bpftool + a live kernel; run with --ignored on a Linux host"]
fn kernel_xdp_map_counter_loads() {
    run_load_case(
        "when XDP {\n map hits array 4 8 4\n setint k 0\n\
         map_get n hits {$k}\n setint n1 {$n + 1}\n map_set hits {$k} {$n1}\n pass }\n",
        TargetAbi::KernelXdp,
        "xdp",
        "xdp-mapcounter",
    );
}

#[test]
#[ignore = "needs root + bpftool + a live kernel; run with --ignored on a Linux host"]
fn kernel_socket_filter_length_loads() {
    run_load_case(
        "when SOCKET_FILTER {\n setbuf p ctx\n pktlen len p\n\
         if {$len < 20} { drop }\n accept {$len} }\n",
        TargetAbi::KernelSocketFilter,
        "flt",
        "sock-len",
    );
}
