# naming.cat.original-cache-and-sharing

Kind: `native-observation`

## Problem statement

A concatenation with identical output can return an input or a newly allocated object depending on sharing, empty operands and cache kind. Reading output bytes before the original physical window would erase the storage being measured.

## Question

How does private TclStringCat preserve original cache, residency and input/result identity across the fixed constructors and sharing axis?

## Conclusion

C9.0/9.1 agree on the retained23-case, two-sharing-axis matrix (46 concatenations per release). For A+A, the unshared route returns the first object and the shared route allocates a distinct result. Empty list/dictionary plus A returns the A operand under either sharing axis. Pure bytearray FF00 pairs project as C3BF C080 C3BF C080. Original before/after/result-before-string and final identity/byte windows remain independent in the312 rows per release.

## Scope

Twenty-three fixed C9-only constructor combinations, each with/without an extra reference to operand zero, compiled against private tclInt.h/tclStringRep.h. Exact full JSONL and13-column projected TSVs are retained. C8.4/8.5/8.6/Jim/BIG-IP are not tested by this corpus, rather than inferred to reject the ABI.

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

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=f88297127324677c227c04a673dfeb0c5d9b47450106f308cd30ed759427768e; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: compiled C private TclStringCat original-object ABI; physical snapshots before result string generation. Dialect: tcl9.0.

Compiled/run status0; 46 concatenations and 312 retained physical/projection rows. Full final [case,sharing,result identities,result hex] observations: [[0,0,[1,0],"4141"],[0,1,[0,0],"4141"],[1,0,[1,0],"c3bfc080c3bfc080"],[1,1,[0,0],"c3bfc080c3bfc080"],[2,0,[1,0],"c3bfc08041"],[2,1,[0,0],"c3bfc08041"],[3,0,[1,0],"c3bfc080c3bfc080"],[3,1,[0,0],"c3bfc080c3bfc080"],[4,0,[1,0],"c3a9f09f988041"],[4,1,[0,0],"c3a9f09f988041"],[5,0,[1,0],"41c3a9f09f9880"],[5,1,[0,0],"41c3a9f09f9880"],[6,0,[1,0],"41e282ac42"],[6,1,[0,0],"41e282ac42"],[7,0,[1,0],"378042"],[7,1,[0,0],"378042"],[8,0,[1,0],"c3a9f09f9880c3bfc080"],[8,1,[0,0],"c3a9f09f9880c3bfc080"],[9,0,[0,1],"41"],[9,1,[0,1],"41"],[10,0,[0,1],"41"],[10,1,[0,1],"41"],[11,0,[1,0],"3741"],[11,1,[0,0],"3741"],[12,0,[0,1,0],"c3a9f09f9880"],[12,1,[0,1,0],"c3a9f09f9880"],[13,0,[1,0,0],"c3a9f09f9880"],[13,1,[1,0,0],"c3a9f09f9880"],[14,0,[0,0,1],"37"],[14,1,[0,0,1],"37"],[15,0,[0,0,1],""],[15,1,[0,0,1],""],[16,0,[1,0],"4141"],[16,1,[0,0],"4141"],[17,0,[1,0],"4141"],[17,1,[0,0],"4141"],[18,0,[1,0,0],"37"],[18,1,[1,0,0],"37"],[19,0,[1,0,0],"37"],[19,1,[1,0,0],"37"],[20,0,[0,0,1],""],[20,1,[0,0,1],""],[21,0,[0,1,0],"41"],[21,1,[0,1,0],"41"],[22,0,[1,0,0],"4141"],[22,1,[0,0,0],"4141"]]. Every original snapshot is in the linked JSONL; no output rendering is applied before its physical windows.

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=f4f91ff2cb78f5bca25ea44d55b2f910c942cff17a906d960a66ad2604d866e2; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: compiled C private TclStringCat original-object ABI; physical snapshots before result string generation. Dialect: tcl9.1.

