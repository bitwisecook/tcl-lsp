# Var-escape analysis — keeping proc vars on WASM locals

## Symptom

A contributor needs to understand how the Tcl→WASM compiler decides whether a
proc-local Tcl variable stays in a WASM local slot (fast) or is spilled into
the runtime frame (slow but name-addressable by `uplevel`, `upvar`, `eval`,
and dynamic `set $name …`).

## Context

The WASM emitter (`compiler/codegen/wasm/`) emits proc-local Tcl variables
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
[`compiler/var_escape/`](../../../compiler/var_escape/) —
[`_types.py`](../../../compiler/var_escape/_types.py),
[`_propagation.py`](../../../compiler/var_escape/_propagation.py),
[`_cfg_propagation.py`](../../../compiler/var_escape/_cfg_propagation.py),
[`_info_subcommands.py`](../../../compiler/var_escape/_info_subcommands.py),
[`_interprocedural.py`](../../../compiler/var_escape/_interprocedural.py),
[`_slot_resolution.py`](../../../compiler/var_escape/_slot_resolution.py),
[`_api.py`](../../../compiler/var_escape/_api.py).

The slot-resolution pass (`_slot_resolution.py`) is a separate
follow-up: for procs whose body passes the by-name-eligibility check
it stamps a `{local_name: slot_index}` map on the summary, and the
WASM emitter routes those names through the runtime's
`tcl_frame_local_at(idx)` / `tcl_frame_local_set_at(idx)` indexed
accessors instead of the name-keyed `tcl_local_set` / `tcl_local_get`
calls.  See `runtime/zig/interp/tcl_frames.zig`'s
`frame_locals_array` for the runtime side.

Consumers:
[`compiler/codegen/wasm/`](../../../compiler/codegen/wasm/)
— the emitter package. Encoding helpers live in
[`_encoding.py`](../../../compiler/codegen/wasm/_encoding.py);
parsing helpers in
[`_parsing.py`](../../../compiler/codegen/wasm/_parsing.py);
the `_WasmEmitter` class and module-level code generation entry
points live in
[`__init__.py`](../../../compiler/codegen/wasm/__init__.py).

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
`SSAValueKey = (var_name, ssa_version)`. Codegen collapses this to
per-name storage (a single physical slot per Tcl variable) by taking
the join over all SSA versions — a variable is `FRAME` if any of its
definitions escapes. The per-SSA-version map stays on the
``ProcEscapeSummary`` as ``ssa_tags`` so future consumers (for
example a register allocator) can read the finer-grained result.

Two propagation drivers are available:

- ``_cfg_propagation.analyse_cfg_function(cfg, ssa, params)`` — the
  primary path. Walks the per-proc CFG in reverse-postorder, visits
  each ``SSABlock``, and tags specific ``(name, version)`` pairs at
  the statement that forces the escape. ``if`` / ``while`` branch
  conditions come off the ``CFGBranch`` terminator and get scanned
  for embedded ``[info …]`` hazards. Runs whenever a ``CompilationUnit``
  is available.

- ``_propagation.analyse_script(body, params)`` — the fallback tree
  walk used when only an ``IRModule`` is supplied. Produces identical
  per-name tags but cannot populate ``ssa_tags`` (no SSA versions
  available). Kept for tests and for callers that avoid compiling
  a full CFG.

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
[`compiler/ir.py`](../../../compiler/ir.py)).

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

Hook: `compiler/def_use.py` `DefUseResult`. No new value-tracking pass.

### `info` subcommand allow-list

Lives in
[`_info_subcommands.py`](../../../compiler/var_escape/_info_subcommands.py).

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
# compiler/var_escape/_api.py
def analyse_var_escape(
    source: str | None = None,
    cu: CompilationUnit | None = None,
    *,
    ir_module: IRModule | None = None,
    interprocedural: bool = True,
) -> dict[str, ProcEscapeSummary]:
    """Return per-proc escape summaries, keyed by qualified name.

    Exactly one of ``source``, ``cu``, or ``ir_module`` must be supplied
    (a ValueError is raised otherwise). When ``cu`` (or ``source``) is
    available the analysis runs the flow-sensitive CFG+SSA pass; the
    ``ir_module``-only path falls back to the IR tree walk.
    """
```

The result is not cached on `CompilationUnit`. The pass is cheap (a tree
walk plus a worklist fixpoint) and runs once per `wasm_codegen_module`
call. Callers that want to avoid re-running it can pass a pre-built
`escape_summaries` dict to `wasm_codegen_module`.

```python
@dataclass(frozen=True)
class ProcEscapeSummary:
    tags: dict[str, EscapeTag]  # per-var-name, collapsed over SSA versions
    dynamic_barrier: bool  # proc-pessimistic flag
    frame_needed: bool  # any FRAME var, or dynamic_barrier
    upvar_source_names: frozenset[str]  # transitive callee-upvar source set
    unbounded_upvar_source: bool  # any callee uses dynamic-source upvar
    direct_callees: frozenset[str]  # statically resolvable callees
    has_fallback: bool  # codegen will dispatch to tcl_eval
    has_call_fallback: bool  # raw call-shaped reasons (interproc-downgradable)
    ssa_tags: dict[SSAValueKey, EscapeTag]  # per-version, populated on the CFG path
```

### Emit-time contract

Codegen consumes `ProcEscapeSummary` through these rewired hooks:

Every hook lives in the emitter
([`compiler/codegen/wasm/_emitter/`](../../../compiler/codegen/wasm/_emitter/__init__.py));
the package layout is a single large class split only for readability.

- **`_intern_local(name)`**: non-parameter FRAME-tagged vars skip the
  ``_tcl_var_locals`` sync map entirely — their authoritative storage is
  the runtime frame, and keeping them out of the sync map means the
  narrow-sync and readback paths ignore them.
- **`_emit_var_read_obj(name)`** / **`_emit_var_write_obj(name)`**: FRAME
  vars route through the existing alias / frame-resolution helpers
  (``tcl_global_get``/``tcl_global_set`` via a stashed name object for
  upvar/variable aliases; future ``frames.var_set`` / ``frames.var_get``
  paths for caller-frame upvar). LOCAL vars take the WASM ``local.get`` /
  ``local.set`` fast path.
- **`_emit_frame_sync()`** / **`_emit_frame_readback()`**: iterate only
  the LOCAL-tagged entries of ``_tcl_var_locals``. FRAME vars are already
  in the frame, so no mirror or readback is needed.
- **Proc prelude** (``_WasmEmitter.generate``): when
  ``summary.frame_needed`` is False AND ``summary.has_fallback`` is False,
  skip ``frame_push``, the per-param ``tcl_local_set`` mirrors, and the
  ``frame_pop`` epilogue entirely — pristine procs pay zero frame
  overhead.

### Pessimistic degradation

When `summary.dynamic_barrier` is True:

- All vars are treated as `FRAME` for read/write emit.
- Frame sync degrades to today's behaviour: mirror every user-visible local
  (though most are already frame-only, so the sync is mostly a no-op).
- Proc prelude always pushes a frame.

### Interaction with existing passes

- **Taint** does not need to know about escape tags; it operates on SSA.
- **Side-effects** (`compiler/side_effects.py`) already classifies
  many of the same commands as `dynamic_barrier`. Var-escape reuses that
  flag as a fast pre-check — if side-effects says a command is a barrier,
  var-escape also marks it.
- **var_scoping** helpers (`compiler/var_scoping.py`) provide the
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
