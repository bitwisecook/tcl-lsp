# Runtime refcount contract

Every C-ABI-exported runtime function lives in one of a handful of
ownership categories. This doc fixes the categories and lists each
export's row so callers (compile-side codegen, the test harness,
other runtime modules) can reason about lifetime without reading
the implementation. The runtime is the Rust crate `tcl-runtime`
(`runtime/rust/`).

> **Coverage.** The categories and conventions below are complete and binding.
> The per-subsystem rows are maintained **by hand**: there is no automated gate
> that flags an export with no row, so a new export needs its row added in the
> same change. Where a subsystem's rows are not yet written out, the section
> says so — an absent row is a gap in this document, never permission to
> invent a convention at the call site.

## Categories

For each C-ABI export (`#[no_mangle] extern "C"`) — and each
internal entry point that takes or returns a TclObj reference — we
record:

- **Args** — what the function expects of each input handle's
  refcount when the caller calls it:
  - `borrowed` — caller still owns the +1 they had; the function
    must not release it. The function may temporarily retain
    while the call is in flight but must release before
    returning.
  - `consumed` — caller transferred their +1 to the function. The
    function takes ownership and is responsible for either
    storing the handle (taking the +1 into a slot) or releasing
    it.
  - `adopted` — the caller passes a **fresh `rc 0`** object it never
    retained; the function takes responsibility for freeing it. This
    is the codegen convention for `tcl_obj_new_string` feeding
    `tcl_eval` / `tcl_eval_code` / `tcl_expr_bool`, and it is
    distinct from `consumed`, where the caller *did* hold a +1.
  - `passthrough_returned` — argument is conceptually `borrowed`
    but the function may also choose to return the same handle
    as its result. When that happens the caller's +1 implicitly
    becomes the +1 the caller holds for the result. No export in the
    current surface is in this category: the eval loop's
    result-aliases-a-word case is handled instead by `set_obj_result`
    taking an independent +1 (`memory-management.md` MM-B.6), so the
    argv release needs no aliasing awareness.

- **Return** — what the caller can assume about the result handle:
  - `owned` — the caller gets a +1 they must eventually release
    (or transfer into a slot).
  - `borrowed` — the caller does not own the result; reading it
    is fine but the caller must not release it.
  - `null_or_owned` — `null` means "no value" (callers test for it);
    any non-null result is `owned`.
  - `void` — no handle returned.

- **Internal storage** — does the function store the input handle
  somewhere that outlives the call (frame slot, namespace var
  table, dict entry, …)? If so, the function must retain on
  store.

- **Notes** — anomalies, fast paths that depend on `rc == 1`,
  cross-references to the MM-B discipline, etc.

## Conventions

- Every C-ABI export (`#[no_mangle] extern "C"`) named `Tcl_*` /
  `tcl_*`, or used by the WASM codegen via the import table, needs
  a row.
- Helpers reachable only from inside the runtime (no `extern "C"`
  export) do not need rows but must follow the same convention
  internally. Their discipline is
  [`memory-management.md`](memory-management.md) MM-B — the store shape,
  `VarTable`'s release-on-`Drop`, and the ownership of `Var::Scalar` /
  `Var::Array` cells.
- A **null-safe** export is marked as such: `Tcl_IncrRefCount`,
  `Tcl_DecrRefCount`, `tcl_obj_release`, and `tcl_obj_retain` all
  accept null and do nothing (or return it unchanged), so an ABI
  composition that propagates a failure as null stays sound.
- Constructors follow the `fresh_zero` convention: every `Tcl_New*Obj`
  returns `rc 0` and the caller takes the first `Tcl_IncrRefCount`.
  The two codegen-facing constructors deliberately differ from each
  other on this point, and the difference is load-bearing — see the
  `tcl_obj_new_string` / `tcl_obj_new_string_owned` rows.

## Subsystems

The contract is organised by source file. Each subsection mirrors a module
under `runtime/rust/src/` that actually carries `#[no_mangle]` exports; the
rest of the runtime is internal and covered by the convention above.

### `capi.rs` — the C Tcl API slice

