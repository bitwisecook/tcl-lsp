# Does explicit native-core child teardown distinguish cached namespace objects from fresh lookup?

[Exact frozen test definition](consumer-test.rs) · [Scoped record](results.json) · [Build provenance](../../build-provenance/README.md)

Recorded consumer result: exit 0; [stdout](../../cmd_namespace-tests-child_teardown_distinguishes_cached_names_from_fresh_lookup.stdout) / [stderr](../../cmd_namespace-tests-child_teardown_distinguishes_cached_names_from_fresh_lookup.stderr). The supplied --exact selector was `cmd_namespace::tests::child_teardown_distinguishes_cached_names_from_fresh_lookup`.

This is a consumer test result, not a C, Jim or F5 oracle execution. The distinct [direct Tcl question](../cached-versus-fresh/README.md) retains its own oracle observations. 

The excerpt is not a standalone Rust program: imports, helpers and fixtures belong to the reconstructed source tree. It comes from the recorded base plus archived patch, not current source. The post-build snapshot does not prove that concurrent edits were stable during compilation. No current runtime, library-initialized engine, Jim/F5 or WASM outcome follows from this unit pass.
