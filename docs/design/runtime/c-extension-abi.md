# C Tcl extension → WASM ABI

The ABI by which an **unmodified** C Tcl extension — one that `#include`s
`<tcl.h>` and calls `Tcl_CreateObjCommand`, `Tcl_GetStringFromObj`, … — is
compiled to WebAssembly and linked against the runtime and the compiled user
code, with no per-extension shim.

> **What the tree exports.** This page states the whole ABI, and the tree
> implements a subset of it. The authored `tcl.h` is
> `runtime/rust/include/tcl.h`: it declares the subset each of its two hosts
> implements (§ 7), and there is no authored `tclOO.h` or `tclTomMath.h`.
> `runtime/rust/src/capi.rs` exports the runtime's half of that subset —
> command registration, object construction and copying, the scalar reads
> and the option-table lookup, lists, `TclFreeObj`, the interpreter's
> result and error state, the package call and two UTF-8 helpers, 35
> functions in all — and nothing else of the C API. The per-function ownership and
> error-path categories live in
> [`c-api-ownership-contract.md`](c-api-ownership-contract.md).

Companion docs: [`memory-management.md`](memory-management.md),
[`refcount-contract.md`](refcount-contract.md),
[`../compiler/wasm-runtime-primitives.md`](../compiler/wasm-runtime-primitives.md).

