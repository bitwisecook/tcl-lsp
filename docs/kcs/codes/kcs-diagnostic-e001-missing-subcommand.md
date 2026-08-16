# KCS: E001 — Why does the analyser flag a bare command with no subcommand?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why do I see a red squiggle on a command like `string` with no subcommand?

## Why

Commands such as `string`, `array`, and `dict` require a subcommand to do anything useful. A bare invocation is always an error at runtime and will raise a Tcl exception.

The same failure shape exists for TclOO objects: an object command invoked with no method word fails before any method lookup (Tcl 9 raises `wrong # args: should be "::oo::Obj… method ?arg ...?"`), so an `unknown` handler cannot save it. The analyser flags every spelling of that dispatch it can prove reaches a TclOO object: a bare `$obj`, a bare `[Dog new]` command substitution, a captured handle (`set b [$a make]; $b`), and a proven object-returning factory (`proc make {} { return [Dog new] }; [make]`).

## Symptoms

- A red squiggle appears under the bare command, with the message "missing subcommand for 'string'" or "'Dog new' requires a method".

## Example that triggers it

```tcl
string
```

The analyser reports **`E001`** on the bare `string` token. Likewise:

```tcl
oo::class create Dog { method bark {} { return woof } }
[Dog new]
```

reports **`E001`** on the `[Dog new]` head.

## Fix

```tcl
string length $x
[Dog new] bark
```

Provide the required subcommand or method so the command knows which operation to perform.

## Limits

The TclOO form fires only when every candidate class is defined in the same document and is a genuine TclOO metaclass. snit and [incr Tcl] dispatchers, external classes, and dynamically-built command names abstain rather than guess.

## How to suppress

Add `# noqa: E001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `E002`, `E003`
