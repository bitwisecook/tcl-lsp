# naming.package.ordinary-versions

Kind: `native-observation`

## Problem statement

The scripts register version 1 and an equivalent 1.00 after version 2. An identity keyed by literal spelling can keep a stale loader instead of replacing the same version slot.

## Question

What versions and ifneeded script remain after registering 2,1 and equivalent1.00?

## Conclusion

The C programs return versions 2/1 and REPLACED for query1.0. Jim fails at ifneeded and supplies no entered version-table replacement observation.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates. The manifest retains each inner source string and result. The original capture helper bytes are unavailable; the current capture.py is a separate wrapper and is not claimed as the original.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The exact original script and recorded process/guest output are:

```json
{
  "case": "versions",
  "script": "package ifneeded probe 2 SECOND; package ifneeded probe 1 FIRST; package ifneeded probe 1.00 REPLACED; list [package versions probe] [package ifneeded probe 1.0]",
  "exit": 0,
  "stdout": "0 {{2 1} REPLACED}\n",
  "stderr": ""
}
```

The process status is distinct from the caught guest result. No alternative input or later frontier is substituted.

### jim

Status: `observed`. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred. Version: jim Jim.

The exact original script and recorded process/guest output are:

```json
{
  "case": "versions",
  "script": "package ifneeded probe 2 SECOND; package ifneeded probe 1 FIRST; package ifneeded probe 1.00 REPLACED; list [package versions probe] [package ifneeded probe 1.0]",
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
