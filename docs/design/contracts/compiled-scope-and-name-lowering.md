# Contract: scope class & qualified-name lowering in the AOT compiler

> **Status:** First-principles design contract (v2 / "if starting over").
> Describes how a from-scratch AOT/WASM compiler should decide *where a name
> lives* and *when a construct must run through the interpreter*, before
> writing any codegen. The semantic model it builds on is
> [runtime-variable-frame-model.md](runtime-variable-frame-model.md) and
> [namespace-model.md](namespace-model.md); the eval hand-off is
> [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md).

## Why this is the first thing to get right

The runtime resolves a name to a cell through a *frame → name → cell*
indirection (see the frame model). The **compiler** has to make the mirror
decision *statically*: for each variable reference and each binding construct,
which scope class is it — a frame local that can become a WASM slot, a
qualified namespace variable, or a global — and, crucially, *can this whole
construct even be compiled, or must it be handed to the interpreter?*

Every bug in this campaign that wasn't a runtime gap was a **lowering**
mistake: a `foreach ::v list body` that ran **zero iterations** because the
qualified loop variable forced a generic-invoke fallback that was then
silently dropped; an `info exists x` that returned a stale `1` after `unset x`
because compile-time local-liveness was never invalidated. The common root:
*scope class and "must interpret" were emergent properties of incidental
encodings (IR node type, a shared registry flag) instead of explicit,
designed lowering outputs.*

**Design rule:** scope class is an explicit attribute computed once per
reference; "compile inline" vs "emit a real call to the interpreter/runtime"
is an explicit lowering *outcome* attached to the node — never something a
downstream pass can flip by accident.

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
compiler must mirror that. Two failure modes this campaign hit, both to be
designed out:

1. **The "emits-nothing" trap.** The inlined `foreach` lowering plants a
   *synthetic header def-marker* call that is intentionally a no-op (the loop
   body is emitted structurally). An earlier design made that marker a no-op
   through a target-specific shared-registry trait. The trait then also
   swallowed a *genuine* opaque `foreach` invoke (the
   qualified-var fallback) → the loop emitted nothing and ran zero times.
   **Rule:** "emits nothing" is a property of a *specific synthetic node
   instance*, never of a command spelling in a shared table. The same command
   can be both a structural marker and a real call in the same unit.

2. **Lossy reconstruction.** When a construct falls back to the interpreter,
   the compiler rebuilds a script string and `eval`s it. Rebuilding from
   brace-stripped IR strings pre-substitutes `\n`, mis-parses a body that
   starts with `[` as a command substitution, and corrupts braced varLists.
   **Rule:** carry the **original command tokens** on any IR node that can
   fall back to eval, and reconstruct from them so braces/quoting on
   varLists, lists, and bodies round-trip byte-for-byte. (`catch`, `while`,
   `for`, `switch`, `foreach`/`lmap` all need this — make `tokens` a uniform,
   non-optional field on fallback-capable nodes rather than per-node retrofit.)

**Design rule:** model "this construct is handled by the interpreter" as a
distinct IR node kind (a *barrier*), not as a normal call that happens to be
named like a control structure. A barrier is immune to call-site
optimisations (emits-nothing, inlining, constant folding) by construction.

## Introspection coherence is part of lowering

`info exists` / `info vars` / `info locals` / `info level` / `trace info` /
`array exists` are **runtime queries against the live cell table**, not pure
functions the compiler may fold. The `set x 1; unset x; info exists x → 1`
bug was a compile-time answer derived from "x was assigned in this unit" that
`unset` never invalidated (the variable itself was correctly gone — a read
errored — only the introspection lied).

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
- [namespace-model.md](namespace-model.md) — qualified-name resolution rules.
- [parser-and-aot-interpret-boundary.md](parser-and-aot-interpret-boundary.md)
  — the eval hand-off a barrier node routes through.
- [variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md)
  — why trace presence defeats slot promotion and why introspection is live.
