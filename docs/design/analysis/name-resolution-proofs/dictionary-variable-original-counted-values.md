# Scalar dictionary-variable original values

Proof ID: `naming.dictionary-variable.original-counted-values`

## Problem statement

Jim dictionary element syntax can read a scalar root that C Tcl requires to be an actual array. Logical String list parsing can also lose an opaque or counted-zero key, or select an earlier duplicate. A mere membership result cannot manufacture a value or prove a current normal read. The check uses original resident counted root/key objects without a Document decoder.

## Question

Does set a($index) select scalar dictionary values for duplicate, raw-zero, modified-zero, surrogate and colon keys, and how do malformed/missing/mutated roots compare with C Tcl?

## Scope

Nine original counted root/key object cases per C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and current Jim0.84-9-g5bac7c9. Public original String script evaluation of set a($index); no source file character channel, native frame table, arbitrary custom object, alias or BIG-IP behavior is established.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Compile0/process0; ten rows. All nine scalar-root reads produce guest code1, variable is not an array. |
| tcl8.5 8.5.19 | observed | Compile0/process0; ten rows. All nine scalar-root reads produce guest code1, variable is not an array. |
| tcl8.6 8.6.18 | observed | Compile0/process0; ten rows. All nine scalar-root reads produce guest code1, variable is not an array. |
| tcl9.0 9.0.4 | observed | Compile0/process0; ten rows. All nine scalar-root reads produce guest code1, variable is not an array. |
| tcl9.1 9.1.0 | observed | Compile0/process0; ten rows. All nine scalar-root reads produce guest code1, variable is not an array. |
| jim 0.84-9-g5bac7c9 | observed | Compile0/process0; ten rows. Simple FIRST, duplicate LAST, raw-zero RAW, modified-zero MOD, surrogate SUR, colon COLON, mutated LAST; odd/missing genuine guest code1. |
| bigip not recorded | not-tested | No appliance observation for this question. |

The original raw-zero key is `6B 00 74`, modified-zero is `6B C0 80 74`, and the lone-surrogate key is `6B ED A0 80`. These are resident String bytes constructed with explicit counts, not numeric escapes or configuration text. The duplicate root is `k FIRST k LAST`; the mutation replaces `k FIRST` with `k LAST`.

## Conclusion

Current Jim returns the retained values for the six admitted key/root forms, chooses the final duplicate value, rejects an odd root or missing key, and reads the replacement root after mutation. Every tested C release rejects the scalar root as not an array. The pure ordinal recipe therefore applies only to independently selected Jim dictionary-variable input. Current cell, parent value lineage, read effects, successful completion and conversion authority remain independent.

## Evidence

