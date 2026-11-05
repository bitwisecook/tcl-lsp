# naming.each-loop.original-lmap-collection

Kind: `native-observation`

## Problem statement

A generic lmap appends body results while assigning original input members. Assuming ordinary foreach empty publication would lose the two entered body results and their native List result header.

## Question

What result and body/member windows are produced by lmap over two original values?

## Conclusion

C8.6–9.1 and Jim enter two bodies and return normal result bytes BODY BODY with a List result before rendering. The first body v cell is the original first member; the second is a different member and the first member is refs1 after its replacement. C8.4/8.5 reject lmap by the measured invalid-command error and supply no supported collection windows.

A marked VM control binds missing C8.4/C8.5 lmap lookup, constructed result String shape and shared backend result-header publication.

## Scope

Selected cases [5] from ten exact direct original object-vector commands per provider. C Tcl_EvalObjv uses TCL_EVAL_GLOBAL; Jim uses Jim_EvalObjVector after core command registration. There is one explicit owning argv reference on each original root. Header primary/refcount/resident-string/backing counts and cell==member identity are sampled before result string rendering. C WRITE observation calls successful get with no READ traces installed; Jim installs no corresponding trace. Backing−1 is unobserved, never zero. No compiled foreach opcode or original document ingress claim. All original compile/processes succeed, but guest errors stay separate; original rust_comparisons0.

This software same-header assertion does not extend original case4's public result/cache window into an observed external pointer identity or arbitrary result-header guarantee. It observes no loop body, entered lmap handler, element effect, compiled loop opcode or original process. Whole native each-loop observations remain separately scoped; no passing assertion is attached.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.4 tcl8.4.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	5	0	before	list,1,0,-1	list,1,0,-1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	after	list,1,1,-1	list,1,1,-1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	5	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
```

### tcl8.5

Status: `unsupported`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.5 tcl8.5.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	5	0	before	list,1,0,1	list,1,0,1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	after	list,1,1,1	list,1,1,1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	5	1	696e76616c696420636f6d6d616e64206e616d6520226c6d617022
```

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	5	0	before	list,1,0,1	list,1,0,1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	5	1	write	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,2,1,-1	none,1,1,-1	none,2,1,-1	1	none,2,1,-1
S	5	2	body	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,2,1,-1	bytecode,2,1,-1	none,2,1,-1	1	none,1,1,-1
S	5	3	write	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	none,2,1,-1	0	none,3,1,-1
S	5	4	body	list,1,0,2	list,1,0,2	parsedVarName,1,1,-1	none,1,1,-1	bytecode,2,1,-1	none,2,1,-1	0	none,1,1,-1
S	5	5	after	list,1,0,1	list,1,0,1	parsedVarName,1,1,-1	none,1,1,-1	bytecode,1,1,-1	absent,-1,0,-1	0	list,1,0,1
R	5	0	424f445920424f4459
```

### jim

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: jim Jim capture.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	5	0	before	list,1,0,-1	list,1,0,-1	none,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	5	1	body	list,2,0,-1	list,2,0,-1	variable,2,1,-1	none,2,1,-1	script,3,1,-1	none,2,1,-1	1	none,7,1,-1
S	5	2	body	list,2,0,-1	list,2,0,-1	variable,2,1,-1	none,1,1,-1	script,3,1,-1	none,2,1,-1	0	none,7,1,-1
S	5	3	after	list,1,0,-1	list,1,0,-1	variable,2,1,-1	none,1,1,-1	script,1,1,-1	absent,-1,0,-1	0	list,1,0,-1
R	5	0	424f445920424f4459
```

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [rust/tcl-cmd-core/src/native_each_loop.rs](../../../../rust/tcl-cmd-core/src/native_each_loop.rs), `EachLoopState::advance`: Retains selected generic group-validation, assignment and body schedule separately from physical original headers.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_120_original_native_completions` (linked): Compares the selected cases within 120 exact original/shimmer guest completions; no new native launch or test pass is asserted.
- [rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs](../../../../rust/tcl-vm/src/cmd_control/native_each_loop_tests.rs), `cmd_control::native_each_loop_tests::generic_each_loops_match_all_406_original_native_physical_windows` (linked): Compares the selected cases within406 original/shimmer header windows; unavailable backing fields remain sentinels and no compiled opcode parity is claimed.

- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::command_lookup_error_value`: Use the actual reached original head and selected reporting/result-producer protocol for lookup failure; table key extent remains independently selected.
- [rust/tcl-vm/src/exec.rs](../../../../rust/tcl-vm/src/exec.rs), `Vm::publish_native_lookup_failure`: Publish failed lookup through the same backend interpreter completion owner despite the absence of an entered-handler epilogue.
- [rust/tcl-vm/src/exec/native_missing_command_result_tests.rs](../../../../rust/tcl-vm/src/exec/native_missing_command_result_tests.rs), `exec::native_missing_command_result_tests::original_missing_lmap_lookup_publishes_the_same_native_result_header` (linked): Selected C8.4/C8.5 software cores receive original List arguments and an unbalanced-brace body, then missing lmap lookup returns Error. The completion and actual backend interpreter result share its constructed String header, and counted bytes equal invalid command name "lmap". No loop handler/body is entered.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-cmd-core/tests/data/native_each_loop/original/probe.c](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/probe.c)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/manifest.json](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/manifest.json)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.4.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.5.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl8.6.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.0.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/tcl9.1.txt)
- [rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt](../../../../rust/tcl-cmd-core/tests/data/native_each_loop/original/jim.txt)
