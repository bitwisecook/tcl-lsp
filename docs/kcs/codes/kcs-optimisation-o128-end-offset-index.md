# KCS: O128 — Use end-offset indices instead of length arithmetic

> **Audience:** User
> **Type:** Functionality

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

- `lindex list index ...`
- `lrange list first last`
- `lreplace list first last ?element ...?`
- `linsert list index ?element ...?`

String commands (paired with `string length`):

- `string index string charIndex`
- `string range string first last`
- `string replace string first last ?newString?`

## Safety conditions

- The container argument must be a single `$var` (or `${var}`) reference.
  Commands such as `lindex [get-list] [expr {[llength [get-list]] - 1}]`
  are **not** rewritten because collapsing them would drop one of the two
  `[get-list]` calls and change observable side effects.
- The length command inside the index expression must read the same
  variable as the outer container argument. Cross-variable patterns like
  `lindex $L [expr {[llength $M] - 1}]` are skipped.
- The subtraction constant must be a positive integer. `[llength $L] - 0`
  and bare `[llength $L]` point one past the last valid index and are
  left alone.
- The length kind must match the command kind: `llength` pairs with list
  commands, `string length` with string commands. Mismatches are skipped
  so that a user-intended shimmer is not silently rewritten away.

## How to disable

Toggle the optimiser profile in your editor settings. See the
[optimiser feature](../features/kcs-feature-optimiser.md) for profile
options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [Constant folding](../../GLOSSARY.md#constant-folding)
- Related codes: `O114`, `O117`, `O118`
