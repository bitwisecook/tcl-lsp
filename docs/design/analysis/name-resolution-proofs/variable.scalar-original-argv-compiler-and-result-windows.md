# naming.variable.scalar-original-argv-compiler-and-result-windows

Kind: `native-observation`

## Problem statement

A public string selector can be abbreviated, braced, quoted or expanded, while a private compiler receives a different operand vector. Counted A-zero-x and A-zero-y arguments also expose a CString comparison boundary that cannot be inferred from the command spelling or result cache alone.

## Question

Which inline opcode, completion/result and original operand/result object fields occur for the exact 15 string-equal, string-length and list-length bodies in each C provider?

## Conclusion

The complete 75 retained C windows distinguish inline scalar compilation from generic invocation, parse/arity failures and the observed counted operand/result/cache fields. C8.4 equality of A-zero-x versus A-zero-y differs from later C equality in this exact roster. Compiler selection and native object windows remain independently scoped to their own original bodies and actual argv; they establish no unrelated variable cell, frame or execution grant.

Selected public-member metadata keeps original compiler operand coordinates and argument offsets. Missing member coordinates refuse; private registrations retain their independent original operand basis.

## Scope

Five private C probes for C8.4.20/8.5.19/8.6.18/9.0.4/9.1.0, exact probe.c and 15 cases.json/cases.rs bodies. Procedure p has two actual original arguments; counted three-byte A-zero-x/A-zero-y values, with a-b list input only cases7–9. Original native object argv, not document-channel text. Jim reference handler inputs are a separate question; BIG-IP unqueried.

The coordinate projection is source/API advice. It supplies no measured original compiler instruction, selected invocation/argv authority, physical frame, scalar result or equivalent handler execution.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Version: tcl8.4 8.4.20.

Compile/run exit 0 with empty stderr; 15 exact original rows. Columns are case, code, result type, result bytes residency, result refs, left type, left bytes residency, result-is-left, result hex and opcodes.

```tsv
0	0	int	0	1	none	1	0	31	loadScalar1,loadScalar1,streq,done
1	0	int	0	1	none	1	0	31	loadScalar1,loadScalar1,streq,done
2	0	boolean	0	1	none	1	0	30	push1,push1,loadScalar1,loadScalar1,invokeStk1,done
3	0	boolean	0	1	none	1	0	30	push1,push1,push1,loadScalar1,loadScalar1,invokeStk1,done
4	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
5	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
6	0	int	0	1	string	1	0	33	push1,push1,loadScalar1,invokeStk1,done
7	0	int	0	1	list	1	0	32	loadScalar1,listlength,done
8	1	none	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	
9	1	none	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	
10	1	none	1	1	none	1	0	6578747261206368617261637465727320616674657220636c6f73652d6272616365	
11	1	none	1	1	none	1	0	6578747261206368617261637465727320616674657220636c6f73652d6272616365	
12	1	none	1	1	none	1	0	6578747261206368617261637465727320616674657220636c6f73652d6272616365	
13	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e67206c656e67746820737472696e6722	push1,push1,invokeStk1,done
14	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e6720657175616c203f2d6e6f636173653f203f2d6c656e67746820696e743f20737472696e673120737472696e673222	push1,push1,loadScalar1,invokeStk1,done
```

### tcl8.5

Status: `observed`. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Version: tcl8.5 8.5.19.

Compile/run exit 0 with empty stderr; 15 exact original rows. Columns are case, code, result type, result bytes residency, result refs, left type, left bytes residency, result-is-left, result hex and opcodes.

```tsv
0	0	int	0	2	none	1	0	31	loadScalar1,loadScalar1,streq,done
1	0	int	1	2	none	1	0	31	loadScalar1,loadScalar1,streq,done
2	0	int	1	2	none	1	0	31	loadScalar1,loadScalar1,streq,done
3	0	int	0	1	none	1	0	30	push1,push1,push1,loadScalar1,loadScalar1,invokeStk1,done
4	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
5	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
6	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
7	0	int	0	1	list	1	0	32	loadScalar1,listLength,done
8	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push1,invokeStk1,done
9	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push1,loadScalar1,loadScalar1,invokeStk1,done
10	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
11	0	int	1	2	none	1	0	31	loadScalar1,loadScalar1,streq,done
12	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push1,push1,push1,invokeStk1,done
13	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e67206c656e67746820737472696e6722	push1,push1,invokeStk1,done
14	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e6720657175616c203f2d6e6f636173653f203f2d6c656e67746820696e743f20737472696e673120737472696e673222	push1,push1,loadScalar1,invokeStk1,done
```

