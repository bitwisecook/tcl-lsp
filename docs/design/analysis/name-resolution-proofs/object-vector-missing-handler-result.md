# naming.object-vector.missing-handler-result

Kind: `native-observation`

## Problem statement

Calling an absent command through an original object vector can render arguments to publish an error, while invoking a source command reaches a different reporting path. A rendered missing-command message does not prove that argument storage stayed untouched.

## Question

For an absent handler, do original object-vector and source evaluation leave the same argument residency and result primary?

## Conclusion

All five C releases return code1 and a string result with invalid command name "missing_original". The original List argument is resident after direct Tcl_EvalObjv but remains stringless after Tcl_EvalEx source evaluation. Jim returns the same message with an untyped result and a stringless original List for both routes. These snapshots precede reading result bytes and do not measure an unknown-handler replacement or full errorInfo.

## Scope

The probe creates one original stringless List containing A B, installs it as arg, and observes five commands through public original-object argv (flags0) and source evaluation (flags0). Case0–4 are direct; case5–9 are source. Result primary and original argument residency are sampled before the result byte observer. Full source, six captured streams and compile/library/header/executable metadata are retained. C release labels are capture metadata; the probe does not query runtime patchlevel. Jim revision/patch/configuration is not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: tcl8.4 8.4.20 (capture association; tcl8.5 8.5.19 (capture association; tcl8.6 8.6.18 (capture association; tcl9.0 9.0.4 (capture association; tcl9.1 9.1.0 (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","1","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","string","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

### jim

Status: `observed`. Channel: compiled public Tcl_EvalObjv/Jim_EvalObjVector and source Tcl_EvalEx/Jim_Eval, flags0. Version: jim Jim (capture association.

Relevant original rows [case,completion,argument resident,result primary,result hex]: [["0","1","0","none","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"],["5","1","0","none","696e76616c696420636f6d6d616e64206e616d6520226d697373696e675f6f726967696e616c22"]]. Process/compile status0; empty stderr.

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
