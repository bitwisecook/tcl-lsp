# KCS: Tail-call and recursion optimisation (O121–O123)

## Symptom

Tcl procs use direct self-recursion where an iterative form or `tailcall` would be more efficient. Stack depth grows linearly with input size, risking `too many nested evaluations` errors.

## Operational context

Three related passes detect and transform self-recursive patterns:

| Code | Summary | Kind |
|------|---------|------|
| O121 | Rewrite self-recursive tail call to `tailcall` | Source rewrite |
| O122 | Convert fully tail-recursive proc to iterative `while` loop | Source rewrite |
| O123 | Detect non-tail recursion eligible for accumulator introduction | Hint only |

O122 subsumes O121 — when a proc is fully tail-recursive (all self-calls are in tail position), O122 fires with higher priority and O121 is suppressed by the non-overlapping selection mechanism. When only some self-calls are in tail position, O121 fires for the tail calls and O123 may fire for non-tail calls.

## Decision rules / contracts

1. **Tail position** — a self-call is in tail position if it is the last statement in the proc body, or the last statement in every branch of an `if`/`elseif`/`else` or `switch` at the end of the body. Calls inside `expr`, `catch`, `try`, loops, or nested command substitutions are never in tail position.

2. **O121 fires when** — for each self-call in tail position. `optimise_tail_calls` emits an O121 candidate at every tail-position self-call; these may later be suppressed when a higher-priority O122 covers the same range. The rewrite wraps the tail call with `tailcall`.

3. **O122 fires when** — every self-call in the proc is in tail position. The entire proc body is rewritten to an iterative `while {1}` loop with parameter reassignment in place of each recursive call.

4. **O123 fires when** — exactly one non-tail self-call appears embedded in a return value (e.g. inside an `expr` or nested command substitution). This is a hint-only diagnostic; no source rewrite is produced. The hint indicates the recursion could be made tail-recursive by introducing an accumulator parameter. Doubly-recursive patterns (two or more self-calls in the same expression) are excluded.

5. **Priority** — `_OPT_PRIORITY` assigns O122=6, O121=5, O123=5. The non-overlapping selection prefers O122 over O121 when both cover the same range.

6. **`hint_only` contract** — O123 sets `hint_only=True` on its `Optimisation`, which causes `apply_optimisations` to skip it during source rewriting while still emitting it as a diagnostic.

## Examples

### GCD — tail-recursive `if`/`else` (O122)

**Before:**
```tcl
proc gcd {a b} {
    if {$b == 0} {
        return $a
    } else {
        return [gcd $b [expr {$a % $b}]]
    }
}
```

**After (O122 rewrite):**
```tcl
proc gcd {a b} {
    while {1} {
        if {$b == 0} {
            return $a
        } else {
            lassign [list $b [expr {$a % $b}]] a b

        }
    }
}
```

### Factorial with accumulator — tail-recursive (O122)

**Before:**
```tcl
proc fact {n acc} {
    if {$n <= 1} {
        return $acc
    }
    return [fact [expr {$n - 1}] [expr {$n * $acc}]]
}
```

**After (O122 rewrite):**
```tcl
proc fact {n acc} {
    while {1} {
        if {$n <= 1} {
            return $acc
        }
        lassign [list [expr {$n - 1}] [expr {$n * $acc}]] n acc
        continue
    }
}
```

### Linked-list traversal — bare tail call (O122)

**Before:**
```tcl
proc loop {xs} {
    set x [lindex $xs 0]
    puts $x
    loop [lrange $xs 1 end]
}
```

**After (O122 rewrite):**
```tcl
proc loop {xs} {
    while {1} {
        set x [lindex $xs 0]
        puts $x
        set xs [lrange $xs 1 end]
        continue
    }
}
```

### Factorial without accumulator — non-tail recursion (O123 hint)

```tcl
proc factorial {n} {
    if {$n <= 1} { return 1 }
    return [expr {$n * [factorial [expr {$n - 1}]]}]
}
```

O123 emits: *"Non-tail recursion in factorial could be eliminated by introducing an accumulator parameter and using tailcall."*

No source rewrite is produced (`hint_only=True`).

### Fibonacci — doubly recursive (no optimisation fires)

```tcl
proc fib {n} {
    if {$n <= 1} { return $n }
    return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]
}
```

Neither O121 nor O122 fires because neither call is in tail position. O123 does not fire because the expression contains two self-calls — the accumulator hint requires exactly one embedded self-call.

## Parameter reassignment strategy

- **Single parameter**: `set param $newval`
- **Multiple parameters**: `lassign [list $arg1 $arg2 ...] param1 param2 ...` — avoids evaluation-order bugs where reassigning `a` before reading the old `a` for `b` would corrupt the value.

## File-path anchors

- `core/compiler/optimiser/_tail_call.py` — pass implementation
- `core/compiler/optimiser/_types.py` — `_OPT_PRIORITY` entries, `hint_only` field
- `core/compiler/optimiser/_manager.py` — pass invocation, `hint_only` skip in `apply_optimisations`
- `core/compiler/optimiser/_helpers.py` — hint-only separation in `_select_non_overlapping_optimisations`

## Failure modes

- Range drift if `body_source` does not exactly match the text between proc body braces.
- False negative if a new control-flow IR node (beyond `IRIf`/`IRSwitch`) is added without updating tail-position walking.
- `lassign` rewrite produces incorrect results if parameter default values change effective arity at runtime.

## Test anchors

- `tests/test_optimiser.py::TestTailCallOptimisation`

## Discoverability

- [compiler KCS index](README.md)
- [pass/fact ownership matrix](kcs-pass-fact-ownership-matrix.md)
- [downstream pass contracts](kcs-downstream-pass-contracts.md)
