# naming.tcloo.private-original-script-bytes

Kind: `native-observation`

## Problem statement

Converting a counted private body into a display string can change raw ff, an opaque surrogate or a binary zero in a method name. A valid Unicode display is insufficient evidence for native script bytes. The exact private method and my wrapper controls distinguish byte preservation from replacement or clipping on C9.

## Question

Can a counted original private body declare and invoke methods whose names contain raw ff, native surrogate bytes or binary zero?

## Conclusion

C9.0/C9.1 accepts all three counted original bodies and their matching my calls return OK. The captured -private roster reports only the public invoke wrapper, keeping true private declaration distinct from ordinary unexporting. Earlier C versions and Jim report the worker absent. The measured result and roster do not grant native compiler, cache, header or editable geometry authority.

## Scope

Complete v2/v3 public C original-object probes with ASCII setup plus ff, eda080 and raw-zero counted method names or operation words. Five linked Tcl providers and one Jim startup/control provider are independently recorded. No document-channel, physical cache/header, arbitrary observer or Rust execution conclusion.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual probe executable; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual probe executable; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Actual probe executable; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual probe executable; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

All three PRIVATE_RAW_* definitions and matching private my invocations succeed; PRIVATE_RAW_*_NAMES contains invoke only.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual probe executable; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

All three PRIVATE_RAW_* definitions and matching private my invocations succeed; PRIVATE_RAW_*_NAMES contains invoke only.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual probe executable; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: ASCII NUL-terminated source passed to Jim_Eval. Dialect: Jim Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

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

- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `cmd_private`: Enter the original script object through the selected body evaluator; retain byte-based inline worker lookup.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_private`: Evaluate the original control-body object instead of a detached byte or display copy.
- [rust/tcl-vm/src/cmd_oo/native_private_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::private_query_and_opaque_original_bodies_match_native_c9_controls` (linked): Compare captured query and private opaque method invocation results on the independently selected C9.0/C9.1 VM cores. This test does not assert complete private-roster parity.
- [runtime/rust/src/cmd_oo/native_private_tests.rs](../../../../runtime/rust/src/cmd_oo/native_private_tests.rs), `cmd_oo::native_private_tests::private_query_and_opaque_original_bodies_match_native_c9_controls` (linked): Compare mode query, opaque original body definitions, true-private roster and matching my invocation results on independently selected C9.0/C9.1 runtime cores.

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
