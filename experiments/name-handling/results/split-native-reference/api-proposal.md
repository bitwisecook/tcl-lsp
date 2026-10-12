# Current owner reuse proposal (unimplemented)

1. `rust/tcl-syntax/src/native_tcl_utf.rs` owns canonical NativeTclUtf::decode_unit/decode_unit_with and TclUtfUnit { value, width }.
They retain original extent, previous C86 unit state, modified NUL, surrogate units and invalid-byte native fallback without Rust UTF8.
Existing tests: encoding_matches_every_selected_native_unit_control, reader_extent_keeps_nul_units_and_native_short_circuit_accesses, canonical_native_units_encoding_and_previous_boundaries_match_five_releases.
Reuse this decoder for the general split branch; do not write another Tcl UTF decoder in list.rs or the adapter.
C86 empty-set split specifically uses TclUtfToUCS4; if its combining purpose needs a public helper, expose it in this same decoder owner.
The existing private native_string_trim::unit composes decode_unit for that C86 purpose; duplicating the composition in each command is unnecessary.
C84/85 byte-unit, C86 surrogate-unit and C9 scalar behavior remain separate selected algorithms. A cache getter is not an equivalent traversal receipt.

2. The generic physical-value boundary is `ValueOps` (tcl-syntax/value.rs), with its actual runtime adapter in runtime/value_ops.rs.
Current native_string_bytes reaches original storage; name_policy_protocol().string_protocol() exposes independently selected native issuer; new_bytes retains exact output bytes.
The protocol accessor cannot grant an unselected adapter C86 behavior, and grammar/profile compatibility cannot replace actual physical admission.
NativeIndexOps already has enter_index_operation/check_index_operation and run_index_result cleanup; reuse its currency pattern without claiming index-specific storage/cache purposes for split.
No new String Unicode cache should be installed merely to traverse split's original resident bytes.
Result planning must retain fresh/borrowed child ownership and cleanup under the original operation. C empty-set character reuse requires actual shared child handles, not equal strings alone.
If existing generic constructors do not attest split's native String/List result and retirement purposes, add the minimal typed capability at this boundary and explicitly refuse selected unavailable adapters.
Current Native physical child identity and result caches remain unanswered by script-only observations.

3. `rust/tcl-cmd-core/src/list.rs:700` owns split admission/branching and retains the portable checked-Unicode path for its independently authored text model.
Current split_splits_as_tclsh_splits exercises TextOps ASCII/Unicode values; it does not cover genuine native encoded NUL or object identity.
A later selected native path should request delimiter storage before input, return empty input before decoding the delimiter, and preserve single-byte CString stopping, general counted-unit matching and empty-set reuse separately.
Use original byte-slice results from a central plan. A lossy decoder, global RawString UTF8 relaxation or universal scalar loop would erase measured branches.
Required acceptance: these 18 vectors with raw C elements, actual Native object/cache/lifetime controls and negative unavailable-capability/refusal cases; no unmeasured profile or F5 admission claim.
This is a preparatory reference study, not an implementation or a Rust test result.
