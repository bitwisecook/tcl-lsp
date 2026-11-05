# naming.command-cache.original-dictionary-size-getter

Kind: `native-observation`

## Problem statement

A dictionary getter reaches list-pair validation through the original object. Unsupported API compilation, failed conversion and successful dictionary conversion are separate outcomes.

## Question

For the six original registered command names, which dictionary-size conversions are actually called and what primary remains?

## Conclusion

C8.5–9.1 accepts 1 2 and 1 x as one-entry dictionaries and installs dict. The four odd-length strings fail and retain cmdName. All called objects retain the original resident string pointer. C8.4 compiles the dictionary getter out and prints six sentinel -1 rows, so this probe supplies no C8.4 dictionary invocation or guest rejection.

## Scope

Six ASCII original Tcl_NewStringObj names per C release, each registered and resolved by Tcl_GetCommandFromObj before the selected public getter. The observer samples original type, getter completion/numeric projection, new type, residency and original string-pointer equality. These original native captures prove only those object windows; an immutable Rust descriptor does not attest current interpreter lookup, live command allocation or normal completion. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: compiled C probe sentinel; dictionary API call excluded by preprocessor. Version: tcl8.4 8.4.20 (manifest release association).

The six -1 rows are observer sentinels, not guest API errors; no dictionary getter runs in this C8.4 probe. Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["-1","0","cmdName","1","1"],["-1","0","cmdName","1","1"],["-1","0","cmdName","1","1"],["-1","0","cmdName","1","1"],["-1","0","cmdName","1","1"],["-1","0","cmdName","1","1"]].

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Version: tcl8.5 8.5.19 (manifest release association; probe does not query runtime patchlevel); tcl8.6 8.6.18 (manifest release association; probe does not query runtime patchlevel); tcl9.0 9.0.4 (manifest release association; probe does not query runtime patchlevel); tcl9.1 9.1.0 (manifest release association; probe does not query runtime patchlevel).

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["0","1","dict","1","1"],["0","1","dict","1","1"],["1","0","cmdName","1","1"]].

### jim, bigip

Status: `not-tested`.

No observation of this exact compiled-object question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/dict.rs](../../../../runtime/rust/src/dict.rs), `ensure_dict_native`: Consumer/cache projection is independently compared with the exact native stage; source metadata does not grant live command-world cache identity.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_getters_match_all_native_original_object_rows` (linked): The relevant six-per-release stage compares result, resulting primary, cache retention, resident original pointer and exact input bytes. C8.4 dictionary sentinel rows are skipped as unavailable.

## Evidence

- [runtime/rust/tests/data/native_command_cache_getters/probe.c](../../../../runtime/rust/tests/data/native_command_cache_getters/probe.c)
- [runtime/rust/tests/data/native_command_cache_getters/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_getters/manifest.json)
- [runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv)
- [runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv)
