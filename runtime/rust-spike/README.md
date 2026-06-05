# Rust-runtime C-extension spikes

> **These are throwaway spikes, not the final design.** Every source file here
> carries a `SPIKE` banner. They prove an approach works end to end; do not
> derive the production runtime, ABI, or `tcl.h` shape from their code. The
> durable artifact is the design doc:
> [`docs/design/runtime/c-extension-abi.md`](../../docs/design/runtime/c-extension-abi.md).

**Question:** if the WASM runtime is rewritten in Rust, can we still compile an
*unmodified* C Tcl extension to WebAssembly and link it against our runtime and
the compiled user code, with **no per-extension shim**, defining the ABI
ourselves (API compatibility, not binary ABI)?

**Answer: yes — proven two ways.**

Both spikes take [`ext/pkga.c`](ext/pkga.c) — a real Tcl extension, vendored
**byte-identical** from Tcl 9.0.3 `unix/dltest/pkga.c` (the file Tcl's own
`load.test` uses) — compile it with **clang** against an authored
[`include/tcl.h`](include/tcl.h) we own, and run it against a **Rust** runtime
that implements the Tcl C API. The extension source is never touched.

## Layout

```
runtime/rust-spike/
├── include/tcl.h        authored, API-compatible subset (shared by both spikes)
├── ext/pkga.c           the unmodified real extension (shared)
├── static-link/         SPIKE 1: extension + runtime linked into ONE wasm
│   ├── src/main.rs      minimal Rust runtime (Tcl C API) + driver
│   ├── build.rs         clang-compiles ext/pkga.c, hands object to rustc
│   └── run.sh
└── dynamic-link/        SPIKE 2: extension loaded at runtime as a side module
    ├── runtime/src/lib.rs  Rust runtime cdylib (exports memory + table + API)
    ├── loader.py           host loader (dylink.0 + memory/table base wiring)
    └── run.sh
```

## Spike 1 — static link (`static-link/run.sh`)

Everything in one module: clang compiles `ext/pkga.c` to a wasm object against
our `tcl.h`; `rustc`/`wasm-ld` links it with the Rust runtime into one
`wasm32-wasip1` binary; `wasmtime` runs it. This is the whole-program link model
(like `core/compiler/codegen/wasm_link.py`). It proves API compatibility,
Rust↔C wasm interop, and the runtime→extension `call_indirect` callback.

## Spike 2 — dynamic link (`dynamic-link/run.sh`)

The real `package require` model. `ext/pkga.c` is built `-fPIC -shared` as a
separate `dylink.0` **side module**. A host loader:

- parses the side module's `dylink.0` footprint (data bytes + table slots),
- reserves a memory region (`__memory_base`) and table slots (`__table_base`)
  from the **runtime's** shared linear memory and shared function table,
- wires the side module's `Tcl_*` imports to the runtime's exports,
- runs `__wasm_apply_data_relocs` + `__wasm_call_ctors` + `Pkga_Init`,
- then dispatches commands through the runtime, which `call_indirect`s back into
  the dynamically-loaded extension via the shared table.

This proves the genuinely-novel mechanism: cross-module shared memory + shared
function table, host-driven relocation, and runtime↔extension calls — none of
which depend on the runtime being Rust vs Zig (clang + `wasm-ld` do the C work).

## Run

```
runtime/rust-spike/static-link/run.sh     # -> SPIKE PASS
runtime/rust-spike/dynamic-link/run.sh    # -> SPIKE PASS
```

Both print `SPIKE PASS` (exit 0) and exercise `pkga_eq` / `pkga_quote`, including
a multi-byte UTF-8 case (`café`). Requirements (already present here): the repo's
`stable` Rust toolchain with `wasm32-wasip1` + `wasm32-unknown-unknown`, `clang`,
`wasm-ld`, `wasmtime`, and `uv` (the dynamic loader runs via
`uv run --with wasmtime`). **No wasi sysroot is needed** — the extension includes
no system headers, and it is compiled `-ffreestanding`.

## Why the runtime's language is not the gate

- The C compiler is **clang** either way (`zig cc` is clang underneath). It, not
  the runtime language, decides whether C extensions compile to WASM.
- Rust expresses the whole C ABI surface: `#[no_mangle] extern "C"`,
  `#[repr(C)] Tcl_Obj`, nullable `extern "C" fn` pointers, exported memory +
  function table, and shared-table `call_indirect`.
- **"API not ABI" removes the hard part**: extensions recompile against *our*
  `tcl.h`, so we drop Tcl's 600-slot binary stubs table and use direct C-ABI
  imports. The header is the only "shim", written once.

## What a production `tcl.h` needs (from the extension survey)

A survey of 25+ real extensions found **~85–90% are public-`tcl.h`-only** at the
Tcl level. A production runtime header must cover, beyond the obj/command/eval
core these spikes already exercise:

| Surface | Why | Public? |
|---|---|---|
| Channel API + `Tcl_ChannelType` | TclTLS (transform/stacking), Memchan | yes (`tcl.h`) |
| Custom `Tcl_ObjType` registration | VecTcl, tcllib `sha1c`/`md5c` | yes (`tcl.h`) |
| `Tcl_FSRegister` / `Tcl_Filesystem` | tclvfs | yes (`tcl.h`) |
| Threading (`Tcl_CreateThread`, mutex/cond, `Tcl_GetThreadData`) | Thread | yes (`tcl.h`) |
| NRE entry points (`Tcl_NRCallObjProc`) | coroutine-aware extensions | yes (`tcl.h`) |
| `tclOO.h` (+`Tcl_OOInitStubs`) | classes-in-C (e.g. `pkgooa.c`) | yes, sibling header |
| `tclTomMath.h` | raw `mp_*` bignum arithmetic | yes, sibling header |

`tclInt.h` (internal) is **out of scope**: in the survey only TclX and Expect
needed it, and both are blocked from WASM by deep POSIX dependencies (ptys,
chroot) regardless. The practical gate for any extension is almost never the Tcl
API — it is whether its **third-party native library** (OpenSSL, libcurl, libpq,
X11, audio, libtcc) can itself reach WASM.

## What these spikes deliberately do NOT cover

- **Memory discipline.** `Tcl_Obj`s are leaked; no refcount/shimmer model. The
  real runtime keeps its existing discipline.
- **API breadth.** Only the ~10 functions `pkga.c` needs are implemented.
- **GOT-heavy side modules.** `pkga.c` produces no `GOT.func`/`GOT.mem` imports;
  a production loader must also resolve those (data/function address fixups).
- **One interp / fixed command table** in the dynamic runtime (single-interp
  simplification).

## Provenance / licence

`ext/pkga.c` is copied verbatim from Tcl 9.0.3 (`unix/dltest/pkga.c`),
Copyright © 1995 Sun Microsystems, Inc., under the Tcl licence; its header is
retained. `include/tcl.h`, the Rust sources, and `loader.py` are original.
