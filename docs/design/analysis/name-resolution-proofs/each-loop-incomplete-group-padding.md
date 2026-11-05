# naming.each-loop.incomplete-group-padding

Kind: `native-observation`

## Problem statement

Two variables over three original values enter the body twice. The final assignment to v can release the first original member while the newly assigned cell has a distinct role; a global member-count guess would conflate both windows.

## Question

What body count, original-member identity and sampled roles follow a two-variable/three-value foreach group?

## Conclusion

Every original capture enters the body twice and completes normally with an empty result. The first v cell is the original first member, at refs2 in the body; the second body follows a different v member, while the first member is retained only by the original value list at refs1. The C write observer samples refs3 on the selected current cell. Jim has no authored write trace and samples root header refs2/body script refs3 while entered. The unobserved w padding cell is not assigned an inferred identity by these snapshots.

## Scope

Selected cases [2] from ten exact direct original object-vector commands per provider. C Tcl_EvalObjv uses TCL_EVAL_GLOBAL; Jim uses Jim_EvalObjVector after core command registration. There is one explicit owning argv reference on each original root. Header primary/refcount/resident-string/backing counts and cell==member identity are sampled before result string rendering. C WRITE observation calls successful get with no READ traces installed; Jim installs no corresponding trace. Backing−1 is unobserved, never zero. No compiled foreach opcode or original document ingress claim. All original compile/processes succeed, but guest errors stay separate; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.4 tcl8.4.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	2	0	before	list,1,0,-1	list,1,0,-1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,-1	list,1,0,-1	parsedVarName,1,1,-1	none,3,1,-1	none,1,1,-1	none,3,1,-1	1	none,1,1,-1
S	2	2	body	list,1,0,-1	list,1,0,-1	parsedVarName,1,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	1	none,1,1,-1
S	2	3	write	list,1,0,-1	list,1,0,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	none,3,1,-1	0	none,1,1,-1
S	2	4	body	list,1,0,-1	list,1,0,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	5	after	list,1,0,-1	list,1,0,-1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
```

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.5 tcl8.5; tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	2	0	before	list,1,0,1	list,1,0,1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	2	1	write	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,2,1,-1	none,1,1,-1	none,2,1,-1	1	none,2,1,-1
S	2	2	body	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	1	none,1,1,-1
S	2	3	write	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,2,1,-1
S	2	4	body	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,1,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	2	5	after	list,1,0,1	list,1,0,1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	2	0	
```

### jim

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: jim Jim capture.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	2	0	before	list,1,0,-1	list,1,0,-1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	2	1	body	list,2,0,-1	list,2,0,-1	variable,2,1,-1	none,2,1,-1	script,3,1,-1	none,2,1,-1	1	none,8,1,-1
S	2	2	body	list,2,0,-1	list,2,0,-1	variable,2,1,-1	none,1,1,-1	script,3,1,-1	none,2,1,-1	0	none,9,1,-1
S	2	3	after	list,1,0,-1	list,1,0,-1	variable,2,1,-1	none,1,1,-1	script,1,1,-1	absent,-1,0,-1	0	none,8,1,-1
R	2	0	
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
