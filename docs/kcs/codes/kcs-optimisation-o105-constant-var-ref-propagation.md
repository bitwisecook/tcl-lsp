# KCS: O105 — Propagate constants into variable references (GVN/CSE)

> **Audience:** User
> **Type:** Optimisation

## Applies to

all-editors, optimisation, gvn

## Profiles

standard, full

## Question

What does O105 rewrite, and when does it fire?

## Why

Replacing `$var` with its known value reduces runtime work and memory traffic.
The same optimiser also spots a command result that has already been worked out
once — but reusing one is only safe when nothing can change which command runs,
and nothing is watching it run. The optimiser proves that first, and stays
quiet when it cannot.

## Before

```tcl
set endpoint /health
set copiedEndpoint $endpoint
```

## After

```tcl
set endpoint /health
set copiedEndpoint /health
```

## When a repeated command is reported

Every one of these must hold:

- the command sits at the top level of the file — not in a procedure, method,
  `apply` body, or `namespace eval` body, and not inside `if`, `while`, `for`,
  `foreach`, `switch`, `catch`, or `try`;
- the command registry declares it stable and worth reusing, such as `llength`,
  `format`, `lindex`, or `lreverse`;
- every argument is a plain word — no `[...]` substitution, no `{*}` expansion,
  and no variable whose read could fire a trace; and
- nothing between the two calls could change or observe how the command is
  dispatched.

```tcl
llength {a b}
llength {a b}   ;# O105: this repeats the computation above
```

## Safety conditions

The report is withheld — deliberately, and without saying so — when any of the
following may be true at that point in the script.

- A live execution or command trace applies to the command, or an `enterstep`
  or `leavestep` trace applies to anything, because that watches every nested
  command too. Removing the trace with literal arguments can restore the
  report; removing it with a computed name or handler cannot.
- The command may have been renamed, aliased with `interp alias`, hidden or
  exposed, or reached through a namespace import, ensemble, path, or `unknown`
  handler, or the interpreter's own policy changed.
- An `eval`, `source`, `package require`, or `namespace eval` ran between the
  two calls, so the optimiser cannot know what that code did.
- A variable trace could fire on a variable used as an argument.
- The result is not reproducible — the current clock time, for example — or it
  reads interpreter state that changed between the two calls.
- The code is inside a procedure or other body, which runs after arbitrary
  earlier history the optimiser does not yet summarise across a file.

O105 is an optimiser report, not an automatic editor quick fix. There is no
rewrite to apply, and the original code is always kept when a proof is missing.

## How to disable

Toggle the optimiser profile in your editor settings. See the [optimiser feature](../features/kcs-feature-optimiser.md) for profile options.

## Related

- [KCS codes index](README.md)
- [Optimiser feature](../features/kcs-feature-optimiser.md)
- [GVN](../../GLOSSARY.md#gvn) and [CSE](../../GLOSSARY.md#cse) — the passes
  that produce this report
- [Dispatch-stability proof](../../design/compiler/dispatch-stability-proof.md)
  — the design doc for the proof described above
- Related codes: `O100`, `O106`
