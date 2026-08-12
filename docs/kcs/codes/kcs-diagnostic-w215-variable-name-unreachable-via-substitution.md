# KCS: W215 — Why does the analyser warn that a variable name is not reachable via $-substitution?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, naming

## Profiles

default

## Question

Why does the analyser flag `set "weird}name" 1` (or `set "arr(weird)stuff)" 1`) with a warning that the variable cannot be read via `$`?

## Why

Tcl variables can be created with names containing arbitrary bytes — including `}` (which `set` accepts as a literal because backslash-substitution at the command-word level converts `\}` into the literal byte) and `)` inside an array element name. However, neither of Tcl's `$`-substitution forms can fetch such values:

- The bare form `$name` stops at the first non-word character (ASCII letters, digits, and `_` only).
- The brace form `${name}` is dialect-dependent. In Tcl 8.x (and the F5 and EDA dialects) it reads literally until the **first** `}` — there is no escape mechanism inside the braces. Tcl 9.0 changed the rule: nested `{…}` pairs are tracked and `\}` no longer ends the name, so a name with **balanced** inner braces (`a{b}c`) becomes reachable in 9.x while a bare `}` still is not. The analyser applies the active dialect's rule, so the same file can warn under `tcl8.6` and stay silent under `tcl9.0`.
- `$arr(idx)` reads up to the matching `)`; an `idx` containing `)` is unreachable.

Such a variable is alive — `set name`, `[set "name"]`, `info exists`, `upvar`, and other commands that take a *variable name* will all see it — but every `$`-form attempt to read it parses as something else (a different variable plus literal text), almost always producing a `can't read` error or a silent wrong value.

## Symptoms

- A yellow squiggle appears under the variable name in `set` / `incr` / `lappend` / `variable` / `global` / `upvar` etc., with the message "variable name contains '}'; it can be created and read via `set name` / `[set "name"]` / `info exists` / `upvar`, but is not reachable via $-substitution (neither `$name` nor `${name}` can fetch it)".

## Example that triggers it

```tcl
set "weird}name" 1
puts ${weird}name}     ;# can't read "weird": no such variable -- the
                        # } closes the brace form at "weird", and "name}"
                        # is just literal text
puts [set "weird}name"]   ;# 1   -- reads correctly
```

The analyser reports **`W215`** on the `set` site of `weird}name`.

## Fix

Pick a name that doesn't contain `}` (or `)` for array indices). If you really need such a name (e.g. encoding raw bytes from external input), keep the variable but read it via `[set $name_holding_the_name]` or `[upvar 0 $name_holding_the_name local]` so the substitution syntax is never required.

```tcl
set tame_name "weirdname"
set $tame_name 1
puts $weirdname    ;# 1
```

## How to suppress

Add `# noqa: W215` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [KCS: How does Tcl split a list into elements?](../kcs-qa-how-tcl-parses-lists.md)
  — why a `\`-wrapped parameter list names the right variables.
- Tcl(n) man page §"Variable substitution"
- Related codes: `W212`
