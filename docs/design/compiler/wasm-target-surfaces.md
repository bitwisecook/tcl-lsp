# WASM target surfaces: WASI vs the browser

> **Status:** audit + design. What the two WASM deployment surfaces of
> `runtime/rust` support, so that AOT direct-emission work targets the
> surface where it earns its keep. The other browser artefacts in the
> workspace — `tcl-vm-wasm` (the bytecode VM as a self-contained
> `wasm32-unknown-unknown` module with no host imports, `make tcl-vm-wasm`),
> `tcl-explorer-wasm`, and `tcl-lsp-server-wasm` — do not embed
> `runtime/rust` and are outside this document.

There are two distinct WASM deployment surfaces for `runtime/rust`, and they
are not the same target with different flags — they have different host
contracts entirely:

- **WASM + WASI** (`wasm32-wasip1`/`wasm32-wasip2`, run under wasmtime) — a
  real operating-system-shaped host: real files via preopens, real `fd_write`
  stdio, a real clock. This is what
  [`rust/tcl-compiler/tests/wasm_real_link.rs`](../../../rust/tcl-compiler/tests/wasm_real_link.rs)
  already exercises.
- **In-browser WASM** (`wasm32-unknown-unknown`, no WASI) — no file
  descriptors, no clock, no console, unless a JavaScript host supplies them
  as explicit imports. Nothing is ambient.

The two are covered by one [`Host`](../../../rust/tcl-platform/src/lib.rs)
trait and two concrete implementations in
[`runtime/rust/src/host_wasm.rs`](../../../runtime/rust/src/host_wasm.rs):
`WasiHost` and `BrowserHost`. This document audits what each host does,
measures the module `runtime/rust` produces, and proposes the smallest
host-import surface a real browser embedding would need.

## 1. Does `runtime/rust` build for `wasm32-unknown-unknown`?

**Yes.**

```
$ rustup target add wasm32-unknown-unknown wasm32-wasip1
$ cd runtime/rust
$ cargo build --target wasm32-unknown-unknown --features wasm_stdlib
```

It produces a real `tcl_runtime.wasm` cdylib with **zero WASI imports** —
disassemble the module (`wasm-dis`) and grep for `(import `: none.
Compare the `wasm32-wasip1` build of the same crate, which imports fifteen
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

## 2. Command family capability matrix

Grounded in the actual `Host` impls, not general Tcl knowledge.
`BrowserHost` (`host_wasm.rs`) keeps the trait defaults for
`filesystem()`/`sockets()`/`process()` — all `None` — and its `Clock`/`StdIo`/
`Env` are inert stubs. `WasiHost` overrides `filesystem()` (via the in-memory
[`MemFs`](../../../runtime/rust/src/mem_fs.rs)) only under the `wasm_stdlib`
feature; its `Clock` is the same stub as the browser's.

