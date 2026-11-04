# naming.command-cache.original-boolean-getter

Kind: `native-observation`

## Problem statement

Boolean parsing can accept 1.0 while retaining a numeric primary, or install release-specific boolean storage. A successful Boolean result cannot identify which cache remains.

## Question

Which original command-name spellings pass the Boolean getter, and which primary remains in each tested release?

## Conclusion

1, 1.0 and true succeed with Boolean value 1; the other three fail and retain cmdName. C8.4 installs boolean for successful inputs. C8.5/8.6 leaves int, double and booleanString respectively; C9.0/9.1 leaves int, double and boolean. Every original resident string pointer is unchanged.

## Scope

Six ASCII original Tcl_NewStringObj names per C release, each registered and resolved by Tcl_GetCommandFromObj before the selected public getter. The observer samples original type, getter completion/numeric projection, new type, residency and original string-pointer equality. These original native captures prove only those object windows; an immutable Rust descriptor does not attest current interpreter lookup, live command allocation or normal completion. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl8.4.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","boolean","1","1"],["0","1","boolean","1","1"],["0","1","boolean","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl8.5

Status: `observed`. Version: 8.5.19 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl8.5.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["0","1","booleanString","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl8.6

Status: `observed`. Version: 8.6.18 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl8.6.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["0","1","booleanString","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl9.0.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["0","1","boolean","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: See historical references.. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl9.1.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["0","1","boolean","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact compiled-object question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Historical references

- `probe` (input): [runtime/rust/tests/data/native_command_cache_getters/probe.c](../../../../runtime/rust/tests/data/native_command_cache_getters/probe.c). Exact original constructors, command registration/lookup, getter calls and physical observation order.
- `receipt` (provider): [runtime/rust/tests/data/native_command_cache_getters/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_getters/manifest.json). Original compiler arguments, library/executable digests, statuses and per-output digests.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv). Original output; select stage 2, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv). Original output; select stage 2, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv). Original output; select stage 2, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv). Original output; select stage 2, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv). Original output; select stage 2, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Consumer/cache projection is independently compared with the exact native stage; source metadata does not grant live command-world cache identity.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_getters_match_all_native_original_object_rows` (linked): The relevant six-per-release stage compares result, resulting primary, cache retention, resident original pointer and exact input bytes. C8.4 dictionary sentinel rows are skipped as unavailable.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. The retained probe and full TSV outputs match their original receipt hashes. Original compiler arguments refer to capture-time paths; no portable executable replay runner is attached. A new run must independently select the release headers/library, compile this exact probe and compare every original output row and process completion.
