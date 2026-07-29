# KCS: Proc argument trait inference

## Purpose

Proc argument trait inference determines how each parameter of a user-defined
proc is used inside its body.  This feeds optimisation, shimmer analysis,
taint propagation, and diagnostics with structured knowledge about parameter
flow.

## Trait types

| Trait | Meaning | Example pattern |
|-------|---------|-----------------|
| `EVAL` | Argument is eval'd as a script | `eval $script`, `uplevel 1 $body` |
| `BODY` | Argument is used as a loop/control body | `foreach item $list $body` |
| `VAR_WRITE` | Argument names a variable the proc writes | `upvar 1 $varName local; set local 42` |
| `VAR_READ` | Argument names a variable the proc reads via upvar without writing | `upvar 1 $varName local; return $local` |
| `EXPR` | Argument is evaluated as an expression | `if {$cond} {...}` |
| `LOOP_LIST` | Argument is used as the list in foreach/lmap | `foreach item $collection {...}` |

## Analysis tiers

### Shallow (synchronous)

`infer_param_traits(params, body_source)` scans top-level commands only.
Fast enough for real-time analysis during typing.  Detects direct patterns
where `$param` appears as an argument to a known command at the outermost
level of the proc body.

### Deep (asynchronous)

`infer_param_traits_deep(params, body_source)` recursively descends into
braced body arguments (foreach bodies, if bodies, etc.) to find traits
that the shallow pass misses.  For example:

```tcl
proc iterate {items body} {
    foreach item $items {
        uplevel 1 $body   ;# deep pass catches EVAL trait on $body
    }
}
```

The shallow pass only sees `$items` as `LOOP_LIST`.  The deep pass also
detects `$body` as `EVAL` inside the braced foreach body.

Recursion is bounded by `_MAX_DEPTH` (default 8) to prevent runaway
analysis on pathological input.

### Merging results

Use `merge_traits(shallow, deep)` to union the results from both passes.
The merged map is stored on `ProcDef.param_traits` and
`ProcSummary.param_traits`.

## Integration points

| Consumer | How traits are used |
|----------|-------------------|
| `Analyser._handle_proc()` | Calls shallow pass, stores on `ProcDef.param_traits` |
| `interprocedural.py` | Calls shallow pass on `IRProcedure.body_source`, stores on `ProcSummary.param_traits` |
| LSP hover | Can display trait annotations on proc parameters |
| Optimiser | Traits inform whether a proc can be inlined or constant-folded |
| Taint engine | `EVAL` and `BODY` traits mark taint-sensitive parameters |
| Shimmer analysis | `LOOP_LIST` and `VAR_WRITE` traits inform shimmer tracking |

## Data model

```python
class ProcArgTrait(Enum):
    EVAL = auto()
    BODY = auto()
    VAR_WRITE = auto()
    VAR_READ = auto()
    EXPR = auto()
    LOOP_LIST = auto()


# On ProcDef (analyser level):
param_traits: dict[str, frozenset[ProcArgTrait]]

# On ProcSummary (interprocedural level):
param_traits: dict[str, frozenset[ProcArgTrait]]
```

## Files

| File | Purpose |
|------|---------|
| `compiler/proc_arg_traits.py` | Trait inference (shallow + deep) |
| `analyser/semantic_model.py` | `ProcArgTrait` enum, `ProcDef.param_traits` |
| `compiler/interprocedural.py` | `ProcSummary.param_traits` |
| `tests/test_proc_arg_traits.py` | Unit tests |
