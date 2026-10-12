# naming.property.owner-probe-availability

Kind: `native-observation`

## Problem statement

An enumeration probe can emit valid header windows and then stop at a failed definition-worker lookup. Those partial observations must not be expanded into later cache/epoch answers, and the failed source cannot be replaced by a different retained program.

## Question

Which property-owner windows are available from the retained c8b7e47d probe process?

## Conclusion

Two header/copy rows are recorded, then the process exits2 after a guest invalid-command-name method error. The later epoch/cache windows are unmeasured. Its original source hash is retained, but the corresponding source bytes are absent, limiting exact replay; the successful 816d811e program is a different input.

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

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: ASCII Tcl_Eval inside a private-header C probe; partial rows precede a caught-by-harness guest worker failure.. Dialect: Tcl.

Two epoch32 header/copy rows, followed by process2 and invalid command name method; later snapshots are not observed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [rust/tcl-registry/tests/data/native_property_owners/initial-manifest.json](../../../../rust/tcl-registry/tests/data/native_property_owners/initial-manifest.json). SHA-256 `7c80d5d7665fd659c389ab58d251a7199cd450e4e8cbf0f105502f0e85b0fbcf`. Compile0, process2, exact partial stdout/error stream and original source/build/executable hashes.
- `e1` (observation): [rust/tcl-registry/tests/data/native_property_owners/initial.tsv](../../../../rust/tcl-registry/tests/data/native_property_owners/initial.tsv). SHA-256 `348302dc14eaec5a35e19bcaeff2389964344e7719fba0f11b4af47c89231604`. Only the two emitted header/copy rows.

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
  "<source-not-retained>",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

The source bytes with recorded SHA c8b7e47d609415e347d1db29bc6f47e81445f62242951b3d9c006194b7e00c29 are not retained. Exact source replay is unavailable; do not substitute the current probe.c, whose SHA differs. Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.
