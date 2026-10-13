# Actual shared Date-worker controls: second build attempt

The repaired-source `tcl-cmd-core` library-test build exited 101 with one E0277
at `rust/tcl-cmd-core/src/clock.rs:1610`: `Vec<Vec<u8>>` actual values cannot
compare with the fixture parser's `Vec<Cow<[u8]>>` expected values. This attempt
stopped at its first failure; no fix or rerun occurred within it. The exclusive
`tmp/name-target-runtime` cache was released immediately. Exact command,
timestamps and exit code are in [execution.json](execution.json).

```text
cargo test --offline -j2 -p tcl-cmd-core --lib --no-run --message-format=json
```

No Core native binary, native `--list` or exact Core test invocation was
reached. The two Core tests remain unexecuted. Core names in the first driver
are expected names, not observed native test names. The
[first attempt](../shared-validation-01/README.md) is independently sealed and
its seal was reverified unchanged.

After that failure, the root separately authorised the first independent
Registry library-test build: the failed Core `cfg(test)` target is separate
from Registry's normal Core library dependency. Registry compilation succeeded,
its native `--list` confirmed the fully qualified guard name, and exactly that
one control passed. [registry-execution.json](registry-execution.json) records
the actual build/list/test argv, timestamps, exit codes, observed name, count
and binary hashes. The cache was released promptly after completion.

```text
cargo test --offline -j2 -p tcl-registry --lib --no-run --message-format=json
<native registry binary> --list
<native registry binary> --exact commands::tcl::clock_::tests::private_date_worker_is_c86_only_shipped_dict_and_null_hook --nocapture
```

[summary.json](summary.json) states the result and its limits. The complete
[rendered error](first-compiler-errors.txt), raw
[Cargo JSON output](tcl-cmd-core-compile.stdout.gz) and
[stderr](tcl-cmd-core-compile.stderr.gz) retain this attempt's first failure.
[artifact-graph.json](artifact-graph.json) preserves actual Cargo dependency
artifact emissions and `fresh` flags, separately from the broader compressed
conservative tracked source/build context. MCP and iRule-test packages are
absent from the emitted Registry graph. Registry's development dependency
closure does include `tcl-irules`, `tcl-compiler` and `tcl-lsp-core`; their
artifact entries are retained without treating them as selected test targets.

The [source readiness](source-readiness.json) records the coordinated five
hashes, including the authorised Core and Runtime test-only type repairs.
All five match before and after this failed build; their gzip source images
are retained here. The original question evidence, oracle fixture, headers and
existing seal were unchanged. There is no binary hash because Cargo emitted no
completed Core test binary. The Registry binary's before/after hashes match;
its [saved gzip image](tcl-registry-native-test-binary.gz) was verified against
that same hash. Registry emitted dependencies and their `fresh` flags are in
[registry-artifact-graph.json](registry-artifact-graph.json), separately from
the conservative tracked source context captured after the Core failure and
before the independent Registry build.

The planned Core control concerns 16 original C86 successful calendar rows in
the pure helper plus two overflow refusal inputs. The Registry control concerns
C86-only surface admission, Dict type, non-pure trait and null compiler hook,
and that guard passed. Shipped backing is declared by the frozen source spec;
the guard contains no direct `runtime_backing` assertion. These scopes are
separate from physical Runtime acceptance. Earlier standalone helper results
do not replace the two unexecuted Core controls.
