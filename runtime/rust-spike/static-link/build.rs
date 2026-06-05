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
    println!("cargo:rerun-if-env-changed=SPIKE_CLANG");

    let clang = env::var("SPIKE_CLANG").unwrap_or_else(|_| "clang".into());

    // --target=wasm32-wasi: compile-only (-c) needs no wasi sysroot because the
    // extension includes no system headers -- only our include/tcl.h (which
    // pulls in clang's freestanding stddef.h/stdint.h).  -nostdlib keeps it
    // freestanding; -fno-builtin avoids implicit libc references.
    let status = Command::new(&clang)
        // -ffreestanding: use clang's own self-contained stddef.h/stdint.h
        // instead of falling back to the host glibc headers (there is no wasi
        // sysroot installed). The extension includes no system headers anyway.
        .args(["--target=wasm32-wasi", "-ffreestanding", "-O2", "-fno-builtin", "-nostdlib", "-c"])
        .arg("-I")
        .arg(&inc)
        .arg(&src)
        .arg("-o")
        .arg(&obj)
        .status()
        .unwrap_or_else(|e| panic!("failed to invoke {clang}: {e}"));
    assert!(status.success(), "clang failed to compile ext/pkga.c");

    // Pass the freshly-built extension object to the final wasm link step.
    println!("cargo:rustc-link-arg={}", obj.display());
}
