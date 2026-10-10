# naming.procedure.original-counted-relocation-identity

Kind: `native-observation`

## Problem statement

A procedure name containing FF or counted zero can be moved using directly constructed name objects. Rendering or Unicode repair before rename can target a different command. This check observes the actual original command pointer and its future namespace, not a logical name roundtrip.

## Question

Do the four recorded counted FF/NUL procedure-name moves preserve the original command identity and select the destination namespace?

## Conclusion

All five C releases and the hashed Jim library record rename success, original command identity1, and future result ::B for all four counted-name controls. These observations establish only the selected rename/command identity within each process; they do not prove a general name comparator, resident string protocol or source Document producer.

## Scope

Exactly four counted FF/NUL name moves per engine. C releases and hashed Jim library are observed; Jim reported revision/patchlevel and executable hashes are not retained. No BIG-IP or Document-character-channel claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion.. Dialect: Tcl.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion.. Dialect: Tcl.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion.. Dialect: Tcl.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion.. Dialect: Tcl.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion.. Dialect: Tcl.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### jim

Status: `observed`. Version: Jim; patchlevel/revision not retained. Build: Exact compiler command, library/header/source hashes and available executable hashes are retained in the selected receipt. A recorded source/build release is not a newly executed startup check.. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion.. Dialect: Jim Tcl.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (observation): [runtime/rust/tests/data/native_procedure_relocation/raw-manifest.json](../../../../runtime/rust/tests/data/native_procedure_relocation/raw-manifest.json). SHA-256 `c92823957a01c56555f4b2cfd4e232ed97925fad2ccd93e2066b3aafbbbd5803`. Six exact original compile/run receipts and inline24-control streams with source/header/library hashes.
- `e1` (input): [runtime/rust/tests/data/native_procedure_relocation/raw-probe.c](../../../../runtime/rust/tests/data/native_procedure_relocation/raw-probe.c). SHA-256 `cdc3a3d56cd549f2d85b0f0379bb50d0f1f80ee28c1a2b876d398d177293f05d`. Exact directly constructed counted original and destination procedure name operands, including FF and NUL.
- `e2` (observation): [runtime/rust/tests/data/native_procedure_relocation/raw-observations.tsv](../../../../runtime/rust/tests/data/native_procedure_relocation/raw-observations.tsv). SHA-256 `9874f817a6721b8728c7f9a5bc5e3acfd68724452450e24a49354c7ff9e09434`. Twenty-four projected success/command-identity/future namespace rows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_procedure_relocation_tests.rs](../../../../runtime/rust/src/interp/native_procedure_relocation_tests.rs), `relocation_of_original_counted_names_matches_native_binding_identity` (linked): Compare the counted rename command identity and future namespace for all24 controls.

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
  "runtime/rust/tests/data/native_procedure_relocation/raw-probe.c",
  "${TCL_BUILD}/libtcl${TCL_ABI}.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "${PROBE_EXE}"
]
```

Jim uses -DJIM_PROBE=1 and its matching captured header/library; preserve lengths and literal zero bytes of both operands. Replay requires the matching recorded headers/library and original source bytes; compare process exit, stdout and stderr exactly before comparing projected rows. Private header/pointer observations are valid only within each original process and cannot authenticate a different interpreter. No Rust execution or fresh native reconfirmation is asserted.
