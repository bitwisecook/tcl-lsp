# Nested array index reads

The exact `$arr($cmd)` probe runs unchanged in original Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned JimTcl. Each interpreter reads both `cmd` and `arr`: changing the index variable selects a different element; independently missing index and array values produce their corresponding errors. Raw channels and executable/library identity are recorded in results.json. No tcltest harness is used.

Conformance mapping: `taint::tests::arg_var_names_vs_vars_in_word::agree_on_var_nested_in_array_index` requires both shared inventories to retain both dependencies. Compiler114 retains the prior arr-only comparison failure; its result is not overwritten. This observation does not establish F5 behaviour.
