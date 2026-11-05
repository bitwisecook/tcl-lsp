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

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.4 tcl8.4.

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

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.5 tcl8.5.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	3	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	3	1	after	list,1,0,1	list,1,1,1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	3	0	
S	4	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	4	1	after	list,1,1,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	4	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
```

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

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

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: jim Jim capture.

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

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_each_loop.rs](../../../../rust/tcl-cmd-core/src/native_each_loop.rs), `EachLoopState::advance`: Retains selected generic group-validation, assignment and body schedule separately from physical original headers.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_120_original_native_completions` (linked): Compares the selected cases within 120 exact original/shimmer guest completions; no new native launch or test pass is asserted.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_406_original_native_physical_windows` (linked): Compares the selected cases within406 original/shimmer header windows; unavailable backing fields remain sentinels and no compiled opcode parity is claimed.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_each_loop/original/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/probe.c)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/manifest.json)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt)
