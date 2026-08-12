# KCS: How does Tcl split a list into elements?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

How does Tcl decide where one list element ends and the next begins, and
what do braces, quotes, and a trailing backslash do to those boundaries?

## Answer

A Tcl list is a string that Tcl splits into elements on **whitespace**
(spaces, tabs, and newlines). This is the same rule the interpreter uses
for a `proc` or method **parameter list**, for the words iterated by
`foreach`, and everywhere else a value is treated as a list. Braces,
quotes, and backslashes change where those whitespace boundaries fall.

### Braces group an element verbatim

An element that starts with `{` runs to its matching `}`; braces nest, so
the inner whitespace does **not** split the element. The bytes between the
braces are taken literally — the one exception is a **backslash-newline**,
which Tcl always collapses to a single space even inside braces (so the
element stays one element, just with a space where the wrapped line was).

```tcl
llength {a {b c} d}   ;# 3 — {b c} is one element containing a space
```

### Quotes group with substitution

An element that starts with `"` runs to its matching `"`, and Tcl performs
backslash and variable substitution on the contents. Like braces, the
enclosed whitespace does not split the element.

```tcl
llength {a "b c" d}   ;# 3 — "b c" is one element
```

### A backslash escapes the next character

Outside braces, a backslash makes the following character part of the
current element instead of acting on its own. So a backslash before a
space or tab keeps that whitespace *inside* the element rather than ending
it:

```tcl
llength {a\ b c}      ;# 2 — a\ b is one element "a b", then c
```

### The wrinkle: a trailing backslash before a newline is a *separator*

The one backslash sequence that does **not** stay inside the element is
**backslash-newline**. A `\` at the end of a line, immediately followed by
the newline, collapses to a single space — and outside braces that space
ends the current element and starts the next one. This is what lets a long
list, such as a parameter list, wrap across several lines:

```tcl
method Fdjac2 {funct ifree n x fvec ldfjac epsfcn pdata nfev \
               step dstep dside ddrtol \
               ddatol} {
    # ...
}
```

Here `ldfjac … pdata nfev` and the wrapped `step …` are all separate
parameters, and the last name on each wrapped line — `nfev`, `ddrtol` — is
just that name. The trailing `\` is consumed as part of the line
continuation, not kept on the name. Windows `\r\n` line endings behave the
same way.

The practical takeaway: wrap a long list wherever you like with a trailing
`\`; the element that ends the line is complete, and the next element
begins on the following line.

## Related

- [KCS index](README.md)
- [KCS: Why does the analyser warn that a variable name is not reachable via $-substitution? (W215)](codes/kcs-diagnostic-w215-variable-name-unreachable-via-substitution.md)
- [How does Tcl 9 handle variable-name corner cases?](kcs-tcl-corner-cases.md)
- [Glossary](../GLOSSARY.md)
