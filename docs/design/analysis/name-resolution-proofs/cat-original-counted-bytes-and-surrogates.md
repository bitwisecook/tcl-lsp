# naming.cat.original-counted-bytes-and-surrogates

Kind: `native-observation`

## Problem statement

Counted raw NUL, invalid FF, Unicode surrogate units and malformed UTF bytes can take different concatenation paths. Uniform display conversion would conflate retained original bytes with a Unicode-derived output and obscure sharing-dependent identity.

## Question

Which exact bytes and original identities does C9 TclStringCat retain or convert for raw-zero, invalid-byte and surrogate-unit operands?

## Conclusion

C9.0/9.1 agree on all12 cases across both sharing axes (24 concatenations per release). Raw A00Z followed by A yields41005A41; original FF followed by A yields FF41, but FF followed by the malformed80B path yields C3BF E282AC42. Unicode D800/DC00 units project as separate EDA080/EDB080 units in input order. Shared nonempty first operands generally receive a distinct result, while an empty/raw-zero/empty triple returns its middle original operand. The observed paths do not establish a universal byte-decoding rule.

## Scope

Twelve counted original C9 object constructor combinations with two sharing states. The probe includes raw zero, invalid byteFF, surrogate pair/lone units and malformed80B; private physical snapshots precede final string generation. Exact148 JSONL/TSV windows per release are retained. Other C releases/Jim/BIG-IP have no run for this precise corpus.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No observation of this exact compiled-object question is attached for this provider.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No observation of this exact compiled-object question is attached for this provider.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No observation of this exact compiled-object question is attached for this provider.

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=b016eaac7a44725db8efc1326f6b8e91cb5540f9de01290e2868e67f0ea1951c; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: compiled C private TclStringCat original-object ABI; physical snapshots before result string generation. Dialect: tcl9.0.

Compiled/run status0; 24 concatenations and 148 retained physical/projection rows. Full final [case,sharing,result identities,result hex] observations: [[0,0,[1,0],"41005a41"],[0,1,[0,0],"41005a41"],[1,0,[1,0],"4141005a"],[1,1,[0,0],"4141005a"],[2,0,[1,0],"ff41"],[2,1,[0,0],"ff41"],[3,0,[1,0],"41ff"],[3,1,[0,0],"41ff"],[4,0,[1,0],"c3bfe282ac42"],[4,1,[0,0],"c3bfe282ac42"],[5,0,[1,0],"41c0805ae282ac42"],[5,1,[0,0],"41c0805ae282ac42"],[6,0,[1,0],"eda080edb08041"],[6,1,[0,0],"eda080edb08041"],[7,0,[1,0],"eda080edb080"],[7,1,[0,0],"eda080edb080"],[8,0,[1,0],"edb080eda080"],[8,1,[0,0],"edb080eda080"],[9,0,[1,0],"eda080edb080e282ac42"],[9,1,[0,0],"eda080edb080e282ac42"],[10,0,[1,0],"eda080e282ac42"],[10,1,[0,0],"eda080e282ac42"],[11,0,[0,1,0],"41005a"],[11,1,[0,1,0],"41005a"]]. Every original snapshot is in the linked JSONL; no output rendering is applied before its physical windows.

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=9dd7f8e8c71a0b2614987fd1121811b35fca5be9322b6636af1b170c553e6af3; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: compiled C private TclStringCat original-object ABI; physical snapshots before result string generation. Dialect: tcl9.1.

Compiled/run status0; 24 concatenations and 148 retained physical/projection rows. Full final [case,sharing,result identities,result hex] observations: [[0,0,[1,0],"41005a41"],[0,1,[0,0],"41005a41"],[1,0,[1,0],"4141005a"],[1,1,[0,0],"4141005a"],[2,0,[1,0],"ff41"],[2,1,[0,0],"ff41"],[3,0,[1,0],"41ff"],[3,1,[0,0],"41ff"],[4,0,[1,0],"c3bfe282ac42"],[4,1,[0,0],"c3bfe282ac42"],[5,0,[1,0],"41c0805ae282ac42"],[5,1,[0,0],"41c0805ae282ac42"],[6,0,[1,0],"eda080edb08041"],[6,1,[0,0],"eda080edb08041"],[7,0,[1,0],"eda080edb080"],[7,1,[0,0],"eda080edb080"],[8,0,[1,0],"edb080eda080"],[8,1,[0,0],"edb080eda080"],[9,0,[1,0],"eda080edb080e282ac42"],[9,1,[0,0],"eda080edb080e282ac42"],[10,0,[1,0],"eda080e282ac42"],[10,1,[0,0],"eda080e282ac42"],[11,0,[0,1,0],"41005a"],[11,1,[0,1,0],"41005a"]]. Every original snapshot is in the linked JSONL; no output rendering is applied before its physical windows.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact compiled-object question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_object_cat/bytes/probe.c](../../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/probe.c). SHA-256 `28bdfaa274155b6d531b7ed75c26ac4ef0a450799c46ce5221038a3a90f09f07`. Exact counted/native object constructors, private TclStringCat call and original snapshot/render order.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_object_cat/bytes/manifest.json](../../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/manifest.json). SHA-256 `4a34f17932cf7eb50d4834613e4f9098f1ab975e69b7ad96375f9a561c5892a9`. Original compile/run metadata and header/library/executable/full-log/projection digests.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_object_cat/bytes/9.0.4.jsonl](../../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/9.0.4.jsonl). SHA-256 `b98a6e8654a20569ae2be38739db90fefded0c5eba7218ac21dff7f1acae7a8e`. Full original before/after/result-before-render JSONL and final result identity/hex; observer order is preserved.
- `projection-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_object_cat/bytes/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/9.0.4.tsv). SHA-256 `009a8057def8f7b54cca7712e848135fce6d9990c94e0cf964c74056806065e8`. Exact13-column fixture projection; its digest agrees with the original manifest.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_object_cat/bytes/9.1.0.jsonl](../../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/9.1.0.jsonl). SHA-256 `b98a6e8654a20569ae2be38739db90fefded0c5eba7218ac21dff7f1acae7a8e`. Full original before/after/result-before-render JSONL and final result identity/hex; observer order is preserved.
- `projection-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_object_cat/bytes/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_object_cat/bytes/9.1.0.tsv). SHA-256 `009a8057def8f7b54cca7712e848135fce6d9990c94e0cf964c74056806065e8`. Exact13-column fixture projection; its digest agrees with the original manifest.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_cat.rs](../../../../rust/tcl-cmd-core/src/native_cat.rs), `concatenate`: Consumes the independently selected native concatenation recipe and original borrowed operand objects; the native observation does not itself prove Rust execution.
- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_cat_preserves_raw_nul_invalid_bytes_and_surrogate_windows` (linked): Compares each retained original cache/residency/reference window, adjusted known returned-owner reference, input/result identity and exact counted output bytes under C9.0/9.1.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. Original full JSONL/projection/probe hashes agree with the original receipts. This private-ABI corpus must be compiled against the corresponding release headers/library; capture-time absolute paths are retained metadata, not a current executable replay runner. No earlier-release behavior follows from these C9 runs.
