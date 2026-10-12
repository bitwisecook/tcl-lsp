# Does the native consumer retire parent commands before visiting every child namespace?

[Exact frozen test definition](consumer-test.rs) · [Scoped record](results.json) · [Build provenance](../../build-provenance/README.md)

Recorded consumer result: exit 0; [stdout](../../cmd_trace-tests-parent_teardown_reaches_each_child_after_parent_commands.stdout) / [stderr](../../cmd_trace-tests-parent_teardown_reaches_each_child_after_parent_commands.stderr). The supplied --exact selector was `cmd_trace::tests::parent_teardown_reaches_each_child_after_parent_commands`.

This is a consumer test result, not a C, Jim or F5 oracle execution. The separate [parent/child Tcl input](../../../parent-child-observation.tcl) and [lifecycle report](../../../RESULTS.md) retain the C controls. 

The excerpt is not a standalone Rust program: imports, helpers and fixtures belong to the reconstructed source tree. It comes from the recorded base plus archived patch, not current source. The post-build snapshot does not prove that concurrent edits were stable during compilation. No current runtime, library-initialized engine, Jim/F5 or WASM outcome follows from this unit pass.
