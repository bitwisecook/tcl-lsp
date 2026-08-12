# O125 — Code sinking into decision blocks

## Summary

O125 sinks side-effect-free variable assignments into the deepest
decision block (``if``/``switch``) that uses them.  This reduces
unnecessary work on execution paths where the variable is never
read.

## Enabling and disabling O125

O125's category is `OptCategory::CodeMotion`
(`rust/tcl-core-types/src/diag_code.rs`).  The default editor profile is
`readability`, which enables only `OptCategory::Readability` — so **O125 does
not surface under the default configuration**.  It needs
`tclLsp.optimiser.profile` set to `full` or `aggressive`.

Once the profile enables it, three switches turn it back off:

| Method | Scope | How |
|--------|-------|-----|
| **VS Code setting** | Workspace / user | Set `tclLsp.optimiser.O125` to `false` in `settings.json` |
| **Inline suppression** | Single command | Put `# noqa: O125` on the line **before** the assignment |
| **LSP ``didChangeConfiguration``** | Session-wide | Send `tclLsp.optimiser.O125 = false` in the settings payload |

The master switch `tclLsp.optimiser.enabled` suppresses O125 with every other
O-code.  All three of the per-code routes land in the same
`disabled_optimisations` set that `lift_compiler_diagnostics`
(`rust/tcl-lsp-server/src/lib.rs`) filters against; the profile's own
disabled set comes from `profile_to_disabled`
(`rust/tcl-compiler/src/optimiser/profiles.rs`).

## What the rewrite looks like

O125 does **not** emit a comment.  `emit_sink` produces a grouped pair (or
group of `n + 1`):

- a **deletion** — an empty replacement over the original ``set`` statement's
  span;
- one **prepend** per target — the first using statement of each target
  branch is replaced by `"<original set text>; <that statement's text>"`.

So `set b foo` followed by `if {$a} { puts $b }` yields a deletion of
`set b foo` and a replacement of `puts $b` with `set b foo; puts $b`.  The
examples below show the resulting source; the `; ` join is literal.

