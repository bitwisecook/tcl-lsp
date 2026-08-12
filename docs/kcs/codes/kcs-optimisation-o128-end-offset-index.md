# KCS: O128 — Use end-offset indices instead of length arithmetic

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, const-fold

## Profiles

readability, standard, full

## Question

What does O128 rewrite, and when does it fire?

## Why

Tcl list and string commands accept `end` and `end-N` as indices, which
the bytecode compiler resolves directly. Computing the same offset via
`[expr {[llength $L] - N}]` or `[expr {[string length $s] - N}]` calls
the length command, runs an `expr` evaluation, and re-reads the
container — all of which the `end`/`end-N` form avoids. The rewritten
form is also shorter and the intent ("last element") reads more clearly.

## Before

```tcl
set last      [lindex $L [expr {[llength $L] - 1}]]
set penult    [lindex $L [expr {[llength $L] - 2}]]
set tail      [lrange $L 0 [expr {[llength $L] - 1}]]
set last_char [string index $s [expr {[string length $s] - 1}]]
```

## After

```tcl
set last      [lindex $L end]
set penult    [lindex $L end-1]
set tail      [lrange $L 0 end]
set last_char [string index $s end]
```

## Commands covered

List commands (paired with `llength`):

- `lindex list index` — only the single-index form rewrites; additional
  indices in `lindex list i j …` resolve against the sub-list produced
  by the previous step, so the outer `llength` would no longer describe
  the correct container.
- `lrange list first last`
- `lreplace list first last ?element ...?`

String commands (paired with `string length`):

- `string index string charIndex`
- `string range string first last`
- `string replace string first last ?newString?`

## Commands intentionally excluded

- `linsert list index ?element ...?` — `linsert $L end x` appends to
  the list, whereas `linsert $L [expr {[llength $L] - 1}] x` inserts
  before the final element. No generic `end`/`end-N` form preserves
  that semantics for the `N == 1` case, so the whole command is
  skipped rather than partially rewritten.
- `lset` — takes a variable name (not a `$var`), and the multi-index
  form has subtly different end-offset semantics from the other list
  commands.

## Safety conditions

- The container argument must be a single `$var` (or `${var}`) reference.
  Commands such as `lindex [get-list] [expr {[llength [get-list]] - 1}]`
  are **not** rewritten because collapsing them would drop one of the two
  `[get-list]` calls and change observable side effects.
- The length command inside the index expression must be the **same
  concrete variable reference** as the outer container. Comparison
  preserves array indices and braced-scalar-vs-array-element forms, so
  `lindex $a(1) [expr {[llength $a(2)] - 1}]` and
  `lindex ${a(1)} [expr {[llength $a(1)] - 1}]` are both rejected.
- The subtracted operand must be a positive integer literal.
  `[llength $L] - 0`, bare `[llength $L]`, `[llength $L] - $N`,
  `[llength $L] - [foo]`, and chained subtractions like
  `[llength $L] - 1 - 1` are all skipped — the latter are left to the
  expression-folding passes to handle first.
- The length kind must match the command kind: `llength` pairs with list
  commands, `string length` with string commands. Mismatches are skipped
  so that a user-intended [shimmer](../../GLOSSARY.md#shimmer) is not
  silently rewritten away.
- For multi-index `lindex`, only the first index (which is relative to
  the original list value) is considered; later indices are left alone.

## How to disable

Toggle the optimiser profile in your editor settings. See the
[optimiser feature](../features/kcs-feature-optimiser.md) for profile
options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O114`, `O117`, `O118`
