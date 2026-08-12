# C Tcl extension → WASM ABI

The ABI by which an **unmodified** C Tcl extension — one that `#include`s
`<tcl.h>` and calls `Tcl_CreateObjCommand`, `Tcl_GetStringFromObj`, … — is
compiled to WebAssembly and linked against the runtime and the compiled user
code, with no per-extension shim.

> **Implementation state.** This is a *design contract*, and the surface it
> describes is not shipped: the repository contains no authored `tcl.h` /
> `tclOO.h` / `tclTomMath.h`, and `runtime/rust/src/capi.rs` exports only a
> small subset of the C-API. The mechanism (static link, dynamic side-module
> load, a six-extension compile-check) was proven end to end by throwaway
> spikes that have since been removed, so **do not derive the shape from spike
> code** — derive it from this document. The per-function ownership and
> error-path categories live in
> [`c-api-ownership-contract.md`](c-api-ownership-contract.md).

Companion docs: [`memory-management.md`](memory-management.md),
[`refcount-contract.md`](refcount-contract.md),
[`../compiler/wasm-runtime-primitives.md`](../compiler/wasm-runtime-primitives.md).

Reference Tcl sources: `tmp/tcl9.0.3/generic/tcl.h` + `tclDecls.h` (the public
API the authored header mirrors), `tmp/tcl9.0.3/unix/dltest/*.c` (the canonical
minimal extensions), and the WebAssembly
[dynamic linking convention](https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md)
(`dylink.0`).

## 1. Goal and non-goals

**Goal.** Let users compile an *unmodified* C Tcl extension (one that
`#include`s `<tcl.h>` and calls `Tcl_CreateObjCommand`, `Tcl_GetStringFromObj`,
…) to WebAssembly and link it against our runtime and the compiled user code,
with **no per-extension shim**.

**In scope.**

- An authored, API-compatible `tcl.h` (+ sibling `tclOO.h`, `tclTomMath.h`) the
  runtime owns and ships.
- A WASM-level ABI we define ourselves for how those calls are wired.
- Two link models: whole-program static link, and dynamic load at
  `package require` time.

**Non-goals.**

- **Binary ABI compatibility.** We do *not* load prebuilt `.so`/`.dll`
  extensions, and we do *not* replicate Tcl's 600-slot binary stubs-table
  layout. Extensions are recompiled from source. This is the key simplification
  (see §3).
- **`tclInt.h` (internal API).** Out of scope — see §7.
- **Bringing arbitrary third-party native libraries to WASM.** That is the real
  per-extension gate (OpenSSL, libcurl, X11, …) and is orthogonal to the Tcl
  API.

## 2. The thesis: API compatibility, not ABI compatibility

A C Tcl extension depends on Tcl at two levels:

1. **Source/API level** — the function signatures, types, and macros in
   `tcl.h`. Satisfying this means the extension *compiles*.
2. **Binary/ABI level** — the exact `tclStubsPtr` table layout, struct offsets,
   and calling conventions of a specific Tcl build. Satisfying this means a
   *prebuilt binary* loads unchanged.

We target **(1) only**. Because every extension is recompiled against the
`tcl.h` we author, we are free to choose the ABI behind each call. We choose
**direct C-ABI imports** instead of stubs-table indirection: `tcl.h` declares
`Tcl_CreateObjCommand` as a plain `extern` function, the extension's WASM
imports it, and the runtime exports it. This removes the single largest piece
of Tcl-ABI machinery and is invisible to extension *source*.

## 3. Architecture: one shared linear memory, host-wired modules

The runtime is the **main module**; compiled user code and C extensions are
**side modules** that share the runtime's linear memory and function table.
This is the same topology the compiled-script pipeline already uses
(`wasm-runtime-primitives.md`): the script module imports `tcl.memory` and the
`tcl_*` primitives. Extensions slot in as additional side modules.

```
                     ┌──────────────────────── runtime.wasm (Rust) ───────────┐
                     │  owns: memory, __indirect_function_table, allocator     │
                     │  exports: Tcl_* (the C API), tcl_* (codegen primitives) │
                     └───────▲───────────────────────▲─────────────────────────┘
   imports tcl.* + memory    │                       │   imports Tcl_* + memory + table
   ┌─────────────────────────┴──────┐      ┌─────────┴───────────────────────────┐
   │  user_code.wasm (our compiler)  │      │  extension.wasm (clang from C)       │
   │  exports ::top, procs           │      │  exports Foo_Init, command procs     │
   └─────────────────────────────────┘      └──────────────────────────────────────┘
```

