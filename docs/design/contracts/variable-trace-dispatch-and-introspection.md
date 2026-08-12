# Contract: variable-trace dispatch & introspection coherence

> The firing, ordering, error, and re-entrancy contract both runtimes
> implement for variable traces, and the rule that introspection (`info`,
> `trace info`) reads live state. Builds on
> [runtime-variable-frame-model.md](runtime-variable-frame-model.md); the
> as-built dispatch is
> [runtime/trace-implementation.md](../runtime/trace-implementation.md). The
> reference is `tmp/tcl9.0.3/generic/tclTrace.c` (`TclCallVarTraces`, the
> `done:` label) and `tclVar.c`.

## Why the dispatcher is shared

A variable trace is a **re-entrant interrupt that runs arbitrary Tcl in the
middle of a read / write / unset**, and its result *reshapes the operation's
own result*. Treat the callback as "fire and forget" — evaluate it and discard
its return code — and a read trace that errors makes the operation report the
raw callback message instead of `can't read "x": …`, and an unset trace error
wrongly aborts the unset. The trace's *return code and result are part of the
operation's contract*, not a side channel.

**Design rule:** one shared trace dispatcher mediates every read / write /
unset / array access. It owns the firing order, the error-wrap/ignore/stop
policy, and the re-entrancy guard. No command-specific path fires traces ad
hoc.

## The dispatcher contract

`fire(op, name1, name2, leaveErrMsg) -> code` where `op ∈ {read, write, unset,
array}`, `name1`/`name2` are the array-and-element (scalar ⇒ `name2` empty):

1. **Order.** Whole-array (`name1`) traces fire **before** element
   (`name1(name2)`) traces. Within a list, **newest-first (LIFO)**. A
   re-entrancy guard (`active` flag per record) skips a record already on the
   stack.
2. **Callback shape.** Each callback is evaluated as a *script*: the verbatim
   command prefix with `name1`, `name2`, and the **full op word**
   (`read`/`write`/`unset`/`array`) appended as list elements. (This is what
   makes the `cmd … ;#` comment idiom and multi-word prefixes work.)
3. **Read / write error → reshape + stop.** If a read or write callback
   returns `TCL_ERROR` (and `leaveErrMsg`), the operation **fails** and the
   result is rewritten:
   * read  → `can't read "NAME": <callback msg>`
   * write → `can't set  "NAME": <callback msg>`   (verb is **`set`**, the
     errorInfo *type* is **`write`** — `(write trace on "NAME")`)
   * **NAME is the user-facing name** — `arr(key)` for an element, **even when
     the matching trace was installed on the whole array** (carry the accessed
     element's name through, do not report the matched key).
   * Firing **stops** at the first read/write error (no further traces).
4. **Unset error → ignore + continue.** An unset callback's error is
   **discarded**: the pre-trace interp state is restored, the unset **still
   succeeds**, and the **remaining unset traces still fire**. Unset never
   propagates a trace error.
5. **Array trace error** mirrors read/write (verb `trace array`) but is its
   own op.

The mechanical mirror of C's `Tcl_SaveInterpState`/`RestoreInterpState` in a
flag-based runtime is a **before/after error-flag delta**: snapshot the flag
before the callback, and only the callback that *newly* set it owns the
wrap-or-ignore decision (so a later callback whose eval no-ops under an
already-set flag can't double-wrap).

## The mutation is independent of the trace outcome

Where C commits the operation around the trace, the runtime must too:

* **Write:** the value is **stored before** write traces run (a write trace
  observes the new value; its error reshapes the *result*, it does not
  un-store the value).
* **Unset:** the cell is torn down as part of the unset; unset traces fire
  *during* teardown and their errors are ignored (#4). The variable is gone
  regardless — a subsequent read must error `no such variable`.

**Rule:** never gate the mutation on the trace's success.

## Re-entrancy and aliasing

* A trace fires **through `upvar`/`global`/`variable` links** in the frame
  where the access occurred — the dispatcher acts on the *target* cell.
* A callback may read/write/unset the same variable (via another alias). The
  `active` guard makes this terminate and match C's ordering. Cascaded
  read-trace re-entry through `lappend` is the sharp edge: the canonical-list
  fast path and the trace re-entry must agree on when the value is
  materialised.
* Removing a variable's traces happens *after* the unset callbacks have fired,
  so a stale trace cannot re-fire on a later variable that reuses the name.

## Introspection coherence (info / trace info)

Trace and variable introspection are **live runtime queries**, never
compile-time-foldable:

* `info exists x` after `unset x` is **0** — including on the compiled path,
  whose local-liveness map is invalidated by `unset`.
* `trace info variable x` returns the live trace list (LIFO order, as added).
* `info vars` / `info locals` / `info level` read the current cell table and
  call stack.

**Rule:** any compile-time const/liveness map MUST be invalidated by `unset`,
`upvar`, `global`, `variable`, `trace add variable`, and every
`eval`/`uplevel`/dispatch boundary; when unsure, lower the `info`/`trace`
query to a real runtime call. (See
[compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md).)
A traced variable also **defeats local-slot promotion** — a slot the
interpreter can't see by name can't be traced.

## Contract vs. incompatible-by-design

| Behaviour | Class | Notes |
|---|---|---|
| Read-trace error → `can't read "NAME": <msg>`; write → `can't set "NAME": <msg>` | **Contract** | Verb `set`, errorInfo type `write`; strings tested verbatim. |
| Full `arr(key)` name reported even for a whole-array trace | **Contract** | The accessed element's name, not the matched key. |
| Unset-trace error ignored; unset succeeds; later unset traces still fire | **Contract** | C disposes the result, leaves code OK, continues. |
| Read/write error stops further trace firing; whole-array before element; LIFO | **Contract** | Ordering is observable. |
| Value stored before write trace; cell gone after unset regardless of trace | **Contract** | Mutation not gated on trace outcome. |
| Fire-through-`upvar`/`global` links; re-entrancy terminates | **Contract** | Acts on the target cell. |
| `info exists/vars/locals`, `trace info` reflect live state | **Contract** | Never compile-time-folded. |
| `(read trace on "x")` errorInfo frame text | **Contract** | Matches C wording. |
| Trace storage layout, callback dispatch internals, refcounts | **Incompatible-by-design** | Object-rep probes never match. |

## See also

- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) — traces
  live on the cell; this is the firing contract for them.
- [compiled-scope-and-name-lowering.md](compiled-scope-and-name-lowering.md) —
  why a traced var can't be a raw slot and why `info` is a live query.
- [runtime/trace-implementation.md](../runtime/trace-implementation.md) —
  as-built variable + command/execution trace dispatch.
- [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — the eval path a trace callback is evaluated through.
