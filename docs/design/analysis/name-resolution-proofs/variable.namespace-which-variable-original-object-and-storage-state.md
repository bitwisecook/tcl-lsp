# naming.variable.namespace-which-variable-original-object-and-storage-state

Kind: `native-observation`

## Problem statement

A namespace variable query can receive a resident string or a pure byte array with raw 00, C080 or opaque units; original and short names may select different keys in absent/defined/declaration/trace-shell storage. Canonical-looking query output alone cannot certify the selected physical cell or a subsequent normal read.

## Question

For the five original name families across six storage states, what original-name and short-name namespace which results occur through each retained object-input path?

## Conclusion

The retained 660 semantic query result values establish the listed input-representation and storage-state naming distinctions. Jim returns canonical spelling without requiring storage; C behaviour depends on selected object/string conversion and storage state. Original physical diagnostic windows are referenced by hashes but the TSV projections do not retain their complete fields, so no physical cell/header, normal-read or alias-following capability is granted.

## Scope

C84-91 sixty cases each: five raw00/modified00/rawFF/encodedFF/plain name families, six absent/full-only/short-only/both/declared-undefined/trace-undefined states, resident-string and pure-byte-array paths. Jim thirty resident-string cases. The source captures query completion/options before direct cell/read observers; this record compares only original/short query result bytes retained in semantic TSVs.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.4 8.4.20.

Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.

| name/state | object input path | original result | short result |
| --- | --- | --- | --- |
| raw00/absent | resident-string | "" | "" |
| raw00/absent | pure-byte-array | "" | "" |
| raw00/full-only | resident-string | "::a" | "::a" |
| raw00/full-only | pure-byte-array | "::a\\xc0\\x80z" | "" |
| raw00/short-only | resident-string | "::a" | "::a" |
| raw00/short-only | pure-byte-array | "" | "::a" |
| raw00/both | resident-string | "::a" | "::a" |
| raw00/both | pure-byte-array | "::a\\xc0\\x80z" | "::a" |
| raw00/declared-undefined | resident-string | "::a" | "::a" |
| raw00/declared-undefined | pure-byte-array | "::a\\xc0\\x80z" | "" |
| raw00/trace-undefined | resident-string | "::a" | "::a" |
| raw00/trace-undefined | pure-byte-array | "::a\\xc0\\x80z" | "" |
| modified00/absent | resident-string | "" | "" |
| modified00/absent | pure-byte-array | "" | "" |
| modified00/full-only | resident-string | "::a\\xc0\\x80z" | "" |
| modified00/full-only | pure-byte-array | "::a\u00c0\u0080z" | "" |
| modified00/short-only | resident-string | "" | "::a" |
| modified00/short-only | pure-byte-array | "" | "::a" |
| modified00/both | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/both | pure-byte-array | "::a\u00c0\u0080z" | "::a" |
| modified00/declared-undefined | resident-string | "::a\\xc0\\x80z" | "" |
| modified00/declared-undefined | pure-byte-array | "::a\u00c0\u0080z" | "" |
| modified00/trace-undefined | resident-string | "::a\\xc0\\x80z" | "" |
| modified00/trace-undefined | pure-byte-array | "::a\u00c0\u0080z" | "" |
| rawFF/absent | resident-string | "" | "" |
| rawFF/absent | pure-byte-array | "" | "" |
| rawFF/full-only | resident-string | "::a\\xff" | "" |
| rawFF/full-only | pure-byte-array | "::a\u00ff" | "" |
| rawFF/short-only | resident-string | "" | "::a" |
| rawFF/short-only | pure-byte-array | "" | "::a" |
| rawFF/both | resident-string | "::a\\xff" | "::a" |
| rawFF/both | pure-byte-array | "::a\u00ff" | "::a" |
| rawFF/declared-undefined | resident-string | "::a\\xff" | "" |
| rawFF/declared-undefined | pure-byte-array | "::a\u00ff" | "" |
| rawFF/trace-undefined | resident-string | "::a\\xff" | "" |
| rawFF/trace-undefined | pure-byte-array | "::a\u00ff" | "" |
| encodedFF/absent | resident-string | "" | "" |
| encodedFF/absent | pure-byte-array | "" | "" |
| encodedFF/full-only | resident-string | "::a\u00ff" | "" |
| encodedFF/full-only | pure-byte-array | "::a\u00c3\u00bf" | "" |
| encodedFF/short-only | resident-string | "" | "::a" |
| encodedFF/short-only | pure-byte-array | "" | "::a" |
| encodedFF/both | resident-string | "::a\u00ff" | "::a" |
| encodedFF/both | pure-byte-array | "::a\u00c3\u00bf" | "::a" |
| encodedFF/declared-undefined | resident-string | "::a\u00ff" | "" |
| encodedFF/declared-undefined | pure-byte-array | "::a\u00c3\u00bf" | "" |
| encodedFF/trace-undefined | resident-string | "::a\u00ff" | "" |
| encodedFF/trace-undefined | pure-byte-array | "::a\u00c3\u00bf" | "" |
| plain/absent | resident-string | "" | "" |
| plain/absent | pure-byte-array | "" | "" |
| plain/full-only | resident-string | "::a" | "::a" |
| plain/full-only | pure-byte-array | "::a" | "::a" |
| plain/short-only | resident-string | "::a" | "::a" |
| plain/short-only | pure-byte-array | "::a" | "::a" |
| plain/both | resident-string | "::a" | "::a" |
| plain/both | pure-byte-array | "::a" | "::a" |
| plain/declared-undefined | resident-string | "::a" | "::a" |
| plain/declared-undefined | pure-byte-array | "::a" | "::a" |
| plain/trace-undefined | resident-string | "::a" | "::a" |
| plain/trace-undefined | pure-byte-array | "::a" | "::a" |

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.

