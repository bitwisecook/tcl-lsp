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
    println!("cargo:rerun-if-env-changed=TCL_REGEX_DIR");
    // Register the cfgs unconditionally — *before* any early return — so the
    // `#[cfg(have_tommath)]` / `#[cfg(have_regex)]` gates are never reported as
    // `unexpected_cfgs` (which `-D warnings` turns into a hard lint failure) in
    // the wasm or no-source builds where a backend is disabled.
    println!("cargo:rustc-check-cfg=cfg(have_tommath)");
    println!("cargo:rustc-check-cfg=cfg(have_regex)");

    // The wasm cdylib link (Track 3) supplies these C libraries; don't
    // cross-compile C with the host toolchain here.
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("wasm") {
        return;
    }

    build_regex();

    let Some(ltm) = locate_libtommath() else {
        // No source tree (e.g. a checkout without `tmp/` fetched): skip the
        // bignum backend rather than fail the build. The bignum obj rep is
        // `#[cfg]`-gated on `have_tommath` so the rest of the runtime still
        // builds + tests.
        println!("cargo:warning=libtommath source not found; bignum backend disabled");
        return;
    };
    println!("cargo:rerun-if-changed={}", ltm.display());

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".into());

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
        let status = Command::new(&cc)
            .args(["-c", "-O2", "-fPIC"])
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
    let mut cmd = Command::new(&ar);
    cmd.arg("rcs").arg(&lib);
    cmd.args(&objs);
    assert!(cmd.status().expect("ar").success(), "archiving failed");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=tommath");
    // Signals the bignum obj rep + FFI are available (gates `src/bignum.rs`).
    // (`rustc-check-cfg` for `have_tommath` is emitted unconditionally up top.)
    println!("cargo:rustc-cfg=have_tommath");
}

/// Build Tcl 9's Henry-Spencer regex engine (the *same* ARE engine `tclsh`
/// uses) to a static archive and link it, so `regexp`/`regsub` get exact Tcl
/// semantics rather than a re-derived approximation. Mirrors the Zig runtime
/// (`runtime/zig/regex_include/`, the behavioural oracle), retargeted to the
/// native host.
///
/// `regcomp.c`/`regexec.c` are the two real translation units — they `#include`
/// the `regc_*.c`/`rege_dfa.c` files (the amalgamation pattern). We copy the
/// engine sources into `$OUT_DIR/tcl-regex/` **omitting** the upstream
/// `regcustom.h`, then put `regex_shim/` first on the `-I` path: C searches the
/// including file's own directory first, so `regguts.h`'s
/// `#include "regcustom.h"` must NOT find a copy beside it (it would shadow our
/// override). The shim binds the engine's host hooks (heap, ASCII char classes,
/// the `Tcl_UniChar*` case/encode helpers) — see `regex_shim/`.
fn build_regex() {
    println!("cargo:rerun-if-changed=regex_shim");

    let Some(generic) = locate_tcl_generic() else {
        println!("cargo:warning=Tcl regex sources not found; regexp/regsub disabled");
        return;
    };
    println!("cargo:rerun-if-changed={}", generic.display());

    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let vendor = out.join("tcl-regex");
    let _ = fs::create_dir_all(&vendor);

    // The self-contained engine files. `regcustom.h` is deliberately excluded
    // (our `regex_shim/regcustom.h` overrides it via the `-I` order). The
    // `regc_*.c`/`rege_dfa.c` units are `#include`d by regcomp/regexec, not
    // compiled directly; `regfronts.c` (narrow-char `regcomp`/`regexec`) is
    // skipped — `__REG_NOFRONT`/`__REG_NOCHAR` in our regcustom.h drop it.
    let copy_files = [
        "regcomp.c",
        "regexec.c",
        "regfree.c",
        "regerror.c",
        "regc_color.c",
        "regc_cvec.c",
        "regc_lex.c",
        "regc_locale.c",
        "regc_nfa.c",
        "rege_dfa.c",
        "regerrs.h",
        "regex.h",
        "regguts.h",
    ];
    for f in copy_files {
        let src = generic.join(f);
        let dst = vendor.join(f);
        fs::copy(&src, &dst).unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shim_dir = manifest.join("regex_shim");
    let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
    let ar = env::var("AR").unwrap_or_else(|_| "ar".into());

    // The two real translation units + the host-hook shim.
    let units = [
        vendor.join("regcomp.c"),
        vendor.join("regexec.c"),
        vendor.join("regfree.c"),
        vendor.join("regerror.c"),
        shim_dir.join("regex_shim.c"),
    ];
    let mut objs = Vec::new();
    for src in &units {
        let stem = src.file_name().and_then(|n| n.to_str()).unwrap();
        let obj = out.join(format!("{stem}.o"));
        let status = Command::new(&cc)
            .args(["-c", "-O2", "-fPIC", "-std=c11", "-w"])
            // shim first (our regcustom.h / tclInt.h win), then the vendored
            // engine (regex.h / regguts.h).
            .arg("-I")
            .arg(&shim_dir)
            .arg("-I")
            .arg(&vendor)
            .arg(src)
            .arg("-o")
            .arg(&obj)
            .status()
            .unwrap_or_else(|e| panic!("failed to spawn {cc}: {e}"));
        assert!(status.success(), "compiling {} failed", src.display());
        objs.push(obj);
    }

    let lib = out.join("libtclregex.a");
    let _ = fs::remove_file(&lib);
    let mut cmd = Command::new(&ar);
    cmd.arg("rcs").arg(&lib);
    cmd.args(&objs);
    assert!(
        cmd.status().expect("ar").success(),
        "archiving regex failed"
    );

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=tclregex");
    println!("cargo:rustc-cfg=have_regex");
}

/// Locate Tcl 9's `generic/` source dir (holds the regex engine). Honours
/// `$TCL_REGEX_DIR`, else the session-fetched tree.
fn locate_tcl_generic() -> Option<PathBuf> {
    if let Ok(dir) = env::var("TCL_REGEX_DIR") {
        let p = PathBuf::from(dir);
        if p.join("regcomp.c").is_file() {
            return Some(p);
        }
    }
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let candidate = manifest
        .join("../../tmp/tcl9.0.3/generic")
        .canonicalize()
        .ok()?;
    candidate.join("regcomp.c").is_file().then_some(candidate)
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
