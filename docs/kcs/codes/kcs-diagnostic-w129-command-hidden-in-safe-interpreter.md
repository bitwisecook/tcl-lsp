# KCS: W129 — Command is hidden in a safe interpreter

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does the analyser warn that a command is hidden in a safe interpreter?

## Why

An interpreter created with `interp create -safe` hides the commands Tcl
considers unsafe: `cd`, `encoding`, `exec`, `exit`, `fconfigure`, `file`,
`glob`, `load`, `open`, `pwd`, `socket`, `source`, `unload`, and `zipfs`
(pinned against tclsh 9.0.4). Calling one inside that interpreter's `interp
eval` body raises `invalid command name` at run time — the command never
executes. The analyser models each interpreter's visible command set (safe
state plus any explicit `interp hide` / `interp expose`), flags the call, and
builds no [source](../../GLOSSARY.md#source-edge) or definition facts from it.

It follows a hidden command through bracket-substitution indirection too: a
nested call (`set x [source b.tcl]`), `{*}` expansion of a built command, the
`[list apply {dir {...}} $dir]` deferred-command idiom used by `package
ifneeded`, `-command`, `after idle`, and `trace add`, and a `namespace
ensemble create`/`configure -map` redirection to a hidden target.

## Symptoms

- A yellow squiggle under a command inside an `interp eval safeInterp { … }`
  body, with the message "'source' is hidden in this safe interpreter — the
  call raises `invalid command name` unless it is exposed or invoked via
  `interp invokehidden`".

## Example that triggers it

```tcl
interp create -safe s
interp eval s { source setup.tcl }
```

The analyser reports **`W129`** on `source`.

## Fix

Either expose the command deliberately:

```tcl
interp create -safe s
interp expose s source
interp eval s { source setup.tcl }
```

or invoke the hidden command from the trusted parent:

```tcl
interp create -safe s
interp invokehidden s source setup.tcl
```

## When it does not fire

- **A dynamic command word.** `{*}$cmdList`, or `set cmd source; $cmd b.tcl`,
  cannot be proven to name a hidden command, so nothing is reported.
- **A dynamic `interp hide` / `interp expose` operand.** The visible set
  becomes unknowable and the analyser abstains for that interpreter entirely.
- **Control-transfer commands.** `break`, `continue`, `yield`, `yieldto`, and
  `tailcall` are not hidden by `interp create -safe`.

An `interp hide` in a **normal** interpreter draws the same warning for the
hidden name.

## How to suppress

Add `# noqa: W129` on the line **above** the offending command. You can also
turn the code off for a project with `disabled = W129` under `[diagnostics]`
in `.tcl-lsp.ini`, or in your editor with `tclLsp.diagnostics.W129` set to
`false`. See
[how to turn a diagnostic off](../kcs-howto-suppress-diagnostics.md).

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W123`, `W128`, `T105`
