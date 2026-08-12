# KCS: W139 — Why does the analyser say this was removed in a later release?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a command, subcommand, option, or
argument value was removed in the version my file resolves to?

## Why

The registry records three lifecycle facts for each command, subcommand,
option, and literal argument value: the release that introduced it, the
release that deprecated it, and the release that removed it. W139 is the
removal rung. It fires when the resolved version floor is **at or past**
the removing release, which means the item no longer exists there and the
call fails at run time.

The retiring release is **exclusive**: a removal recorded at `10.0.0`
means `10.0.0` is the first release without the item, not the last one
with it.

W139 is the counterpart of `W135` and `W136`, which report the opposite
end of the same lifecycle — a floor *below* the introducing release.
`W144` covers the middle state, where the item is deprecated but still
present.

## Symptoms

- A yellow squiggle under the command, option, or value, with a message
  like "'SOME::command' was removed in f5-irules-cmds 21.0.0 but
  f5-irules ships f5-irules-cmds 21.1.0".

## Example that triggers it

The version floor comes from the profile's library pin, raised by any
versioned `package require`. Pin a project above the release that removed
an item and every use of that item draws W139:

```tcl
package require SomePackage 3.0

SomePackage::oldCall x
```

If the registry records `SomePackage::oldCall` as removed in 3.0, the
analyser reports **`W139`** on the call: the resolved floor of 3.0 is
already past the removal.

## Fix

Replace the removed item with its supported successor, or lower the
resolved floor to a release that still has it. Which of the two is right
depends on why the floor is set where it is — a pin that reflects the
runtime you actually deploy on is the one to keep, and the call is the
thing to change.

For BIG-IP projects, the floor is the configured BIG-IP version rather
than a `package require`; adjusting it changes which release the whole
file is checked against.

## How to suppress

Turn the code off for a project with `disabled = W139` under
`[diagnostics]` in `.tcl-lsp.ini`, for one file with a
`# tcl-lsp: disable=W139` directive at the top of the file, or in your
editor with `tclLsp.diagnostics.W139` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W135`, `W136`, `W144`, `IRULE1003`
