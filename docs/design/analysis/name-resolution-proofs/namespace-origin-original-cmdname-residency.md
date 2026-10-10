# naming.namespace.origin-original-cmdname-residency

Kind: `native-observation`

## Problem statement

namespace origin may consume a name object that already has a cmdName cache, including one whose string representation has been invalidated. Reconstructing original source ownership from the returned origin or resident bytes would conflate distinct values. This direct object-vector probe retains the same original name across origin, rename and replacement operations.

## Question

Does namespace origin preserve the original cmdName primary and resident-string absence across the recorded C import operations?

## Conclusion

All five resident captures preserve original cmdName and resident bytes while results change from ::src::p to ::src::q to ::dest::p. C8.5–C9.1 stringless captures preserve cmdName and leave the original bytes absent after invalidation, returning ::src::p again. C8.4 has no retained stringless observation. The result spelling cannot donate original word/namespace/Native ownership.

## Scope

Only these nine C process snapshots are observed; C8.4 stringless, Jim and BIG-IP are not tested. Returned namespace origin, original primary and resident string are independent purposes.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant.. Dialect: Tcl.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless mode not recorded.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant.. Dialect: Tcl.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless: cmdName remains, original bytes absent after invalidation, result ::src::p.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant.. Dialect: Tcl.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless: cmdName remains, original bytes absent after invalidation, result ::src::p.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant.. Dialect: Tcl.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless: cmdName remains, original bytes absent after invalidation, result ::src::p.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant.. Dialect: Tcl.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless: cmdName remains, original bytes absent after invalidation, result ::src::p.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [runtime/rust/tests/data/native_namespace_origin/original_provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/original_provenance.json). SHA-256 `e272081d1b2b831762605ffd8b53f200faf0c10f95ff29db68d5143e994ef0ee`. Nine exact resident/stringless process receipts, inline stdout/stderr and source/archive/executable hashes.
- `e1` (input): [runtime/rust/tests/data/native_namespace_origin/original_probe.c](../../../../runtime/rust/tests/data/native_namespace_origin/original_probe.c). SHA-256 `a92f35cb0dafec70f001edf032334266fa4f53aaf16f9fd0b815d808dffd5e01`. Original ::dest::p object reused across public Tcl_EvalObjv namespace origin, rename/replacement and explicit string invalidation.

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
  "runtime/rust/tests/data/native_namespace_origin/original_probe.c",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

Run the probe once without arguments for the resident sequence and with argument stringless for the recorded C8.5–C9.1 variant. Compare the matching inline receipt stdout/stderr and exit0 exactly. Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.
