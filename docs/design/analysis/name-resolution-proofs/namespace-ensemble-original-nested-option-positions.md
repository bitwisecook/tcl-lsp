# naming.namespace-ensemble.original-nested-option-positions

Kind: `native-observation`

## Problem statement

Nested namespace ensemble operations have different required positional operands and option boundaries. A dash-named required command can be data; treating it as an option changes the selected operation. An exists capability failure cannot establish that bare ensemble creation is unavailable.

## Question

Where do options begin in the fixed namespace ensemble create/configure/exists source forms, and what do dash-named commands and independently caught bare-create controls actually return?

## Conclusion

C8.5–9.1 accept create options at nested operand0, consume configure command data before options at operand1, and treat exists as a command-name query without an option scan. Dash-named required configure/exists commands retain data behavior. The extra exists argument reports NONE in C8.5 and TCL WRONGARGS in C8.6–9.1. C8.4 has no namespace ensemble operation in these captures. Current Jim supports bare ensemble creation and its exported invocation, refuses the C-style optioned create shape, and declines configure/exists with its only-create-supported error. Its exists-gated capture supplies no create answer by itself.

## Scope

Twelve actual source-file CLI processes retain twelve queried versions and 134 labelled caught observations across two exact ASCII scripts. Capture167 has an exists-gated option body; capture168 independently catches fourteen labels per provider, including bare creation and invocation. Each script/provider process shares its own ordered created commands and namespace state; these are not isolated original object-vector cases. Actual result strings, errorCode and capability boundaries remain limited to the fixed source. No opaque names, physical object/header/cache/pointer, compiler admission, arbitrary handler, whole-script Normal, edit equivalence or BIG-IP answer follows.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Original CLI executable; exact selected SDK/header/library/build/source associations retained. This runner executes no compiler command.. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms.. Dialect: C Tcl.

The capability and all independently caught ensemble forms fail because namespace ensemble is unavailable. Capture167 skips the option body; capture168 records every fixed refusal. No option-position or bare-creation behavior is inferred for C8.4.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Original CLI executable; exact selected SDK/header/library/build/source associations retained. This runner executes no compiler command.. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms.. Dialect: C Tcl.

Create accepts options immediately. Configure consumes its required command name first, including dash-named -map and -namespace commands, then options; the fixed map queries return x ::list. Exists consumes its command name and does not scan trailing options. Bare creation and invocation succeed. The extra exists argument returns errorCode NONE. Invalid -command for configure and -namespace for create retain their distinct actual option error rosters in the streams.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Original CLI executable; exact selected SDK/header/library/build/source associations retained. This runner executes no compiler command.. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms.. Dialect: C Tcl.

Create accepts options immediately. Configure consumes its required command name first, including dash-named -map and -namespace commands, then options; the fixed map queries return x ::list. Exists consumes its command name and does not scan trailing options. Bare creation and invocation succeed. The extra exists argument returns TCL WRONGARGS. Invalid -command for configure and -namespace for create retain their distinct actual option error rosters in the streams.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Original CLI executable; exact selected SDK/header/library/build/source associations retained. This runner executes no compiler command.. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms.. Dialect: C Tcl.

Create accepts options immediately. Configure consumes its required command name first, including dash-named -map and -namespace commands, then options; the fixed map queries return x ::list. Exists consumes its command name and does not scan trailing options. Bare creation and invocation succeed. The extra exists argument returns TCL WRONGARGS. Invalid -command for configure and -namespace for create retain their distinct actual option error rosters in the streams.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Original CLI executable; exact selected SDK/header/library/build/source associations retained. This runner executes no compiler command.. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms.. Dialect: C Tcl.

Create accepts options immediately. Configure consumes its required command name first, including dash-named -map and -namespace commands, then options; the fixed map queries return x ::list. Exists consumes its command name and does not scan trailing options. Bare creation and invocation succeed. The extra exists argument returns TCL WRONGARGS. Invalid -command for configure and -namespace for create retain their distinct actual option error rosters in the streams.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Original CLI executable; exact selected SDK/header/library/build/source associations retained. This runner executes no compiler command.. Channel: Two fixed ASCII LF source-file CLI scripts; one actual interpreter process per script/provider, ordered caught source forms.. Dialect: Jim.

Capture167 reports only [namespace ensemble create] is supported from exists, then skips subsequent forms. Capture168 directly records bare create code0 with r2286_nestedBare and invocation code0 with payload. C-style optioned create returns arity refusal; configure and exists return only-create-supported refusals. The ensemble surface is not wholly unavailable.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No BIG-IP attempt or corresponding source-file ensemble answer is attached.

## Historical references

- `nested-options-sdk-inputs` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/sdk-queue.json). JSON pointer `/providers`. Exact retained SDK queue. Only its providers array selects the CLI executable/environment/header/library/build/source associations; other queue cases supply no ensemble result.
- `nested-options-167-source` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/probe.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/probe.tcl). Exact ASCII LF source. Capture167 gates subsequent observations on exists capability; capture168 independently catches all fourteen labels including bare creation and its invocation.
- `nested-options-167-runner` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/capture.py). Exact original CLI capture runner; no compilation command or Rust assertions are executed by this runner.
- `nested-options-167-request` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/request.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/request.json). Original question/input request; this request supplies no execution outcome independently of actual per-provider receipts.
- `nested-options-168-source` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/probe.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/probe.tcl). Exact ASCII LF source. Capture167 gates subsequent observations on exists capability; capture168 independently catches all fourteen labels including bare creation and its invocation.
- `nested-options-168-runner` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/capture.py). Exact original CLI capture runner; no compilation command or Rust assertions are executed by this runner.
- `nested-options-168-request` (input): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/request.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/request.json). Original question/input request; this request supplies no execution outcome independently of actual per-provider receipts.
- `nested-options-167-8.4.20-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.4.20/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-167-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.4.20/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-167-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.4.20/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-168-8.4.20-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.4.20/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-168-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.4.20/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-168-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.4.20/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-167-8.5.19-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.5.19/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-167-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.5.19/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-167-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.5.19/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-168-8.5.19-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.5.19/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-168-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.5.19/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-168-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.5.19/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-167-8.6.18-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.6.18/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-167-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.6.18/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-167-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/8.6.18/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-168-8.6.18-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.6.18/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-168-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.6.18/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-168-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/8.6.18/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-167-9.0.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.0.4/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-167-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.0.4/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-167-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.0.4/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-168-9.0.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.0.4/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-168-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.0.4/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-168-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.0.4/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-167-9.1.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.1.0/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-167-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.1.0/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-167-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/9.1.0/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-168-9.1.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.1.0/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-168-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.1.0/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-168-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/9.1.0/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-167-jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/jim/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-167-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/jim/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-167-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v167/jim/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.
- `nested-options-168-jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/jim/receipt.json). Actual CLI command/process exit, queried version, exact source/runner/executable/SDK pins and raw-stream SHA joins.
- `nested-options-168-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/jim/stdout). Original version and ordered catch status/result/errorCode rows. The capture167 unsupported conclusion is only its exists-gated branch output; capture168 independently measures each label.
- `nested-options-168-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/jim/stderr). Exact empty stderr for this process; no wider observer, handler or completion closure is inferred.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_namespace_ensemble_options_original/v168/capture.py"
]
```

Each retained runner reads its original absolute SDK queue and creates new provider output directories. Replay requires exact independent SDK/executable/environment inputs and fresh output paths. Original request fields and broad raw branch labels do not override actual process receipts or independently caught rows. These source-file observations execute no Rust comparison and grant no original object-vector, Native compiler or edit permission.
