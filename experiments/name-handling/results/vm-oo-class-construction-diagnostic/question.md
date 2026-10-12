# Actual C9.1 class-construction refusal

Question: what complete neutral first cause does the actual C9.1 `oo::class create` object-vector entry return in the first repaired definition-exit control?

Prior [control 1](../vm-test-api-repairs/control-01.log) reports only the generic `invoke_command` wrapper assertion. The six definition-exit failures do not expose their underlying `NativeExecutionError`; the separate active-frame setup failures cannot establish their cause.

The patch changes only the cfg(test) `class` helper in `cmd_oo/native_context/definition_exit/tests.rs`: call the existing identical `try_invoke_command` producer and expect successful actual C9.1 class construction. Preserve every argument Value factory, successful completion assertion, object resolution, callback, frame and currency assertion. This is measurement, not a semantic repair.

Scope: compile the VM lib-test target after the coordinating parent's cache/source grant, then execute only the first exact existing definition-exit control with one-test-count verification. Keep the full returned first cause and failure explicit; do not change a factory, manufacture an entry or broaden the suite. Earlier repair logs and snapshots remain immutable.
