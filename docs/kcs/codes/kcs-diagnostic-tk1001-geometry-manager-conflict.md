# KCS: TK1001 — Why does the analyser warn about mixing geometry managers?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag two geometry managers (`pack`, `grid`, or
`place`) used on children of the same parent widget?

## Why

A Tk container can only be managed by one geometry manager. When two of
them claim the same parent, each keeps resizing the parent
to suit its own children and the other responds in kind. Tk raises
"cannot use geometry manager grid inside … which already has slaves
managed by pack" — or, in the versions that do not, the window loops
forever and never settles.

## Symptoms

- A yellow squiggle on **every** geometry call for the offending parent,
  with the message "Geometry manager conflict: cannot mix 'pack' and
  'grid' in the same parent '.frame'."

## Example that triggers it

```tcl
package require Tk

frame .frame
label .frame.title -text "Settings"
button .frame.ok -text OK

pack .frame.title
grid .frame.ok -row 1 -column 0
```

The analyser reports **`TK1001`** on both geometry calls: `.frame` is the
parent of both children, and it is claimed by `pack` and by `grid`.

## Fix

Pick one manager per container:

```tcl
package require Tk

frame .frame
label .frame.title -text "Settings"
button .frame.ok -text OK

grid .frame.title -row 0 -column 0
grid .frame.ok    -row 1 -column 0
```

Different containers may use different managers — that is the normal way
to mix them. Only sharing one parent is the problem.

## When it does not fire

- **Outside a Tk document.** The check needs the `tk` dialect or a
  `package require Tk` the analyser can resolve.
- **Across interpreters.** A `pack` in the main script and a `grid` in an
  `interp eval` body claim two different windows that merely share a path
  string, so they cannot conflict.
- **Only one manager.** Any *two* of `pack`, `grid`, and `place` on one
  parent conflict; a container claimed by just one of them is fine.

## How to suppress

`TK1001` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a `# tcl-lsp: disable=TK1001`
directive at the top of the file, or for a whole project with
`disabled = TK1001` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `TK1002`, `TK1003`
