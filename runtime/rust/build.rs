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

//! Build the bignum backend: compile reference libtommath to a static archive
//! and link it, so the `TCL_BIGNUM_TYPE` obj rep can FFI the `mp_*` functions
//! (the numeric tower's bignum rung — EXP-BIGNUM).
//!
//! Recipe (validated native + wasm32, `experiments/bignum/`): build **pristine**
//! libtommath with `-DTCL_WITH_EXTERNAL_TOMMATH` (so its `.c` files use the real
//! `tommath.h`, not Tcl's stubs-renamed `tclTomMath.h`) + `-DLTM_ALL`, all
//! `libtommath/*.c` except `bn_deprecated`/`*rand*`/`*prime*` (the integer tower
//! needs no RNG/primality, and they pull unresolved RNG externals).
//!
//! Native build has zero build-deps: shells out to the host C compiler + `ar`
//! directly. The wasm cross-compile uses clang + a WASI sysroot (wasi-sdk),
//! located via `WASI_SDK_PATH`; absent that, the bignum backend degrades to a
//! tower-less wasm build (with a `cargo:warning`) rather than failing.
//!
//! Source location: `$TCL_TOMMATH_DIR`, else the fetched tree at
//! `<repo>/tmp/tcl9.0.4/libtommath`. (Vendoring the source for a
//! fresh-checkout-reproducible build is a tracked follow-up.)

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=TCL_TOMMATH_DIR");
    println!("cargo:rerun-if-env-changed=TCL_WASM_CC");
    println!("cargo:rerun-if-env-changed=TCL_WASM_AR");
    println!("cargo:rerun-if-env-changed=TCL_WASM_SYSROOT");
    println!("cargo:rerun-if-env-changed=WASI_SDK_PATH");
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

    // Reserve the low data window the AOT WASM backend relocates its constant
    // pool into (`tcl-compiler` `RESERVED_DATA_BASE` = 0x10_0000). Rust's wasm32
    // layout is stack-first with a 1 MiB stack, so the runtime's own data would
    // otherwise start at 0x10_0000 and *collide* with the emitted pool once the
    // two are merged into one module (`wasm-merge`). Push the runtime's data to
    // 0x20_0000 so `[0x10_0000, 0x20_0000)` is a free 1 MiB window for the pool —
    // safe between the downward-growing stack (`[0, 0x10_0000)`) and runtime data
    // (`[0x20_0000, …)`). Transparent to native (absolute addressing); the only
    // cost is ~1 MiB more zero-init linear memory in the wasm module.
    if is_wasm {
        println!("cargo:rustc-link-arg=--global-base=2097152"); // 0x20_0000
        // Export the wasm indirect function table, and let it grow.
        //
        // A wasm32 function pointer *is* an index into this table, so an
        // emitted module that wants the runtime to call one of its functions
        // (issue #1774's native proc entries) has to install a `ref.func` into
        // a table the runtime can `call_indirect` over. `wasm-ld` links the
        // table private and fixed-size by default (`(table 2 2 funcref)`),
        // which makes both halves impossible: the module cannot import it, and
        // `table.grow` on a table at its maximum returns `-1`.
        //
        // `--export-table` publishes it as `__indirect_function_table` (the
        // name `WASM32_FUNCTION_TABLE_IMPORT` in `tcl-runtime-api` owns) and
        // `--growable-table` drops the maximum so a module can `table.grow`
        // room for its own entries. Both are inert for a module that never
        // touches the table: the runtime's own indirect calls index below the
        // initial size, and an emitted module imports the table only when it
        // actually binds a native proc, so an older module against this
        // runtime is unaffected.
        println!("cargo:rustc-link-arg=--export-table");
        println!("cargo:rustc-link-arg=--growable-table");
    }

    let Some(ltm) = locate_libtommath() else {
        // No source tree (e.g. a checkout without `tmp/` fetched): skip the
        // bignum backend rather than fail the build. The bignum obj rep is
        // `#[cfg]`-gated on `have_tommath` so the rest of the runtime still
        // builds + tests.
        //
        // Re-probe on every build while the sources are absent: a watched
        // path that never exists keeps the fingerprint stale, so the first
        // build after a fetch re-runs this script and turns the tower on.
        // (Watching the candidate directory itself is not enough — tar
        // restores the archive's old mtimes on extraction, so the fetched
        // tree looks *older* than the cached probe and cargo would keep the
        // tower-less result until `build.rs` was touched by hand.)
        if let Ok(out) = env::var("OUT_DIR") {
            let never = PathBuf::from(out).join("force-tower-reprobe");
            println!("cargo:rerun-if-changed={}", never.display());
        }
        println!("cargo:warning=libtommath source not found; bignum backend disabled");
        return;
    };
    println!("cargo:rerun-if-changed={}", ltm.display());

    // Pick the C cross-compiler + archiver and target flags.
    //
    // Native: the host `cc`/`ar` (overridable via `CC`/`AR`), `-fPIC` for the
    // shared `cdylib`/`staticlib`.
    //
    // WASM: clang + a WASI sysroot (wasi-sdk) cross-compile the *same* pristine
    // libtommath to a `wasm32-wasi` object archive, which rustc's wasm link
    // (`rust-lld`) pulls into the runtime `cdylib` — so the numeric tower
    // (`expr`, `::tcl::math*`, `lseq`, the bignum obj rep) is present on wasm
    // exactly as on native, and a whole-program AOT link gets a self-contained
    // tower with no host tower dependency. The toolchain is located via
    // `WASI_SDK_PATH` (`$WASI_SDK_PATH/bin/clang` + `.../share/wasi-sysroot` +
    // `$WASI_SDK_PATH/bin/llvm-ar`, which writes a wasm-aware archive symbol
    // index the host GNU `ar` does not); each piece is overridable via
    // `TCL_WASM_CC` / `TCL_WASM_SYSROOT` / `TCL_WASM_AR`. If neither a wasi-sdk
    // nor an explicit sysroot is available we degrade to a tower-less wasm build
    // (with a warning) rather than fail.
    let (cc, ar, extra_cflags): (String, String, Vec<String>) = if is_wasm {
        // Locate a wasi-sdk: `WASI_SDK_PATH` if set, else the canonical
        // `/opt/wasi-sdk` the dep installers drop it at. A usable one has a
        // `bin/clang` under it; anything else falls through to the graceful
        // tower-less path below.
        let wasi_sdk = env::var("WASI_SDK_PATH")
            .ok()
            .map(PathBuf::from)
            .or_else(|| Some(PathBuf::from("/opt/wasi-sdk")))
            .filter(|p| p.join("bin/clang").is_file());

        let cc = env::var("TCL_WASM_CC").unwrap_or_else(|_| match &wasi_sdk {
            Some(p) => p.join("bin/clang").display().to_string(),
            None => "clang".into(),
        });
        if !cc_available(&cc) {
            println!(
                "cargo:warning=wasm C compiler ({cc}) not found; bignum backend \
                 disabled on wasm (install wasi-sdk and set WASI_SDK_PATH, or set \
                 TCL_WASM_CC to a wasm32-wasi clang)"
            );
            return;
        }

        // The WASI sysroot (headers + libc) is required for the C compile. Take
        // an explicit override first, then the wasi-sdk's bundled sysroot.
        let Some(sysroot) = env::var("TCL_WASM_SYSROOT").ok().or_else(|| {
            wasi_sdk
                .as_ref()
                .map(|p| p.join("share/wasi-sysroot").display().to_string())
        }) else {
            println!(
                "cargo:warning=no WASI sysroot found; bignum backend disabled on \
                 wasm (install wasi-sdk and set WASI_SDK_PATH, or set \
                 TCL_WASM_SYSROOT to a wasm32-wasi sysroot)"
            );
            return;
        };

        let ar = env::var("TCL_WASM_AR").unwrap_or_else(|_| match &wasi_sdk {
            Some(p) => p.join("bin/llvm-ar").display().to_string(),
            None => "llvm-ar".into(),
        });

        let cflags = vec![
            "--target=wasm32-wasi".to_string(),
            format!("--sysroot={sysroot}"),
        ];
        (cc, ar, cflags)
    } else {
        let cc = env::var("CC").unwrap_or_else(|_| "cc".into());
        let ar = env::var("AR").unwrap_or_else(|_| "ar".into());
        (cc, ar, vec!["-fPIC".to_string()])
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
            .args(&extra_cflags)
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

/// Build a [`Command`] from a possibly multi-word program string (e.g. a
/// `TCL_WASM_CC="clang --target=wasm32-wasi"` override → program `clang`, arg
/// `--target=wasm32-wasi`). The first whitespace-separated token is the
/// program; the rest are leading arguments.
fn split_cmd(cmd: &str) -> Command {
    let mut parts = cmd.split_whitespace();
    let prog = parts.next().expect("empty compiler/archiver command");
    let mut c = Command::new(prog);
    c.args(parts);
    c
}

/// Whether the wasm C compiler can be spawned at all (a cheap `--version`
/// probe), so a missing clang / wasi-sdk degrades to a tower-less wasm build
/// instead of a hard `build.rs` panic.
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
    // <crate>/../../tmp/tcl9.0.4/libtommath (the session-fetched tree).
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").ok()?);
    let candidate = manifest
        .join("../../tmp/tcl9.0.4/libtommath")
        .canonicalize()
        .ok()?;
    candidate.join("tommath.h").is_file().then_some(candidate)
}

#[allow(dead_code)]
fn exists(p: &Path) -> bool {
    p.exists()
}
