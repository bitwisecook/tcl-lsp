# naming.expression.original-implicit-function-slot-presence

Kind: `native-observation`

## Problem statement

Implicit expression calls do not own written command-head words. Determine whether a missing Pi function is a command-slot lookup and whether a published ::tcl::mathfunc::Pi procedure affects dispatch under each measured provider.

## Question

What do the original command inventory and caught Pi() results report before and after the exact namespace/procedure installation controls under each retained provider?

## Conclusion

All C releases begin with an empty Pi command inventory and a failed Pi() call. C8.5–9.1 return 17 after successful ::tcl::mathfunc::Pi installation; C8.4 retains its unknown math function result even though the procedure inventory contains that name. Jim accepts the namespace/procedure controls with its exact reported names but Pi() remains a syntax error.

A marked readonly source occurrence control preserves fixed-function presence/absence and retained snapshot correspondence without manufacturing command references.

## Scope

Six fresh provider processes retain six version rows and 36 sequential caught controls of the exact ASCII source. Initial lookup, namespace creation, installation and later expression evaluation share each provider process; successful namespace and install controls are measured rather than assumed. Binary-scan result hex supplies no native getter or original object identity. These observations bound expression-engine behavior and the selected command-slot purpose only; they prove no generic callback-free lookup, source snapshot completeness, compiler admission, cache/header/physical allocation or Native Normal. Jim names and expression syntax remain distinct from C namespace semantics. BIG-IP is not tested. The linked Rust selector retains its separate source-owner scope and has no execution result here.

No expression evaluation, operand preparation/topology, native function entry, private command slot or Guest failure is observed by this source definition. Jim Pi syntax error belongs to its independent original process purpose. Equal expression bytes, catalogue labels or an absence projection supply no Normal/SSA/frame/body authority; no assertion outcome is attached.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual CLI executable SHA f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a with retained SDK/header/library/build/source, environment and runner joins. No compilation command or compiler binary version is recorded by this source-file runner.. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection.. Dialect: Tcl.

The initial command inventory is empty and Pi() reports unknown math function "Pi". Namespace creation and procedure installation succeed, and the installed inventory contains ::tcl::mathfunc::Pi; the expression still reports the same unknown math function. This separates the fixed function engine from successful procedure publication.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual CLI executable SHA e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a with retained SDK/header/library/build/source, environment and runner joins. No compilation command or compiler binary version is recorded by this source-file runner.. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection.. Dialect: Tcl.

The initial command inventory is empty and Pi() reports invalid command name "tcl::mathfunc::Pi". Namespace creation and procedure installation succeed, the installed inventory contains ::tcl::mathfunc::Pi, and Pi() returns 17 afterwards.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual CLI executable SHA 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff with retained SDK/header/library/build/source, environment and runner joins. No compilation command or compiler binary version is recorded by this source-file runner.. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection.. Dialect: Tcl.

The initial command inventory is empty and Pi() reports invalid command name "tcl::mathfunc::Pi". Namespace creation and procedure installation succeed, the installed inventory contains ::tcl::mathfunc::Pi, and Pi() returns 17 afterwards.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual CLI executable SHA f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1 with retained SDK/header/library/build/source, environment and runner joins. No compilation command or compiler binary version is recorded by this source-file runner.. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection.. Dialect: Tcl.

The initial command inventory is empty and Pi() reports invalid command name "tcl::mathfunc::Pi". Namespace creation and procedure installation succeed, the installed inventory contains ::tcl::mathfunc::Pi, and Pi() returns 17 afterwards.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual CLI executable SHA 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d with retained SDK/header/library/build/source, environment and runner joins. No compilation command or compiler binary version is recorded by this source-file runner.. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection.. Dialect: Tcl.

The initial command inventory is empty and Pi() reports invalid command name "tcl::mathfunc::Pi". Namespace creation and procedure installation succeed, the installed inventory contains ::tcl::mathfunc::Pi, and Pi() returns 17 afterwards.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual CLI executable SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806 with retained SDK/header/library/build/source, environment and runner joins. No compilation command or compiler binary version is recorded by this source-file runner.. Channel: Fixed ASCII LF source-file CLI, one fresh provider process; six caught controls execute sequentially and share namespace/procedure state after the actual version row. Results use binary scan H* projection.. Dialect: Jim Tcl.

