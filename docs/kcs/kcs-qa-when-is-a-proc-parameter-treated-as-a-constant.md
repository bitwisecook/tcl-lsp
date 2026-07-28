# KCS: When does the analyser treat a proc parameter as a constant?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, diagnostic, optimisation, sccp, ipa

## Question

When does the analyser decide that a procedure parameter always holds the
same value, and why does adding one line elsewhere in the file sometimes make
that decision — and the diagnostics that follow from it — disappear?

## Answer

The analyser reads every call to a procedure in the file. If each one passes
the *same* literal at a parameter's position, it analyses the procedure body
with that parameter bound to that value. Conditions on it then fold, which is
where [`I230`](codes/kcs-diagnostic-i230-constant-existence-check.md) and the
optimiser's dead-code suggestions come from:

```tcl
proc helper {mode} {
    if {$mode eq "prod"} { ... } else { ... }   ;# I230: always true
}
helper prod
helper prod
```

The decision is only safe if the analyser can prove it saw **every** call. It
therefore also follows calls it cannot see at a glance: calls nested inside a
`catch { … }` or `uplevel { … }` body, calls made from a `TclOO` method, calls
reached through a `namespace import` alias, and — this is the part that
surprises people — calls **dispatched through a variable**:

```tcl
set cmd helper
$cmd dev          ;# a third call to helper, passing "dev"
```

Here the analyser works out which names `$cmd` can hold (`helper`), treats the
line as an ordinary call to `helper` with `dev`, and stops folding `$mode`.
Adding that one line is what makes the earlier `I230` disappear — correctly.

Some indirections cannot be resolved at all. When the analyser cannot tell
which command a dispatch will run, or cannot read a script the code hands to
another command, it must assume the worst — that *any* procedure in the file
may be called with *any* argument — and it stops treating **every** parameter
in that file as constant:

```tcl
set cmd [gets stdin]     ;# could be any command name
$cmd dev

eval $script             ;# could be any script
lsort -command helper $l ;# helper runs with arguments the runtime supplies
```

A [`package provide`](codes/kcs-diagnostic-w123-unresolved-command.md) in the
file has the same effect, because another file may call the procedure
differently. None of this is something to work around: fewer folded parameters
means fewer diagnostics, never wrong ones. If you see a condition folded that
your code really does vary at run time, that is a bug worth reporting — the
fix is always to make the analyser see the call, never to special-case the
parameter.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [I230 — constant existence check](codes/kcs-diagnostic-i230-constant-existence-check.md)
- [W307 — non-literal command name](codes/kcs-diagnostic-w307-non-literal-command.md)
- [How are command names resolved?](kcs-qa-how-are-command-names-resolved.md)
