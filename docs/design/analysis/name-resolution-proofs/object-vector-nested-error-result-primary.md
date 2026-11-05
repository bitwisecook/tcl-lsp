# naming.object-vector.nested-error-result-primary

Kind: `native-observation`

## Problem statement

An entered child script can supply the error result before the outer vector returns. The child result primary and outer argument materialization are distinct; an error byte string does not identify which object representation produced it.

## Question

What original argument residency and result primary remain after a native callback enters error CHILD through vector versus source evaluation?

## Conclusion

All providers return code1 and CHILD. C results are untyped and the outer List argument becomes resident only for direct object-vector evaluation. Jim retains a source primary on the CHILD result and a stringless original argument for both routes. The result is sampled before its byte observer; this is not an object-pointer identity or callback-footprint proof.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.4 8.4.20 (capture association; tcl8.5 8.5.19 (capture association; tcl8.6 8.6.18 (capture association; tcl9.0 9.0.4 (capture association; tcl9.1 9.1.0 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["2","1","1","none","4348494c44"],["7","1","0","none","4348494c44"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: jim Jim (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["2","1","0","source","4348494c44"],["7","1","0","source","4348494c44"]]. Process/compile status0; empty stderr.

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
