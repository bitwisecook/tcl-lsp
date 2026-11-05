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
