# naming.compiled-local-inventory.array-shaped-formal-validation

Kind: `native-observation`

## Problem statement

An array-like formal may be rejected during declaration or installed using a different variable receiver. Syntax validation, CString presentation and actual local enumeration are different purposes.

## Question

Which ordinary/after-zero array-shaped original formals install and what local inventory follows?

## Conclusion

All C releases reject ordinary a(q) at definition. C8.4/8.5 accept array syntax after raw zero, enumerate k for one formal and reject the two-argument call with usage p k; C8.6–9.1 reject the after-zero array form at definition. Jim accepts both forms, enumerates full counted k-zero-a(q) members for the after-zero case and a for ordinary a(q)/a(r). Failed C definitions are followed by invalid-command p, not by a body execution.

## Scope

Original counted String formal members and an original List parameter object enter public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. The body is exactly set extra EXTRA;info exists absent;list [info locals] [info vars]. One/two-formal axes and definition/invocation outcomes are captured before later error observers; the source retains result/return-options and uses the C8.4 private error-storage observer. originalA/originalB callbacks are registered but not called by this body. The manifest observation_window boilerplate mentions braced/dynamic substitution; this exact probe has only the stated enumeration body. Forty original rows per provider, full output/source/library/header/executable hashes and compile/run status are retained; Jim patchlevel/revision/configure flags and all launched C patchlevels are unqueried. BIG-IP is not tested; manifest vm_comparisons is0.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried).

Exact reached rows: opaque-array-after-nul/one-original-formal/original-definition: code0 result_hex=; opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b2065787472617d207b6b2065787472617d; opaque-array-ordinary/one-original-formal/original-definition: code1 result_hex=70726f636564757265202270222068617320666f726d616c20706172616d6574657220226128712922207468617420697320616e20617272617920656c656d656e74; opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022; opaque-array-after-nul/two-original-formals/original-definition: code0 result_hex=; opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=77726f6e67202320617267733a2073686f756c64206265202270206b22; opaque-array-ordinary/two-original-formals/original-definition: code1 result_hex=70726f636564757265202270222068617320666f726d616c20706172616d6574657220226128712922207468617420697320616e20617272617920656c656d656e74; opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### tcl8.5

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs. Version: tcl8.5 8.5.19 (capture association; launched patchlevel unqueried).

Exact reached rows: opaque-array-after-nul/one-original-formal/original-definition: code0 result_hex=; opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b2065787472617d207b6b2065787472617d; opaque-array-ordinary/one-original-formal/original-definition: code1 result_hex=666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74; opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022; opaque-array-after-nul/two-original-formals/original-definition: code0 result_hex=; opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=77726f6e67202320617267733a2073686f756c64206265202270206b22; opaque-array-ordinary/two-original-formals/original-definition: code1 result_hex=666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74; opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs. Version: tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact reached rows: opaque-array-after-nul/one-original-formal/original-definition: code1 result_hex=666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74; opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022; opaque-array-ordinary/one-original-formal/original-definition: code1 result_hex=666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74; opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022; opaque-array-after-nul/two-original-formals/original-definition: code1 result_hex=666f726d616c20706172616d6574657220226b2220697320616e20617272617920656c656d656e74; opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022; opaque-array-ordinary/two-original-formals/original-definition: code1 result_hex=666f726d616c20706172616d657465722022612871292220697320616e20617272617920656c656d656e74; opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=696e76616c696420636f6d6d616e64206e616d6520227022. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### jim

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs.

Exact reached rows: opaque-array-after-nul/one-original-formal/original-definition: code0 result_hex=70; opaque-array-after-nul/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b00612871297d207b6578747261206b00612871297d; opaque-array-ordinary/one-original-formal/original-definition: code0 result_hex=70; opaque-array-ordinary/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b612065787472617d207b612065787472617d; opaque-array-after-nul/two-original-formals/original-definition: code0 result_hex=70; opaque-array-after-nul/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b0061287129206b00622871297d207b6578747261206b0061287129206b00622871297d; opaque-array-ordinary/two-original-formals/original-definition: code0 result_hex=70; opaque-array-ordinary/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b612065787472617d207b612065787472617d. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NamePolicyProtocol::formal_enumeration_name_input`: Owns release-selected formal enumeration projection independently from validation and storage keys.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `frame_var_name_bytes`: Returns exact actual frame member byte names under the selected link-inclusion purpose.

## Evidence

- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory.c](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory.c)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-manifest.json](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-manifest.json)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-8.4.20.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-8.4.20.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-8.5.19.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-8.5.19.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-8.6.18.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-8.6.18.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-9.0.4.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-9.0.4.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-9.1.0.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-9.1.0.jsonl)
- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-Jim.jsonl](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory-Jim.jsonl)
