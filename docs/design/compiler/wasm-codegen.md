# WASM codegen pipeline

Walkthrough of how a Tcl script becomes a `tcl_runtime.wasm`-linkable
module, end to end.  Links into the actual files so a contributor can
follow the flow.

## Entry point

`core/compiler/codegen/wasm/__init__.py::wasm_codegen_module(ir_module)`
produces a `WasmModule` (defined in `_ir.py`) from an `IRModule`.
That's the top of the call graph — every other file below executes
under this call.

## The six phases

### Phase 0 — Var-escape analysis (per proc)

`core/compiler/var_escape/` runs before codegen starts.  For each
`IRProcedure` it identifies which local variables escape the frame
(via `upvar`, `variable`, dynamic `$name` reads, etc.) and tags them
`FRAME`.  The WASM emitter later routes those reads/writes through
`tcl_local_set` / `tcl_local_get` instead of a plain WASM local slot.

### Phase 1 — Pre-scan IR for runtime imports

`_scan.py::_scan_needed_imports()` walks the full IR tree once —
every `IRCall`, embedded `[cmd]` substitution, `expr` operator, and
statement-level side effect — and accumulates the set of Zig runtime
imports the emitter will need (`tcl_puts`, `tcl_list_index`, `tcl_arith_add`,
…).  This is a read-only pass; no code emitted yet.

The scan reads import keys from two sources:
1. `runtime_import_for(command)` — walks `CommandSpec.wasm_runtime_import`
   on every matching spec.
2. `subcommand_runtime_import_for(command, subcommand)` — same for
   `SubCommand.wasm_runtime_import`.

The legacy `_RUNTIME_IMPORTS` dict in `_imports.py` is still used for
infrastructure imports (obj lifecycle, arith ops, frame ops) that
don't have a command owner.

### Phase 2 — Register imports as WASM function indices

Back in `wasm_codegen_module()`, the set collected by Phase 1 is
allocated `funcref` indices in the `WasmModule.imports` section.  The
order matters — WASM function references are opaque indices into the
full `(imports ++ defined_funcs)` table, so every `call $idx`
instruction elsewhere in the module assumes this order.  The scan
phase produces a deterministic set; Phase 2 sorts it and assigns
indices `0..N-1`.

### Phase 3 — Build proc index + namespace-import tables

For each `IRProcedure` in the module, the emitter pre-allocates a
function index and collects `namespace import` / `namespace export` /
`namespace forget` declarations so the prologue of each proc can emit
the alias-setup calls.

### Phase 4 — Compile `::top` (the module's top-level script)

The script body is emitted into function 0 (exported as `::top`).
`_statements.py::_WasmEmitterStmtMixin._emit_stmt()` is the entry
point; it pattern-matches on `IRStatement` variants:

- `IRCall(command, args, defs)` — the main dispatch path (see below).
- `IRAssignConst` / `IRAssignValue` / `IRAssignExpr` — value-to-var
  store.
- `IRIf` / `IRWhile` / `IRFor` / `IRForeach` / `IRSwitch` / `IRTry`
  — control flow, handled by `_control_flow.py`.
- `IRReturn` — inlined proc return.
- `IRBarrier` — an IR node that forces the emitter to fall back to
  `tcl_eval()` for a specific command.

### Phase 5 — Compile each proc / method

Same path as Phase 4 but the prologue sets up per-proc state: frame
push, namespace push, argv materialisation (for `info level 0`), and
default-argument fill-in.

## Per-statement command dispatch

`_statements.py::_emit_call_stmt()` — called for every `IRCall` —
resolves in this order:

1. Synthetic markers: `<upvar-invalidate>`, `<cond>` (no emit).
2. `command_emits_nothing(command)` — scope-declaration commands
   (`global`, `upvar`, `variable`, `foreach`, `namespace eval` in
   some contexts).  Their side-effects are captured by the IR; the
   call itself emits nothing.
