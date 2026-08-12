# KCS: IRULE5003 — Why does the analyser warn that my loop can miss zero?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default, dialect:irule

## Question

Why does the analyser flag `while {$count != 0}` when the body counts the
variable down?

## Why

`!=` only stops the loop on the exact value zero. If anything moves the
counter past zero — a decrement of more than one, a starting value that
is already negative, or a second `incr` somewhere in the body — the test
never becomes false and the loop runs away. In an iRule that means a TMM
spinning on a single connection.

`>` cannot be stepped over: once the counter is at or below zero the loop
ends, whatever route it took to get there.

## Symptoms

- A hint underline on the loop condition, with the message "Loop
  condition '$count != 0' can miss zero if decremented past it. Consider
  '$count > 0'."

## Example that triggers it

```tcl
when HTTP_REQUEST {
  set count 10
  while {$count != 0} {
    log local0.info "tick $count"
    incr count -2
  }
}
```

The analyser reports **`IRULE5003`** on the condition: `count` steps
`10, 8, 6, 4, 2, 0` here, but any odd starting value skips zero entirely
and the loop never ends.

## Fix

```tcl
when HTTP_REQUEST {
  set count 10
  while {$count > 0} {
    log local0.info "tick $count"
    incr count -2
  }
}
```

The check looks for a `while` whose condition compares a variable against
zero with `!=` or `ne` — in either order, and with `${name}` as well as
`$name` — and whose body decrements that same variable with `incr`.

## How to suppress

`IRULE5003` is internal: it has no per-code entry in the generated editor
settings list. Silence it for one file with a
`# tcl-lsp: disable=IRULE5003` directive at the top of the file, or for a
whole project with `disabled = IRULE5003` under `[diagnostics]` in
`.tcl-lsp.ini`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W240`, `W241`, `W242`
