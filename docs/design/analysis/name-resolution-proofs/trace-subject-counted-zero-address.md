# naming.variable.trace-subject-counted-zero-address

Kind: `native-observation`

## Problem statement

Trace subject names pass through a CString registration purpose, while ordinary variable names and prefix values have separate counted purposes. A raw counted zero can suppress an apparent array suffix; a binary zero object can materialize encoded C080 instead. Conflating these inputs can attach a trace to the wrong root or donate prefix semantics to a subject address.

## Question

How do raw counted v00tail(k), numeric escaped zero and binary00-derived subject values register, report, trigger and remove variable traces, with plain v and a distinct raw-zero suffix as controls?

## Conclusion

All C5 register the raw counted-zero subject on scalar v; plain v triggers that trace and an alternate raw-zero suffix selects/removes it. Numeric escaped zero and binary00-derived values materialize C080 on all C5 and register the distinct element k; plain v does not trigger them and raw-alternate removal leaves them intact. C8.4 differs on the subsequent raw original Set error, which is retained independently. Jim rejects trace registration; its name materialization rows do not establish trace address semantics.

## Scope

Original explicit-count string subjects and independent numeric-escape/binary source-produced subjects, whose reached string bytes are observed before public trace object-vector invocation. C8.4 legacy trace variable/vinfo/vdelete and C8.5+ add/info/remove forms. ASCII setup source has explicit C EvalEx length (Jim Eval uses NUL-terminated source). Callback argument hex and caught guest results retained. No arbitrary object updater, Native variable/header, copied prefix or full read/write-normal grant.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual public C/Jim probe executable SHA-256 61aea1bb312fd718a670b18f073469efed397beccbf90a737cd3b6c9394efa69; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Dialect: Tcl.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set after plain v errors variable is not array.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual public C/Jim probe executable SHA-256 6b385b8c44237a14121465546399e559fe0ee63b44262c05d7e822b17f8d8992; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Dialect: Tcl.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set returns VALUE without firing that scalar trace.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual public C/Jim probe executable SHA-256 d591fc02d6b0d2e0842e653e01d70bc8ecbe0f680797141b0cd3d76f7e8e20d8; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Dialect: Tcl.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set returns VALUE without firing that scalar trace.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual public C/Jim probe executable SHA-256 c1c5c80dac03446897428183d1898d4f8aba68b040820e53235472737308cb91; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Dialect: Tcl.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set returns VALUE without firing that scalar trace.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual public C/Jim probe executable SHA-256 5801d8f730c9689e2485e17854b2ab97f9e0e54f78c942dd031c327792cda501; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Dialect: Tcl.

RAW_COUNTED registers scalar v; plain v fires name1 hex76 and no index; alternate raw-zero suffix selects/removes the same registration. NUMERIC_ESCAPE and BINARY00 both materialize 76c0807461696c286b29 and register element k; plain v does not fire them and raw alternate removal leaves them present. Raw original Set returns VALUE without firing that scalar trace.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual public C/Jim probe executable SHA-256 a89489ca13323bbfa54a0e5ed3c2f8e0c13ec4cedb7baff63d084dc9c9551cb8; independent original header/library/Makefile/source-owner hashes retained. Actual info patchlevel result is retained.. Channel: explicit-count Tcl_NewStringObj/Jim_NewStringObj raw subject; escaped/binary subjects produced by explicit C source calls; public object-vector trace/set/query invocations. Dialect: Jim Tcl.

All three subject values materialize hex76007461696c286b29, but each trace registration returns guest error invalid command name trace; no subject-address/trigger behavior is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this exact question.

## Exact evidence

