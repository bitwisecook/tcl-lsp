# naming.variable-table.array-unset-observer-order

Kind: `native-observation`

## Problem statement

Array deletion invokes unset observers while retiring physical entries. A sorted traversal or reconstructed source order can differ from the actual native callback sequence.

## Question

What exact unset callback key sequence follows deletion of the13-entry ordinary array?

## Conclusion

All five C captures record callback order k05 k06 k07 k08 k09 k10 k11 k12 k00 k01 k02 k03 k04 with normal completion. Jim stops at its unsupported trace command, so no Jim array teardown callback sequence is observed. This is an entered script callback result; it does not measure individual header free timing.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-teardown","op":"teardown","code":0,"bytes":"6b3035206b3036206b3037206b3038206b3039206b3130206b3131206b3132206b3030206b3031206b3032206b3033206b3034"}
```

### jim

Status: `unsupported`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: jim Jim capture.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"array-teardown","op":"teardown","code":1,"bytes":"696e76616c696420636f6d6d616e64206e616d652022747261636522"}
```

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-registry/src/native_variable_table.rs](../../../../rust/tcl-registry/src/native_variable_table.rs), `native_variable_table_protocol`: Selects table/name semantics only with an independently supplied actual ABI.
- [rust/tcl-core-types/src/native_hash_order.rs](../../../../rust/tcl-core-types/src/native_hash_order.rs), `NativeEntryLedger`: Keeps original entry chronology and optional selected hash order separately from defined inventory.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `teardown_names`: Uses current physical variable/array entry order for native entered unset observers.

## Evidence

- [rust/tcl-core-types/tests/data/native_variable_tables/probe.c](../../../../rust/tcl-core-types/tests/data/native_variable_tables/probe.c)
- [rust/tcl-core-types/tests/data/native_variable_tables/manifest.json](../../../../rust/tcl-core-types/tests/data/native_variable_tables/manifest.json)
- [rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.4.20.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.5.19.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/8.6.18.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.0.4.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/9.1.0.jsonl)
- [rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl](../../../../rust/tcl-core-types/tests/data/native_variable_tables/Jim.jsonl)
