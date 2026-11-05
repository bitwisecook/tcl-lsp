# naming.object-vector.completion.error-list-result-storage

Kind: `native-observation`

## Problem statement

Error and List can retain List-shaped results even though one completes with Error and the other succeeds. An outer completion code should not by itself force a String result or identify an original pointer.

## Question

How do error and list preserve result primary and original argument storage through vector/source completion?

## Conclusion

Error returns code1 with List result bytes7B4120427D in all providers. Its original argument is resident after direct C evaluation but stringless after source C evaluation and both Jim routes. List returns code0 with a List result7B7B4120427D7D and a stringless original argument in both routes for all providers. No native result-pointer identity, lifetime or full error state is measured.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.4 8.4.20 (capture association; tcl8.5 8.5.19 (capture association; tcl8.6 8.6.18 (capture association; tcl9.0 9.0.4 (capture association; tcl9.1 9.1.0 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["3","1","1","list","7b4120427d"],["4","0","0","list","7b7b4120427d7d"],["8","1","0","list","7b4120427d"],["9","0","0","list","7b7b4120427d7d"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: jim Jim (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["3","1","0","list","7b4120427d"],["4","0","0","list","7b7b4120427d7d"],["8","1","0","list","7b4120427d"],["9","0","0","list","7b7b4120427d7d"]]. Process/compile status0; empty stderr.

### bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `eval_original_object_vector`: Consumes original borrowed argv through the independently selected evaluator/error/completion protocol; the captured native row does not manufacture a Rust original object.

## Evidence

- [runtime/rust/tests/data/native_object_vector/completions/probe.c](../../../../runtime/rust/tests/data/native_object_vector/completions/probe.c)
- [runtime/rust/tests/data/native_object_vector/completions/manifest.json](../../../../runtime/rust/tests/data/native_object_vector/completions/manifest.json)
- [runtime/rust/tests/data/native_object_vector/completions/8.4.20.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.4.20.txt)
- [runtime/rust/tests/data/native_object_vector/completions/8.5.19.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.5.19.txt)
- [runtime/rust/tests/data/native_object_vector/completions/8.6.18.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.6.18.txt)
- [runtime/rust/tests/data/native_object_vector/completions/9.0.4.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/9.0.4.txt)
- [runtime/rust/tests/data/native_object_vector/completions/9.1.0.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/9.1.0.txt)
- [runtime/rust/tests/data/native_object_vector/completions/Jim.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/Jim.txt)
