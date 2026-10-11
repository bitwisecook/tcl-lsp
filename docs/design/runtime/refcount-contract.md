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
| `tcl_runtime_create_interp()` | (n/a) | owning `*mut Interp` | (n/a) | Runtime host entry point, not declared in `tcl.h`. Boxes the `Rc` handle so C has a stable owning pointer. |
| `tcl_runtime_delete_interp(interp)` | `consumed` | `void` | (n/a) | Reclaims the box, running `Drop` once; releases the interp's hold on its result. **Null-safe.** |
| `tcl_test_reset_counters()` / `tcl_test_alloc_count()` / `tcl_test_double_free_count()` / `tcl_test_finalize()` | (n/a) | `void` / i64 | (n/a) | The leak-check surface (MM-C). No refcount interaction. |

### `codegen_abi.rs` — the compiled-module host bridge

The import table an emitted WASM module links against. Explicit entry selects
the interpreter installed by the bootstrap. Paired work retains that actual
interpreter until its matching cleanup, independent of later pointer changes.

The table below covers the eval / value / variable core. The slot family
(`tcl_codegen_slot_*`), the guard and activation families
(`tcl_codegen_guard_*`, `tcl_codegen_activation_*`), the typed-value
constructors and accessors (`tcl_value_new_bool` / `_double` / `_wide_int` and
their `tcl_value_get_*` readers), `tcl_codegen_word_concat`,
`tcl_codegen_var_get_element`, `tcl_codegen_var_traced`,
`tcl_codegen_slot_traced`, `tcl_codegen_return_state`,
`tcl_codegen_log_command`, `tcl_codegen_proc_define_native`,
`tcl_codegen_native_proc_dispatches`, and `tcl_intrinsic_invoke_argv` are
exported but have no row yet — a gap in this document, not permission to invent
a convention at the call site.

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `tcl_runtime_set_current_interp(interp)` | raw pointer, not retained | `void` | module-level current-interp cell | Null clears the bootstrap pointer. Outstanding paired work retains its actual interpreter; otherwise interpreter-dependent exports use their documented absent-interpreter behaviour. Installing a pointer outside paired work clears the completed cleanup observation, while preserving each interpreter's actual first Host cause. |
| `tcl_runtime_init_library()` | (n/a) | i32 (0 ok) | (n/a) | C's `Tcl_Init` equivalent: sources `$TCL_LIBRARY/init.tcl` (the embedded-stdlib VFS on the `wasm_stdlib` build). Returns 1 with no current interp. |
| `tcl_runtime_identity(out, capacity)` | writable bytes, not retained | i32 (the encoding's byte length; 0 with no current interp) | (n/a) | What the runtime states of itself — the current interp's pinned context and this build's ABI, intrinsic table and embedded library — as an `ArtefactIdentityManifest`'s bytes, written only when `capacity` holds them, so a host asks once with a null buffer for the size and compares a module's own manifest before linking. No object is created or retained. |
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
| `tcl_codegen_expr_add(left, right)` | both `consumed` | `null_or_owned` | (n/a) | Consumes both operand-stack references on **every** path, including missing interpreter, pending Host and partial-null inputs. Uses the original interpreter's selected operand conversions and shared arithmetic kernel, releases inputs before final currency checks, and returns an owned `+1` only on success. Without `have_tommath`, it consumes the operands and reports `arithmetic support is not available` only when actual entry remains current. |
| `tcl_codegen_puts(value)` | `consumed` | i32 (0 ok) | (n/a) | Dispatches the runtime's own `puts`. Adopts the transferred operand at entry and releases it on every path, before final currency settlement. |
| `tcl_codegen_proc_register(name, params, body)` | byte ranges, no handles | i32 (0 ok) | defines a `Command::Proc` | Registers source metadata without evaluating `proc`. Parameters and body are held through the actual original procedure installer; success requires its installed generation. Host refusal preserves the first cause and prevents a false successful empty result. |
| `tcl_eval(script)` | `script` `adopted` (`rc 0`, freed here) | `null_or_owned` (`+1` on the original interp result) | (n/a) | Guest completion codes are discarded. Host refusal returns null and retains its typed cause. No-interpreter compatibility returns an owned empty string. Input ownership is settled on every path. |
| `tcl_eval_code(script)` | `script` `adopted` | i32 completion code or Host status | (n/a) | Genuine Guest codes retain the original interpreter result. Host refusal returns `-6` without Guest publication. Input ownership is settled on every path. Explicit no-interpreter compatibility reports 0. |
| `tcl_expr_bool(expr)` | `expr` `adopted` | i32 | (n/a) | Compatibility truth projection: genuine Guest error yields false; Host refusal returns `-6` in the independent typed channel. Callers check that channel before consuming truth. Full compiler conditions use `tcl_codegen_expr_bool` and its whole completion. No interpreter or reduced numeric build yields 0. |
| `tcl_invoke_argv(argv, argc, out)` | every argv word `borrowed`; `out` writable `TclCompletionAbi` | i32 ABI status; on a write, `out.result` and `out.options` are each `owned` | normal interpreter result/error state only | Retains each argv word for the dispatch only, then releases those temporaries. Dispatches the prebuilt argv through the same `Interp::dispatch` as interpreted Tcl — namespaces, `unknown`, aliases, ensembles, TclOO — with no parsing or substitution. A Tcl error is `out.code == 1`; Host refusal returns `-6` with output untouched. Capture and activation cleanup use the retained original interpreter. |
| `tcl_intrinsic_invoke_argv(intrinsic, argv, argc, out)` | every argv word `borrowed` | i32 status; written result/options each `owned` | actual intrinsic result/error state | Form decline writes no completion. The checked original getter's Host refusal stops before fallback and leaves output untouched. Temporary argv references are balanced through capture and activation cleanup. |
| `tcl_codegen_word_concat(parts, count)` | every part `borrowed` | `null_or_owned` | counted result bytes | Retains borrowed parts before checked original String access, preserving exact counted bytes. Getter/refusal stops before result publication; caller references are never adopted. |
| `tcl_completion_release(out)` | `out.result` + `out.options` `consumed` | `void` | clears the output storage | Releases both handles exactly once and zeroes the fields. Idempotent on already-reset storage; mixing it with individual `tcl_obj_release` calls is not sound. |
| `tcl_codegen_call_frame_alloc(bytes, align)` | validated positive layout | owned transient frame pointer | shared linear memory | Records the exact layout, increments the diagnostic outstanding-frame count. Frames are distinct and survive re-entrant command callbacks. |
| `tcl_codegen_call_frame_free(frame)` | `frame` consumed once | i32 status | shared linear memory | Looks up the authoritative layout; unknown, forged, or repeated pointers fail without dereference or deallocation. Must run after the argv objects and completion output are released. |
| `tcl_codegen_call_frame_outstanding()` | (n/a) | i32 | (n/a) | Diagnostic counter; no refcount interaction. |

The compiler bridge's `CodegenOperation` retains the original interpreter and
shared `NativeOperationScope` before getters or callbacks. Its variable, slot, scalar, procedure, argv and source operations inherit that
receipt, preserve reached effects and retain the
first typed Host refusal. Consumed setters and `puts` adopt their transferred
`+1` at entry, including absent-interpreter, invalid-slot and pending-refusal
paths. They release it before final currency checks; borrowed append, increment
and argv operands remain the caller's references. A trace-admission query
returns the conservative traced answer when the operation is unavailable.
Neutral numeric fast-path probes keep Guest rejection distinct from original
access refusal and publish their output only after the current receipt check.
Registry form decline remains distinct from failed
object access. Checked completion capture runs while the activation still owns
its Guest error state; mandatory activation cleanup then runs before final
publication. A refusal during capture or cleanup releases retained result and
options and leaves caller output untouched. Mutable ambient interpreter state
cannot retarget this settlement. Arithmetic and unary numeric operators use
the same entered operand preparation as comparison. Only accepted magnitudes
reach temporary typed kernel inputs; a selected numeric rejection never
reparses its original spelling under another ambient release. Unary logical
NOT retains its separate Boolean operand purpose. Original operand cleanup
runs before final result publication. Expression list membership retains the
actual List backing incarnation and allocation-only member leases across original
getters. These leases add no native child references and grant no conversion
purpose. A retired backing or member refuses before the next getter or result;
checked String access and selected List conversion retain the same entered
currency. Authored F5 predicates use that checked String ingress after their
independently installed expression policy admits the operator. Their counted
bytes remain separate from the checked Unicode inputs of the pure predicate;
consumed operands retire before final result publication.

Math function handlers and fixed-function calls enter through the actual
interpreter expression context. Their selected
numeric operands reach the pure function kernel as accepted magnitudes, so a
rejected original spelling cannot be reparsed there. The shared function table
selects primitive Boolean operand acceptance separately from expression truth.
Integer-forward functions retain the original header and resident spelling
only after selected integer classification; window conversion uses the accepted
integer magnitude. Random-generator mutation follows successful seed access
and classification on the same actual interpreter.

Constant expression preparation retains the actual interpreter and entry
through literal cleanup and evaluation. An admitted numeric-result instruction
reaches its shared physical producer; a following Boolean peephole and a public
expression API copy are separate obligations. Caller tags and source profiles
cannot attest that conversion has occurred.

Separate compiler frame and activation calls retain their actual interpreter
and entered currency in a LIFO ledger. Ordinary facade calls, completion
exports and Host-status reads use that owner while it is outstanding; explicit
entry selects the bootstrap's installed interpreter. Every void frame-push
attempt requires one pop, including a refused attempt, so a failed nested push
cannot remove an outer frame. Activation callers owe a leave only after a
successful status. Cleanup stays on the original owner during reentrant object
retirement and always runs under refusal. A wrong entry kind or repeated
reentrant exit retains an operational refusal without removing another entry.
If cleanup leaves a callback-entered entry open, its completed record remains
below that entry. The matching nested exit retires completed scopes in order,
without repeating frame or activation cleanup. No ledger borrow crosses a
callback or scope destructor. A cleanup refusal retains its weak actual owner
and exact first cause until
an explicit independent entry or bootstrap outside paired work; it is never
copied into an outer interpreter. Installing a pointer during held work clears
none of its original currency or refusal. The native
ledger is per thread; the single-threaded Wasm module uses a lazy module-global
ledger without requiring TLS initialisation.

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
direction. That shape already exists for the C-API half:
`scripts/check_c_api_ownership.py` (`make check-c-api-ownership`) enforces the
rows in [`c-api-ownership-contract.md`](c-api-ownership-contract.md) against
`capi.rs`'s `Tcl_*` / `mp_*` exports. Extending it to this document's rows —
which also cover `codegen_abi.rs`, `regex_capi.rs`, and `c_alloc.rs` — is what
remains.

## Cross-references

- Runtime refcount discipline: [`memory-management.md`](memory-management.md).
- The C-API surface's ownership rows:
  [`c-api-ownership-contract.md`](c-api-ownership-contract.md).
- Compile-side proof and ownership contract:
  [`../compiler/var-escape-analysis.md`](../compiler/var-escape-analysis.md).

`OriginalArguments` retains the actual interpreter operation and allocation-only
leases for a command's original argv. Entry checks live allocations without
materialising their Strings or changing native reference counts. `original(index)`
checks a borrowed header; `bytes(index)` reaches the existing selected original
String getter at that command stage, retaining its result and checking the
original currency before returning counted bytes. Neither accessor grants a
name, frame, cache or getter purpose. Native worker consumers select those
through their existing sealed protocols.

Command drivers retain this context through their read/effect sequence. They
check before table effects and after a callback or worker, preserve reached
effects on failure, and finish through the original first Host cause. They read
each subsequent operand only when the selected command reaches it. The common
dispatch boundary enters the same operation before rewrite retirement, result
retirement, traces, lookup and invocation, and checks after tailcall settlement.
An independent engine entry retains its separate reset contract; nested dispatch
never resets the original operation's first cause.

Entered operation and retained scalar issuer currency use one non-wrapping
execution-context epoch. Host installation, a changed pinned context or dialect,
and logical evaluation/source/expression/naming ingress retire those receipts
before changing the actual context. Restoring equal context does not revive an
older receipt; counter exhaustion permanently refuses issuance. An unchanged
pin keeps its existing context currency.

Speculative command guards have a separate interpreter-policy epoch. Legal
Guest changes to commands, children, visibility, safety, limits and background
handlers invalidate their relevant guards and preserve entered operation and
scalar context currency. Currency does not attest command generations,
namespace/frame lifecycle, native getter recipes or value headers; their
existing owners check them independently.
