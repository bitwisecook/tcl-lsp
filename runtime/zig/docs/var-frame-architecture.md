# Cohesive variable / frame / namespace / exception architecture

Status: design proposal, not yet implemented.

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

## Out of scope (for now)

* `trace add variable` support (the `traces` field is there to keep
  the door open).
* Bytecode-level direct frame access (we still go through the
  hash table).
* Cross-thread / cross-interp variable access beyond what
  `interp alias` already does.

## Action item

Treat this doc as the contract for the next round of internal
plumbing changes. Each phase lands behind a `--strict-vars` style
build flag if needed, but the public surface stays compatible. The
remaining stragglers in set.test / parse.test / namespace.test that
have been deferred as "deep refactor" become tractable once phases 1
and 4 land.