| Family | WASI (`wasip1`) | Browser (`unknown-unknown`) today | Backing file |
|---|---|---|---|
| `puts`/`flush` (stdout/stderr) | Real: `WasiStdIo` writes `fd_write` via `std::io::stdout()`/`stderr()`, flushed every call | **Discarded**: `BrowserStdIo::write_stdout`/`write_stderr` are empty bodies | `host_wasm.rs` `WasiStdIo`/`BrowserStdIo` |
| `gets`/`read`/`open`/`close`/`chan` on **files** | Works only via the VFS read path (`open_cmd`'s `std::fs` branch always errors under WASI without preopens; it falls back to `host.filesystem()`, i.e. the embedded-stdlib `MemFs`, for reads) | **Always fails**: `BrowserHost::filesystem()` returns `None` (the trait default), so `open`'s VFS fallback has nothing to read from, even with real file semantics never available in the first place | `cmd_chan.rs` `open_cmd`; `host_wasm.rs` |
| `file`/`glob`/`cd`/`pwd` | `pwd`/`cd` work (`WasiEnv` tracks a virtual cwd); `file exists`/`glob`/etc. work **only** against the seeded `MemFs` (embedded stdlib paths) | `pwd` reports `/` (`BrowserEnv::cwd` is hard-coded); `cd` always errors (`HostError::Unsupported`); every `file`/`glob` query against the (absent) filesystem reports false/empty, not an error | `cmd_fs.rs`; `host_wasm.rs` `BrowserEnv`/`WasiEnv` |
| `exec` | Explicit **unsupported** stub: `"exec" is not supported under the WASM runtime` | Same — target-neutral, both WASM hosts have no `Process` | `cmd_misc.rs::install`'s `unsupported_cmd` loop |
| `socket` | Same explicit unsupported stub | Same | `cmd_misc.rs` |
| `load`/`fileevent`/`fcopy` | Same explicit unsupported stub (native loading, event-driven channel copy — no event loop backing them) | Same | `cmd_misc.rs` |
| `clock` (seconds/format/scan) | **Runs, but wrong**: `BrowserClock::now_secs`/`now_millis` (shared by both WASM hosts — `WasiHost` never overrides the clock) hard-return `0`. `clock seconds` always reports the Unix epoch; `clock format`/`scan` still work as pure civil-date math over whatever timestamp is *given* | Identical — same stub struct | `host_wasm.rs` `BrowserClock`; `cmd_clock.rs` |
| `after`/`vwait`/event loop | **Compiles and "runs", but is not functionally a timer**: `cmd_event.rs`'s loop calls `std::thread::sleep`, which the Rust `std` for `wasm32-unknown-unknown` implements as **a no-op** (`library/std/src/sys/pal/wasm/thread.rs`: `sleep` just returns) — combined with a clock stuck at epoch `0`, deadline ordering has no real time axis to order against. Under WASI, `thread::sleep` is real (backed by `poll_oneoff`'s clock subscription), so timers work there | Same event-loop code path, same broken clock/sleep combination — no host import exists yet to fix either | `cmd_event.rs`; `host_wasm.rs` |
| `source`/`package require` | Work **only** with the `wasm_stdlib` feature: `WasiHost::new()` seeds `MemFs` with the embedded Tcl 9 library (`init.tcl`, `package.tcl`, `tcltest`, …) and reports `$TCL_LIBRARY=/tcl/library` via `WasiEnv`. Without the feature, `filesystem()` is `None` and both commands fail | **Always fail**, feature flag or not: `embedded_stdlib::seed()` is called nowhere outside `WasiHost::new()` (`#[cfg(target_os = "wasi")]`), so on the browser target it is simply never invoked — confirmed by disassembling the release module: the WASI build's binary literally contains the string `# tcl_library, which is the directory containing this init.tcl script.` (verbatim from the embedded `init.tcl`), the browser build's binary contains **none** of it, byte-for-byte absent | `embedded_stdlib.rs`; `host_wasm.rs` |
| `env`/`exit` | `env` array populates from `WasiEnv::vars()` — empty (nothing sets vars beyond `TCL_LIBRARY` under `wasm_stdlib`) but present, not an error. `exit` never terminates the host process on **any** target — it records the code via `Interp::set_exit`/`take_exit` and unwinds with `Code::Error`, so the embedding process (an LSP server, or a browser tab) survives | Identical — same target-neutral `exit_cmd`; `env` array is empty (`BrowserEnv::vars()` returns `Vec::new()`) | `builtins.rs::exit_cmd`; `host_wasm.rs` |
| `encoding` | UTF-8 pass-through, target-neutral (no encoding files loaded on any host) | Identical | `cmd_misc.rs::encoding_cmd` |

### Reading the matrix

- **Fine on both today**: `encoding`, `exit`, `expr`/`::tcl::mathfunc::*`
  (the numeric tower links on both wasm targets, §1), and
  arithmetic-free scripting (`proc`, `set`, `string`, `list`, control
  flow) — anything that never touches a `Host` capability.
- **WASI-only today, but only because of *seeding*, not the target**:
  `source`/`package require`/`file`/`glob` work under WASI purely because
  `WasiHost::new()` happens to call `embedded_stdlib::seed()`. Nothing about
  WASI itself provides this — it is an in-memory `MemFs`, not a real
  filesystem, and preopens/`path_open` are never used by this runtime at
  all. **The exact same wiring, done for `BrowserHost`, would make this work
  in the browser too** — this is a wiring gap, not a WASI-specific
  capability. See "Wiring gap, not a WASI dependency" immediately below for
  why this is confidently a small, mechanical fix; §3 proposes closing it
  without a host import at all, since `MemFs` needs no I/O.
