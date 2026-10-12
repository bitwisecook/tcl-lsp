# Does the namespace-object consumer match its 100 selected C producer/getter vectors?

[Exact frozen test definition](consumer-test.rs) · [Scoped record](results.json) · [Build provenance](../../build-provenance/README.md)

Recorded consumer result: exit 0; [stdout](../../interp-native_namespace_names-tests-original_namespace_objects_match_100_native_c_producer_and_getter_windows.stdout) / [stderr](../../interp-native_namespace_names-tests-original_namespace_objects_match_100_native_c_producer_and_getter_windows.stderr). The supplied --exact selector was `interp::native_namespace_names::tests::original_namespace_objects_match_100_native_c_producer_and_getter_windows`.

This is a consumer test result, not a C, Jim or F5 oracle execution. The test reads [producer/getter vectors](../../../../../../../rust/tcl-syntax/tests/data/native_namespace_name/producer_getter.txt); their separate canonical question records retain provider provenance. Canonical question records:

- [namespace-native-current-root-primary](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-current-root-primary.md)
- [namespace-native-terminal-colon-original-versus-text](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-terminal-colon-original-versus-text.md)
- [namespace-native-deleted-current-object-recreation](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-deleted-current-object-recreation.md)
- [namespace-native-raw-zero-bytearray-parent](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-raw-zero-bytearray-parent.md)
- [namespace-native-opaque-original-parent](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-opaque-original-parent.md)
- [namespace-native-active-retirement-cache](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-active-retirement-cache.md)
- [namespace-native-missing-parent-cache](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-missing-parent-cache.md)

The excerpt is not a standalone Rust program: imports, helpers and fixtures belong to the reconstructed source tree. It comes from the recorded base plus archived patch, not current source. The post-build snapshot does not prove that concurrent edits were stable during compilation. No current runtime, library-initialized engine, Jim/F5 or WASM outcome follows from this unit pass.
