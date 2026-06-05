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

**Answer: yes — proven three ways**, against real Tcl extensions vendored
byte-identical from Tcl 9.0.3 `unix/dltest/` (the files Tcl's own `load.test`
uses), compiled with `zig cc` against an authored `tcl.h` we own, and run
against a **Rust** runtime.

## Layout

```
runtime/rust-spike/
├── include/             authored, API-compatible headers (shared by all spikes)
│   ├── tcl.h            core + widened public surface (channels, ObjType, FS,
│   │                    threading, NRE, bignum-obj, Tcl 9 Tcl_*ObjCmd2)
│   ├── tclOO.h          TclOO C API subset
│   └── tclTomMath.h     mp_* bignum API subset
├── ext/                 the extensions (real dltest samples + 2 synthetic probes)
│   ├── pkga.c pkgb.c pkgt.c pkgooa.c   unmodified, byte-identical to Tcl 9.0.3
│   └── synth_surface.c synth_tommath.c synthetic compile-probes (never run)
├── compile-check/       SPIKE 3: every ext/*.c compiles unmodified vs include/
│   └── check.sh
├── static-link/         SPIKE 1: extension + runtime linked into ONE wasm
│   ├── src/main.rs      minimal Rust runtime (Tcl C API) + driver
│   ├── build.rs         zig-cc-compiles ext/pkga.c, hands object to rustc
│   └── run.sh
└── dynamic-link/        SPIKE 2: extension loaded at runtime as a side module
    ├── runtime/src/lib.rs  Rust runtime cdylib (exports memory + table + API)
    ├── loader.py           host loader (dylink.0 + memory/table base wiring)
    └── run.sh
```

## Spike 1 — static link (`static-link/run.sh`)

Everything in one module: `zig cc` compiles `ext/pkga.c` to a wasm object
against our `tcl.h`; `rustc`/`wasm-ld` links it with the Rust runtime into one
`wasm32-wasip1` binary; `wasmtime` runs it. This is the whole-program link model
(like `core/compiler/codegen/wasm_link.py`). Proves API compatibility, Rust↔C
wasm interop, and the runtime→extension `call_indirect` callback.

## Spike 2 — dynamic link (`dynamic-link/run.sh`)

The real `package require` model. `ext/pkga.c` is built `-fPIC -shared` as a
separate `dylink.0` **side module**. A host loader parses its `dylink.0`
footprint, reserves a memory region (`__memory_base`) and table slots
(`__table_base`) from the **runtime's** shared linear memory and shared function
table, wires the side module's `Tcl_*` imports to the runtime's exports, runs
`__wasm_apply_data_relocs` + `__wasm_call_ctors` + `Pkga_Init`, then dispatches
commands through the runtime, which `call_indirect`s back into the
dynamically-loaded extension via the shared table. Proves cross-module shared
memory + shared function table + host-driven relocation — none of which depend
on the runtime being Rust vs Zig (clang/`wasm-ld` do the C work).

## Spike 3 — compile-check (`compile-check/check.sh`)

API-compatibility breadth: compiles **every** extension in `ext/` unmodified to
a wasm object against `include/`, proving the authored headers are API-complete
for real-world extension source. Covers:

| Extension | Exercises |
|---|---|
| `pkga.c` (real) | command/obj/result/UTF core |
| `pkgb.c` (real) | `Tcl_GetIntFromObj`, `Tcl_GetWideIntFromObj`, `Tcl_GetErrorLine`, `Tcl_AppendResult`, `Tcl_EvalEx`, `snprintf` |
| `pkgt.c` (real) | Tcl 9 `Tcl_CreateObjCommand2` / `Tcl_CreateObjTrace2` (`Tcl_Size` arity) |
| `pkgooa.c` (real) | `tclOO.h` + stub-table introspection (compile-only; see note) |
| `synth_surface.c` | custom `Tcl_ObjType`, channels, `Tcl_FSRegister`, threading, NRE |
| `synth_tommath.c` | `tclTomMath.h` `mp_*` + `Tcl_NewBignumObj` |

`pkgooa.c` deliberately introspects the *binary stub table* — a concept our
direct-linking ABI does not use. We provide the `TclOOStubs` shape so its source
compiles; the address comparison it performs is not meaningful under our ABI.
This is the one real-world pattern that wants a *nominal* stub table (populated
with our functions) for source compatibility — noted in the design doc.

## Run

```
runtime/rust-spike/compile-check/check.sh     # -> COMPILE-CHECK PASS (6/6)
runtime/rust-spike/static-link/run.sh         # -> SPIKE PASS
runtime/rust-spike/dynamic-link/run.sh        # -> SPIKE PASS
```

