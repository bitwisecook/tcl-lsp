# Contract: variable-trace dispatch & introspection coherence

> The firing, ordering, error, and re-entrancy contract both runtimes
> implement for variable traces, and the rule that introspection (`info`,
> `trace info`) reads live state. Builds on
> [runtime-variable-frame-model.md](runtime-variable-frame-model.md); the
> as-built dispatch is
> [runtime/trace-implementation.md](../runtime/trace-implementation.md). The
> reference is `tmp/tcl9.0.4/generic/tclTrace.c` (`TclCallVarTraces`, the
> `done:` label) and `tclVar.c`, cross-checked against `tmp/tcl8.6.16/`.

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
   (`name1(name2)`) traces — as *groups*, so registration order never puts an
   element trace ahead of an array one. Within a group, **newest-first
   (LIFO)**, for every op. A re-entrancy guard (`active` flag per record)
   skips a record already on the stack.
2. **Callback shape.** Each callback is evaluated as a *script*: the verbatim
   command prefix with `name1`, `name2`, and the **full op word**
   (`read`/`write`/`unset`/`array`) appended as list elements. (This is what
   makes the `cmd … ;#` comment idiom and multi-word prefixes work.) The one
   exception is a trace installed by Tcl 8.x's deprecated `trace variable`
   form, which keeps C's `TCL_TRACE_OLD_STYLE` flag and receives the single
   `rwua` **letter** instead. The flag affects nothing else: `trace remove`
   and `trace vdelete` both mask it out, so either spelling removes a trace
   the other installed.
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
  un-store the value). `TclPtrSetVarIdx` swaps the value in, *then* calls the
  traces, and on error jumps straight to `cleanup`, which never restores the
  old one. This covers every writing command — `set`, `append`, `incr`,
  `lappend` — on scalars and array elements alike, **including a variable or
  element the failing write itself created**: it stays in existence, holding
  the new value.
* **Unset:** the cell is torn down as part of the unset; unset traces fire
  *during* teardown and their errors are ignored (#4). The variable is gone
  regardless — a subsequent read must error `no such variable`.

* **The result is the value read back *after* the write traces**, not the
  value the store was handed: C's `TclPtrSetVarIdx` returns
  `varPtr->value.objPtr` only while the variable is still a defined scalar and
  the interp's empty object otherwise, so a callback that rewrites the variable
  changes what `set`/`append`/`lappend`/`incr` evaluate to, and one that unsets
  it — or turns it into an array — makes the result empty.
* **`incr`, and the `lappend` paths that reach `TclPtrGetVarIdx`, fire `read`
  before `write`**; `append` with values never does (its no-value form is a
  plain read and does). The `lappend` split is C's, not the command's: the
  dispatched `Tcl_LappendObjCmd` and the multi-value `INST_LAPPEND_LIST*`
  opcodes fire `read`, while the single-value in-proc opcodes
  `INST_LAPPEND_{SCALAR,ARRAY,STK,ARRAY_STK}` omit `TCL_TRACE_READS` and fire
  `write` only. Such a read treats a trace error as "no current value" — the
  command still succeeds (`incr` counts from 0, `lappend` discards the old
  value) and the swallowed error stays visible in `errorInfo`.

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
* **`trace remove` deletes the newest match.** It removes exactly one
  registration — the first hit walking the list the same way firing does, and
  that head is the newest. When several registrations are identical the choice
  is observable twice: in the surviving firing order and in `trace info`. The
  match is ops-set plus command prefix only; the old-style flag is masked out,
  so `trace remove variable` and `trace vdelete` each remove what the other
  installed. The same rule governs command and execution traces.

## Introspection coherence (info / trace info)

Trace and variable introspection are **live runtime queries**, never
compile-time-foldable:

* `info exists x` after `unset x` is **0** — including on the compiled path,
  whose local-liveness map is invalidated by `unset`.
* `trace info variable x` returns the live trace list (LIFO order, as added).
  Each entry's op list is rendered in C's fixed `array read write unset`
  order — the order `TraceVariableObjCmd`'s `TRACE_INFO` arm tests the stored
  flag bits, **not** the alphabetical `opStrings[]` order the bad-operation
  error enumerates, and never the order the caller spelled. The sibling kinds
  have their own fixed orders (`rename delete`; `enter leave enterstep
  leavestep`). 8.x's `trace vinfo` reports the same live list with each op set
  collapsed to its `rwua` letters.
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
| Value stored before write trace; cell gone after unset regardless of trace | **Contract** | Mutation not gated on trace outcome — a failed write keeps the new value and any cell it created. |
| `trace info` op order is C's fixed per-kind order, not the spelled order | **Contract** | `array read write unset` / `rename delete` / `enter leave enterstep leavestep`. |
| 8.x `trace variable`/`vdelete`/`vinfo` exist ≤8.6 and are `bad option` at 9.0+ | **Contract** | The registry's `DialectSet::TCL8X` gate states the boundary; the option enumeration follows it. |
| An old-style-installed callback gets the `rwua` letter, not the op word | **Contract** | `TCL_TRACE_OLD_STYLE`; matching still ignores the flag. |
| `trace remove` deletes the **newest** of several identical registrations | **Contract** | Observable in the survivors' firing order and in `trace info`. |
| Fire-through-`upvar`/`global` links; re-entrancy terminates | **Contract** | Acts on the target cell. |
| `info exists/vars/locals`, `trace info` reflect live state | **Contract** | Never compile-time-folded. |
| `(read trace on "x")` errorInfo frame text | **Contract** | Matches C wording. |
| A store's result is the value read back after its write traces | **Contract** | `set`/`append`/`lappend`/`incr`/`lset`/`ledit`; empty once the variable is no longer a defined scalar. |
| `incr`, and `lappend` on the paths reaching `TclPtrGetVarIdx`, fire `read` first | **Contract** | A trace error there is "no current value", not a failure; the swallowed error stays in `errorInfo`. |
| `trace info command\|execution NAME` errors for an unknown command | **Contract** | `unknown command "NAME"`, name as written; `trace info variable` stays empty. |
| Element recovery through a link, and `unset a(k)`'s `name1` | **Contract, release-split** | 9.0+ only — `TclVersion::traces_recover_linked_array_element`. |
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