- **Needs a real host-provided import to be *correct* in the browser**:
  `puts`/`clock`/a real timer tick for `after`/`vwait`. These need genuine
  host facilities (a console, `Date.now()`, a scheduler) that only
  JavaScript can provide across the wasm32-unknown-unknown boundary — no
  amount of runtime-side wiring fixes them.
- **Impossible/meaningless on either WASM surface**: `exec`, `socket`,
  `load`, `fileevent`, `fcopy` — all explicitly and honestly reported as
  unsupported rather than silently miscompiling or dispatching to
  `invalid command name`.

### Wiring gap, not a WASI dependency

This distinction decides whether a working browser stdlib bootstrap is days
or weeks away, so it is worth stating precisely, with the evidence: **it is
a pure wiring fix.** Neither `MemFs` (`mem_fs.rs`) nor `embedded_stdlib.rs`
contains a single `#[cfg]` attribute, a WASI import, or any I/O call:

- `mem_fs.rs`'s `MemFs` is `RefCell<BTreeMap<String, Vec<u8>>>` (files) plus
  `RefCell<BTreeSet<String>>` (explicit directories), implementing the
  `Filesystem` trait entirely through in-memory string/byte operations
  (`norm`, `child_prefix`, `read_dir`'s prefix walk, …). Nothing in the file
  references `std::fs`, WASI, or any target-specific API — it already
  builds and passes its own unit tests as an ordinary native crate module
  (`#[cfg(test)] mod tests` runs natively), and it is included in the
  `wasm32-unknown-unknown` build (module-gated only on
  `#[cfg(any(feature = "wasm_stdlib", test))]` in `lib.rs`, with no
  `target_os` restriction).
- `embedded_stdlib.rs`'s only non-trivial operation is `include_bytes!`
  (a compile-time, target-neutral embed) followed by `MemFs::insert` calls
  in `seed()`. Also no `#[cfg(target_os = …)]` anywhere in the file — its
  module declaration in `lib.rs` is gated only on
  `#[cfg(feature = "wasm_stdlib")]`.
- The **only** place `target_os = "wasi"` appears anywhere in this call
  chain is on `WasiHost`'s own `struct`/`impl` blocks in `host_wasm.rs` —
  a host-selection choice (which concrete `Host` impl a given target uses),
  not a capability boundary. `WasiHost::new()` happens to construct a
  `MemFs` and call `embedded_stdlib::seed(&fs)`; `BrowserHost::new()`
  simply never does either, and `BrowserHost`'s `filesystem()` falls through
  to the `Host` trait's `None` default because `BrowserHost` never
  overrides it.

Closing the gap is therefore: add a `#[cfg(feature = "wasm_stdlib")] fs:
crate::mem_fs::MemFs` field to `BrowserHost`, seed it in `BrowserHost::new()`
exactly as `WasiHost::new()` does, and override `filesystem()` to return
`Some(&self.fs)` — the same handful of lines already proven out in
`WasiHost`, copied to a struct that already compiles cleanly for
`wasm32-unknown-unknown` (§1). No new trait, no new capability, no research
question. `Env::cwd`/`chdir` would need the same small treatment (currently
`BrowserEnv::chdir` is a hard `Err(HostError::Unsupported)`) so `pwd`/`cd`
against the virtual mount behave like `WasiEnv`'s. This is realistically a
same-day change plus review, not a multi-week redesign — the multi-week
work, if any, is in §3's host-import surface (real console/clock/exit,
which *does* need new JS-facing plumbing), not in the stdlib bootstrap.

## 3. The smallest useful browser host-import surface

Given §2, closing the *wiring* gap (seeding `MemFs` into `BrowserHost`,
mirroring `WasiHost`) needs **no host import at all** — it is pure Rust data
already embedded in the binary. That should happen before any new import is
added, because it is what makes `tcl_runtime_init_library()` (the
`Tcl_Init`-equivalent the AOT `_start` bootstrap calls) succeed on the
browser target at all; today it always returns `1` there because
`init_library()` reads `$TCL_LIBRARY/init.tcl` through `host.filesystem()`,
which is `None`.

