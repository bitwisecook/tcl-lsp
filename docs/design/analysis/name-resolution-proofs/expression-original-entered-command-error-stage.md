# naming.expression.original-entered-command-error-stage

Kind: `native-observation`

## Problem statement

A direct expression API and an entered expr command can normalize error state at different boundaries even for the same original variable value. Result bytes alone cannot establish when the seeded code was replaced.

## Question

What result/code/options remain after entered expr {$v+1} inside the callback and after outer propagation?

## Conclusion

The six original values x,1-zero-X,1.5-zero-X,NaN,08,true reach distinct arithmetic/Boolean/entered-command outcomes by release. The matrix independently retains inside result, inside error code/return-options and later propagated code. C8.4 direct invalid arithmetic can retain PROBE BEFORE until outer evaluation changes it to NONE, while NaN domain state is already published. Full provider rows bound this conclusion to the selected API rather than the expression spelling alone.

## Scope

One retained original counted variable value per case enters the exact API indicated by route0/1/2. The callback saves its inside result and options/code before returning; outer Tcl_Eval then produces the propagated code. Eighteen rows per C release have exact source/library/output hashes, but compile status/executable/header/compiler/configure/runtime-version query are absent. Jim and BIG-IP are not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=1 code=1 inside=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=2 code=1 inside=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e27742075736520666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=3 code=1 inside=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 insideCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d insideOptionsCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d result=646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e6765 propagatedCode=415249544820444f4d41494e207b646f6d61696e206572726f723a20617267756d656e74206e6f7420696e2076616c69642072616e67657d
route=2 case=4 code=1 inside=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
route=2 case=5 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=4e4f4e45 insideOptionsCode=4e4f4e45 result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=4e4f4e45
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
route=2 case=1 code=0 inside=32 insideCode=414253454e54 insideOptionsCode=414253454e54 result=32 propagatedCode=414253454e54
route=2 case=2 code=0 inside=322e35 insideCode=414253454e54 insideOptionsCode=414253454e54 result=322e35 propagatedCode=414253454e54
route=2 case=3 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d result=63616e277420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c7565206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d
route=2 case=4 code=1 inside=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d insideOptionsCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d result=63616e27742075736520696e76616c6964206f6374616c206e756d626572206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b696e76616c6964206f6374616c206e756d6265727d
route=2 case=5 code=1 inside=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e277420757365206e6f6e2d6e756d6572696320737472696e67206173206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original variable value set from counted Tcl_NewStringObj, selected direct expression API or entered expr, then outer Tcl_Eval propagation. Version: tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=2 case=0 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227822206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227822206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
route=2 case=1 code=0 inside=32 insideCode=414253454e54 insideOptionsCode=414253454e54 result=32 propagatedCode=414253454e54
route=2 case=2 code=0 inside=322e35 insideCode=414253454e54 insideOptionsCode=414253454e54 result=322e35 propagatedCode=414253454e54
route=2 case=3 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c756520224e614e22206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c756520224e614e22206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320666c6f6174696e672d706f696e742076616c75657d
route=2 case=4 code=0 inside=39 insideCode=414253454e54 insideOptionsCode=414253454e54 result=39 propagatedCode=414253454e54
route=2 case=5 code=1 inside=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227472756522206173206c656674206f706572616e64206f6620222b22 insideCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d insideOptionsCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d result=63616e6e6f7420757365206e6f6e2d6e756d6572696320737472696e6720227472756522206173206c656674206f706572616e64206f6620222b22 propagatedCode=415249544820444f4d41494e207b6e6f6e2d6e756d6572696320737472696e677d
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
