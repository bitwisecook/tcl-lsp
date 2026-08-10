# Runtime refcount contract

Every C-ABI-exported runtime function lives in one of a handful of
ownership categories. This doc fixes the categories and lists each
export's row so callers (compile-side codegen, the test harness,
other runtime modules) can reason about lifetime without reading
the implementation. The runtime is the Rust crate `tcl-runtime`
(`runtime/rust/`).

> Status: **scaffolding** — categories and conventions are fixed;
> the per-subsystem rows fill in incrementally as the audit proceeds
> against the Rust runtime (`runtime/rust/`). The `cargo xtask
> refcount-contract` lint that once flagged exports missing a row
> walked the runtime's exports and was **retired** (see
> [Lint script](#lint-script) below); the rows
> are maintained by hand for now.

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
  - `passthrough_returned` — argument is conceptually `borrowed`
    but the function may also choose to return the same handle
    as its result. When that happens the caller's +1 implicitly
    becomes the +1 the caller holds for the result. The
    aliasing-aware retain/release in the Rust eval loop
    (`interp.rs`; the pattern was first worked out in commit
    `43a12cb2`) is the canonical example.

- **Return** — what the caller can assume about the result handle:
  - `owned` — the caller gets a +1 they must eventually release
    (or transfer into a slot).
  - `borrowed` — the caller does not own the result; reading it
    is fine but the caller must not release it.
  - `null_or_owned` — `0` means "no value" (callers test `!= 0`);
    any non-zero result is `owned`.
  - `void` — no handle returned.

- **Internal storage** — does the function store the input handle
  somewhere that outlives the call (frame slot, namespace var
  table, dict entry, …)? If so, the function must retain on
  store.

- **Notes** — anomalies, fast paths that depend on `rc == 1`,
  cross-references to MM-B audit commits, etc.

## Conventions

- Every C-ABI export (`#[no_mangle] extern "C"`) named `Tcl_*` /
  `tcl_*`, or used by the WASM codegen via the import table, needs
  a row.
- Helpers reachable only from inside the runtime (no `extern "C"`
  export) do not need rows but should follow the same convention
  internally.
- Fast paths that mutate the input in place (`tcl_cmd_lappend`'s
  in-place append, `tcl_cmd_append`'s buffer growth) are
  flagged: the rc==1 check is the predicate today; once
  compile-side discipline (S2 in
  `docs/design/compiler/wasm-aot-staircase.md`) lands the
  predicate may need updating.

## Subsystems

The contract is organised by source file. Each subsection
mirrors a module under `runtime/rust/src/`.

### `obj.rs` — object lifecycle and primitives

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `Tcl_NewWideIntObj(value)` | (n/a) | `owned` | none | Fresh obj; caller-owned. Rust constructors follow the `fresh_zero` (rc=0) convention — the caller takes the first `Tcl_IncrRefCount`. |
| `Tcl_NewDoubleObj(value)` | (n/a) | `owned` | none | Fresh obj; caller-owned. |
| `Tcl_NewStringObj(data_ptr, length)` | (n/a) | `owned` | copies/owns the bytes | Fresh obj; caller-owned. |
| `obj_get_int(obj)` | `borrowed` | (i64) | none | Reads cached int rep. |
| `obj_get_float(obj)` | `borrowed` | (f64) | none | Reads cached float rep. |
| `incr_ref_count(obj)` (C-ABI `Tcl_IncrRefCount`) | `borrowed` | `void` | (n/a) | Increments rc by 1. **Null-safe**: returns early on obj == null. |
| `decr_ref_count(obj)` (C-ABI `Tcl_DecrRefCount`) | `consumed` | `void` | (n/a) | Decrements rc by 1; frees immediately at rc→0 (bumps the double-free counter if called on an already-zero obj). **Null-safe**. |
| `tcl_obj_drain_pending()` | (n/a) | `void` | (n/a) | Drains the runtime-internal deferred-free queue (eval-loop aliasing case). Only safe at outermost `tcl_eval` depth. |
| OOM flag access (`counters::oom` / reset) | (n/a) | bool / void | (n/a) | No refcount interaction. |

### `frame.rs` — proc-local frame slots

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `frame_push()` | (n/a) | i32 (frame idx) | (n/a) | Frame entry; no obj refs yet. |
| `frame_pop()` | (n/a) | `void` | releases every slot's value | MM-B.3 retain/release on each slot. Idempotent for slots already at 0. |
| `frame_local_set_at(idx, value)` | idx `u32`, value `borrowed` | i32 (= value) | retains value into slot, releases prior occupant | MM-B.3 commit `fe68d410`. |
| `frame_local_at(idx)` | idx `u32` | `borrowed` | (n/a) | Returns the slot's current value without retain. Caller must retain if it stores elsewhere. |
| `var_set` / `var_resolve` / `var_exists` | (TBD) | (TBD) | (TBD) | Audit pending. |
| `frame_set_argv(argv)` / `frame_get_argv()` | (TBD) | (TBD) | (TBD) | MM-B.5d retain/release; audit pending. |

### `namespace.rs` — namespace var tables

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `global_set(name, value)` | name `borrowed`, value `borrowed` | i32 (= value) | retains value, releases prior global value | MM-B.2 (commit `fe68d410`). |
| `global_get(name)` | name `borrowed` | `borrowed` (or 0) | (n/a) | Returns the global table's current value without retain. |
| `global_exists(name)` | name `borrowed` | `owned` (TclObj wrapping 0/1) | (n/a) | Returns a fresh int obj. Caller-owned. |
| `var_set_scalar(v_addr, obj_handle)` | obj_handle `borrowed` | `void` | retains, releases prior | MM-B.2. Internal helper used by `global_set`. |

### `cmd_proc.rs` / `interp.rs` — proc table

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `proc_register(name, params, body)` | all `borrowed` | i32 (status) | retains owned-copy of params + body into proc-table slot | Promotes borrowing TclObjs to owning copies via `ensure_owned`. |
| `proc_set_body_source(name, body)` | name `borrowed`, body `borrowed` | i32 | retains body; releases prior body | Commit `82c0b4ae` fixed prior leak. |
| `proc_register_compiled(...)` | name `borrowed` | i32 | stores raw name in sidecar (not a TclObj ref) | Compiled procs only; func_idx is a wasm function-table index. |

(Other proc-table queries — `proc_get_n_params`, `proc_get_args_tail`,
`proc_get_export_name` — are pure readers, all args `borrowed`,
return primitive types.)

### `interp.rs` — eval loop

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `tcl_eval(script)` | `borrowed` (retained for the call's duration) | `null_or_owned` | (n/a) | Drains pending-free queue at outermost depth. |
| `eval_command(words)` (internal) | each word `borrowed`, alias-aware retain on result | `passthrough_returned` | (n/a) | MM-B.4 release loop after dispatch. |

### `cmd_control.rs` / `cmd_error.rs` — error handling

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `tcl_cmd_error(msg)` | `borrowed` | `void` | sets `error_msg` global to msg (no retain — relies on caller's hold for the duration of the catch frame) | Outside catch, traps via `@trap()`. |
| `catch_enter()` / `catch_leave()` | (n/a) | i32 | (n/a) | Catch-depth counter. |
| `catch_result()` | (n/a) | `borrowed` | (n/a) | Reads `error_msg`; valid until the next command. |

### `list.rs` / `cmd_list.rs` — list operations

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `tcl_cmd_list_length(list)` | `borrowed` | `owned` (fresh int obj) | none | |
| `tcl_cmd_lappend(current, value)` | `current` `borrowed`, `value` `borrowed` | `owned` (often == current via fast path) | (n/a) | **Fast path**: if `rc(current) == 1` AND owns its buffer, mutates in place and returns `current`. Slow path returns fresh obj. The rc==1 predicate is sensitive to compile-side ownership discipline (S2). |
| `tcl_cmd_list_index(list, idx)` | both `borrowed` | `owned` (fresh `obj_new_string_copy`) | none | |
| (other list funcs — TBD) | | | | |

### `value_ops.rs` / `cmd_string.rs` — string operations

(TBD — audit pending. The salient one is `tcl_cmd_append` which
has a similar rc==1 fast path to `tcl_cmd_lappend`.)

### `dict.rs` — dict operations

(TBD — audit pending.)

### `cmd_array.rs` — array (Tcl array, not list) operations

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `array_set(arr, key, value)` | all `borrowed` | i32 | retains value, releases prior | MM-B.5a (commit `fe68d410`). |
| `array_get(arr, key)` | both `borrowed` | `borrowed` | (n/a) | |

### `cmd_regex.rs`

(TBD — one of the capture-storage paths retains the captured value
(`incr_ref_count`) so it outlives the match struct; audit the
remaining `-inline`/`-indices` storage paths.)

### `cmd_*.rs` — per-command handlers

(TBD — these all consume `borrowed` args and return `owned` or
`null_or_owned`. Specific anomalies need rows.)

### `codegen_abi.rs` — host bridge

| Function | Args | Return | Storage | Notes |
|---|---|---|---|---|
| `dispatch(bucket, words)` | each word `borrowed` | `null_or_owned` | (n/a) | Calls `call_compiled_proc` (host bridge); the host receives borrowed handles and returns owned. |
| `tcl_codegen_call_frame_alloc(bytes, align)` | validated positive layout | owned transient frame | shared linear memory | Every successful allocation records its exact layout in the runtime, increments the diagnostic outstanding-frame count, and has one `tcl_codegen_call_frame_free`. Frames are distinct and survive re-entrant command callbacks. |
| `tcl_codegen_call_frame_free(frame)` | `frame` consumed once | i32 status | shared linear memory | The runtime looks up the authoritative allocation layout. Unknown, forged, or repeated pointers fail without dereference or deallocation. A successful free occurs after argv objects and the completion output release, and decrements the outstanding-frame count. |
| `tcl_obj_new_string_owned(ptr, len)` | copied bytes | `owned` Tcl object (`+1`) | object heap | Constructor for prebuilt argv words. `tcl_invoke_argv` borrows this reference, so generated cleanup releases it afterwards. |
| `tcl_invoke_argv(argv, argc, out)` | every argv word `borrowed` (caller holds `+1`); `out` writable `TclCompletionAbi` storage | i32 ABI status; on every write `out.result` + `out.options` are each `owned` | normal interpreter result/error state only | Retains every argv word only for the dispatch, then releases those temporary refs. Dispatches the full prebuilt argv through normal namespace/unknown/alias/TclOO resolution without source parsing or substitution. A Tcl error is `out.code == 1`; a negative status is a malformed ABI call. |
| `tcl_obj_retain(obj)` | `borrowed` live object | a new `owned` (`+1`) reference | object heap | Generated completion transport retains result/options for its caller before releasing its private completion storage. |
| `tcl_completion_release(out)` | `out.result` + `out.options` `consumed` | `void` | clears output storage | Releases both completion handles exactly once and zeroes the fields. A repeated call on the reset storage is idempotent; mixing it with individual `tcl_obj_release` calls is not. |

### `cmd_chan.rs` / `cmd_fs.rs` / `cmd_clock.rs` — stdio + filesystem + clock

(TBD — most are pure consumers; no obj storage.)

## Lint script

**Retired.** The `cargo xtask refcount-contract` lint (S0.1 deliverable) and
the even earlier Python `scripts/check/refcount_contract.py` it had been ported
from have both been removed.
No automated gate enforces this contract today: the rows above are
maintained by hand against the Rust runtime (`runtime/rust/`). The
refcount **discipline** they document still applies in full — only
the tool that mechanically checked for missing rows is gone.

## Cross-references

- Runtime memory-management plan and audit history:
  [`memory-management.md`](memory-management.md).
- Compile-side discipline plan:
  [`../compiler/wasm-aot-staircase-s2.md`](../compiler/wasm-aot-staircase-s2.md).
- The `MM-B` audit commits (`fe68d410`, `1ddb903d`, `9c7e4add`,
  `48a7138b`, `43a12cb2`) provide the historical basis for many
  of the rows above.
