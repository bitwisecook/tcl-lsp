# Actual Core Date-worker controls: successful final attempt

The final frozen-source Core library-test build succeeded. Its native test list
confirmed both fully qualified control names; each exact test ran once and
passed. The already successful Registry control was not repeated. Both earlier
compile-failure evidence directories were reverified against their immutable
seals.

```text
cargo test --offline -j2 -p tcl-cmd-core --lib --no-run --message-format=json
<native core binary> --list
<native core binary> --exact clock::c86_calendar_tests::retained_c86_numeric_calendar_rows --nocapture
<native core binary> --exact clock::c86_calendar_tests::unmeasured_native_calendar_overflow_refuses --nocapture
```

[execution.json](execution.json) records actual argv, timestamps, exit codes,
observed names and one-test counts. [summary.json](summary.json) states the
outcomes and limits. The exclusive `tmp/name-target-runtime` cache was released
promptly after the checks. No wider Core, Registry or canonical suite ran.

The calendar control compared 16 retained successful original C86 worker rows
with the pure `c86_calendar_fields` / `c86_date_values` helper output bytes. It
excludes the empty-table unavailable-localtime case. The overflow control checked
`(i64::MAX, 1)` and `(i64::MAX, 0)` for `CommandProtocolUnavailable`; these are
explicit operational refusal guards for unmeasured extents, not additional
oracle-equivalence rows. These Core results are distinct from physical Runtime
acceptance and from earlier standalone helper evidence.

The five [coordinated source hashes](source-readiness.json) match before and
after the build/tests. Their gzip source images are retained. The existing
original question/fixture seal also matches before and after. The new fixture
repair normalised expected byte-container ownership; production code, oracle
bytes, assertions and the 16-row count were unchanged.

The [saved native binary](tcl-cmd-core-native-test-binary.gz) decompresses to the
recorded identical before/after binary hash. Raw compile JSON, native list and
each exact test stdout/stderr are retained as gzip files.
[artifact-graph.json](artifact-graph.json) preserves actual emitted local
dependency/target artifacts and their `fresh` flags. MCP, iRule-test and iRules
packages are absent from this Core graph. The compressed conservative tracked
source/build manifests are a separate superset; they do not claim every file
compiled.

The first independent Registry build/list/exact result remains in
[shared-validation-02](../shared-validation-02/README.md): one C86-only
availability, Dict-type, non-pure-trait and native null-hook guard passed. Its
source declares shipped backing; that guard has no direct backing assertion.
