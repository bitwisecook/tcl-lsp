# KCS: O125 — Code sinking into decision blocks

## Summary

O125 sinks side-effect-free variable assignments into the deepest
decision block (``if``/``switch``) that uses them.  This reduces
unnecessary work on execution paths where the variable is never
read.

## Disabling O125

O125 can be disabled at three levels:

| Method | Scope | How |
|--------|-------|-----|
| **VS Code setting** | Workspace / user | Set `tclLsp.optimiser.O125` to `false` in `settings.json` |
| **Inline suppression** | Single line / command | Add `# noqa: O125` on the line containing the assignment |
| **LSP ``didChangeConfiguration``** | Session-wide | Send `tclLsp.optimiser.O125 = false` in the settings payload |

## Examples

### 1. Basic sinking into ``if`` body

**Before:**

```tcl
set b foo
if {$a} {
    puts $b
}
```

**After:**

```tcl
# [O125] set b foo → sunk into if body
if {$a} {
    set b foo
    puts $b
}
```

The assignment ``set b foo`` is only used inside the ``if`` body, so it
is moved there.  When ``$a`` is false the assignment never executes.

### 2. Sinking into both branches

When the variable is used in every branch, the assignment is duplicated
into each:

**Before:**

```tcl
set b foo
if {$a} {
    puts $b
} else {
    puts $b
}
```

**After:**

```tcl
# [O125] set b foo → sunk into if body
if {$a} {
    set b foo
    puts $b
} else {
    set b foo
    puts $b
}
```

### 3. Selective branch sinking

When the variable is used in only some branches, it is inserted only
where needed:

**Before:**

```tcl
set b foo
if {$a} {
    puts $b
} else {
    puts hello
}
```

**After:**

```tcl
# [O125] set b foo → sunk into if body
if {$a} {
    set b foo
    puts $b
} else {
    puts hello
}
```

### 4. Deep sinking into nested decision blocks

O125 recursively descends to place the assignment at the deepest level:

**Before:**

```tcl
set b foo
if {$a} {
    if {$c} {
        puts $b
    }
}
```

**After:**

```tcl
# [O125] set b foo → sunk into if body
if {$a} {
    if {$c} {
        set b foo
        puts $b
    }
}
```

### 5. Sinking into ``switch`` arms

**Before:**

```tcl
set mode strict
switch -exact -- $action {
    validate {
        process $mode
    }
    skip {
        puts skipped
    }
}
```

**After:**

```tcl
# [O125] set mode strict → sunk into switch arm
switch -exact -- $action {
    validate {
        set mode strict
        process $mode
    }
    skip {
        puts skipped
    }
}
```

### 6. Cases where O125 does NOT fire

**Variable used in condition** — cannot sink because the condition is
evaluated before the body:

```tcl
set b $x
if {$b} {           ;# $b is the condition — no sinking
    puts hello
}
```

**Variable used after the decision block** — sinking would remove the
definition from a path that still needs it:

```tcl
set b foo
if {$a} {
    puts $b
}
puts $b              ;# used after — no sinking
```

**Command substitution in value** — not provably side-effect-free:

```tcl
set b [clock seconds]   ;# has side effects — no sinking
if {$a} {
    puts $b
}
```

**Cross-event variable in iRules** — shared across ``when`` blocks:

```tcl
when HTTP_REQUEST {
    set b foo            ;# shared with HTTP_RESPONSE — no sinking
    if {[HTTP::uri] eq "/"} {
        puts $b
    }
}
when HTTP_RESPONSE {
    puts $b
}
```

**Variable not used in any branch** — nothing to sink:

```tcl
set b foo
if {$a} {
    puts hello           ;# $b never used — no sinking (O109 may delete instead)
}
```

### 7. Inline suppression

Append ``# noqa: O125`` to suppress sinking for a specific assignment:

```tcl
set b foo  ;# noqa: O125
if {$a} {
    puts $b
}
```

## Operational context

- **IR-level pass**: walks the structured IR tree (not the CFG).
- **Runs after**: elimination (O107/O108/O109) — avoids conflict with
  dead-store elimination.
- **Runs before**: structure elimination (O112).
- **Deepest sinking**: recursively descends into nested decision blocks
  to place the assignment at the deepest level where the variable is
  first used.

## Decision rules / contracts

1. **Sinkable statements**: only ``IRAssignConst`` and ``IRAssignValue``
   without command substitutions (``[…]`` in the value).  These are
   side-effect-free and safe to reorder.
2. **Variable must NOT appear in any condition** of the target decision
   block (including ``elseif`` conditions and ``switch`` subjects).
3. **Variable must NOT be used after** the decision block in the same
   script scope.
4. **Cross-event variables** (iRules ``connection`` scope) are excluded.
5. **Conflict avoidance**: if another optimisation (e.g. O109) already
   covers the statement's range, O125 does not fire.
6. **Numeric constants** are typically handled by constant propagation
   (O100) and dead-store elimination (O109) instead of sinking.

## Grouped edits

Each sinking produces a **grouped** set of ``Optimisation`` objects:

| Part | Range | Replacement |
|------|-------|-------------|
| Comment | original ``set`` statement | ``# [O125] set b foo → sunk into if body`` |
| Insertion(s) | opening ``{`` of each target body | ``{\n<indent>set b foo`` |

All parts share a ``group`` ID so they are applied or dropped together.

## File-path anchors

- `core/compiler/optimiser/_code_sinking.py`
- `core/compiler/optimiser/_manager.py` (wiring)
- `core/compiler/optimiser/_types.py` (priority table — O125 has priority 5)
- `lsp/server.py` (``_ALL_OPTIMISATION_CODES`` — disable toggle)
- `editors/vscode/package.json` (``tclLsp.optimiser.O125`` setting)
- `tests/test_optimiser.py` (``TestCodeSinking``)

## Failure modes

- Sinking changes observable behaviour because the value expression
  has hidden side effects not detected by ``_is_sinkable``.
- Orphaned insertion (comment dropped by overlap resolution but
  insertion survives).  Mitigated by conflict-avoidance check and
  post-selection filtering in ``find_optimisations()``.

## Test anchors

- `tests/test_optimiser.py::TestCodeSinking`

## Related KCS notes

- [kcs-downstream-pass-contracts.md](kcs-downstream-pass-contracts.md)
- [kcs-diagnostics-integration.md](kcs-diagnostics-integration.md)
- [kcs-pass-fact-ownership-matrix.md](kcs-pass-fact-ownership-matrix.md)
- [kcs-execution-intent-model.md](kcs-execution-intent-model.md)

## Discoverability

- [compiler KCS index](README.md)
- [compiler architecture overview](../../compiler-architecture.md)
