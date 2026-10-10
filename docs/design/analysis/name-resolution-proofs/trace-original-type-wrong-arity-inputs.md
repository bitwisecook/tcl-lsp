# naming.trace.original-type-wrong-arity-inputs

Kind: `native-observation`

## Problem statement

An abbreviated trace type may contain raw zero plus opaque bytes. String reconstruction can panic or print the original abbreviation, while encoded zero can follow a different native lookup path. Source spelling and direct counted object argv must remain distinct.

## Question

For these exact original argv and counted-source controls, which trace type operands reach canonical wrong-arity reporting and which fail native type selection?

## Conclusion

All five C providers select var/com/exec prefixes for direct ASCII, direct raw00FF, ordinary ASCII source and counted source raw00FF, then return guest error1 with canonical variable/command/execution usage headers. Direct C080FF fails type lookup instead. Jim reports trace unavailable in all30 controls. C8.5+ return-options bytes are retained separately; C8.4 and Jim C-return-options API absence is explicit. The thirty raw-zero direct C result controls bind the VM regression; this supplies no Rust pass, trace registration/callback effect or whole options equivalence claim.

## Scope

Thirty finite controls per actual C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9:3type prefixes×2members×5independent input modes. Six builds/processes successful;180 controls and546 protocol rows including version/input/result/options metadata. Direct object argv versus counted source remain separate; no Document decoding or trace callback/registration behavior measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual compile0/process0; exact command, linked executable/header/Makefile/source/static-library SHA retained in provider receipt.. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands.. Dialect: Tcl.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guest code 1 in all thirty. Exact result bytes and explicit no-Tcl_GetReturnOptions markers are retained; return-options bytes were not captured.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual compile0/process0; exact command, linked executable/header/Makefile/source/static-library SHA retained in provider receipt.. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands.. Dialect: Tcl.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guestcode1 in all30; exact result/options bytes retained.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual compile0/process0; exact command, linked executable/header/Makefile/source/static-library SHA retained in provider receipt.. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands.. Dialect: Tcl.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guestcode1 in all30; exact result/options bytes retained.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual compile0/process0; exact command, linked executable/header/Makefile/source/static-library SHA retained in provider receipt.. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands.. Dialect: Tcl.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guestcode1 in all30; exact result/options bytes retained.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual compile0/process0; exact command, linked executable/header/Makefile/source/static-library SHA retained in provider receipt.. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands.. Dialect: Tcl.

Twenty-four prefix-selection controls report canonical wrong-arity headers; six direct C080FF controls reject type lookup. Guestcode1 in all30; exact result/options bytes retained.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual compile0/process0; exact command, linked executable/header/Makefile/source/static-library SHA retained in provider receipt.. Channel: Separate original counted string-object argv and counted native source; ASCII/raw00FF/C080FF are distinct finite operands.. Dialect: Jim Tcl.