| name/state | object input path | original result | short result |
| --- | --- | --- | --- |
| raw00/absent | resident-string | "" | "" |
| raw00/absent | pure-byte-array | "" | "" |
| raw00/full-only | resident-string | "" | "" |
| raw00/full-only | pure-byte-array | "::a\\xc0\\x80z" | "" |
| raw00/short-only | resident-string | "::a" | "::a" |
| raw00/short-only | pure-byte-array | "" | "::a" |
| raw00/both | resident-string | "::a" | "::a" |
| raw00/both | pure-byte-array | "::a\\xc0\\x80z" | "::a" |
| raw00/declared-undefined | resident-string | "" | "" |
| raw00/declared-undefined | pure-byte-array | "::a\\xc0\\x80z" | "" |
| raw00/trace-undefined | resident-string | "::a" | "::a" |
| raw00/trace-undefined | pure-byte-array | "::a\\xc0\\x80z" | "" |
| modified00/absent | resident-string | "" | "" |
| modified00/absent | pure-byte-array | "" | "" |
| modified00/full-only | resident-string | "::a\\xc0\\x80z" | "" |
| modified00/full-only | pure-byte-array | "::a\u00c0\u0080z" | "" |
| modified00/short-only | resident-string | "" | "::a" |
| modified00/short-only | pure-byte-array | "" | "::a" |
| modified00/both | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/both | pure-byte-array | "::a\u00c0\u0080z" | "::a" |
| modified00/declared-undefined | resident-string | "::a\\xc0\\x80z" | "" |
| modified00/declared-undefined | pure-byte-array | "::a\u00c0\u0080z" | "" |
| modified00/trace-undefined | resident-string | "::a\\xc0\\x80z" | "" |
| modified00/trace-undefined | pure-byte-array | "::a\u00c0\u0080z" | "" |
| rawFF/absent | resident-string | "" | "" |
| rawFF/absent | pure-byte-array | "" | "" |
| rawFF/full-only | resident-string | "::a\\xff" | "" |
| rawFF/full-only | pure-byte-array | "::a\u00ff" | "" |
| rawFF/short-only | resident-string | "" | "::a" |
| rawFF/short-only | pure-byte-array | "" | "::a" |
| rawFF/both | resident-string | "::a\\xff" | "::a" |
| rawFF/both | pure-byte-array | "::a\u00ff" | "::a" |
| rawFF/declared-undefined | resident-string | "::a\\xff" | "" |
| rawFF/declared-undefined | pure-byte-array | "::a\u00ff" | "" |
| rawFF/trace-undefined | resident-string | "::a\\xff" | "" |
| rawFF/trace-undefined | pure-byte-array | "::a\u00ff" | "" |
| encodedFF/absent | resident-string | "" | "" |
| encodedFF/absent | pure-byte-array | "" | "" |
| encodedFF/full-only | resident-string | "::a\u00ff" | "" |
| encodedFF/full-only | pure-byte-array | "::a\u00c3\u00bf" | "" |
| encodedFF/short-only | resident-string | "" | "::a" |
| encodedFF/short-only | pure-byte-array | "" | "::a" |
| encodedFF/both | resident-string | "::a\u00ff" | "::a" |
| encodedFF/both | pure-byte-array | "::a\u00c3\u00bf" | "::a" |
| encodedFF/declared-undefined | resident-string | "::a\u00ff" | "" |
| encodedFF/declared-undefined | pure-byte-array | "::a\u00c3\u00bf" | "" |
| encodedFF/trace-undefined | resident-string | "::a\u00ff" | "" |
| encodedFF/trace-undefined | pure-byte-array | "::a\u00c3\u00bf" | "" |
| plain/absent | resident-string | "" | "" |
| plain/absent | pure-byte-array | "" | "" |
| plain/full-only | resident-string | "::a" | "::a" |
| plain/full-only | pure-byte-array | "::a" | "::a" |
| plain/short-only | resident-string | "::a" | "::a" |
| plain/short-only | pure-byte-array | "::a" | "::a" |
| plain/both | resident-string | "::a" | "::a" |
| plain/both | pure-byte-array | "::a" | "::a" |
| plain/declared-undefined | resident-string | "::a" | "::a" |
| plain/declared-undefined | pure-byte-array | "::a" | "::a" |
| plain/trace-undefined | resident-string | "::a" | "::a" |
| plain/trace-undefined | pure-byte-array | "::a" | "::a" |