For the four things Rust cannot fabricate host-side — a console, a wall
clock, a real script source, and a graceful way to stop — the minimum import
surface is four functions, declared the same way as the existing
[`CodegenAbiImportId`](../../../rust/tcl-runtime-api/src/codegen_abi.rs)
table (module `"env"`, distinct from the compiler's own `"tcl"` module, so
the two ABIs never collide):

```rust
/// Write `len` bytes at `ptr` (UTF-8 Tcl string data) to the browser console.
/// `stream`: 0 = stdout, 1 = stderr. Mirrors `StdIo::write_stdout/write_stderr`.
(import "env" "tcl_host_console_write" (func (param $ptr i32) (param $len i32) (param $stream i32)))

/// Milliseconds since the Unix epoch (`Date.now()`), as an i64 split into two
/// i32 halves (wasm32 has no native i64 import value on some embedders'
/// JS-to-wasm boundaries; splitting keeps the import MVP-safe). Mirrors
/// `Clock::now_millis`.
(import "env" "tcl_host_now_millis_hi" (func (result i32)))
(import "env" "tcl_host_now_millis_lo" (func (result i32)))

/// Read the host-provided entry script into `runtime`-owned memory at `ptr`,
/// up to `cap` bytes; returns the actual length written (or -1 if `cap` is
/// too small — the caller re-queries with a larger buffer). Backs a single
/// `source`-equivalent for the top-level script only; `package require`
/// still needs the `MemFs` stdlib seed from above, not this import.
(import "env" "tcl_host_read_script" (func (param $ptr i32) (param $cap i32) (result i32)))

/// Report the interpreter's `exit` code (from `Interp::take_exit`) to the
/// host once evaluation completes, instead of the host guessing from a
/// completion code. A no-op host may ignore this; a browser embedding uses
/// it to decide whether to show an error banner.
(import "env" "tcl_host_exit" (func (param $code i32)))
```

These would live beside the existing declarations in
[`rust/tcl-runtime-api/src/codegen_abi.rs`](../../../rust/tcl-runtime-api/src/codegen_abi.rs)
as a new `BrowserHostImportId` enum (parallel to `CodegenAbiImportId`, same
`descriptor()` pattern), with the concrete glue implemented in a new
`BrowserHostImports` struct in `host_wasm.rs` that `BrowserHost` calls
through instead of its current inert stub bodies. `after`/`vwait` need no
new import beyond the clock above — once `now_millis` is real, the existing
`cmd_event.rs` deadline logic needs only a real (non-no-op) sleep primitive,
which on `wasm32-unknown-unknown` has no synchronous equivalent at all (no
blocking without an async runtime or `Atomics.wait` on a shared array
buffer) — that is a structural limitation of the browser's single-threaded
event loop, not a missing import, and is out of scope for "smallest useful
subset."

This list deliberately excludes filesystem, sockets, and process imports:
`Capabilities::FILESYSTEM`/`SOCKETS`/`PROCESS` stay unset for `BrowserHost`,
matching the honest "not supported under the WASM runtime" errors already in
place for `exec`/`socket`/`load` — nothing in this proposal changes that.

## 4. Module size

Measured by building `runtime/rust` exactly as
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

The `wasm_stdlib` feature costs the browser build next to nothing today
because `embedded_stdlib::seed` is called nowhere on the
`wasm32-unknown-unknown` target (a wiring question, not a numeric-code one —
see "Wiring gap, not a WASI dependency" in §2). Once §2's wiring fix lands
for `BrowserHost`, the ~250 KB `embedded_stdlib.rs`'s module doc describes
is paid on this target too.

## 5. Recommendation: what does AOT direct emission actually buy the browser?

**Mainly startup time and size — not steady-state speed** — and the size
win is more modest than "drop the interpreter" suggests, because most of
today's module is not the tree-walking interpreter's own code; it is Tcl's
supporting data (regex tables, numeric formatting, and — per §4 — a fair
amount of registry documentation text this runtime never needed to link in
the first place). Concretely:

