# Contract: variable & call-frame resolution model

> **Status:** First-principles design contract (v2 / "if starting over").
> Describes the semantic model a from-scratch Tcl runtime should commit to
> *before* writing any command, not a description of the current
> implementation. The as-built notes are
> [runtime/memory-management.md](../runtime/memory-management.md),
> [runtime/refcount-contract.md](../runtime/refcount-contract.md),
> [runtime/rename-alias.md](../runtime/rename-alias.md),
> [runtime/trace-implementation.md](../runtime/trace-implementation.md) and
> [namespace-model.md](namespace-model.md).

## Why this is the first thing to get right

In Tcl a variable name resolves to a *cell* through a chain that is only
fully known at runtime: `upvar`, `global`, `variable`, `namespace eval`,
array elements, and traces can all redirect a plain name to a cell in a
different frame or namespace. An AOT compiler that assumes "a proc local is
a WASM local slot" is wrong the moment any of those appear — and they appear
constantly. Almost every deep variable bug (alias sentinels leaking into
handle arguments, traces re-entering during a write, `uplevel` running code
that mutates the caller's locals) is a symptom of the cell/frame model being
implicit instead of designed.

**Design rule:** there is exactly one way to reach a variable — through a
*frame → name → cell* indirection. Optimisations (treating a proven
non-aliased, non-traced local as a raw slot) are layered *on top* behind a
guard; they are never the base case.

## The two stacks and the two contexts

Tcl conflates them in surface syntax but they are distinct:

| Concept | What it is | Who changes it |
|---|---|---|
| **Call frame** | A variable table for the *currently executing* body (proc locals, or the global table at script top level). | A proc call pushes one; `apply`, methods, `uplevel`, coroutines, `[interp] eval` reshape the active one. |
| **Call stack / level** | The numbered chain of frames. `info level` reads it. | Proc calls push; `tailcall` replaces the top; coroutines suspend/resume sub-stacks. |
| **Current namespace** | Context for resolving *command* names and *qualified* variable names. | `namespace eval`, and a proc's *defining* namespace on entry. |
| **Namespace variable tables** | Per-namespace variable storage (`::`, `::foo`, …). NOT the same as call frames. | `variable`, `set ::ns::x`, `namespace eval`. |

The classic trap: the current namespace is **not** the call frame. Inside a
proc defined in `::foo`, an *unqualified* `set x` writes a frame **local**,
not `::foo::x`. You only reach the namespace var by qualifying (`::foo::x`)
or by declaring `variable x` (which installs a local alias to it). At script
top level, by contrast, the call frame *is* the global namespace table, so
unqualified `x` is `::x`.

## The cell

```
Cell {
  storage : Unset | Scalar(value_obj) | Array(name→Cell)
  link    : ?*Cell        // non-null ⇒ this name is an alias (upvar/global/variable)
  traces  : ?*TraceList   // read / write / unset / array traces
  refcount                // a cell may be referenced by several aliasing names
}
Frame {
  names   : Map<string, *Cell>   // local table
  level   : int                  // absolute level number (#0 = global)
  ns      : *Namespace           // current namespace for this frame
  caller  : ?*Frame              // for uplevel/info-level
}
```

A name is resolved to a cell once; an *alias* cell's `link` is followed to
the real cell on every access (so `unset`, traces, and array-ness all act on
the target). Following links must be cycle-safe (`upvar 0 a b; upvar 0 b a`
is an error in C Tcl — detect it).

## Resolution algorithm (the contract)

`resolve(frame, name, create?) -> *Cell | error`:

1. **Parse the name.** Split a trailing `(index)` → array element. Detect a
   `::` prefix or embedded `::` → *qualified*; otherwise *unqualified*.
2. **Qualified `::a::b::x`:** resolve namespace `::a::b` from the frame's
   current namespace (absolute if leading `::`, else relative), then look up
   `x` in that namespace's variable table. `create?` makes the namespace
   var; intermediate namespaces are **not** auto-created for variables.
3. **Unqualified `x`:** look in `frame.names`. If present, follow `link` to
   the target cell. If absent and `create?`, create a local cell in
   `frame.names`. (Unqualified never falls through to the namespace — that
   is what `global`/`variable` are for.)
4. **Array element `a(k)`:** resolve `a` by 1–3 to a cell; require it be
   `Array` (auto-vivify on write, error on read of a missing element); then
   the element cell. `k` is itself substituted before this step.

`upvar L other my`, `global v`, `variable v` all reduce to: resolve `other`
(in the target frame/namespace), then install `frame.names[my] = aliasCell`
whose `link` points at it. The alias is created even if the target is
currently Unset (link to a yet-to-exist cell).

### Frame addressing for upvar/uplevel

* `#N` — absolute, counting from the global frame (`#0`).
* `N` — relative, N frames up from the current (default `1`).
* The target frame must exist (`upvar 3` from level 1 is an error).
* `uplevel L {script}` sets the *active call frame* to frame `L` for the
  duration of `script`, then restores. The script is evaluated with frame
  `L`'s locals visible — see the AOT note below.

## Interactions that must be modelled, not patched

* **`unset` through a link** unsets the *target* cell (and fires its unset
  traces), then removes the local alias name. Unsetting a plain alias does
  not orphan the target's other aliases.
* **Traces are re-entrant interrupts.** A write trace runs arbitrary Tcl
  *during* the write, can itself read/write/unset (including the same
  variable, through another alias), can error (the error replaces the
  operation's result), and fires through `upvar` links in the frame where
  the access occurred. Define ordering (read traces before the read returns;
  write traces after the value is stored; unset traces during teardown) and
  a re-entrancy/recursion policy up front. (Cascaded read-trace re-entry on
  `lappend` is a real crash in the as-built runtime.)
* **`tailcall`** replaces the current frame's *pending* continuation; it must
  not leave a dangling frame or double-pop.
* **Coroutines** capture and resume a *slice* of the call stack; `upvar`/
  `info level` inside a coroutine see the coroutine's stack, not the
  resumer's.
* **`rename`/aliases at the command layer** are the command-table parallel of
  this and share the re-entrancy hazards ([rename-alias.md](../runtime/rename-alias.md)).

## AOT compiler implications

* Compile a proc's variable accesses against the frame's name table by
  default. A local may be promoted to a raw register **only** when the
  compiler can prove, for that proc, that the name is never `upvar`/`global`/
  `variable`-aliased, never traced, never an array, and not reachable by an
  `uplevel`/`eval` in the body. The proof is fragile — guard it and fall back.
* `uplevel`/`eval`/`apply`/`namespace eval` bodies are generally **not**
  compilable in the caller's context (they were compiled, if at all, as
  separate units). They route to the runtime interpreter
  ([parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md))
  with the target frame injected. Treat that hand-off as a first-class path.
* The handle ABI matters here: alias/global sentinels must not collide with
  the small-integer immediate tag or with negative-as-i32 heap addresses
  (see the numeric/handle notes). Reserve sentinel space deliberately.

## Contract vs. incompatible-by-design

| Behaviour | Class | Notes |
|---|---|---|
| Unqualified name = frame local (not namespace var) inside a proc | **Contract** | The single most common semantic surprise; tests depend on it. |
| `upvar`/`global`/`variable` aliasing, link-following on read/write/unset | **Contract** | Including alias to a not-yet-existing var. |
| `#N` vs `N` frame addressing, errors on out-of-range level | **Contract** | |
| Trace fire order, fire-through-link, error-replaces-result | **Contract** | Re-entrancy must terminate and match C Tcl ordering. |
| `info level` / `info frame` *numbers and line tables* | **Incompatible-by-design** | Line/PC tables are tied to C Tcl's bytecode; a from-scratch codegen cannot match them byte-for-byte (the W9-internal bucket). Match the *structure* (`info level` command lists), classify the line tables. |
| Exact wording of `can't read "x": no such variable` etc. | **Contract** | Error strings are tested verbatim. |
| Internal cell layout, refcount values exposed via test hooks | **Incompatible-by-design** | Object-rep probes never match. |

## See also

- [namespace-model.md](namespace-model.md) — command/variable namespace
  resolution this builds on.
- [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — where `uplevel`/`eval` hand off to the interpreter.
- [runtime/trace-implementation.md](../runtime/trace-implementation.md) —
  as-built trace dispatch.
