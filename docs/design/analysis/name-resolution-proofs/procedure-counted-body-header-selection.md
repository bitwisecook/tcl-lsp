# naming.procedure.counted-body-header-selection

Kind: `native-observation`

## Problem statement

A no-op procedure compiler header may classify a body through a CString prefix or its counted bytes, and continuation/control whitespace can differ by release. Inferring body emptiness from a presentation string would collapse these measured boundaries.

## Question

Which compiler header pointer remains after defining args procedures with counted raw-zero, FF, continuation and whitespace body bytes?

## Conclusion

For cases0,3,4,5,6,7, C8.4 header bits are1,1,0,0,1,1; C8.5–9.1 bits are1,0,0,1,1,0. Every definition returns code0. Thus the counted raw-zero and backslash-newline controls differ across the observed release boundary; this establishes only actual header presence, not procedure argument acceptance or entered body effects.

## Scope

Counted original Tcl_NewStringObj parameter/body objects passed to proc via public Tcl_EvalObjv with TCL_EVAL_DIRECT, followed by private Command.compileProc inspection. Eight definitions redefine p in one native interpreter; all observed code0. Full original source/five outputs and linked-library digests are retained; runtime version queries, compiler/executable/header digests and full stderr are not in this receipt. Jim/BIG-IP are not tested. This compiler-header purpose remains independent of declaration/name/procedure-body ownership.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (receipt release association; not queried by probe). Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; compiler/executable/header metadata not recorded. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Dialect: tcl8.4.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,1],[4,0,0],[5,0,0],[6,0,1],[7,0,1]]. Recorded process status0.

### tcl8.5

Status: `observed`. Version: 8.5.19 (receipt release association; not queried by probe). Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; compiler/executable/header metadata not recorded. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Dialect: tcl8.5.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,0],[4,0,0],[5,0,1],[6,0,1],[7,0,0]]. Recorded process status0.

### tcl8.6

Status: `observed`. Version: 8.6.18 (receipt release association; not queried by probe). Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; compiler/executable/header metadata not recorded. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Dialect: tcl8.6.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,0],[4,0,0],[5,0,1],[6,0,1],[7,0,0]]. Recorded process status0.

### tcl9.0

Status: `observed`. Version: 9.0.4 (receipt release association; not queried by probe). Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; compiler/executable/header metadata not recorded. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Dialect: tcl9.0.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,0],[4,0,0],[5,0,1],[6,0,1],[7,0,0]]. Recorded process status0.

### tcl9.1

Status: `observed`. Version: 9.1.0 (receipt release association; not queried by probe). Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; compiler/executable/header metadata not recorded. Channel: compiled public Tcl_EvalObjv TCL_EVAL_DIRECT; private Command.compileProc observer. Dialect: tcl9.1.

Relevant [case,definition code,compiler-header present] rows: [[0,0,1],[3,0,0],[4,0,0],[5,0,1],[6,0,1],[7,0,0]]. Recorded process status0.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No native observation of this exact question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No native observation of this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_procedure_headers/probe.c](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/probe.c). SHA-256 `4c88bf66e1da56ca812565f2861a7f7c2bb33d9f737746f3072771f089b30b33`. Exact counted parameter/body arrays, direct-evaluation flags and private compiler-header pointer observer.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_procedure_headers/manifest.json](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/manifest.json). SHA-256 `f7c731a9d9d43ab78d5ff73308d5d224298a21b7692aef11dae02df2042f5244`. Original five release associations, library digests, output digests/counts and process status.
- `replay-description` (limitation): [rust/tcl-syntax/tests/data/native_procedure_headers/README.md](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/README.md). SHA-256 `6edef5cc965bbf9599866aed1c954abfd4eb6df3ec060cda6aea256019d07b3f`. Maintained description gives a selected-header/library compiler protocol; no executed fresh run is asserted.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_procedure_headers/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/8.4.20.txt). SHA-256 `06131ac9067d58dbc6e6d9b7f4b5b8da12ae59cdbca4a35152c9980854af9034`. Original full output, relevant cases [0, 3, 4, 5, 6, 7]. Each line reports case, definition code and compiler pointer presence.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_procedure_headers/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/8.5.19.txt). SHA-256 `cf998ec4c933fbcf67d01f7ccfe5a2331011819d0d572b7d05ec38da587cf6d6`. Original full output, relevant cases [0, 3, 4, 5, 6, 7]. Each line reports case, definition code and compiler pointer presence.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_procedure_headers/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/8.6.18.txt). SHA-256 `cf998ec4c933fbcf67d01f7ccfe5a2331011819d0d572b7d05ec38da587cf6d6`. Original full output, relevant cases [0, 3, 4, 5, 6, 7]. Each line reports case, definition code and compiler pointer presence.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/9.0.4.txt). SHA-256 `cf998ec4c933fbcf67d01f7ccfe5a2331011819d0d572b7d05ec38da587cf6d6`. Original full output, relevant cases [0, 3, 4, 5, 6, 7]. Each line reports case, definition code and compiler pointer presence.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_procedure_headers/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_procedure_headers/9.1.0.txt). SHA-256 `cf998ec4c933fbcf67d01f7ccfe5a2331011819d0d572b7d05ec38da587cf6d6`. Original full output, relevant cases [0, 3, 4, 5, 6, 7]. Each line reports case, definition code and compiler pointer presence.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `procedure_header_compilation_bytes`: Owns the selected header byte-purpose independently of formal storage and actual entered procedure execution.
- [rust/tcl-registry/src/native_procedure.rs](../../../../rust/tcl-registry/src/native_procedure.rs), `tests::byte_header_selection_matches_original_native_storage` (linked): The specific counted parameter/body controls compare NoOp/Absent header selection against actual native pointer observations, without proving allocation or call acceptance.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `tests::physical_procedure_capture_and_header_remain_separate_from_authored_names` (linked): Checks raw-zero body header absence under actual C9 despite an authored C8.4 name policy, retained/copy body ownership separately, and missing native provider refusal.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native launch or executed Rust result is claimed. The exact probe and captured output can support a new run only after independently pinning the actual release, headers, library and build configuration; capture-time absolute compiler paths are metadata, not a current portable replay runner.
