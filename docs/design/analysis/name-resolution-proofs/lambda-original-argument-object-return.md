# naming.lambda.original-argument-object-return

Kind: `native-observation`

## Problem statement

VM apply serialized caller arguments into source words, replacing original object identity and requiring Unicode of opaque original values.

## Question

Does apply of a whole-object return lambda retain the original argument object and bytes?

## Conclusion

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

## Scope

Seven ordinary original counted string-object arguments with lambda {x} {return $x}:35 successful direct controls plus7 unavailable C84 controls. No custom object hooks, type/header/refcount/cache, commandless Proc storage, compiler admission, general completion equivalence, namespace or Document/source ingress claim.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

Actual command unavailable; no entered body/resume claim.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Reported actual patchlevel; configured header/Makefile/library/source/probe/executable hashes and compile/process0 recorded.. Channel: Direct counted original string-object vector through Tcl_EvalObjv/Jim_EvalObjVector; shadow control ASCII source through Tcl_Eval/Jim_Eval. Pointer comparison before result getter. C85+ return-options bytes retained, C84/Jim options API unqueried.. Dialect: Jim Tcl.

C8.5.19/8.6.18/9.0.4/9.1.0 and Jim0.84-9-g5bac7c9 return the exact original argument pointer for all seven measured counted string values. The pointer comparison precedes the only result getter. Raw zero, encoded C080, FF, D800 and supplementary bytes remain their separate original values. C8.4 reports apply unavailable.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation.

## Historical references

- `probe-c` (input): [rust/tcl-registry/tests/data/native_apply_original_argv/probe.c](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/probe.c). Exact retained probe.c.
- `inputs-json` (input): [rust/tcl-registry/tests/data/native_apply_original_argv/inputs.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/inputs.json). Exact retained inputs.json.
- `capture-py` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/capture.py](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/capture.py). Exact retained capture.py.
- `queue-json` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/queue.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/queue.json). Exact retained queue.json.
- `receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/receipt.json). Exact retained receipt.json.
- `tcl8.4-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/receipt.json). Original provider capture association only; receipt.json.
- `tcl8.4-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stdout.tsv). Original provider capture association only; stdout.tsv.
- `tcl8.4-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/stderr). Original provider capture association only; stderr.
- `tcl8.4-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stdout). Original provider capture association only; compile.stdout.
- `tcl8.4-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.4.20/compile.stderr). Original provider capture association only; compile.stderr.
- `tcl8.5-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/receipt.json). Original provider capture association only; receipt.json.
- `tcl8.5-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stdout.tsv). Original provider capture association only; stdout.tsv.
- `tcl8.5-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/stderr). Original provider capture association only; stderr.
- `tcl8.5-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stdout). Original provider capture association only; compile.stdout.
- `tcl8.5-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.5.19/compile.stderr). Original provider capture association only; compile.stderr.
- `tcl8.6-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/receipt.json). Original provider capture association only; receipt.json.
- `tcl8.6-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stdout.tsv). Original provider capture association only; stdout.tsv.
- `tcl8.6-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/stderr). Original provider capture association only; stderr.
- `tcl8.6-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stdout). Original provider capture association only; compile.stdout.
- `tcl8.6-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/8.6.18/compile.stderr). Original provider capture association only; compile.stderr.
- `tcl9.0-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/receipt.json). Original provider capture association only; receipt.json.
- `tcl9.0-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stdout.tsv). Original provider capture association only; stdout.tsv.
- `tcl9.0-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/stderr). Original provider capture association only; stderr.
- `tcl9.0-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stdout). Original provider capture association only; compile.stdout.
- `tcl9.0-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.0.4/compile.stderr). Original provider capture association only; compile.stderr.
- `tcl9.1-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/receipt.json). Original provider capture association only; receipt.json.
- `tcl9.1-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stdout.tsv). Original provider capture association only; stdout.tsv.
- `tcl9.1-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/stderr). Original provider capture association only; stderr.
- `tcl9.1-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stdout). Original provider capture association only; compile.stdout.
- `tcl9.1-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/9.1.0/compile.stderr). Original provider capture association only; compile.stderr.
- `jim-receipt-json` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/receipt.json). Original provider capture association only; receipt.json.
- `jim-stdout-tsv` (observation): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/stdout.tsv). Original provider capture association only; stdout.tsv.
- `jim-stderr` (limitation): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/stderr). Original provider capture association only; stderr.
- `jim-compile-stdout` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stdout](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stdout). Original provider capture association only; compile.stdout.
- `jim-compile-stderr` (provider): [rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stderr](../../../../rust/tcl-registry/tests/data/native_apply_original_argv/jim/compile.stderr). Original provider capture association only; compile.stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::defer_original_lambda_call`: Retains the original invoked operand and argv objects for the existing independently admitted procedure activation; no source reserialization.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `apply_cmd`: Selects the actual lambda object and original argument vector at the runtime handler boundary.
- [rust/tcl-vm/src/command/native_apply_original_tests.rs](../../../../rust/tcl-vm/src/command/native_apply_original_tests.rs), `command::native_apply_original_tests::apply_returns_original_counted_argument_objects` (linked): 35 direct original-object code, pre-getter identity and counted result-byte comparisons for C8.5/C8.6/C9.0/C9.1/Jim; C8.4 unsupported remains outside this selector. No refcount, custom callback, cache, header or completion-options assertion.
- [runtime/rust/src/cmd_proc/native_apply_original_tests.rs](../../../../runtime/rust/src/cmd_proc/native_apply_original_tests.rs), `cmd_proc::native_apply_original_tests::apply_returns_original_counted_argument_objects` (linked): 35 direct original-object code, pre-getter identity and counted result-byte comparisons for C8.5/C8.6/C9.0/C9.1/Jim; C8.4 unsupported remains outside this selector. No refcount, custom callback, cache, header or completion-options assertion.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3 rust/tcl-registry/tests/data/native_apply_original_argv/replay.py --verify-only"
]
```

Exact original input/stream/receipt hash and named pointer/result fields verified offline. Native replay requires every original provider input pinned in queue.json; no native or Rust pass inferred from verification.
