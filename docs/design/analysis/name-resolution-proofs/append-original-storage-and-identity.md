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

Status: `observed`. Version: 8.4.20 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl8.4.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","1","string","1","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl8.5

Status: `observed`. Version: 8.5.19 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl8.5.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","1","string","1","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl8.6

Status: `observed`. Version: 8.6.18 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl8.6.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl9.0.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: tcl9.1.

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### jim

Status: `observed`. Version: Jim (patchlevel/revision not recorded by this probe). Build: See historical references.. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Dialect: jim.

90 cases with routes {"ordinary": 42, "self-alias": 6, "shared-destination": 42}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","1","string","1","0","0","0","1","0","0","35"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-syntax/tests/data/native_object_append/probe.c](../../../../rust/tcl-syntax/tests/data/native_object_append/probe.c). Original physical windows, constructors and captured result/return-option observer.
- `identity-probe` (input): [rust/tcl-syntax/tests/data/native_object_append/identity-probe.c](../../../../rust/tcl-syntax/tests/data/native_object_append/identity-probe.c). Original physical windows also sample source/destination/alias/result pointer relations.
- `receipt` (provider): [rust/tcl-syntax/tests/data/native_object_append/manifest.json](../../../../rust/tcl-syntax/tests/data/native_object_append/manifest.json). Original provider/header/library/binary/projection digests; missing full logs are a replay limitation.
- `rows-tcl8.4` (observation): [rust/tcl-syntax/tests/data/native_object_append/8.4.20.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/8.4.20.tsv). Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl8.5` (observation): [rust/tcl-syntax/tests/data/native_object_append/8.5.19.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/8.5.19.tsv). Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl8.6` (observation): [rust/tcl-syntax/tests/data/native_object_append/8.6.18.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/8.6.18.tsv). Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl9.0` (observation): [rust/tcl-syntax/tests/data/native_object_append/9.0.4.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/9.0.4.tsv). Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-tcl9.1` (observation): [rust/tcl-syntax/tests/data/native_object_append/9.1.0.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/9.1.0.tsv). Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.
- `rows-jim` (observation): [rust/tcl-syntax/tests/data/native_object_append/jim.tsv](../../../../rust/tcl-syntax/tests/data/native_object_append/jim.tsv). Exact 17-column captured projection: case,path,source type/residency before,destination type/residency before,source type/residency after,destination type/residency after,source=dest,alias=dest,alias=source,result=dest,result=source,completion,result hex.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_append_preserves_every_measured_primary_storage_and_identity_window`: Compares the exact projected source/receiver primaries, residency, pointer relationships and result bytes independently of absent raw logs.
- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_append_preserves_every_measured_primary_storage_and_identity_window` (linked): Recreates the fixed original value constructors and compares every retained17-column storage/identity/completion/result projection. A linked comparison is not an executed Rust result.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. Both original probe sources match manifest hashes and all six retained TSVs match fixture_sha256. Full physical_log_sha256/identity_log_sha256 objects are absent; a complete raw-log replay cannot be asserted from the projection. Any fresh run must pin actual version/configuration/header/library identity and preserve observer ordering before rendering.
