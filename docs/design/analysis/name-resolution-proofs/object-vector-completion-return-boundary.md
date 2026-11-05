# naming.object-vector.completion.return-boundary

Kind: `native-observation`

## Problem statement

A return completion can be consumed or surfaced by the evaluator, and direct original vectors can cross a different completion boundary from source text. Treating all successful-looking return bytes as code0 loses the release-specific boundary.

## Question

How do direct original-object return and source return normalize completion while preserving the original List result storage?

## Conclusion

Direct return is code0/List/stringless in C8.4 and C8.6–9.1, but C8.5 returns code1, materializes the original argument, and publishes string "command returned bad code: 0". Source return is code0/List/stringless in all five C releases. Jim returns code2/List/stringless through both routes. Successful List result bytes are7B4120427D. No result pointer is sampled by the native probe, so List type/bytes alone do not attest original-object identity.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.4 8.4.20 (capture association; tcl8.6 8.6.18 (capture association; tcl9.0 9.0.4 (capture association; tcl9.1 9.1.0 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","0","0","list","7b4120427d"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### tcl8.5

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.5 8.5.19 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","636f6d6d616e642072657475726e65642062616420636f64653a2030"],["5","0","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: jim Jim (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","2","0","list","7b4120427d"],["5","2","0","list","7b4120427d"]]. Process/compile status0; empty stderr.

### bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `eval_original_object_vector`: Consumes original borrowed argv through the independently selected evaluator/error/completion protocol; the captured native row does not manufacture a Rust original object.
- [runtime/rust/src/interp/native_object_vector.rs](../../../../runtime/rust/src/interp/native_object_vector.rs), `tests::public_original_return_matches_all_six_native_completion_boundaries` (linked): Compares the six direct case0 return completion/residency/primary/byte windows; additional Rust pointer equality is an implementation assertion rather than a native pointer measurement. Source return rows5 are retained empirical controls, not asserted by this selector.

## Evidence

- [runtime/rust/tests/data/native_object_vector/completions/probe.c](../../../../runtime/rust/tests/data/native_object_vector/completions/probe.c)
- [runtime/rust/tests/data/native_object_vector/completions/manifest.json](../../../../runtime/rust/tests/data/native_object_vector/completions/manifest.json)
- [runtime/rust/tests/data/native_object_vector/completions/8.4.20.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.4.20.txt)
- [runtime/rust/tests/data/native_object_vector/completions/8.5.19.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.5.19.txt)
- [runtime/rust/tests/data/native_object_vector/completions/8.6.18.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/8.6.18.txt)
- [runtime/rust/tests/data/native_object_vector/completions/9.0.4.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/9.0.4.txt)
- [runtime/rust/tests/data/native_object_vector/completions/9.1.0.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/9.1.0.txt)
- [runtime/rust/tests/data/native_object_vector/completions/Jim.txt](../../../../runtime/rust/tests/data/native_object_vector/completions/Jim.txt)
