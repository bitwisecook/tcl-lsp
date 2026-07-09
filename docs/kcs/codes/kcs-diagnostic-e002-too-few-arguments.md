# KCS: E002 — Why does the analyser say a command has too few arguments?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why do I see a red squiggle saying a command was called with too few arguments?

## Why

Calling a command with fewer arguments than it requires will always raise a runtime error. Catching this statically prevents unexpected failures in production.

This check is not limited to builtin commands: it also applies to same-file `proc` calls, `interp alias` targets (shifted by any prepended arguments), `rename`d commands (which keep the original's arity), and TclOO methods and `forward`s (including `forward NAME my TARGET ?ARG…?`, the idiom for forwarding to a sibling or inherited method).

## Symptoms

- A red squiggle appears under the command, with the message "too few arguments for 'puts'".
- The same squiggle can appear on a call to a `proc` you defined earlier in the file, an `interp alias`, a `rename`d command, or a `$obj method` call — not just builtin commands.

## Example that triggers it

```tcl
puts
```

The analyser reports **`E002`** on the bare `puts` token.

```tcl
proc greet {name} {
    return "hello $name"
}
greet
```

The analyser reports **`E002`** on the call, since `greet` requires one argument.

## Command-prefix callback context

`E002` also fires on a **callback proc that requires more arguments than its
command prefix supplies**. When a command invokes a callback (`lsort -command
cb`, `trace add … cb`, `$graph walk … -command cb`), it appends a fixed number
of arguments; if the referenced proc has more *required* parameters than that,
the runtime call raises "too few arguments". Here the squiggle is under the
**callback proc name** (the head of the prefix), not under the calling command
— look at the proc it names.

```tcl
proc cmp {a b c} { return 0 }
lsort -command cmp {3 1 2}   ;# lsort appends only 2 → E002 on `cmp`
```

Fix by giving the extra parameters defaults (`{a b {c 0}}`) or removing them so
the callback matches the appended-argument count. (A callback whose appended
count is open-ended — `AtLeast(n)` — never draws `E002`.)

## Fix

```tcl
puts "hello"
```

Supply the required arguments so the command can execute successfully.

## How to suppress

Add `# noqa: E002` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `E001`, `E003`
