# naming.tcloo.method-info-scope-ordering

Kind: `native-observation`

## Problem statement

Runtime inspected only the first -scope and could retain recursive -all enumeration; VM ignored -scope and -localprivate. Option order changes which installed script-method names are listed.

## Question

Which measured local script-method membership results from repeated visibility options, repeated C9 scope, and -all before or after an explicit scope?

## Conclusion

In the fixed C8.6/C9 ASCII script-method setup, -localprivate reports no methods because no direct-instance-private C API methods were installed; a following -private restores ordinary local methods, and reversing the order reports none. C9 public/unexported/private scope selects Public/hidden/Secret for the class and corresponding object methods; repeated scope takes the last value. Explicit scope forces local enumeration even with later -all. Default -all adds inherited/destroy names. These outcomes do not establish arbitrary inheritance/export layering or physical result-name object identity.

## Scope

Same finite original counted/source vectors, with separate source setup of inherited/public/unexported script methods and C9 private method declarations. Local hash order is not an equivalence premise; tests compare sorted native list-member bytes for successful rows. No arbitrary C API private flags, original method headers/caches, opaque method name serialization, CPP or Normal grant.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

In the fixed C8.6/C9 ASCII script-method setup, -localprivate reports no methods because no direct-instance-private C API methods were installed; a following -private restores ordinary local methods, and reversing the order reports none. C9 public/unexported/private scope selects Public/hidden/Secret for the class and corresponding object methods; repeated scope takes the last value. Explicit scope forces local enumeration even with later -all. Default -all adds inherited/destroy names. These outcomes do not establish arbitrary inheritance/export layering or physical result-name object identity.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

In the fixed C8.6/C9 ASCII script-method setup, -localprivate reports no methods because no direct-instance-private C API methods were installed; a following -private restores ordinary local methods, and reversing the order reports none. C9 public/unexported/private scope selects Public/hidden/Secret for the class and corresponding object methods; repeated scope takes the last value. Explicit scope forces local enumeration even with later -all. Default -all adds inherited/destroy names. These outcomes do not establish arbitrary inheritance/export layering or physical result-name object identity.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

In the fixed C8.6/C9 ASCII script-method setup, -localprivate reports no methods because no direct-instance-private C API methods were installed; a following -private restores ordinary local methods, and reversing the order reports none. C9 public/unexported/private scope selects Public/hidden/Secret for the class and corresponding object methods; repeated scope takes the last value. Explicit scope forces local enumeration even with later -all. Default -all adds inherited/destroy names. These outcomes do not establish arbitrary inheritance/export layering or physical result-name object identity.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Jim Tcl.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation.

## Exact evidence

- `probe` (input): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/probe.c](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/probe.c). SHA-256 `9ffd92f10951198f3abf3746bd574508ad08e10c8bd22e7dbdca8f9251a4c56e`. Exact retained C/Jim original-vector and counted-source harness.
- `inputs` (input): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/inputs.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/inputs.json). SHA-256 `5513197c17f2653559bca142b2bc4e54f976d33dc554ee6dc96ab9a6f97f2a79`. Fixed case labels and exact option hex, input channels and limits.
- `queue` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/queue.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/queue.json). SHA-256 `cc79de1addc40484c501d44f9ffde538e1d53d219ec89b614324a567e62c607b`. Pinned headers, configured makefiles, static libraries, sources and compiler/link environment.
- `aggregate` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/receipt.json). SHA-256 `f90346218d0f14a56da65e25cda1829b1cb068bbc4a969465fd6c7400bcbbb62`. All six original compile/process receipts, reported patchlevels and named protocol rows.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/receipt.json). SHA-256 `7b577f47846fa62bd41627d9f9a51734c073eb2fa62ac6824afe2400963bcb16`. Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stdout.tsv). SHA-256 `f83c20cd8f17f6d9fab6043284b656937ec5b0b03d3a962d525bd91b1e46e8cb`. Original counted rows including setup, input hex, code/result and available return options.
- `tcl8.4-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty native process stderr.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/receipt.json). SHA-256 `cc70385c42e4709b9f8d7814ac30c824f8725f5c79fb1ed6f9c23066a1fbccea`. Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stdout.tsv). SHA-256 `d1b8f71948ef8ed39ae300be118699f4a0443285f89eba7ebe6e4ef318a8c5ce`. Original counted rows including setup, input hex, code/result and available return options.
- `tcl8.5-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty native process stderr.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/receipt.json). SHA-256 `a34648a4427fe945c813c8f6c632540d1bde9011c4a8e5fba562635aaf87afba`. Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stdout.tsv). SHA-256 `39b5b868463fea94a1dc6c8c19de22bbbc9d952cc441dbdd422ec737087fa07a`. Original counted rows including setup, input hex, code/result and available return options.
- `tcl8.6-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty native process stderr.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/receipt.json). SHA-256 `e736e9265d0c926ae90bd8d9d6458f5e1c61977020629d9a9a3210dd1d8a89f2`. Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stdout.tsv). SHA-256 `907419a81bfbe910eef071353ca92d23cf443f5110cc7c61b3e02b742221315b`. Original counted rows including setup, input hex, code/result and available return options.
- `tcl9.0-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty native process stderr.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/receipt.json). SHA-256 `e034fec2cda67a1b5aef9c9d37407cb7c8d4d108d8fc6598c8dc69d59089e8eb`. Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stdout.tsv). SHA-256 `3f560052911517b782779577050c70f7126e9f979bdbb5b802bcbc720b82ec76`. Original counted rows including setup, input hex, code/result and available return options.
- `tcl9.1-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty native process stderr.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/receipt.json). SHA-256 `1dbe47ded617ced6885ef56248e3462b36fc5250155d03f4dbe8b45cca4f1c09`. Actual compiler/process commands, executable/library/header and exact stream hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stdout.tsv). SHA-256 `19839e6642b4e26221f1d0967a336cac774cee45192edd946fd98f571e1b97b4`. Original counted rows including setup, input hex, code/result and available return options.
- `jim-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty native process stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_info/methods.rs](../../../../rust/tcl-registry/src/native_tcloo_info/methods.rs), `NativeTclooMethodInfoProtocol::parse_original`: Actual original-option sequential table selection after target lookup.
- [rust/tcl-vm/src/cmd_oo/native_method_info_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_method_info_tests.rs), `cmd_oo::native_method_info_tests::method_info_matches_original_counted_option_and_scope_controls` (linked): 150 direct supported-C code/result controls; successful list members compared as sorted exact native bytes, errors as exact bytes. Does not assert physical options/header/CPP/Normal.
- [runtime/rust/src/cmd_oo/native_method_info_tests.rs](../../../../runtime/rust/src/cmd_oo/native_method_info_tests.rs), `cmd_oo::native_method_info_tests::method_info_matches_original_counted_option_and_scope_controls` (linked): 150 direct supported-C code/result controls; successful list members compared as sorted exact native bytes, errors as exact bytes. Does not assert physical options/header/CPP/Normal.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_tcloo_method_info_original/replay.py",
  "--verify-only"
]
```

Offline full-byte receipt/stream association only; zero guest, compiler or Rust launches. Re-execution needs every pinned header/build/library input in queue.json and fresh output path.
