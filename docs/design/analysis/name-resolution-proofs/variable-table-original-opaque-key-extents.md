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

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: header_sha256=824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

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

### tcl8.5

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: header_sha256=c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

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

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: header_sha256=aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: header_sha256=eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

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

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: header_sha256=30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

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

Status: `observed`. Version: Jim capture; patchlevel, revision and UTF configuration unrecorded. Build: header_sha256=d9b020a910ba79cd90d2194849ad56b691fecb5dafd2b3f01c3d4d94e04c369d; library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: Jim Tcl.

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

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-core-types/tests/data/native_variable_tables/probe.c](../../../../rust/tcl-core-types/tests/data/native_variable_tables/probe.c). SHA-256 `515d491323ffb7be74530b7736484608cd6de17060240ff87eb31eedd9335846`. Exact original keys, constructor/evaluator flags, native metadata and independent script observer cases.
- `receipt` (provider): [rust/tcl-core-types/tests/data/native_variable_tables/manifest.json](../../../../rust/tcl-core-types/tests/data/native_variable_tables/manifest.json). SHA-256 `ef2b629ba28eeae46b8823ea8e02bdb7b32b28d2326a004b8cad350d53425a72`. Six original source/header/library/compile/process/raw output closures; absent additional artifacts remain explicit.
- `array-order` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/array-order.tsv](../../../../rust/tcl-core-types/tests/data/native_variable_tables/array-order.tsv). SHA-256 `d305208efa6f325bb4ec9929c877db77ca1b497ef8a67ca5d28ba44372e9fdf9`. 320 C array-names controls independently byte-matched to exact64 original JSONL array-names rows for each release.
- `rows-tcl8.4` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl). SHA-256 `f2e9bf0f841206ee09d7b6d2fa0879fe4ff6c9035720bbc89df808f271b5a043`. Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl8.5` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl). SHA-256 `0b2a033f8057e4398eebee5374bf7530e1c33c5bf0a6c80f12773b05107f6d6f`. Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl8.6` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl). SHA-256 `f0aab00c89ff26c20e7a9c7c16eb8fcbcb2dc2fd5c02ee6175ff58eee73a410c`. Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl9.0` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl). SHA-256 `950174873e3c020bbb6678850238ef83f4e37d2e3e1327c00499aa94a989b039`. Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl9.1` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl). SHA-256 `950174873e3c020bbb6678850238ef83f4e37d2e3e1327c00499aa94a989b039`. Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-jim` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl). SHA-256 `59b46068237c18aaed0fd8e81ba0706291089a0ac4057f183208b721494df451`. Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `native_variable_table_protocol`: Selects table/name semantics only with an independently supplied actual ABI.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `NativeEntryLedger`: Keeps original entry chronology and optional selected hash order separately from defined inventory.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `native_hash_order::tests::native_c_array_order_retains_original_keys_growth_and_reinsertion` (linked): Checks all320 exact C array-order controls against independently selected arithmetic; it does not test Jim search, global metadata or every attached callback case.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `frame::native_inventory_tests::array_entry_ledgers_match_all_320_native_table_controls` (linked): Compares current element ledgers to320 original C order controls using selected native key extent and hash recipe; real native launches are not performed by this test.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
