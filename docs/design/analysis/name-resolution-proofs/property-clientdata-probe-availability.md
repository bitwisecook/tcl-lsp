# naming.property.clientdata-probe-availability

Kind: `native-observation`

## Problem statement

A successfully compiled private-header probe can fail before emitting any property observations. Treating such an execution as a Tcl feature rejection or a successful ownership test would invent evidence. The recorded process outcome must remain independent of the successful client-data program.

## Question

What compile and process outcome is retained for the c176f651 C9.1 client-data probe, and does it contain a usable ownership observation?

## Conclusion

Compilation succeeds, but execution terminates with signal11 and empty stdout/stderr. It provides no guest property answer and no ownership observation. This process failure neither proves feature unavailability nor invalidates the separately captured program.

## Scope

Only the recorded C Tcl9.1.0 process linked to the hashed private-header build is observed. Other C releases, Jim and BIG-IP are not tested for this question. Native object/cache/reference snapshots are distinct from source-name bytes, command dispatch and current runtime ownership.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`.

No observation for this question.

### tcl9.1

Status: `observed`. Channel: Private C probe with public Tcl_EvalObjv entry; execution fails before a retained observation. Version: tcl9.1 9.1.0.

The recorded compiler succeeds and the isolated native process exits -11 with empty stdout/stderr. This is an observed process failure; no guest property ownership was measured.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [runtime/rust/tests/data/native_property_clientdata/initial-manifest.json](../../../../runtime/rust/tests/data/native_property_clientdata/initial-manifest.json)
- [runtime/rust/tests/data/native_property_clientdata/initial-probe.c](../../../../runtime/rust/tests/data/native_property_clientdata/initial-probe.c)
- [runtime/rust/tests/data/native_property_clientdata/initial-native.tsv](../../../../runtime/rust/tests/data/native_property_clientdata/initial-native.tsv)
