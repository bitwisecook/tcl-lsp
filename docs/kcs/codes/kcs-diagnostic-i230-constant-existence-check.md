# KCS: I230 — Why does the analyser say my `[info exists]` branch never runs?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, sccp

## Profiles

default

## Question

Why does the analyser report that `[info exists X]` (or `[array exists X]`) is
"always false" — or "always true" — and that one branch of my `if` is
unreachable?

## Why

When a variable is provably set (or provably never set) at the point of the
check, `[info exists X]` has a known answer, so one arm of the `if` can never
run. The most common surprise is a **local** variable used for "remember this
between calls":

```tcl
proc authorize {} {
    if {[info exists handle]} {
        # re-use the existing handle
    } else {
        set handle [ILX::init Access-Plugin Access-Extension]
    }
}
```

Each call to `authorize` gets a **fresh** local scope — Tcl locals do not
survive from one invocation to the next. So `handle` is never set when the
check runs, `[info exists handle]` is always false, and the re-use branch is
dead. Re-entrancy does not change this: a new call (from APM, an ILX callback,
or anywhere) is a new frame with empty locals.

The analyser folds the check to its constant value and reports **`I230`** on the
condition. The optimiser can then drop the dead branch
([`O107`](kcs-optimisation-o107-unreachable-dead-code.md)).

## Fix

To remember a value across calls, give it real cross-call storage instead of a
local:

```tcl
proc authorize {} {
    global handle              ;# or:  variable handle
    if {[info exists handle]} {
        # re-use the existing handle
    } else {
        set handle [ILX::init Access-Plugin Access-Extension]
    }
}
```

With `global` (or a namespace `variable`, or iRules `table` / `session` /
`static::`), the variable can persist, the existence check is no longer
constant, and `I230` disappears. If a branch really is dead, delete it.

## How to suppress

Add `# noqa: I230` at the end of the condition's line.

## Related

- [KCS codes index](README.md)
- [W210 — variable read before set](kcs-diagnostic-w210-variable-read-before-set.md)
- [O107 — unreachable dead code](kcs-optimisation-o107-unreachable-dead-code.md)
- [W240 — loop condition constant false](kcs-diagnostic-w240-loop-constant-false.md)
