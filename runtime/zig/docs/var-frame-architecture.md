# Cohesive variable / frame / namespace / exception architecture

Status by phase:

| Phase | Status | Notes |
|---|---|---|
| 1: Variable unification (scalar+array+link) | **landed** | Implemented as the simpler "single directory, synthetic ``::__local::N::*`` namespace" form. Frame buckets are no longer involved in local-array storage; ``array_names`` / ``info exists`` / ``array unset`` etc. all reach the same table. Frame_pop drops local entries via ``drop_local_arrays_for_depth``. |
| 2: `InterpResult` typed return code | **landed** | Four sub-phases all landed: 2a (typed inspection — `result_mod.snapshot` / `consume` / `restore` / `has_signal`), 2b/2c (typed mutation — `set_break` / `set_continue` / `set_return` / `set_signal_break` / `set_signal_continue` plus the `signal_save_and_clear` / `flow_save_and_clear` / `break_continue_save_and_clear` save/restore helpers), HandlerFn signature change (every registered builtin now returns `InterpResult` directly via `from_globals` / `ok` / `err` / `ret` / `brk` / `cont` constructors), and the storage collapse (loose `pub var` flags in `tcl_catch.zig` are now fields of a single `State` struct: `tcl_catch.state.error_flag` etc.).  The doc-stated end-state — "globals become read-through views over the active frame's pending result, populated by a small adapter" — is now a one-line change: swap `tcl_catch.state` for `current_frame().pending` once the per-frame InterpResult slot is wired, and every caller observes the relocated state without further migration. |
| 3: `Frame.cmd_source` metadata | **landed** | Reserved-field-only — see phase 5 note. |
| 4: Cross-interp alias namespace preservation | **n/a** | On further investigation the existing ``dispatch_alias`` behaviour (anchor at parent's root namespace) is the correct TCL_EVAL_INVOKE semantic that real Tcl 9 implements. The proposed "save/restore parent's prior current_ns" would change that semantic and break parse-8.12, not fix it. No work needed. |
| 5: Multi-frame `Tcl_LogCommandInfo` traceback | **landed** | Driven off `tcl_diag.eval_ctx_*` push/pop instead of per-frame `cmd_source`; the latter is reserved for phase 8 (info frame). |
| 6: `trace add variable` | **landed** | Implemented as a separate trace registry (`interp/tcl_var_trace.zig`) keyed by canonical variable name rather than as a per-bucket field — most variables are not traced, so paying the per-bucket field would inflate the array directory.  Hooks fire from `array_set` / `array_get` / `array_unset_element`; whole-array and element-specific traces both work, with re-entrancy guarded by an `active` bit per Trace record.  Proc-local variable traces remain follow-up work (would need per-frame trace lists cleaned up at `frame_pop`). |
| 7: Compile-time slot resolution | **runtime API only — no codegen consumer** | Per-frame `locals_array` of fixed capacity (`LOCALS_ARRAY_CAP = 16` slots) provides direct-indexed local storage parallel to the existing hash-keyed store.  Exports `frame_local_at(idx) i32` and `frame_local_set_at(idx, value) i32` (re-exported through `tcl_runtime.zig` and registered as WASM imports `tcl_frame_local_at` / `tcl_frame_local_set_at` in the codegen import table).  The slot drain on `frame_pop` releases retained values.  **No codegen path emits calls to these helpers yet** — completing the phase requires a Python codegen pass that identifies compile-time-known scalar locals (no `upvar` / `info` / `trace` interference, name is a literal) and assigns them slot indices, plus a `_variables.py` change to route simple reads/writes through the indexed accessors instead of the name-keyed `var_resolve` / `local_set`.  Until that lands the indexed slots are dead storage; the hash-keyed path remains the primary store and there's no perf win.  Open follow-up. |
| 8: `info frame` rich metadata | **landed** | Per-frame `FrameInfo` slots (`type`, `script_obj`, `line`, `cmd_text`, `proc_name`) populated by `eval_proc_call_bucket` / `eval_apply` for interpreted procs, and by the codegen-emitted compiled-proc prologue (which calls `frame_set_type` + `frame_set_proc_name` after the existing `frame_set_argv` it already emits).  `info frame ?N?` exposes them — no-arg returns the current depth (1-based, counting the synthetic top-level frame); `info frame N` returns a `{type X cmd Y proc Z line N}` dict-list for frame N.  Negative offsets count from current per Tcl 9 convention.  Open follow-ups: per-command line tracking (parser would need to thread line numbers through to the dispatcher; today the `-line` field is always 0) and `frame_set_script` / `frame_set_cmd_text` invocation (the compiled-proc body source isn't readily available at the prologue and the cmd_text needs the line-tracking work). |
| 9: Cross-thread / cross-interp variable channels | **typed shapes only — no consumer** | Typed `Interp` handle is already in place (the existing `interp_root` / `child_create` build `Interp` extern structs).  Added `CrossInterpLink` (`{target_interp, target_name}`) as the typed shape for a variable in interp A that aliases a variable in interp B, plus `transfer_result(from, to, value)` for refcount-bookkept TclObj handoff.  **No consumer wires the link traversal**: `var_resolve` doesn't walk `Variable.link` chains, because we never built the per-name `Variable` record the design doc's Phase 1 ideal calls for (we used the simpler scope-keyed-name approach instead).  The `interp transfer CHILD channel` Tcl command is a no-op stub today and stays that way — channels are single-instance file descriptors in the WASM runtime, so cross-interp channel transfer has no meaning here.  Open follow-up: variable-record refactor that adds a Link-storage variant; once that lands `var_resolve` walks `CrossInterpLink.target_interp`. |
| 10: Coroutine-aware frame stacks | **API only — no driver consumer** | `FrameContext` aggregates the per-frame slot arrays (`frame_stack` / `frame_capacity` / `frame_ns` / `frame_argv` / `frame_info` / depth) into a single snapshot type.  `frame_context_save` captures the live state; `frame_context_restore` writes it back; `frame_context_reset` zeros the depth for an isolated eval.  **No driver wires it up.**  The v1 segment-based coroutine driver evaluates each segment inline in the caller's frame stack — it deliberately doesn't isolate, and adding isolation would require careful refcount accounting for TclObj handles that are shared between the live state and the saved snapshot (a copy that doesn't bump refcounts can use-after-free on the next `frame_pop`).  The asyncify path handles its own wasm-side state via `wasm-opt --asyncify` and doesn't use the FrameContext API either.  Open follow-up: proper coroutine driver that holds a per-coro persistent frame, swaps via `frame_context_save`/`restore` on yield/resume, and hands ownership of slot references between live state and saved snapshot. |

The "shims" the original plan called for at the end of each phase
were eliminated as each phase landed — there is **no** backward-
compatibility veneer in the runtime today.  The deferred phases
will land directly without a transition window.

## Why this exists

Today the runtime's storage and control-flow concerns are spread across
six modules that each model their own slice of the picture:

| Module | What it owns |
|---|---|
| `interp/tcl_frames.zig` | Per-frame hash table for local scalars. Encodes upvar aliases as `ALIAS_GLOBAL` / `ALIAS_EXT` sentinel values in bucket value slots. |
| `interp/tcl_ns.zig` | Namespace tree, per-namespace `vars` table, `global_get` / `global_set`. |
| `valtypes/tcl_array.zig` | Standalone "array directory" that's only consulted for **global** arrays. Local arrays are stored as `arr(key)` scalars in the frame. |
| `interp/tcl_catch.zig` | Loose globals for `error_flag` / `return_flag` / `break_flag` / `continue_flag` / `error_msg` / `return_val` / `error_logged` / `catch_depth`. |
| `interp/tcl_interp.zig` | `eval_script`, `dispatch_alias`, `log_command_info`. Responsible for stamping `::errorInfo` traceback frames. |
| `dispatch/tcl_diag.zig` | Tracks the *currently-executing* command source span (`current_eval_ptr` / `current_eval_pos`) for diagnostics. |

Each module is internally consistent, but the **interactions** are
where bugs live. Concrete symptoms we've tripped over:

* **Local arrays are invisible to `array_names` / `array_get`.** Stored
  as flat `arr(key)` scalars in the frame, so the array directory's
  walk misses them. Worked around in three places now: `var_set`,
  `var_resolve`, and most recently `frame_iter_local_array` driven from
  `array_names`. A fourth caller (`array unset`) needs the same
  treatment.
* **Cross-interp aliases lose namespace context.** `dispatch_alias`'s
  `enter()` zeros `tcl_ns.current_ns`, dropping the parent's
  `namespace eval test_ns_1 { ... }` context the test relied on
  (parse-8.12 — currently fixed by a glob over `auto_index` keys, not
  by restoring the actual calling namespace).
* **Tcl 9 stack traceback** (`Tcl_LogCommandInfo`) requires walking
  active frames and appending each "while executing" entry. We stamp
  exactly one entry today (gated on `error_logged`) and skip nested
  frames entirely. Tests like parse-9.1 (currently SKIP'd via
  `testevalex`) want a multi-frame trace.
* **Aliases (upvar / namespace import) use ad-hoc encodings.** Frames
  use sentinel-int values inside bucket payload bytes. Namespace
  imports use a separate "import ref head" pointer. Nothing recognises
  both as instances of the same "this name is a Link to that other
  storage" pattern.
* **`return -level` / `signal_break_flag` / `signal_continue_flag`** —
  three separate flags answer "did break/continue/return need to
  unwind one or more proc frames?". They're checked piecewise; a
  unified result code would do the same job once.

## Proposed model

Two core types — `Variable` and `Frame` — plus a typed `InterpResult`
for control flow. Existing modules become thin wrappers around them.

### `Variable`

```zig
pub const Variable = struct {
    flags: VarFlags,            // SCALAR | ARRAY | LINK | UNDEFINED
    storage: union(enum) {
        scalar: i32,            // TclObj handle
        array: ArrayTable,      // hash of key → TclObj
        link: VarLink,          // alias to another Variable
    },
    traces: ?*TraceList = null, // future-proofing for `trace add variable`
};

pub const VarLink = struct {
    /// 0 = global namespace; otherwise an absolute frame depth or
    /// per-interpreter handle that identifies the owning scope.
    target_scope: u32,
    /// Owned name bytes for the target's lookup key — array element
    /// links carry the full ``arr(key)`` form here.
    target_name: TclObj,
};
```

Replaces the current `ALIAS_GLOBAL` / `ALIAS_EXT` sentinel encoding in
frame buckets, and unifies it with the namespace-import "tracked
back-reference" pointer. `local_set` / `var_resolve` / `array_set` /
`array_get` collapse into:

```zig
pub fn var_lookup(scope: *Scope, name: []const u8) ?*Variable;
pub fn var_set(scope: *Scope, name: []const u8, value: i32) void;
pub fn array_lookup(scope: *Scope, arr: []const u8, key: []const u8) ?i32;
pub fn array_set(scope: *Scope, arr: []const u8, key: []const u8, value: i32) void;
```

Each of those follows `LINK` chains until it lands on a real
`SCALAR` / `ARRAY` storage cell (or proves the chain is unset).
Iteration helpers (`array_names`, `info vars`, `info locals`) get a
single source of truth instead of two parallel directories.

### `Frame`

```zig
pub const Frame = struct {
    parent: ?*Frame,            // call-stack parent (for `uplevel 1`)
    namespace: *Namespace,      // namespace context at frame entry
    locals: VariableTable,      // name → *Variable
    argv: ?[]i32,               // for `info level 0`
    cmd_source: ?SourceSpan,    // for ::errorInfo append
    cmd_obj: i32,               // current invoking word for traceback
};
```

Two important changes from today:

1. **`namespace` is a real pointer** captured on `frame_push`. The
   global mutable `tcl_ns.current_ns` we manipulate today becomes a
   read-through accessor that returns `current_frame().namespace`.
   Cross-interp `enter` / `leave` save and restore the *frame's*
   namespace pointer (not a separate global), so a parent interp
   re-entered via alias dispatch keeps its caller's `namespace eval`
   context.
2. **`cmd_source`** is set by `eval_script` (and the cached-slab
   replay path) when each command is executed. `Tcl_LogCommandInfo`
   walks `parent` and appends `\n    while executing\n"<cmd>"` for
   each frame whose `cmd_source` is populated, mirroring real Tcl's
   `Tcl_LogCommandInfo` traceback construction.

`uplevel` / `upvar` resolve their `level` argument to a `*Frame`
(walking `parent` up the absolute or relative count) and operate on
that frame's `locals` directly. The current `frame_at_depth` linear
scan over a `frame_stack` array becomes parent-pointer chasing.

### `Namespace`

Mostly already shaped right, but loses two responsibilities:

* `tcl_ns.current_ns` — moves into the active `Frame`.
* The global "array directory" — folds into `Namespace.vars` (each
  array becomes a `Variable` with `ARRAY` storage).

### `InterpResult`

```zig
pub const Code = enum(u8) { OK = 0, ERROR, RETURN, BREAK, CONTINUE };

pub const InterpResult = struct {
    code: Code,
    value: i32,             // OK: result obj; RETURN: return value;
                            //   ERROR: error message obj
    error_info: i32 = 0,    // ::errorInfo (gradually appended traceback)
    error_code: i32 = 0,    // ::errorCode dict (NONE / TCL WRONGARGS / …)
    return_level: u32 = 0,  // for `return -level N`
    return_code: Code = .OK,// for `return -code break/continue/error`
};
```

Replaces the seven-flag spread in `tcl_catch.zig`. Every command
implementation returns one of these (or writes through an out-param);
`eval_script`'s loop dispatches on `code` directly.

* `catch BODY` evaluates the body, observes the result, packages
  it as an int code + value pair, and returns OK.
* Loop bodies (`while` / `for` / `foreach`) absorb `BREAK` and
  `CONTINUE`, propagate `ERROR` / `RETURN`.
* Proc bodies absorb `RETURN` (folding to OK with the return value),
  decrement `return_level` for `return -level N`, and propagate
  `ERROR` / `BREAK` / `CONTINUE`. The dispatcher that ran the proc
  sees the absorbed result.
* `Tcl_LogCommandInfo`-equivalent runs once per error event (not once
  per unwinding eval_script frame) by walking the parent-frame chain
  and appending each frame's `cmd_source`.

The `error_logged` u32 we use today disappears — the result struct
is the bookkeeping.

## Migration path

The point of writing this down is that we **don't** have to land it
in one PR. Existing call sites can move incrementally:

1. **Variable + Scope** (independent of frames). Introduce the type
   and a wrapper that `tcl_ns.global_set` / `tcl_array.array_set` etc.
   delegate to. Keep the legacy entry points calling the new core so
   existing callers don't need to migrate yet. Land the array
   directory ↔ frame-local unification first — that closes the
   `array_names` / `array_unset` / `info vars` divergence.
2. **InterpResult** (independent of variables). New `eval_*` helpers
   that take/return the typed result. Existing globals
   (`error_flag` etc.) become read-through views over the active
   frame's pending result, populated by a small adapter.
3. **Frame** (depends on 1 and 2). Push a real `*Frame` per
   `frame_push` call, populate `parent` from `current_frame()`,
   capture the current ns at entry. The per-frame hash table
   continues to back `locals`. Add `cmd_source` write at each
   `execute_parsed_command` site.
4. **Cross-interp alias context** (depends on 3). `interp_reg.enter`
   saves the parent interp's *current frame*, not just the namespace
   pointer. Re-entering on alias dispatch restores both, so the
   parent's `namespace eval` context survives the round-trip.
5. **Tcl_LogCommandInfo traceback** (depends on 3). Replace the
   single-frame `log_command_info` we wrote for parse-9.2 with a
   walk over `parent` chains driven from the new `InterpResult`.

Each of those phases is independently testable: phase 1 closes a
known set-1.* / array-* gap; phase 4 fixes the parse-8.12 namespace-
context workaround we landed via `auto_index` glob; phase 5 lets
parse-9.1 (currently `testevalex`-skipped) actually run.

## What gets simpler

* `var_set` stops inspecting `paren` characters in name strings to
  decide between local-scalar storage and global-array storage —
  there's one Variable lookup per name.
* `array_names`'s two-phase walk over the array directory + a
  separate frame iterator collapses into one walk over the
  resolved scope's `vars` table.
* The `error_flag` / `error_msg` / `error_logged` triplet collapses
  into one field on the active frame's pending result.
* `dispatch_alias`'s save/restore of `current_ns` becomes save/
  restore of the active *frame*; the cross-interp namespace-loss
  case parse-8.12 hits today goes away.

## What stays the same

* Hash table machinery (`hash_table.Table(N)`) is fine as-is — the
  generic table backs both `Variable.array.storage` and `Frame.locals`.
* Namespace tree, command registry, parse cache, dispatch all stay
  put.
* TclObj internals (`TYPE_INT` / `TYPE_BIGNUM` / etc.) are unaffected.
* Existing externally-visible exports (`var_set`, `global_get`,
  `array_get`) continue to work — they become thin shims over the
  new core during phases 1–2.

## Phases 6–10 (formerly "out of scope")

The phases above (1–5) close the bugs we're tripping over today.
Phases 6–10 round out the model to handle features we don't
implement yet but that the unified design naturally accommodates.
Each is independently landable after phases 1–5.

### Phase 6: Variable traces

`trace add variable name ops cmd` — install a callback that fires
on read / write / unset of a variable.  The `Variable.traces`
field reserved in phase 1 is the storage; this phase wires the
callbacks:

```zig
pub const TraceList = struct {
    head: ?*Trace,
};
pub const Trace = struct {
    next: ?*Trace,
    ops: TraceOps,             // bitmask: READ | WRITE | UNSET | ARRAY
    cmd_prefix: i32,           // TclObj — script prefix to invoke
};
```

Hook points: `var_set` / `var_lookup` (read) / `var_unset` /
`array_set` walk the variable's `traces` list and invoke each
callback with `(name, op)`.  Re-entrancy is guarded by a per-
variable "trace active" bit so a write trace that sets the same
variable doesn't loop.

Closes: `trace.test` (currently mostly skipped), `var.test`
trace-related cases.

### Phase 7: Compile-time variable resolution

The bytecode compiler currently emits `var_set` / `var_resolve`
calls keyed on the variable name string — every call hashes the
name.  With Phase 1's typed variables, the compiler can resolve a
local name once at compile time to a frame slot index and emit
direct `frame_local_at(idx)` accesses, skipping the hash on every
call.

```zig
pub const ProcLocal = struct {
    name: []const u8,          // for `info locals`
    slot: u32,                 // index into Frame.locals_array
    has_default: bool,
    default_value: i32,
};
pub const Frame = struct {
    parent: ?*Frame,
    namespace: *Namespace,
    cmd_source: ?SourceSpan,
    cmd_obj: i32,
    locals_array: []Variable,  // direct-indexed; for compiled procs
    locals_table: VariableTable, // hash for dynamic / unknown names
    argv: ?[]i32,
};
```

Compiled procs get the array form (zero-hash); interpreted bodies
fall back to the table.  `info locals` walks both.

Closes: `proc.test` `Tcl 9` semantic gaps; major perf win on tight
loops that hammer the same name.

### Phase 8: Real Tcl call-frame stack for `info frame` / `info level`

`info frame` returns rich call-site metadata (script, line, type,
proc, etc.).  Today we expose only `info level` with a single-arg
shim.  Phase 8 captures the metadata at frame push time:

```zig
pub const FrameInfo = struct {
    type: enum { proc, source, eval, uplevel, alias, unknown },
    script_obj: i32,           // the script being evaluated
    line: u32,                 // 1-based line within script
    cmd_text: i32,             // source slice for the call site
    proc_name: i32,            // for type=proc
};
```

Stamped on `frame_push` from the same source the traceback uses
(phase 5's `cmd_source`).  The diag module already tracks
`current_eval_ptr/pos`; phase 8 promotes that into a stack of
typed entries.

Closes: `info.test` `info frame` cases; debugger / step-over
support.

### Phase 9: Cross-thread / cross-interp variable channels

Today `interp eval` runs the target interp's script on the calling
thread.  `interp transfer` (Tcl 8.6+) hands a channel between
interps; `Tcl_TransferResult` likewise hands a result.  Phase 9
generalises:

* Each interp gets a typed handle (`*Interp`) instead of the raw
  base address we use today.
* Cross-interp variable links (a `Variable.link` whose target is
  `OtherInterp.namespace.var`) work transparently — the lookup
  walks links across interp boundaries.
* `interp transfer` becomes a Variable-link rebinding.

Closes: `interp.test` `interp transfer` cases; `thread`-package
support if/when we ship one.

### Phase 10: Coroutine-aware frame stacks

`coroutine` / `yield` / `yieldto` swap the active frame stack.
Today's globals (`frame_stack`, `frame_depth`) make this awkward
— we have to manually save/restore each global on every yield.
Phase 10 captures the stack head as a `*FrameContext`:

```zig
pub const FrameContext = struct {
    head: ?*Frame,             // top of the active stack
    depth: u32,
    saved_eval_ctx: ?*EvalContext, // for nested coros
};
pub var current_context: *FrameContext;
```

`yield` swaps `current_context` to the parent coroutine's; resume
swaps back.  Dispatched commands always read `current_context.head`
so they automatically see the right stack.  `frame_push` /
`frame_pop` mutate `current_context` instead of the file-scope
globals.

Closes: `coroutine.test` (currently partially supported);
generator-style iterators in user code.

## Action item

Treat this doc as the contract for the next round of internal
plumbing changes. Each phase lands behind a `--strict-vars` style
build flag if needed, but the public surface stays compatible
during the migration window — at the END of the refactor, the
backward-compat shims (e.g. `tcl_array.array_set` proxying to
`scope.var_set` for an `ARRAY` variable) get deleted, since this
is an internal runtime and we don't ship a stable C ABI.

## Open follow-ups

Phases 1, 2, 3, 5, 6, 8 are behaviourally complete.  Phase 4 was
ruled n/a after investigation.  The remaining work:

* **Phase 7** — Python-side codegen analysis pass that identifies
  compile-time-known scalar locals (no `upvar` / `info` / `trace`
  interference, name is a literal) and assigns them slot indices
  0..15.  ``_emitter/_variables.py`` then routes simple reads /
  writes through ``frame_local_at`` / ``frame_local_set_at``
  instead of the name-keyed ``var_resolve`` / ``local_set``.  The
  runtime substrate is in place — only the codegen side is
  needed to deliver the perf goal.

* **Phase 8 (smaller follow-ups)** — parser threading line
  numbers through to the dispatcher so ``info frame N``'s
  ``-line`` field is non-zero, and a body-script invocation
  pathway (``frame_set_script`` / ``frame_set_cmd_text``) for
  the compiled-proc prologue.

* **Phase 9** — variable-record refactor that adds a Link-storage
  variant (the design doc's true Phase 1 ideal that we sidestepped
  with scope-keyed names).  Once ``Variable`` has a Link variant,
  ``var_resolve`` walks ``CrossInterpLink.target_interp`` to
  resolve cross-interp aliases.  ``interp transfer`` for channels
  remains a no-op stub — channels are single-instance file
  descriptors in the WASM runtime, so cross-interp channel
  transfer has no meaning here.

* **Phase 10** — proper coroutine driver that holds a per-coro
  persistent frame and uses ``frame_context_save`` /
  ``frame_context_restore`` on yield / resume.  The blocker is
  TclObj refcount accounting: a flat copy of the slot arrays
  doesn't bump retain counts, so ownership of slot references
  has to be transferred between live state and snapshot.  The
  runtime API is in place — only the driver-level rewrite is
  needed.

* **Phase 6 (smaller follow-up)** — proc-local variable traces.
  Today the trace registry is keyed by canonical FQ name, so
  only global-scope variables (and namespace vars) can be
  traced.  Proc-local trace support needs per-frame trace lists
  cleaned up at ``frame_pop``.

The remaining stragglers in set.test / parse.test / namespace.test
that have been deferred as "deep refactor" become tractable once
phases 1 and 4 land.  Phases 6–10 unlock features we don't yet
support (trace, info frame, coroutines, transfer) without further
restructure — they're additive against the unified core.
