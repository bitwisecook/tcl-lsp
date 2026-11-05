# naming.tcloo.source-generated-zero-option-classification

Kind: `native-observation`

## Problem statement

A runtime-generated zero operand can be classified as an object name or a leading-dash method selector. Reusing a raw native String-zero truncation rule would wrongly accept a flag produced by binary format or a source escape.

## Question

How do superclass/mixin classify binary-format zero, zeroSet, -zero-set and written -s\u0000et operands?

## Conclusion

C8.6/9.0/9.1 reject zero and zeroSet as object names that do not refer to an object, and reject the leading-dash zero forms as unknown methods. The returned diagnostic units contain C080 for zero. C8.6 lists -append/-clear/-set; C9 additionally lists -appendifnew/-prepend/-remove. C8.4/8.5/Jim reject oo::class before those operands are reached. All inputs are ASCII NUL-terminated source; binary format creates runtime values that can materialize modified zero. Neither original rawString00 identity nor CString matching is measured.

## Scope

Selected cases superclass-zero,superclass-zero-suffix,superclass-dash-zero,superclass-escaped-zero,mixin-zero,mixin-zero-suffix,mixin-dash-zero,mixin-escaped-zero from25 actual source programs per provider,150 total captured completions. Every case uses a new interpreter. Public Tcl_Eval or Jim_Eval consumes NUL-terminated ASCII source; binary-format/source-escape values are produced at runtime and are not raw zero source or independently held original native String objects. Result bytes are sampled after the guest completion through the actual public string getter. Source/header/library/executable and separate stdout/stderr hashes, compile/process0 and actual original command arrays are retained. C release associations and Jim git-describe tag are recorded, but this probe does not query a linked runtime patchlevel or object primary/refcount. BIG-IP is not tested; no Rust execution is inferred.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: tcl8.4 8.4.20 (recorded build association; tcl8.5 8.5.19 (recorded build association; jim 0.84-9-g5bac7c9 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
superclass-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-zero-suffix	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-dash-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-escaped-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

Compile status0 and whole process0 are separate from the printed guest completions. oo::class is unavailable, so the targeted slot worker and zero operand are unreached.

### tcl8.6

Status: `observed`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: tcl8.6 8.6.18 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-dash-zero	1	56	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
superclass-escaped-zero	1	56	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
mixin-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-dash-zero	1	56	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
mixin-escaped-zero	1	56	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d636c656172206f72202d736574
```

Compile status0 and whole process0 are separate from the printed guest completions.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: tcl9.0 9.0.4 (recorded build association; tcl9.1 9.1.0 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
superclass-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
superclass-dash-zero	1	89	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
superclass-escaped-zero	1	89	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
mixin-zero	1	30	c08020646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-zero-suffix	1	33	c08053657420646f6573206e6f7420726566657220746f20616e206f626a656374
mixin-dash-zero	1	89	756e6b6e6f776e206d6574686f6420222dc080736574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
mixin-escaped-zero	1	89	756e6b6e6f776e206d6574686f6420222d73c0806574223a206d757374206265202d617070656e642c202d617070656e6469666e65772c202d636c6561722c202d70726570656e642c202d72656d6f7665206f72202d736574
```

Compile status0 and whole process0 are separate from the printed guest completions.

### bigip

Status: `not-tested`.

No BIG-IP load/event or appliance-Tcl result for these exact programs is attached.

## Consumer bindings

- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `SlotSpec::split_original_call`: Selects original query versus update grammar and exact slot operation without issuing target/effect or successful native dispatch.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `SlotOp::resolve_runtime`: Matches the release-selected original operation bytes, preserving materialized-zero rejection separately from a raw String-zero byte purpose.
- [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs), `definer::tests::original_tcloo_slot_calls_distinguish_queries_empty_updates_and_materialised_zero` (linked): Pure selected slot grammar distinguishes no-operand query from bare/explicit empty updates and refuses materialized C080 method flags on C86/90/91, with unavailable C84/85/Jim source admission. It does not invoke these native providers.

## Evidence

- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls)
- [scripts/dev](../../../../scripts/dev)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.4.20)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.5.19)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/8.6.18)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.0.4)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/9.1.0)
- [rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim](../../../../rust/tcl-registry/tests/data/native_array_and_oo_source_controls/jim)
