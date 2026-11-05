# naming.package.ordinary-conflict

Kind: `native-observation`

## Problem statement

A conflicting provide can publish a release-specific error code while leaving the original registration. Equal printed package names do not determine error-publication behavior.

## Question

What caught message and errorCode result follows providing probe1 then probe2?

## Conclusion

Tcl 8.4/8.5 return NONE; Tcl 8.6/C9 return TCL PACKAGE VERSIONCONFLICT. Jim reports an already-provided message with NONE. These are script-level captured globals, not a trace-order or physical header proof.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The original capture helper bytes are unavailable; the current capture.py is a separate wrapper and is not claimed as the original.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

The exact original script and recorded process/guest output are:

```json
{
  "case": "conflict",
  "script": "package provide probe 1; catch {package provide probe 2} m; list $m $::errorCode",
  "exit": 0,
  "stdout": "0 {{conflicting versions provided for package \"probe\": 1, then 2} NONE}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "conflict",
  "script": "package provide probe 1; catch {package provide probe 2} m; list $m $::errorCode",
  "exit": 0,
  "stdout": "0 {{conflicting versions provided for package \"probe\": 1, then 2} {TCL PACKAGE VERSIONCONFLICT}}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "conflict",
  "script": "package provide probe 1; catch {package provide probe 2} m; list $m $::errorCode",
  "exit": 0,
  "stdout": "0 {{package \"probe\" was already provided} NONE}\n",
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
