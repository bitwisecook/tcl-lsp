# KCS: W129 fires on `break` / `continue` / `yield`, and inline-proc is missing on `file` / `exec`

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, refactoring

## Question

Why does the analyser call `break` "hidden in this safe interpreter", and
why is *Inline proc* not offered on a call to `file`, `exec`, or `open`?

## Symptoms

- A yellow squiggle under `break`, `continue`, `yield`, `yieldto`, or
  `tailcall` inside an `interp eval` body for a `-safe` interpreter:
  "'break' is hidden in this safe interpreter — the call raises
  `invalid command name` unless it is exposed or invoked via
  `interp invokehidden`". None of these commands is hidden by
  `interp create -safe`, so the warning is wrong.
- Worse than the squiggle: W129 also **skips** the command it flags, so
  those control-flow commands contributed nothing to analysis inside a
  safe-interpreter body. Definition, source, and package edges that
  should have been built there went missing, so go-to-definition and
  related features could come up empty.
- Pressing `Ctrl+.` on a call to `file`, `encoding`, `open`, `socket`,
  `exec`, `cd`, `pwd`, `glob`, `load`, or `unload` never offers
  *Inline proc*, even where inlining is safe.

## Why

Both symptoms are one bug with one cause, and neither is a rule anyone
intended.

The command registry describes each command with behavioural trait flags.
Two of them — the flag for "hidden by `interp create -safe`" and the flag
for "jumps out of the current frame" — were accidentally given the *same
bit*, so at run time the two were a single flag. Every safe-hidden command
therefore read as control-transferring, and every control-transferring
command read as safe-hidden.

That single collision produced both symptoms, from opposite directions:

- W129's hidden set is registry data, so the control-transfer commands
  were read straight into it and flagged.
- *Inline proc* declines any call whose head is **frame-sensitive** —
  a command that terminates a block, transfers control, creates a scope
  alias, or creates a barrier — because moving such a command out of its
  proc frame changes what it returns from, breaks out of, or binds
  against. The collision pulled the safe-hidden commands into that set,
  suppressing the action on them.

## Answer

Both are fixed — no configuration change or workaround is needed. Update
to a build containing the fix: the false warning disappears, and
*Inline proc* reappears on the ten commands listed above.

To confirm the fix is present:

1. Open a file containing an `interp eval` body for a `-safe` interpreter
   with a `break` or `continue` in it.
2. Check the Problems view: no **W129** on the control-flow command. A
   W129 on a genuinely hidden command such as `source` or `exec` in the
   same body is correct and still appears — see
   [the W129 diagnostic note](codes/kcs-diagnostic-w129-command-hidden-in-safe-interpreter.md)
   for that set.
3. Put the cursor on a call to a single-command proc whose body is
   `file …` or `exec …` and press `Ctrl+.`. *Inline proc* is offered.

`source` and `exit` are **still** frame-sensitive and still decline
*Inline proc*, on their own merits rather than through the collision:
`source` creates a barrier and `exit` terminates the block. That is
correct behaviour, not a leftover of this bug.

If the false warning persists after updating, collect the output channel
log and open an issue.

## Notes

The traits are now declared so that each flag's bit is assigned by the
compiler rather than written by hand, which makes two flags sharing a bit
impossible to express rather than merely corrected. See
[the command-spec-studio contract](../design/contracts/command-spec-studio.md)
for that arrangement.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [W129 — command hidden in a safe interpreter](codes/kcs-diagnostic-w129-command-hidden-in-safe-interpreter.md)
- [W129 via bracket indirection](kcs-issue-w129-safe-interp-hidden-command-via-bracket-indirection.md)
- [Code actions](features/kcs-feature-code-actions.md)