### tcl8.6, tcl9.0

Status: `observed`. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

Compile/run exit 0 with empty stderr; 15 exact original rows. Columns are case, code, result type, result bytes residency, result refs, left type, left bytes residency, result-is-left, result hex and opcodes.

```tsv
0	0	int	0	2	none	1	0	30	loadScalar1,loadScalar1,streq,done
1	0	int	1	2	none	1	0	30	loadScalar1,loadScalar1,streq,done
2	0	int	1	2	none	1	0	30	loadScalar1,loadScalar1,streq,done
3	0	int	0	1	none	1	0	30	push1,push1,push1,loadScalar1,loadScalar1,push1,invokeReplace,done
4	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
5	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
6	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
7	0	int	0	1	list	1	0	32	loadScalar1,listLength,done
8	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push1,invokeStk1,done
9	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push1,loadScalar1,loadScalar1,invokeStk1,done
10	0	int	0	1	string	1	0	33	loadScalar1,strlen,done
11	0	int	1	2	none	1	0	30	loadScalar1,loadScalar1,streq,done
12	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push1,push1,push1,invokeStk1,done
13	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e67206c656e67746820737472696e6722	push1,push1,push1,invokeReplace,done
14	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e6720657175616c203f2d6e6f636173653f203f2d6c656e67746820696e743f20737472696e673120737472696e673222	push1,push1,loadScalar1,push1,invokeReplace,done
```

### tcl9.1

Status: `observed`. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Version: tcl9.1 9.1.0.

Compile/run exit 0 with empty stderr; 15 exact original rows. Columns are case, code, result type, result bytes residency, result refs, left type, left bytes residency, result-is-left, result hex and opcodes.

```tsv
0	0	int	0	2	none	1	0	30	loadScalar,loadScalar,streq,done
1	0	int	1	2	none	1	0	30	loadScalar,loadScalar,streq,done
2	0	int	1	2	none	1	0	30	loadScalar,loadScalar,streq,done
3	0	int	0	1	none	1	0	30	push,push,push,loadScalar,loadScalar,push,invokeReplace,done
4	0	int	0	1	string	1	0	33	loadScalar,strlen,done
5	0	int	0	1	string	1	0	33	loadScalar,strlen,done
6	0	int	0	1	string	1	0	33	loadScalar,strlen,done
7	0	int	0	1	list	1	0	32	loadScalar,listLength,done
8	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push,invokeStk,done
9	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push,loadScalar,loadScalar,invokeStk,done
10	0	int	0	1	string	1	0	33	loadScalar,strlen,done
11	0	int	1	2	none	1	0	30	loadScalar,loadScalar,streq,done
12	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c6420626520226c6c656e677468206c69737422	push,push,push,invokeStk,done
13	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e67206c656e67746820737472696e6722	push,push,push,invokeReplace,done
14	1	string	1	1	none	1	0	77726f6e67202320617267733a2073686f756c642062652022737472696e6720657175616c203f2d6e6f636173653f203f2d6c656e67746820696e743f20737472696e673120737472696e673222	push,push,loadScalar,push,invokeReplace,done
```

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `original_scalar_recipes_match_seventy_five_native_compiler_windows`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `native_scalar_compilation::tests::original_scalar_recipes_match_seventy_five_native_compiler_windows` (linked): Checks actual release-specific compiler acceptance separately from original native completion/object fields.

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::original_operand_from_for_facts`: Use selected public-member facts and their actual argument offset to retain original operand coordinates; private registrations keep their own basis.
- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `native_scalar_compilation::tests::original_scalar_coordinates_retain_the_selected_public_member` (linked): Selected public member facts retain argument_offset when projecting original compiler operands; a missing member coordinate refuses, and private registration facts use their own independent operand basis. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-registry/tests/data/native_scalar_compilation](../../../../rust/tcl-registry/tests/data/native_scalar_compilation)
