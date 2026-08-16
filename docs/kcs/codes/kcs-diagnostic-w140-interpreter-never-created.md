# KCS: W140 — Target interpreter is never created in this file

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does the analyser warn that an `interp eval` targets an interpreter that is never created?

## Why

An interpreter path names a child interpreter that must exist before any
`interp eval` into it — evaluating into a path that was never
`interp create`d raises `could not find interpreter "name"` at run time
(pinned against tclsh 9.0.4).  The analyser tracks each literal
`interp create` / `interp delete` in the file, so an `interp eval` whose
literal path matches no live creation is flagged.  When any `interp`
operation in the file uses a dynamic path (`interp create $name`),
interpreter existence becomes unknowable and the warning abstains
entirely.

## Symptoms

- A yellow squiggle appears under the interpreter path word of an
  `interp eval`, with the message: "interpreter 'worker' is never created
  in this file — `interp eval` will raise `could not find interpreter`."

## Example that triggers it

```tcl
interp eval worker { puts hi }
```

The analyser reports **`W140`** on `worker`.

## Fix

```tcl
interp create worker
interp eval worker { puts hi }
```

## Notes

Paths are relative to the current interpreter: an `interp create t`
inside `interp eval s { … }` creates the grandchild `{s t}`, which a
top-level `interp eval {s t} { … }` reaches without warning.  A deleted
path must be re-created before the next `interp eval` into it.