The obj-lifecycle and result slice of `tcl.h`, exported for extensions
(`c-extension-abi.md` §4.3). The remainder of the C surface is not exported yet.

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `Tcl_NewObj()` | (n/a) | `owned` (`rc 0`) | none | Fresh empty-string object. |
| `Tcl_NewStringObj(bytes, length)` | (n/a) | `owned` (`rc 0`) | copies/owns the bytes | `length < 0` means `strlen`; `bytes` may be null when `length == 0`. |
| `Tcl_NewWideIntObj(value)` | (n/a) | `owned` (`rc 0`) | none | Pure int object; string rep generated on demand. |
| `Tcl_NewDoubleObj(value)` | (n/a) | `owned` (`rc 0`) | none | |
| `Tcl_NewBooleanObj(value)` | (n/a) | `owned` (`rc 0`) | none | 0/1 int object. |
| `Tcl_IncrRefCount(obj)` | `borrowed` | `void` | (n/a) | **Null-safe.** |
| `Tcl_DecrRefCount(obj)` | `consumed` | `void` | (n/a) | Frees immediately at `rc → 0`; bumps the double-free counter if called on an already-zero object. **Null-safe.** |
| `Tcl_SetObjResult(interp, resultObjPtr)` | `resultObjPtr` `borrowed` | `void` | retains into the interp's result slot, releases the prior occupant | The independent +1 here is what makes the eval loop's unconditional argv release safe. |
| `Tcl_GetObjResult(interp)` | (n/a) | `borrowed` | (n/a) | The interp keeps its +1; valid until the result is replaced. |
| `Tcl_GetStringFromObj(objPtr, lengthPtr)` | `borrowed` | borrowed `char*` | (n/a) | Shimmers on demand. The pointer is into the object's own owned buffer and dies with it. Writes the byte length through `lengthPtr` when non-null. |
| `Tcl_GetString(objPtr)` | `borrowed` | borrowed `char*` | (n/a) | As above with no length out-param. |
| `tcl_runtime_create_interp()` | (n/a) | owning `*mut Interp` | (n/a) | Not in `tcl.h` — the host entry point until `Tcl_CreateInterp` lands. Boxes the `Rc` handle so C has a stable owning pointer. |
| `tcl_runtime_delete_interp(interp)` | `consumed` | `void` | (n/a) | Reclaims the box, running `Drop` once; releases the interp's hold on its result. **Null-safe.** |
| `tcl_test_reset_counters()` / `tcl_test_alloc_count()` / `tcl_test_double_free_count()` / `tcl_test_finalize()` | (n/a) | `void` / i64 | (n/a) | The leak-check surface (MM-C). No refcount interaction. |

### `codegen_abi.rs` — the compiled-module host bridge

The import table an emitted WASM module links against. It evaluates against a
**current interp** installed by the bootstrap, not an interp argument.

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `tcl_runtime_set_current_interp(interp)` | raw pointer, not retained | `void` | module-level current-interp cell | Null clears it. Every other export here no-ops or reports failure when it is null. |
| `tcl_runtime_init_library()` | (n/a) | i32 (0 ok) | (n/a) | C's `Tcl_Init` equivalent: sources `$TCL_LIBRARY/init.tcl` (the embedded-stdlib VFS on the `wasm_stdlib` build). Returns 1 with no current interp. |
| `tcl_obj_new_string(ptr, len)` | copied bytes | `owned` (`rc 0`) | none | **Not retained.** Its consumer (`tcl_eval` / `tcl_eval_code` / `tcl_expr_bool`) *adopts* and frees it. |
| `tcl_obj_new_string_owned(ptr, len)` | copied bytes | `owned` (`+1`) | none | The argv constructor for generic invocation. Deliberately unlike `tcl_obj_new_string`: `tcl_invoke_argv` **borrows** argv words, so generated cleanup must release this reference itself. |
| `tcl_value_new_string(ptr, len)` | copied bytes | `owned` (`+1`) | none | The generated operand stack owns one reference per value. |
| `tcl_value_release(value)` | `consumed` | `void` | (n/a) | Releases one operand-stack reference. |
| `tcl_obj_retain(obj)` | `borrowed` | `owned` (a new `+1` on the same object) | (n/a) | **Null-safe** (returns null unchanged). Used to forward a completion result while releasing the private completion storage. |
| `tcl_obj_release(obj)` | `consumed` | `void` | (n/a) | **Null-safe.** |
| `tcl_codegen_frame_push()` / `tcl_codegen_frame_pop()` | (n/a) | `void` | pushes/pops a real Tcl variable frame | The frame's `VarTable` releases every cell on `Drop`; no per-slot cleanup in the emitter. |
| `tcl_codegen_local_bind(slot, name_ptr, name_len, value)` | `value` `consumed` | i32 (0 ok) | stores into the named cell (which retains) | Associates the compiled slot index with the name-addressable cell, then stores. Releases the operand-stack reference on every path, including the error path. |
| `tcl_codegen_local_set(slot, value)` | `value` `consumed` | i32 (0 ok) | stores into the bound cell | Releases the operand-stack reference even when the slot is unbound. |
| `tcl_codegen_local_get(slot)` | (n/a) | `null_or_owned` | (n/a) | Fires read traces first; null on an unbound slot, a failed trace, or a read miss (with the interp error set). |
| `tcl_codegen_var_set(name_ptr, name_len, value)` | `value` `consumed` | i32 (0 ok) | stores into the named cell | The by-name form; same release discipline. |
| `tcl_codegen_var_get(name_ptr, name_len)` | name bytes | `null_or_owned` | (n/a) | |
| `tcl_codegen_expr_add(left, right)` | both `consumed` | `null_or_owned` | (n/a) | Consumes both operand-stack references on **every** path, including the error and null-argument paths, and returns a fresh `+1`. Without the `have_tommath` backend it consumes the operands and reports `arithmetic support is not available`. |
| `tcl_codegen_puts(value)` | `consumed` | i32 (0 ok) | (n/a) | Dispatches the runtime's own `puts`, then releases the operand-stack reference. |
| `tcl_codegen_proc_register(name, params, body)` | byte ranges, no handles | i32 (0 ok) | defines a `Command::Proc` | Registers source metadata without evaluating `proc`. The body object is fresh and dropped after `define_proc` takes its own copy. |
| `tcl_eval(script)` | `script` `adopted` (`rc 0`, freed here) | `owned` (`+1` on the interp result) | (n/a) | Completion codes are discarded — use `tcl_eval_code` when they matter. With no current interp it returns an owned empty string rather than null, so the misuse path stays leak-safe. |
| `tcl_eval_code(script)` | `script` `adopted` | i32 completion code | (n/a) | Returns 0/1/2/3/4 or a `return -code N` value; the result stays the interp's own (borrowed), so the emitter has nothing to release. Reports 0 with no current interp. |
| `tcl_expr_bool(expr)` | `expr` `adopted` | i32 | (n/a) | The condition primitive for emitted control flow. Yields `0` on an expression error, with no current interp, or in a build without the numeric tower. |
| `tcl_invoke_argv(argv, argc, out)` | every argv word `borrowed`; `out` writable `TclCompletionAbi` | i32 ABI status; on a write, `out.result` and `out.options` are each `owned` | normal interpreter result/error state only | Retains each argv word for the dispatch only, then releases those temporaries. Dispatches the prebuilt argv through the same `Interp::dispatch` as interpreted Tcl — namespaces, `unknown`, aliases, ensembles, TclOO — with no parsing or substitution. A Tcl error is `out.code == 1`; a negative return is a malformed ABI call. |
| `tcl_completion_release(out)` | `out.result` + `out.options` `consumed` | `void` | clears the output storage | Releases both handles exactly once and zeroes the fields. Idempotent on already-reset storage; mixing it with individual `tcl_obj_release` calls is not sound. |
| `tcl_codegen_call_frame_alloc(bytes, align)` | validated positive layout | owned transient frame pointer | shared linear memory | Records the exact layout, increments the diagnostic outstanding-frame count. Frames are distinct and survive re-entrant command callbacks. |
| `tcl_codegen_call_frame_free(frame)` | `frame` consumed once | i32 status | shared linear memory | Looks up the authoritative layout; unknown, forged, or repeated pointers fail without dereference or deallocation. Must run after the argv objects and completion output are released. |
| `tcl_codegen_call_frame_outstanding()` | (n/a) | i32 | (n/a) | Diagnostic counter; no refcount interaction. |

