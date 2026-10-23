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
the inner whitespace does **not** split the element. At the list-parsing
stage, the text inside a braced element is taken verbatim: list parsing does
not collapse its backslashes, including a backslash-newline. A different phase
can run first: when a Tcl script parser constructs a braced word, it collapses
backslash-newline while parsing that script word, before a later command treats
the resulting value as a list. Do not attribute that earlier script phase to
list-value parsing.
```tcl
llength {a {b c} d}   ;# 3 — {b c} is one element containing a space
```

### Quotes group and decode escapes

An element that starts with `"` runs to its matching `"` and groups its
whitespace. List parsing can collapse backslash escapes in a quoted element,
but it does **not** perform variable substitution. Variable substitution, if
any, belongs to an earlier Tcl script-evaluation phase that produced the list
string; parsing a list value does not re-evaluate `$name` text.

```tcl
llength {a "b c" d}   ;# 3 — "b c" is one element
llength {"$x"}         ;# the element is the literal text $x, not x's value
```

### A backslash escapes the next character

Outside braces, a backslash makes the following character part of the
current element instead of acting on its own. So a backslash before a
space or tab keeps that whitespace *inside* the element rather than ending
it:

```tcl
llength {a\ b c}      ;# 2 — a\ b is one element "a b", then c
```

### Backslash-newline in list data is a continuation

In a bare or quoted list element, **backslash-newline** and following spaces
or tabs collapse to one space, which stays inside the element rather than
separating it. Inside a braced list element it remains verbatim at this phase.
This list-value rule is distinct from the Tcl script parser's earlier
backslash-newline handling when it constructs a braced word. A continuation in
list data can therefore let one element wrap across lines:

```tcl
method Fdjac2 {funct ifree n x fvec ldfjac epsfcn pdata nfev \
               step dstep dside ddrtol \
               ddatol} {
    # ...
}
```

Here the Tcl script parser first consumes each backslash-newline while it
constructs the braced argument. It leaves whitespace in the resulting list
string, and the later list parser treats that whitespace as the separator
between parameter names. This is script construction followed by list parsing,
not a backslash-newline separator rule in raw list data.

The practical takeaway: in a Tcl script's braced parameter-list word, a
trailing backslash-newline is consumed before the list is split, leaving a
space between elements. If you parse already-formed list data, the list
parser's own braced versus bare/quoted rules described above apply.

## Related

- [KCS index](README.md)
- [KCS: Why does the analyser warn that a variable name is not reachable via $-substitution? (W215)](codes/kcs-diagnostic-w215-variable-name-unreachable-via-substitution.md)
- [How does Tcl 9 handle variable-name corner cases?](kcs-tcl-corner-cases.md)
- [Glossary](../GLOSSARY.md)
