# naming.variable-table.original-opaque-key-extents

Kind: `native-observation`

## Problem statement

Raw-zero, FF and modified-zero original keys enter array set as genuine List members. The stored key extent must come from the selected native matcher rather than decoding their display.

## Question

Which original opaque array key bytes are returned before and after the three-key k03 reinsertion control?

## Conclusion

C8.4 returns raw-zero key k while C8.5–9.1 and Jim retain k00x (hex6b0078). Every provider preserves the separate FF and modified-zero key bytes. C preserves the attached hash order; Jim returns original insertion order then appended k03. C names/get/search succeed, while Jim startsearch rejects. This observes counted original key results, not Unicode document ingress or an arbitrary table-key identity.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.4 8.4.20.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6bff206bc080206b"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6bff2056206bc0802056206b2056"}
{"case":"array-3-opaque1","op":"array-search","code":0,"bytes":"6bff206bc080206b"}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":16,"global_entries":12,"array_buckets":4,"array_entries":4}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6b3033206bff206bc080206b"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6b30332056206bff2056206bc0802056206b2056"}
{"case":"array-3-opaque1","op":"array-search","code":0,"bytes":"6b3033206bff206bc080206b"}
```

### tcl8.5, tcl8.6

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6bff206bc080206b0078"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6bff2056206bc0802056206b00782056"}
{"case":"array-3-opaque1","op":"array-search","code":0,"bytes":"6bff206bc080206b0078"}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":4,"array_entries":4}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6b3033206bff206bc080206b0078"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6b30332056206bff2056206bc0802056206b00782056"}
{"case":"array-3-opaque1","op":"array-search","code":0,"bytes":"6b3033206bff206bc080206b0078"}
```

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":4,"array_entries":3}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6bff206bc080206b0078"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6bff2056206bc0802056206b00782056"}
{"case":"array-3-opaque1","op":"array-search","code":0,"bytes":"6bff206bc080206b0078"}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":4,"array_entries":4}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6b3033206bff206bc080206b0078"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6b30332056206bff2056206bc0802056206b00782056"}
{"case":"array-3-opaque1","op":"array-search","code":0,"bytes":"6b3033206bff206bc080206b0078"}
```

### jim

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: jim Jim capture.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-3-opaque1","op":"physical-table","global_buckets":16,"global_entries":7,"global_hash_uniq":0}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6b0078206bff206bc080"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6b00782056206bff2056206bc0802056"}
{"case":"array-3-opaque1","op":"array-search","code":1,"bytes":"61727261792c20756e6b6e6f776e20636f6d6d616e6420227374617274736561726368223a2073686f756c64206265206578697374732c206765742c206e616d65732c207365742c2073697a652c20737461742c20756e736574"}
{"case":"array-3-opaque1","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-3-opaque1","op":"array-names","code":0,"bytes":"6b0078206bff206bc080206b3033"}
{"case":"array-3-opaque1","op":"array-get","code":0,"bytes":"6b00782056206bff2056206bc0802056206b30332056"}
{"case":"array-3-opaque1","op":"array-search","code":1,"bytes":"61727261792c20756e6b6e6f776e20636f6d6d616e6420227374617274736561726368223a2073686f756c64206265206578697374732c206765742c206e616d65732c207365742c2073697a652c20737461742c20756e736574"}
```

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `native_variable_table_protocol`: Selects table/name semantics only with an independently supplied actual ABI.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `NativeEntryLedger`: Keeps original entry chronology and optional selected hash order separately from defined inventory.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `native_hash_order::tests::native_c_array_order_retains_original_keys_growth_and_reinsertion` (linked): Checks all320 exact C array-order controls against independently selected arithmetic; it does not test Jim search, global metadata or every attached callback case.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `frame::native_inventory_tests::array_entry_ledgers_match_all_320_native_table_controls` (linked): Compares current element ledgers to320 original C order controls using selected native key extent and hash recipe; real native launches are not performed by this test.

## Evidence

- [rust/tcl-core-types/tests/data/native_variable_tables](../../../../rust/tcl-core-types/tests/data/native_variable_tables)
