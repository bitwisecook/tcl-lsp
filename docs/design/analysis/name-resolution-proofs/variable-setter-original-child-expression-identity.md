# naming.variable.setter-original-child-expression-identity

Kind: `native-observation`

## Problem statement

A nested source command can be the stock expression handler, its interpreter alias or a same-spelled procedure returning literal data. The written expression label alone cannot distinguish their variable-read behavior.

## Question

What do the four exact direct, aliased and shadowed setter/expression source forms return under the six retained CLI providers?

## Conclusion

All five C providers return4 for direct and aliased expressions and VALUE for the same-spelled procedure through direct and aliased setters. Current Jim returns4 and VALUE for the direct forms, while both interpreter-alias attempts return the actual interp wrong-arity error. These results distinguish selected source child behavior without establishing compiler or private storage facts.

## Scope

Six original provider processes, each with an actual version row and four caught observations (24 observations total). Fixed ASCII LF source-file CLI input; observations run sequentially in the same fresh process, so alias_expression reuses input and alias_setter intentionally reuses the earlier expression shadow. The original authored source and executed version-prefixed source have separate retained SHA joins. Raw receipts call the catches independent; that sentence does not imply independent interpreter state. No opaque/counting-object input, compiler admission, physical read/frame/cell/cache/header, callback closure, Native Normal transfer or BIG-IP answer is established.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Retained exact CLI executable SHA f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a with actual SDK/header/library/build/source joins and environment. This source-file runner performs no compilation and records no compiler binary version.. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained.. Dialect: Tcl.

Direct and aliased expression return4. The same-spelled procedure returnsVALUE through direct and aliased setters; its braced missing name is not evaluated by the stock expression handler.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Retained exact CLI executable SHA e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a with actual SDK/header/library/build/source joins and environment. This source-file runner performs no compilation and records no compiler binary version.. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained.. Dialect: Tcl.

Direct and aliased expression return4. The same-spelled procedure returnsVALUE through direct and aliased setters; its braced missing name is not evaluated by the stock expression handler.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Retained exact CLI executable SHA 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff with actual SDK/header/library/build/source joins and environment. This source-file runner performs no compilation and records no compiler binary version.. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained.. Dialect: Tcl.

Direct and aliased expression return4. The same-spelled procedure returnsVALUE through direct and aliased setters; its braced missing name is not evaluated by the stock expression handler.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Retained exact CLI executable SHA f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1 with actual SDK/header/library/build/source joins and environment. This source-file runner performs no compilation and records no compiler binary version.. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained.. Dialect: Tcl.

Direct and aliased expression return4. The same-spelled procedure returnsVALUE through direct and aliased setters; its braced missing name is not evaluated by the stock expression handler.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Retained exact CLI executable SHA 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d with actual SDK/header/library/build/source joins and environment. This source-file runner performs no compilation and records no compiler binary version.. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained.. Dialect: Tcl.

Direct and aliased expression return4. The same-spelled procedure returnsVALUE through direct and aliased setters; its braced missing name is not evaluated by the stock expression handler.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Retained exact CLI executable SHA e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806 with actual SDK/header/library/build/source joins and environment. This source-file runner performs no compilation and records no compiler binary version.. Channel: Exact ASCII LF source-file CLI; one fresh provider process runs four caught observations sequentially after its actual version row. Original source and the separately added version-prefix source are both retained.. Dialect: Jim Tcl.

Direct builtin expression returns4; same-spelled procedure returnsVALUE. Both interpreter-alias attempts fail with the recorded interp arity usage; no successful alias read or setter is observed.

### bigip

Status: `not-tested`. Version: not tested. Build: not recorded. Channel: not tested. Dialect: F5 iRules.

No appliance measurement of these source forms.

## Exact evidence

- `setter-probe.tcl` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/probe.tcl). SHA-256 `36b6678065adfe744cc46ecaffcce83a75a89ccd433d0063d68fa9b5c64038f2`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-request.json` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/request.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/request.json). SHA-256 `f0cd1f22ba2012bae564760b9a735a4912b4ce6a2239de6d710130546d8ea584`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-capture.py` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/capture.py](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/capture.py). SHA-256 `98b2d6cd7e407d2b13839177dbf7a0c933e4746e0632cbf87c978f9555d28a80`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/receipt.json). SHA-256 `fe9472c603424b29925abea03be7264e4976f846bc4d984d0d6427631b19da83`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stdout). SHA-256 `c87bf75a9e505ba36ac711f269c45873aaaf1d8210c4cfc80a4dc137974640ee`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/receipt.json). SHA-256 `0e7d0c74ba8c2c38647ca01ef50ee69ba65e2523b27b105c0680c0f03dc758ba`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stdout). SHA-256 `91f5d8d41e81f9a1360ace1c3649ee9e68040909988609aed2eca4fe2250824b`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/receipt.json). SHA-256 `fe9bdf35ec4e6033866cddc1959b4b0cdc907add3fd5545b2e437516152ff70b`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stdout). SHA-256 `814f0717770423579bd9cd28607256f92ec911743df7a2d1bf771abbbecf0aee`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/receipt.json). SHA-256 `007a9e1cc4d36f2dd80e2efb334c0c9f5eab7e91432cb01f44139dbf865f70ce`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stdout). SHA-256 `b123c3c8d53a67a366732bebb6ac0d504c814d65da35a0f757e839f095a22b3c`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/receipt.json). SHA-256 `95435020fbb158aa16e3d4b081e53a8a34c95e34cf9de49639aa80750932d43a`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stdout). SHA-256 `a6eeac51d084751ea5f9e5d58af2cf3f0becb73b132fd17af4fc0a731e6cfbf5`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim/receipt.json). SHA-256 `a4e764932e5b1f65af7edffffaa2c759d307ae2448f0d8098868bc093506710f`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim/stdout). SHA-256 `81010903b97bfa979932b8df37529b29fe1edd51371fd1d60e05fa1e271b7fbc`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-original-probe.tcl` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/original-probe.tcl](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/original-probe.tcl). SHA-256 `2cbe4c863a59554f0538d96ed7cacaea6ac534c714b053c06e4d8f712e363036`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-sdk-queue.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/sdk-queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.

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
  "rust/tcl-registry/tests/data/native_setter_expression_original/capture.py"
]
```

The retained runner reads its original absolute SDK queue and creates fresh provider output directories. Replay requires those exact independently retained SDK/executable/environment inputs and a fresh output path. Original captured streams and request fields remain immutable. No Rust implementation assertion or successful Jim alias behavior is inferred.
