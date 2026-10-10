# naming.variable.error-producer-reset-callback-order

Kind: `native-observation`

## Problem statement

The actual error worker retains original info/code operands differently from visible globals, and reset publishes those objects through write traces. Callback-time additional references cannot be inferred from pre-reset values or script-trace behaviour.

## Question

Which original ByteArray info/code objects does the error worker retain, and what do direct native write traces see during reset?

## Conclusion

For the measured direct-worker ff 00 61 operands, C8.5+ retains the info object and validates the same code object into a List. Direct native reset callbacks observe code publication before info and an additional saved-interpreter-state owner. C8.4 uses its separately recorded global path. These windows do not apply to script callbacks or prove source-analysis observer closure.

## Scope

Original C API/worker and internal root-cell inspection with the exact retained probes, headers, libraries and controlled exception entry points. The input objects are retained before the reported windows; any report conversion is part of the exact probe. Only C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 were queried. No fresh rerun, Rust pass, general variable lookup, Jim or BIG-IP observation is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Captured library SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Dialect: Tcl.

Observed capture for 8.4.20:
default-code	privateinfo=null	privatecode=null	info_same=1	code_same=1	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=list	global1_same=0
error_code=1
explicit-error	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=1	code_refs=2	global0=string	global0_same=0	global1=bytearray	global1_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=1	code_refs=2	global0=string	global0_same=0	global1=bytearray	global1_same=1

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Captured library SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Dialect: Tcl.

Observed capture for 8.5.19:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Captured library SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Dialect: Tcl.

Observed capture for 8.6.18:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Captured library SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Dialect: Tcl.

Observed capture for 9.0.4:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Captured library SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: Original C API objects and counted/ASCII Tcl scripts as specified by producers.c. Dialect: Tcl.

Observed capture for 9.1.0:
default-code	privateinfo=null	privatecode=list	info_same=1	code_same=0	info_refs=-1	code_refs=-1	global0=null	global0_same=1	global1=null	global1_same=1
error_code=1
explicit-error	privateinfo=bytearray	privatecode=list	info_same=1	code_same=1	info_refs=3	code_refs=3	global0=null	global0_same=0	global1=null	global1_same=0
trace-::errorCode	private=list	private_refs=5	global_same=1
trace-::errorInfo	private=bytearray	private_refs=5	global_same=1
after-reset	privateinfo=null	privatecode=null	info_same=0	code_same=0	info_refs=2	code_refs=2	global0=bytearray	global0_same=1	global1=list	global1_same=1

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `capture` (provider): [rust/tcl-registry/tests/data/native_error_variables/producers-manifest.json](../../../../rust/tcl-registry/tests/data/native_error_variables/producers-manifest.json). SHA-256 `a69004c9aac5b891e466c6da2e254baee81260a046bbe8aef2e225275d47cc98`. Exact original build and capture receipt, including all observed engine versions and source/library/output hashes.
- `probe` (input): [rust/tcl-registry/tests/data/native_error_variables/producers.c](../../../../rust/tcl-registry/tests/data/native_error_variables/producers.c). SHA-256 `557d71f70cb1e73bb71417edb027a82e086757b628530e12fc45accddd2446bf`. Original API calls, reporting order, object retention and internal root-cell observation windows.
- `rows-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.4.20-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.4.20-producers.tsv). SHA-256 `fce12318d7f826d6240b1962f809d511d0162255367b1edc4850c03f187c6458`. Exact native result/state/callback rows for 8.4.20
- `rows-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.5.19-producers.tsv). SHA-256 `d06adeabd57ec85e2435248b12fd3e91c667e214c4a4a1a424f2058d1e2488a6`. Exact native result/state/callback rows for 8.5.19
- `rows-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/8.6.18-producers.tsv). SHA-256 `d06adeabd57ec85e2435248b12fd3e91c667e214c4a4a1a424f2058d1e2488a6`. Exact native result/state/callback rows for 8.6.18
- `rows-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.0.4-producers.tsv). SHA-256 `d06adeabd57ec85e2435248b12fd3e91c667e214c4a4a1a424f2058d1e2488a6`. Exact native result/state/callback rows for 9.0.4
- `rows-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv](../../../../rust/tcl-registry/tests/data/native_error_variables/9.1.0-producers.tsv). SHA-256 `d06adeabd57ec85e2435248b12fd3e91c667e214c4a4a1a424f2058d1e2488a6`. Exact native result/state/callback rows for 9.1.0

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `original_private_error_primaries_and_script_reset_match_native_producers`: Independent Rust fixture comparison; no native capability inferred from the test marker.
- [runtime/rust/src/interp/native_error_variables.rs](../../../../runtime/rust/src/interp/native_error_variables.rs), `interp::native_error_variables::tests::original_private_error_primaries_and_script_reset_match_native_producers` (linked): Checks the scoped original-object/cell/phase windows in this question; retained native rows remain independent evidence.

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
  "rust/tcl-registry/tests/data/native_error_variables/producers.c",
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
