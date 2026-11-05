# naming.each-loop.group-validation-order

Kind: `native-observation`

## Problem statement

A malformed value source and a later empty variable group compete for the first error. Choosing a body or value parser from display order alone can execute a worker that the native handler never reaches.

## Question

Which malformed-value versus empty-variable validation error is reached when the two groups are reordered, and for a single empty variable list?

## Conclusion

C returns unmatched open brace in list when the malformed value belongs to the first nonempty variable group, but foreach varlist is empty when the empty variable group is first; a single empty variable group also gives that empty-varlist error. Jim returns the empty-varlist error in all three controls. No selected control enters a body or performs an observed write. Exact after-header conversion states are retained separately and do not imply successful parsing.

## Scope

Selected cases [6, 7, 9] from ten exact direct original object-vector commands per provider. C Tcl_EvalObjv uses TCL_EVAL_GLOBAL; Jim uses Jim_EvalObjVector after core command registration. There is one explicit owning argv reference on each original root. Header primary/refcount/resident-string/backing counts and cell==member identity are sampled before result string rendering. C WRITE observation calls successful get with no READ traces installed; Jim installs no corresponding trace. Backing−1 is unobserved, never zero. No compiled foreach opcode or original document ingress claim. All original compile/processes succeed, but guest errors stay separate; original rust_comparisons0.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.4 tcl8.4.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	6	0	before	list,1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	9	0	before	none,1,1,-1	list,1,0,-1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,-1	list,1,1,-1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### tcl8.5

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.5 tcl8.5.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	6	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	9	0	before	none,1,1,-1	list,1,0,1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,1	list,1,1,1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: tcl8.6 tcl8.6; tcl9.0 tcl9.0; tcl9.1 tcl9.1.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	6	0	before	list,1,0,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	6	1	after	list,1,1,1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	6	1	756e6d617463686564206f70656e20627261636520696e206c697374
S	7	0	before	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	7	1	after	list,1,1,1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	9	0	before	none,1,1,-1	list,1,0,1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,1,1,-1
S	9	1	after	list,1,1,1	list,1,1,1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	string,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
```

### jim

Status: `observed`. Channel: Original String/List objects via public object-vector evaluator; private header snapshots before result-string getter. Version: jim Jim capture.

Exact selected S/R rows. S fields are case/sequence/window; variable-root,value-root,first-name,first-member,body,current-cell headers (primary,refs,resident,backing refs); cell==member; interpreter result header. R fields are case/guest code/result hex.

```tsv
S	6	0	before	list,1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	6	1	after	list,2,0,-1	none,2,1,-1	none,1,1,-1	absent,-1,0,-1	none,2,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	6	1	666f7265616368207661726c69737420697320656d707479
S	7	0	before	list,1,0,-1	none,1,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	7	1	after	list,2,0,-1	none,2,1,-1	absent,-1,0,-1	absent,-1,0,-1	none,2,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	7	1	666f7265616368207661726c69737420697320656d707479
S	9	0	before	list,1,0,-1	list,1,0,-1	absent,-1,0,-1	none,1,1,-1	none,1,1,-1	absent,-1,0,-1	0	none,4,1,-1
S	9	1	after	list,2,0,-1	list,2,0,-1	absent,-1,0,-1	none,1,1,-1	none,2,1,-1	absent,-1,0,-1	0	none,1,1,-1
R	9	1	666f7265616368207661726c69737420697320656d707479
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