When the original statement's source text cannot be recovered from
`ctx.source` — a proc body lowered with local-offset spans — the pass falls
back to a single `hint_only` diagnostic on the assignment and proposes no
edit.

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
if {$a} {
    set b foo; puts $b
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
if {$a} {
    set b foo; puts $b
} else {
    set b foo; puts $b
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
if {$a} {
    set b foo; puts $b
} else {
    puts hello
}
```

### 4. Deep sinking into nested decision blocks

`find_deepest_targets` descends when a branch has exactly one using
statement, that statement is itself a decision, no earlier statement in the
branch redefines the variable, and the inner decision's conditions do not
read it:

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
if {$a} {
    if {$c} {
        set b foo; puts $b
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
switch -exact -- $action {
    validate {
        set mode strict; process $mode
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

**A branch redefines something the value reads** — the relocated
computation would observe a different `$a`:

```tcl
set x [expr {$a + 1}]
if {$c} {
    set a 0             ;# clobbers an RHS read — no sinking
    puts $x
}
```

**A condition command substitution that may write an RHS read** — a
`[regexp … -> a]` in the guard can assign `a`, so the sink is refused
conservatively:

```tcl
set x [expr {$a + 1}]
if {[regexp {b} $s -> a]} {
    puts $x
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

Put ``# noqa: O125`` on the line *before* the assignment.  The suppression
scanner (`apply_preceding_noqa`,
`rust/tcl-compiler/src/analyser/utils.rs`) reads a command's
`preceding_comment`, so a trailing ``;# noqa: O125`` on the ``set`` line
attaches to the *next* command instead and does not suppress the sink:

```tcl
# noqa: O125
set b foo
if {$a} {
    puts $b
}
```

## Operational context

- **IR-level pass**: `walk_script` descends the structured IR tree
  (`cu.ir_module.top_level` plus every procedure body), not the CFG.  It is
  bounded by `MAX_OPTIMISER_WALK_DEPTH`.
- **Position in the pass order**: `PassId::all()`
  (`rust/tcl-compiler/src/optimiser/mod.rs`) runs propagation, branch
  folding, structure elimination, expr simplification, pattern recognition,
  elimination, **code sinking**, tail call, unused procs.  So O125 runs after
  structure elimination (O112) and after dead-store elimination
  (O107/O108/O109/O126), and before the tail-call and unused-proc passes.
- **Deepest sinking**: `find_deepest_targets` and `try_deeper_sink`
  recursively descend into nested decision blocks to place the assignment at
  the deepest level where the variable is first used.

## When O125 fires

`walk_script` scans each script for a statement at index `i` whose successor
at `i + 1` is a decision, and requires all of:

1. **Sinkable statement** (`sinkable_assignment`) — `Statement::AssignConst`,
   `Statement::AssignValue` whose value contains no `[`, or
   `Statement::AssignExpr` whose expression contains no command
   substitution.  These are side-effect-free and safe to reorder.
2. **Not a cross-event variable** — `ctx.cross_event_vars` (iRules
   `connection`-scope state observable after the handler returns) excludes
   the name.
3. **Not already covered** — no optimisation already in `ctx.optimisations`
   spans the statement.  Earlier passes (O109 / O126 dead stores) run first,
   and two rewrites of the same range would conflict.
4. **Successor is a decision** — `Statement::If` or `Statement::Switch`.
5. **The variable is absent from every condition** of that decision
   (`decision_condition_uses_var`, covering `elseif` conditions and the
   `switch` subject).
6. **At least one branch body uses the variable**
   (`any_decision_body_uses_var`).
7. **No later use** — no statement after the decision in the same script
   reads the variable (`statement_uses_var`).
8. **The value's read-set survives the move**
   (`sink_rhs_clobbered_by_decision`) — no branch body at any nesting
   redefines a variable the RHS reads, and no `if` condition contains a
   command substitution (nor a `switch` subject a `[`) that could write one.

### Known gap — barriers are invisible to the later-use scan

Condition 7 is intended to include dynamic barriers: a `Statement::Barrier`
after the decision may observe the variable through `uplevel`, `eval`, or an
alias, so the definition must stay put.  `statement_uses_var`
(`rust/tcl-compiler/src/optimiser/code_sinking.rs:584`) answers `false` for
`Statement::Barrier`, unlike every other consumer of that query, so the
barrier case is not currently enforced and a sink past a barrier is possible.
Tracked as issue #1402.

## Grouped edits

Each sinking produces a **grouped** set of ``Optimisation`` values:

| Part | Span | Replacement |
|------|------|-------------|
| Deletion | original ``set`` statement | `""` |
| Prepend(s) | first using statement of each target body | `"set b foo; puts $b"` |

All parts share a ``group`` id allocated by `PassContext::alloc_group`.
`select_non_overlapping` (`optimiser/helpers/select.rs`) applies a group
all-or-nothing: if any member loses an overlap contest, every surviving
member of the group is dropped too, so a prepend can never land without its
deletion.

## File-path anchors

- `rust/tcl-compiler/src/optimiser/code_sinking.rs` — the pass and its unit tests
- `rust/tcl-compiler/src/optimiser/mod.rs` — `PassId::CodeSinking` dispatch, and
  `opt_priority` (O125 has priority 5)
- `rust/tcl-compiler/src/optimiser/manager.rs` — the `optimise_*` façades that
  run `PassId::all()`
- `rust/tcl-compiler/src/optimiser/helpers/select.rs` — `select_non_overlapping`
- `rust/tcl-compiler/src/optimiser/profiles.rs` — profile → disabled-code set
- `rust/tcl-core-types/src/diag_code.rs` — the `O125` row and its `CodeMotion` category
- `rust/tcl-lsp-server/src/lib.rs` — `lift_compiler_diagnostics` (per-code and
  master toggles, `# noqa` filtering)
- `editors/vscode/package.json` — the ``tclLsp.optimiser.O125`` setting
- `editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/` — the
  JetBrains checkbox (`optimiserO125`)

## Failure modes

- Sinking changes observable behaviour because the value expression has
  hidden side effects `sinkable_assignment` does not detect.
- A sink past a `Statement::Barrier` that can observe the variable — see the
  known gap above (#1402).
- Orphaned prepend (deletion dropped by overlap resolution but the prepend
  survives).  Prevented by the group all-or-nothing rule in
  `select_non_overlapping`.

## Tests

- `rust/tcl-compiler/src/optimiser/code_sinking.rs` unit tests

## Related KCS notes

- [kcs-downstream-pass-contracts.md](../../../docs/design/compiler/downstream-pass-contracts.md)
- [kcs-diagnostics-integration.md](../../../docs/design/compiler/diagnostics-integration.md)
- [kcs-pass-fact-ownership-matrix.md](../../../docs/design/compiler/pass-fact-ownership-matrix.md)
- [kcs-execution-intent-model.md](../../../docs/design/compiler/execution-intent-model.md)

## See also

- [compiler KCS index](README.md)
- [compiler architecture overview](../../../docs/design/compiler-architecture.md)
