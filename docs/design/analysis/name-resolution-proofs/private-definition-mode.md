# naming.tcloo.private-definition-mode

Kind: `native-observation`

## Problem statement

A private modifier with no arguments may be a query or an arity error, and nested modifiers must restore the prior definition mode. Using an empty result or sticky depth changes surrounding declarations. These controls apply to actual admitted C9 definition frames; they do not attest arbitrary source or object caches.

## Question

Does the actual private definition worker report and restore its mode before, inside and after a private block?

## Conclusion

C9.0/C9.1 returns 0 1 0 before, inside and after the block. The worker is absent in the measured C8.4-C8.6 and Jim installations. This is an actual definition-frame mode observation; no editable source, method-header or compiler capability follows.

## Scope

Complete v2/v3 public C original-object probes with ASCII setup plus ff, eda080 and raw-zero counted method names or operation words. Five linked Tcl providers and one Jim startup/control provider are independently recorded. No document-channel, physical cache/header, arbitrary observer or Rust execution conclusion.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Actual probe executable SHA-256 e0eb871dc216faa2d8d144b7de65341657b7c7e4b13f907a5173909d4a4657f7; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Actual probe executable SHA-256 00f4f9f3c90f57e134da8dfb6d1d9f0d37d353e76e5854f88ee47b12641ac31f; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Actual probe executable SHA-256 15a2f964f29869af37504da2aa6980fcd1b3a64c183992d9894cd69222fcc759; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_AVAILABLE=0: the measured private definition worker is absent.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual probe executable SHA-256 9aac75925255a53e2ad858c3996db3e78e8d9fe61e868e3e91bec1bf03ecda63; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_QUERY=0 1 0.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual probe executable SHA-256 09a61d2fb42ee2112efe02f57b1af88c4e1a47b9e51d479e9aa29dbfa6b58282; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: Original Tcl_NewStringObj(pointer, count) operands passed to public Tcl_EvalObjv; ASCII Tcl_Eval setup and query source. Dialect: Tcl.

PRIVATE_QUERY=0 1 0.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Actual probe executable SHA-256 ce8739719e7104799b5d524c57c22e89ff99ea6a2367cb7c136fded79939ee58; compile command, public header, static library, Makefile, source-owner and process stream hashes retained. Executed info patchlevel reports this version.. Channel: ASCII NUL-terminated source passed to Jim_Eval. Dialect: Jim Tcl.

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

- [rust/tcl-vm/src/cmd_oo.rs](../../../../rust/tcl-vm/src/cmd_oo.rs), `cmd_private`: Query and restore the actual private definition mode independently of body byte evaluation.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `def_private`: Report the authenticated current private definition mode.
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