The initial inventory is empty and Pi() reports a syntax error. The namespace control succeeds; procedure installation returns ::tcl::mathfunc::Pi and the queried inventory reports ::::tcl::mathfunc::Pi. Pi() retains its syntax error. These exact names and outcomes establish no C namespace or expression-command dispatch behavior for Jim.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation of these exact implicit function and installation controls.

## Exact evidence

- `function-probe.tcl` (input): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/probe.tcl). SHA-256 `52606ff182cbccd8125c7be37bea2397fe93285047cc37a8cb6adfbe7c190332`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-request.json` (input): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/request.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/request.json). SHA-256 `63c7dacefaa57c4c0c0a40466e9911f27aa6ba18323daad2b85c1bc940c4f6b8`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-capture.py` (input): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/capture.py](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/capture.py). SHA-256 `379080da40335d5f4e57c0bd411249eb81253d6994b82533c6c1d52cc0f48222`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18/receipt.json). SHA-256 `a4cf483c584bcd337cf7daf81c0da622165f6ed5b93d65be15787d5c4f94d229`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.6.18/stdout). SHA-256 `b6c8d483819dd2fbb462e4ceb854013b13cda2e2e9f9e85042ea3b3f2401c5df`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0/receipt.json). SHA-256 `ccec52856c4bb085428e9b8dd49eb5fafe94b070e5b3374e2651448d91aab2cd`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.1.0/stdout). SHA-256 `2e59e14d21073e395d367fa15cd38ced5e91f39cf6d5ad0c54f00e7481cfafcd`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20/receipt.json). SHA-256 `f94123e6b84b52b669f77ab58151d522e8fb6725c5e6ea11f3283ba4822d80d8`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.4.20/stdout). SHA-256 `48804c7abb6a21d7d08dfd81534b11e2882a358e1101c3f82da6183aae12bdaa`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4/receipt.json). SHA-256 `550dd2d8e39bb888aea961b9538e03e34a30faeb04635d97123c4516a6e4c6ed`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/9.0.4/stdout). SHA-256 `3ea242e40aa00dbf84bb1223d99f311ddcc1e34279e8675876bcb346d2b04ce8`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19/receipt.json). SHA-256 `8237b32caa5816cd32e4e02588e288488ca9f0b74f7d21ec256b5c3199e62dfa`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/8.5.19/stdout). SHA-256 `6e6d022c5b87448dc2f5b97380938c2fd93ddef0b8a83c93a045c9f16de4eeac`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim/receipt.json). SHA-256 `a02bb2d77272f86765e074b3056de1d6349ff3fb210fad062af26eac41975cdf`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/jim/stdout). SHA-256 `0efb006e248b3ec836a4ea25c2c523fec2608b535103501f9e575afbcca34420`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.
- `function-sdk-queue.json` (provider): [rust/tcl-registry/tests/data/native_implicit_function_presence_original/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_implicit_function_presence_original/sdk-queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact original source/request/runner, provider association or raw process stream. The controls retain their sequential shared process state; hex is binary-scan result presentation, not an object getter or allocation measure.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `command_binding::original_math_function::tests::original_function_lookup_keeps_absence_and_unknown_separate` (linked): C8.5–9.1 original source function-name receipts retain absence without a fabricated written head; actual Unknown observations and operand/policy/observer barriers remain terminal. These are source-purpose assertions, not a replay of the native installation controls.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `SourceCommandBindings::original_math_functions_in_source`: Join retained actual source/config/Registry snapshot and selected original expression-word ancestry before projecting fixed/implicit function occurrence and diagnostic presence.
- [rust/tcl-compiler/src/command_binding/original_math_function.rs](../../../../rust/tcl-compiler/src/command_binding/original_math_function.rs), `command_binding::original_math_function::tests::original_fixed_function_absence_retains_source_snapshot_without_command_name` (linked): Actual selected C8.4 and Jim source fixtures retain fixed-table Pi absence with no command name/reference/Registry identity; changed image, withdrawn snapshot, dynamic unknown child/future command or shadowed expr withholds presence. abs remains a selected present fixed-function control. Source absence does not execute operands or report Jim Guest syntax failure.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_implicit_function_presence_original/capture.py"
]
```

The retained runner uses its original absolute SDK queue and creates provider output directories. Exact SDK/executable/environment associations and a fresh output directory are required. Installation and expression results share process state; source-purpose assertions and later Rust executions remain independent.
