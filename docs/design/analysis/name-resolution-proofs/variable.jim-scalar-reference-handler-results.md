# naming.variable.jim-scalar-reference-handler-results

Kind: `native-observation`

## Problem statement

Jim shares several public scalar commands with C Tcl but has no C compiler registration or private bytecode donor. A logical handler result can be useful as an independent reference while remaining insufficient to assert C opcode or object-header equivalence.

## Question

What results and caught arity diagnostics does the retained ASCII Jim scalar reference script report?

## Conclusion

The recorded Jim script reports equality0, length3, listlength2 and nocase equality1, followed by the exact four caught arity diagnostics. It is reference handler evidence only and supplies no C compiler registration, native object/header identity or activation proof.

## Scope

One recorded current-label Jim shell binary SHA and exact jim-reference.tcl ASCII source/output. The manifest did not query the Jim revision/version or record a build recipe. All five C providers and BIG-IP are not tested for this separate reference-handler question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `observed`. Version: Exact revision/version not recorded. Build: Shell SHA d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Channel: ASCII script through the recorded Jim shell. Dialect: Jim Tcl.

Exit 0 and empty stderr.

```text
equal 0
length 3
listlength 2
equalnocase 1
llengthempty 1 {wrong # args: should be "llength list"}
llengthextra 1 {wrong # args: should be "llength list"}
lengthmissing 1 {wrong # args: should be "string length string"}
equalmissing 1 {wrong # args: should be "string equal ?-nocase? ?-length int? string1 string2"}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference-manifest.json](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference-manifest.json). SHA-256 `4f1b613ed346e1c135187b0775fbaa620af65e4a623bebfb5accc553f8668b6b`. Original Jim binary/source hashes, argv, exit and stdout/stderr hex.
- `e1` (input): [rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.tcl](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.tcl). SHA-256 `8e4c58a12d2e2f07d42a6e357b8274a89845062bc0b74fe4b82c052adfce9409`. Actual independent ASCII reference handler script.
- `e2` (observation): [rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.txt](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.txt). SHA-256 `167fa8d83c3e1845f38036d3cf906224e8544811cce0e2889ab1e58de4207ceb`. Exact eight result/caught-diagnostic lines.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `NativeScalarScope`: Keep reference public handler results separate from independently selected C compiler operand scope.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "/path/to/recorded/jimsh",
  "rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.tcl"
]
```

The retained script owns its catch/presenter and can be replayed with the recorded binary or an independently documented Jim build. Require exact eight stdout lines, empty stderr and exit0. A new build is a separate observation; this replay is not a C compiler or physical object-header control.
