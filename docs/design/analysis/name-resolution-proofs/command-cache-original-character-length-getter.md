# naming.command-cache.original-character-length-getter

Kind: `native-observation`

## Problem statement

Character length can use a one-byte fast path without replacing cmdName, while other lengths materialize string primary. A numerical length alone cannot attest that a getter changed the cache.

## Question

Does character-length retrieval replace the original command-name primary for all six registered names?

## Conclusion

Lengths are 1,3,4,3,3,3. C8.4/8.5 replaces cmdName with string for all six; C8.6/9.0/9.1 preserves cmdName for the one-byte name 1 and installs string for the others. Every original resident string pointer remains unchanged. All inputs are ASCII, so this does not establish Unicode length semantics.

## Scope

Six ASCII original Tcl_NewStringObj names per C release, each registered and resolved by Tcl_GetCommandFromObj before the selected public getter. The observer samples original type, getter completion/numeric projection, new type, residency and original string-pointer equality. These original native captures prove only those object windows; an immutable Rust descriptor does not attest current interpreter lookup, live command allocation or normal completion. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Version: tcl8.4 8.4.20 (manifest release association; probe does not query runtime patchlevel); tcl8.5 8.5.19 (manifest release association; probe does not query runtime patchlevel).

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","string","1","1"],["0","3","string","1","1"],["0","4","string","1","1"],["0","3","string","1","1"],["0","3","string","1","1"],["0","3","string","1","1"]].

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Version: tcl8.6 8.6.18 (manifest release association; probe does not query runtime patchlevel); tcl9.0 9.0.4 (manifest release association; probe does not query runtime patchlevel); tcl9.1 9.1.0 (manifest release association; probe does not query runtime patchlevel).

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","cmdName","1","1"],["0","3","string","1","1"],["0","4","string","1","1"],["0","3","string","1","1"],["0","3","string","1","1"],["0","3","string","1","1"]].

### jim, bigip

Status: `not-tested`.

No observation of this exact compiled-object question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/value_ops.rs](../../../../runtime/rust/src/value_ops.rs), `native_char_len`: Consumer/cache projection is independently compared with the exact native stage; source metadata does not grant live command-world cache identity.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_getters_match_all_native_original_object_rows` (linked): The relevant six-per-release stage compares result, resulting primary, cache retention, resident original pointer and exact input bytes. C8.4 dictionary sentinel rows are skipped as unavailable.

## Evidence

- [runtime/rust/tests/data/native_command_cache_getters/probe.c](../../../../runtime/rust/tests/data/native_command_cache_getters/probe.c)
- [runtime/rust/tests/data/native_command_cache_getters/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_getters/manifest.json)
- [runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv)
