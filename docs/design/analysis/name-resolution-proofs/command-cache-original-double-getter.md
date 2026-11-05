# naming.command-cache.original-double-getter

Kind: `native-observation`

## Problem statement

A floating getter may reuse an integer primary for an integral spelling while another release always installs double. Treating the returned number as proof of one uniform primary would erase this object difference.

## Question

After successful original command lookup, what primary and resident storage does the double getter leave on six original names?

## Conclusion

Spellings 1 and 1.0 succeed with the printed numeric projection 1. C8.4 installs double for both; C8.5–9.1 installs int for 1 and double for 1.0. Other inputs fail and retain cmdName; all original string pointers remain identical. The probe casts double output to long long and does not measure arbitrary floating results.

## Scope

Six ASCII original Tcl_NewStringObj names per C release, each registered and resolved by Tcl_GetCommandFromObj before the selected public getter. The observer samples original type, getter completion/numeric projection, new type, residency and original string-pointer equality. These original native captures prove only those object windows; an immutable Rust descriptor does not attest current interpreter lookup, live command allocation or normal completion. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Version: tcl8.4 8.4.20 (manifest release association; probe does not query runtime patchlevel).

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","double","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Version: tcl8.5 8.5.19 (manifest release association; probe does not query runtime patchlevel); tcl8.6 8.6.18 (manifest release association; probe does not query runtime patchlevel); tcl9.0 9.0.4 (manifest release association; probe does not query runtime patchlevel); tcl9.1 9.1.0 (manifest release association; probe does not query runtime patchlevel).

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### jim, bigip

Status: `not-tested`.

No observation of this exact compiled-object question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Consumer/cache projection is independently compared with the exact native stage; source metadata does not grant live command-world cache identity.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_getters_match_all_native_original_object_rows` (linked): The relevant six-per-release stage compares result, resulting primary, cache retention, resident original pointer and exact input bytes. C8.4 dictionary sentinel rows are skipped as unavailable.

## Evidence

- [runtime/rust/tests/data/native_command_cache_getters/probe.c](../../../../runtime/rust/tests/data/native_command_cache_getters/probe.c)
- [runtime/rust/tests/data/native_command_cache_getters/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_getters/manifest.json)
- [runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv)
