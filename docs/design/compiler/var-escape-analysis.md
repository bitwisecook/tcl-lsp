# Var-escape analysis — keeping proc vars on WASM locals

## Symptom

A contributor needs to understand how the Tcl→WASM compiler decides whether a
proc-local Tcl variable stays in a WASM local slot (fast) or is spilled into
the runtime frame (slow but name-addressable by `uplevel`, `upvar`, `eval`,
and dynamic `set $name …`).

## Context

The WASM emitter (`core/compiler/codegen/wasm/`) emits proc-local Tcl variables
as WASM `local.get` / `local.set` for speed. Before this pass existed, every
compiled proc paid a **sync-all-locals-before-fallback** cost on every
interpreter dispatch: `_emit_frame_sync` mirrored every local into the runtime
frame, then `_emit_frame_readback` mirrored them back. Procs that never
invoked the interpreter paid the cost anyway, because the emitter could not
prove their locals were private.

Var-escape analysis answers the question **"could this variable ever be
observed by name across a frame boundary?"** at compile time. Vars proved
private stay on WASM locals; vars that might escape live in the runtime
frame from the start, so no sync is needed.

Source:
[`core/compiler/var_escape/`](../../../core/compiler/var_escape/) —
[`_types.py`](../../../core/compiler/var_escape/_types.py),
[`_propagation.py`](../../../core/compiler/var_escape/_propagation.py),
[`_info_subcommands.py`](../../../core/compiler/var_escape/_info_subcommands.py),
[`_api.py`](../../../core/compiler/var_escape/_api.py).

Consumers:
[`core/compiler/codegen/wasm/`](../../../core/compiler/codegen/wasm/)
— the emitter package. Encoding helpers live in
[`_encoding.py`](../../../core/compiler/codegen/wasm/_encoding.py);
parsing helpers in
[`_parsing.py`](../../../core/compiler/codegen/wasm/_parsing.py);
the `_WasmEmitter` class and module-level code generation entry
points live in
[`__init__.py`](../../../core/compiler/codegen/wasm/__init__.py).

## Content

### Lattice

```
LOCAL  ⊑  FRAME
(bottom)    (top)
```

- `LOCAL` — the variable is only accessed by name in statically resolved
  positions; the WASM local slot is the single source of truth.
- `FRAME` — the variable must live in the runtime frame so the interpreter
  (or an `upvar` alias) can read or write it by name.

Join operator: `FRAME` dominates. A var is `FRAME` if **any** CFG path
escapes it.

The analysis is flow-sensitive over the per-proc CFG+SSA: keyed by
`SSAValueKey = (var_name, ssa_version)`. Codegen collapses this to per-name
storage (a single physical slot per Tcl variable) by taking the join over all
SSA versions — a variable is `FRAME` if any of its definitions escapes.

### Proc-level pessimistic fallback

The per-var tagging is not always sufficient. Some constructs defeat
analysis: the compiler cannot enumerate which vars might be accessed by
name. In those cases, the whole proc is marked **pessimistic** and every var
is forced to `FRAME`. This is strictly weaker than the pre-analysis
"sync-everything" behaviour (it still avoids the sync-back on every
fallback, because FRAME vars are already in the frame) but equivalent in
name-resolution cost.

A proc is pessimistic when **any** of its IR statements trigger these rules:

| Construct | Why pessimistic |
|-----------|-----------------|
| `uplevel 1 …` / `uplevel $dynamic …` | Caller frame can name-read our locals |
| `eval $dynamic_body` | Can contain arbitrary `$name` references |
| `{*}$dynamic` expansion inside an unknown `IRCall` | Can expand into `upvar`/`uplevel`/`set $name` |
| `info level`, `info frame`, `info vars`, `info locals` | Introspect the frame by name |
| `set $dynamic_name …` (and `incr`/`append`/`lappend`/`unset` siblings) whose name cannot be bounded by alias inference | Any var might be the target |
| Unknown or unhandled `IRCall` reaching the generic fallback | Interpreter can see the full frame |

