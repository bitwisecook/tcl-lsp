# Does the native consumer validate all delete operands before retirement and relookup after callbacks?

[Exact frozen test definition](consumer-test.rs) · [Scoped record](results.json) · [Build provenance](../../build-provenance/README.md)

Recorded consumer result: exit 0; [stdout](../../interp-native_namespace_names-tests-namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks.stdout) / [stderr](../../interp-native_namespace_names-tests-namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks.stderr). The supplied --exact selector was `interp::native_namespace_names::tests::namespace_delete_validates_every_original_before_retirement_and_relooks_after_callbacks`.

This is a consumer test result, not a C, Jim or F5 oracle execution. Canonical question records:

- [namespace-native-delete-validates-later](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-delete-validates-later.md)
- [namespace-native-delete-dependent-child-duplicate](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-delete-dependent-child-duplicate.md)
- [namespace-native-delete-unset-trace-later-effects](../../../../../../../docs/design/analysis/name-resolution-proofs/namespace-native-delete-unset-trace-later-effects.md)

The excerpt is not a standalone Rust program: imports, helpers and fixtures belong to the reconstructed source tree. It comes from the recorded base plus archived patch, not current source. The post-build snapshot does not prove that concurrent edits were stable during compilation. No current runtime, library-initialized engine, Jim/F5 or WASM outcome follows from this unit pass.
