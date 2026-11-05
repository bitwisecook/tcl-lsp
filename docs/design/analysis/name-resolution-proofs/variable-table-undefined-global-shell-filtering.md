# naming.variable-table.undefined-global-shell-filtering

Kind: `native-observation`

## Problem statement

Scalar trace shells coexist with the defined foreach key variable. Counting all physical roots or filtering by trace presence can expose undefined names or hide a subsequently defined shell.

## Question

Which global/info vars names are visible before and after filling k03 and removing the k04 trace?

## Conclusion

Every C capture exposes key and k00/k01/k02 initially, then adds k03 after filling it. Removing the undefined k04 trace changes no defined inventory. Its physical table includes undefined trace entries and16buckets. Jim rejects trace setup/removal; its ordinary key/value inventories have the same visible progression but do not prove a trace-shell mechanism.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.4 8.4.20; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variable-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"variable-undefined-trace-shell","op":"physical-table","global_buckets":16,"global_entries":20,"array_buckets":-1,"array_entries":-1}
{"case":"variable-undefined-trace-shell","op":"info-globals","code":0,"bytes":"6b6579206b3030206b3031206b3032"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032"}
{"case":"variable-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032206b3033"}
{"case":"variable-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032206b3033"}
```

### tcl8.5, tcl8.6

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variable-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"variable-undefined-trace-shell","op":"physical-table","global_buckets":16,"global_entries":21,"array_buckets":-1,"array_entries":-1}
{"case":"variable-undefined-trace-shell","op":"info-globals","code":0,"bytes":"6b6579206b3030206b3031206b3032"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032"}
{"case":"variable-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032206b3033"}
{"case":"variable-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032206b3033"}
```

### jim

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: jim Jim capture.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variable-undefined-trace-shell","op":"setup","code":1,"bytes":"696e76616c696420636f6d6d616e64206e616d652022747261636522"}
{"case":"variable-undefined-trace-shell","op":"physical-table","global_buckets":16,"global_entries":11,"global_hash_uniq":0}
{"case":"variable-undefined-trace-shell","op":"info-globals","code":0,"bytes":"6b6579206b3030206b3031206b3032"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032"}
{"case":"variable-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032206b3033"}
{"case":"variable-undefined-trace-shell","op":"remove-shell","code":1,"bytes":"696e76616c696420636f6d6d616e64206e616d652022747261636522"}
{"case":"variable-undefined-trace-shell","op":"info-vars","code":0,"bytes":"6b6579206b3030206b3031206b3032206b3033"}
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