Reference Tcl sources: `tmp/tcl9.0.4/generic/tcl.h` + `tclDecls.h` (the public
API the authored header mirrors), `tmp/tcl9.0.4/unix/dltest/*.c` (the canonical
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
per extension. Only `tcl.h` exists (`runtime/rust/include/tcl.h`, § 7);
`tclOO.h` and `tclTomMath.h` do not. `runtime/rust/include/` also holds
`tcl_regex_capi.h`, the C surface of the pure-Rust ARE engine's shim
([`rust-regex-port.md`](rust-regex-port.md)), which is not part of this ABI.

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

On `wasm32` that is `{ i32, ptr, i32, ptr, 8 bytes }`, 24 bytes, 8-aligned, and
`obj.rs` asserts those offsets when it compiles. `internalRep` is C's 8-byte
`Tcl_ObjInternalRep` union; the Rust side keeps it as a raw `u64` and
reinterprets it for the `wide` / `double` variants, since core-API extensions
never touch the others. `tcl.h` declares the same struct (its fields are the
hosts' `ptrdiff_t`-sized `TclHost_Size` whatever `Tcl_Size` is) and defines
`Tcl_IncrRefCount`, `Tcl_DecrRefCount` and `Tcl_IsShared` as Tcl's own macros
over `refCount`, so a decrement that frees reaches the runtime through the
exported `TclFreeObj` and an extension's decrement of a fresh object (count 0)
frees it, as in Tcl. `TclObjType` is the same shape as C's
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

Each Tcl C API function — the header's macros apart — is a runtime export with
the C ABI (`#[no_mangle] extern "C"` in Rust). The extension imports it from the runtime's module
namespace. No stubs table is consulted. `Tcl_InitStubs` is a macro in the
header that yields the Tcl-API version string the host presents, since there is
no table to negotiate.

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
link it with the runtime's objects via `wasm-ld` into a single module. Same
model as the compiler's whole-program WASM link. Simplest deployment; proves API
compatibility, Rust↔C wasm interop, and the §4.5 callback.

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

The runtime then dispatches as in §4.6. It needs a `wasmtime`-class host loader
plus a runtime cdylib exporting memory and a growable, exported
`__indirect_function_table`; `rust/tcl-engine-wasm` is that loader (§12.1).

**Linker flags that matter.** The main module must export its table
(`--export-table`) and make it growable (`--growable-table`); the side module is
built `--experimental-pic -shared --import-memory --import-table`.

### 5.3 Which model

Model A suits "bake these extensions into one artifact" (CI images, fixed
deployments). Model B is required for true `package require` at runtime. Both
are language-agnostic; they are independent of the runtime's implementation language.

### 5.4 The libc question

`pkga.c` uses no libc, but most real extensions do (`snprintf`, `<string.h>`,
`malloc`). The answer, compatible with §4.4:

- **Compile with clang + a WASI sysroot (wasi-sdk)** — the project standard
  (what `runtime/rust/build.rs` uses for the libtommath tower). The authored
  `tcl.h` includes only `<stdarg.h>`, `<stddef.h>` and `<stdlib.h>`; an
  extension includes the rest of libc it uses, and compiles against the
  wasi-sdk sysroot. `malloc`/`free`
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

**One header, two hosts.** The authored `tcl.h`
(`runtime/rust/include/tcl.h`) is *the* C hosting contract, and it has two
hosts: the WASM leg this document specifies, and the native leg
`rust/tcl-cshim` provides — `Interp<E: Engine>`, the engine interface's second
consumer, trusted host code loaded only through `Interp::load_static` or a
host's `load` ([c-extension-shim.md](c-extension-shim.md)). One extension
source compiles for both. Each host implements the subset it can, and the
header declares for each host the functions it implements and nothing else:
`TCL_HOST_WASM` the runtime's, `TCL_HOST_NATIVE` the shim's, so a call the
compiling host does not implement is a compile error and never a call that
fails at run time. The host is the compilation target (wasm32 is the WASM leg,
anything else the native one) unless the build names one with
`-DTCL_HOST_NATIVE` or `-DTCL_HOST_WASM`; naming both declares both, which
checks a source against the header and names no host, since none implements the
union. `rust/tcl-cshim/tests/pkga_e2e.rs`'s expectations, captured against
Tcl 9.0.4's own `tcl.h`, are the shared conformance vectors.

Both legs declare what the test extension `tests/c/pkga.c` calls: command
registration (`Tcl_CreateObjCommand`, `Tcl_DeleteCommand`), object construction
and copying, the scalar reads (`Tcl_GetIntFromObj` and its siblings) and the
option-table lookup (`Tcl_GetIndexFromObjStruct`, and `Tcl_GetIndexFromObj` as
its macro), the three list calls, the interpreter's result and error state
(`Tcl_SetObjResult`, `Tcl_GetObjResult`, `Tcl_ResetResult`, `Tcl_WrongNumArgs`,
`Tcl_SetObjErrorCode`, and `Tcl_SetResult`, `Tcl_AppendResult` and
`Tcl_SetErrorCode` as inline functions over two fixed-arity exports,
`TclHost_SetResultString` and `TclHost_AppendResultString`), the package call,
the two UTF-8 helpers and `TclFreeObj`, with the `Tcl_Obj` layout of § 4.2 and
the reference-count macros over it. The WASM leg adds `Tcl_NewObj`; the native
leg adds the calls that read, write and unset a variable of the caller's frame
and evaluate a script there (`Tcl_GetVar2Ex`, `Tcl_ObjSetVar2`, `Tcl_UnsetVar2`,
`Tcl_EvalObjEx`). A source built with `-DTCL_MAJOR_VERSION=8` sees `Tcl_Size` as
`int`, as an 8.x source does, with inline wrappers for the functions that write a
size through a pointer.

`make check-c-extension-wasm` (`scripts/check_c_extension_wasm.py`, part of
`xtask-check`) holds the header to its two hosts. Offline, it reads each leg's
declarations out of the header and checks them against what the host exports, in
both directions: the `#[no_mangle]` functions of `runtime/rust/src/capi.rs`
against the WASM leg, and the `export_name` functions of
`rust/tcl-cshim/src/ffi.rs` against the native leg; a header macro such as
`Tcl_DecrRefCount` stands for an export without being declared. With wasi-sdk's
`clang` it compiles for `wasm32-wasip1`: `tests/c/layout.c`, whose static
assertions are the 24-byte `Tcl_Obj` layout; the shim's test extensions
`tests/c/pkga.c`, against the WASM leg alone and against both legs at once, each
also as an 8.x source, and `tests/c/doors.c`, against both legs at once; and the
leg a compile with no host named gets, on a wasm32 target and on one that is not.
Three of those compiles are negatives and must be refused: `doors.c` against the
WASM leg alone, which calls the frame functions only the shim implements, and
each default leg's call to a function only the other declares. Without wasi-sdk
the compiles are skipped, unless `TCL_REQUIRE_WASM_LINK` is set, as it is in the
CI job that installs the toolchain. The runtime's own test,
`runtime/rust/tests/pkga_extension.rs`, compiles `pkga.c` for the host against
the WASM leg, loads it through the runtime's exports and holds it to the
conformance vectors; two of them differ, because the runtime keeps a NUL inside
a value as one byte where C Tcl keeps it as `C0 80`, so C code that reads such a
value as a C string stops at it.

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
  It belongs in the runtime itself.

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

## 10. Open questions / production work

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
  `Tcl_Filesystem` / `Tcl_ObjType` bodies.
- **Nominal stub tables** for introspecting extensions (§6).
- **Safe interpreters, multiple interpreters, `unload`.** How extension state
  and command tables map onto child interps.
- **Threads in WASM.** The threading API maps onto wasm threads or a
  cooperative shim; decide per deployment.
The durable artefact is *this ABI plus the headers*, which is reusable
whichever language the runtime is written in.

## 11. The extension corpus this ABI is held to

The reference corpus is the nine in-tree Tcl 9.0.4 dltest extensions
(`pkga`–`pkge`, `pkgt`, `pkgua`, `pkgπ`, `pkgooa`) plus two synthetic probes.
Between them they exercise every part of the surface that is easy to
under-specify:

- `pkgua` — the hash-table API, thread-local data, the load/unload protocol,
  `Tcl_SetVar2`, `Tcl_DeleteCommandFromToken`;
- `pkgπ` — non-ASCII init-function naming;
- `pkgooa` — stubs introspection, and with it the only GOT-relocation pattern
  in the corpus (§10);
- the synthetic probes — static `Tcl_ObjType` / `Tcl_ChannelType` /
  `Tcl_Filesystem` tables of function pointers.

`embtest.c` is deliberately excluded: it *embeds* Tcl (`main()` +
`Tcl_FindExecutable`), which is the opposite of extending it.

## 12. The seam, against the real runtime

The seam in §4.6 — a Tcl script compiled by `tcl_compiler::codegen::wasm`
calling an **extension-registered** command and dispatching into that
extension — runs against the real runtime in
`a_compiled_script_calls_an_extension_registered_command`
(`rust/tcl-compiler/tests/wasm_real_link.rs`):
an extension module shares the runtime's memory and function table, installs its
`Tcl_ObjCmdProc` in the table and registers it from `Foo_Init` through the
runtime's `Tcl_CreateObjCommand` export, and a compiled `foo` then reaches it
with the `clientData` it was registered with and the completion code it answers.
The same compiled module, run before `Foo_Init`, fails with `invalid command name
"foo"`, so what finds `foo` afterwards is the registration and not anything the
compiled code carries.

Compiled code reaches an arbitrary runtime command through the live command
table: `tcl_invoke_argv` (`codegen_abi.rs`) takes a prebuilt argv from generated
code and routes it through the same `Interp::dispatch` interpreted Tcl uses, so
namespaces, `unknown`, aliases, ensembles, and TclOO all resolve identically, and
a compiled script reaches any command the table holds without the lookup needing
an addition. The registration side is `Tcl_CreateObjCommand`
(`runtime/rust/src/capi.rs`), which binds the name — in the current namespace, or
the one a qualified name names — to a `Command::ObjCmd` (`interp.rs`). That holds
the procedure (an index into the shared function table under `wasm32`, an ordinary
function pointer natively), its `clientData` and its delete procedure behind an
`Rc`; dispatch calls the procedure with the call's words as `objv` and takes the
completion code it answers, the result being what it left through
`Tcl_SetObjResult`. The delete procedure runs when the command's last handle
drops: at the deletion, a replacement or a `rename` to the empty name for an idle
command, and when the call returns for one that deletes itself, so its
`clientData` stays live for as long as its own procedure runs (C Tcl runs it at
the deletion itself). `Tcl_DeleteCommand` is `rename name {}`.
`runtime/rust/tests/extension_commands.rs` holds this natively with extensions
written against the C ABI in Rust.

### 12.1 Model B, hosted

`rust/tcl-engine-wasm` is the §5.2 loader, and the runtime compiled to
`wasm32-wasip1` is an engine of the extension interface under it (`WasmEngine`).
An extension built for the runtime as a side module — `clang
--target=wasm32-wasip1 -fPIC -DTCL_HOST_WASM` against `runtime/rust/include/tcl.h`,
then `wasm-ld --experimental-pic -shared --no-entry --import-memory
--import-table`, with wasi-libc's `libc.a` for what it uses of the C library — is
loaded as `load` would load it. The host reads `dylink.0`'s `MEM_INFO` by hand,
reserves the module's data in the runtime's heap (`tcl_codegen_call_frame_alloc`),
grows the runtime's exported table for its functions, gives it a 64 KiB stack of
its own, resolves each import of the C API (`env.Tcl_*`, and the `env.TclHost_*`
and `env.TclFreeObj` the header's macros and inline functions call) to the
runtime's export, applies the relocations and the constructors, and calls
`PREFIX_Init` with the interpreter. An import of any other runtime export is
refused, so an extension reaches the interpreter only through the C API, which
has no eval or variable door. A module that names libraries to load first
(`NEEDED`), imports a `GOT.*` entry, calls a function the runtime does not
export, or defines no `PREFIX_Init` is refused, the refusal naming it.

The host drives the interpreter through the runtime's `tcl_engine_*` exports
(`runtime/rust/src/engine_abi.rs`), which do across the module boundary what the
native engine does in process: the command and value-size limits, the
confinement, the whitelist, the pinned release, a unit's procedure, a package
provided, and a host command's `return` and error. What the interpreter's counts
cannot see is the host's to bound. Fuel stands in for the command count in a C
command's own loop and in a loop that dispatches nothing; the epoch keeps the
wall clock, since every WASI clock reads zero; and the memory's growth is capped
by the value-size budget. An evaluation gets `FUEL_PER_COMMAND` (2,000,000
instructions) for each command its budget allows and one more, beside
`FIRST_USE_FUEL` (2,000,000,000) for what a fresh instance builds the first
time it is used, granted once per instance — on the debugging build the tests
run, a command costs about 170,000 instructions and the first ensemble command
an instance dispatches about 700 million, building the release's command
tables; the epoch advances every 5 ms; memory may grow four bytes for each
byte of the value-size budget, and at least 32 MiB, before the growth traps;
and the limits are lifted when an evaluation ends, so what is set up between
evaluations runs under none. In the interpreter itself the command count is
charged at the dispatch boundary every command crosses, the wall clock is read
there every 64 dispatches and at the loop commands' poll every 4096
iterations, and the value size is charged in `string repeat`; a limit once
outrun stays outrun until the next evaluation begins, so a body that catches
the error cannot run on. Every WASI function either module imports is a stub
that answers the same on every run: the clock reads zero, randomness is zeros,
there is no environment, no argument and no preopened directory, output is
swallowed, and `proc_exit` ends the evaluation. So nothing of the machine reaches
an answer. A trap leaves an instance unusable, and the engine rebuilds it from
what was set up on it.

`rust/tcl-engine-wasm/tests/under_wasm.rs` builds `pkga.c` this way, holds it
under fuel to the vectors `tclsh9.0` answered (`rust/tcl-cshim/tests/vectors/pkga.rs`),
and runs the cases the native runtime engine is held to
(`runtime/rust/tests/common/engine_cases.rs`) on the WASM engine. The registry's
extension seam (`tcl_registry::extension_host`) is what an analysis binds — a
pack's `evaluate -implementation ID -host wasm_extension { extension FILE
PREFIX … }` — and a thread with no
host installed declines every evaluation as `Transient`, so the language
server, which never links wasmtime, declines.
