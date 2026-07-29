# KCS: Why does a multi-word `eval` report a wrong-argument-count error?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, analyser

## Question

Why does `eval set total 0` draw a "wrong number of arguments" error, and why
is a variable it sets still reported as read before it is set?

## Symptoms

- A red squiggle under a multi-word `eval`, `uplevel`, or `namespace eval`
  call, reporting **`E002`** — too few arguments — for a command that is
  perfectly well formed.
- A yellow squiggle further down the file reporting **`W210`** — "Variable
  'total' is read before it is set" — even though the `eval` above it sets
  that very variable.
- The same code written as `eval {set total 0}`, with the whole script in one
  braced word, reports nothing.

## Why

`eval` does not evaluate its first argument as a script. It **joins all of
its arguments** into one script first, and then evaluates that. So these
three lines are the same program:

```tcl
eval set total 0
eval {set total} 0
eval {set total 0}
```

The analyser used to walk only the first script word and ignore the rest. It
therefore analysed `eval set total 0` as if the whole script were the single
word `set` — which really is a wrong-argument-count error — and never saw the
write to `total`, so a later `puts $total` looked like a read before set.

The same joining rule applies to `uplevel`, `namespace eval`, and
`interp eval`. It does **not** apply to `catch`, whose script is a single
bounded argument.

## Answer

Both false reports are fixed — no configuration change or workaround is
needed. Update to a build containing the fix.

To confirm the fix is present, put this in a file and check the Problems
view:

```tcl
eval set total 0
puts $total
```

Before the fix: one `E002` on the `eval` line and one `W210` on the `puts`
line. After the fix: nothing.

A genuinely malformed joined script is still reported, because the analyser
now checks the script Tcl actually runs:

```tcl
eval set
```

still reports `E002` — the joined script really is `set` on its own.

## When the analyser stays quiet

When any word of the call is substituted, the script cannot be known before
the program runs:

```tcl
eval $cmd arg
```

`$cmd` could hold anything, so the analyser makes no claim at all about the
call — no argument-count check, and no assumption about which variables it
sets. The security warning **`W101`** still fires, because building a script
by concatenation from a variable is exactly the injection risk it reports.

`namespace inscope` is quiet for a different reason. Its trailing words are
appended as whole list elements rather than joined into the script text, so
`namespace inscope ::app {handler} ready` passes `ready` to `handler` as one
argument. The analyser does not reconstruct that quoting, so it makes no
claim about the call.

## How to suppress

If you still see one of these on a shape not covered above, add
`# noqa: E002` or `# noqa: W210` at the end of the offending line, and please
open an issue with the snippet.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [E002 — too few arguments](codes/kcs-diagnostic-e002-too-few-arguments.md)
- [W210 — variable read before set](codes/kcs-diagnostic-w210-variable-read-before-set.md)
- [W101 — string-concatenated eval](codes/kcs-diagnostic-w101-eval-string-concatenation.md)
