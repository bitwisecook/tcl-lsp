# Results

The rebuilt VM lib-test target compiled successfully (exit 0, 108.996 seconds) against the restored current compiler/shared sources. The single requested exact control exists in the Cargo-emitted executable and executed exactly one test; exit 101, no timeout. This is a diagnostic failure, not a semantic repair.

The [full control log](control-01.log) exposes the exact neutral first cause at `definition_exit/tests.rs:32`:

```text
actual C9.1 class construction: ValueAccessRefusal(CommandProtocolUnavailable("original OO Name target/facet"))
```

This proves the first definition-exit control's actual class-construction refusal is a Name target/facet refusal. The old expression-provider wrapper wording did not identify its variant. The control's later successful object resolution, callback/restoration and currency assertions remain unreached; this result does not establish the remaining five wrapper failures' typed causes.

[build-record.json](build-record.json), [Cargo receipt](lib-test-build.jsonl), [build stderr](lib-test-build.stderr.txt), [exact name check](control-list.log) and [control-record.json](control-record.json) retain commands, cache, source scope, actual executable, one-test count and exits. The emitted binary SHA256 is `e6ec3c6a39e504dd3e879639b59907584ee32b5644b5bb92984491dad063bcf4` and remained unchanged during the control.

The [before](source-build-before.json)/[after](source-build-after.json) snapshots cover 4,943 current Rust source, Cargo manifest/lock and toolchain files, including the restored compiler and relevant shared dependencies; no scoped source changed through build/control execution. This scope excludes fixtures, external dependencies and raw F5 data, and is not a whole-dependency identity claim.

Only the cfg(test) class helper's invocation API changed. Every Value factory, argument, Code::Ok/object-resolution assertion and later object/frame assertion remains. Direct rustfmt/scoped whitespace checks passed. No other test ran, no Rust edit occurred after freeze, and the issues-cache lease was released immediately after this control. The prior 18-pass/11-fail logs and source snapshots remain untouched.

The exact failed Name-owner predicate remains unanswered: its shared purpose covers unavailable original target capture or a failed retained-target currentness check. Production repair requires separately coordinated source/oracle diagnosis of that boundary; changing a factory or relaxing an assertion is unsupported by this measurement.