All `Tcl_Obj *`, `Tcl_Interp *`, and `char *` cross the boundary as raw
addresses into the **one** shared linear memory. There is exactly **one
allocator**, owned by the runtime; extensions never carry their own heap (see
§5.4).

## 4. The ABI contract

### 4.1 Headers

The runtime ships authored headers that are signature-faithful subsets of real
Tcl 9.0:

- `tcl.h` — the public surface (§7).
- `tclOO.h` — the object-system C API.
- `tclTomMath.h` — the `mp_*` bignum API.

These are the *only* shim. They are written once, by the runtime author, not
per extension. None of the three exists yet: `runtime/rust/include/` currently
holds one header, `tcl_regex_capi.h`, which is the §10 regex shim's own
C surface and not part of this ABI.

### 4.2 `Tcl_Obj` layout

`Tcl_Obj` is `#[repr(C)]` with the exact field order `tcl.h` declares
(`refCount`, `bytes`, `length`, `typePtr`, `internalRep`), because extensions
read `objPtr->refCount` / `objPtr->bytes` directly through macros. This half
*is* shipped: `runtime/rust/src/obj.rs` declares

```rust
#[repr(C)]
pub struct TclObj {
    pub ref_count: TclSize,          // Tcl_Size — ptrdiff_t
    pub bytes: *mut c_char,
    pub length: TclSize,
    pub type_ptr: *const TclObjType,
    pub internal_rep: u64,
}
```

On `wasm32` that is `{ i32, ptr, i32, ptr, 8 bytes }`, 8-aligned. `internalRep`
is C's 8-byte `Tcl_ObjInternalRep` union; the Rust side keeps it as a raw `u64`
and reinterprets it for the `wide` / `double` variants, since core-API
extensions never touch the others. `TclObjType` is the same shape as C's
registered type descriptor, with the four `free`/`dup`/`updateString`/
`setFromAny` procs typed to match `tcl.h`, so an extension's own `Tcl_ObjType`
slots in unchanged.

`bytes` is the object's UTF-8 **string representation**. A byte-array object
instead stores its exact raw payload in an `internalRep` allocation with the
`bytearray` type pointer. Its `bytes` field stays unset until a string consumer
needs it. At that point the runtime creates the C Tcl string view: byte `0xNN`
becomes Unicode U+00NN. The raw internal representation remains intact, so
`binary` and `zlib` can still consume the original payload after a read-only
string operation.

This dual-port model matters after a string-changing command. The result is an
ordinary Unicode string, not a byte array. The central `TclVersion` byte policy
then controls a later binary conversion: Tcl 8.x truncates a code point to its
low byte, while Tcl 9 rejects a code point above U+00FF with `TCL VALUE BYTES`.
`binary format`, `binary decode`, `binary scan`, and the `zlib` byte-producing
commands all construct the typed representation. See the
[byte-array KCS note](../../kcs/kcs-qa-how-does-the-wasm-runtime-preserve-byte-arrays.md)
for a worked example and user-facing limits.

### 4.3 Calls: direct imports

