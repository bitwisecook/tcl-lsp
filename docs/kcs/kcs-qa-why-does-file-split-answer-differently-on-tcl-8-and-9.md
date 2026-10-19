# KCS: Why does `file split` give a different answer for `//a` or `~b` on Tcl 8.6 and Tcl 9?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp CLI

## Question

Why do `file split`, `file join`, `file dirname` and `file tail` give
different answers for a name with a leading `//` or a `~` when I run the
same script as Tcl 8.6 and as Tcl 9?

## Answer

Because Tcl 9 changed how it reads those two things, and the runtimes
answer as the Tcl release you pick reads them. Tcl 8.4 to 8.6 read them
one way and Tcl 9.0 and 9.1 the other.

**A leading `//`.** Tcl 9 reads `//host` at the start of a name as a root
of its own, like a network share. Tcl 8.x reads it as the ordinary root
`/`.

```tcl
file split //a/b    ;# 8.6: / a b      9.0: //a b
file join //a b     ;# 8.6: /a/b       9.0: //a/b
file tail //a       ;# 8.6: a          9.0: (empty)
```

**A `~`.** Tcl 8.x reads a name that starts with `~` as a home directory
(`~` is yours, `~jo` is the user `jo`'s). Tcl 9 reads `~` as an ordinary
character. So on Tcl 8.x, `file split` writes a later `~` element as `./~b`
to stop it being read as a home, and `file join` starts the name again at
a `~` element.

```tcl
file split a/~b     ;# 8.6: a ./~b     9.0: a ~b
file join a ~b      ;# 8.6: ~b         9.0: a/~b
file pathtype ~b    ;# 8.6: absolute   9.0: relative
file dirname ~      ;# 8.6: the parent of your home   9.0: .
```

If a script must behave the same on both, avoid names that start with `//`
or contain a `~` element. On Tcl 9, use `file home` and `file tildeexpand`
when you do want a home directory.

Tcl 8.5 has one more difference. When the last name given to `file join`
starts with `//` and has no other doubled or trailing `/`, Tcl 8.5 keeps
the `//` as it is: `file join a //b` is `//b` there, where Tcl 8.4 and 8.6
give `/b`.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Why does the same `namespace eval` script write a different variable on Tcl 8.6 and Tcl 9?](kcs-qa-why-does-a-namespace-variable-behave-differently-on-tcl-8-and-9.md)
