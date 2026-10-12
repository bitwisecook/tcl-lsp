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

Status: `observed`. Version: 8.4.20. Build: Archive SHA d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; header SHA 824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf; probe SHA b06aa4f2a0c3b2544783922b9ebd82d8ea4f7dab7fa54e99656baf171bed705c. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Dialect: Tcl.

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

Status: `observed`. Version: 8.5.19. Build: Archive SHA 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; header SHA c94cc4e9c79077f0fbb49da3f5f45b8a4e01d53ed605a9a2b8da50cdab7ecdd5; probe SHA 81128b8e4150ecae895169c8434734ac27eb1b3fb259dd853689ff310ba923da. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Dialect: Tcl.

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

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Archive SHA a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; header SHA aed709900889091def1cd98ffad6f214b870b0e8dbbf7933314fb4668655c245; probe SHA 15f56435ed577987d9cb5ff4409f2775c31238b1f04fea9a2ed5350c950fa098. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Dialect: Tcl.

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

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Archive SHA 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; header SHA eacd3dc6b0f9615fa654e4645699221dd30137811bd09f92c34f3f7135e96f7a; probe SHA 26927590a7b12ba0f164041aaf261178a23c2990edbcd79549966fbdd20cdee2. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Dialect: Tcl.

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

Status: `observed`. Version: 9.1.0. Build: Archive SHA 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; header SHA 30fa3a517fae1771933cc40955702c942f7be74efcecff4790e7b423dc1be950; probe SHA 2114b56819485e2db06f4ed1ce64046723506cf2b2dc1f153bd9efe64d98cb67. Channel: Original Tcl_NewStringObj counted argv through Tcl_EvalObjv/private procedure bytecode. Dialect: Tcl.

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

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_scalar_compilation/manifest.json](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/manifest.json). SHA-256 `f4b7cb94b7ee39d8a040b69a676874d3f37a20e9da7c4633eb491a2a28376dfb`. All five original compile/run commands, header/archive/probe hashes and stdout/stderr hex.
- `e1` (input): [rust/tcl-registry/tests/data/native_scalar_compilation/probe.c](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/probe.c). SHA-256 `d60649c8ccecd9c9d88781fda3b5c4006b96f3fa99eecec52e118d8f69fb96ca`. Actual 15-body procedure, original three-byte argv, private bytecode and object/result inspection.
- `e2` (input): [rust/tcl-registry/tests/data/native_scalar_compilation/cases.json](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/cases.json). SHA-256 `1620d5110cda29a730e268f0a4d06e3e3086e783638deeeef1de74c7784c7cec`. Exact shared 15-body roster.
- `e3` (input): [rust/tcl-registry/tests/data/native_scalar_compilation/cases.rs](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/cases.rs). SHA-256 `2c4d16f01383ca329bb1e25e1b92b5869832e5296e823ddc9c698817e1b221d8`. Same bodies consumed by the current Rust comparison.
- `e4` (observation): [rust/tcl-registry/tests/data/native_scalar_compilation/8.4.20.tsv](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/8.4.20.tsv). SHA-256 `a6e55904a11165e0a275405a1235d6511e7405594b830def4377e66efb0ce12a`. All fifteen opcode/completion/result/type/cache/refcount/identity windows.
- `e5` (observation): [rust/tcl-registry/tests/data/native_scalar_compilation/8.5.19.tsv](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/8.5.19.tsv). SHA-256 `411b1ad6b1a9015329801fd0db66900f47a8f18f48a934b141a29e5839d577c0`. All fifteen opcode/completion/result/type/cache/refcount/identity windows.
- `e6` (observation): [rust/tcl-registry/tests/data/native_scalar_compilation/8.6.18.tsv](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/8.6.18.tsv). SHA-256 `135f35d392c8075b5191659035a939b76272e4f77a01d10781ac505100b4fbe6`. All fifteen opcode/completion/result/type/cache/refcount/identity windows.
- `e7` (observation): [rust/tcl-registry/tests/data/native_scalar_compilation/9.0.4.tsv](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/9.0.4.tsv). SHA-256 `135f35d392c8075b5191659035a939b76272e4f77a01d10781ac505100b4fbe6`. All fifteen opcode/completion/result/type/cache/refcount/identity windows.
- `e8` (observation): [rust/tcl-registry/tests/data/native_scalar_compilation/9.1.0.tsv](../../../../rust/tcl-registry/tests/data/native_scalar_compilation/9.1.0.tsv). SHA-256 `36c33c4b676bead216dabd1b96327ff3762634c36fb5f2ac0b7a3269746a4691`. All fifteen opcode/completion/result/type/cache/refcount/identity windows.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `original_scalar_recipes_match_seventy_five_native_compiler_windows`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `native_scalar_compilation::tests::original_scalar_recipes_match_seventy_five_native_compiler_windows` (linked): Checks actual release-specific compiler acceptance separately from original native completion/object fields.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::original_operand_from_for_facts`: Use selected public-member facts and their actual argument offset to retain original operand coordinates; private registrations keep their own basis.
- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `native_scalar_compilation::tests::original_scalar_coordinates_retain_the_selected_public_member` (linked): Selected public member facts retain argument_offset when projecting original compiler operands; a missing member coordinate refuses, and private registration facts use their own independent operand basis. This is the current software/API definition; no assertion outcome or new original-provider observation is attached to this binding.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-registry/tests/data/native_scalar_compilation/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Replace provider/header/archive paths with the recorded release/build and preserve the manifested original compiler options. Execute /tmp/native-variable-proof in a fresh process; require all stdout bytes to equal that provider's retained transcript, empty stderr and exit 0. The retained probe uses private interpreter/frame/compiler headers; a stock shell invocation is not an equivalent replay. Original artifact hashes identify the observed build, not a fresh rerun. No Rust pass is inferred.
