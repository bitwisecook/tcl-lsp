# Results

Lib-test compilation passed (exit 0, 111.699 seconds). The retained 26 diagnostics across nine test owners are resolved. Historical baseline remains unmeasured.

Exact command: `source /tmp/name-env.sh`; `CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues cargo test --offline -p tcl-vm --lib --no-run -j 2 --message-format=json-render-diagnostics`.

[build-record.json](build-record.json), [lib-test-build.jsonl](lib-test-build.jsonl) and [lib-test-build.stderr.txt](lib-test-build.stderr.txt) retain the exact argv, cache, elapsed time, exit and Cargo-emitted executable. Binary SHA256: `d7c76fd5701410f6869b38c2ce1dd929684c2964fb13e5eb55e1420ecddd0a47`.

All 29 requested names exist in that actual binary. Each exact invocation executed one test, with no timeouts: 18 passed, 11 failed. [controls-results.json](controls-results.json) retains every exact argv, one-test count, exit, duration and full log path; [controls-summary.json](controls-summary.json) records the total.

Both specifically required controls passed: [original complete earlier Index Host cause](control-17.log) and [embedded-NUL duplicate constructor](control-18.log). Both C84 integer-result controls, all three repaired grouped-sort/comparator controls, the genuine OriginalPackage helper control, observed-input cache-retirement control and actual arena pin-retirement control passed. These are bounded software controls, with their original independent Native provenance unchanged.

| Failed controls | First observed boundary | Scope reached |
| --- | --- | --- |
| [1](control-01.log), [2](control-02.log), [3](control-03.log), [4](control-04.log), [5](control-05.log), [6](control-06.log) | `exec.rs:9170`: `command requires a genuine native expression provider` assertion | Public OO class setup/invocation refuses; the scalar panic does not identify the underlying typed refusal. Intended callback/restoration assertions are not established. |
| [14](control-14.log), [15](control-15.log), [16](control-16.log) | Setup `eval_source` returns `Host(Execution(ValueAccessRefusal(CommandProtocolUnavailable("original OO Name target/facet"))))` | Intended OO callback/epilogue controls remain unreached. |
| [22](control-22.log) | `cmd_binary.rs:1254`: retired loan `resident_string_bytes().is_some()` assertion | Actual hex decoding, exact full retired-header refusal and scope first-cause equality passed before this unchanged assertion failed. |
| [23](control-23.log) | `cmd_binary.rs:1444`: initial authored binary format returns `Error`, expected `Ok` | Scan, none-entry and later retired-header controls remain unreached; this log does not expose the format's detailed cause. |

[Source snapshots](source-build-before.json) cover 4,943 Rust source, Cargo manifest/lock and toolchain files, including current shared compiler/callback/clock owners. The [post-build snapshot](source-build-after.json) matches; the same scope and binary hash remained unchanged after controls. This scope excludes fixtures, external dependencies and raw F5 data and is not a whole-dependency identity claim. All ten declared repair-owner hashes also matched before compilation.

Direct rustfmt check passed for the ten changed source files with edition 2024 and skip_children=true; scoped git diff whitespace check passed. The original failed log remains unchanged. No Rust edit was made after the source freeze; no broad suite, extra Cargo build, Git mutation, network or raw F5 read was performed.

The compile boundary is repaired. The 11 runtime failures remain explicit; no test assertion was relaxed or replaced. Next work needs separately coordinated genuine OO entry/Name-facet and binary lifetime/authored-format diagnosis. The [read-only Native ingress diagnosis](native-ingress-diagnosis.md) remains independent of these 29 results.
