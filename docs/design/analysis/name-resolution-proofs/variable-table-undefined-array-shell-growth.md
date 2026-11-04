# naming.variable-table.undefined-array-shell-growth

Kind: `native-observation`

## Problem statement

Read traces can create undefined array entries that grow the physical table but should not appear as defined values. Removing one shell can retire an entry without shrinking the retained buckets.

## Question

What table counts and defined inventories follow nine undefined traced array shells, filling k03 and removing the k04 trace?

## Conclusion

All C releases begin with12physical array entries in16buckets but expose only k00/k01/k02. Filling k03 preserves12entries/buckets and exposes k03. Removing the undefined k04 trace retires its entry to11 while16buckets and the four defined values remain. Jim rejects trace setup/removal and search; its later ordinary names/get follow three then four defined elements, supplying no traced-shell lifecycle proof.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":11}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
```

### tcl8.5

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":11}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
```

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":9,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":11,"array_buckets":16,"array_entries":11}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
```

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":11}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-undefined-trace-shell","op":"setup","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":8,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":12}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"remove-shell","code":0,"bytes":""}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":4,"global_entries":10,"array_buckets":16,"array_entries":11}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
```

### jim

Status: `observed`. Version: Jim capture; patchlevel, revision and UTF configuration unrecorded. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: Jim Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-undefined-trace-shell","op":"setup","code":1,"bytes":"696e76616c696420636f6d6d616e64206e616d652022747261636522"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":16,"global_entries":8,"global_hash_uniq":0}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":1,"bytes":"61727261792c20756e6b6e6f776e20636f6d6d616e6420227374617274736561726368223a2073686f756c64206265206578697374732c206765742c206e616d65732c207365742c2073697a652c20737461742c20756e736574"}
{"case":"array-undefined-trace-shell","op":"fill-shell","code":0,"bytes":"56"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":16,"global_entries":9,"global_hash_uniq":0}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":1,"bytes":"61727261792c20756e6b6e6f776e20636f6d6d616e6420227374617274736561726368223a2073686f756c64206265206578697374732c206765742c206e616d65732c207365742c2073697a652c20737461742c20756e736574"}
{"case":"array-undefined-trace-shell","op":"remove-shell","code":1,"bytes":"696e76616c696420636f6d6d616e64206e616d652022747261636522"}
{"case":"array-undefined-trace-shell","op":"physical-table","global_buckets":16,"global_entries":9,"global_hash_uniq":0}
{"case":"array-undefined-trace-shell","op":"array-names","code":0,"bytes":"6b3030206b3031206b3032206b3033"}
{"case":"array-undefined-trace-shell","op":"array-get","code":0,"bytes":"6b30302056206b30312056206b30322056206b30332056"}
{"case":"array-undefined-trace-shell","op":"array-search","code":1,"bytes":"61727261792c20756e6b6e6f776e20636f6d6d616e6420227374617274736561726368223a2073686f756c64206265206578697374732c206765742c206e616d65732c207365742c2073697a652c20737461742c20756e736574"}
```

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Historical references

- `probe` (input): [rust/tcl-core-types/tests/data/native_variable_tables/probe.c](../../../../rust/tcl-core-types/tests/data/native_variable_tables/probe.c). Exact original keys, constructor/evaluator flags, native metadata and independent script observer cases.
- `receipt` (provider): [rust/tcl-core-types/tests/data/native_variable_tables/manifest.json](../../../../rust/tcl-core-types/tests/data/native_variable_tables/manifest.json). Six original source/header/library/compile/process/raw output closures; absent additional artifacts remain explicit.
- `rows-tcl8.4` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl). Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl8.5` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl). Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl8.6` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl). Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl9.0` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl). Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-tcl9.1` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl). Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.
- `rows-jim` (observation): [rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl). Full original298-row JSONL stream; relevant case/op selectors are listed in the answer.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `native_variable_table_protocol`: Selects table/name semantics only with an independently supplied actual ABI.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `NativeEntryLedger`: Keeps original entry chronology and optional selected hash order separately from defined inventory.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `frame::native_inventory_tests::trace_shells_grow_table_without_becoming_defined_inventory` (linked): Recreates C shell growth to16buckets, undefined filtering, retirement to11 and fill order. No native trace invocation or Jim parity is asserted by this model test.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