- `input` (input): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/trace-subject-probe.c](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/trace-subject-probe.c). SHA-256 `f8f469dc3047bcdccad5842e591a829088d6d304583761f57a7e71462bc807ae`. Exact immutable public C/Jim object-vector probe; raw subject extent supplied explicitly and escaped/binary subjects independently produced.
- `tcl8.4-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/receipt.json). SHA-256 `3894a288ecf53e55462db6326668d1e2fbff60b0ff0dbab0395b5f2a93feb624`. JSON pointer `/trace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl8.4-trace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/trace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/trace.stdout). SHA-256 `0e53dbd727610e50d266ab369dd3c6a5b774bbfdcd12246f9763f60fb660f717`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.4-trace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/trace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/trace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.4-compile.stdout` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl8.4-compile.stderr` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl8.5-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/receipt.json). SHA-256 `748bcd7b740b18871429681b33ff65cd542093889f86488140fee3db969317ea`. JSON pointer `/trace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl8.5-trace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/trace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/trace.stdout). SHA-256 `c887a005749f11d2d5ead20d3e1aa7d6dcfcb1d925334664759705a65f569ea4`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.5-trace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/trace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/trace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.5-compile.stdout` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl8.5-compile.stderr` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl8.6-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/receipt.json). SHA-256 `7f8d85a9efaa740768d94faf3d442da13951ddc251b4824a41a55c6f28a5a04c`. JSON pointer `/trace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl8.6-trace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/trace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/trace.stdout). SHA-256 `8cd248b7727f2369c0b358c30a665f5297d8fccf1501dcb08ccf9e24ac959d55`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.6-trace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/trace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/trace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl8.6-compile.stdout` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl8.6-compile.stderr` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl9.0-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/receipt.json). SHA-256 `221390c1d3e2f71689b649043e485da290603d5ae7266f9465109002722f7514`. JSON pointer `/trace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl9.0-trace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/trace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/trace.stdout). SHA-256 `83de553af444613a03c03e78a6bce73625d7cd4e4d5fc6f5fb73e9587a3a3c87`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.0-trace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/trace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/trace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.0-compile.stdout` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl9.0-compile.stderr` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl9.1-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/receipt.json). SHA-256 `d80dd5e75bedca0c65f99420f2451069ab0d301522002fc0204bbf3c341e815e`. JSON pointer `/trace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `tcl9.1-trace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/trace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/trace.stdout). SHA-256 `4e050681a0ee7e716f8c0c82404d97c0f3fa3dc49eb760c8547c6adf22461936`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.1-trace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/trace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/trace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `tcl9.1-compile.stdout` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `tcl9.1-compile.stderr` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `jim-receipt.json` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/receipt.json](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/receipt.json). SHA-256 `9d3d471970950421fc1b5155d444a40b2338bb82397156b03a27d6485ce26718`. JSON pointer `/trace`. Actual original provider/source/header/library/Makefile/executable/process correspondence.
- `jim-trace.stdout` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/trace.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/trace.stdout). SHA-256 `c3024f81691501b7199d23e92e497f8bacfd208ca66ae93fd3d2b50447476032`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `jim-trace.stderr` (observation): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/trace.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/trace.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original native process stream; guest errors remain caught output and are separate from harness exit.
- `jim-compile.stdout` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/compile.stdout](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.
- `jim-compile.stderr` (provider): [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/compile.stderr](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original compiler stream; compile exit0 retained separately in the receipt.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::trace_registration_input`: Pure selected trace subject extent, independent of variable-root and prefix purposes.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::trace_query_input`: Pure selected trace info/removal subject extent without a physical receiver grant.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `naming::native::tests::trace_subject_registration_keeps_escaped_zero_distinct_from_raw_zero` (linked): Pure selected trace registration/query CString extent distinguishes raw-zero subject from C080 subject while ordinary variable-root extent remains independently selected; Jim trace purpose unavailable.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `variable_bindings::tests::original_trace_subject_keeps_cstring_purpose_separate_from_counted_runtime_lookup` (linked): The selected trace-subject CString purpose clips raw counted zero independently of ordinary counted C9 variable root/index; Jim trace registration remains unavailable. This is a Rust correspondence assertion, not physical registration or current-value proof.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::original_trace_subject_extent_preserves_counted_original_and_independent_array_form` (linked): Selected Registry trace projection keeps complete original subject bytes while its separate CString address extent distinguishes raw-zero from C080 and preserves independent array root/index form. This Rust correspondence binding supplies no physical registration, storage or Normal claim.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-namespace-store-trace-subject-reconfirmation"
]
```

Requires exact provider shell/header/static-library/source-owner/Makefile hashes and immutable input/probe bytes. Compiles the public probe, compares complete stdout/stderr and exit for both inputs. Guest errors are data; nonzero harness exit or stream mismatch fails replay. --verify-only checks source/provider/stream associations without compiling or launching native code. No Rust execution claim.
