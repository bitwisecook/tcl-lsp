# naming.package.ordinary-invalid

Kind: `native-observation`

## Problem statement

A bad version can fail in a version parser or at an unavailable member. Assuming the same failure owner loses both the message and error code.

## Question

What caught message/errorCode follows package vcompare1 bad?

## Conclusion

C rejects the invalid version, with NONE on Tcl 8.4/8.5 and TCL VALUE VERSION on Tcl 8.6/C9. Jim rejects the vcompare member before a version comparison is entered.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The original capture helper bytes are unavailable; the current capture.py is a separate wrapper and is not claimed as the original.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The exact original script and recorded process/guest output are:

```json
{
  "case": "invalid",
  "script": "catch {package vcompare 1 bad} m; list $m $::errorCode",
  "exit": 0,
  "stdout": "0 {{expected version number but got \"bad\"} NONE}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "invalid",
  "script": "catch {package vcompare 1 bad} m; list $m $::errorCode",
  "exit": 0,
  "stdout": "0 {{expected version number but got \"bad\"} {TCL VALUE VERSION}}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "invalid",
  "script": "catch {package vcompare 1 bad} m; list $m $::errorCode",
  "exit": 0,
  "stdout": "0 {{package, unknown command \"vcompare\": should be forget, names, provide, require} NONE}\n",
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
