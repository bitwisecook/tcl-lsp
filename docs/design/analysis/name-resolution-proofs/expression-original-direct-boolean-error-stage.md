# naming.expression.original-direct-boolean-error-stage

Kind: `native-observation`

## Problem statement

A direct expression API and an entered expr command can normalize error state at different boundaries even for the same original variable value. Result bytes alone cannot establish when the seeded code was replaced.

## Question

What seeded result/code/options are captured inside direct Tcl_ExprBooleanObj $v before outer script propagation?

## Conclusion

The six original values x,1-zero-X,1.5-zero-X,NaN,08,true reach distinct arithmetic/Boolean/entered-command outcomes by release. The matrix independently retains inside result, inside error code/return-options and later propagated code. C8.4 direct invalid arithmetic can retain PROBE BEFORE until outer evaluation changes it to NONE, while NaN domain state is already published. Full provider rows bound this conclusion to the selected API rather than the expression spelling alone.

## Scope

One retained original counted variable value per case enters the exact API indicated by route0/1/2. The callback saves its inside result and options/code before returning; outer Tcl_Eval then produces the propagated code. Eighteen rows per C release have exact source/library/output hashes, but compile status/executable/header/compiler/configure/runtime-version query are absent. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=1 case=0 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 propagatedCode=4e4f4e45
route=1 case=1 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=50524f4245204245464f5245
route=1 case=2 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f742022312e3522 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=657870656374656420626f6f6c65616e2076616c75652062757420676f742022312e3522 propagatedCode=4e4f4e45
route=1 case=3 code=1 inside=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 insideCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d insideOptionsCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d result=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 propagatedCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d
route=1 case=4 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=50524f4245204245464f5245
route=1 case=5 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=50524f4245204245464f5245
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=1 case=0 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 insideCode=54434c2056414c5545204e554d424552 insideOptionsCode=54434c2056414c5545204e554d424552 result=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 propagatedCode=54434c2056414c5545204e554d424552
route=1 case=1 code=0 inside=31 insideCode=414253454e54 insideOptionsCode=414253454e54 result=31 propagatedCode=414253454e54
route=1 case=2 code=0 inside=31 insideCode=414253454e54 insideOptionsCode=414253454e54 result=31 propagatedCode=414253454e54
route=1 case=3 code=1 inside=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 insideCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d insideOptionsCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d result=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 propagatedCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d
route=1 case=4 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 insideCode=54434c2056414c5545204e554d424552 insideOptionsCode=54434c2056414c5545204e554d424552 result=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 propagatedCode=54434c2056414c5545204e554d424552
route=1 case=5 code=0 inside=31 insideCode=414253454e54 insideOptionsCode=414253454e54 result=31 propagatedCode=414253454e54
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.6

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=1 case=0 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 insideCode=54434c2056414c5545204e554d424552 insideOptionsCode=54434c2056414c5545204e554d424552 result=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 propagatedCode=54434c2056414c5545204e554d424552
route=1 case=1 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
route=1 case=2 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
route=1 case=3 code=1 inside=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 insideCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d insideOptionsCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d result=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 propagatedCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d
route=1 case=4 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 insideCode=54434c2056414c5545204e554d424552 insideOptionsCode=54434c2056414c5545204e554d424552 result=657870656374656420626f6f6c65616e2076616c75652062757420676f74202230382220286c6f6f6b73206c696b6520696e76616c6964206f6374616c206e756d62657229 propagatedCode=54434c2056414c5545204e554d424552
route=1 case=5 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=1 case=0 code=1 inside=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 insideCode=54434c2056414c5545204e554d424552 insideOptionsCode=54434c2056414c5545204e554d424552 result=657870656374656420626f6f6c65616e2076616c75652062757420676f7420227822 propagatedCode=54434c2056414c5545204e554d424552
route=1 case=1 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
route=1 case=2 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
route=1 case=3 code=1 inside=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 insideCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d insideOptionsCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d result=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 propagatedCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d
route=1 case=4 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
route=1 case=5 code=0 inside=31 insideCode=50524f4245204245464f5245 insideOptionsCode=50524f4245204245464f5245 result=31 propagatedCode=414253454e54
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/interp.rs](../../../../runtime/rust/src/interp.rs), `report_cmd_error`: Keeps primitive/direct expression error publication separate from later entered script error normalization.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/probe.c](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/probe.c)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.1.0.txt)
