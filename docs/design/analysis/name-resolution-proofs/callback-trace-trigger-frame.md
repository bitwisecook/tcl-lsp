# naming.callback.trace-trigger-frame

Kind: `native-observation`

## Problem statement

A trace prefix is installed in ::A while the operation that fires it runs in ::B, and matching prefix heads also exist at root. Resolving a prefix at installation can bind the wrong callback even when prefix bytes remain intact. Variable, command and execution trace purposes each need an actual triggering-frame observation.

## Question

Which namespace resolves unqualified prefix heads for the exact variable write, command rename and execution enter traces installed in ::A and triggered by ::B procedures?

## Conclusion

All five C providers resolve each of the three callback heads in ::B and report the triggering caller at level1 in ::B. The selected Jim guest rejects trace as unavailable for each case. These results establish the measured triggering lookup purposes; they do not establish arbitrary script equivalence, prefix storage, native argv objects or callback results.

## Scope

Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected native executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; launcher SHA-256 b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected native executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; launcher SHA-256 6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected native executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; launcher SHA-256 2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected native executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; launcher SHA-256 f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected native executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; launcher SHA-256 c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected native executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; launcher SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Jim Tcl.

trace 1 {invalid command name "trace"} {} | command-trace 1 {invalid command name "trace"} {} | execution-trace 1 {invalid command name "trace"} {}

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_callback_lookup_scope/scope.tcl](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/scope.tcl). SHA-256 `212514c650b20975e7a3fc1f414463bf6d24c5f24f423c604bac176efc20332f`. Exact complete ASCII source for all three lookup purposes; each question is restricted to its own outcome row.
- `capture` (provider): [rust/tcl-registry/tests/data/native_callback_lookup_scope/receipt.json](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/receipt.json). SHA-256 `acb035fed207cdca14aeb72fa4e065ec401ff3aadf5b08282cd1569e1406326c`. Actual six completed processes, version stdout, selected launcher and independently recorded native executable hashes, exact source SHA and limits.
- `output-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stdout). SHA-256 `8fda5a0c7a0ca3bb45ec66ca54f6a2def098e5e2abc37583c8b8c937ea484825`. Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stdout). SHA-256 `db8647084f06810f41619736bc185e8912007ee7ff6e5bcbea5a7be75a565491`. Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stdout). SHA-256 `a41598e2078d2a61cd3cb736bd8bc5011d66bdff64f2be8e26dab6276767ff11`. Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stdout). SHA-256 `5ca9999a3b64990cf6fd37a92667ba630646ded3d22a1bbe4502e28024e317a3`. Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stdout). SHA-256 `1f046914602167fc4e772ca34192fc6bb26cf5356d17c1ac3d3a7cd9d839f2d0`. Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr retained; guest errors are caught in stdout.
- `output-jim` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stdout). SHA-256 `7245221f4916a0d4e5889427a166fc53518bbd8fbc705e950a02a4c866e4e741`. Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-jim` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty stderr retained; guest errors are caught in stdout.

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
  "scripts/dev/replay-grammar-native-source.py",
  "--proof",
  "naming.callback.trace-trigger-frame",
  "--provider",
  "8.4",
  "--executable",
  "/absolute/path/to/native/tclsh8.4",
  "--output",
  "/absolute/path/outside/repository"
]
```

Repeat with 8.5, 8.6, 9.0, 9.1 and jim independently selected native CLI providers. Exact entire stdout, stderr and exit must match, including companion controls and the Jim caught unsupported trace diagnostics. No automatic provider fallback; global concurrency2 and per-process timeout60s are unchanged. Reconfirmation does not replay internal objects or unmeasured BIG-IP.
