//! Build the bignum backend: compile reference libtommath to a static archive
//! and link it, so the `TCL_BIGNUM_TYPE` obj rep can FFI the `mp_*` functions
//! (the numeric tower's bignum rung — see
//! `docs/design/runtime/rust-runtime-port.md`, EXP-BIGNUM).
//!
//! Recipe (validated native + wasm32, `experiments/bignum/`): build **pristine**
//! libtommath with `-DTCL_WITH_EXTERNAL_TOMMATH` (so its `.c` files use the real
//! `tommath.h`, not Tcl's stubs-renamed `tclTomMath.h`) + `-DLTM_ALL`, all
//! `libtommath/*.c` except `bn_deprecated`/`*rand*`/`*prime*` (the integer tower
//! needs no RNG/primality, and they pull unresolved RNG externals).
//!
//! Zero build-deps: shells out to the host C compiler + `ar` directly. Skipped
//! for the `wasm32` cdylib target, where the whole-program link (Track 3)
//! provides libtommath itself.
//!
//! Source location: `$TCL_TOMMATH_DIR`, else the fetched tree at
//! `<repo>/tmp/tcl9.0.3/libtommath`. (Vendoring the source for a
//! fresh-checkout-reproducible build is a tracked follow-up.)

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TCL_TOMMATH_DIR");
    println!("cargo:rerun-if-env-changed=TCL_WASM_CC");
    println!("cargo:rerun-if-env-changed=TCL_WASM_AR");
    // Register the cfg unconditionally — *before* any early return — so the
    // `#[cfg(have_tommath)]` gate is never reported as `unexpected_cfgs`
    // (which `-D warnings` turns into a hard lint failure) in the wasm or
    // no-source builds where the backend is disabled.
    println!("cargo:rustc-check-cfg=cfg(have_tommath)");

    // The regex engine is now the pure-Rust `tcl-regex` crate (a normal cargo
    // dependency); no C is compiled for it here. Only the bignum backend
    // (libtommath) is still linked from C.

    let target = env::var("TARGET").unwrap_or_default();
    let is_wasm = target.contains("wasm");

    let Some(ltm) = locate_libtommath() else {
        // No source tree (e.g. a checkout without `tmp/` fetched): skip the
        // bignum backend rather than fail the build. The bignum obj rep is
        // `#[cfg]`-gated on `have_tommath` so the rest of the runtime still
        // builds + tests.
        println!("cargo:warning=libtommath source not found; bignum backend disabled");
        return;
    };
    println!("cargo:rerun-if-changed={}", ltm.display());

    // Pick the C cross-compiler + archiver and target flags.
    //
    // Native: the host `cc`/`ar` (overridable via `CC`/`AR`), `-fPIC` for the
    // shared `cdylib`/`staticlib`.
    //
    // WASM: `zig cc`/`zig ar` cross-compile the *same* pristine libtommath to a
    // `wasm32-wasi` object archive, which rustc's wasm link (`rust-lld`) pulls
    // into the runtime `cdylib` — so the numeric tower (`expr`, `::tcl::math*`,
    // `lseq`, the bignum obj rep) is present on wasm exactly as on native, and a
    // whole-program AOT link gets a self-contained tower with no host tower
    // dependency. `zig cc` bundles clang + wasi-libc and emits relocatable wasm
    // objects; `zig ar` (LLVM ar) writes a wasm-aware archive symbol index that
    // the host `ar` (GNU) does not. If `zig` is unavailable we degrade to the
    // prior tower-less wasm build rather than fail.
    let (cc, ar, extra_cflags): (String, String, &[&str]) = if is_wasm {
        let cc = env::var("TCL_WASM_CC").unwrap_or_else(|_| "zig cc".into());
        if !cc_available(&cc) {
            println!(
                "cargo:warning=wasm C compiler ({cc}) not found; bignum backend \
                 disabled on wasm (set TCL_WASM_CC to a wasm32-wasi clang)"
            );
            return;
        }
        let ar = env::var("TCL_WASM_AR").unwrap_or_else(|_| "zig ar".into());
        (cc, ar, &["--target=wasm32-wasi"])
    } else {
        let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
        let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
        (cc, ar, &["-fPIC"])
    };

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    let mut objs = Vec::new();
    for entry in fs::read_dir(&ltm).expect("read libtommath dir") {
        let path = entry.expect("dir entry").path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".c") => n.to_string(),
            _ => continue,
        };
        if name.contains("deprecated") || name.contains("rand") || name.contains("prime") {
            continue;
        }
        let obj = out.join(format!("{name}.o"));
        let status = split_cmd(&cc)
            .args(["-c", "-O2"])
            .args(extra_cflags)
            .args(["-DTCL_WITH_EXTERNAL_TOMMATH", "-DLTM_ALL", "-DMP_64BIT"])
            .arg("-I")
            .arg(&ltm)
            .arg(&path)
            .arg("-o")
            .arg(&obj)
            .status()
            .unwrap_or_else(|e| panic!("failed to spawn {cc}: {e}"));
        assert!(status.success(), "compiling {name} failed");
        objs.push(obj);
    }
    assert!(!objs.is_empty(), "no libtommath sources compiled");

    let lib = out.join("libtommath.a");
    let _ = fs::remove_file(&lib);
    let mut cmd = split_cmd(&ar);
    cmd.arg("rcs").arg(&lib);
    cmd.args(&objs);
    assert!(cmd.status().expect("ar").success(), "archiving failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=tommath");
    // Signals the bignum obj rep + FFI are available (gates `src/bignum.rs`).
    // (`rustc-check-cfg` for `have_tommath` is emitted unconditionally up top.)
    println!("cargo:rustc-cfg=have_tommath");
}

/// Build a [`Command`] from a possibly multi-word program string (e.g.
/// `"zig cc"` → program `zig`, arg `cc`). The first whitespace-separated token
/// is the program; the rest are leading arguments.
fn split_cmd(cmd: &str) -> Command {
    let mut parts = cmd.split_whitespace();
    let prog = parts.next().expect("empty compiler/archiver command");
    let mut c = Command::new(prog);
    c.args(parts);
    c
}

/// Whether the wasm C compiler can be spawned at all (a cheap `--version`
/// probe), so a missing `zig` degrades to a tower-less wasm build instead of a
/// hard `build.rs` panic.
fn cc_available(cmd: &str) -> bool {
    split_cmd(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Find the pristine libtommath source dir.
fn locate_libtommath() -> Option<PathBuf> {
    if let Ok(dir) = env::var("TCL_TOMMATH_DIR") {
        let p = PathBuf::from(dir);
        if p.join("tommath.h").is_file() {
            return Some(p);
        }
    }
    // <crate>/../../tmp/tcl9.0.3/libtommath (the session-fetched tree).
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let candidate = manifest
        .join("../../tmp/tcl9.0.3/libtommath")
        .canonicalize()
        .ok()?;
    candidate.join("tommath.h").is_file().then_some(candidate)
}

#[allow(dead_code)]
fn exists(p: &Path) -> bool {
    p.exists()
}
