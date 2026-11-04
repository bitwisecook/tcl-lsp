# naming.variable-table.bootstrap-physical-versus-visible-keys

Kind: `native-observation`

## Problem statement

A newly created interpreter can have undefined physical entries omitted by info globals. Treating every table entry as a defined variable confuses metadata storage with returned inventory.

## Question

Which physical bootstrap global keys are omitted from the initial info globals result of this exact constructor?

## Conclusion

C8.4 has7physical bootstrap entries and omits tcl_precision from its6visible initial names. C8.5/8.6 have8physical and5visible, omitting tcl_precision/errorInfo/errorCode. C9 has7physical and5visible, omitting errorInfo/errorCode. Jim has6physical and6visible for this core/static-extension constructor. Exact root names and enumeration order are attached; these are this constructor's inventory results, not arbitrary bootstrap allocation/header proof.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"initial-physical-table","global_buckets":4,"global_entries":7,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","74636c44656661756c744c696272617279","656e76","74636c5f707265636973696f6e","74636c5f70617463684c6576656c","74636c5f706c6174666f726d"]}
{"case":"variables-opaque0","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b67506174682074636c44656661756c744c69627261727920656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
{"case":"variables-opaque1","op":"initial-physical-table","global_buckets":4,"global_entries":7,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","74636c44656661756c744c696272617279","656e76","74636c5f707265636973696f6e","74636c5f70617463684c6576656c","74636c5f706c6174666f726d"]}
{"case":"variables-opaque1","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b67506174682074636c44656661756c744c69627261727920656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
```

### tcl8.5

Status: `observed`. Version: 8.5.19; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"initial-physical-table","global_buckets":4,"global_entries":8,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f707265636973696f6e","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque0","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
{"case":"variables-opaque1","op":"initial-physical-table","global_buckets":4,"global_entries":8,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f707265636973696f6e","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque1","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
```

### tcl8.6

Status: `observed`. Version: 8.6.18; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"initial-physical-table","global_buckets":4,"global_entries":8,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f707265636973696f6e","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque0","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
{"case":"variables-opaque1","op":"initial-physical-table","global_buckets":4,"global_entries":8,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f707265636973696f6e","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque1","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
```

### tcl9.0

Status: `observed`. Version: 9.0.4; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"initial-physical-table","global_buckets":4,"global_entries":7,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque0","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
{"case":"variables-opaque1","op":"initial-physical-table","global_buckets":4,"global_entries":7,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque1","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
```

### tcl9.1

Status: `observed`. Version: 9.1.0; exact launched patchlevel not queried by this probe. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: C Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"initial-physical-table","global_buckets":4,"global_entries":7,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque0","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque0","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
{"case":"variables-opaque1","op":"initial-physical-table","global_buckets":4,"global_entries":7,"array_buckets":-1,"array_entries":-1}
{"case":"variables-opaque1","op":"physical-global-keys","keys":["74636c5f76657273696f6e","74636c5f706b6750617468","656e76","6572726f72436f6465","74636c5f70617463684c6576656c","6572726f72496e666f","74636c5f706c6174666f726d"]}
{"case":"variables-opaque1","op":"initial-info-globals","code":0,"bytes":"74636c5f76657273696f6e2074636c5f706b675061746820656e762074636c5f70617463684c6576656c2074636c5f706c6174666f726d"}
```

### jim

Status: `observed`. Version: Jim capture; patchlevel, revision and UTF configuration unrecorded. Build: compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer.. Dialect: Jim Tcl.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"variables-opaque0","op":"initial-physical-table","global_buckets":16,"global_entries":6,"global_hash_uniq":0}
{"case":"variables-opaque0","op":"physical-global-keys","keys":["74636c5f696e746572616374697665","62696e6172793a3a636f6e7674797065","62696e6172793a3a7363616c6172696e666f","6175746f5f70617468","74636c5f706c6174666f726d","656e76"]}
{"case":"variables-opaque0","op":"initial-info-globals","code":0,"bytes":"74636c5f696e7465726163746976652062696e6172793a3a636f6e76747970652062696e6172793a3a7363616c6172696e666f206175746f5f706174682074636c5f706c6174666f726d20656e76"}
{"case":"variables-opaque1","op":"initial-physical-table","global_buckets":16,"global_entries":6,"global_hash_uniq":0}
{"case":"variables-opaque1","op":"physical-global-keys","keys":["74636c5f696e746572616374697665","62696e6172793a3a636f6e7674797065","62696e6172793a3a7363616c6172696e666f","6175746f5f70617468","74636c5f706c6174666f726d","656e76"]}
{"case":"variables-opaque1","op":"initial-info-globals","code":0,"bytes":"74636c5f696e7465726163746976652062696e6172793a3a636f6e76747970652062696e6172793a3a7363616c6172696e666f206175746f5f706174682074636c5f706c6174666f726d20656e76"}
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

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
