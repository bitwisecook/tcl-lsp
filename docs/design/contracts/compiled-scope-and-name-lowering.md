# Contract: scope class & qualified-name lowering in the AOT compiler

> How the AOT/WASM compiler decides *where a name lives* and *when a construct
> must run through the interpreter*. The semantic model it mirrors is
> [runtime-variable-frame-model.md](runtime-variable-frame-model.md) and
> [command-resolution.md](command-resolution.md); the eval hand-off is
> [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md).

## The decision the compiler has to make

The runtime resolves a name to a cell through a *frame → name → cell*
indirection (see the frame model). The **compiler** has to make the mirror
decision *statically*: for each variable reference and each binding construct,
which scope class is it — a frame local that can become a WASM slot, a
qualified namespace variable, or a global — and, crucially, *can this whole
construct even be compiled, or must it be handed to the interpreter?*

Both halves are explicit lowering **outputs**, never emergent properties of an
incidental encoding (an IR node type, a shared registry flag). Scope class is
an attribute computed once per reference; "compile inline" vs "emit a real
call to the interpreter/runtime" is an outcome attached to the node that no
downstream pass may flip.

## The three scope classes (compiler view)

| Class | Surface form | Lowers to |
|---|---|---|
| **Frame local** | unqualified `x` inside a proc/apply/method body | the frame name table; *may* be promoted to a raw slot behind a proof (below) |
| **Qualified namespace var** | `::a::b::x`, or unqualified after `variable x` | a namespace-var access against the resolved namespace table |
| **Global** | unqualified `x` at *script top level*; `::x` anywhere | the global table |

The single most common surprise (and a hard contract): **unqualified inside a
proc is a frame local, not the defining namespace's variable.** At script top
level the call frame *is* the global table, so unqualified `x` is `::x`. The
compiler must encode exactly this split — it cannot treat "unqualified" as one
thing.

## Local-slot promotion is an optimisation behind a proof

Default: every variable reference compiles against the frame name table. A
local is promoted to a raw WASM slot **only** when the compiler proves, for
that proc, that the name is never:

* `upvar` / `global` / `variable`-aliased,
* used as an array,
* traced (`trace add variable`),
* reachable by an `uplevel` / `eval` / `apply` / dynamic dispatch in the body
  (which could read or mutate it by name), or
* `unset` and then re-introspected in a way the slot model can't represent.

The proof is fragile; **guard it and fall back to the table.** Promotion is
never the base case.

## When the construct must run through the interpreter

Some constructs are *defined* to fall back to a generic invoke. The canonical
example: tclsh 9 compiles `foreach`/`lmap` inline, **but drops to a generic
invoke when a loop variable is namespace-qualified** (`foreach ::v …`). The
compiler mirrors that, under two rules:

1. **"Emits nothing" is a property of a node instance, never of a command
   spelling.** The inlined `foreach` lowering plants a *synthetic header
   def-marker* call that is intentionally a no-op, because the loop body is
   emitted structurally. That must not be expressed as a target-specific
   shared-registry trait keyed on the command name, or the same table also
   swallows the *genuine* opaque `foreach` invoke that the qualified-var
   fallback emits — and the loop then runs zero times. The same command can be
   both a structural marker and a real call in the same unit.

2. **Reconstruction is token-faithful.** When a construct falls back to the
   interpreter, the compiler rebuilds a script string and `eval`s it.
   Rebuilding from brace-stripped IR strings pre-substitutes `\n`, mis-parses
   a body that starts with `[` as a command substitution, and corrupts braced
   varLists. Any IR node that can fall back to eval therefore carries the
   **original command tokens** and reconstructs from them, so braces and
   quoting on varLists, lists, and bodies round-trip byte-for-byte. `tokens`
   is a uniform, non-optional field on fallback-capable nodes (`catch`,
   `while`, `for`, `switch`, `foreach`/`lmap`), not a per-node retrofit.

**Design rule:** "this construct is handled by the interpreter" is a distinct
IR node kind (a *barrier*), not a normal call that happens to be named like a
control structure. A barrier is immune to call-site optimisations
(emits-nothing, inlining, constant folding) by construction.

## Introspection coherence is part of lowering

`info exists` / `info vars` / `info locals` / `info level` / `trace info` /
`array exists` are **runtime queries against the live cell table**, not pure
functions the compiler may fold. A compile-time answer derived from "x was
assigned in this unit" that `unset` does not invalidate makes `set x 1;
unset x; info exists x` report `1` — the variable is correctly gone (a read
errors), only the introspection lies.

**Rule:** any compile-time liveness/const map MUST be invalidated by `unset`,
`upvar`, `global`, `variable`, `trace add variable`, and every
`eval`/`uplevel`/dispatch boundary. When in doubt, lower these introspection
commands to real runtime queries; never constant-fold them.

## Contract vs. incompatible-by-design

| Behaviour | Class | Notes |
|---|---|---|
| Unqualified = frame local in a proc; = global at top level | **Contract** | The defining split; tests depend on it. |
| Qualified loop var (`foreach ::v …`) ⇒ generic invoke, run once per element | **Contract** | Must not be swallowed by an emits-nothing fast-path. |
| `{*}` / unbounded arg expansion across the inline↔interp boundary | **Contract** | Reconstruction must be token-faithful. |
| Braces/quoting on eval-fallback varLists / lists / bodies preserved verbatim | **Contract** | Carry original tokens; don't rebuild from stripped IR. |
| `info exists/vars/locals`, `trace info`, `array exists` reflect live state | **Contract** | Never compile-time-folded; invalidate on scope-affecting ops. |
| `info frame` line/PC tables, bytecode disassembly, `info cmdcount` | **Incompatible-by-design** | Tied to C Tcl bytecode (W9-internal). |
| Which constructs C *chooses* to bytecode-compile vs invoke | **Internal** | Mirror the *observable* result (run-once, correct binding), not C's codegen. |

## See also

- [runtime-variable-frame-model.md](runtime-variable-frame-model.md) — the
  cell/frame indirection this lowering mirrors statically.
- [command-resolution.md](command-resolution.md) — qualified-name resolution rules.
- [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — the eval hand-off a barrier node routes through.
- [variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md)
  — why trace presence defeats slot promotion and why introspection is live.
