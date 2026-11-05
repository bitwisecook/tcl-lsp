# naming.variable-table.dynamic-global-and-local-order

Kind: `native-observation`

## Problem statement

Dynamically setting names through an original List in a procedure can enumerate differently from the global table with the same printed names. Compiled local declaration order is another purpose and cannot supply this dynamic inventory.

## Question

Do global info globals/vars and procedure info locals/vars return the same order after15 original ordinary/opaque stores?

## Conclusion

Every provider returns matching global info globals versus info vars results, and matching procedure info locals versus info vars results. C global and dynamic procedure order differ for the selected keys; Jim global/procedure results match in this capture. C8.4 truncates the raw-zero key to k, later C and Jim retain its counted bytes. These returned values and table counts do not attest compile-local declaration order or cell allocation.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.4 8.4.20.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"physical-table","global_buckets":16,"global_entries":22,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"info-globals","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b3034"}
{"case":"variables-opaque0","op":"info-vars","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b3034"}
{"case":"variables-opaque0","op":"proc-define","code":0,"bytes":""}
{"case":"variables-opaque0","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b30347d207b6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b30347d"}
{"case":"variables-opaque1","op":"physical-table","global_buckets":16,"global_entries":22,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"info-globals","code":0,"bytes":"6b3035206b3036206b3037206bff206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b206bc080206b3033206b3034"}
{"case":"variables-opaque1","op":"info-vars","code":0,"bytes":"6b3035206b3036206b3037206bff206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b206bc080206b3033206b3034"}
{"case":"variables-opaque1","op":"proc-define","code":0,"bytes":""}
{"case":"variables-opaque1","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3036206bff206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b206bc080206b3033206b30347d207b6b3035206b3036206bff206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b206bc080206b3033206b30347d"}
```

### tcl8.5, tcl8.6

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"physical-table","global_buckets":16,"global_entries":23,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"info-globals","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b3034"}
{"case":"variables-opaque0","op":"info-vars","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b3034"}
{"case":"variables-opaque0","op":"proc-define","code":0,"bytes":""}
{"case":"variables-opaque0","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b30347d207b6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b30347d"}
{"case":"variables-opaque1","op":"physical-table","global_buckets":16,"global_entries":23,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"info-globals","code":0,"bytes":"6b3035206b3036206b3037206bff206b3038206b0078206b3130206b3039206b3131206b3132206b3133206b3134206bc080206b3033206b3034"}
{"case":"variables-opaque1","op":"info-vars","code":0,"bytes":"6b3035206b3036206b3037206bff206b3038206b0078206b3130206b3039206b3131206b3132206b3133206b3134206bc080206b3033206b3034"}
{"case":"variables-opaque1","op":"proc-define","code":0,"bytes":""}
{"case":"variables-opaque1","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3036206bff206b3037206b0078206b3038206b3039206b3130206b3131206b3132206b3133206b3134206bc080206b3033206b30347d207b6b3035206b3036206bff206b3037206b0078206b3038206b3039206b3130206b3131206b3132206b3133206b3134206bc080206b3033206b30347d"}
```

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"physical-table","global_buckets":16,"global_entries":22,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"info-globals","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b3034"}
{"case":"variables-opaque0","op":"info-vars","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3130206b3039206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b3034"}
{"case":"variables-opaque0","op":"proc-define","code":0,"bytes":""}
{"case":"variables-opaque0","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b30347d207b6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3133206b3134206b3030206b3031206b3032206b3033206b30347d"}
{"case":"variables-opaque1","op":"physical-table","global_buckets":16,"global_entries":22,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"info-globals","code":0,"bytes":"6b3035206b3036206b3037206bff206b3038206b0078206b3130206b3039206b3131206b3132206b3133206b3134206bc080206b3033206b3034"}
{"case":"variables-opaque1","op":"info-vars","code":0,"bytes":"6b3035206b3036206b3037206bff206b3038206b0078206b3130206b3039206b3131206b3132206b3133206b3134206bc080206b3033206b3034"}
{"case":"variables-opaque1","op":"proc-define","code":0,"bytes":""}
{"case":"variables-opaque1","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3036206bff206b3037206b0078206b3038206b3039206b3130206b3131206b3132206b3133206b3134206bc080206b3033206b30347d207b6b3035206b3036206bff206b3037206b0078206b3038206b3039206b3130206b3131206b3132206b3133206b3134206bc080206b3033206b30347d"}
```

### jim

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: jim Jim capture.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"physical-table","global_buckets":32,"global_entries":21,"global_hash_uniq":0}
{"case":"variables-opaque0","op":"info-globals","code":0,"bytes":"6b3035206b3037206b3039206b3131206b3133206b3030206b3032206b3034206b3036206b3038206b3130206b3132206b3134206b3031206b3033"}
{"case":"variables-opaque0","op":"info-vars","code":0,"bytes":"6b3035206b3037206b3039206b3131206b3133206b3030206b3032206b3034206b3036206b3038206b3130206b3132206b3134206b3031206b3033"}
{"case":"variables-opaque0","op":"proc-define","code":0,"bytes":"70"}
{"case":"variables-opaque0","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206b3037206b3039206b3131206b3133206b3030206b3032206b3034206b3036206b3038206b3130206b3132206b3134206b3031206b30337d207b6b3035206b3037206b3039206b3131206b3133206b3030206b3032206b3034206b3036206b3038206b3130206b3132206b3134206b3031206b30337d"}
{"case":"variables-opaque1","op":"physical-table","global_buckets":32,"global_entries":21,"global_hash_uniq":0}
{"case":"variables-opaque1","op":"info-globals","code":0,"bytes":"6b3035206bff206b3037206b0078206b3039206b3131206b3133206bc080206b3034206b3036206b3038206b3130206b3132206b3134206b3033"}
{"case":"variables-opaque1","op":"info-vars","code":0,"bytes":"6b3035206bff206b3037206b0078206b3039206b3131206b3133206bc080206b3034206b3036206b3038206b3130206b3132206b3134206b3033"}
{"case":"variables-opaque1","op":"proc-define","code":0,"bytes":"70"}
{"case":"variables-opaque1","op":"proc-local-inventory","code":0,"bytes":"7b6b3035206bff206b3037206b0078206b3039206b3131206b3133206bc080206b3034206b3036206b3038206b3130206b3132206b3134206b30337d207b6b3035206bff206b3037206b0078206b3039206b3131206b3133206bc080206b3034206b3036206b3038206b3130206b3132206b3134206b30337d"}
```

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `native_variable_table_protocol`: Selects table/name semantics only with an independently supplied actual ABI.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `NativeEntryLedger`: Keeps original entry chronology and optional selected hash order separately from defined inventory.

## Evidence

- [rust/tcl-core-types/tests/data/native_variable_tables/probe.c](../../../../rust/tcl-core-types/tests/data/native_variable_tables/probe.c)
- [rust/tcl-core-types/tests/data/native_variable_tables/manifest.json](../../../../rust/tcl-core-types/tests/data/native_variable_tables/manifest.json)
- [rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl)
