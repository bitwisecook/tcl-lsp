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

Status: `observed`. Version: 8.4.20. Build: binary_sha256=113e7f96e525217bfe756c702737df1434c2ef6c12e7406078e4b701557426df; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: binary_sha256=8851ed4d50d09b40bfd3a865273946f496d2e8d59ae4d8eb508db3398be0f9e4; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: binary_sha256=245ecbfa7925183a259cd146dce1efa0631d77d1cb3d993fc1f01b99afe2f096; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: binary_sha256=7b01bad5de07fe8879f56671ffb8e13cc752e65bc46b9cc08db8a8fb6b7e723d; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: binary_sha256=f3b3e1c94ef0ea8dcd28aaf9f1acd219ed7c87ef565c7f7b1eeceab8ffbd75d1; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

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

Status: `observed`. Version: not recorded (manifest label Jim). Build: binary_sha256=e25a463020da46f0a18329c3eabbebc3dcb268eaa7f4fe7a8295228463817538; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Jim Tcl.

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

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-vm/tests/data/native_namespace_variables/manifest.json](../../../../rust/tcl-vm/tests/data/native_namespace_variables/manifest.json). SHA-256 `90ce8e8f4ee3455d527de2f41d33674b3eb819236af92c2cda426f2a4aa27e61`. Five C360-window and Jim180-window raw-log hashes, semantic TSV hashes and exact native header/library/binary builds; raw logs themselves are not retained here.
- `e1` (input): [rust/tcl-vm/tests/data/native_namespace_variables/probe.c](../../../../rust/tcl-vm/tests/data/native_namespace_variables/probe.c). SHA-256 `61c32b56bd6540e4f9c248829b3adc5b77aed53fff7fe4457ee2f9ada6a6a542`. Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [rust/tcl-vm/tests/data/native_namespace_variables/README.md](../../../../rust/tcl-vm/tests/data/native_namespace_variables/README.md). SHA-256 `84f9ef88b404c0be8c5715260426147b3c957697a6d43d1ea05226e1a588b3c8`. Retained exact README.md; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (observation): [rust/tcl-vm/tests/data/native_namespace_variables/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_variables/8.4.20.tsv). SHA-256 `160a3d63de26d075d42aebe265334cf0993415e2ef333548d972a9034d6b2bc4`. Complete retained semantic TSV for 8.4.20; Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.
- `e4` (observation): [rust/tcl-vm/tests/data/native_namespace_variables/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_variables/8.5.19.tsv). SHA-256 `171bf838defe0d5037ef2f1b927b52d94a5363be6c116bafa7a5ef29f6811c0b`. Complete retained semantic TSV for 8.5.19; Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.
- `e5` (observation): [rust/tcl-vm/tests/data/native_namespace_variables/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_variables/8.6.18.tsv). SHA-256 `171bf838defe0d5037ef2f1b927b52d94a5363be6c116bafa7a5ef29f6811c0b`. Complete retained semantic TSV for 8.6.18; Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.
- `e6` (observation): [rust/tcl-vm/tests/data/native_namespace_variables/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_variables/9.0.4.tsv). SHA-256 `171bf838defe0d5037ef2f1b927b52d94a5363be6c116bafa7a5ef29f6811c0b`. Complete retained semantic TSV for 9.0.4; Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.
- `e7` (observation): [rust/tcl-vm/tests/data/native_namespace_variables/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_variables/9.1.0.tsv). SHA-256 `171bf838defe0d5037ef2f1b927b52d94a5363be6c116bafa7a5ef29f6811c0b`. Complete retained semantic TSV for 9.1.0; Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.
- `e8` (observation): [rust/tcl-vm/tests/data/native_namespace_variables/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_namespace_variables/Jim.tsv). SHA-256 `914ca0cf8de013381ede849a36a4fb7de6a98e1bf208ab61541b9fa02a394e1c`. Complete retained semantic TSV for Jim; Quoted values below are exact byte strings. Empty string is an empty query result; each original and short query is distinct.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `namespace_queries_match_660_fixed_native_original_and_short_results`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `interp::native_namespace_variable_fixture_tests::namespace_queries_match_660_fixed_native_original_and_short_results` (linked): Compares the 660 fixed original/short query result values, not every field of the separately hashed raw native diagnostic logs.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-vm/tests/data/native_namespace_variables/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Rebuild retained probe.c with each exact manifest command and authentic original release/header/archive, including separate Jim USE_JIM/libjim flags. Run one fresh probe process per provider; capture the emitted raw windows and apply the original/short query result projection stated by probe.c and each TSV. Require exactly the 60 C or 30 Jim semantic rows (two result values each), matching all retained query bytes, and host exit 0. The complete original raw logs are not retained here, only SHA256, so comparing every raw physical diagnostic field is unavailable without recovering that log. Pure byte-array materialisation differs from counted resident strings; do not replay via a shell document channel. No Rust pass is inferred.
