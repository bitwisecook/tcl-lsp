# naming.variable.error-globals-bootstrap-and-legacy-copy

Kind: `native-observation`

## Problem statement

A fresh errorInfo/errorCode root can be absent, present-but-undefined, or backed by an implicit private-object copy. Treating it as an ordinary quiet global read can change contents or suppress a recreation callback. The private legacy-copy bit is explicitly controlled; C8.4 has no inspected private-field counterpart.

## Question

What do original error-variable construction, controlled legacy-copy reads, unsets, missing private values and reset do to the physical global cells?

## Conclusion

C8.4 has ordinary lazily created error globals in this probe. C8.5–9.1 start with undefined allocated error cells; the controlled legacy-copy flag selects private-object copying on reads, and unsets retain/recreate an undefined root that can later copy again. The exact rows include null private values and reset. This is a special error-variable purpose, not a general quiet namespace read or a Jim/iRules policy.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Captured library SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Dialect: Tcl.

Observed capture for 8.4.20:
constructor	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
fresh-read	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
clear-flag-read	code=0	read="475545535420494e464f"	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
set-flag-read	code=0	read="475545535420494e464f"	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
unset-defined	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
read-after-unset	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"
unset-code-defined	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
unset-code-undefined	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
null-private-read	code=1	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null
reset-legacy	code=0	read=null	legacy=-1	privateinfo=null	privatecode=null	errorInfo_present=0	errorInfo_defined=0	errorInfo_value=null	errorCode_present=0	errorCode_defined=0	errorCode_value=null

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Captured library SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Dialect: Tcl.

Observed capture for 8.5.19:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Captured library SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Dialect: Tcl.

Observed capture for 8.6.18:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode="54434c2052454144205641524e414d45"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Captured library SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Dialect: Tcl.

Observed capture for 9.0.4:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode="54434c2052454144205641524e414d45"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Captured library SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by probe.c. Dialect: Tcl.

Observed capture for 9.1.0:
constructor	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
fresh-read	code=1	read=null	legacy=0	privateinfo=null	privatecode="54434c2052454144205641524e414d45"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
clear-flag-read	code=0	read="475545535420494e464f"	legacy=0	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="475545535420494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
set-flag-read	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=0	errorInfo_value=null	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
read-after-unset	code=0	read="5052495641544520494e464f"	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=1	errorCode_value="475545535420434f4445"	errorCode_same=0
unset-code-defined	code=0	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
unset-code-undefined	code=1	read=null	legacy=1	privateinfo="5052495641544520494e464f"	privatecode="5052495641544520434f4445"	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=1	errorCode_present=1	errorCode_defined=0	errorCode_value=null	errorCode_same=0
null-private-read	code=0	read=""	legacy=1	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value=""	errorCode_same=0
reset-legacy	code=0	read=null	legacy=0	privateinfo=null	privatecode=null	errorInfo_present=1	errorInfo_defined=1	errorInfo_value="5052495641544520494e464f"	errorInfo_same=0	errorCode_present=1	errorCode_defined=1	errorCode_value="5052495641544520434f4445"	errorCode_same=0

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `capture` (provider): [rust/tcl-registry/tests/data/native_error_variables/manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/manifest.json). SHA-256 `b4b56484236768bafb41b9f74cc262693d2172ff6e1dfff3b90ea9210e806bbc`. Exact original build and capture receipt, including all observed engine versions and source/library/output hashes.
- `probe` (input): [rust/tcl-registry/tests/data/native_error_variables/probe.c](../../../../rust/tcl-registry/tests/data/native_error_variables/probe.c). SHA-256 `1e8a1d8ec6512fcf0991d33d403da62f78f7e6dc372c7bc366b8dcceb03e3085`. Original API calls, reporting order, object retention and internal root-cell observation windows.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.4.20.tsv). SHA-256 `3381bc073bd7502d0f1240b330b063fcab81ee626107bc1d19c506749e6c0d4c`. Exact native result/state/callback rows for 8.4.20
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19.tsv). SHA-256 `846ff9095a3f5e572f316b42907f1d6896640183d5293b474a32be2cfffe4894`. Exact native result/state/callback rows for 8.5.19
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18.tsv). SHA-256 `5ce398c7dc280ad30a4a17ae5cbf73515289ad3570c1ae2d4dfc829123072a3e`. Exact native result/state/callback rows for 8.6.18
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_error_variables/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4.tsv). SHA-256 `5ce398c7dc280ad30a4a17ae5cbf73515289ad3570c1ae2d4dfc829123072a3e`. Exact native result/state/callback rows for 9.0.4
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_error_variables/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0.tsv). SHA-256 `5ce398c7dc280ad30a4a17ae5cbf73515289ad3570c1ae2d4dfc829123072a3e`. Exact native result/state/callback rows for 9.1.0

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `hidden_error_reads_and_unsets_match_original_native_objects`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::hidden_error_reads_and_unsets_match_original_native_objects` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-std=c99",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/tcl9.0.4/generic",
  "-I/path/tcl9.0.4/unix",
  "rust/tcl-registry/tests/data/native_error_variables/probe.c",
  "/path/tcl9.0.4/unix/libtcl9.0.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/naming-error-variable-probe"
]
```

Build against each exact manifest version/header/library and run the produced executable in a fresh process. Require completed exit 0 and exact full stdout equality with the corresponding evidence TSV (reset episodes excludes only the aggregate version prefix/header). Match retained source/library/header SHA before interpreting any difference. Original captures are retained; a new execution/build availability is not claimed.
