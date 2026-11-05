# naming.variable-table.actual-abi-inputs

Kind: `native-observation`

## Problem statement

Native hash arithmetic width and plain-char promotion are actual build facts, independently of a release label. A matching key order cannot fabricate ABI facts or a Jim seed.

## Question

Which actual plain-char/unsigned-int/size_t facts does each original variable table observer print?

## Conclusion

Every retained original provider prints plain_char_signed1, unsigned_int_bits32 and size_t_bits64. Jim separately prints hash uniq0 in the actual table metadata. These finite build observations are independent from selected hash promotion semantics, runtime table identity, or another compiler configuration.

## Scope

One exact native probe uses fresh interpreters per case, counted original String/List object-vector array set and public script-object evaluation for query/trace procedures. Its opaque inputs are rawzero/FF/modifiedzero byte strings, not document source literals. Results are retained before rendering; no error-global observer. C private fields measure real global/array tables; Jim table metadata is its global vars hash only. All1788 original JSONL rows are retained and whole-stream/source/header/library hashes checked. Launch patchlevel/Jim revision/configuration, compiler version and executable hashes are not recorded. Missing additional-artifact hashes in the manifest do not reconstruct absent artifacts; no runtime comparison or Rust execution claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Original counted public object-vector variable/array constructors; original String script objects enter Tcl_EvalObjEx(TCL_EVAL_GLOBAL) or Jim_EvalObj; private table/ABI observer. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim Jim capture.

Exact selected original JSONL rows in reached order. Hex bytes are retained without display decoding; any negative guest code remains a measured failure.

```jsonl
{"case":"actual-native-abi","op":"abi","plain_char_signed":1,"unsigned_int_bits":32,"size_t_bits":64}
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
