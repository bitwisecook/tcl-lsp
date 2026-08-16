# KCS: W216 — Why does the analyser flag `${arr}(...)` or `${arr($foo)}`?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn that `puts ${arr}(name)` or `puts ${arr($foo)}` is wrong, and offer to rewrite it?

## Why

Two related shapes look like array-element access but Tcl parses them differently from how users normally read them:

1. **`${arr}(foo)`** — `${arr}` is a complete variable substitution that ends at the closing `}`. Whatever follows is unrelated literal text. Tcl parses this as scalar `${arr}` *concatenated* with the literal characters `(foo)`. No array element access happens at all — and if `arr` doesn't exist as a scalar, you'll get a `can't read "arr"` error instead of the array element you wanted.

2. **`${arr($foo)}`** — Inside the brace form, Tcl applies *no further substitution*. From the Tcl(n) man page on variable substitution: *"There is no further substitution or modification to its character contents."* So the four characters `$foo` inside the braces are looked up *literally*. The runtime asks for element `$foo` of array `arr` — not the value of `foo`. If `foo` is a variable holding `bar`, you'd expect `arr(bar)`; you'll actually read `arr($foo)` (a different element, almost certainly missing).

In both cases the user almost always meant array-element access with index substitution.

## The right form

Bare `$arr(foo)` and `$arr($foo)` are the only `$`-substitution syntaxes that run substitution inside the index — that's the canonical Tcl idiom. The brace form `${name}` is for protecting names with characters the bare form can't carry, but it loses index substitution.

## Symptoms

- A yellow squiggle appears under the `${...}(...)` or `${...($...)}` expression, with a message like *"`${arr}(name)` is parsed as scalar `${arr}` followed by literal text `(name)`; did you mean `$arr(name)` for array element access?"*
- A quick fix is offered to replace the expression with the canonical form.

## Examples that trigger it

```tcl
set arr(name) hello
puts ${arr}(name)        ;# W216 -- parsed as ${arr} + literal "(name)"
                         ;# fix: $arr(name)

set foo bar
set arr(bar) world
puts ${arr($foo)}        ;# W216 -- $foo not substituted; reads element "$foo"
                         ;# fix: $arr($foo)
```

When the array name has characters the bare form can't carry (hyphens, spaces, etc.), the fix falls back to a `[set "..."]` form, which still substitutes the index because `set`'s argument is parsed by the command parser:

```tcl
set "funny name" 1
puts ${funny name($foo)}     ;# W216
                              ;# fix: [set "funny name($foo)"]
```

## How to suppress

Add `# noqa: W216` on the line **above** the offending command. (Suppress only when you genuinely want the documented "no substitution" semantics — almost never.)

## Related

- [KCS codes index](README.md)
- Tcl(n) man page §"Variable substitution"
- Related codes: `W212`, `W215`
