# KCS: How well does tcl-lsp understand the tcltest package?

> **Audience:** User
> **Type:** Q&A

## Applies to

VS Code, Zed, JetBrains, Neovim, Helix, Emacs, Sublime Text, tcl-lsp CLI

## Question

When I `package require tcltest`, does the language server understand `test`,
`configure`, and the rest of the tcltest commands — and does it track how they
differ between Tcl versions?

## Answer

Yes. Once a document does `package require tcltest`, the whole tcltest command
surface activates: the functional commands (`test`, `cleanupTests`,
`makeFile`, `runAllTests`, …), the configuration commands (`configure`,
`customMatch`, `testConstraint`, …), and the deprecated convenience commands
(`verbose`, `match`, `skip`, `debug`, `bytestring`, …). Hover, completion, and
the diagnostics for arity, options, and script bodies all work on them.

### Options are modelled, not guessed

The `test` and `configure` commands carry their full option list, so
completion offers each option with a short description and the analyser
recurses into the script-valued options (`-body`, `-setup`, `-cleanup`, and
`configure -load`) to report nested diagnostics.

```tcl
package require tcltest
namespace import tcltest::*

test math-1.1 "addition" -setup {
    set a 2
} -body {
    expr {$a + 2}
} -result 4
```

### Version awareness

The registry knows which tcltest ships with which Tcl release — 2.2.11 with
8.4, 2.3.8 with 8.5, 2.5.9 with 8.6, and 2.5.10 with 9.0 — and gates the
surface accordingly:

* `test -errorCode` is offered only when the resolved tcltest is 2.5 or newer
  (Tcl 8.6+); a `package require tcltest 2.3` document does not see it.
* `bytestring` is dropped under Tcl 9.0, where the package no longer defines
  it.

The internal `test*` commands from Tcl's own test build (`testchannel`,
`testobj`, `teststaticlibrary`, …) are modelled too, each gated to the Tcl
versions whose test build actually registers it — for example
`teststaticlibrary` is Tcl 9.0+ while its old name `teststaticpkg` is 8.x
only.

## See also

- [kcs-howto-add-command-registry-package.md](kcs-howto-add-command-registry-package.md)
  — how registry support for a Tcl package is added.
