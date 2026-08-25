# KCS: TK1001 — Why does the analyser warn about mixing geometry managers?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag `pack` and `grid` using the same effective geometry
container?

## Why

With geometry propagation enabled, `pack` and `grid` both claim their effective
container through Tk's exclusive geometry-container API. Tk raises "cannot use
geometry manager grid inside … which already has slaves managed by pack" when
the second claim targets the same container. `place` does not make that claim
and may coexist with either manager.

## Symptoms

- A yellow squiggle on the placement that attempts the conflicting claim,
  naming the active manager and effective container.

## Example that triggers it

```tcl
package require Tk

frame .frame
label .frame.title -text "Settings"
button .frame.ok -text OK

pack .frame.title
grid .frame.ok -row 1 -column 0
```

The analyser reports **`TK1001`** on `grid .frame.ok`: `.frame` still has
content managed by `pack` when `grid` attempts its claim.

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

Different containers may use different managers — that is the normal way to
mix them. The `-in` option selects the effective container, so it can differ
from a widget pathname's parent.

## When it does not fire

- **Outside a Tk document.** The check needs the `tk` dialect or a
  `package require Tk` the analyser can resolve.
- **Across interpreters.** A `pack` in the main script and a `grid` in an
  `interp eval` body claim two different windows that merely share a path
  string, so they cannot conflict.
- **`place` with another manager.** `place` positions content without claiming
  or resizing its container.
- **Different `-in` containers.** Pathname siblings can be managed in distinct
  effective containers.
- **A sole widget switches manager.** `pack .item; grid .item` first releases
  `.item` from `pack`, so no packed sibling remains to retain the claim.
- **After release.** `pack forget .item` and `grid forget/remove .item` end
  that widget's active placement. Query forms such as `pack info` do not place
  anything.

A later `destroy` does not suppress a conflict that already occurred: Tcl
would have stopped at the rejected placement before reaching that teardown.

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
