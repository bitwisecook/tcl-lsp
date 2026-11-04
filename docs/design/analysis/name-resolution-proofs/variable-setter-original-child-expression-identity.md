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

## Historical references

- `setter-probe.tcl` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/probe.tcl). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-request.json` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/request.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/request.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-capture.py` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/capture.py](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/capture.py). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.6.18-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/receipt.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.6.18-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stderr). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.6.18-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.6.18/stdout). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.1.0-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/receipt.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.1.0-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stderr). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.1.0-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.1.0/stdout). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.4.20-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/receipt.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.4.20-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stderr). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.4.20-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.4.20/stdout). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.0.4-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/receipt.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.0.4-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stderr). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-9.0.4-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/9.0.4/stdout). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.5.19-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/receipt.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.5.19-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stderr). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-8.5.19-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/8.5.19/stdout). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-jim-receipt.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim/receipt.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-jim-stderr` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim/stderr). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-jim-stdout` (observation): [rust/tcl-registry/tests/data/native_setter_expression_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/jim/stdout). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-original-probe.tcl` (input): [rust/tcl-registry/tests/data/native_setter_expression_original/original-probe.tcl](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/original-probe.tcl). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.
- `setter-sdk-queue.json` (provider): [rust/tcl-registry/tests/data/native_setter_expression_original/sdk-queue.json](../../../../rust/tcl-registry/tests/data/native_setter_expression_original/sdk-queue.json). Exact original source, request, runner, actual provider association or raw process stream. Shared sequential source state and caught alias refusal remain explicit; source CLI supplies no private object or compiler evidence.

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
