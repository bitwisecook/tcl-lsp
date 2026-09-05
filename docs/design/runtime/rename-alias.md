# Runtime rename + interp alias

The Tcl 9 semantics for ``rename`` and ``interp alias`` in the WASM runtime
(`runtime/rust`), built on the namespace tree from
[`namespace-tree.md`](namespace-tree.md).  Source of truth for the C semantics
is ``tmp/tcl9.0.4/generic/tclBasic.c`` (``TclRenameCommand``, ``:3152``)
and ``tmp/tcl9.0.4/generic/tclInterp.c`` (``AliasCreate`` /
``AliasDelete`` / ``OPT_TARGET``, ``:1121``).

The command-layer *contract* this implements — and its compiler-side
consequences — is
[`../contracts/command-binding-and-aliasing.md`](../contracts/command-binding-and-aliasing.md);
the static, editor-facing slice is
[`../contracts/command-alias-resolution.md`](../contracts/command-alias-resolution.md).

## 1. Scope

In:

- ``rename`` — same-ns, cross-ns, rename-of-an-import-source,
  rename-to-empty (delete).
- ``interp alias`` — create / query / delete, with frozen prefix
  args.  The *target* interpreter path must always be ``{}`` (the
  interpreter running the ``interp alias`` command); a non-empty
  target path raises ``only single-interp aliases (empty
  interpreter paths) are supported``.  The *source* path may name a
  child, which installs a child→parent alias (§4.5).
- ``interp aliases ?path?`` — every alias command's name in the
  addressed interpreter.
- ``interp target path alias`` — the interpreter path to the
  interpreter an alias resolves its target in (§4.3).
- Alias dispatch trampoline wired into the one command-dispatch
  switch so aliases are transparent to callers.  Resolution is by
  *stored target name* on each dispatch, anchored at the global
  namespace — this lazily observes deletion of the target but does
  NOT follow ``rename`` of the target (the stored name stops
  resolving).  Matches C Tcl's semantics.
- C's ``TclPreventAliasLoop`` gate on both alias creation and
  ``rename``: an alias that would close a cycle is refused with
  ``cannot define or rename alias "X": would create a loop`` (§4.6).

