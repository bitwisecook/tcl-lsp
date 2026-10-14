# Recorded WASM target build measurements

These are the original recorded observations from
`docs/design/compiler/wasm-target-surfaces.md`, retained independently of the
current host contract. They were not rerun during this documentation cleanup.
The original document does not retain a complete provider/build-input closure
or a measured source revision. Its source/toolchain/output descriptions are
bounded observations, not current module-size guarantees or conformance evidence.

Original document SHA-256: `e4150bdc9836b13a94fdd62dcef8a0ff152fa8fda2983df6b337dfc423df1667`.
Archive source: repository document at the current patch's recorded preimage.

## Original build and linkage observations
 Does `runtime/rust` build for `wasm32-unknown-unknown`?

**Yes.**

```
$ rustup target add wasm32-unknown-unknown wasm32-wasip1
$ cd runtime/rust
$ cargo build --target wasm32-unknown-unknown --features wasm_stdlib
```

It produces a real `tcl_runtime.wasm` cdylib with **zero WASI imports** —
disassemble the module (`wasm-dis`) and grep for `(import `: none.
The measured `wasm32-wasip1` build of the same crate imports fifteen
`wasi_snapshot_preview1` functions (`environ_get`, `clock_time_get`,
`fd_write`, `path_open`, `poll_oneoff`, `random_get`, …) pulled in by Rust's
`std` runtime bootstrap, not by anything this crate calls directly.

So the premise "WASI-only" is wrong at the **build** level: `BrowserHost`
already exists precisely to make the crate wasm32-clean (its module doc says
so explicitly — "Its job is to make `runtime/rust` build and link for
`wasm32-unknown-unknown`"), and it does. The `tcl-host-native` dependency
(the one piece that pulls in `std::fs`/`std::process` unconditionally) is
target-gated in
[`runtime/rust/Cargo.toml`](../../../runtime/rust/Cargo.toml) to
`cfg(not(target_arch = "wasm32"))`, so it never reaches either wasm target.
What blocks a *useful* browser deployment is not the build — it is that
almost every capability `BrowserHost` reports is a stub. See §2.

### The numeric tower links on both targets

`runtime/rust/build.rs` cross-compiles libtommath (the bignum backend behind
`expr`/`::tcl::mathfunc::*`) as C, **unconditionally through a
`--target=wasm32-wasi` sysroot** — its `is_wasm` branch always passes
`"--target=wasm32-wasi"` plus the wasi-sdk sysroot, whichever *Rust* wasm
target is being built. That object code links cleanly into
`wasm32-unknown-unknown`: the export list gains `calloc`/`malloc`/`realloc`/
`free` on both wasm targets (wasi-sdk's sysroot libc allocator, which
libtommath calls), and the `#[cfg(have_tommath)]` arms of
`tcl_codegen_expr_add` and `expr_bool_impl` are the code that ships.

Without `TCL_TOMMATH_DIR` (`tmp/tcl9.0.4/libtommath`) and `WASI_SDK_PATH`,
`build.rs` takes its graceful-degradation path — `libtommath source not
found; bignum backend disabled`, `have_tommath` unset, `expr`/arithmetic
compiled out — so a size or behaviour measurement must confirm the warning
is absent.

## Original module-size observations
 Module size

The sizes below are measurements of one build, rather than guarantees
for every revision or toolchain. The build uses `runtime/rust` as
[`wasm_real_link.rs`](../../../rust/tcl-compiler/tests/wasm_real_link.rs)
does (same crate, same `--global-base=2097152` linker flag for the WASI
build), with the `wasm_stdlib` feature on and the numeric tower linked
(`TCL_TOMMATH_DIR=tmp/tcl9.0.4/libtommath`, `WASI_SDK_PATH=/opt/wasi-sdk`;
confirmed via the absent `libtommath source not found` warning and the
`calloc`/`malloc`/`realloc`/`free` exports — see §1).

| Build | Raw size | `wasm-opt -Oz` | gzip -9 (raw) | gzip -9 (opt) |
|---|---:|---:|---:|---:|
| `wasm32-unknown-unknown`, debug | 13.19 MB (13,188,109 B) | not measured (debug) | — | — |
| `wasm32-unknown-unknown`, release | 7.15 MB (7,145,368 B) | 5.75 MB (5,754,122 B) | 1.86 MB (1,860,974 B) | 1.88 MB (1,884,649 B) |
| `wasm32-wasip1`, debug | 13.63 MB (13,628,945 B) | not measured (debug) | — | — |
| `wasm32-wasip1`, release | 7.45 MB (7,446,457 B) | 6.00 MB (5,994,025 B) | 1.95 MB (1,953,239 B) | 1.97 MB (1,965,922 B) |

The tower costs roughly 180–190 KB raw per release build — under 3% of the
total, because libtommath is a small, allocation-light C library once the
parts Tcl doesn't use (RNG, primality testing) are excluded (`build.rs`
skips `*rand*`/`*prime*`/`bn_deprecated`).

`wasm-opt -Oz` (binaryen v123) cuts raw size by ~19–20% and **very slightly
increases the gzip'd size** in both builds — unopt→opt gzip goes
1,860,974→1,884,649 for the browser build (+1.3%) and 1,953,239→1,965,922
for WASI (+0.6%): `-Oz`'s size-optimised code shape is less repetitive than
`rustc`'s own output and compresses marginally worse. For a browser
deployment served compressed (the normal case), `wasm-opt -Oz` is close to
a wash on transfer size; its value is faster parse/instantiate, not smaller
payload.

Structural breakdown (`wasm-opt --metrics`, release, `wasm_stdlib` on, tower
linked):

| | `unknown-unknown` | `wasip1` |
|---|---:|---:|
| functions | 2,554 | 2,844 |
| imports | 0 | 15 (`wasi_snapshot_preview1`) |
| exports | 54 | 50 |
| data-segment bytes | 3,254,544 (~3.25 MB) | 3,454,192 (~3.45 MB) |

The tower is almost entirely **code**, not data. Static data is **~45.6%**
of the `unknown-unknown` release module (3,254,544 / 7,145,368) and
**~46.4%** of `wasip1` (3,454,192 / 7,446,457): roughly half the module,
dominated by string tables — regex/Unicode tables, format strings, and
`tcl-registry` documentation text (`strings -n 15` finds ~193 KB of
unmistakably F5 iRules/BigIP command documentation, e.g. `"This command is
valid only for following MQTT message types:"`). That text is a real,
modest (under 3% of the release binary), tower-independent size-reduction
opportunity — not the dominant cost.

The `wasm_stdlib` feature costs the measured browser build next to nothing
because `embedded_stdlib::seed` is called nowhere on the
`wasm32-unknown-unknown` target (a wiring question, not a numeric-code one —
see "Wiring gap, not a WASI dependency" in §2). Seeding `BrowserHost` with
the embedded library would add the ~250 KB described in
`embedded_stdlib.rs`'s module documentation to this target too.

