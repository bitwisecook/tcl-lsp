# naming.object-vector.native-error-result-publication

Kind: `native-observation`

## Problem statement

A native callback can publish an untyped original result and return Error; explicit logging and outer evaluation are separate producers. Replacing every error with a formatted string would discard the observed result primary and argument window.

## Question

Does direct/source invocation preserve the original FAILED callback result and argument residency, including the explicit logged-error control?

## Conclusion

Both failing and explicitly logged callbacks return code1 and bytes FAILED in all tested providers. C results remain untyped, but direct object-vector errors materialize the original List argument while source errors do not. Jim results and original arguments remain untyped/stringless in both routes; its logged control is the plain fail callback, because no Tcl_LogCommandInfo call is compiled there. The probe does not print errorInfo, so it does not prove equality of logged error state.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.4 8.4.20 (capture association; tcl8.5 8.5.19 (capture association; tcl8.6 8.6.18 (capture association; tcl9.0 9.0.4 (capture association; tcl9.1 9.1.0 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["1","1","1","none","4641494c4544"],["4","1","1","none","4641494c4544"],["6","1","0","none","4641494c4544"],["9","1","0","none","4641494c4544"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: jim Jim (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["1","1","0","none","4641494c4544"],["4","1","0","none","4641494c4544"],["6","1","0","none","4641494c4544"],["9","1","0","none","4641494c4544"]]. Process/compile status0; empty stderr.

### bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `eval_original_object_vector`: Consumes original borrowed argv through the independently selected evaluator/error/completion protocol; the captured native row does not manufacture a Rust original object.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `tests::original_public_vectors_and_source_commands_match_all_60_native_windows` (linked): The named original cases independently compare completion, original argument residency, result primary and bytes before the physical observer renders output.

## Evidence

- [runtime/rust/tests/data/native_object_vector/probe.c](../../../../runtime/rust/tests/data/native_object_vector/probe.c)
- [runtime/rust/tests/data/native_object_vector/manifest.json](../../../../runtime/rust/tests/data/native_object_vector/manifest.json)
- [runtime/rust/tests/data/native_object_vector/8.4.20.txt](../../../../runtime/rust/tests/data/native_object_vector/8.4.20.txt)
- [runtime/rust/tests/data/native_object_vector/8.5.19.txt](../../../../runtime/rust/tests/data/native_object_vector/8.5.19.txt)
- [runtime/rust/tests/data/native_object_vector/8.6.18.txt](../../../../runtime/rust/tests/data/native_object_vector/8.6.18.txt)
- [runtime/rust/tests/data/native_object_vector/9.0.4.txt](../../../../runtime/rust/tests/data/native_object_vector/9.0.4.txt)
- [runtime/rust/tests/data/native_object_vector/9.1.0.txt](../../../../runtime/rust/tests/data/native_object_vector/9.1.0.txt)
- [runtime/rust/tests/data/native_object_vector/Jim.txt](../../../../runtime/rust/tests/data/native_object_vector/Jim.txt)