### `regex_capi.rs` — the ARE engine's C shim

`TclReComp` / `TclReExec` / `TclReFree` / `TclReError` re-export the pure-Rust
`tcl-regex` engine over the C ABI. They deal in compiled-regex handles and byte
ranges, not `TclObj`, so no row carries a refcount category: the only rule is
that a handle from `TclReComp` is freed exactly once by `TclReFree`.

### `c_alloc.rs` — libtommath's allocator

`malloc` / `calloc` / `realloc` / `free`, compiled only for freestanding
`wasm32-unknown` with the tommath backend. Raw bytes, no `TclObj`; they forward
to Rust's global allocator with a 16-byte size header (see
[`memory-management.md`](memory-management.md)).

### Internal entry points

`frame.rs`, `namespace.rs`, `vars.rs`, `interp.rs`, `list.rs`, `dict.rs`, and
the `cmd_*.rs` handlers export nothing over the C ABI, so they have no rows
here. Their discipline is uniform and stated once in
[`memory-management.md`](memory-management.md) MM-B: a builtin receives its
`argv` as `borrowed` and returns a `Code`, publishing any result through
`set_result` / `set_obj_result` (which retains independently); a cell store
retains the incoming value and releases the displaced one; and a `VarTable`
releases everything it owns on `Drop`.

## Known gap: no automated enforcement

No gate mechanically checks that every `#[no_mangle] extern "C"` export has a
row here, or that a row's claimed category matches the code. A new export can
therefore ship undocumented, and a changed ownership category can silently
diverge from its row — which is exactly the ambiguity this document exists to
remove.

Closing it means a check that walks the runtime's exports (they are
enumerable from `runtime/rust/src/capi.rs`, `codegen_abi.rs`, `regex_capi.rs`,
and `c_alloc.rs`), diffs them against the rows above, and fails on either
direction. The same shape would serve
[`c-api-ownership-contract.md`](c-api-ownership-contract.md), which has the
identical gap.

## Cross-references

- Runtime refcount discipline: [`memory-management.md`](memory-management.md).
- The C-API surface's ownership rows:
  [`c-api-ownership-contract.md`](c-api-ownership-contract.md).
- Compile-side proof and ownership contract:
  [`../compiler/var-escape-analysis.md`](../compiler/var-escape-analysis.md).
