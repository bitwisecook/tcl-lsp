# naming.package.ordinary-stable

Kind: `native-observation`

## Problem statement

A stable version1 competes with beta2b1. Reusing modern version grammar on Tcl 8.4 or borrowing the C loader surface for Jim changes the selected frontier.

## Question

What does require return after registering stable1 and beta2b1?

## Conclusion

Tcl 8.4 rejects beta2b1; Tcl 8.5/C8.6/C9 return stable1. Jim stops at absent ifneeded. The observed outcomes do not establish a broader version grammar.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The original capture helper bytes are unavailable; the current capture.py is a separate wrapper and is not claimed as the original.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20.

The exact original script and recorded process/guest output are:

```json
{
  "case": "stable",
  "script": "package ifneeded probe 1 {package provide probe 1}; package ifneeded probe 2b1 {package provide probe 2b1}; package require probe",
  "exit": 0,
  "stdout": "1 {expected version number but got \"2b1\"}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "stable",
  "script": "package ifneeded probe 1 {package provide probe 1}; package ifneeded probe 2b1 {package provide probe 2b1}; package require probe",
  "exit": 0,
  "stdout": "0 1\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "stable",
  "script": "package ifneeded probe 1 {package provide probe 1}; package ifneeded probe 2b1 {package provide probe 2b1}; package require probe",
  "exit": 0,
  "stdout": "1 {package, unknown command \"ifneeded\": should be forget, names, provide, require}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### bigip

Status: `not-tested`.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_package_ordinary](../../../../rust/tcl-registry/tests/data/native_package_ordinary)
