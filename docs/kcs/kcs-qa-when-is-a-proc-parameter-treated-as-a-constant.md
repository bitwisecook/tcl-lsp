# KCS: When does the analyser treat a proc parameter as a constant?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, diagnostic, optimisation, sccp, ipa

## Question

When does the analyser decide that a procedure parameter always holds the
same value inside the procedure body?

## Answer

Never. A procedure body is analysed from a generic entry: each parameter holds
whatever a caller passes, even when every call in the file passes the same
literal:

```tcl
proc helper {mode} {
    if {$mode eq "prod"} { ... } else { ... }   ;# not folded: $mode is unknown here
}
helper prod
helper prod
```

The calls in one file are not all the calls. A procedure lands in the
interpreter's global command table, so a later `source` of the file, a
callback (`after`, `trace`, `-command`) or an interactive session can call it
with any value. tclsh 8.4 to 9.1 run the body with whatever arrives: after
sourcing the file above, `list [helper prod] [helper dev]` gives both branches.

So a condition over a parameter never produces an
[`I230`](codes/kcs-diagnostic-i230-constant-existence-check.md) hint, the
optimiser never rewrites the body from what the file's calls pass, and
compiled bytecode never specialises the body for them.

What *does* fold is a call site: where a procedure is pure and its result is
proved for the arguments a call passes, that one call can be replaced by its
value (O103), while the body stays as written.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [I230 — constant existence check](codes/kcs-diagnostic-i230-constant-existence-check.md)
- [A condition in my library file is reported as always true](kcs-issue-always-true-condition-in-a-sourced-library-file.md)
- [How are command names resolved?](kcs-qa-how-are-command-names-resolved.md)
