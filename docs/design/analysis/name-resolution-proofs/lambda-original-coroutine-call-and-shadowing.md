# naming.lambda.original-coroutine-call-and-shadowing

Kind: `native-observation`

## Problem statement

Coroutine had a textual apply special case that bypassed the actual selected handler; serialized handoff could also replace the retained value across suspension.

## Question

Does native coroutine apply retain its original argument across yield and honor a shadowed apply handler?

## Conclusion

C8.6.18/9.0.4/9.1.0 return READY on suspension, then the same original argument pointer and bytes on resume for all seven values. Replacing apply with a source procedure makes coroutine call that handler and return SHADOW. C8.4/C8.5/current Jim report coroutine unavailable, and resume is explicitly not attempted.

## Scope

Seven original counted vectors per provider and one ASCII shadow source per provider.21 reached suspension/resume pairs and3 shadow controls. Unsupported providers are operation failures, not repaired setup. No arbitrary coroutine injection/frame introspection/trace/refcount/cache, complete options equivalence, custom callbacks or Normal certificate.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

Actual command unavailable; no entered body/resume claim.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

Actual command unavailable; no entered body/resume claim.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 return READY on suspension, then the same original argument pointer and bytes on resume for all seven values. Replacing apply with a source procedure makes coroutine call that handler and return SHADOW. C8.4/C8.5/current Jim report coroutine unavailable, and resume is explicitly not attempted.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 return READY on suspension, then the same original argument pointer and bytes on resume for all seven values. Replacing apply with a source procedure makes coroutine call that handler and return SHADOW. C8.4/C8.5/current Jim report coroutine unavailable, and resume is explicitly not attempted.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.6.18/9.0.4/9.1.0 return READY on suspension, then the same original argument pointer and bytes on resume for all seven values. Replacing apply with a source procedure makes coroutine call that handler and return SHADOW. C8.4/C8.5/current Jim report coroutine unavailable, and resume is explicitly not attempted.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Jim Tcl.

Actual command unavailable; no entered body/resume claim.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation.

## Exact evidence