3. User-defined proc call — `_proc_index` lookup.
4. Registry hook — `REGISTRY.get_wasm_hook(command)` finds a
   `codegens["wasm"]` entry on the `CommandSpec` (or `SubCommand`
   via Phase B.8's `wasm_runtime_import` on SubCommand).  Hooks are
   registered by modules under `_emitter/cmds/` at import time (see
   `_emitter/__init__.py`'s `from . import cmds as _cmds`).
5. Specialised branches for `catch`, `break`, `continue`, bare
   `return`.
6. Generic runtime dispatch — `_emit_cmd_runtime(command, args, defs,
   context)` uses the `WasmRuntimeImport` on the spec (via
   `runtime_import_for(command)`) to find the target import, builds
   the arg stack, calls, and drops/stores/keeps the result via
   `_runtime_call_end(spec, defs, context)`.
7. Eval fallback — `tcl_eval(<script>)` for anything still
   unhandled.

## Per-command files

After Phases E.2 / E.3 each Tcl command's Python WASM codegen lives
in one file under `_emitter/cmds/`:

```
_emitter/cmds/
├── set_.py          # set, incr
├── return_.py       # return
├── list_.py         # list, lset, lassign, + helpers
├── string_.py       # string + subcommand dispatch
├── dict_.py         # dict + subcommand dispatch
├── info_.py         # info + its big inline impl
├── array_.py        # array subcmd helpers + array set literal
├── clock_.py        # clock subcommand dispatch
├── uplevel_.py      # uplevel, array, unset hooks + helpers
├── scope_.py        # global, upvar, variable
├── catch_.py        # catch
├── runtime_.py      # auto-registration loop over specs with
│                    #   CommandSpec.wasm_runtime_import
└── __init__.py      # imports all of the above for side-effect
                     #   registration
```

Each file:
1. Imports `REGISTRY` / `EmitContext` from the registry package.
2. Defines hook functions: `def _emit_cmd(emitter, args, defs, context)
   -> bool`.
3. Calls `REGISTRY.register_wasm_emitter("cmd", _emit_cmd)` at import
   time.
4. May also declare a small mixin class (`_CmdFooMixin`) exposing
   helper functions as methods — this is how per-command helpers
   like `_emit_info_value` reach the emitter without being defined
   on a central `_cmd_helpers.py` mixin.

## Runtime interop contract

Each spec that maps to a Zig runtime import declares a
`WasmRuntimeImport(import_key, argc, nontrapping, module, export_name,
params, results)` on its `CommandSpec.wasm_runtime_import` field (or
on the parent's `SubCommand.wasm_runtime_import` for sub-command
dispatchers).  The fields:

| field | meaning |
|---|---|
| `import_key` | Internal name used by the compiler to refer to the import (e.g. `"tcl_puts"`) |
| `argc` | Fixed argument count, or `None` for variadic |
| `nontrapping` | When `True`, skip the `tcl_diag_set` preamble (Zig side is total) |
| `module` | WASM import module (typically `"tcl"`) |
| `export_name` | Zig-exported symbol (defaults to `tcl_cmd_<key[4:]>`) |
| `params` | WASM value types — `("i32",)`, `("i64", "i32")`, etc. |
| `results` | WASM result types (`()` for void) |

The CI parity gate (`make check-wasm-parity`) cross-verifies every
CommandSpec's arity + subcommand arity against the Zig side's
`CmdEntry` / `SubEntry` arity fields.  Drift is impossible to merge.

## Reference map

| File | Role |
|---|---|
| `_emitter/__init__.py` | `_WasmEmitter` class composition — multi-mixin inheritance |
| `_emitter/_core.py` | Base class with state: locals, aliases, strings, imports |
| `_emitter/_statements.py` | Statement dispatch, `_emit_call_stmt` |
| `_emitter/_expressions.py` | `expr` language codegen |
| `_emitter/_values.py` | Value-context codegen (`[cmd]` in expressions) |
| `_emitter/_control_flow.py` | `if` / `while` / `for` / `foreach` / `switch` / `try` / `catch` |
| `_emitter/_variables.py` | `set` / `incr` / alias handling / frame escape routing |
| `_emitter/_commands.py` | Generic `_emit_cmd_runtime` + `_runtime_prep` / `_runtime_call_end` helpers + `_emit_cmd_proc_call` |
| `_emitter/_optimisation.py` | Constant folding, barrier short-circuiting |
| `_emitter/_ops.py` | Expr operator → WASM op translation |
| `_emitter/cmds/*.py` | Per-command hook registration + helpers |
| `_scan.py` | Pre-codegen IR scan for runtime imports |
| `_imports.py` | Infrastructure FFI signatures; `import_signature()` helper |
| `_ir.py` | WASM binary format, `ValType`, `WasmOp`, `WasmModule` |
| `_encoding.py` | LEB128, string interning, list quoting |
| `_parsing.py` | `_parse_array_ref` and other cheap string lookups |

### `_core.py` cohesion — why it stays as one file

After the Phase E per-command split, `_core.py` holds ~200 lines of
`__init__` state, the `generate()` prologue/body/epilogue pipeline
(~320 lines), and a handful of low-level emit helpers.  Decomposing
it further was considered during Phase E.4 and declined: the state
it owns — locals, strings, shared imports, proc metadata,
namespace context, diag map, escape summary — is needed by every
mixin, and `generate()` reads linearly through the proc prologue
(frame push, namespace set, argv build, default substitution,
param mirror) without natural seams.  Splitting would multiply
mixin interfaces without reducing real complexity.  The Phase E
per-command migration already achieved the main goal: individual
command behaviour lives in `cmds/<name>_.py`, not in a central
helpers mixin.

### Per-command hook file layout

Each file under `_emitter/cmds/` registers one or more
`(name, hook)` pairs via `REGISTRY.register_wasm_emitter`.  Hooks
receive `(emitter, args, defs, context)` and return `True` when
they handled the call — returning `False` falls through to the
next dispatch layer (generic runtime, eval fallback, trap).

Import order inside `cmds/__init__.py` matters: the generic
`runtime_.py` auto-registration loop runs last, skipping commands
that already have a specialised hook (first-writer-wins).  New
specialised hooks therefore only need to be imported in
`cmds/__init__.py` before `runtime_` to take effect.
