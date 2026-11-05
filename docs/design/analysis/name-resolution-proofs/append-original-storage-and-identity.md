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

### tcl8.4, tcl8.5

Status: `observed`. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Version: tcl8.4 8.4.20 (manifest release association; probe does not query runtime patchlevel); tcl8.5 8.5.19 (manifest release association; probe does not query runtime patchlevel).

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","1","string","1","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public original-object argv; before/after physical samples precede result rendering. Version: tcl8.6 8.6.18 (manifest release association; probe does not query runtime patchlevel); tcl9.0 9.0.4 (manifest release association; probe does not query runtime patchlevel); tcl9.1 9.1.0 (manifest release association; probe does not query runtime patchlevel).

189 cases with routes {"ordinary": 90, "self-alias": 9, "shared-destination": 90}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","0","int","0","1","0","0","1","1","0","35"],["dest-pure-ba-source-pure-ba","ordinary","bytearray","0","bytearray","0","bytearray","0","bytearray","0","0","0","0","1","0","0","c3bfc080c3bfc080"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### jim

Status: `observed`. Channel: compiled public original-object argv; before/after physical samples precede result rendering.

90 cases with routes {"ordinary": 42, "self-alias": 6, "shared-destination": 42}; all retained completion fields are 0. Example full projection rows: [["dest-missing-source-integer","ordinary","int","0","missing","0","int","1","string","1","0","0","0","1","0","0","35"]]. The complete native log/process-status capture is unavailable; only these hash-matched projections are claimed.

### bigip

Status: `not-tested`.

No observation of this exact compiled-object question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_append_preserves_every_measured_primary_storage_and_identity_window`: Compares the exact projected source/receiver primaries, residency, pointer relationships and result bytes independently of absent raw logs.
- [runtime/rust/src/native_append_tests.rs](../../../../runtime/rust/src/native_append_tests.rs), `native_append_preserves_every_measured_primary_storage_and_identity_window` (linked): Recreates the fixed original value constructors and compares every retained17-column storage/identity/completion/result projection. A linked comparison is not an executed Rust result.

## Evidence

- [rust/tcl-syntax/tests/data/native_object_append](../../../../rust/tcl-syntax/tests/data/native_object_append)
