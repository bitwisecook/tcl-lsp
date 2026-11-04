# naming.tcloo.configurable-support-allocation-lifetime

Kind: `native-observation`

## Problem statement

An already created class retains the support allocation, whereas a later factory call resolves the original support name anew. A final spelling map cannot combine those two currencies.

## Question

After the support command is renamed, does existing class manufacture continue while a fresh configurable factory call fails its original support lookup?

## Conclusion

C9.0.4 and C9.1.0 successfully manufacture through existing ::C after the support rename. The following configurable create fails because ::oo::configuresupport::configurable no longer names an object. The generated result name is capture-specific.

## Scope

Exact unchanged ASCII cases.tcl source-file ingress to C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim 0.84-9-g5bac7c9. A separate unchanged ASCII stdin query records startup patchlevels. C8.4/C8.5/C8.6/Jim record configurable availability 0 and skip later controls; this does not classify general TclOO availability. No BIG-IP invocation, counted raw-string NUL input, native header/cache identity, Rust execution or entered body frame is inferred.

## Provider answers

### tcl8.4

Status: `unsupported`. Version: 8.4.20. Build: Executable; pinned header/library/Makefile/source hashes retained in tcl8.4-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl8.5

Status: `unsupported`. Version: 8.5.19. Build: Executable; pinned header/library/Makefile/source hashes retained in tcl8.5-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl8.6

Status: `unsupported`. Version: 8.6.18. Build: Executable; pinned header/library/Makefile/source hashes retained in tcl8.6-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable; pinned header/library/Makefile/source hashes retained in tcl9.0-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

moved.support: code 0, ::oo::Obj24; future.factory.missing.support: code 1, ::oo::configuresupport::configurable does not refer to an object.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable; pinned header/library/Makefile/source hashes retained in tcl9.1-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Tcl.

moved.support: code 0, ::oo::Obj24; future.factory.missing.support: code 1, ::oo::configuresupport::configurable does not refer to an object.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Executable; pinned header/library/Makefile/source hashes retained in jim-receipt.. Channel: Exact ASCII source file argument to the actual native shell; Tcl file-character ingress/Jim file evaluator. Separate ASCII stdin startup version query.. Dialect: Jim Tcl.

The reached available control reports code 0 and value 0 for ::oo::configurable; later controls for this question are skipped.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/cases.tcl](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/cases.tcl). Exact unchanged ASCII public source program.
- `aggregate` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/receipt.json). Six original aggregate provider rows with source, executable, header, library and startup hashes.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl8.4-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/stdout). Original status/value rows for moved.support, future.factory.missing.support, or the reached configurable availability refusal.
- `tcl8.4-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.4/startup.stdout). Independent actual startup patchlevel output.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl8.5-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/stdout). Original status/value rows for moved.support, future.factory.missing.support, or the reached configurable availability refusal.
- `tcl8.5-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.5/startup.stdout). Independent actual startup patchlevel output.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl8.6-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/stdout). Original status/value rows for moved.support, future.factory.missing.support, or the reached configurable availability refusal.
- `tcl8.6-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl8.6/startup.stdout). Independent actual startup patchlevel output.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl9.0-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/stdout). Original status/value rows for moved.support, future.factory.missing.support, or the reached configurable availability refusal.
- `tcl9.0-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.0/startup.stdout). Independent actual startup patchlevel output.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/receipt.json). Original provider identity and exact source/stream/build hashes.
- `tcl9.1-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/stdout). Original status/value rows for moved.support, future.factory.missing.support, or the reached configurable availability refusal.
- `tcl9.1-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/tcl9.1/startup.stdout). Independent actual startup patchlevel output.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/receipt.json). Original provider identity and exact source/stream/build hashes.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/stdout). Original status/value rows for moved.support, future.factory.missing.support, or the reached configurable availability refusal.
- `jim-startup` (provider): [rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/startup.stdout](../../../../rust/tcl-registry/tests/data/native_tcloo_configurable_bootstrap/jim/startup.stdout). Independent actual startup patchlevel output.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The retained capture.py and unchanged six provider receipts retain the original source-file/startup invocation paths. No portable executable replay or Rust test execution is asserted.
