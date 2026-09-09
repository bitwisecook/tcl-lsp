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

- A yellow squiggle under the interpreter path word of an `interp eval`, with
  the message "interpreter 'worker' is never created in this file — `interp
  eval` will raise `could not find interpreter`".

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

## When it does not fire

- **A dynamic path anywhere in the file.** One `interp create $name` makes
  interpreter existence unknowable, and the check abstains for the whole file.
- **A grandchild reached by its full path.** Paths are relative to the current
  interpreter: an `interp create t` inside `interp eval s { … }` creates
  `{s t}`, which a top-level `interp eval {s t} { … }` reaches cleanly.

A deleted path must be re-created before the next `interp eval` into it.

## How to suppress

Add `# noqa: W140` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = W140` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.W140` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W123`, `W129`, `T105`
