# Does explicit native-core deletion preserve the global namespace across entangled command traces?

[Exact frozen test definition](consumer-test.rs) · [Scoped record](results.json) · [Build provenance](../../build-provenance/README.md)

Recorded consumer result: exit 0; [stdout](../../cmd_namespace-tests-mutually_entangled_deletion_preserves_the_global_namespace.stdout) / [stderr](../../cmd_namespace-tests-mutually_entangled_deletion_preserves_the_global_namespace.stderr). The supplied --exact selector was `cmd_namespace::tests::mutually_entangled_deletion_preserves_the_global_namespace`.

This is a consumer test result, not a C, Jim or F5 oracle execution. The distinct [direct Tcl question](../entangled-deletion-tcltest/README.md) retains its own oracle observations. 

The excerpt is not a standalone Rust program: imports, helpers and fixtures belong to the reconstructed source tree. It comes from the recorded base plus archived patch, not current source. The post-build snapshot does not prove that concurrent edits were stable during compilation. No current runtime, library-initialized engine, Jim/F5 or WASM outcome follows from this unit pass.