Out, deliberately: ``interp cancel``, ``interp share`` and ``interp
transfer`` are neither implemented nor *advertised*.  They need
infrastructure this runtime has none of — a cancellation flag the eval
loop polls (C's ``Tcl_CancelEval``) and a channel table shared between
interpreters — so the ``bad option`` list names only what actually
dispatches, rather than repeating tclsh's full list and naming three
subcommands that would then fail with the wrong error (issue #1412
item 3).  That is a knowing divergence from tclsh's advertised list;
see [`child-interp.md`](child-interp.md) §1.

Covered by sibling documents:

- Child interpreters and the rest of the ``interp`` ensemble —
  [`child-interp.md`](child-interp.md).
- ``interp hide`` / ``interp expose`` —
  [`command-introspection.md`](command-introspection.md).
- Command traces firing on rename and delete —
  [`trace-implementation.md`](trace-implementation.md).

## 2. The `Command` handle

There is no flags word and no type-punned payload slot.  A command
table entry is the `Command` enum in `interp.rs`, and each redirect
kind is its own variant carrying exactly the data that kind needs:

| Variant | Payload | Dispatch |
|---|---|---|
| `Builtin(BuiltinFn)` | a native Rust fn pointer | call it |
| `Proc(Rc<ProcDef>)` | params, body, defining `NsId`, FQN, source provenance | push a frame, run the body |
| `Alias { target, prefix }` | target name + frozen prefix words | §4.2 |
| `ParentAlias { target, prefix }` | same, but the target runs in the parent interp | §4.5 |
| `Imported { source }` | the source command's FQN | re-resolve `source` at global, forward argv unchanged |
| `Ensemble(EnsembleConfig)` | the subcommand map | map `argv[1]` to a target prefix |
| `ChildInterp(name)` | the child's name | route the subcommand into the child |
| `OoObject(fqn)` | the object/class FQN | route to `cmd_oo` |

`Command` is `Clone` but not `Copy`, and that is load-bearing:
`Namespaces::resolve` hands back a **clone** of the handle, so the
dispatcher holds no borrow on the command table and a command is free
to mutate the table it was found in (`rename` inside a proc, an alias
whose target deletes the alias).

Two consequences differ from a flags-and-payload layout:

- **Imports are not unwrapped at lookup.**  `resolve` returns the
  `Imported` handle itself; `Interp::invoke` re-resolves `source`
  anchored at global on every call and forwards the caller's argv
  unchanged.  The redirect is transparent to the *caller* but is
  still a distinct table entry, which is what lets `namespace forget`
  find it and what lets a source rename retarget it (§3.3).
- **Aliases are not unwrapped either**, for the same reason plus one
  more: `interp alias {} foo` has to introspect the redirect, and
  `alias_info` does exactly that — resolve the name and match
  `Command::Alias`.

## 3. ``rename``

### 3.1 Semantics mirror

| Form                              | Behaviour                                                                                     |
|-----------------------------------|-----------------------------------------------------------------------------------------------|
| ``rename old new``                | Move the `Command` out of the namespace where ``old`` *resolves* and insert it under ``new`` (relative to the current namespace, absolute when ``::``-led). |
| ``rename old ""``                 | Delete ``old``: drop the table entry, tear down a suspended coroutine of that name, remove the command's traces. |
| ``rename foo foo``                | Refused — ``can't rename to "foo": command already exists`` (`TCL OPERATION RENAME TARGET_EXISTS`); the source still occupies the slot when the destination is checked, so a self-rename is *not* a no-op. |
| ``rename a b`` with ``b`` bound   | Refused with the same message and errorcode; **both** commands survive intact.                |
| ``rename nosuch x``               | ``can't rename "nosuch": command doesn't exist`` (`TCL LOOKUP COMMAND nosuch`).                |
| ``rename nosuch ""``              | ``can't delete "nosuch": command doesn't exist`` (`TCL LOOKUP COMMAND nosuch`) — the verb follows the requested operation, not the failure. |

Any command may be renamed, builtins included: C Tcl has no protected
list at this layer, and `rename ::return ::myreturn` succeeds on real
tclsh (8.6.16 / 9.0.4-pinned), so the runtime does not refuse it either.

### 3.2 Where ``old`` and ``new`` resolve

Both names go through the same written-name split the resolver uses, so
`rename` retires exactly the binding `resolve` would have hit — not a
same-namespace guess.  `Namespaces::rename` calls `home_of(current,
old)` to find the owning namespace and simple key, removes it, then
splits `new`: absolute when it starts with ``::``, otherwise relative to
the current namespace, creating any intermediate namespaces on the way.

One edge case is pinned rather than incidental: a `new` ending in a
separator run names the empty-string ``{}`` command in the full
qualifier chain — `rename foo x::` binds ``::x::{}`` and `rename bar ::`
binds the global ``{}`` command (tclsh 8.6/9.0-pinned, issue #934).
The command table is a `BTreeMap<Vec<u8>, Command>` per namespace, so
removal is a real removal; there are no tombstones and no probe chains
to preserve.

Occupancy of the destination is decided one layer above the table
operation, in `Interp::rename_command`, via
`Namespaces::destination_occupant_fqn` — and *before* any command trace
fires, matching C, which checks ``newNsPtr->cmdTable`` before it touches
the source (``tclBasic.c:3213``).  `Namespaces::rename` itself stays the
unconditional table op and refuses nothing; occupancy protection is the
caller's job.  One destination that looks occupied but is not: a
release-gated TclOO root, which `Interp::is_gate_hidden_object_root`
reads as free because the emulated release does not carry it.

### 3.3 What follows the command

`Interp::rename_command` is the layer above the table operation, and it
carries five things across the move:

- **Command traces** fire *before* the mutation (C's `TclRenameCommand`
  semantics — the command still exists under its old name during the
  callback), with both names fully qualified: `rename` ops get
  ``old new``, the delete form gets ``old {}``.  Afterwards the trace
  list is moved to the new FQN, or dropped on delete.
- **`namespace import` redirects** are retargeted.  In C an imported
  command holds the source's command *token*, so renaming the source
  keeps every import working and `namespace origin` reports the
  source's **new** name (tclsh 8.6.16 / 9.0.4-pinned).  This runtime
  stores the source by name, so `Namespaces::retarget_imports` walks
  the tree and rewrites every `Imported { source }` matching the old
  FQN.  Deletion is deliberately *not* retargeted: a deleted source
  leaves the import dangling with ``invalid command name``, also pinned.
- **Coroutines.**  A `rename $coro {}` runs `cmd_coro::on_command_deleted`
  first, so the suspended worker is torn down rather than orphaned.
- **TclOO objects.**  When the OO registry is non-empty, a renamed or
  deleted object command updates (or is removed from) it.
- **Proc home.**  A cross-namespace rename re-homes the proc:
  `Namespaces::rehome_proc` builds a fresh `Rc<ProcDef>` with the
  destination's namespace and FQN, so ``namespace current`` inside the
  body reports the *destination* (C assigns ``cmdPtr->nsPtr =
  newNsPtr``, ``tclBasic.c:3239``).  Frames already on the stack keep the
  snapshot they captured, so a rename during an active call does not
  re-home that call.

A delete-trace callback may itself delete the command — by deleting the
object's namespace, say.  C captured the command token before the
callback, so the deletion still succeeds; the runtime reproduces that by
treating "existed at entry, gone now, ``new`` empty" as a normal delete
rather than reporting ``command doesn't exist``.

### 3.4 No protected-command list, three refusals

`Namespaces::rename` is the pure table operation and refuses nothing.
There is no protected-command list above it either: renaming a builtin,
including ``return`` and ``error``, succeeds, which is the tclsh-pinned
behaviour described in §3.1.  C's ``TclProtectedCommandsList`` is not
mirrored — the pin is the observed tclsh result, not the C data
structure.

What the `rename` builtin *does* refuse is not about the source command
but about the move itself, and there are exactly three cases:

- an **occupied destination** — ``can't rename to "X": command already
  exists`` (§3.1), self-rename included;
- a **rename that would close an alias cycle** — ``cannot define or
  rename alias "X": would create a loop`` (§4.6);
- a **release-gated builtin**, which the emulated release does not
  carry and so cannot be moved (#1462 / #1463).

### 3.5 Invalidation

Nothing to invalidate.  Resolution is a live `BTreeMap` walk on every
dispatch — there is no per-namespace command-reference epoch, no
resolver cache, and no proc-lookup LRU in this runtime, so a rename is
observable on the very next resolve with no bookkeeping.  The one
cache-shaped structure that does exist is the command-FQN ⇆ `CommandId`
arena (`InterpState::cmd_arena`), and it is a name interner, not a
binding cache: ids map to FQNs, and the FQN is re-resolved when a
`dispatch_id` invokes it.

## 4. ``interp alias``

### 4.1 The alias record

```rust
Command::Alias {
    target: Vec<u8>,      // the target command's name, as written
    prefix: Vec<Vec<u8>>, // the frozen prefix words
}
```

That is the whole record.  It is stored in the command table like any
other binding, via `Namespaces::register`, so a qualified alias name is
rooted at global and creates its intermediate namespaces; an unqualified
one binds at global (aliases are interpreter-wide, matching C, which
registers them in the interp's alias table rather than a namespace).

`Namespaces::alias_names` is what `interp aliases` reads: it scans every
namespace for `Alias` and `ParentAlias` entries and returns global ones
under their simple name, namespaced ones fully qualified.

### 4.2 Dispatch

`Interp::dispatch_alias` runs when `invoke` matches `Command::Alias`:

1. Resolve the stored `target` **by name, anchored at the global
   namespace** (`resolve(GLOBAL, target)`), on every dispatch.
2. On a miss, raise ``invalid command name "<target>"`` — the alias is
   lazily bound, so a target deleted after the alias was created fails
   here, and a target *renamed* after the alias was created also fails
   here, because the stored name simply stops resolving.  Both match C.
3. Synthesise ``[target, *prefix, *caller_tail]`` as a fresh owned argv
   (each element `+1`; released as a block after the call).  There is
   no word-count ceiling — the argv is a `Vec`.
4. `invoke` the resolved handle.  Alias-of-alias chains fall out
   naturally: the resolved target may itself be an `Alias`, so the
   trampoline re-enters itself until a hop resolves to something that is
   not an alias.  A *cycle* would never reach such a hop, which is why
   the definition-time gate in §4.6 exists; `ALIAS_DISPATCH_DEPTH`
   bounds the nesting as well, so a cycle that somehow bypassed the gate
   raises ``too many nested alias invocations (infinite loop?)`` instead
   of exhausting the native stack (a WASM trap).

### 4.3 Create / query / delete

`cmd_alias.rs` implements the ``alias`` and ``aliases`` subcommands of
the `interp` ensemble; the rest of the ensemble
(``create``/``eval``/``delete``/``hide``/``expose``/``invokehidden``/…)
is live too and is documented in [`child-interp.md`](child-interp.md).

| Form                                           | Action                                            |
|------------------------------------------------|---------------------------------------------------|
| ``interp alias {} new {} target ?arg…?``       | Register `Command::Alias`; result is ``new`` (refused when it would close a loop — §4.6) |
| ``interp alias {} new``                        | Result is the ``target ?arg…?`` list; ``alias "new" not found`` if it is not an alias |
| ``interp alias {} new {}``                     | Delete the binding; empty result                  |
| ``interp aliases ?path?``                      | Tcl list of every alias name in the addressed interp |
| ``interp target path alias``                   | The interp path (from this interp) to the interpreter ``alias`` resolves its target in: ``{}`` for a same-interp alias, ``path`` minus its last element for a child→parent one.  ``alias "X" in path "P" not found`` (`TCL LOOKUP ALIAS X`) when the addressed interp has no such alias |

### 4.4 Performance

Alias dispatch is warm-path (tcltest's cross-test thunks all hit it).
The cost is one `resolve` walk to find the target by name, one `Vec`
build for the synthesised argv, and one `invoke` recursion — the enum
match that selects the trampoline is free.

By-name resolution on every dispatch is not an oversight to be
optimised away: it is exactly what makes the alias observe target
deletion lazily, which is the C semantics.  Any future cached-target
slot has to be invalidated on every table mutation to stay correct, so
it is deliberately not taken on speculation.

### 4.5 Child→parent aliases

``interp alias childPath name {} target ?arg…?`` installs a
`Command::ParentAlias` in the *child*'s table.  `dispatch_parent_alias`
upgrades the child's `Weak` parent handle, builds the same
``[target, *prefix, *caller_tail]`` argv, and dispatches it **through
the parent handle** — a plain nested native call, mirroring C's
``Tcl_EvalObjv(parentInterp, …)`` on one shared C stack.  The parent's
result is copied back into the child and the completion code
propagates.

Two guards sit on that path:

- Invoking a parent alias from the root interpreter (no parent to
  upgrade) errors with ``cannot invoke a parent alias from the root
  interpreter``.
- `CROSS_INTERP_DEPTH` bounds the nesting; exceeding
  `MAX_CROSS_INTERP_DEPTH` raises ``too many nested cross-interpreter
  calls``.  The recursion itself is sound by construction — each interp
  is an `Rc<InterpState>` reached through a cloned handle with
  per-field interior mutability, so a re-entry shares state rather than
  aliasing a `&mut` — and the counter only caps native-stack growth.

Two forms are not implemented: querying a child alias
(``interp alias childPath name`` reports ``querying a child alias is
not yet supported``), and any non-empty *target* path, which reports
the ``only single-interp aliases`` error from §1.

### 4.6 Alias loops (`TclPreventAliasLoop`)

``interp alias {} a {} b`` followed by ``interp alias {} b {} a`` is a
closed cycle: dispatching either one would trampoline for ever.  C
refuses the alias that closes it, at *definition* time, in
`TclPreventAliasLoop` (``tclInterp.c``); this runtime does the same, and
so does the bytecode VM (issue #1447 — before the fix the pair installed
happily and recursed natively until the stack ran out).

The algorithm is C's, ported: **create first, walk the chain, roll back
on a hit.**

- `Interp::install_alias` binds the `Command::Alias` through
  `Namespaces::register_at`, which reports the ``(namespace, simple
  name)`` it bound at, then calls `Namespaces::alias_chain_loops` on
  that binding.  Each hop resolves the alias's stored target name
  anchored at the global namespace — exactly what dispatch does, so the
  walk follows the chain dispatch would take.  An unresolvable target
  ends the walk (legal: aliases late-bind), a non-alias target ends it,
  and a hop landing back on the binding we started from is the loop.
- On a loop the binding is removed again and the command reports
  ``cannot define or rename alias "b": would create a loop``
  (``-errorcode TCL OPERATION INTERP ALIASLOOP``), naming the alias by
  its *simple* command name the way C's `Tcl_GetCommandName` does.
  Because the alias is bound before the check, a refused definition also
  destroys the command it displaced — ``proc x …; interp alias {} x {}
  x`` leaves no ``x`` at all.  C does not restore it either, and that is
  tclsh 8.6.16 / 9.0.4-pinned.
- The walk terminates because every alias already in the table passed
  this same gate, so no chain can hold a pre-existing cycle; a visited
  list bounds it regardless.

`rename` is gated too, since moving an alias onto a name its own target
chain resolves back to closes the same cycle
(``interp alias {} a {} b; rename a b``).  The chain can only close on
the alias once the alias is visible at its *destination*, so
`Namespaces::rename_creates_alias_loop` makes the move tentatively,
walks, and undoes it — restoring any command the tentative move
displaced — leaving `Interp::rename_command` to perform the real rename
only when the answer is "no loop".  The refusal therefore happens before
anything observable: no rename trace fires for a rename that does not
take place, and the table is byte-for-byte unchanged (``a`` survives,
``b`` stays free).  `RenameOutcome::AliasLoop` carries that answer up to
the `rename` builtin, which formats the same message.

Cross-interp (`ParentAlias`) bindings are not walked: a child-side alias
targets a command in the *parent*, and this runtime has no parent-side
alias into a child, so the alias graph cannot leave an interpreter and
come back.  Genuine parent⇄child re-entrancy is real recursion, not a
name cycle, and is bounded by `CROSS_INTERP_DEPTH` (§4.5).

## 5. Compiler surface

The compiler has no per-command table entry for ``rename`` or
``interp``, and no stub that traps them.  Both are ordinary commands
that reach the runtime through generic dispatch.

What the compiler does instead is *reason about them*:
`tcl_compiler::command_binding::scan_module_command_mutations` builds a
whole-module summary of every command-table transition a script
performs — ``rename``, ``interp alias``, ``namespace import``, and the
dynamic forms.  `ModuleCommandMutations::trusts` then gates builtin
assumptions and `trusts_proc_binding` gates direct procedure calls, per
name.  A transition with literal arguments distrusts only the names it
names; an unbounded one sets the wildcard state and distrusts every
binding, which makes the selector fall back to generic argv dispatch
against the live runtime table.

## 6. Test strategy

Two layers:

1. **Unit tests co-located with the implementation** —
   `runtime/rust/src/cmd_alias.rs`'s own `mod tests` exercises alias create /
   query / delete, the dispatch trampoline, and the alias-loop gate
   (mutual pair, self-alias, longer cycle, rename-into-a-loop, and the
   legitimate chains that must still work) directly against a live
   interpreter; `interp.rs`'s `mod tests` plants a cycle straight into
   the command table to prove the dispatch bound catches what the gate
   cannot see.
2. **Oracle-pinned integration tests** —
   `runtime/rust/tests/rename_interp_semantics.rs` runs whole `rename` /
   `interp` sheets through a live `Interp` and asserts the exact bytes a
   pinned `tclsh` produced, message and ``-errorcode`` together.  Each
   test quotes the sheet it pins, so a reader can paste it into a real
   tclsh and re-derive the expectation.
3. **Upstream `.test` coverage** (``rename.test``, the single-interp subset of
   ``interp.test``) runs through the tcltest harness; where it sits on the
   capability ladder is [`tcl-test-tiers.md`](tcl-test-tiers.md), and the
   per-stem numbers are [`rust-vm-tier-parity.md`](rust-vm-tier-parity.md).

## 7. Implementation map

| Piece | Where |
|---|---|
| `rename` and the `interp` ensemble entry point | `runtime/rust/src/cmd_alias.rs` |
| `Command` enum, `dispatch_alias`, `dispatch_parent_alias`, `rename_command` | `runtime/rust/src/interp.rs` |
| The table operations (`Namespaces::rename` / `delete` / `register` / `resolve` / `retarget_imports` / `alias_names`) | `runtime/rust/src/namespace.rs` |
| The alias-loop gate (`Namespaces::alias_chain_loops` / `rename_creates_alias_loop`) | `runtime/rust/src/namespace.rs` |
| Command traces fired on rename and delete | `runtime/rust/src/cmd_trace.rs` |

The bytecode VM (`rust/tcl-vm`) implements the same two commands with a
comparable `Command` enum (`Alias(Rc<Vec<Value>>)` and a `CrossAlias`
carrying an `InterpId`), and runs the same ``TclPreventAliasLoop`` gate
on alias creation and rename (§4.6), walking its alias graph across the
whole interpreter tree by `InterpId` because its aliases really can span
interpreters.  On the two points this document used to record as
VM-only, the engines now agree: both refuse a rename onto an occupied
destination (``can't rename to "X": command already exists``) and both
re-home a renamed proc, so ``namespace current`` inside the body reports
the destination.

The divergences that remain are the VM's, and belong to `rust/tcl-vm`
rather than here:

- its ``interp`` option list advertises ``cancel`` and ``target`` with
  no arm behind either, and accepts ``share`` / ``transfer`` as silent
  no-ops (this runtime implements ``target`` and advertises neither of
  the other three — §1);
- its ``$child`` option list advertises ``transfer``, which C's child
  command object does not, and omits ``debug``, which it does;
- its ``interp invokehidden`` has no ``-global`` at all and skips
  ``-namespace ns`` and any unknown flag silently, instead of switching
  evaluation context and refusing
  ([`command-introspection.md`](command-introspection.md) §2.5).
