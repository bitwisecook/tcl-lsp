# KCS: S100 — Why does the analyser warn about a shimmer outside a loop?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, shimmer

## Profiles

default

## Question

Why does the analyser flag a single type-conversion (shimmer) outside a loop?

## Why

Tcl converts the value's internal type each time the code runs, which wastes CPU time and can accumulate in hot paths.

## Symptoms

- A blue information underline appears under the variable use, with the message "value shimmers between types".

## Example that triggers it

```tcl
set x "42"
expr {$x + 1}
string length $x
```

The analyser reports **`S100`** because `x` is used as both an integer and a string.

## When it does not fire

### A pure literal used as a list, dict, or number

A value written as a literal — a braced list, a quoted string, a bare word, or a
number — is a *pure string* until something reads it as another type. That first
read installs the internal type once, for free; there is no cached type to throw
away, so it is **not** a shimmer. A well-formed list literal iterated by
`foreach` (or read by `lindex`, `llength`, `dict for`, …) is silent:

```tcl
set fontSizes {10.0 12.0 16.0 24.0}
foreach size $fontSizes { ... }    ;# no S100 — {…} is a valid list, read once for free
set a 1
foreach b $a { ... }               ;# no S100 — "1" is still a valid one-element list
set d {a 1 b 2}
dict for {k v} $d { ... }          ;# no S100 — an even-length list literal is a valid dict
```

The warning still fires when the value carries a *committed* internal type from
a command — `[list …]`, `[dict create …]`, an object constructor, or
`binary format` — and is then read as a different type, or when a literal is
read as a type it is **not** a valid instance of (`incr` on `hello`, which fails
at runtime).

It also fires on the **second** conversion of a literal: the first read commits
the type, and a later read as a different type converts again on every run:

```tcl
set v 5
expr {$v + 1}   ;# first read — commits the numeric type, free
lindex $v 0     ;# second read — numeric converted to list: S100 fires here
```

The message names the type the value actually held ("has numeric intrep but
`lindex` expects list"), and the related information points at the line that
first committed it.

### A variable filled by a destructuring command

A variable filled by a *destructuring* command — `lassign`, `scan`, `regexp`,
`regsub`, or `binary scan` — is **not** flagged. These commands write list
elements or parsed pieces whose internal type the analyser cannot know, so it
makes no claim about them and no shimmer is reported:

```tcl
set point [list 1 2 3]
lassign $point x y z
set offset [expr {$x + $y + $z}]   ;# no S100 — x/y/z are elements, not lists
```

### A variable named inside a brace-quoted word

Tcl substitutes nothing inside `{…}`, so a `$name` written there is just the
characters `$name` and the variable is never read:

```tcl
set x [llength $l]
lindex {$x} 0        ;# no S100 — the word is the two characters "$x"
lindex "$x" 0        ;# S100 — the quoted word does substitute
lindex $x 0          ;# S100 — so does the bare one
```

The exceptions are the commands that evaluate their braced word as code where
your own variables are in scope — `expr`, `if`, `while`, and the other
expression and body positions. There the names really are read, so the warning
still fires:

```tcl
set x [lrange $l 0 1]
expr {$x + 1}        ;# S100 — expr evaluates the word here and now
```

`apply` is not one of them: its lambda runs in a fresh frame, so it never reads
the caller's variables.

## Fix

Use separate variables for numeric and string use:

```tcl
set x "42"
set x_num [expr {$x + 0}]
expr {$x_num + 1}; string length $x
```

## How to suppress

Add `# noqa: S100` on the line **above** the offending command, or apply
the "Suppress S100 with a noqa comment" quick fix offered on the
diagnostic.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [shimmer](../../GLOSSARY.md#shimmer) · `S101`, `S102`