All30 guest controls report invalid command trace; no C-return-options API used.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `probe.c` (input): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/probe.c). SHA-256 `7ff952d7ae719441c5b9099551840f7358482cdf6ac463396c558aacf554e0ab`. Exact original probe or capture protocol for the finite original object/source input modes.
- `capture.py` (input): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/capture.py](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/capture.py). SHA-256 `0c6621451877c345527db6205bb5c31972f9a19f499a13275dd5ad956f77d375`. Exact original probe or capture protocol for the finite original object/source input modes.
- `queue.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/queue.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/queue.json). SHA-256 `a341b408e0eeddf5b526e95b17e1b84c2506e8f850aca329ed24915e7fa6e6c3`. Authored capture queue and build/input expectations; it supplies no guest outcome.
- `inputs.json` (input): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/inputs.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/inputs.json). SHA-256 `11f154abf91ddc4699b66c5debad690348c525c5dda7376ae1ed7ec0b535d9ad`. Exact fixed input/probe/capture protocol or offline local receipt verification; no additional launch.
- `replay.py` (reconfirmation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/replay.py](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/replay.py). SHA-256 `31923fb0dad4769932a0efa3ff31d31ebde9228a57ee75c8d3ccedff68a2f14e`. Exact fixed input/probe/capture protocol or offline local receipt verification; no additional launch.
- `aggregate` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/receipt.json). SHA-256 `dfa85792fa7f68203f4c6f2c80cbbdbf84a86c582dd0b43196a9c2ba797d5e86`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `tcl8.4-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/receipt.json). SHA-256 `9573acc3dc87ac8fbb79744e5a02577a40a88c86dac117450f80581272fe87f0`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `tcl8.4-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/stdout.tsv). SHA-256 `99fa9355620365d655131fb16f46d0fb24d785e97d49880f74040c44050f71d9`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.4-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained process stderr.
- `tcl8.4-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.4-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.5-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/receipt.json). SHA-256 `f6b9b468714ec7867cdb641c92cf1d6be01fffd55e5629e766e4b9e8ea12c317`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `tcl8.5-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/stdout.tsv). SHA-256 `1cb50f09940b31bb5b5a8468fb257577ee12152ddd280d4ff011e4b942afd7ed`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.5-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained process stderr.
- `tcl8.5-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.5-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.6-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/receipt.json). SHA-256 `acadec81cee1e15797c2147b7d4f14966ba409098a798696364bfe5c9e424ad1`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `tcl8.6-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/stdout.tsv). SHA-256 `37fddcc1da72c7be09f9161e8d45c3d501d4f876a2e6a4fc163a3819b1c57bb0`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.6-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained process stderr.
- `tcl8.6-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl8.6-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl9.0-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/receipt.json). SHA-256 `46c1e9c1be87856744dd20b4db09a4bfe9a31e369bb2db13136ed7e7c57f2b51`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `tcl9.0-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/stdout.tsv). SHA-256 `3d5182dc2ce89b13aec561aaccf476dc09524728040c839749d97512b3e1e78f`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl9.0-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained process stderr.
- `tcl9.0-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl9.0-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl9.1-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/receipt.json). SHA-256 `1df51e888fb344445e70904a13368b0602ec034a52505fb1beb1c171023a9677`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `tcl9.1-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/stdout.tsv). SHA-256 `08b1129c94c8fe5abd14d4030e08ef6bfecf30d6984c21c63c34d4a8a89a5392`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl9.1-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained process stderr.
- `tcl9.1-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `tcl9.1-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `jim-receipt.json` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/receipt.json](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/receipt.json). SHA-256 `7cd26bae4d97a545b19abfcc65a993ddb4ef707401c528236e10059ac519a278`. Actual capture attribution, build/input hashes, version and process outcome; exact guest stream is independent.
- `jim-stdout.tsv` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/stdout.tsv](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/stdout.tsv). SHA-256 `283ff1056b6abe967a1a2fba98f234a90a9e610860ccf2a80ef5be3df0b2c138`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `jim-stderr` (observation): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact retained process stderr.
- `jim-compile.stdout` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/compile.stdout](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.
- `jim-compile.stderr` (provider): [rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/compile.stderr](../../../../rust/tcl-cmd-core/tests/data/native_trace_selected_type/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original per-provider input, result, option/API availability and process/build bytes with exact linked version observation and source/header/library pin association.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `TraceKind::canonical_name`: Already-selected native type reporting only; actual lookup remains independent.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `trace_add_remove`: Use canonical selected type for add/remove usage headers without String decoding.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `trace_info`: Use canonical selected type for info usage headers without String decoding.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `NativeIndexLookupProtocol::selected_input`: Pure extent of independently selected C Index input, shared by physical lookup; no materialization/cache/registration grant.
- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `resolve_type_bytes`: Pure actual trace table matcher over already selected bytes; original Index lookup remains independent.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::trace_type_wrong_arity_uses_selected_member_without_decoding_original_operand` (linked): Compares only30 original direct raw00FF C code/result controls retained in this capture; no option/callback or executed Rust parity claim.
- [rust/tcl-registry/src/native_index_lookup.rs](../../../../rust/tcl-registry/src/native_index_lookup.rs), `native_index_lookup::tests::selected_trace_type_bytes_keep_raw_and_encoded_zero_distinct` (linked): Implementation comparator preserves observed raw00FF versus encoded C080FF selector distinction for independently selected C5 recipe and refuses Jim Index recipe. It grants no getter/cache/registration or Rust pass.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3 rust/tcl-cmd-core/tests/data/native_trace_selected_type/replay.py --verify-only"
]
```

Offline verify-only matches exact local receipts/streams/probe/inputs and all180 input/result associations; it launches no guest. Original capture runner requires every pinned source/header/Makefile/static library and a fresh output directory. C84/Jim C return-options retrieval was not performed; no Rust test execution or registration/callback semantics inferred.
