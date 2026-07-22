# KCS: W129 — Command is hidden in a safe interpreter

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Profiles

default

## Question

Why does the analyser warn that a command is hidden in a safe interpreter?

## Why

An interpreter created with `interp create -safe` hides the commands Tcl
considers unsafe — `source`, `load`, `file`, `exec`, `open`, `socket`,
`cd`, `pwd`, `glob`, `exit`, `fconfigure`, `encoding`, and `unload`
(pinned against tclsh 9.0.4).  Calling one of them inside that
interpreter's `interp eval` body raises `invalid command name` at run
time — the command never executes.  The analyser models each
interpreter's visible command set (safe state plus any explicit
`interp hide` / `interp expose`), flags the call, and builds no
[source](../../GLOSSARY.md#source-edge) or definition facts from it,
because C Tcl never runs it.

The check also follows a hidden command through `[...]` bracket-substitution
indirection — a direct nested call (`set x [source b.tcl]`), `{*}` expansion
of a built command, and the pervasive `package ifneeded name ver [list apply
{dir {...}} $dir]` deferred-command idiom (also seen as `-command [list
apply {...} $x]`, `after idle [list apply {...} $x]`, `trace add ...
command [list apply {...} $x]`) — so a hidden `source` nested inside such a
lambda body is flagged the same way a direct `source` call would be.  The
underlying runtime already refuses every one of these shapes at execution
time regardless of whether this diagnostic catches it ahead of time — this
check is early, editor-time feedback, not the enforcement mechanism itself.

## Symptoms

- A yellow squiggle appears under a command inside an
  `interp eval safeInterp { … }` body, with the message: "'source' is
  hidden in this safe interpreter — the call raises `invalid command
  name` unless it is exposed or invoked via `interp invokehidden`."

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

## Notes

An `interp hide` in a **normal** interpreter draws the same warning for
the hidden name, and a dynamic `interp hide` / `interp expose` operand
makes the visible set unknowable, so the analyser abstains entirely for
that interpreter.

A command reached only through an unresolvable dynamic value (`{*}$cmdList`,
or `set cmd source; $cmd b.tcl`) is not flagged — its identity cannot be
proven statically, and this diagnostic prefers a missed warning over a
false one. See the
[contributor-level note on the bracket-indirection fix](../kcs-issue-w129-safe-interp-hidden-command-via-bracket-indirection.md)
for the full list of indirection shapes this check follows (and the ones it
deliberately does not).
