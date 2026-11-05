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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `not-tested`.

No observation for this question.

### jim

Status: `observed`. Channel: ASCII script through the recorded Jim shell.

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

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `NativeScalarScope`: Keep reference public handler results separate from independently selected C compiler operand scope.

## Evidence

- [rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference-manifest.json](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference-manifest.json)
- [rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.tcl](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.tcl)
- [rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.txt](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/jim-reference.txt)
