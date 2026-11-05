# naming.compiled-local-inventory.opaque-byte-enumeration

Kind: `native-observation`

## Problem statement

An original FF-containing formal is not a UTF document identifier. Replacing it with decoded display advice would alter the stored and enumerated member even when the procedure accepts its arguments.

## Question

Do original k FF a and k FF b formals retain exact bytes in successful info locals/info vars enumeration?

## Conclusion

All five C releases and Jim install the original opaque formals and return exact k FF a, plus k FF b in the two-formal case. C enumerates formal members before extra; Jim one-formal enumerates extra first and its two-formal hash order is k FF a, extra, k FF b. The observed order and bytes belong to this original object-vector definition and exact body, not source UTF ingress or an arbitrary table walk.

## Scope

Original counted String formal members and an original List parameter object enter public Tcl_EvalObjv(TCL_EVAL_GLOBAL) or Jim_EvalObjVector. The body is exactly set extra EXTRA;info exists absent;list [info locals] [info vars]. One/two-formal axes and definition/invocation outcomes are captured before later error observers; the source retains result/return-options and uses the C8.4 private error-storage observer. originalA/originalB callbacks are registered but not called by this body. The manifest observation_window boilerplate mentions braced/dynamic substitution; this exact probe has only the stated enumeration body. Forty original rows per provider, full output/source/library/header/executable hashes and compile/run status are retained; Jim patchlevel/revision/configure flags and all launched C patchlevels are unqueried. BIG-IP is not tested; manifest vm_comparisons is0.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs. Version: tcl8.4 8.4.20 (capture association; launched patchlevel unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel unqueried).

Exact reached rows: raw-invalid-byte/one-original-formal/original-definition: code0 result_hex=; raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6bff612065787472617d207b6bff612065787472617d; raw-invalid-byte/two-original-formals/original-definition: code0 result_hex=; raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6bff61206bff622065787472617d207b6bff61206bff622065787472617d. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### jim

Status: `observed`. Channel: Public original object-vector procedure definition and invocation; counted original String/List name inputs.

Exact reached rows: raw-invalid-byte/one-original-formal/original-definition: code0 result_hex=70; raw-invalid-byte/one-original-formal/compiled-and-dynamic-invocation: code0 result_hex=7b6bff612065787472617d207b6bff612065787472617d; raw-invalid-byte/two-original-formals/original-definition: code0 result_hex=70; raw-invalid-byte/two-original-formals/compiled-and-dynamic-invocation: code0 result_hex=7b6bff61206578747261206bff627d207b6bff61206578747261206bff627d. Full error/options fields remain in the evidence; invocation after a rejected definition does not enter the body.

### bigip

Status: `not-tested`.

No observation of this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NamePolicyProtocol::formal_enumeration_name_input`: Owns release-selected formal enumeration projection independently from validation and storage keys.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `frame_var_name_bytes`: Returns exact actual frame member byte names under the selected link-inclusion purpose.

## Evidence

- [rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory.c](../../../../rust/tcl-vm/tests/data/native_formal_slots/compiled-local-inventory.c)
