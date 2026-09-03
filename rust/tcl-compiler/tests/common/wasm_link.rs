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

//! The real-link toolchain: the gate, the reserved-runtime build, and the
//! per-checkout scratch paths every WASM whole-program suite shares.
//!
//! Two suites link emitted modules against the *real* `runtime/rust` compiled
//! to `wasm32-wasip1`: `wasm_real_link.rs` (hand-written bootstraps proving one
//! ABI surface at a time) and `wasm_tiers.rs` (every `samples/wasm` script
//! diffed against its `tclsh9.0` oracle). Both need the identical four-part
//! toolchain check, the identical `--global-base=0x200000` runtime build, and
//! the identical per-checkout/per-process scratch discipline — so it lives here
//! once rather than being copied, because a *divergent* copy is exactly how a
//! suite ends up linking someone else's `tcl_runtime.wasm` (issue #1590) or
//! silently skipping in CI (issue #1542).
#![allow(dead_code)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The environment variable that turns every real-link skip into a
/// failure. Set it in any environment that is *supposed* to have the whole
/// toolchain — above all CI, where a silent skip is indistinguishable from a
/// pass and made this entire file vacuous (issue #1542).
pub const REQUIRE_VAR: &str = "TCL_REQUIRE_WASM_LINK";

/// The workspace root (`CARGO_MANIFEST_DIR` is `…/rust/tcl-compiler`).
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Where the reserved `wasm32-wasip1` runtime is built: **inside this
/// checkout's own `target/`**, never a machine-global `/tmp` name.
///
/// This is the isolation that matters most, and getting it wrong is what
/// issue #1590 turned out to be. A shared `--target-dir` means every
/// concurrent checkout builds its own `runtime/rust` over the same
/// `tcl_runtime.wasm`; cargo's lock serialises the *builds* but nothing holds
/// it across the gap between building the runtime and linking against it, so a
/// suite can link a runtime compiled from a different checkout's source. When
/// that stale runtime lacks the current tree's host wiring, `puts` writes into
/// a hostless `StdIo` and is silently dropped (`runtime/rust`'s
/// `cmd_chan.rs:474`) — the module still runs, still exits 0, and the
/// assertion reads `"2"` against `"6\n2"`. That is exactly the shape #1542 was
/// filed as, and it is indistinguishable from a codegen regression.
///
/// Keying on the checkout keeps the first-run build cached (the point of a
/// fixed path) without ever handing this suite someone else's runtime, and
/// living under `target/` means CI's existing cache covers it — a `/tmp` path
/// would be rebuilt from scratch on every run.
pub fn reserved_runtime_target_dir() -> PathBuf {
    workspace_root().join("target/tcl_reserved_runtime")
}

/// A scratch path for one transient `.wasm` / `.wat`, private to this
/// **checkout** and this **process**.
///
/// Two independent collisions had to be closed. The `tag` callers weave into
/// `name` keeps concurrent *cases* apart, but only within one process, and the
/// path was otherwise machine-global: two checkouts running this suite at the
/// same time (one worktree per agent, or a `make test` beside an editor's test
/// run) wrote, read, and `remove_file`d each other's modules mid-run. The
/// per-checkout directory closes the cross-worktree hole and the pid prefix
/// closes the cross-process one, leaving `tag` doing its original job.
///
/// These stay under the system temp dir rather than `target/`: they are
/// deleted immediately after each run and there is nothing to cache.
///
/// *Short* because "unique" must not mean "deep" — `make test-ext` puts a VS
/// Code IPC socket under the system temp dir and an `AF_UNIX` `sun_path` is
/// capped at 103 bytes, so `/tmp/tclwl-XXXXXXXX/<pid>-<name>` stays well
/// inside it. (A `TMPDIR` that is itself deep eats into that budget; nothing
/// here can recover it, so keep agent `TMPDIR`s shallow.)
///
/// The directory id is an FNV-1a hash of the **canonicalised** workspace root
/// — `workspace_root` resolves symlinks and `..`, so two spellings of one
/// checkout hash alike, and the id is stable across runs and toolchain
/// upgrades.
pub fn scratch(name: &str) -> PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in workspace_root().as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // Deliberately truncating: 32 bits of an FNV-1a digest is plenty to keep
    // concurrent checkouts apart, and eight hex digits keep the path short.
    let dir = std::env::temp_dir().join(format!("tclwl-{:08x}", hash & 0xffff_ffff));
    std::fs::create_dir_all(&dir).expect("create the real-link scratch root");
    dir.join(format!("{}-{name}", std::process::id()))
}

