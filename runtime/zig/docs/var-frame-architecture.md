# Var / frame / ns / exception architecture — open follow-ups

The phased refactor described in earlier revisions of this doc is
behaviourally complete except for the items below.  Phases 1, 2, 3,
5, 6, 8 landed; phase 4 was ruled n/a (existing `dispatch_alias`
already implements the correct TCL_EVAL_INVOKE semantic).  Phases 7,
9, 10 have runtime substrate in place but no consumer wires it up
yet — each entry below states the missing piece and the file that
needs the change.

## Phase 6 follow-up — proc-local variable traces

**Status today**: `interp/tcl_var_trace.zig` keys traces by canonical
fully-qualified name, so global / namespace variables can be traced
but proc-local variables can't.  Hooks fire from `array_set` /
`array_get` / `array_unset_element` on the global array directory.

**To finish**:

1. Add a per-frame trace list (parallel array in `interp/tcl_frames.zig`,
   shaped like `frame_argv` / `frame_info`).
2. `trace add variable LOCAL …` from inside a proc body — when the
   target name resolves to a frame-local rather than a global,
   register the trace in the per-frame list instead of the global
   registry.
3. Hook the frame-local read / write paths in `tcl_frames.zig`
   (`var_resolve` / `var_set` / `local_set` / `local_get`) to fire
   matching frame-local traces, mirroring what `tcl_array`'s hooks
   already do for arrays.
4. `frame_pop` walks the per-frame trace list, fires UNSET callbacks
   for each, releases the retained `cmd_prefix` TclObj handles, then
   clears the list.

The re-entrancy guard mechanism (`active` bit per Trace record) and
the `fire` / `fire_at_key` helpers in `tcl_var_trace.zig` are reused
unchanged — only the storage location and the hook points are new.

## Phase 7 — compile-time slot resolution (codegen consumer)

**Status today**: runtime substrate landed in `interp/tcl_frames.zig`
(`frame_locals_array: [MAX_DEPTH][LOCALS_ARRAY_CAP]i32`,
`frame_local_at(idx)`, `frame_local_set_at(idx, value)`,
`frame_locals_array_drop_current()` called from `frame_pop`).
WASM imports `tcl_frame_local_at` / `tcl_frame_local_set_at`
declared in `core/compiler/codegen/wasm/_imports.py`.  No codegen
path emits calls to them.

**To finish** (Python codegen, not Zig):

1. Add a compile-time scan over each proc body (likely in
   `core/compiler/cfg.py` or a dedicated pass next to it) that
   collects every scalar local name and proves it can be slot-indexed:
   the name is a literal (no `set $varname` indirection), the body
   doesn't `upvar` / `global` / `variable` / `info exists` /
   `trace add variable` against it, and no nested `eval` /
   `uplevel` / `apply` body could reach it via name.  Conservative
   default: if any of those checks fails, fall back to the
   hash-keyed store for that local.
2. Assign indices `0..LOCALS_ARRAY_CAP-1` (capacity is 16; spill
   beyond that to the hash store).  Stash the assignments on the
   per-proc summary so `_emitter/_core.py` and
   `_emitter/_variables.py` see them.
3. Modify `_emitter/_variables.py` to emit
   `tcl_frame_local_set_at(idx, value)` and `tcl_frame_local_at(idx)`
   for slot-resolved names instead of the current name-keyed
   `tcl_local_set` / `tcl_local_get`.
4. Param-binding in `_emitter/_core.py`'s prologue (around line
   1080, just after `frame_set_argv`) currently routes through
   `tcl_local_set` for each parameter; route through the indexed
   accessor when the parameter is slot-resolved.
5. `info locals` and `info args` continue to walk the hash store;
   slot-resolved names need to be enumerated alongside (Python-side
   metadata join, no runtime change required since the hash store
   already covers anything not slot-resolved).

The runtime side is tested by Phase 6 / Phase 8 already pulling on
the same frame infrastructure; once the codegen lands, sweep
results should show the same pass counts plus a perf delta on
tight-loop benchmarks.

## Phase 8 follow-ups — `info frame` line tracking + body-script

**Status today**: `info frame` and `info frame N` work end-to-end
for both interpreted and compiled procs.  Type / proc-name / depth
are populated.  `-line` is always 0; `-script` and `-cmd` are
unpopulated for compiled procs.

**To finish**:

1. **`-line` field** — parser-side change in
   `runtime/zig/parse/tcl_parse.zig`.  `ParseCommand` already returns
   per-command byte spans; thread the 1-based line number alongside.
   The dispatcher (`tcl_interp.zig::eval_script` and the cached-slab
   replay path) then calls `frame_set_line(line)` immediately before
   `eval_command`.  Compiled procs need the codegen prologue to also
   emit `frame_set_line(line_const)` per-callsite — the codegen knows
   the source line at compile time, so this is a constant.
