# naming.tcloo.method-info-option-selection

Kind: `native-observation`

## Problem statement

Method introspection must select original option objects under the provider-specific counted or prefix recipe and preserve the handler error order. Display-string decoding or comparison over an unrelated full extent can conflate raw zero, encoded zero and different option-table matches. Missing targets and malformed options require separate stage evidence.

## Question

How do original counted C TclOO method options select prefixes, raw zero and encoded C080, and does target lookup precede option parsing?

## Conclusion

For C8.6.18/9.0.4/9.1.0, the measured direct and counted-source options use CString unique-prefix Index selection: -private with raw-zero suffix succeeds, encoded C080 suffix fails, and the one-character - prefix is ambiguous. Missing target lookup fails before bad-option or missing-scope validation. C8.6 rejects -scope. C8.4/C8.5/Jim fail OO setup and do not reach the measured method operation.

## Scope

25 fixed cases x object/class x direct counted original string-vector or counted source; 100 controls per provider, 300 reached controls on three C providers and 300 explicitly not-attempted setup controls. No Document/read-channel, custom object hook, cache/header/Index identity, method allocation, worker CPP, callback, normal-completion or optimizer assertion.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

For C8.6.18/9.0.4/9.1.0, the measured direct and counted-source options use CString unique-prefix Index selection: -private with raw-zero suffix succeeds, encoded C080 suffix fails, and the one-character - prefix is ambiguous. Missing target lookup fails before bad-option or missing-scope validation. C8.6 rejects -scope. C8.4/C8.5/Jim fail OO setup and do not reach the measured method operation.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

For C8.6.18/9.0.4/9.1.0, the measured direct and counted-source options use CString unique-prefix Index selection: -private with raw-zero suffix succeeds, encoded C080 suffix fails, and the one-character - prefix is ambiguous. Missing target lookup fails before bad-option or missing-scope validation. C8.6 rejects -scope. C8.4/C8.5/Jim fail OO setup and do not reach the measured method operation.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Tcl.

For C8.6.18/9.0.4/9.1.0, the measured direct and counted-source options use CString unique-prefix Index selection: -private with raw-zero suffix succeeds, encoded C080 suffix fails, and the one-character - prefix is ambiguous. Missing target lookup fails before bad-option or missing-scope validation. C8.6 rejects -scope. C8.4/C8.5/Jim fail OO setup and do not reach the measured method operation.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Recorded configured static library/header/Makefile and actual probe executable digests; compile0/process0.. Channel: Direct original counted string-object vector and counted source through Tcl_EvalEx; Jim setup through Jim_EvalObj.. Dialect: Jim Tcl.

OO setup returned guest error on every case; method operation explicitly NOT_ATTEMPTED. This measures setup unavailability, no method selector/roster answer.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation.

## Historical references

- `probe` (input): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/probe.c](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/probe.c). Exact retained C/Jim original-vector and counted-source harness.
- `inputs` (input): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/inputs.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/inputs.json). Fixed case labels and exact option hex, input channels and limits.
- `queue` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/queue.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/queue.json). Pinned headers, configured makefiles, static libraries, sources and compiler/link environment.
- `aggregate` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/receipt.json). All six original compile/process receipts, reported patchlevels and named protocol rows.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/receipt.json). Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stdout.tsv). Original counted rows including setup, input hex, code/result and available return options.
- `tcl8.4-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.4.20/stderr). Exact empty native process stderr.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/receipt.json). Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stdout.tsv). Original counted rows including setup, input hex, code/result and available return options.
- `tcl8.5-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.5.19/stderr). Exact empty native process stderr.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/receipt.json). Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stdout.tsv). Original counted rows including setup, input hex, code/result and available return options.
- `tcl8.6-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/8.6.18/stderr). Exact empty native process stderr.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/receipt.json). Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stdout.tsv). Original counted rows including setup, input hex, code/result and available return options.
- `tcl9.0-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.0.4/stderr). Exact empty native process stderr.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/receipt.json). Actual compiler/process commands, executable/library/header and exact stream hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stdout.tsv). Original counted rows including setup, input hex, code/result and available return options.
- `tcl9.1-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/9.1.0/stderr). Exact empty native process stderr.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/receipt.json). Actual compiler/process commands, executable/library/header and exact stream hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stdout.tsv). Original counted rows including setup, input hex, code/result and available return options.
- `jim-stderr` (limitation): [rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_tcloo_method_info_original/jim/stderr). Exact empty native process stderr.

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