/// One dimension of the real-link toolchain, with the remedy for it.
pub struct MissingRequirement {
    pub what: &'static str,
    pub remedy: String,
}

/// Whether `rustc` can see the `wasm32-wasip1` standard library. Asking rustc
/// rather than rustup keeps this working on a toolchain installed any other
/// way.
pub fn have_wasip1_target() -> bool {
    Command::new("rustc")
        .args(["--print", "target-libdir", "--target", "wasm32-wasip1"])
        .output()
        .is_ok_and(|out| {
            out.status.success() && Path::new(String::from_utf8_lossy(&out.stdout).trim()).is_dir()
        })
}

/// The wasi-sdk root whose `bin/clang` compiles libtommath to wasm, if present.
pub fn wasi_sdk_root() -> Option<PathBuf> {
    std::env::var_os("WASI_SDK_PATH")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from("/opt/wasi-sdk")))
        .filter(|path| path.join("bin/clang").is_file())
}

/// The pristine libtommath source tree the runtime's bignum backend is built
/// from, if present.
///
/// This is a **hard requirement**, not an optimisation. `runtime/rust`'s
/// `build.rs` degrades silently when it cannot find the source — it prints a
/// `cargo:warning` and builds with the bignum backend disabled — and the
/// resulting module then fails `expr {$b + $c}` inside the link, so `puts`
/// prints nothing and the assertion reads `"2"` against `"6\n2"`. That looks
/// exactly like a codegen regression and has cost at least one lane a
/// diagnosis (issue #1542). Every worktree hits it, because `tmp/` is
/// gitignored and so is empty in a fresh checkout.
pub fn libtommath_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("TCL_TOMMATH_DIR").map(PathBuf::from)
        && dir.join("tommath.h").is_file()
    {
        return Some(dir);
    }
    let root = workspace_root();
    [
        root.join("tmp/tcl9.0.4/libtommath"),
        root.join("tmp/tcl9.0.3-src/libtommath"),
        root.join("tmp/tcl8.6.16/libtommath"),
    ]
    .into_iter()
    .find(|path| path.join("tommath.h").is_file())
}

/// Every dimension of the toolchain this file needs that is currently absent,
/// each with the command that installs it. Empty means the real link can run.
///
/// Enumerating *all* of them (rather than returning at the first) matters:
/// a contributor who installs wasmtime, re-runs, and is then told about
/// wasi-sdk, re-runs, and is then told about the wasip1 target has paid three
/// slow round trips for one answer.
pub fn missing_requirements() -> Vec<MissingRequirement> {
    let mut missing = Vec::new();
    if !Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        missing.push(MissingRequirement {
            what: "the `wasmtime` CLI",
            remedy: "install it (https://wasmtime.dev), e.g. \
                     `curl https://wasmtime.dev/install.sh -sSf | bash`"
                .to_owned(),
        });
    }
    if !have_wasip1_target() {
        missing.push(MissingRequirement {
            what: "the `wasm32-wasip1` Rust target",
            remedy: "`rustup target add wasm32-wasip1`".to_owned(),
        });
    }
    if wasi_sdk_root().is_none() {
        missing.push(MissingRequirement {
            what: "wasi-sdk (`bin/clang`, to compile libtommath to wasm)",
            remedy: "install wasi-sdk to /opt/wasi-sdk, or point WASI_SDK_PATH at it \
                     (https://github.com/WebAssembly/wasi-sdk/releases)"
                .to_owned(),
        });
    }
    if libtommath_dir().is_none() {
        missing.push(MissingRequirement {
            what: "the libtommath source (the runtime's bignum backend)",
            remedy: format!(
                "fetch the Tcl 9.0.4 source into `{}/tmp` (the fetch-tcl-source helper), \
                 or export TCL_TOMMATH_DIR=<dir containing tommath.h>. \
                 Without it the runtime builds with bignums DISABLED and the link \
                 fails with a missing-output diff that looks like a codegen bug.",
                workspace_root().display()
            ),
        });
    }
    missing
}

