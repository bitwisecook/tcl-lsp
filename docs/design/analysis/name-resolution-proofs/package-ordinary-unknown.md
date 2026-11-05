# naming.package.ordinary-unknown

Kind: `native-observation`

## Problem statement

An -exact requirement can reach an unknown callback with an explicit flag or with a rewritten version range. Equal desired version text does not attest the callback argv.

## Question

Which original arguments does discover receive for require -exact missing1?

## Conclusion

Tcl 8.4 captures missing1 -exact, while Tcl 8.5/C8.6/C9 capture missing1-1. Jim rejects unknown registration and never enters this callback.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The original capture helper bytes are unavailable; the current capture.py is a separate wrapper and is not claimed as the original.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20.

The exact original script and recorded process/guest output are:

```json
{
  "case": "unknown",
  "script": "proc discover args {set ::seen $args}; package unknown discover; catch {package require -exact missing 1}; set ::seen",
  "exit": 0,
  "stdout": "0 {missing 1 -exact}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "unknown",
  "script": "proc discover args {set ::seen $args}; package unknown discover; catch {package require -exact missing 1}; set ::seen",
  "exit": 0,
  "stdout": "0 {missing 1-1}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "unknown",
  "script": "proc discover args {set ::seen $args}; package unknown discover; catch {package require -exact missing 1}; set ::seen",
  "exit": 0,
  "stdout": "1 {package, unknown command \"unknown\": should be forget, names, provide, require}\n",
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