- `probe-c` (input): [rust/tcl-registry/tests/data/native_apply_original_argv/probe.c](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/probe.c). SHA-256 `3fb4c2e90417f55475932a3748c104fdc306b6121073b8aae56eca8d2af8b58b`. Exact retained probe.c.
- `inputs-json` (input): [rust/tcl-registry/tests/data/native_apply_original_argv/inputs.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/inputs.json). SHA-256 `2ed850f8d4c873605510ba6e9ea96ae33bb3d67e0a7df8ad77b3e975bab2be0b`. Exact retained inputs.json.
- `capture-py` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/capture.py](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/capture.py). SHA-256 `c99a4f35b4130380eae7d314cddd58768b00c35b6d21da94a6c3d4c49d9f7194`. Exact retained capture.py.
- `queue-json` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/queue.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/queue.json). SHA-256 `31749e18925729df417fbded8c21e7f5af85ce7e2ee3835b6a38fa419d5ab4b6`. Exact retained queue.json.
- `receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/receipt.json). SHA-256 `c190dc22ee26da9cd0c5b8da09c7ebca500b1a466de51e3526b6874e35dad404`. Exact retained receipt.json.
- `tcl8.4-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/receipt.json). SHA-256 `d84ba7da7e7feebe78f58867bc8b7b2ed9a145e30a2ecb9fa4c3b8a5e0e0e246`. Original provider capture association only; receipt.json.
- `tcl8.4-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stdout.tsv). SHA-256 `289197e0e787144ef6a57776ba95e5b4a0e2b3a4b6c6daf5603cfa16f48c0d1b`. Original provider capture association only; stdout.tsv.
- `tcl8.4-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; stderr.
- `tcl8.4-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stdout.
- `tcl8.4-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stderr.
- `tcl8.5-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/receipt.json). SHA-256 `743ff6e037f901a53664e10d6f6d0a31a279475dba9a9ee632047ae78a5bfbdf`. Original provider capture association only; receipt.json.
- `tcl8.5-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stdout.tsv). SHA-256 `fda41904ac70b1570e82a4bc446b85ab68f96fc7dd1145dd4fd59a9abad91d94`. Original provider capture association only; stdout.tsv.
- `tcl8.5-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; stderr.
- `tcl8.5-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stdout.
- `tcl8.5-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stderr.
- `tcl8.6-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/receipt.json). SHA-256 `58a11f8040a54aca1d48947aa8b20a2c5d513ad240fcca37d1b60a079a2b8cd9`. Original provider capture association only; receipt.json.
- `tcl8.6-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv). SHA-256 `97e36c5e79b6f57fafed8c26f102d6e4d96f5f6d5ba2ceedefb320bcc8241670`. Original provider capture association only; stdout.tsv.
- `tcl8.6-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; stderr.
- `tcl8.6-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stdout.
- `tcl8.6-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stderr.
- `tcl9.0-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/receipt.json). SHA-256 `81b9eeedf93eca2e90620258d795ab6f3873c60af002d7c855ea13c6f74cb818`. Original provider capture association only; receipt.json.
- `tcl9.0-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv). SHA-256 `e3878d6b172017916379ea63e6e8a7fed42eb6c0abf5102b59352b6dbdecb630`. Original provider capture association only; stdout.tsv.
- `tcl9.0-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; stderr.
- `tcl9.0-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stdout.
- `tcl9.0-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stderr.
- `tcl9.1-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/receipt.json). SHA-256 `57a66d0de786ad2174f334565b20a93125caa103e9edd4594f41b347798ce986`. Original provider capture association only; receipt.json.
- `tcl9.1-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv). SHA-256 `7db32f0859e1ea72a72b664f540f1c1de0ef2147647229ad1e6452f91201bd02`. Original provider capture association only; stdout.tsv.
- `tcl9.1-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; stderr.
- `tcl9.1-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stdout.
- `tcl9.1-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stderr.
- `jim-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/receipt.json). SHA-256 `b8b79714cc19d9d4fc5c33156a97e486b1e8b85f2cb2dce5a41df23d7c3680be`. Original provider capture association only; receipt.json.
- `jim-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv). SHA-256 `e238237b1282d3980fd2604aebef334d2e9f3dc2bbe7d746fc83d68f44bd9c87`. Original provider capture association only; stdout.tsv.
- `jim-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; stderr.
- `jim-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stdout). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stdout.
- `jim-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Original provider capture association only; compile.stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/cmd_coro.rs](../../../../rust/tcl-vm/src/cmd_coro.rs), `cmd_coroutine`: Dispatches the original requested command operand and unchanged argv; textual apply recognition supplies no handler identity.
- [runtime/rust/src/cmd_coro.rs](../../../../runtime/rust/src/cmd_coro.rs), `coroutine_cmd`: Runs the selected original command and argv across the existing serialised coroutine handoff.
- [rust/tcl-vm/src/command/native_apply_original_tests.rs](../../../../rust/tcl-vm/src/command/native_apply_original_tests.rs), `command::native_apply_original_tests::apply_coroutine_keeps_arguments_and_selects_the_actual_shadowed_handler` (linked): 21 suspension starts,21 resumed original-object returns and three shadowed-apply handler controls for C8.6/C9.0/C9.1. Asserts code, pre-getter result identity and counted result bytes; no commandless cache, refcount or custom-release assertion. Runtime selector is non-WASM.
- [runtime/rust/src/cmd_proc/native_apply_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_apply_original_tests.rs), `cmd_proc::native_apply_original_tests::apply_coroutine_keeps_arguments_and_selects_the_actual_shadowed_handler` (linked): 21 suspension starts,21 resumed original-object returns and three shadowed-apply handler controls for C8.6/C9.0/C9.1. Asserts code, pre-getter result identity and counted result bytes; no commandless cache, refcount or custom-release assertion. Runtime selector is non-WASM.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3 rust/tcl-registry/tests/data/native_apply_original_argv/replay.py --verify-only"
]
```

Exact original input/stream/receipt hash and named pointer/result fields verified offline. Native replay requires every original provider input pinned in queue.json; no native or Rust pass inferred from verification.