- `rust/tcl-registry/tests/data/native_dictionary_variable_values/probe.c`, SHA-256 `546d7b5b88b63999848a828c8650f8800d8487ed96a7942631028f87b2df5941`. Exact root/key constructor lengths and native script source.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/receipt.json`, SHA-256 `2177b6f9cabe098fbba1b6360a422faeb260e25be094f5cbf34bf30daed2c33a`. All six actual compile/process/library/header/executable and output identities.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/observations.tsv`, SHA-256 `365b0b6254d12e24bcf6d4a6a2677cbd1fba4daf208b4e96db2796aa41631a5d`. Derived exact guest code/result-byte rows for implementation comparison.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/replay.py`, SHA-256 `1ab24fca8c519c705ae8e6b73a6006033169f32159bdad30f082a560dcd4fd74`. Sequential strict six-provider startup and complete-row comparison.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/8.4.20/stdout.jsonl`, SHA-256 `65c04d37a6f30da277b4e0680f88ad4334fbdcf343b5d2fba8889ec64aa80425`. Original ten stdout rows, including launched patchlevel and all guest errors.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/8.4.20/receipt.json`, SHA-256 `46756f6698e9bbdda9632995eb440cd107bbb70f8853a2c84e7f5009aa4d1d86`. Actual original compile/launch and provider identity.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/8.5.19/stdout.jsonl`, SHA-256 `fcf4cd2fe71f001799e6fdbfee7c74b0451a6f1675cf0fbe4c11f60628f9399a`. Original ten stdout rows, including launched patchlevel and all guest errors.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/8.5.19/receipt.json`, SHA-256 `21e5e63b2e3ea84738b11f6ac10980e85ab4ff623f9ce12ebe86e3fe4038bbf0`. Actual original compile/launch and provider identity.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/8.6.18/stdout.jsonl`, SHA-256 `c061984f2be275967748cacf5447d76950905d784b9adb4f35208909221cf642`. Original ten stdout rows, including launched patchlevel and all guest errors.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/8.6.18/receipt.json`, SHA-256 `f08d46064810e1f89535717351ec3e7b7c0ac1a1d6d10944bd881542bd19893d`. Actual original compile/launch and provider identity.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/9.0.4/stdout.jsonl`, SHA-256 `1329bf645e8e3d93f30a873c38e8698420f8b1c1484d79cb9171c040404cf2e2`. Original ten stdout rows, including launched patchlevel and all guest errors.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/9.0.4/receipt.json`, SHA-256 `ffdaedf621da29ed49ea9f0af088fb624e87e9c845746a5c8a0e2d18859e1a0b`. Actual original compile/launch and provider identity.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/9.1.0/stdout.jsonl`, SHA-256 `91a444ab7f25889b2f3edcd1d78ee117618b9a65ba0c1fb1b14e2e5e150e536f`. Original ten stdout rows, including launched patchlevel and all guest errors.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/9.1.0/receipt.json`, SHA-256 `c7b87553fb539c97d497460830e12bc0e390b274feea6f58823b82a435c607d6`. Actual original compile/launch and provider identity.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/jim/stdout.jsonl`, SHA-256 `b6ca57edfcb527cb4da52f26e945ac90ff1a94be587c6ef2be5b95b18cf326fd`. Original ten stdout rows, including launched patchlevel and all guest errors.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/jim/receipt.json`, SHA-256 `ba4e5b8f87f7a34b606710d914d11857a4ad4dabb9ce24158f4598d9efc993fc`. Actual original compile/launch and provider identity.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/source-anchors/Jim_StringEqObj.c`, SHA-256 `56c324056af6b1ef2d6c7b9ceb85ff014a9647dab122c6bbe77fc8d01c6fcc8a`. Pinned current Jim source explains counted equality and last duplicate replacement; independent of native output.
- `rust/tcl-registry/tests/data/native_dictionary_variable_values/source-anchors/SetDictFromAny.c`, SHA-256 `b0311655aa2f0952d72fc7bdfd92f9efcf321452d312f7063987ec597b826e60`. Pinned current Jim source explains counted equality and last duplicate replacement; independent of native output.

## Source anchors

- Jim `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c::Jim_StringEqObj`, lines[2627, 2640]; source SHA `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`, exact excerpt SHA `56c324056af6b1ef2d6c7b9ceb85ff014a9647dab122c6bbe77fc8d01c6fcc8a`. Source inspection explains the operation and remains separate from its measured rows.
- Jim `5bac7c99ad65864c87da513e22e2f01703fa4e03`, `jim.c::SetDictFromAny`, lines[7882, 7948]; source SHA `fe5e3bea157b898e91e6c8e50c8f9eb0e89a6aba8c6a26b5d4b7b2b03cba3867`, exact excerpt SHA `b0311655aa2f0952d72fc7bdfd92f9efcf321452d312f7063987ec597b826e60`. Source inspection explains the operation and remains separate from its measured rows.

## Shared owners and tests

- `rust/tcl-registry/src/native_dictionary.rs`: `InvocationDialect::dictionary_variable_value_ordinal`. Pure selected native list-value ordinal, no current read/value grant.
- `rust/tcl-registry/src/native_dictionary.rs`: `InvocationDialect::dictionary_variable_root_valid`. Independent even-cardinality validity for selected Jim root.
- `rust/tcl-registry/src/native_dictionary.rs`: `native_dictionary::compiler_tests::dictionary_variable_ordinal_matches_current_native_key_values`. Selected Jim ordinal derives exact native successful values and rejects odd/missing roots; C releases receive no scalar dictionary purpose. Distinct prefix/native zero/surrogate negatives independently check the counted comparison.

## Reconfirmation

```text
python3 rust/tcl-registry/tests/data/native_dictionary_variable_values/replay.py --tcl-source-root tmp --jim-source-root /workspace/.proofs/native-providers/jimtcl --output /tmp/dictionary-value-reconfirmation
```

Exact built C5 releases/current UTF8=1 Jim revision5bac7c99ad65864c87da513e22e2f01703fa4e03 required. The original probe includes the counted byte arrays and source; no runtime source encoding occurs. All ten rows including startup and guest failures compare as data. Nonzero harness exit/stderr/compile failure are separately retained failures. Native success does not establish a Rust cell or parent-producer lifetime.
