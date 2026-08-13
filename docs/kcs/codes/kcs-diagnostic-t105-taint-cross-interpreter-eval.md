# KCS: T105 — Why does the analyser warn about tainted data in `interp eval`?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser flag user-controlled data passed as the script
argument of `interp eval` or `interp invokehidden`?

## Why

The script argument of a cross-interpreter eval is *code*. A tainted
value inside it is code the attacker wrote, running in the child
interpreter. That is a code-injection hole, and a particularly awkward
one: a child interpreter is often used precisely because it is meant to
be a boundary, so an injection there defeats the reason it exists at all.

`interp invokehidden` is worse still — hidden commands are the ones the
safe interpreter deliberately withholds.

## Symptoms

- A yellow squiggle under the script argument, with the message "Tainted
  variable $cmd in interp eval script argument; risk of cross-interpreter
  code injection".

## Example that triggers it

```tcl
set child [interp create -safe]
set cmd [gets stdin]
interp eval $child $cmd
```

The analyser reports **`T105`** on `$cmd`.

## Fix

Build the script with `list` so the untrusted value can only ever be an
argument, never a command word:

```tcl
set child [interp create -safe]
set name [gets stdin]
interp eval $child [list set user $name]
```

## When it does not fire

- **A canonical list clears it.** Once the analyser can see the script
  was built with `list`, the value is a quoted word and the finding is
  withdrawn.
- **A literal command word wraps it.** In `interp eval $child [list
  puts $v]` the head of the constructed list is a known literal command,
  so `$v` is an argument rather than code.

The check covers `interp eval` and `interp invokehidden`, plus Tk's
`console eval` and `consoleinterp eval` / `consoleinterp record`. An
abbreviation such as `interp ev` resolves to the same subcommand and is
checked the same way.

## How to suppress

Add `# noqa: T105` on the line **above** the offending command. You can
also turn the code off for a project with `disabled = T105` under
`[diagnostics]` in `.tcl-lsp.ini`, or in your editor with
`tclLsp.diagnostics.T105` set to `false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T100`, `T104`, `W312`, `W129`
