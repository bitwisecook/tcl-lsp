# naming.variable.error-info-append-original-object

Kind: `native-observation`

## Problem statement

Appending error context can preserve an original result object for an empty append yet convert or replace it for a nonempty append. Bytes alone cannot certify the original ByteArray header or private/public reference ownership.

## Question

How do empty and nonempty Tcl_AddObjErrorInfo appends and reset affect the original ByteArray result and private/global error objects?

## Conclusion

The original ff 00 61 ByteArray, empty append, eight-byte context append and reset have distinct measured object/header windows. The retained rows determine sameness and reference counts for each C release; they do not establish an arbitrary string mutation or physical identity from equal bytes.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Captured library SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Dialect: Tcl.

Observed capture for 8.4.20:
empty-append	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=3	code_refs=-1	global0=bytearray	global0_same=1	global1=string	global1_same=0
nonempty-append	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=2	code_refs=-1	global0=string	global0_same=0	global1=string	global1_same=0
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=string	global1_same=0

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Captured library SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Dialect: Tcl.

Observed capture for 8.5.19:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Captured library SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Dialect: Tcl.

Observed capture for 8.6.18:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Captured library SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Dialect: Tcl.

Observed capture for 9.0.4:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Captured library SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by info-append.c. Dialect: Tcl.

Observed capture for 9.1.0:
empty-append	privateinfo=bytearray	privatecode=list	info_same=1	code_same=0	info_refs=3	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
nonempty-append	privateinfo=string	privatecode=list	info_same=0	code_same=0	info_refs=2	code_refs=-1	global0=null	global0_same=0	global1=null	global1_same=1
append-reset	privateinfo=null	privatecode=null	info_same=0	code_same=1	info_refs=1	code_refs=-1	global0=string	global0_same=0	global1=list	global1_same=0

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `capture` (provider): [rust/tcl-registry/tests/data/native_error_variables/info-append-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/info-append-manifest.json). SHA-256 `9bdc22e97e6fc37c4c03843f577657ec7831648cf0fc5e8cd78530e498510c3f`. Exact original build and capture receipt, including all observed engine versions and source/library/output hashes.
- `probe` (input): [rust/tcl-registry/tests/data/native_error_variables/info-append.c](../../../../rust/tcl-registry/tests/data/native_error_variables/info-append.c). SHA-256 `370e2e15b2de0c73471783baa4d9d96c93c846eaadedd7b3b3a1f4f366a42921`. Original API calls, reporting order, object retention and internal root-cell observation windows.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.4.20-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.4.20-info-append.tsv). SHA-256 `88f030fbe4c69fafbc0eef8174c5e2c82c113bdde9c678ec5818f1ab266ac95c`. Exact native result/state/callback rows for 8.4.20
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.5.19-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19-info-append.tsv). SHA-256 `d4c34c2232955a9b0b62e67fdc0246ba56918a9bf3189e436e9c9665f6951346`. Exact native result/state/callback rows for 8.5.19
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.6.18-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18-info-append.tsv). SHA-256 `d4c34c2232955a9b0b62e67fdc0246ba56918a9bf3189e436e9c9665f6951346`. Exact native result/state/callback rows for 8.6.18
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_error_variables/9.0.4-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4-info-append.tsv). SHA-256 `d4c34c2232955a9b0b62e67fdc0246ba56918a9bf3189e436e9c9665f6951346`. Exact native result/state/callback rows for 9.0.4
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_error_variables/9.1.0-info-append.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0-info-append.tsv). SHA-256 `d4c34c2232955a9b0b62e67fdc0246ba56918a9bf3189e436e9c9665f6951346`. Exact native result/state/callback rows for 9.1.0

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `error_info_byte_append_preserves_seed_primary_until_nonempty_mutation`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::error_info_byte_append_preserves_seed_primary_until_nonempty_mutation` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

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
  "rust/tcl-registry/tests/data/native_error_variables/info-append.c",
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