Compiled/run status0; 46 concatenations and 312 retained physical/projection rows. Full final [case,sharing,result identities,result hex] observations: [[0,0,[1,0],"4141"],[0,1,[0,0],"4141"],[1,0,[1,0],"c3bfc080c3bfc080"],[1,1,[0,0],"c3bfc080c3bfc080"],[2,0,[1,0],"c3bfc08041"],[2,1,[0,0],"c3bfc08041"],[3,0,[1,0],"c3bfc080c3bfc080"],[3,1,[0,0],"c3bfc080c3bfc080"],[4,0,[1,0],"c3a9f09f988041"],[4,1,[0,0],"c3a9f09f988041"],[5,0,[1,0],"41c3a9f09f9880"],[5,1,[0,0],"41c3a9f09f9880"],[6,0,[1,0],"41e282ac42"],[6,1,[0,0],"41e282ac42"],[7,0,[1,0],"378042"],[7,1,[0,0],"378042"],[8,0,[1,0],"c3a9f09f9880c3bfc080"],[8,1,[0,0],"c3a9f09f9880c3bfc080"],[9,0,[0,1],"41"],[9,1,[0,1],"41"],[10,0,[0,1],"41"],[10,1,[0,1],"41"],[11,0,[1,0],"3741"],[11,1,[0,0],"3741"],[12,0,[0,1,0],"c3a9f09f9880"],[12,1,[0,1,0],"c3a9f09f9880"],[13,0,[1,0,0],"c3a9f09f9880"],[13,1,[1,0,0],"c3a9f09f9880"],[14,0,[0,0,1],"37"],[14,1,[0,0,1],"37"],[15,0,[0,0,1],""],[15,1,[0,0,1],""],[16,0,[1,0],"4141"],[16,1,[0,0],"4141"],[17,0,[1,0],"4141"],[17,1,[0,0],"4141"],[18,0,[1,0,0],"37"],[18,1,[1,0,0],"37"],[19,0,[1,0,0],"37"],[19,1,[1,0,0],"37"],[20,0,[0,0,1],""],[20,1,[0,0,1],""],[21,0,[0,1,0],"41"],[21,1,[0,1,0],"41"],[22,0,[1,0,0],"4141"],[22,1,[0,0,0],"4141"]]. Every original snapshot is in the linked JSONL; no output rendering is applied before its physical windows.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact compiled-object question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_object_cat/probe.c](../../../../rust/tcl-syntax/tests/data/native_object_cat/probe.c). SHA-256 `6d0a9ef345435e4228d3107936740fc9c3165e18d1ec7ddf149cf30732b1ac3e`. Exact counted/native object constructors, private TclStringCat call and original snapshot/render order.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_object_cat/manifest.json](../../../../rust/tcl-syntax/tests/data/native_object_cat/manifest.json). SHA-256 `6637cecca9eeb397017b855d23659b0f9b30e3df8d8b41dd24b0de81e0801f77`. Original compile/run metadata and header/library/executable/full-log/projection digests.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_object_cat/9.0.4.jsonl](../../../../rust/tcl-syntax/tests/data/native_object_cat/9.0.4.jsonl). SHA-256 `1f480ff295bb886788f9a5457a83d168e4a0f781c40848dc91f6842871d96aea`. Full original before/after/result-before-render JSONL and final result identity/hex; observer order is preserved.
- `projection-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_object_cat/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_object_cat/9.0.4.tsv). SHA-256 `bab68a8ce4ea55e93e0522c8db0783a3569b08fd1b21432eaafd03ab934281b3`. Exact13-column fixture projection; its digest agrees with the original manifest.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_object_cat/9.1.0.jsonl](../../../../rust/tcl-syntax/tests/data/native_object_cat/9.1.0.jsonl). SHA-256 `1f480ff295bb886788f9a5457a83d168e4a0f781c40848dc91f6842871d96aea`. Full original before/after/result-before-render JSONL and final result identity/hex; observer order is preserved.
- `projection-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_object_cat/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_object_cat/9.1.0.tsv). SHA-256 `bab68a8ce4ea55e93e0522c8db0783a3569b08fd1b21432eaafd03ab934281b3`. Exact13-column fixture projection; its digest agrees with the original manifest.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_cat.rs](../../../../rust/tcl-cmd-core/src/native_cat.rs), `concatenate`: Consumes the independently selected native concatenation recipe and original borrowed operand objects; the native observation does not itself prove Rust execution.
- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_cat_preserves_all_original_cache_and_identity_windows` (linked): Compares each retained original cache/residency/reference window, adjusted known returned-owner reference, input/result identity and exact counted output bytes under C9.0/9.1.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. Original full JSONL/projection/probe hashes agree with the original receipts. This private-ABI corpus must be compiled against the corresponding release headers/library; capture-time absolute paths are retained metadata, not a current executable replay runner. No earlier-release behavior follows from these C9 runs.
