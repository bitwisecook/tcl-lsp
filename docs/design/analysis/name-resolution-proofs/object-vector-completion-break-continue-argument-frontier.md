# naming.object-vector.completion.break-continue-argument-frontier

Kind: `native-observation`

## Problem statement

The one original List argument is extra argv for stock C break/continue but a potential level operand in Jim. The same Error completion can arise from different argument consumers and primary changes.

## Question

For break/continue with the original List argument, which usage/numeric frontier is reached through direct and source evaluation?

## Conclusion

C Tcl returns code1 with string wrong-argument usage for break or continue. Direct vector calls materialize the original argument; source calls leave it stringless. Jim reaches its integer-level conversion in both routes, materializes the argument, and returns code1/untyped result with expected integer but got "{A B}". These are rejected one-argument calls, not successful loop-control executions.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.4 8.4.20 (capture association; tcl8.5 8.5.19 (capture association; tcl8.6 8.6.18 (capture association; tcl9.0 9.0.4 (capture association; tcl9.1 9.1.0 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["1","1","1","string","77726f6e67202320617267733a2073686f756c642062652022627265616b22"],["2","1","1","string","77726f6e67202320617267733a2073686f756c642062652022636f6e74696e756522"],["6","1","0","string","77726f6e67202320617267733a2073686f756c642062652022627265616b22"],["7","1","0","string","77726f6e67202320617267733a2073686f756c642062652022636f6e74696e756522"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: jim Jim (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["1","1","1","none","657870656374656420696e74656765722062757420676f7420227b4120427d22"],["2","1","1","none","657870656374656420696e74656765722062757420676f7420227b4120427d22"],["6","1","1","none","657870656374656420696e74656765722062757420676f7420227b4120427d22"],["7","1","1","none","657870656374656420696e74656765722062757420676f7420227b4120427d22"]]. Process/compile status0; empty stderr.

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
