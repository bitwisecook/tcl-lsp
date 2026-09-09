# KCS: Why is a variable my helper sets through `uplevel` reported as unset?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Question

My helper procedure assigns a variable in its caller's frame with
`uplevel 1 [list set $varName $value]`. When does the analyser believe that
write, and when does it still report the caller's read as unset?

## Symptoms

- A yellow squiggle reporting **`W210`** — "Variable 'answer' is read before
  it is set" — on a read of a variable you expect a helper to have assigned.
- A yellow squiggle reporting **`W212`** — "'set' expects a variable name,
  got substitution" — on a `set $var …` word.

## Why

Both checks turn on *whose* frame a write lands in.

**`W210`.** A procedure that writes its caller's frame is summarised once,
and every call site consults that summary. A level that lands on the direct
caller (`upvar 1`, `uplevel 1`) is emphatically *not* transitive through an
ordinary call — `worker`'s `upvar 1` reaches `wrapper`'s frame, not
`wrapper`'s caller's — so the summary stops there. A level that lands *past*
the caller does travel one hop further along an ordinary call:

```tcl
proc setUp2  {var} { uplevel 2 [list set $var 99] }
proc middle  {}    { setUp2 answer }
proc outer   {}    { middle ; return $answer }
puts [outer]        ;# -> 99, on tclsh 8.6 and 9.0 alike
```

`answer` is genuinely assigned in `outer`'s frame by a procedure `outer`
never calls directly, so the read is not a read before set.

**`W212`.** That check asks whether a variable-*name* word was spelled as a
`$` substitution by mistake. Inside a `[list …]`-built script the question
does not arise: `list` substitutes `$varName` in the **building** frame, so
the script `uplevel` finally runs already carries the literal name the
author meant. The substitution is the whole idiom, not a slip.

## Answer

Neither shape above draws a report: the summary carries a past-the-caller
level one hop further along an ordinary call, and `W212` does not ask its
question inside a `[list …]`-built script at all.

## What still reports

```tcl
proc setLocally {var} { set $var 99 }     ;# W212 — written by hand
proc useIt {} { setLocally answer
                return $answer }          ;# W210 — nothing reached this frame
```

`setLocally` assigns its *own* local, so `useIt`'s read really does fail
(`can't read "answer": no such variable` under tclsh 8.6 and 9.0), and the
directly written `set $var` really is the name/value shape `W212` exists
for. `uplevel #0` (the global frame) and `uplevel 0` (the procedure's own
frame) reach no caller either, so they silence nothing.

## When the analyser stays quiet anyway

A level the analyser cannot place — `uplevel [expr {[info level] - $n}] …`,
the spelling `tcltest`-style output-capture helpers use — could be any
frame, so the analyser assumes the write may have happened rather than
reporting a read it cannot disprove. The same applies two ordinary calls out
from an `uplevel 3`: the summary carries one hop, and widens beyond it.

A writer materialised at run time is outside this entirely. A procedure
built by `eval [subst {proc ::puts …}]` and then reached through a renamed
built-in does not exist in the source the analyser reads, so a variable only
that procedure assigns is still reported. Suppress it per line if you hit
that shape.

## How to suppress

Add `# noqa: W210` or `# noqa: W212` on the line **above** the offending
command, and open an issue with the snippet.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [W210 — variable read before set](codes/kcs-diagnostic-w210-variable-read-before-set.md)
- [W212 — substitution where a variable name is expected](codes/kcs-diagnostic-w212-variable-substitution-where-name-expected.md)
- [Why does a multi-word `eval` report a wrong-argument-count error?](kcs-issue-false-diagnostics-inside-a-multi-word-eval.md)
