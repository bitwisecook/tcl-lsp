# naming.variable.original-array-observer-renamed-alias-controls

Kind: `native-observation`

## Problem statement

Does renaming or aliasing the original array handler preserve its whole-array read, requiring point-aware compiler observers rather than Registry role lookup by head spelling?

## Question

Do the exact builtin, renamed original array handler and supported interpreter alias return the same k OLD whole-array value, and what is the caught alias-setup result under each retained provider?

## Conclusion

All six providers return k OLD through the builtin and renamed original array handler. All five C releases also set up array_alias successfully and return k OLD through it. Jim rejects interpreter alias setup, so its source has no alias-read result.

## Scope

Six fresh CLI processes retain six actual version rows and 23 observable control fields: twelve direct read results, six caught setup records and five caught alias-read records. These are not 23 caught commands. Original ASCII source and its version-prefixed executed source retain separate exact hashes; all controls share each process in source order. Output is Tcl list presentation of values/status, not native getter bytes or object identity. No trace callback, compiler point-read inventory, dead-store proof, physical cell/header/cache, source-model completion, compiler admission or Native Normal is established. Jim alias setup refusal is separate from its successful builtin/rename reads; BIG-IP is not tested. The linked compiler selector retains an independent implementation purpose and supplies no executed Rust result here.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Actual CLI executable SHA f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a with retained SDK/header/library/build/source, environment and runner joins. This source-file runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status.. Dialect: Tcl.

Builtin and renamed original array get each return k OLD. Interpreter alias setup succeeds with result array_alias, and the caught alias read succeeds with k OLD.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Actual CLI executable SHA e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a with retained SDK/header/library/build/source, environment and runner joins. This source-file runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status.. Dialect: Tcl.

Builtin and renamed original array get each return k OLD. Interpreter alias setup succeeds with result array_alias, and the caught alias read succeeds with k OLD.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Actual CLI executable SHA 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff with retained SDK/header/library/build/source, environment and runner joins. This source-file runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status.. Dialect: Tcl.

Builtin and renamed original array get each return k OLD. Interpreter alias setup succeeds with result array_alias, and the caught alias read succeeds with k OLD.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Actual CLI executable SHA f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1 with retained SDK/header/library/build/source, environment and runner joins. This source-file runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status.. Dialect: Tcl.

Builtin and renamed original array get each return k OLD. Interpreter alias setup succeeds with result array_alias, and the caught alias read succeeds with k OLD.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Actual CLI executable SHA 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d with retained SDK/header/library/build/source, environment and runner joins. This source-file runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status.. Dialect: Tcl.

Builtin and renamed original array get each return k OLD. Interpreter alias setup succeeds with result array_alias, and the caught alias read succeeds with k OLD.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual CLI executable SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806 with retained SDK/header/library/build/source, environment and runner joins. This source-file runner records no compilation command or compiler binary version.. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status.. Dialect: Jim Tcl.

Builtin and renamed original array get each return k OLD. Interpreter alias setup fails with the actual interp arity message; the source skips alias evaluation, so no successful alias read is observed.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance observation of these exact original array rename/alias controls.

## Exact evidence

- `array-original-question.tcl` (input): [rust/tcl-registry/tests/data/native_original_array_observer/original-question.tcl](../../../../rust/tcl-registry/tests/data/native_original_array_observer/original-question.tcl). SHA-256 `0d2c19184b7e5ac5a88eff114dff52279a15527afb71e8b0d0097c52b1a60b2d`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-probe.tcl` (input): [rust/tcl-registry/tests/data/native_original_array_observer/probe.tcl](../../../../rust/tcl-registry/tests/data/native_original_array_observer/probe.tcl). SHA-256 `798ba687127cd5eb8e2e462e4befbeb30265b81c2e92b07311f8077f1d880b11`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-request.json` (input): [rust/tcl-registry/tests/data/native_original_array_observer/request.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/request.json). SHA-256 `0848dc2b919431932dd1d6bf652ee3710935cdaf77757cb4d353d3ff8d64cb41`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-capture.py` (input): [rust/tcl-registry/tests/data/native_original_array_observer/capture.py](../../../../rust/tcl-registry/tests/data/native_original_array_observer/capture.py). SHA-256 `75af813dd5322d4a3e2abc9789d4478da643eb1309620d5607cd4f15bc1b41eb`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.6.18/receipt.json). SHA-256 `04f2f170d2c98fba436649b189df59ea742e06720511dd067a77a5951655a5f2`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.6.18/stdout). SHA-256 `1ba7b9a28bf395945150f7722fb30d3b45001fa3ab69634db0ffa746ffdede2d`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.1.0/receipt.json). SHA-256 `56da78a14c1bf9993697041c3e39aab32ac99e240be8e271cf462a267f729c46`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.1.0/stdout). SHA-256 `90ca8d25a54d58a1e645f4d6188a1e06cf1d53420c1a6d610b17a29584224a72`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.4.20/receipt.json). SHA-256 `e347ed8f3eb69c5d8d5b689394317af1df366a44c5376d2da16ea6891d804e9c`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.4.20/stdout). SHA-256 `508e763cee2ca0ca0983777ca5bab57e1bd6d5a7dc9bc6cf3104e39ea03000d7`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.0.4/receipt.json). SHA-256 `a93db398ee51600e77e93e03eb10a091cc8ac60384d1887e2beb30400889c0a7`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.0.4/stdout). SHA-256 `104f88a62e568995e0b7d22e1d6ea266c8a3b1829562f534f15d4272924656ff`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.5.19/receipt.json). SHA-256 `8355582b5b5275fd3b94b585913b9448f0e23913bccf6cfdf6bd549a4ea71808`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.5.19/stdout). SHA-256 `5b2eb4b18759a445f50bd006fa1c7f5d255826edd6a21942963b352ddf4be0ed`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/jim/receipt.json). SHA-256 `f417d046b50fc5f3dd4712ac3271934214e460aa31d026ca3257fe8c12ad185a`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/jim/stderr](../../../../rust/tcl-registry/tests/data/native_original_array_observer/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_original_array_observer/jim/stdout](../../../../rust/tcl-registry/tests/data/native_original_array_observer/jim/stdout). SHA-256 `e39048acf5c06d5cf754ce50f64cc4f3c75f3cf3f3b6bc982eb96b2abaedc9a6`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.
- `array-sdk-queue.json` (provider): [rust/tcl-registry/tests/data/native_original_array_observer/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_original_array_observer/sdk-queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact original or independently version-prefixed source, request/runner, actual provider association or raw process stream. Sequential values and supported alias setup are measured; no compiler read inventory or physical allocation is inferred.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/place_bridge.rs](../../../../rust/tcl-compiler/src/place_bridge.rs), `place_bridge::tests::element_observers_follow_original_renamed_and_aliased_array_reads` (linked): Fifteen source controls in five C authoring contexts independently require authentic Native registration ownership, actual selected original array child target, concrete whole-array read inventory and a real element store before point-aware observation. This is a compiler-source assertion, not native transcript replay.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "rust/tcl-registry/tests/data/native_original_array_observer/capture.py"
]
```

The retained runner uses its original absolute SDK queue and creates provider output directories. Exact SDK/executable/environment associations and a fresh output directory are required. The builtin and renamed reads are direct source actions; only alias setup and supported alias evaluation are caught. Compiler source assertions and later Rust execution remain independent.
