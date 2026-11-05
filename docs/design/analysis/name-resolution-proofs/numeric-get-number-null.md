# naming.numeric.get-number-null

Kind: `native-observation`

## Problem statement

An original Number, Bignum or increment consumer can change primary, invalidate bytes or publish failure at a different stage. Borrowing one stage's result/cache rules for another would fabricate conversion and interpreter state.

## Question

What original header/storage and number-kind remain after GetNumber with NULL interpreter on sixteen input shapes?

## Conclusion

C8.5–9.1 distinguish fresh/cached integer, Double, Boolean-like and Bignum shapes under the original Number parser. Successful conversions can change primary while preserving resident bytes, and the NULL interpreter stage leaves SEED result. Raw-zero and modified-zero inputs differ;08 changes by release. Number-kind/header snapshots occur before the byte observer and do not publish a general expression operand proof.

## Scope

The exact probe constructs sixteen independent original String/Double/Wide/Boolean-primed/Bignum objects and selects one of three APIs. ResultSEED/errorCodeSENTINEL are seeded; before/after header fields are captured before the trailing byte observer. Four C8.5–9.1 full48-row JSONL hashes/build/status are retained. C8.4 and Jim branches print a compiled-out CNumberBignumUnavailable sentinel only; they execute none of these APIs and are not-tested, not guest rejections. BIG-IP is not tested; runtime patchlevels/configure/compiler versions are unqueried.

## Provider answers

### tcl8.4, jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

### tcl8.5

