# Rust-runtime C-extension spike

**Question this answers:** if the WASM runtime is rewritten in Rust, can we still
compile an *unmodified* C Tcl extension to WebAssembly and link it against our
runtime and the compiled user code, with **no per-extension shim** — defining
the ABI ourselves (API compatibility, not binary ABI compatibility)?

**Answer: yes.** This spike does it end to end.

## What it proves

It takes [`ext/pkga.c`](ext/pkga.c) — a real Tcl extension, vendored
**byte-identical** from Tcl 9.0.3 `unix/dltest/pkga.c` (the file Tcl's own
`load.test` uses) — and:

1. compiles it to a WebAssembly object with **clang**, against an authored
   [`include/tcl.h`](include/tcl.h) that we (the runtime) own, with the
   extension source unchanged;
2. links that clang object against a minimal **Rust** runtime
   ([`src/main.rs`](src/main.rs)) that implements the slice of the Tcl C API
   `pkga.c` calls, into a single `wasm32-wasip1` module;
3. runs it under **wasmtime**, where the Rust driver:
   - calls the extension's `Pkga_Init(interp)`,
   - which calls *back* into Rust (`Tcl_PkgProvide`, `Tcl_CreateObjCommand`) to
     register `pkga_eq` / `pkga_quote`,
   - then dispatches those commands through the runtime's **own command table**
     (the path compiled user code takes), which `call_indirect`s into the C
     extension's `Tcl_ObjCmdProc` through the shared function table,
   - and reads back the result `Tcl_Obj` the C code built via the Rust obj API.

Expected output: `SPIKE PASS` (exit 0), with `pkga_eq`/`pkga_quote` returning
correct results, including a multi-byte UTF-8 case (`café`) that exercises
`Tcl_NumUtfChars` / `Tcl_UtfNcmp`.

## The three roles, one module

```
   ext/pkga.c  --clang/wasm-->  pkga.o  ─┐
                                          ├─ wasm-ld ─► tcl_ext_spike.wasm ─► wasmtime
   src/main.rs --rustc/wasm--> runtime ──┘   (one shared linear memory,
   (Rust: Tcl C API + driver)                 one shared function table)
```

The Rust runtime exports the Tcl C API as `#[no_mangle] extern "C"` symbols; the
clang-compiled extension imports them; `wasm-ld` resolves everything into one
module. The driver in `main()` stands in for compiled user Tcl code calling a
command the extension registered.

This is the **static-link** model (everything in one wasm, like the existing
`core/compiler/codegen/wasm_link.py` whole-program link). It is the simplest
deployment and conclusively answers the API/ABI-compatibility question.

## Run it

```
runtime/rust-spike/run.sh
```

Requires (already present in this environment): the repo's `stable` Rust
toolchain with the `wasm32-wasip1` target, `clang`, `wasm-ld`, and `wasmtime`.
No network access and **no wasi sysroot** are needed — the extension includes
no system headers, and `build.rs` compiles it `-ffreestanding` so clang uses its
own `stddef.h`/`stdint.h`.

## Why this is the load-bearing result

- **The C compiler is clang either way.** `zig cc` (today's runtime) *is* clang
  underneath. The runtime's language does not decide whether C extensions
  compile — clang + `wasm-ld` do, and they are language-agnostic.
- **Rust expresses the whole C ABI surface.** `#[no_mangle] extern "C"`,
  `#[repr(C)]` `Tcl_Obj`, nullable `extern "C" fn` pointers, and
  shared-memory/shared-table `call_indirect` all work, as shown.
- **"API not ABI" removes the hard part.** Because extensions recompile from
  source against *our* `tcl.h`, we drop the real Tcl 600-slot stubs table and
  use direct C-ABI imports. The header is the only "shim", written once.

## What this spike does NOT yet prove (deliberate scope)

- **Dynamic loading.** Real `package require` loads an extension at runtime. The
  production path compiles the extension `-fPIC` as a `dylink.0` side module
  that imports the runtime's memory + function table, with a small loader in the
  runtime that relocates it and calls `Foo_Init`. That mechanism is
  toolchain-driven (clang/`wasm-ld`) and **independent of the runtime language**;
  it is the next spike. The static link here proves the API/ABI and the
  cross-language link; it does not exercise the side-module loader.
- **Memory discipline.** `Tcl_Obj`s are leaked; there is no refcount/shimmer
  model. The real runtime keeps its existing discipline.
- **API breadth.** Only the ~10 functions `pkga.c` needs are implemented. Per
  the extension survey (`docs/design/runtime/` write-up), a production `tcl.h`
  also needs the channel API + `Tcl_ChannelType`, custom `Tcl_ObjType`
  registration, `Tcl_FSRegister`, the threading primitives, and the NRE entry
  points, plus the sibling public headers `tclOO.h` and `tclTomMath.h`.
  `tclInt.h` (internal) is **out of scope** — in a 25-extension survey only
  TclX and Expect needed it, and both are blocked from WASM by deep POSIX
  dependencies regardless.

## Provenance / licence

`ext/pkga.c` is copied verbatim from Tcl 9.0.3 (`unix/dltest/pkga.c`),
Copyright © 1995 Sun Microsystems, Inc., distributed under the Tcl licence
(`license.terms`). Its copyright header is retained. `include/tcl.h` and
`src/main.rs` are original to this repository.
