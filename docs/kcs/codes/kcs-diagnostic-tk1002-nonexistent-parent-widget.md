# KCS: TK1002 — Why does the analyser say the parent widget does not exist?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that the parent in my widget path has not been
created?

## Why

A Tk widget path names its parent: `.frame.ok` is the child `ok` of the
container `.frame`. Tk creates the child inside that parent, so the
parent has to exist first. If it does not, the call fails with
"bad window path name".

The usual causes are a typo in the path, or creating widgets in the wrong
order.

## Symptoms

- A yellow squiggle on the widget-creation command, with the message
  "Widget path '.frame.ok' references non-existent parent '.frame'."

## Example that triggers it

```tcl
package require Tk

button .frame.ok -text OK
```

The analyser reports **`TK1002`** on `button`: nothing in the file
creates `.frame`.

## Fix

Create the container first:

```tcl
package require Tk

frame .frame
button .frame.ok -text OK
```

The root window `.` always exists, so a single-component path such as
`.ok` is never flagged.

## When it does not fire

- **Outside a Tk document.** The check needs the `tk` dialect or a
  `package require Tk` the analyser can resolve.
- **Across interpreters.** Each interpreter has its own widget
  hierarchy, so a parent created in one is not visible to a path used in
  another.

## How to suppress

`TK1002` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a `# tcl-lsp: disable=TK1002`
directive at the top of the file, or for a whole project with
`disabled = TK1002` under `[diagnostics]` in `.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `TK1001`, `TK1003`
