# Tail-call and recursion optimisation (O121–O123)

How the three self-recursion passes decide what is in tail position, which of
them fires for a given proc, and how priority resolves an overlap. Between
them they rewrite self-recursion to `tailcall` or to a loop, or hint that an
accumulator parameter would make it tail-recursive.

Three related passes detect and transform self-recursive patterns:

| Code | Summary | Kind |
|------|---------|------|
| O121 | Rewrite self-recursive tail call to `tailcall` | Source rewrite |
| O122 | Convert fully tail-recursive proc to iterative `while` loop | Source rewrite |
| O123 | Detect non-tail recursion eligible for accumulator introduction | Hint only |

O122 subsumes O121 — when a proc is fully tail-recursive (all self-calls are in tail position) and the loop conversion can be built, O122 fires with higher priority and O121 is suppressed by the non-overlapping selection mechanism. When only some self-calls are in tail position, O121 fires for the tail calls and O123 may fire for non-tail calls.

## When each code fires

1. **Tail position** — a self-call is in tail position if it is the last statement in the proc body, or the last statement in every branch of an `if`/`elseif`/`else` or `switch` at the end of the body. Calls inside `expr`, `catch`, `try`, loops, or nested command substitutions are never in tail position.

2. **O121 fires when** — for each self-call in tail position. `optimise_tail_calls` emits an O121 candidate at every tail-position self-call; these may later be suppressed when a higher-priority O122 covers the same range. The rewrite wraps the tail call with `tailcall`.

3. **O122 fires when** — every self-call in the proc is in tail position, the proc has at least one parameter, every tail site passes exactly one argument per parameter, and — for a proc with more than one parameter — the dialect has `lassign` (Tcl 8.5+). The entire proc body is rewritten to an iterative `while {1}` loop with parameter reassignment in place of each recursive call.

   Arguments are counted as Tcl words, so a bracketed argument such as `[expr {$n - 1}]` is **one** argument. A tail site that expands its arguments with `{*}`, or whose value holds more than one command, has no statically known arity and stands down to O121.

   Tail position is judged across the whole body, conditions included: a self-call in an `if` / `while` / `for` condition or a `switch` subject is not in tail position, and the loop body would still evaluate it recursively, so it blocks the conversion.

4. **O123 fires when** — exactly one non-tail self-call appears embedded in a return value (e.g. inside an `expr` or nested command substitution). This is a hint-only diagnostic; no source rewrite is produced. The hint indicates the recursion could be made tail-recursive by introducing an accumulator parameter. Doubly-recursive patterns (two or more self-calls in the same expression) are excluded.

5. **Priority** — `opt_priority` (`rust/tcl-compiler/src/optimiser/mod.rs`) returns 6 for O122 and 5 for O121 and O123. `select_non_overlapping` prefers O122 over O121 when both cover the same range.

6. **`hint_only` contract** — O123 sets `hint_only: true` on its `Optimisation`, which causes `apply_optimisations` to skip it during source rewriting while still emitting it as a diagnostic.

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

### Factorial with accumulator — every argument bracketed (O122)

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

O123 emits: *"Proc 'factorial' is a candidate for accumulator-style rewriting"* — the hint is that an accumulator parameter would put the recursive call in tail position, where O121 or O122 could take it.

No source rewrite is produced (`hint_only: true`).

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

- `rust/tcl-compiler/src/optimiser/tail_call.rs` — pass implementation
- `rust/tcl-compiler/src/optimiser/mod.rs` — `opt_priority`, the `Optimisation` struct's `hint_only` field
- `rust/tcl-compiler/src/optimiser/manager.rs` — pass invocation, `hint_only` skip in `apply_optimisations`
- `rust/tcl-compiler/src/optimiser/helpers/select.rs` — hint-only separation in `select_non_overlapping`

## Failure modes

- Range drift if `body_source` does not exactly match the text between proc body braces.
- False negative if a new control-flow statement (beyond `Statement::If` / `Statement::Switch`) is added without updating tail-position walking.
- `lassign` rewrite produces incorrect results if parameter default values change effective arity at runtime.

## Tests

- `rust/tcl-compiler/src/optimiser/tail_call.rs` unit tests

## See also

- [pass/fact ownership matrix](pass-fact-ownership-matrix.md)
- [downstream pass contracts](downstream-pass-contracts.md)