2. **`-script` / `-cmd`** — already-emitted FQ-name interning in
   `_emitter/_core.py`'s prologue (around line 1090) can be extended
   to intern the proc body source and emit
   `frame_set_script(body_obj)`.  `cmd_text` is per-callsite; when
   the line-tracking work above lands, the same path stamps the
   callsite's source slice via `frame_set_cmd_text(slice_obj)`.

## Phase 9 — cross-interp variable links

**Status today**: typed `Interp` handle is in place
(`interp/tcl_interp_registry.zig::Interp` extern struct).
`CrossInterpLink {target_interp, target_name}` shape and
`transfer_result(from, to, value)` helper added.  `interp transfer
CHILD channel` is a no-op stub (channels are single-instance in the
WASM runtime — keep the stub).  `var_resolve` doesn't walk
`CrossInterpLink` chains because we never built the per-name
`Variable` record.

**To finish**:

1. Decide whether to build the Variable-record refactor we sidestepped
   in Phase 1 (replace the `::__local::N::*` synthetic-namespace
   approach with real `Variable` structs that have a Link variant),
   or to special-case cross-interp links in the existing array
   directory (a separate "cross-interp links" registry keyed by
   `(local_interp, local_name)` → `CrossInterpLink`).  The former is
   the design doc's preferred shape but is genuinely a Phase 1 redo.
   The latter is contained but adds another lookup path.
2. Either way, `var_resolve` (in `interp/tcl_frames.zig`) gains a
   pre-flight check: if the name resolves to a CrossInterpLink,
   call `transfer_result` to move the value back from
   `target_interp.var_resolve(target_name)` into the caller's interp.
3. The `interp share INTERP_A var INTERP_B` and
   `interp transfer-variable` (Tcl 9 spelling — verify against
   `tmp/tcl9.0.3/generic/tclInterp.c`) Tcl commands wire user-facing
   surface to the link installer.

## Phase 10 — coroutine driver consuming FrameContext

**Status today**: `interp/tcl_frames.zig` exposes `FrameContext`
(struct holding `depth` + slot-array snapshots), `frame_context_save`,
`frame_context_restore`, `frame_context_reset`.  `sched/tcl_coro.zig`
v1 segment driver evaluates each segment inline in the caller's
frame stack and doesn't use the API.  Asyncify driver handles its
own wasm-side state via `wasm-opt --asyncify` and doesn't use it
either.

**Blocker**: `FrameContext` is a flat copy of the slot arrays.  Slot
entries are TclObj handles with retain bookkeeping; copying the array
doesn't bump retain counts.  After `frame_context_save` returns, both
the live arrays and the saved struct point at the same handles, but
only one of them logically "owns" the +1 retain.  A subsequent
`frame_pop` releases the slot handle through the live state and
leaves a dangling pointer in the saved snapshot.

**To finish**:

1. Decide the ownership model.  Two viable options:
   * **Save-transfers-ownership**: `frame_context_save` retains every
     non-zero slot handle once (transferring the slot's hold to the
     snapshot) and zeroes the live slot.  `frame_context_restore`
     unpacks the snapshot back into the live arrays without
     additional retains.  Simple but expensive — every save walks
     up to `MAX_DEPTH × per-frame-slot-count` handles.
   * **Refcount-on-restore**: `save` is a flat memcpy as today; the
     coroutine driver keeps the snapshot OUT of the live state until
     restore, and the live state's slots are explicitly drained
     between save and restore.  Symmetric on restore (live drains
     pre-restore, snapshot bytes copy in).  Requires the driver to
     enforce the live-drain rule.
2. Add a per-Coro `ctx: FrameContext` + `has_ctx: u32` field in
   `sched/tcl_coro.zig::Coro`.
3. `resume_segments` (and the asyncify equivalent if we ever finish
   2.6): on entry, save caller's context with the chosen ownership
   model, restore the coro's context (or push a fresh frame on first
   resume).  On yield, save coro's context, restore caller's.  On
   terminal completion, drop coro's frame and restore caller.
4. Validate via `coroutine.test` cases that exercise per-coro local
   state across yields — those currently work-by-accident because
   the v1 segments share the caller's stack and don't isolate.

---

The remaining stragglers in `set.test` / `parse.test` /
`namespace.test` that were deferred as "deep refactor" became
tractable with phases 1 + 4 (already landed).  The follow-ups above
are additive: each one extends a feature surface that already builds
and tests cleanly today.