Status: `observed`. Channel: Direct original Number NULL-interpreter/GetBignum interpreter/private TclIncrObj API, according to selected stage. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
{"shape":0,"stage":0,"before_type":"none","before_resident":1,"before_length":1,"code":0,"number_type":1,"after_type":"int","after_resident":1,"after_length":1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"31"}
{"shape":1,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":4,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"312e35"}
{"shape":2,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":5,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"4e614e"}
{"shape":3,"stage":0,"before_type":"none","before_resident":1,"before_length":2,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":2,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652030","object_after_window":"3038"}
{"shape":4,"stage":0,"before_type":"none","before_resident":1,"before_length":24,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":24,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"313834343637343430373337303935353136313630303030"}
{"shape":5,"stage":0,"before_type":"none","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652030","object_after_window":"74727565"}
{"shape":6,"stage":0,"before_type":"none","before_resident":1,"before_length":0,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":0,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652030","object_after_window":""}
{"shape":7,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":1,"after_type":"int","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"203220"}
{"shape":8,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":1,"after_type":"int","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"310078"}
{"shape":9,"stage":0,"before_type":"none","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652030","object_after_window":"31c08078"}
{"shape":10,"stage":0,"before_type":"double","before_resident":0,"before_length":-1,"code":0,"number_type":4,"after_type":"double","after_resident":0,"after_length":-1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"312e35"}
{"shape":11,"stage":0,"before_type":"double","before_resident":1,"before_length":3,"code":0,"number_type":5,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"4e614e"}
{"shape":12,"stage":0,"before_type":"int","before_resident":0,"before_length":-1,"code":0,"number_type":1,"after_type":"int","after_resident":0,"after_length":-1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"37"}
{"shape":13,"stage":0,"before_type":"booleanString","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"booleanString","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652030","object_after_window":"74727565"}
{"shape":14,"stage":0,"before_type":"bignum","before_resident":1,"before_length":24,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":24,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"313834343637343430373337303935353136313630303030"}
{"shape":15,"stage":0,"before_type":"bignum","before_resident":1,"before_length":0,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":0,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":""}
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Channel: Direct original Number NULL-interpreter/GetBignum interpreter/private TclIncrObj API, according to selected stage. Version: tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
{"shape":0,"stage":0,"before_type":"none","before_resident":1,"before_length":1,"code":0,"number_type":1,"after_type":"int","after_resident":1,"after_length":1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"31"}
{"shape":1,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":4,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"312e35"}
{"shape":2,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":5,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"4e614e"}
{"shape":3,"stage":0,"before_type":"none","before_resident":1,"before_length":2,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":2,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"3038"}
{"shape":4,"stage":0,"before_type":"none","before_resident":1,"before_length":24,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":24,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"313834343637343430373337303935353136313630303030"}
{"shape":5,"stage":0,"before_type":"none","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"74727565"}
{"shape":6,"stage":0,"before_type":"none","before_resident":1,"before_length":0,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":0,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":""}
{"shape":7,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":1,"after_type":"int","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"203220"}
{"shape":8,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":1,"after_type":"int","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"310078"}
{"shape":9,"stage":0,"before_type":"none","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"31c08078"}
{"shape":10,"stage":0,"before_type":"double","before_resident":0,"before_length":-1,"code":0,"number_type":4,"after_type":"double","after_resident":0,"after_length":-1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"312e35"}
{"shape":11,"stage":0,"before_type":"double","before_resident":1,"before_length":3,"code":0,"number_type":5,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"4e614e"}
{"shape":12,"stage":0,"before_type":"int","before_resident":0,"before_length":-1,"code":0,"number_type":1,"after_type":"int","after_resident":0,"after_length":-1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"37"}
{"shape":13,"stage":0,"before_type":"booleanString","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"booleanString","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"74727565"}
{"shape":14,"stage":0,"before_type":"bignum","before_resident":1,"before_length":24,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":24,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"313834343637343430373337303935353136313630303030"}
{"shape":15,"stage":0,"before_type":"bignum","before_resident":1,"before_length":0,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":0,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":""}
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Direct original Number NULL-interpreter/GetBignum interpreter/private TclIncrObj API, according to selected stage. Version: tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
{"shape":0,"stage":0,"before_type":"none","before_resident":1,"before_length":1,"code":0,"number_type":2,"after_type":"int","after_resident":1,"after_length":1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"31"}
{"shape":1,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":4,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"312e35"}
{"shape":2,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":5,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"4e614e"}
{"shape":3,"stage":0,"before_type":"none","before_resident":1,"before_length":2,"code":0,"number_type":2,"after_type":"int","after_resident":1,"after_length":2,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"3038"}
{"shape":4,"stage":0,"before_type":"none","before_resident":1,"before_length":24,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":24,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"313834343637343430373337303935353136313630303030"}
{"shape":5,"stage":0,"before_type":"none","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"74727565"}
{"shape":6,"stage":0,"before_type":"none","before_resident":1,"before_length":0,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":0,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":""}
{"shape":7,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":2,"after_type":"int","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"203220"}
{"shape":8,"stage":0,"before_type":"none","before_resident":1,"before_length":3,"code":0,"number_type":2,"after_type":"int","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"310078"}
{"shape":9,"stage":0,"before_type":"none","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"none","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"31c08078"}
{"shape":10,"stage":0,"before_type":"double","before_resident":0,"before_length":-1,"code":0,"number_type":4,"after_type":"double","after_resident":0,"after_length":-1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"312e35"}
{"shape":11,"stage":0,"before_type":"double","before_resident":1,"before_length":3,"code":0,"number_type":5,"after_type":"double","after_resident":1,"after_length":3,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"4e614e"}
{"shape":12,"stage":0,"before_type":"int","before_resident":0,"before_length":-1,"code":0,"number_type":2,"after_type":"int","after_resident":0,"after_length":-1,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"37"}
{"shape":13,"stage":0,"before_type":"boolean","before_resident":1,"before_length":4,"code":1,"number_type":-1,"after_type":"boolean","after_resident":1,"after_length":4,"result":"53454544","options":"2d636f64652031202d6c6576656c2030202d6572726f72737461636b207b7d202d6572726f72636f64652053454e54494e454c202d6572726f72696e666f2053454544202d6572726f726c696e652031","object_after_window":"74727565"}
{"shape":14,"stage":0,"before_type":"bignum","before_resident":1,"before_length":24,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":24,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":"313834343637343430373337303935353136313630303030"}
{"shape":15,"stage":0,"before_type":"bignum","before_resident":1,"before_length":0,"code":0,"number_type":3,"after_type":"bignum","after_resident":1,"after_length":0,"result":"53454544","options":"2d636f64652030202d6c6576656c2030202d6572726f72636f64652053454e54494e454c","object_after_window":""}
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_number_probe`: Separates original Number/Bignum numeric preparation from interpreter error publication and backend availability.
- [runtime/rust/src/native_number_tests.rs](../../../../runtime/rust/src/native_number_tests.rs), `native_number_tests::tests::original_number_bignum_and_increment_match_all_192_native_windows` (linked): Compares the independently selected original stage under each retained C8.5–9.1 header/result window; backend withdrawal is additional Rust obligation.
- [rust/tcl-vm/src/native_number_tests.rs](../../../../rust/tcl-vm/src/native_number_tests.rs), `native_number_tests::original_number_bignum_and_increment_match_all_192_native_windows` (linked): Compares the independently selected original stage under each retained C8.5–9.1 header/result window; backend withdrawal is additional Rust obligation.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/number/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/8.5.19.jsonl](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/8.5.19.jsonl)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/8.6.18.jsonl](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/8.6.18.jsonl)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/9.0.4.jsonl](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/9.0.4.jsonl)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/9.1.0.jsonl](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/9.1.0.jsonl)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/8.4.20.jsonl](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/8.4.20.jsonl)
- [rust/tcl-syntax/tests/data/native_scalar_getters/number/Jim.jsonl](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/number/Jim.jsonl)
