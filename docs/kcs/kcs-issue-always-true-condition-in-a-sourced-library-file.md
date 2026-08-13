# KCS: A condition in my library file is reported as always true

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, tcl-lsp-cli

## Question

Why does the editor say a condition in my library file is always true or
always false, when the file's procedure is clearly called with different
values from somewhere else?

## Symptoms

- An `I230` hint on an `if` in a library file: `Condition '$mode eq "prod"'
  is always true`.
- The procedure the condition lives in really is called with more than one
  value — just not from the file the hint appears in.
- Opening the calling file makes the hint disappear.

## Answer

The compiler works out a parameter's value from the calls it can see. If
every call agrees on one literal, the parameter is treated as a constant and
conditions on it fold. That reasoning is only correct when the calls it can
see are *all* the calls.

A library file that is loaded by another file has callers the file itself
does not contain:

```tcl
# lib.tcl
proc helper {mode} {
    if {$mode eq "prod"} { ... } else { ... }
}
helper prod
helper prod
```

```tcl
# main.tcl
source lib.tcl
helper dev
```

The server reads the whole workspace, so with both files in the project it
sees `helper dev` and does not fold. If you see the hint anyway, one of these
is usually why:

1. **The calling file is not in the workspace.** Open the folder that
   contains both files (**File > Open Folder**), not just the single file.
   Opening a single file gives the server no project to enumerate callers
   from.
2. **The calling file is outside the workspace entirely** — a vendored
   script, or a separate project that `package require`s this one. The
   server cannot enumerate callers it has never been shown. Add a
   `package provide` line to the library: a file that declares itself a
   package is never seeded from its own call sites, because its procedures
   are public API by definition.
3. **You are running the command line on one file.** `tcl diag lib.tcl` has
   no project to consult. Pass every file — `tcl diag lib.tcl main.tcl`, or a
   directory — and the calls are shared across them.

If none of those apply, the hint is telling you something true: within
everything the server can see, that parameter really does only ever take one
value.

To see exactly what the compiler decided and why, run:

```
tcl explore --show unitScope --text lib.tcl
```

It lists the boundaries the file crosses (`package provide`, `source`,
`namespace export`, …), whether a cross-file view was available, and the
verdict for each argument position.

If the calling file *is* in the workspace and the hint still appears, collect
the output channel log and open an issue.

## Related

- [I230 — constant existence check](codes/kcs-diagnostic-i230-constant-existence-check.md)
- [Compilation-unit scope](../design/compiler/compilation-unit-scope.md) —
  the contract behind this behaviour.
- [How to suppress diagnostics](kcs-howto-suppress-diagnostics.md)
