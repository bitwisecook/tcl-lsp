# naming.namespace.source-import-preload-order

Kind: `native-observation`

## Problem statement

Treating namespace import as a table-only operation can skip a current global auto_import callback and publish commands despite an earlier callback error.

## Question

Does the exact installed global auto_import helper run before namespace import validates an absent source namespace?

## Conclusion

Every retained C Tcl release returns PRELOAD91 from the installed helper before reporting any missing-source namespace error. Jim returns empty success and does not enter the helper in this control.

## Scope

One exact ASCII source-file control on Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. The helper raises a fixed ASCII error and the import names an absent source namespace. Result code, provider string length/index and scan %c units are retained as ASCII. No arbitrary helper summary, callback absence, import completion, raw counted NUL, native table, compiler preparation or cache authority follows. BIG-IP was not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Executable SHA-256 b100108bc3747ea4fa6a4411a28756c26c8c201390a1983ae35443ce7bece817. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant.. Dialect: Tcl.

Code 1, length 9, decimal units 80 82 69 76 79 65 68 57 49 (PRELOAD91); the helper error precedes missing-source namespace validation.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Executable SHA-256 6b2005f227b608208c8bfdf472d5d6c1ed9595ad10f4b7442d5ee1e23e9e400b. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant.. Dialect: Tcl.

Code 1, length 9, decimal units 80 82 69 76 79 65 68 57 49 (PRELOAD91); the helper error precedes missing-source namespace validation.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Executable SHA-256 2e103024652a7d81f5496a979464fe0acad87a9033e798305be231ef484c5c5a. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant.. Dialect: Tcl.

Code 1, length 9, decimal units 80 82 69 76 79 65 68 57 49 (PRELOAD91); the helper error precedes missing-source namespace validation.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Executable SHA-256 f55225fd9e74448b244b90f41d3dce2989fc7e205bee4f2f0d4a08057880e668. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant.. Dialect: Tcl.

Code 1, length 9, decimal units 80 82 69 76 79 65 68 57 49 (PRELOAD91); the helper error precedes missing-source namespace validation.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Executable SHA-256 c5638d48d9a54d3a0d0901a4dc94698f8e018715473f8f6c2e3264408694ce47. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant.. Dialect: Tcl.

Code 1, length 9, decimal units 80 82 69 76 79 65 68 57 49 (PRELOAD91); the helper error precedes missing-source namespace validation.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806. Channel: ASCII source-file shell ingress; decimal string-index scan-%c result units; no counted API/physical cache grant.. Dialect: Jim Tcl.

Code 0, empty result; the installed helper is not called.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/cases.tcl](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/cases.tcl). SHA-256 `36ca69186f9ac05b396ffa557d3e64b66d347c989fda63ef2105a6bc5b39f136`. Exact ASCII helper/error control.
- `receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/receipt.json). SHA-256 `7173d266562d4796f3339b9fc6546785eed1590f518dc80567309b9e8b8a1d5e`. Actual six process/version/executable/source/output hash rows.
- `runner` (input): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/capture.py](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/capture.py). SHA-256 `360f7f77864328f22a7b35c6c0be045269908466818dae87ed444a054c655338`. Retained recorded capture protocol with exact workspace paths.
- `tcl8.4-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20/stdout). SHA-256 `f009714dbe41b9d301b7946b32384131c78e03797b227f760baecf60689e235f`. Exact code/length/decimal result units.
- `tcl8.4-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20/receipt.json). SHA-256 `d46cf826402da42bfc197f3f7e1611284dea5c2dc1cf50e314b2b72542327ad3`. Exact provider capture receipt.
- `tcl8.4-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr.
- `tcl8.5-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19/stdout). SHA-256 `f009714dbe41b9d301b7946b32384131c78e03797b227f760baecf60689e235f`. Exact code/length/decimal result units.
- `tcl8.5-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19/receipt.json). SHA-256 `219c83e9db2790a8c285a08fb25424ebc0e398d69306cf3539e40061b959da99`. Exact provider capture receipt.
- `tcl8.5-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr.
- `tcl8.6-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18/stdout). SHA-256 `f009714dbe41b9d301b7946b32384131c78e03797b227f760baecf60689e235f`. Exact code/length/decimal result units.
- `tcl8.6-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18/receipt.json). SHA-256 `a7faa21ff02273c65b1fe3b2ceb25a180fb527c969742563f08e6ab626872e2d`. Exact provider capture receipt.
- `tcl8.6-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr.
- `tcl9.0-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4/stdout). SHA-256 `f009714dbe41b9d301b7946b32384131c78e03797b227f760baecf60689e235f`. Exact code/length/decimal result units.
- `tcl9.0-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4/receipt.json). SHA-256 `93922cacba75b28b2d53472c4b9abf7ab57eab84576eaf979645e0ef53549f2f`. Exact provider capture receipt.
- `tcl9.0-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr.
- `tcl9.1-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0/stdout). SHA-256 `f009714dbe41b9d301b7946b32384131c78e03797b227f760baecf60689e235f`. Exact code/length/decimal result units.
- `tcl9.1-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0/receipt.json). SHA-256 `02886b69b5cb91a69c8e3bf0b5fe2614dded00e2fb11247e78a96218b2cc8a9b`. Exact provider capture receipt.
- `tcl9.1-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr.
- `jim-output` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim/stdout](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim/stdout). SHA-256 `5fce106c095529b59ce0dee9316a2661e405d7855cc87029478189929f4a4ba9`. Exact code/length/decimal result units.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim/receipt.json). SHA-256 `c21c8578541931c11a63c2e4f7f25b6bc76b9b31859ae312be2651f5e56cdbeb`. Exact provider capture receipt.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim/stderr](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/import-prelude/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Actual empty stderr.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The unchanged capture.py records exact retained executable and source paths. Permanent inputs and raw results retain full hashes; portable executable provisioning is not supplied.