/// The gate every real-link test opens with: `Some(runtime)` when the whole
/// toolchain is present and the reserved runtime built, otherwise a skip.
///
/// Under `TCL_REQUIRE_WASM_LINK=1` there is no skip — a missing dimension is a
/// panic naming every one of them and how to install it. That is the point of
/// issue #1542: without an assertive mode `wasm_real_link.rs`'s eight tests reported
/// green in CI while linking nothing at all, and the issue's own acceptance criterion
/// would have passed vacuously.
///
/// Without the variable the skip is still **loud and specific** — it names the
/// dimension, not just "skipping". Callers add their own suite name to the
/// message they print around it.
///
/// A *build* failure is treated the same way, and symmetrically: required ⇒
/// panic with cargo's own output, otherwise a loud skip. Every precondition
/// has already been checked at that point, so a failing build is far more
/// likely to be a real breakage than a missing tool — but a developer whose
/// unrelated `cargo test -p tcl-compiler` cannot build a wasm runtime should
/// still be told why rather than handed a red suite they did not cause.
#[must_use]
pub fn real_link_runtime() -> Option<PathBuf> {
    let required = std::env::var(REQUIRE_VAR).is_ok_and(|v| v != "0" && !v.is_empty());
    let mut report = String::new();
    for item in &missing_requirements() {
        let _ = writeln!(
            report,
            "  - missing {}\n    remedy: {}",
            item.what, item.remedy
        );
    }
    if report.is_empty() {
        match build_reserved_runtime() {
            Ok(runtime) => return Some(runtime),
            Err(why) => report = why,
        }
    }
    assert!(
        !required,
        "{REQUIRE_VAR} is set, so the real WASM link must actually run, but:\n{report}"
    );
    eprintln!(
        "SKIPPING the real WASM link:\n{report}\
         Set {REQUIRE_VAR}=1 to turn this skip into a failure."
    );
    None
}

/// Build `runtime/rust` to `wasm32-wasip1` with the reserved-region linker
/// flag, into this checkout's own target dir
/// ([`reserved_runtime_target_dir`]).
///
/// `Err(report)` carries cargo's own output for the caller to raise or report,
/// so the required-vs-skip decision stays in one place ([`real_link_runtime`])
/// rather than being taken twice with different rules. Every precondition is
/// checked before this runs, and the bignum backend is never silently disabled
/// — `TCL_TOMMATH_DIR` is always passed explicitly rather than left to the
/// build script's fallback.
pub fn build_reserved_runtime() -> Result<PathBuf, String> {
    let root = workspace_root();
    let target_dir = reserved_runtime_target_dir();
    let out = Command::new("cargo")
        .env("WASI_SDK_PATH", wasi_sdk_root().expect("wasi-sdk checked"))
        .env(
            "TCL_TOMMATH_DIR",
            libtommath_dir().expect("libtommath checked"),
        )
        .arg("build")
        .arg("--manifest-path")
        .arg(root.join("runtime/rust/Cargo.toml"))
        .args(["--target", "wasm32-wasip1"])
        .arg("--target-dir")
        .arg(&target_dir)
        // Reserve [0x100000, 0x200000): the 1 MiB shadow stack stays at
        // [0, 0x100000), data/heap move to >= 0x200000, leaving the gap free for
        // the emitted constant pool.
        .env("RUSTFLAGS", "-C link-arg=--global-base=2097152")
        .output()
        .map_err(|e| format!("  - could not run cargo for the wasm32-wasip1 runtime: {e}\n"))?;
    if !out.status.success() {
        return Err(format!(
            "  - the wasm32-wasip1 reserved runtime failed to build.\n    \
             Every toolchain precondition was present, so this is far more \
             likely a real breakage than a missing tool.\n\
             --- cargo stdout ---\n{}\n--- cargo stderr ---\n{}\n",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        ));
    }
    let artifact = target_dir.join("wasm32-wasip1/debug/tcl_runtime.wasm");
    if !artifact.is_file() {
        return Err(format!(
            "  - cargo reported success but {} does not exist\n",
            artifact.display()
        ));
    }
    Ok(artifact)
}