Each Tcl C API function is a runtime export with the C ABI (`#[no_mangle]
extern "C"` in Rust). The extension imports it from the runtime's module
namespace. No stubs table is consulted. `Tcl_InitStubs` is a nominal success
check (returns the runtime's Tcl-API version string), since there is no table
to negotiate.

### 4.4 Allocation

The runtime owns the allocator. Extensions obtain memory through the Tcl API
(`Tcl_Alloc` / `Tcl_NewObj` / `Tcl_NewStringObj` / …), never through a private
libc heap, so there is one coherent view of the shared memory. `Tcl_Free`
returns to the same allocator.

### 4.5 Function pointers are shared-table indices

`Tcl_CreateObjCommand(interp, "foo", FooCmd, …)` passes `FooCmd` as an index
into the shared `__indirect_function_table`. The runtime stores the index in
its command table. To invoke `foo`, the runtime `call_indirect`s that index —
which lands in the extension's function because both modules share the one
table. This is the crux of the runtime→extension callback and is why the table
must be shared, not per-module.

### 4.6 Command dispatch flow

```
user code: `foo a b`
   │  (compiled code can't inline a command that only exists at load time)
   ▼
runtime command-table lookup "foo"  ──►  CmdEntry{ proc_idx, clientData }
   ▼
call_indirect proc_idx (shared table)  ──►  extension's Tcl_ObjCmdProc
   ▼
extension builds result via Tcl_SetObjResult(interp, Tcl_New*Obj(...))
   ▼
runtime reads interp->result
```

Extension-registered commands always go through the runtime's dynamic dispatch
(never inlined by the compiler), which is the correct and only place a
load-time-registered command can be resolved.

## 5. Link models

### 5.1 Model A — whole-program static link

Compile the extension `.c` to a WASM object with clang + wasi-sdk;
link it with the runtime's objects via `wasm-ld` into a single module. Same model as
the compiler's whole-program WASM link. Simplest deployment;
proves API compatibility, Rust↔C wasm interop, and the §4.5 callback. This was
spike-validated (spike since removed).

### 5.2 Model B — dynamic side-module load (`package require`)

The extension is compiled `-fPIC` and linked `wasm-ld --experimental-pic
-shared --no-entry --import-memory --import-table` into a `dylink.0` **side
module**. A loader in the runtime/host loads it at runtime:

1. Parse the side module's `dylink.0` `MEM_INFO`: data size + alignment, table
   size + alignment. (For `pkga.c`: 55 data bytes, 2 table slots, no GOT.)
2. Reserve a memory region from the runtime's allocator → `__memory_base`.
3. Reserve table slots: grow the shared `__indirect_function_table` by
   `tablesize`; the old size is `__table_base`.
4. Provide a dedicated C stack region (`__stack_pointer`) disjoint from the
   runtime's.
5. Resolve imports: `memory`, `__indirect_function_table`, the base globals,
   and each `Tcl_*` to the runtime's export. Resolve any `GOT.mem.*` /
   `GOT.func.*` (data/function address fixups) — `pkga.c` has none, but a
   production loader must handle them.
6. Instantiate; run `__wasm_apply_data_relocs` then `__wasm_call_ctors`.
7. Call `Foo_Init(interp)`. Its `Tcl_CreateObjCommand` calls register command
   procs (now resident in the shared table at `__table_base + k`).

The runtime then dispatches as in §4.6. This was spike-validated (spike since
removed) with a `wasmtime` host loader plus a Rust cdylib runtime exporting
memory and a growable, exported `__indirect_function_table`.

**Linker flags that matter.** The main module must export its table
(`--export-table`) and make it growable (`--growable-table`); the side module is
built `--experimental-pic -shared --import-memory --import-table`.

### 5.3 Which model

Model A suits "bake these extensions into one artifact" (CI images, fixed
deployments). Model B is required for true `package require` at runtime. Both
are language-agnostic; they are independent of the runtime's implementation language.

### 5.4 The libc question

`pkga.c` uses no libc, but most real extensions do (`snprintf`, `<string.h>`,
`malloc`). Two coherent options, both compatible with §4.4:

- **Compile with clang + a WASI sysroot (wasi-sdk)** — the project standard
  (what `runtime/rust/build.rs` uses for the libtommath tower). The authored
  `tcl.h` `#include`s `<stdio.h>`/`<string.h>` like the real header, and
  libc-using extensions compile against the wasi-sdk sysroot. `malloc`/`free`
  used internally by an extension resolve to wasi-libc; for memory that crosses
  the boundary, the extension must use `Tcl_Alloc` (which is the runtime's
  allocator) — this is already the Tcl convention.

A production runtime should additionally route `Tcl_Alloc`/`ckalloc` to its own
allocator so all boundary-crossing memory is single-owner.

## 6. The stub-table introspection nuance

A minority of extensions read the stub table as *data* rather than just calling
through it. The canonical example is `pkgooa.c`, which compares
`Tcl_CopyObjectInstance == tclOOStubsPtr->tcl_CopyObjectInstance`. Under our
direct-ABI model there is no live stub table, so:

- We ship the `TclOOStubs` / `TclStubs` struct *shapes* so such source
  *compiles* (validated: `pkgooa.c` compiles in the compile-check).
- For such an extension to *behave*, the runtime should expose a **nominal**
  stub table — a real struct populated with our function pointers — even though
  ordinary calls do not route through it. This is the one place our "no binary
  ABI" stance needs a small concession, and it is a fixed, write-once cost, not
  per-extension.

## 7. Header scope

Source of truth for "what the API surface must cover": the 25-extension survey.
**~85–90% of real extensions are public-`tcl.h`-only.**

`tcl.h` must cover, in priority order:

| Surface | Driven by |
|---|---|
| obj / command / result / eval / UTF core | universal |
| scalar accessors, `Tcl_AppendResult`, `Tcl_EvalEx`/`EvalObjv` | universal |
| Tcl 9 `Tcl_*ObjCmd2` (`Tcl_Size` arity) | modern extensions |
| channel API + `Tcl_ChannelType` | TclTLS, Memchan, Trf |
| custom `Tcl_ObjType` registration | VecTcl, tcllib sha1c/md5c |
| `Tcl_FSRegister` / `Tcl_Filesystem` | tclvfs |
| threading (`Tcl_CreateThread`, mutex/cond, thread-data) | Thread |
| NRE entry points (`Tcl_NRCreateCommand`/`Tcl_NRCallObjProc`) | coroutine-aware |
| bignum objects (`Tcl_NewBignumObj`) | bignum extensions |

Sibling **public** headers: `tclOO.h` (classes-in-C) and `tclTomMath.h` (raw
`mp_*` arithmetic). All of the above are public Tcl API — channels,
`Tcl_ObjType`, VFS, threading, and NRE do **not** require internal headers.

**Out of scope: `tclInt.h`.** In the survey only TclX and Expect reach into it,
and both are independently blocked from WASM by deep POSIX dependencies (ptys,
`chroot`). The practical gate for any extension is its **third-party native
library**, never the Tcl API.

## 8. Toolchain

- **C compiler:** clang + a WASI sysroot (wasi-sdk) is the project standard —
  a hermetic C→wasm cross-compiler with libc, independent of the runtime's
  language (`runtime/rust/build.rs` uses it to build the libtommath tower).
  Because the C toolchain is external to the runtime, the runtime's language
  never gates extension compilation.
- **Linker:** `wasm-ld`. Main module: `--export-table --growable-table`
  (+ exported `memory`). Side module: `--experimental-pic -shared --no-entry
  --import-memory --import-table`.
- **Host/loader (Model B):** parses `dylink.0`, allocates bases, wires imports.
  The spike loader is Python + `wasmtime`; production would put the loader in
  the runtime itself.

## 9. Runtime-language analysis (Rust)

The mechanism is language-agnostic; the runtime is Rust. What the language
provides for the ABI surface:

| Concern | Rust |
|---|---|
| Export C ABI symbols | `#[no_mangle] extern "C"` |
| `Tcl_Obj` layout | `#[repr(C)]` |
| Consume `tcl.h` for self-consistency | `bindgen` (build step) |
| Compile the extension's C | external clang + wasi-sdk |
| Safety in the obj/memory layer | partial — raw-pointer `unsafe` over shared memory |

Net: Rust is fully **capable**. Its safety benefit is real for the pure-logic
halves and partial in the `Tcl_Obj`/shared-memory layer, which is inherently
`unsafe`.

## 10. The regex engine is the first C library

> **Superseded.** This section argued for keeping the C Henry-Spencer engine as
> the runtime's first vendored C library. That is no longer the design: the ARE
> engine is now the pure-Rust `tcl-regex` crate (bit-for-bit fidelity —
> backreferences, lookahead, POSIX leftmost-longest — verified against
> `reg.test`), with **no C compiled, vendored, or fetched**. The direction is
> reversed: rather than C being linked into the runtime, C consumers link the
> *Rust* engine through the `regex_capi` C-ABI shim (`TclReComp`/`TclReExec`/…).
> The rationale below is retained as historical context.

Tcl's Henry Spencer ARE engine is already C, already compiled into the runtime
(the vendored `tcl-regex` engine). Once "compile C against the runtime" exists,
the regex engine *is* the first such C library — keeping it gives bit-for-bit
ARE fidelity (backreferences + lookahead + POSIX leftmost-longest, which no
pure-Rust crate matches) in the Rust runtime. Its small locale shim is
a *runtime-internal* C component, not a user extension, so it is exempt from the
"no per-extension shim" rule.

## 11. Open questions / production work

- **GOT relocations are narrowly scoped (measured).** Linked as `-shared` side
  modules, `pkga`/`pkgb`/`pkgt` and even `synth_surface` (static `Tcl_ObjType` /
  `Tcl_ChannelType` / `Tcl_Filesystem` tables of function pointers) emit **zero**
  GOT entries — their imports are exactly `memory`, the shared table, the three
  PIC base globals (`__memory_base`, `__table_base`, `__stack_pointer`), and the
  `Tcl_*` functions. GOT entries appear **only** when an extension takes the
  *address of a runtime-exported symbol*: `pkgooa.c` (stubs introspection) emits
  4 — `GOT.mem.{tclStubsPtr, tclOOStubsPtr, tclOOIntStubsPtr}` and
  `GOT.func.Tcl_CopyObjectInstance`. Resolution is mechanical: `GOT.mem.X` → the
  runtime's linear-memory address of data symbol `X`; `GOT.func.X` → a
  shared-table index for function `X`. So the loader's GOT path is small and
  tied to the (rare) stubs-introspection / address-of-runtime-symbol pattern,
  not to extension size — it is **not** a blocker, just a finite list to wire.
- **Refcount ownership across the boundary.** The caller/callee `+1` rules per
  entry point are
  [`c-api-ownership-contract.md`](c-api-ownership-contract.md); what remains is
  encoding those categories in the `runtime/rust/` implementations and gating
  on them.
- **Faithful struct fidelity.** Ship the full versioned `Tcl_ChannelType` /
  `Tcl_Filesystem` / `Tcl_ObjType` bodies (the spike carries only the fields the
  probes touch).
- **Nominal stub tables** for introspecting extensions (§6).
- **Safe interpreters, multiple interpreters, `unload`.** How extension state
  and command tables map onto child interps.
- **Threads in WASM.** The threading API maps onto wasm threads or a
  cooperative shim; decide per deployment.
The durable artefact is *this ABI plus the headers*, which is reusable
whichever language the runtime is written in.

## 12. The extension corpus this ABI is held to

The reference corpus is the nine in-tree Tcl 9.0.3 dltest extensions
(`pkga`–`pkge`, `pkgt`, `pkgua`, `pkgπ`, `pkgooa`) plus two synthetic probes.
Between them they exercise every part of the surface that is easy to
under-specify:

- `pkgua` — the hash-table API, thread-local data, the load/unload protocol,
  `Tcl_SetVar2`, `Tcl_DeleteCommandFromToken`;
- `pkgπ` — non-ASCII init-function naming;
- `pkgooa` — stubs introspection, and with it the only GOT-relocation pattern
  in the corpus (§11);
- the synthetic probes — static `Tcl_ObjType` / `Tcl_ChannelType` /
  `Tcl_Filesystem` tables of function pointers.

`embtest.c` is deliberately excluded: it *embeds* Tcl (`main()` +
`Tcl_FindExecutable`), which is the opposite of extending it.

## 13. The unproven seam

One seam in §4.6 has never been exercised against the real product: a Tcl
script compiled by `tcl_compiler::codegen::wasm` calling an
**extension-registered** command and dispatching into that extension. Every
demonstration so far used a hand-written driver as the stand-in for compiled
user code.

Half of what it needs is now in place. Compiled code reaching an arbitrary
runtime command through the live command table is shipped: `tcl_invoke_argv`
(`codegen_abi.rs`) takes a prebuilt argv from generated code and routes it
through the same `Interp::dispatch` interpreted Tcl uses, so namespaces,
`unknown`, aliases, ensembles, and TclOO all resolve identically. A compiled
script therefore already reaches any command the table holds, without the
lookup needing an addition.

What is missing is the registration side: there is no `Tcl_CreateObjCommand`
export and no `Command` variant holding a shared-table function index, so
nothing can put an extension's `Tcl_ObjCmdProc` into that table for
`tcl_invoke_argv` to find. Proving the seam means adding both, then having
`Foo_Init` register `foo` and a compiled script call it.