A proc that does not trigger any of these rules, yet uses `upvar 1` or
`global` to bind specific names, escapes only the **named** vars — not the
whole proc.

### Transfer functions

Transfer functions run over IR statements (see
[`core/compiler/ir.py`](../../../core/compiler/ir.py)).

| IR shape | Effect |
|----------|--------|
| `IRAssignConst(var=X, …)` | no escape |
| `IRAssignExpr(var=X, …)` | no escape |
| `IRAssignValue(var=X, …)` | no escape |
| `IRIncr(var=X, …)` with literal `X` | no escape |
| `IRCall(command="global", …)` | escape listed vars |
| `IRCall(command="variable", …)` | escape listed vars |
| `IRCall(command="upvar", args=("#0" \| "0", src, X, …))` | escape local-side vars, source is global |
| `IRCall(command="upvar", args=(level_literal_positive, src, X, …))` | escape local-side vars, source is caller frame (still name-bounded) |
| `IRCall(command="upvar", args=(dynamic_level, …))` | proc-pessimistic |
| `IRCall(command="set" \| "incr" \| "append" \| "lappend" \| "unset", args=(dynamic_name, …))` | see alias inference |
| `IRCall(command="info", args=(subcmd, …))` | see `info` subcommand table |
| `IRBarrier(command="eval", args=(literal_body,))` | recurse into `body` via `VarReferenceScanner` to bound the ref set; literal refs escape |
| `IRBarrier(command="eval", args=(dynamic,))` | proc-pessimistic |
| `IRBarrier(command="uplevel", args=("#0" \| "0", literal_body))` | recurse into body at global scope — no local escape |
| `IRBarrier(command="uplevel", args=(other, …))` | proc-pessimistic |
| `IRIf` / `IRFor` / `IRWhile` / `IRForeach` / `IRSwitch` / `IRTry` | recurse into clauses |
| `IRCatch(result=R, options=O, body=…)` | escape `R` and `O` only if they are accessed by name elsewhere; recurse into body |
| `IRCall(command=unknown, …)` with `{*}` expansion | proc-pessimistic |
| `IRCall(command=unknown, …)` without `{*}` | not pessimistic by itself; side effects are contained by the call's `defs`/`reads` metadata |

### Alias inference for dynamic var names

A command like `set $name $value` can target any variable whose name equals
the runtime value of `$name`. The cheap inference rule:

1. Locate the SSA definition that reaches the use of `$name`.
2. If it is a single `IRAssignConst(var=name, value=<valid-ident>)` and
   `<valid-ident>` is a legal Tcl variable name, treat the dynamic `set` as
   if it targeted that literal. Escape only that name.
3. Otherwise, escape **every var in the proc** (not proc-pessimistic — a
   var can still be proven `LOCAL` if it is unreachable from this branch on
   all paths, which is why the analysis is flow-sensitive).

Hook: `core/compiler/def_use.py` `DefUseResult`. No new value-tracking pass.

### `info` subcommand allow-list

Lives in
[`_info_subcommands.py`](../../../core/compiler/var_escape/_info_subcommands.py).

| Subcommand | Escape behaviour |
|------------|------------------|
| `info body` | safe |
| `info args` | safe |
| `info default` | safe |
| `info commands` | safe |
| `info procs` | safe |
| `info class` | safe |
| `info functions` | safe |
| `info exists <literal>` | escape only the literal name |
| `info exists $dynamic` | proc-pessimistic |
| `info vars` / `info vars ?pattern?` | proc-pessimistic (enumerates frame) |
| `info locals` / `info locals ?pattern?` | proc-pessimistic |
| `info level` / `info level N` | proc-pessimistic (reveals caller args) |
| `info frame` / `info frame N` | proc-pessimistic (exposes frame by level) |
| `info patchlevel` / `info tclversion` / `info nameofexecutable` | safe |
| `info script` / `info library` / `info hostname` | safe |
| Any other subcommand | proc-pessimistic (unknown — assume worst) |

### Public API

