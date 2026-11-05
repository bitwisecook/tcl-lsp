# naming.variable.separate-root-index-quiet-write-error-presentation

Kind: `native-observation`

## Problem statement

A missing namespace can prevent a write before value installation. Quiet flags and error-reporting flags may select the same failure but preserve different interpreter results; successful value mutation cannot be inferred from receiver-name conversion.

## Question

Quiet versus error-reporting separate root/index Tcl_ObjSetVar2 lookup What exact stages and results occur in the retained native probe?

## Conclusion

All five C releases reject the missing namespace in both flag modes. Quiet selection preserves SENTINEL; TCL_LEAVE_ERR_MSG publishes the exact array diagnostic. This is a failed lookup/presentation control, not a successful store or normal-completion proof.

## Scope

Two original root/index string-object writes per C release, fresh interpreter/result sentinel, public Tcl_ObjSetVar2, ten output tuples; no Jim/BIG-IP.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl8.4 8.4.20.

Exact captured transcript:

"engine_patchlevel=8.4.20\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl8.5

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl8.5 8.5.19.

Exact captured transcript:

"engine_patchlevel=8.5.19\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl8.6

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl8.6 8.6.18.

Exact captured transcript:

"engine_patchlevel=8.6.18\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl9.0

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl9.0 9.0.4.

Exact captured transcript:

"engine_patchlevel=9.0.4\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl9.1

Status: `observed`. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Version: tcl9.1 9.1.0.

Exact captured transcript:

"engine_patchlevel=9.1.0\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-manifest.json)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.4.20.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.5.19.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.6.18.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.0.4.stdout)
- [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.1.0.stdout)
