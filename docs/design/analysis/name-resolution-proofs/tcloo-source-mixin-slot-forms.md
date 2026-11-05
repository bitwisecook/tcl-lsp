# naming.tcloo.source-mixin-slot-forms

Kind: `native-observation`

## Problem statement

A mixin getter and empty update can print the same empty list from a fresh class, while bare or explicit -set B install an actual provider. The form must remain separate even when current output is equal.

## Question

What current mixin list follows no operands, bare B, -set B, -set with no values and -clear?

## Conclusion

C8.6/9.0/9.1 report empty for the no-operand getter, ::B for bare B and -set B, and empty for -set-empty and -clear. Each control begins with a fresh A, so equal empty outputs do not alone prove getter/update identity. C8.4/8.5/Jim reject oo::class before the worker. No method dispatch, private visibility or current Source provider join follows from these list queries.

## Scope

Selected cases mixin-getter,mixin-one,mixin-set-one,mixin-set-zero,mixin-clear from25 actual source programs per provider,150 total captured completions. Every case uses a new interpreter. Public Tcl_Eval or Jim_Eval consumes NUL-terminated ASCII source; binary-format/source-escape values are produced at runtime and are not raw zero source or independently held original native String objects. Result bytes are sampled after the guest completion through the actual public string getter. Source/header/library/executable and separate stdout/stderr hashes, compile/process0 and actual original command arrays are retained. C release associations and Jim git-describe tag are recorded, but this probe does not query a linked runtime patchlevel or object primary/refcount. BIG-IP is not tested; no Rust execution is inferred.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: tcl8.4 8.4.20 (recorded build association; tcl8.5 8.5.19 (recorded build association; jim 0.84-9-g5bac7c9 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
mixin-getter	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-one	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-set-one	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-set-zero	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
mixin-clear	1	32	696e76616c696420636f6d6d616e64206e616d6520226f6f3a3a636c61737322
```

Compile status0 and whole process0 are separate from the printed guest completions. oo::class is unavailable, so the targeted slot worker and zero operand are unreached.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Tcl_Eval/Jim_Eval NUL-terminated ASCII source; public result string getter. Version: tcl8.6 8.6.18 (recorded build association; tcl9.0 9.0.4 (recorded build association; tcl9.1 9.1.0 (recorded build association.

Exact selected rows in case/guest-code/result-byte-count/result-hex order:

```text
mixin-getter	0	0	
mixin-one	0	3	3a3a42
mixin-set-one	0	3	3a3a42
mixin-set-zero	0	0	
mixin-clear	0	0	
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
