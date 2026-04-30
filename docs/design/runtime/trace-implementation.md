# Variable traces — implementation gap

## Status

`trace add variable …` and `trace add command …` are currently
**no-ops**.  The runtime accepts the syntax (so tcltest harnesses
load), silently drops the callback, and never fires it.  See
`runtime/zig/interp/tcl_trace.zig` (header comment line 1) for the
canonical statement of intent.

This is the largest spec divergence we still carry.  It's tracked
here because none of the other gap fixes (see
[`leak-sweep-trap-triage.md`](leak-sweep-trap-triage.md) and the
S2/S6 work) touch it, and because doing it right is a multi-day
project rather than a one-commit fix.

## What has to change

A real implementation needs a hook on every `var` mutation site.
The runtime touches vars through three layers:

1. **Compiled-side direct writes.**  `_emit_var_write_obj_impl` in
   `core/compiler/codegen/wasm/_emitter/_variables.py` emits
   `tcl_global_set` / `tcl_local_set` / `_emit_owned_local_write`
   / `tcl_array_set` for set-shaped operations.  Each of those
   call sites is an opportunity to invoke a write trace.
2. **Runtime-side mutators.**  `tcl_cmd_lappend`, `dict_set` (when
   it routes back through a var), `incr`, and the array-element
   commands all eventually call `var_set_scalar` (scalars) or
   `array_set` / `bucket_set_value` (array elements).  Centralising
   the trace-fire on those bottlenecks would cover compiled and
   eval-fallback paths together.
3. **Eval-fallback writes.**  `tcl_eval` routes every interpreter-
   side `set` through `tcl_cmd_set` in `runtime/zig/cmds/var.zig`.
   Traces installed on a var must fire there too.

## Trace storage

Each `Var` (in `tcl_ns.zig`) and each array bucket (in
`tcl_array.zig`) needs an optional pointer to a TraceList struct:

```
TraceList:
  count: u32
  cap:   u32
  ents:  pointer to TraceEntry[]

TraceEntry:
  ops:        u32   // bitmask: 1=read, 2=write, 4=unset, 8=array
  cmd_obj:    i32   // TclObj of the callback prefix
  next:       u32   // pointer for chained removals
```

Adding a 4-byte `traces` slot to the existing `Var` and bucket
layouts is the minimal change.  It's zero-cost on the no-trace path
(load-and-test against zero in the existing fast paths).

## Firing semantics

When a write fires:

1. Build a 3-element TclObj list `[name1 name2 op]` per Tcl spec
   (name1 = base name, name2 = element key for arrays else empty,
   op = "write" / "read" / "unset" / "array").
2. Append to the user callback prefix to form the full command.
3. Bump `recursion_depth`; if a write trace mutates the same var
   from inside the trace, fire-and-forget rather than re-enter
   (Tcl matches this).
4. Run the command via `tcl_eval`.
5. Decrement; clear `trace_in_progress`.

`tcl_eval` already knows how to invoke commands, so step 4 is just
re-using existing machinery.  The complexity is in steps 1–3: name
synthesis, recursion guards, and integration with the catch system
(a trace error becomes the var-op's error).

## Effort estimate

Roughly:

* **0.5 day** — TraceList allocator + add/remove ops on Var and
  array bucket.
* **0.5 day** — fire-on-write hook in `var_set_scalar` and
  `bucket_set_value`.
* **0.5 day** — read trace (pre-fetch hook in `var_get_scalar` /
  `array_get`).
* **1.0 day** — unset traces + the array-shape `array` op
  variant + recursion guard.
* **0.5 day** — `trace info variable` / `trace remove variable`
  back-ends.
* **0.5 day** — port `_emit_unsupported_trap` for `trace info` /
  `trace info variable` back to the no-op return-path so existing
  tests that probe the surface API still load.

So **~3.5 days** of careful work.  This is the order-of-magnitude
estimate for a correct implementation; someone familiar with the
runtime might do it in 2 days, somebody not might take 5.

## What unblocks once it lands

* tcltest's lazy-init traces (e.g. `testConstraint` lookups that
  populate `tcl_platform` on first read) start firing — currently
  the harness silently runs against the *initial* values for all
  constraints, which incorrectly rejects some platform-conditional
  tests.
* `trace add command rename` fires on `[rename]` — used by some
  tcltest fixtures to count proc swaps.
* User-script `trace add variable` invocations are observed —
  removes the silent-drop hazard for end users who write
  reactive-style Tcl.

## Out of scope here

* Execution traces (`trace add execution …`) — separate project,
  hooks per-proc rather than per-var.
* `trace add command rename` for compiled procs — straightforward
  once the var-trace machinery is in place but follows from it.