```python
# core/compiler/var_escape/_api.py
def analyse_var_escape(cu: CompilationUnit) -> dict[str, ProcEscapeSummary]:
    """Return per-proc escape summaries, keyed by qualified name."""
```

Cached on `CompilationUnit` alongside the per-procedure analysis slot used by
taint and memory-SSA. Reuses the interprocedural fixed-point driver pattern
from `core/compiler/taint/_interprocedural.py`.

```python
@dataclass(frozen=True)
class ProcEscapeSummary:
    tags: dict[str, EscapeTag]          # per-var-name, collapsed over SSA versions
    dynamic_barrier: bool               # proc-pessimistic flag
    frame_needed: bool                  # True if any FRAME var exists or dynamic_barrier is set
```

### Emit-time contract

Codegen consumes `ProcEscapeSummary` through these rewired hooks:

- **`_intern_local(name)`** (`wasm/emitter.py`): if
  `summary.tags[name] is FRAME`, record the var in `self._frame_only_vars`
  and do not allocate a WASM local slot.
- **`_emit_var_read_obj(name)`** (`wasm/var_emit.py`): if the var is
  frame-only, emit `frames.var_resolve(name_obj)` (or `frames.local_get` for
  proc-private lookups that skip the global fallthrough). Otherwise take the
  existing fast path (`local.get <slot>`).
- **`_emit_var_write_obj(name)`** (`wasm/var_emit.py`): symmetric — emit
  `frames.var_set(name_obj, val_obj)` for frame-only vars, `local.set <slot>`
  for the fast path.
- **`_emit_frame_sync()`** (`wasm/frame_sync.py`): sync only vars tagged
  `LOCAL`. Frame-only vars are already in the frame; the fallback reads and
  writes them in place.
- **`_emit_frame_readback()`** (`wasm/frame_sync.py`): readback only vars
  tagged `LOCAL`. Frame-only vars need no readback.
- **Proc prelude** (`wasm/emitter.py`): if `summary.frame_needed` is False
  and the proc body has no IRBarrier or unknown IRCall, skip `frame_push` /
  `frame_pop` entirely. (Second-phase optimisation, flag-gated.)

### Pessimistic degradation

When `summary.dynamic_barrier` is True:

- All vars are treated as `FRAME` for read/write emit.
- Frame sync degrades to today's behaviour: mirror every user-visible local
  (though most are already frame-only, so the sync is mostly a no-op).
- Proc prelude always pushes a frame.

### Interaction with existing passes

- **Taint** does not need to know about escape tags; it operates on SSA.
- **Side-effects** (`core/compiler/side_effects.py`) already classifies
  many of the same commands as `dynamic_barrier`. Var-escape reuses that
  flag as a fast pre-check — if side-effects says a command is a barrier,
  var-escape also marks it.
- **var_scoping** helpers (`core/analysis/var_scoping.py`) provide the
  index tables for `global`, `variable`, and `upvar` declarations. Reused
  directly.

### Why flow-sensitive

A proc like:

```tcl
proc f {x} {
    if {$x > 0} {
        upvar 1 outer y
        set y $x
    }
    set local_only 1
    return $local_only
}
```

is perfectly safe: `local_only` never escapes, even though `y` does. A
flow-insensitive analysis would see the `upvar` anywhere in the body and
conservatively escape every SSA use of every var. Flow-sensitive keying by
`SSAValueKey` lets codegen keep `local_only` in a WASM local slot while
routing `y` through the frame.

## See also

- [`ir-types-lowering.md`](./ir-types-lowering.md) — IR node shapes.
- [`cfg-construction.md`](./cfg-construction.md) — how the CFG is built.
- [`ssa-construction.md`](./ssa-construction.md) — SSA for flow-sensitive keying.
- [`taint-analysis.md`](./taint-analysis.md) — template for lattice-based
  interprocedural analysis.
- [`wasm-runtime-primitives.md`](./wasm-runtime-primitives.md) — the
  `frame_*` / `local_*` primitives this analysis's codegen hooks call.
- [`side-effects-system.md`](./side-effects-system.md) — how
  `dynamic_barrier` is already defined for commands.
