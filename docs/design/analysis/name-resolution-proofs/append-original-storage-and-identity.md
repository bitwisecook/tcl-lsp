# naming.append.original-storage-and-identity

Kind: `native-observation`

## Problem statement

Appending a value to a missing, shared or self-aliased variable can either reuse an original object or allocate/convert a receiver. Equal rendered bytes do not establish which primary, resident string or alias relationship remains. Consumers of original values need the actual before/after window before the observer renders the result.

## Question

Across the fixed append receiver/source matrix, which original storage and result identities remain immediately after native append?

## Conclusion

The retained projection records 1035 append cases: 189 per C release and 90 for Jim, with ordinary, shared-destination and self-alias routes. C Tcl adopts an original integer source for a missing destination without materializing its string; Jim materializes the source and returns a distinct resident string. For pure bytearray pairs, C8.4/8.5 renders source/receiver strings, whereas C8.6/9.0/9.1 keeps bytearray primaries and absent resident strings at the sampled window. All individual primary/storage/identity axes remain in the exact row matrix; these examples do not replace its release-specific cases.

## Scope

Original objects are constructed by compiled public C/Jim APIs and passed through Tcl_EvalObjv/Jim_EvalObjVector. Physical samples precede result rendering; the observer separately captures original result/return options. The six hash-matched TSVs are retained projections, while the full physical/identity JSONL logs named by the manifest are unavailable. Thus 7041 raw log rows, global side effects and complete process status are not reasserted as retained evidence. Jim patch/revision/build configuration is unqueried; its recorded header/library/binary hashes identify that capture. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=e8bf674dfe66dcd687f1622ab95d36c2814b8afef22dc0589a62462248c3ae52; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl8.4.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","1","string","1","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl8.5

Status: `observed`. Version: 8.5.19 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=581dff2777b501d838635bc611978192101d123ae596ea967d73e395e6c41778; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl8.5.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","1","string","1","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl8.6

Status: `observed`. Version: 8.6.18 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=e8edbc2bef689e349a6942eb5213bd2d76d9906d3796e01c2b62eca4aecf5cab; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl8.6.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=980e6ea9860ffd6eada7c709969dab995585766909c18da4f352d39f8375bcf0; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl9.0.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: binary_sha256=4753c0fdbd540a4b4cb30aeab87824b6c0098e3c3d0db02985de1d6a38b915cb; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl9.1.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### jim

Status: `observed`. Version: Jim (patchlevel/revision not recorded by this probe). Build: binary_sha256=84d76226e1097bd995a86e3f40fbe14f7a61ad7ecb982e658a19b5c9af79f529; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: jim.

90 cases with routes {"ordinary": 42, "self-alias": 6, "shared-destination": 42}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","1","string","1","0","0","0","1","0","0","35"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-syntax/tests/data/native_object_append/probe.c](../../../../rust/tcl-syntax/tests/data/native_object_append/probe.c). SHA-256 `5aba86871bb6c6aa7659db725cc023045b4c1a14adbea8a01279efd4ad286e48`. Original physical windows, constructors and captured result/return-option observer.
- `identity-probe` (input): [rust/tcl-syntax/tests/data/native_object_append/identity-probe.c](../../../../rust/tcl-syntax/tests/data/native_object_append/identity-probe.c). SHA-256 `60a5cfa6374abfdf38923e909eb868fdeaf0200a729cb393a10e74df707313ee`. Original physical windows also sample source/destination/alias/result pointer relations.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_object_append/manifest.json](../../../../rust/tcl-syntax/tests/data/native_object_append/manifest.json). SHA-256 `1b2e9f4e3b6adb998ce4170387d7b595757a8820c4a85e6f1a5706785f889c48`. Original provider/header/library/binary/projection digests; missing full logs are a replay limitation.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_object_append/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/8.4.20.tsv). SHA-256 `06b932a86dee609d474a637cf81996384a44d1e71c2c7ed0ecf9cb3cb5831958`. Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_object_append/8.5.19.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/8.5.19.tsv). SHA-256 `06b932a86dee609d474a637cf81996384a44d1e71c2c7ed0ecf9cb3cb5831958`. Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_object_append/8.6.18.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/8.6.18.tsv). SHA-256 `1154a12747e703bb61ee74992f68daba02d2849ba79ae53cd18a269253738ab0`. Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_object_append/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/9.0.4.tsv). SHA-256 `83b5ef137828dfe38f523be3935567ff239021c55adf097261c518f6868a80a3`. Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_object_append/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/9.1.0.tsv). SHA-256 `83b5ef137828dfe38f523be3935567ff239021c55adf097261c518f6868a80a3`. Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_object_append/jim.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/jim.tsv). SHA-256 `577cb481aeced1fed0c2db5c75fcf963e04e166a7c70a8f647b7d1ff08735f73`. Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_append_preserves_every_measured_primary_storage_and_identity_window`: Compares the exact projected source/receiver primaries, residency, pointer relationships and result bytes independently of absent raw logs.
- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_append_preserves_every_measured_primary_storage_and_identity_window` (linked): Recreates the fixed original value constructors and compares every retained17-column storage/identity/completion/result projection. A linked comparison is not an executed Rust result.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. Both original probe sources match manifest hashes and all six retained TSVs match fixture_sha256. Full physical_log_sha256/identity_log_sha256 objects are absent; a complete raw-log replay cannot be asserted from the projection. Any fresh run must pin actual version/configuration/header/library identity and preserve observer ordering before rendering.