- **Startup time** is the clearest, most defensible win. Every script run
  through the eval-fallback tier (`tcl_eval`/`tcl_eval_code`) re-parses and
  re-lexes Tcl source at every call, in a WASM module that has already paid
  the one-time cost of `Tcl_Init`-equivalent bootstrap (§2/§3) and, per §2,
  currently *cannot even complete that bootstrap* on the browser target.
  Direct emission (the `GenericInvoke`/prebuilt-argv plan the compiler
  already selects when it can — see
  [`wasm-codegen.md`](wasm-codegen.md)) skips re-lexing and re-parsing
  the source text on every invocation; that is a real, measurable
  wall-clock win independent of module size, and it matters more in a
  browser tab (where a user is waiting on first paint) than under wasmtime
  (where the process is usually long-lived).
- **Size** is a real but *secondary* win, and the ceiling on it is lower
  than "ship no interpreter" implies. Per §4, with the numeric tower now
  linked, roughly 3.25 MB of the 7.15 MB release module (~46%) is still
  static data — regex/Unicode tables, format strings, and (avoidably)
  F5/iRules documentation text — that direct command emission does not
  remove, because the emitted code still calls into shared library code
  (`tcl-regex`, `tcl-cmd-core`'s string/list helpers, now also libtommath)
  for anything beyond the literal-argv fast path. "Compile more commands
  directly" shrinks the *eval-fallback data segment* (the boxed Tcl-source
  strings for each unresolved command/condition) and the round-trip through
  `tcl_eval_code`'s string re-parse, but it does not remove `tcl-regex`'s
  tables, `tcl-registry`'s dispatch metadata, or the numeric tower's code
  and tables, which every build now links regardless of how many commands
  are direct-emitted. A more effective size lever, if the browser budget is
  tight, is trimming what `tcl-registry`/`tcl-cmd-core` link in for a
  browser artefact that will never see an iRules script — orthogonal to the
  AOT compiler question, and unaffected by whether the tower is present.
- **Steady-state speed** is the weakest case, because this audit has **no
  throughput measurement**. The eval-fallback
  tier's per-call overhead is a `tcl_eval_code` FFI call plus a fresh
  lex/parse of a short boxed string; for most single-command Tcl leaves,
  that overhead competes with the cost of the command's own work (list/dict
  operations, regex matches, now real arithmetic), and this document has
  not run a benchmark to say which dominates. Direct emission removes the
  fallback's string-round-trip overhead, and the compiler-explorer's own
  generic-invoke plan already reuses the runtime's ordinary dispatcher
  (namespaces, aliases, `unknown`) rather than a naive tree-walk, so the
  ceiling on a *speed*-motivated case for more direct emission is narrower
  than the startup-time case. A direct-emit-vs-eval-fallback throughput
  comparison with the tower linked is the natural follow-up before anyone
  makes a speed claim for this surface.

**Recommendation**: prioritise AOT direct-emission work for the browser
surface on **startup latency**, not module size or throughput — the shape of
the module (roughly half static data, dominated by tables direct emission
cannot touch) settles the size question. Concretely: (1) finish the
`MemFs`-seeding wiring for `BrowserHost` (§2/§3) so the interpreter can
complete its own bootstrap in a browser embedding — a prerequisite for *any*
browser deployment, AOT or interpreted, and a small, mechanical fix; (2)
treat "shrink the eval-fallback data segment via direct command emission"
as the AOT lever that matters for the browser, since it shortens the
critical path between "module instantiated" and "first script output
visible"; and (3) before making any *speed* claim, run the throughput
comparison rather than inferring one from module-size structure.

## Related

- [WASM code generation](wasm-codegen.md) — the canonical `compile_wasm`
  pipeline, plan selection, and the generic-argv ABI these numbers assume.
- [WASM runtime boundary](wasm-runtime-primitives.md) — the codegen ABI's
  invocation, completion, and ownership contracts.
- [WASM extensions](wasm-extensions.md) — the `wasm_stdlib` embedding
  boundary this document measures the cost of, and the not-yet-built
  package-driven extension design.