Requirements (present here): the repo's `stable` Rust toolchain with
`wasm32-wasip1` + `wasm32-unknown-unknown`, `zig` (used as the C compiler —
`zig cc` — bundling wasi-libc), `wasm-ld`, `wasmtime`, and `uv` (the dynamic
loader runs via `uv run --with wasmtime`).

## Toolchain note — the Rust + `zig cc` hybrid

The C is compiled by **`zig cc`** (clang + bundled wasi-libc), so our `tcl.h`
can `#include <stdio.h>`/`<string.h>` exactly like the real header and
extensions that use `snprintf`/`<string.h>` compile with **no separate wasi
sysroot**. The runtime is Rust. This is the recommended hybrid: the runtime's
language (Rust) and the extension compiler (`zig cc`) are independent choices,
and `zig cc` is the lowest-friction way to get a hermetic C→wasm cross-compiler.
(Plain `clang` works too, but then libc-using extensions need a wasi sysroot.)

## Why the runtime's language is not the gate

- The C compiler is clang either way (`zig cc` is clang underneath). It, not the
  runtime language, decides whether C extensions compile to WASM.
- Rust expresses the whole C ABI surface: `#[no_mangle] extern "C"`,
  `#[repr(C)] Tcl_Obj`, nullable `extern "C" fn` pointers, exported memory +
  function table, and shared-table `call_indirect`.
- **"API not ABI" removes the hard part**: extensions recompile against *our*
  `tcl.h`, so we drop Tcl's 600-slot binary stubs table and use direct C-ABI
  imports. The headers are the only "shim", written once.

## What a production `tcl.h` needs (from the extension survey)

A survey of 25+ real extensions found **~85–90% are public-`tcl.h`-only** at the
Tcl level. The headers here now declare the surface those need; the rows marked
*validated* are compile-exercised by `compile-check`:

| Surface | Why | In header | Validated |
|---|---|---|---|
| obj / command / result / eval / UTF | universal | `tcl.h` | ✅ pkga/pkgb/pkgt |
| int/wide/double/bool accessors, `Tcl_AppendResult`, `Tcl_EvalEx` | universal | `tcl.h` | ✅ pkgb |
| Tcl 9 `Tcl_*ObjCmd2` (`Tcl_Size` arity) | modern extensions | `tcl.h` | ✅ pkgt |
| channel API + `Tcl_ChannelType` | TclTLS, Memchan | `tcl.h` | ✅ synth_surface |
| custom `Tcl_ObjType` registration | VecTcl, sha1c/md5c | `tcl.h` | ✅ synth_surface |
| `Tcl_FSRegister` / `Tcl_Filesystem` | tclvfs | `tcl.h` | ✅ synth_surface |
| threading (`Tcl_CreateThread`, mutex/cond, thread-data) | Thread | `tcl.h` | ✅ synth_surface |
| NRE entry points (`Tcl_NRCreateCommand`) | coroutine-aware ext | `tcl.h` | ✅ synth_surface |
| bignum objects (`Tcl_NewBignumObj`) + `mp_*` | bignum ext | `tcl.h` + `tclTomMath.h` | ✅ synth_tommath |
| TclOO C API | classes-in-C | `tclOO.h` | ✅ pkgooa (compile) |

`tclInt.h` (internal) is **out of scope**: in the survey only TclX and Expect
needed it, and both are blocked from WASM by deep POSIX dependencies (ptys,
chroot) regardless. The practical gate for any extension is almost never the Tcl
API — it is whether its **third-party native library** (OpenSSL, libcurl, libpq,
X11, audio, libtcc) can itself reach WASM.

## What these spikes deliberately do NOT cover

- **Execution breadth.** Only `pkga` is *run* (static + dynamic). The wider
  surface is compile-validated, not executed; the runtime implements only the
  API slice `pkga` needs to run.
- **Memory discipline.** `Tcl_Obj`s are leaked; no refcount/shimmer model.
- **Faithful struct bodies.** `Tcl_ChannelType` / `Tcl_Filesystem` /
  `Tcl_ObjType` carry only the fields the probes touch (designated-initialiser
  usage); a production header mirrors the full versioned structs.
- **GOT-heavy side modules.** `pkga.c` produces no `GOT.func`/`GOT.mem` imports;
  a production loader must also resolve those.
- **One interp / fixed command table** in the dynamic runtime.

## Provenance / licence

`ext/pkga.c`, `pkgb.c`, `pkgt.c`, `pkgooa.c` are copied verbatim from Tcl 9.0.3
(`unix/dltest/`), Copyright © Sun Microsystems, Inc., under the Tcl licence;
their headers are retained. The `synth_*.c` probes, the `include/` headers, the
Rust sources, and `loader.py` are original.
