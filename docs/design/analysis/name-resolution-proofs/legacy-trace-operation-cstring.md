# naming.variable.legacy-trace-operation-cstring

Kind: `native-observation`

## Problem statement

The legacy operation object has a counted native string but its letters are scanned as a CString. A raw zero and encoded zero therefore select different operation sets and errors. Treating the display or full count as the operation word changes registration and diagnostics. This question is confined to the measured legacy Tcl trace forms, not copied callback-prefix storage.

## Question

How do legacy trace variable operation words distinguish plain w, w plus raw zero, encoded zero, raw ff and a single binary zero?

## Conclusion

C8.4-C8.6 accepts plain w and w plus raw zero as write; encoded zero and raw ff fail with their exact original bytes in the legacy message. A single binary zero is counted nonempty, then delegates an empty operation list and reports the modern empty-list error. C9 rejects the retired variable/vinfo forms; Jim rejects trace. These results do not prove errorCode, callback execution or prefix-copy semantics.

## Scope

Complete v2/v3 public C original-object probes with ASCII setup plus ff, eda080 and raw-zero counted method names or operation words. Five linked Tcl providers and one Jim startup/control provider are independently recorded. No document-channel, physical cache/header, arbitrary observer or Rust execution conclusion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual probe executable SHA-256 e0eb871dc216faa2d8d144b7de65341657b7c7e4b13f907a5173909d4a4657f7; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

Plain and raw-zero suffix w register write; C080 and ff fail byte-exactly; initial raw zero yields modern empty operation-list error.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual probe executable SHA-256 00f4f9f3c90f57e134da8dfb6d1d9f0d37d353e76e5854f88ee47b12641ac31f; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

Plain and raw-zero suffix w register write; C080 and ff fail byte-exactly; initial raw zero yields modern empty operation-list error.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual probe executable SHA-256 15a2f964f29869af37504da2aa6980fcd1b3a64c183992d9894cd69222fcc759; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

Plain and raw-zero suffix w register write; C080 and ff fail byte-exactly; initial raw zero yields modern empty operation-list error.

### tcl9.0

Status: `unsupported`. Version: 9.0.4. Build: Actual probe executable SHA-256 9aac75925255a53e2ad858c3996db3e78e8d9fe61e868e3e91bec1bf03ecda63; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

Legacy variable and vinfo are rejected as retired options.

### tcl9.1

Status: `unsupported`. Version: 9.1.0. Build: Actual probe executable SHA-256 09a61d2fb42ee2112efe02f57b1af88c4e1a47b9e51d479e9aa29dbfa6b58282; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

Legacy variable and vinfo are rejected as retired options.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual probe executable SHA-256 ce8739719e7104799b5d524c57c22e89ff99ea6a2367cb7c136fded79939ee58; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: ASCII NUL-terminated source passed to Jim_Eval. Dialect: Jim Tcl.

The actual trace command is unavailable: invalid command name trace.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation for this exact question.

## Historical references

- `v2-input` (input): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/probe.c](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/probe.c). Exact public API probe source; Tcl objects use explicit counted original bytes. Jim source controls use Jim_Eval.
- `v2-aggregate` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/receipt.json). Original complete six-provider correspondence, independently preserving this exact probe variant.
- `v3-input` (input): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/probe.c](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/probe.c). Exact public API probe source; Tcl objects use explicit counted original bytes. Jim source controls use Jim_Eval.
- `v3-aggregate` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/receipt.json). Original complete six-provider correspondence, independently preserving this exact probe variant.
- `v2-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.4.20/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.4.20/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.4.20/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.4-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.4.20/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.4.20/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.4-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.4.20/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.4.20/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.4-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.4.20/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.4.20/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.5.19/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.5.19/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.5.19/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.5-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.5.19/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.5.19/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.5-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.5.19/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.5.19/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.5-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.5.19/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.5.19/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.6.18/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.6.18/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/8.6.18/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.6-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.6.18/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.6.18/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.6-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.6.18/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.6.18/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl8.6-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.6.18/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/8.6.18/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.0.4/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.0.4/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.0.4/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl9.0-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.0.4/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.0.4/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl9.0-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.0.4/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.0.4/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl9.0-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.0.4/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.0.4/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.1.0/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.1.0/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/9.1.0/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl9.1-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.1.0/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.1.0/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl9.1-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.1.0/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.1.0/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-tcl9.1-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.1.0/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/9.1.0/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/jim/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/jim/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v2-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v2/jim/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v2/jim/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-jim-receipt.json` (provider): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/jim/receipt.json](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/jim/receipt.json). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-jim-stdout.tsv` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/jim/stdout.tsv](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/jim/stdout.tsv). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.
- `v3-jim-stderr` (observation): [rust/tcl-vm/tests/data/native_private_legacy_trace/v3/jim/stderr](../../../../rust/tcl-vm/tests/data/native_private_legacy_trace/v3/jim/stderr). Exact original provider receipt or native process stream; guest failures remain result rows and are distinct from harness exit.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `parse_legacy_variable_ops_original`: Materialise the original native operation string, then apply the counted-empty and CString letter purposes.
- [rust/tcl-vm/src/cmd_trace.rs](../../../../rust/tcl-vm/src/cmd_trace.rs), `legacy_variable`: Use the actual original operation object instead of its display string.
- [runtime/rust/src/cmd_trace.rs](../../../../runtime/rust/src/cmd_trace.rs), `legacy_var_add_remove`: Use the independently selected original string getter before shared legacy parsing.
- [rust/tcl-vm/src/cmd_trace/native_original_tests.rs](../../../../rust/tcl-vm/src/cmd_trace/native_original_tests.rs), `cmd_trace::native_original_tests::legacy_operation_objects_match_native_counted_and_cstring_controls` (linked): Compare the measured operation/result bytes and stored legacy trace report for original counted inputs; keep binary zero distinct from C080 and opaque ff.
- [rust/tcl-cmd-core/src/trace.rs](../../../../rust/tcl-cmd-core/src/trace.rs), `trace::tests::legacy_operation_extent_preserves_counted_empty_and_opaque_errors` (linked): Compare the measured operation/result bytes and stored legacy trace report for original counted inputs; keep binary zero distinct from C080 and opaque ff.
- [runtime/rust/src/cmd_trace/native_command_tests.rs](../../../../runtime/rust/src/cmd_trace/native_command_tests.rs), `cmd_trace::native_command_tests::legacy_operation_objects_match_native_counted_and_cstring_controls` (linked): Compare the observed counted raw-zero, C080 and ff operation errors plus actual stored legacy report on C8.4-C8.6; original operations use selected native getters.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-vm/tests/data/native_private_legacy_trace/replay.py",
  "--c-root",
  "tmp",
  "--jim-root",
  "/workspace/.proofs/native-providers/jimtcl",
  "--output",
  "/tmp/native-private-legacy-trace-reconfirmation"
]
```

Requires exact pinned public header/static-library/Makefile/source-owner and input hashes. Recompiles both exact probe variants and compares complete exit/stdout/stderr. Guest error rows are expected data, not harness failures. --verify-only checks bytes without native or Rust execution. ErrorCode and cache/refcount/header effects were not inspected. The incomplete first metadata attempt is retained separately without a full provider attribution.
