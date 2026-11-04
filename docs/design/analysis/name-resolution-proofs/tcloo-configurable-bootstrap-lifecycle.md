# naming.tcloo.configurable-bootstrap-lifecycle

Kind: `native-observation`

## Problem statement

A native constructor can be present even when constructor introspection cannot return a Tcl source body. Default manufacture needs the support class own lifecycle state independently of that introspection refusal.

## Question

Which own constructor and destructor definitions do source introspection queries return for the support and newly created classes, and how does the factory constructor query differ?

## Conclusion

C9.0.4 and C9.1.0 report a native factory constructor as unavailable for source introspection, while the support and created class own constructor/destructor queries return empty definitions. The native factory refusal does not prove constructor absence.

## Scope

Exact unchanged ASCII cases.tcl source-file ingress to C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim 0.84-9-g5bac7c9. A separate unchanged ASCII stdin query records startup patchlevels. C8.4/C8.5/C8.6/Jim record configurable availability 0 and skip later controls; this does not classify general TclOO availability. No BIG-IP invocation, counted raw-string NUL input, native header/cache identity, Rust execution or entered body frame is inferred.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; pinned header/library/Makefile/source hashes retained in tcl8.4-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; pinned header/library/Makefile/source hashes retained in tcl8.5-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; pinned header/library/Makefile/source hashes retained in tcl8.6-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; pinned header/library/Makefile/source hashes retained in tcl9.0-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

support.constructor, support.destructor, created.constructor, created.destructor: code 0 with empty definitions; factory.constructor: code 1, definition not available for this kind of method.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; pinned header/library/Makefile/source hashes retained in tcl9.1-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

support.constructor, support.destructor, created.constructor, created.destructor: code 0 with empty definitions; factory.constructor: code 1, definition not available for this kind of method.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; pinned header/library/Makefile/source hashes retained in jim-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Jim Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/cases.tcl](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/cases.tcl). Exact unchanged ASCII public source program.
- `aggregate` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/receipt.json). Six original aggregate provider rows with source, executable, header, library and startup hashes.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/stdout). Original status/value rows for factory.constructor, support.constructor, support.destructor, created.constructor, created.destructor, or the reached configurable availability refusal.
- `tcl8.4-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/startup.stdout). Independent actual startup patchlevel output.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/stdout). Original status/value rows for factory.constructor, support.constructor, support.destructor, created.constructor, created.destructor, or the reached configurable availability refusal.
- `tcl8.5-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/startup.stdout). Independent actual startup patchlevel output.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/stdout). Original status/value rows for factory.constructor, support.constructor, support.destructor, created.constructor, created.destructor, or the reached configurable availability refusal.
- `tcl8.6-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/startup.stdout). Independent actual startup patchlevel output.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/stdout). Original status/value rows for factory.constructor, support.constructor, support.destructor, created.constructor, created.destructor, or the reached configurable availability refusal.
- `tcl9.0-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/startup.stdout). Independent actual startup patchlevel output.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/stdout). Original status/value rows for factory.constructor, support.constructor, support.destructor, created.constructor, created.destructor, or the reached configurable availability refusal.
- `tcl9.1-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/startup.stdout). Independent actual startup patchlevel output.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/receipt.json). Original provider identity and exact source/stream/build hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/stdout). Original status/value rows for factory.constructor, support.constructor, support.destructor, created.constructor, created.destructor, or the reached configurable availability refusal.
- `jim-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/startup.stdout). Independent actual startup patchlevel output.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The retained capture.py and unchanged six provider receipts retain the original source-file/startup invocation paths. No portable executable replay or Rust test execution is asserted.
