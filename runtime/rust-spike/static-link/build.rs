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

// ============================================================================
// SPIKE -- throwaway proof-of-concept. NOT the final design. Do not derive the
// production build wiring from this file. See runtime/rust-spike/README.md.
// ============================================================================
//
//! Compile the UNMODIFIED C Tcl extension (`ext/pkga.c`, vendored byte-identical
//! from Tcl 9.0.3 `unix/dltest/`) to a WebAssembly object with clang, against
//! our own `include/tcl.h`, then hand the object to rustc's linker so it links
//! together with the Rust runtime into one `wasm32-wasip1` module.
//!
//! This is the toolchain half of the spike: it proves a stock C extension
//! compiles to WASM against an authored `tcl.h` with no per-extension shimming,
//! and that a clang-produced wasm object links against Rust-produced wasm
//! objects.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    // include/ and ext/ are shared by both spikes, one level up.
    let src = manifest.join("../ext/pkga.c");
    let inc = manifest.join("../include");
    let obj = out.join("pkga.o");

    println!("cargo:rerun-if-changed={}", src.display());
    println!("cargo:rerun-if-changed={}/tcl.h", inc.display());
    println!("cargo:rerun-if-env-changed=SPIKE_CC");

    // Compile with `zig cc` (override via SPIKE_CC="prog arg..."): it bundles
    // wasi-libc, so our tcl.h can #include <stdio.h>/<string.h> like the real
    // header does. This also demonstrates the Rust-runtime + zig-cc hybrid.
    // pkga.c calls no libc function, so the object stays free of wasi-libc
    // references and links cleanly into the Rust wasm32-wasip1 binary.
    let cc = env::var("SPIKE_CC").unwrap_or_else(|_| "zig cc".into());
    let mut parts = cc.split_whitespace();
    let prog = parts.next().expect("empty SPIKE_CC");
    let status = Command::new(prog)
        .args(parts)
        .args(["--target=wasm32-wasi", "-O2", "-c"])
        .arg("-I")
        .arg(&inc)
        .arg(&src)
        .arg("-o")
        .arg(&obj)
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke {cc}: {e}"));
    assert!(status.success(), "C compiler failed to compile ext/pkga.c");

    // Pass the freshly-built extension object to the final wasm link step.
    println!("cargo:rustc-link-arg={}", obj.display());
}
