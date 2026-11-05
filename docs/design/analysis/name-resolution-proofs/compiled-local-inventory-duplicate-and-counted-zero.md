# naming.compiled-local-inventory.duplicate-and-counted-zero

Kind: `native-observation`

## Problem statement

Original formal objects can be distinct by counted bytes while procedure validation, binding and enumeration use different release-specific keys. Equal printed prefixes and identical duplicates therefore do not establish the same local inventory or accepted argv count.

## Question

What info locals/info vars inventory is reached for one/two original ordinary, raw-zero and modified-zero formal names?

## Conclusion

All C releases preserve both ordinary duplicate x slots in enumeration when two arguments are supplied. C8.4/8.5 enumerate a raw-zero formal as k and the two raw-zero or identical-zero-formal calls reject two arguments with usage p k; modified-zero names remain counted and two distinct members enumerate. C8.6–9.1 preserve full raw-zero suffixes and enumerate both supplied formal slots, including identical raw-zero duplicates. Jim retains counted distinct raw-zero/modified-zero members, collapses identical duplicate enumeration to one member, and orders extra differently. This is the returned enumeration for these bodies, not a complete allocation/refcount/compiled-name lookup proof.

## Scope

Original counted String formal members and an original List parameter object enter public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. The body is exactly set extra EXTRA;info exists absent;list [info locals] [info vars]. One/two-formal axes and definition/invocation outcomes are captured before later error observers; the source retains result/return-options and uses the C8.4 private error-storage observer. originalA/originalB callbacks are registered but not called by this body. The manifest observation_window boilerplate mentions braced/dynamic substitution; this exact probe has only the stated enumeration body. Forty original rows per provider, full output/source/library/header/executable hashes and compile/run status are retained; Jim patchlevel/revision/configure flags and all launched C patchlevels are unqueried. BIG-IP is not tested; manifest vm_comparisons is0.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel unqueried).

Exact reached rows: ordinary-duplicate/one-original-formal/original-definition: code0 result_hex=; ordinary-duplicate/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b782065787472617d207b782065787472617d; nul-equal-length/one-original-formal/original-definition: code0 result_hex=; nul-equal-length/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b2065787472617d207b6b2065787472617d; nul-different-length/one-original-formal/original-definition: code0 result_hex=; nul-different-length/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b2065787472617d207b6b2065787472617d; modified-nul/one-original-formal/original-definition: code0 result_hex=; modified-nul/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6bc080612065787472617d207b6bc080612065787472617d; identical-duplicate/one-original-formal/original-definition: code0 result_hex=; identical-duplicate/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b2065787472617d207b6b2065787472617d; ordinary-duplicate/two-original-formals/original-definition: code0 result_hex=; ordinary-duplicate/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b7820782065787472617d207b7820782065787472617d; nul-equal-length/two-original-formals/original-definition: code0 result_hex=; nul-equal-length/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=77726f6e67202320617267733a2073686f756c64206265202270206b22; nul-different-length/two-original-formals/original-definition: code0 result_hex=; nul-different-length/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=77726f6e67202320617267733a2073686f756c64206265202270206b22; modified-nul/two-original-formals/original-definition: code0 result_hex=; modified-nul/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6bc08061206bc080622065787472617d207b6bc08061206bc080622065787472617d; identical-duplicate/two-original-formals/original-definition: code0 result_hex=; identical-duplicate/two-original-formals/compiled-and-dynamic-invocation: code1 result_hex=77726f6e67202320617267733a2073686f756c64206265202270206b22. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs. Version: tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact reached rows: ordinary-duplicate/one-original-formal/original-definition: code0 result_hex=; ordinary-duplicate/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b782065787472617d207b782065787472617d; nul-equal-length/one-original-formal/original-definition: code0 result_hex=; nul-equal-length/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b00612065787472617d207b6b00612065787472617d; nul-different-length/one-original-formal/original-definition: code0 result_hex=; nul-different-length/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b00612065787472617d207b6b00612065787472617d; modified-nul/one-original-formal/original-definition: code0 result_hex=; modified-nul/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6bc080612065787472617d207b6bc080612065787472617d; identical-duplicate/one-original-formal/original-definition: code0 result_hex=; identical-duplicate/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6b00612065787472617d207b6b00612065787472617d; ordinary-duplicate/two-original-formals/original-definition: code0 result_hex=; ordinary-duplicate/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b7820782065787472617d207b7820782065787472617d; nul-equal-length/two-original-formals/original-definition: code0 result_hex=; nul-equal-length/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6b0061206b00622065787472617d207b6b0061206b00622065787472617d; nul-different-length/two-original-formals/original-definition: code0 result_hex=; nul-different-length/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6b0061206b0062622065787472617d207b6b0061206b0062622065787472617d; modified-nul/two-original-formals/original-definition: code0 result_hex=; modified-nul/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6bc08061206bc080622065787472617d207b6bc08061206bc080622065787472617d; identical-duplicate/two-original-formals/original-definition: code0 result_hex=; identical-duplicate/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6b0061206b00612065787472617d207b6b0061206b00612065787472617d. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### jim

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs.

Exact reached rows: ordinary-duplicate/one-original-formal/original-definition: code0 result_hex=70; ordinary-duplicate/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b657874726120787d207b657874726120787d; nul-equal-length/one-original-formal/original-definition: code0 result_hex=70; nul-equal-length/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b00617d207b6578747261206b00617d; nul-different-length/one-original-formal/original-definition: code0 result_hex=70; nul-different-length/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b00617d207b6578747261206b00617d; modified-nul/one-original-formal/original-definition: code0 result_hex=70; modified-nul/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206bc080617d207b6578747261206bc080617d; identical-duplicate/one-original-formal/original-definition: code0 result_hex=70; identical-duplicate/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b00617d207b6578747261206b00617d; ordinary-duplicate/two-original-formals/original-definition: code0 result_hex=70; ordinary-duplicate/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b657874726120787d207b657874726120787d; nul-equal-length/two-original-formals/original-definition: code0 result_hex=70; nul-equal-length/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b0061206b00627d207b6578747261206b0061206b00627d; nul-different-length/two-original-formals/original-definition: code0 result_hex=70; nul-different-length/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b0061206b0062627d207b6578747261206b0061206b0062627d; modified-nul/two-original-formals/original-definition: code0 result_hex=70; modified-nul/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206bc08061206bc080627d207b6578747261206bc08061206bc080627d; identical-duplicate/two-original-formals/original-definition: code0 result_hex=70; identical-duplicate/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6578747261206b00617d207b6578747261206b00617d. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

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