### jim

Status: `observed`. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c.

Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.

| name/state | object input path | original result | short result |
| --- | --- | --- | --- |
| raw00/absent | resident-string | "::a\x00z" | "::a" |
| raw00/full-only | resident-string | "::a\x00z" | "::a" |
| raw00/short-only | resident-string | "::a\x00z" | "::a" |
| raw00/both | resident-string | "::a\x00z" | "::a" |
| raw00/declared-undefined | resident-string | "::a\x00z" | "::a" |
| raw00/trace-undefined | resident-string | "::a\x00z" | "::a" |
| modified00/absent | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/full-only | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/short-only | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/both | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/declared-undefined | resident-string | "::a\\xc0\\x80z" | "::a" |
| modified00/trace-undefined | resident-string | "::a\\xc0\\x80z" | "::a" |
| rawFF/absent | resident-string | "::a\\xff" | "::a" |
| rawFF/full-only | resident-string | "::a\\xff" | "::a" |
| rawFF/short-only | resident-string | "::a\\xff" | "::a" |
| rawFF/both | resident-string | "::a\\xff" | "::a" |
| rawFF/declared-undefined | resident-string | "::a\\xff" | "::a" |
| rawFF/trace-undefined | resident-string | "::a\\xff" | "::a" |
| encodedFF/absent | resident-string | "::a\u00ff" | "::a" |
| encodedFF/full-only | resident-string | "::a\u00ff" | "::a" |
| encodedFF/short-only | resident-string | "::a\u00ff" | "::a" |
| encodedFF/both | resident-string | "::a\u00ff" | "::a" |
| encodedFF/declared-undefined | resident-string | "::a\u00ff" | "::a" |
| encodedFF/trace-undefined | resident-string | "::a\u00ff" | "::a" |
| plain/absent | resident-string | "::a" | "::a" |
| plain/full-only | resident-string | "::a" | "::a" |
| plain/short-only | resident-string | "::a" | "::a" |
| plain/both | resident-string | "::a" | "::a" |
| plain/declared-undefined | resident-string | "::a" | "::a" |
| plain/trace-undefined | resident-string | "::a" | "::a" |

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `namespace_queries_match_660_fixed_native_original_and_short_results`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `interp::native_namespace_variable_fixture_tests::namespace_queries_match_660_fixed_native_original_and_short_results` (linked): Compares the 660 fixed original/short query result values, not every field of the separately hashed raw native diagnostic logs.

## Evidence

- [rust/tcl-vm/tests/data/native_namespace_variables](../../../../rust/tcl-vm/tests/data/native_namespace_variables)
