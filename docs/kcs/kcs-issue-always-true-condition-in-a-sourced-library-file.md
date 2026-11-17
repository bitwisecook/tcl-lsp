# KCS: A condition in my library file is reported as always true

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, tcl-lsp-cli

## Question

Why did the editor say a condition on a procedure parameter in my library file
is always true or always false, when the procedure is called with different
values from somewhere else?

## Symptoms

- An `I230` hint on an `if` inside a procedure: `Condition '$mode eq "prod"'
  is always true`, where `$mode` is a parameter.
- Every call in the file passes the same literal, but other files, callbacks
  or an interactive session call the procedure with other values.

## Answer

Earlier releases treated a parameter as a constant when every call in the
file passed the same literal, and folded conditions on it. That was a false
positive: a procedure stays callable from a later `source`, a callback or an
interactive session, which no reading of one file or one workspace bounds.

```tcl
# lib.tcl
proc helper {mode} {
    if {$mode eq "prod"} { set r 1 } else { set r 2 }
}
helper prod
helper prod
```

After `source lib.tcl`, tclsh 8.4 to 9.1 return `1 2` for
`list [helper prod] [helper dev]`.

A procedure body is now always analysed from a generic entry, so a condition
over a parameter is never reported as constant, whatever the file's calls
pass. If you still see such a hint on a condition over a parameter, collect
the output channel log and open an issue.

## Related

- [When does the analyser treat a proc parameter as a constant?](kcs-qa-when-is-a-proc-parameter-treated-as-a-constant.md)
- [I230 — constant existence check](codes/kcs-diagnostic-i230-constant-existence-check.md)
- [How to suppress diagnostics](kcs-howto-suppress-diagnostics.md)
