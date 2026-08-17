# KCS: W142 — Why does the analyser say this command is invalid here?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser flag a command that has the right arguments but is
in the wrong place?

## Why

A few commands are restricted by *where* they appear rather than by how
many arguments they take. The restriction is on the surrounding context,
so the call looks perfectly well formed on its own and only the enclosing
body makes it wrong.

The case you are most likely to hit is `return` in an iRule. Directly
inside a `when EVENT { … }` body, F5's `return` takes no arguments — it
just exits the event. The full Tcl syntax with `-code`, `-level`,
`-errorcode`, and friends is only meaningful inside a `proc`, which is a
real call frame.

## Symptoms

- A yellow squiggle on the command word, with a message like
  "`return` takes no arguments directly inside an iRules event body; wrap
  the call in a proc to use -code/-level/-errorcode/etc."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  if {[HTTP::uri] eq "/health"} {
    return -code error "health endpoint disabled"
  }
}
```

The analyser reports **`W142`** on `return`: inside the event body only
the bare form is valid.

## Fix

Use the bare form in the event, and move anything that needs the full
syntax into a procedure:

```tcl
proc check_uri {uri} {
  if {$uri eq "/health"} {
    return -code error "health endpoint disabled"
  }
  return 1
}

when HTTP_REQUEST {
  if {[catch {check_uri [HTTP::uri]} err]} {
    log local0.error $err
    return
  }
}
```

A `proc` is its own call frame, so the check does not fire inside one —
even when the `proc` statement itself sits lexically inside a `when`
body. (That placement is a separate finding, `IRULE5006`.)

## How to suppress

Turn the code off for a project with `disabled = W142` under
`[diagnostics]` in `.tcl-lsp.ini`, for one file with a
`# tcl-lsp: disable=W142` directive at the top of the file, or in your
editor with `tclLsp.diagnostics.W142` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W125`, `IRULE5006`, `IRULE5007`
