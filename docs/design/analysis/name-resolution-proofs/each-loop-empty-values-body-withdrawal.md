# naming.each-loop.empty-values-body-withdrawal

Kind: `native-observation`

## Problem statement

An empty value list can make a malformed body unreachable, but an absent command can fail before loop semantics. Both may return without a body observation for entirely different reasons.

## Question

Does an empty foreach/lmap value list enter or compile the malformed original body, and which lmap command surfaces are actually available?

## Conclusion

All providers complete the empty foreach normally without write/body observations or a compiled body primary. C8.6–9.1 and Jim likewise complete the empty lmap without entering its malformed body. C8.4/8.5 instead return the measured invalid-command lmap error before supported loop behavior; no physical lmap/body rule is inferred for those releases.

## Scope

Selected cases [3, 4] from ten exact direct original object-vector commands per provider. C Tcl_EvalObjv uses TCL_EVAL_GLOBAL; Jim uses Jim_EvalObjVector after core command registration. There is one explicit owning argv reference on each original root. Header primary/refcount/resident-string/backing counts and cell==member identity are sampled before result string rendering. C WRITE observation calls successful get with no READ traces installed; Jim installs no corresponding trace. Backing−1 is unobserved, never zero. No compiled foreach opcode or original document ingress claim. All original compile/processes succeed, but guest errors stay separate; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Version: tcl8.4; exact launched patchlevel not queried by this probe. Build: library_sha256=d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; binary_sha256=c57c0b8ed17ff800234a1d331b9555e90b77725b488330d35090aa5c0e3a6864; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter.. Dialect: C Tcl.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,-1	list,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	4	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
```

### tcl8.5

Status: `observed`. Version: tcl8.5; exact launched patchlevel not queried by this probe. Build: library_sha256=99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; binary_sha256=ef65f168ca2e4baa6676414f0261a2f27a1b553439b6d5f884cb895f93f28619; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter.. Dialect: C Tcl.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,1,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	4	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
```

### tcl8.6

Status: `observed`. Version: tcl8.6; exact launched patchlevel not queried by this probe. Build: library_sha256=a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; binary_sha256=932e366c8f0fcd37de889c45adba7a67d3fb07a4b8d679e15b16a5b8c9fdf234; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter.. Dialect: C Tcl.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	4	0	
```

### tcl9.0

Status: `observed`. Version: tcl9.0; exact launched patchlevel not queried by this probe. Build: library_sha256=2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; binary_sha256=979fdd36ef05810130a8b44fccaaf7444434d82988eb80ee42396fdb653da32c; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter.. Dialect: C Tcl.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	4	0	
```

### tcl9.1

Status: `observed`. Version: tcl9.1; exact launched patchlevel not queried by this probe. Build: library_sha256=8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; binary_sha256=7f76b4f8e359ea0a472f5dfd27f88cc8924f77ec2b46ac708bf3f405f0d4d1b9; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter.. Dialect: C Tcl.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	4	0	
```

### jim

Status: `observed`. Version: Jim capture; patchlevel, revision and UTF configuration unrecorded. Build: library_sha256=75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; binary_sha256=3a8e584b0528ded18c1d2e5bc5769c42ca46afb3f709bab2a8356b8fd0c87f01; compile_exit=0; exit=0. Compiler/version/configuration fields absent from this original receipt remain unknown.. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter.. Dialect: Jim Tcl.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,-1	list,1,0,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	3	1	after	list,1,0,-1	list,1,0,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,5,1,-1
R	3	0	
S	4	0	before	list,1,0,-1	list,1,0,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	4	1	after	list,1,0,-1	list,1,0,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	list,1,0,-1
R	4	0	
```

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No original observation for this exact question is attached for this provider.

## Exact evidence

- `probe` (input): [rust/tcl-cmd-core/tests/data/native_each_loop/original/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/probe.c). SHA-256 `9151561b7aa1309982ecbdb44e436e575fa8c65e01abff2a21ac03725bbe98d7`. Exact native object constructors, direct evaluator flags, observer windows and ten case layouts.
- `receipt` (provider): [rust/tcl-cmd-core/tests/data/native_each_loop/original/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/manifest.json). SHA-256 `6331df842ad47fb5610d98174f248ce32b3c657f35736ef95afb0c1405e1a22c`. Actual original per-provider compilation/process/output hashes and explicitly limited header scopes.
- `rows-tcl8.4` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt). SHA-256 `42230e280a24132e1770f25498854c4046c923091565d94c3c42215497fe12f8`. Full original stream; selected exact case IDs [3, 4].
- `rows-tcl8.5` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt). SHA-256 `6fe04f828f139082f99d09280c7b6219216a2839797353d767f7cc928b407369`. Full original stream; selected exact case IDs [3, 4].
- `rows-tcl8.6` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt). SHA-256 `aae7374777c02c90e356049fc6f510a9bb3bad413a8e8e37d281a8d6571ab948`. Full original stream; selected exact case IDs [3, 4].
- `rows-tcl9.0` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt). SHA-256 `aae7374777c02c90e356049fc6f510a9bb3bad413a8e8e37d281a8d6571ab948`. Full original stream; selected exact case IDs [3, 4].
- `rows-tcl9.1` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt). SHA-256 `aae7374777c02c90e356049fc6f510a9bb3bad413a8e8e37d281a8d6571ab948`. Full original stream; selected exact case IDs [3, 4].
- `rows-jim` (observation): [rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt). SHA-256 `f513876ec1aee700ec831bdc53e569afa873bd5d45e46fca7261a78b18f2c6d2`. Full original stream; selected exact case IDs [3, 4].

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_each_loop.rs](../../../../rust/tcl-cmd-core/src/native_each_loop.rs), `EachLoopState::advance`: Retains selected generic group-validation, assignment and body schedule separately from physical original headers.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_120_original_native_completions` (linked): Compares the selected cases within 120 exact original/shimmer guest completions; no new native launch or test pass is asserted.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_406_original_native_physical_windows` (linked): Compares the selected cases within406 original/shimmer header windows; unavailable backing fields remain sentinels and no compiled opcode parity is claimed.

A named test is a coverage binding, not a claim that it executed.

## Replay

No new native run or Rust execution result is claimed. Replay must compile the retained exact probe against independently identified release headers/libraries and preserve separate compile/process status and raw streams before comparing the attached original hashes. Original absolute compile paths are capture metadata, not a portable executable command. No evaluator/source/parser or native allocation authority may be obtained by matching display text. Probe code is input/observer code; absent native source excerpts supply no implementation explanation.
