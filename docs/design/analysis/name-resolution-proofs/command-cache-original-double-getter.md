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

Status: `observed`. Version: 8.4.20 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=935f1118e51b6011a30690b36645cf175923eb084443554fe224805f17f5d83d; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl8.4.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","double","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl8.5

Status: `observed`. Version: 8.5.19 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=4b372a3c7b408500eb51b52e574edd7f6e57ee21b403b52172cbf8a831c939fd; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl8.5.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl8.6

Status: `observed`. Version: 8.6.18 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=9bd366a4e5524e8c5bc0609abcd8db9da65218fb09043c396fb606cb6003ad45; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl8.6.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl9.0

Status: `observed`. Version: 9.0.4 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=47511a24ec7525aaa556aef37de10f7a9018c7b643513ebaf36784fdd474af0d; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl9.0.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### tcl9.1

Status: `observed`. Version: 9.1.0 (manifest release association; probe does not query runtime patchlevel). Build: elf_sha256=420e427211d56a9416e5ffa5405fcb1a5aa8a0952533fe731beab44a84d9dd43; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33. Channel: compiled C public original-object getter after Tcl_GetCommandFromObj. Dialect: tcl9.1.

Case-order observations [code,numeric projection,after type,resident,same original pointer]: [["0","1","int","1","1"],["0","1","double","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"],["1","0","cmdName","1","1"]].

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No observation of this exact compiled-object question is attached for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No observation of this exact compiled-object question is attached for this provider.

## Exact evidence

- `probe` (input): [runtime/rust/tests/data/native_command_cache_getters/probe.c](../../../../runtime/rust/tests/data/native_command_cache_getters/probe.c). SHA-256 `605d8b7b35d3d3c692769418dd69a1ed15443234a608a202c3898ad56a5532aa`. Exact original constructors, command registration/lookup, getter calls and physical observation order.
- `receipt` (provider): [runtime/rust/tests/data/native_command_cache_getters/manifest.json](../../../../runtime/rust/tests/data/native_command_cache_getters/manifest.json). SHA-256 `3e74bf58654d88ddffa93c83d84742ff243e0a4327d235697171a5d49bfbddc5`. Original compiler arguments, library/executable digests, statuses and per-output digests.
- `rows-tcl8.4` (observation): [runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.4.20.tsv). SHA-256 `322330c7d50f288920a57d6473f8477079e819a7af53996f3af0b0d53374dafc`. Original output; select stage 1, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl8.5` (observation): [runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.5.19.tsv). SHA-256 `6d45304fd79aaa7841c4ad5718f97d98bb362de28515945a0a6ecf31f6aa7ceb`. Original output; select stage 1, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl8.6` (observation): [runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/8.6.18.tsv). SHA-256 `d97f27d6791b28d73e94e9ee9ee3f2cdcdbb38a82afc591841199f9bedb66f98`. Original output; select stage 1, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl9.0` (observation): [runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.0.4.tsv). SHA-256 `7fa77a9a86c3fbdacbcc7a4fa896df631453e0c640a247baa19fa564e6490651`. Original output; select stage 1, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.
- `rows-tcl9.1` (observation): [runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv](../../../../runtime/rust/tests/data/native_command_cache_getters/9.1.0.tsv). SHA-256 `7fa77a9a86c3fbdacbcc7a4fa896df631453e0c640a247baa19fa564e6490651`. Original output; select stage 1, six cases. Columns: stage, case, prior type, code, numeric projection, resulting type, residency, same original string pointer.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Consumer/cache projection is independently compared with the exact native stage; source metadata does not grant live command-world cache identity.
- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `tests::command_cache_getters_match_all_native_original_object_rows` (linked): The relevant six-per-release stage compares result, resulting primary, cache retention, resident original pointer and exact input bytes. C8.4 dictionary sentinel rows are skipped as unavailable.

A named test is a coverage binding, not a claim that it executed.

## Replay

No fresh native run or executed Rust test is asserted by this record. The retained probe and full TSV outputs match their original receipt hashes. Original compiler arguments refer to capture-time paths; no portable executable replay runner is attached. A new run must independently select the release headers/library, compile this exact probe and compare every original output row and process completion.
