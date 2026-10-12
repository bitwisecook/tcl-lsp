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

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Private C probe with public Tcl_EvalObjv entry; execution fails before a retained observation.. Dialect: Tcl.

The recorded compiler succeeds and the isolated native process exits -11 with empty stdout/stderr. This is an observed process failure; no guest property ownership was measured.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [runtime/rust/tests/data/native_property_clientdata/initial-manifest.json](../../../../runtime/rust/tests/data/native_property_clientdata/initial-manifest.json). SHA-256 `94a30168d64768558962ee34471c14e8df06f9b132fee43eb27b1c78ac7699ac`. Compile success and run exit -11, exact failed probe/build/executable hashes; no emitted native rows.
- `e1` (input): [runtime/rust/tests/data/native_property_clientdata/initial-probe.c](../../../../runtime/rust/tests/data/native_property_clientdata/initial-probe.c). SHA-256 `c176f65164e16d8db1363db652da51546652e6e8ee5f37b3e24605ba2c12b7a0`. Exact failed C probe whose hash matches the original receipt.
- `e2` (observation): [runtime/rust/tests/data/native_property_clientdata/initial-native.tsv](../../../../runtime/rust/tests/data/native_property_clientdata/initial-native.tsv). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Empty retained stdout for the failed process.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I${TCL_SOURCE}/generic",
  "-I${TCL_SOURCE}/unix",
  "runtime/rust/tests/data/native_property_clientdata/initial-probe.c",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

The expected original process failure is not a successful guest comparison. Reconfirming it may terminate the isolated probe process; it must never be counted as a passed property test. Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.
