# Historical196 original #2286 debugger observations

These are two captures of the same retained historical196 compiler-test executable,
SHA-256 `a7fcd9a1d03f738d9434012f81e145c458039c54923161bf97330ce9a57e371a`,
running the unchanged exact selector
`analyser::diagnostics::tests::tcltest_oracle_program_keeps_missing_reads_and_setup_completion`.
The selector analyses retained original Tcltest source under all five C profiles;
it does not execute an external C interpreter. This record is not a C oracle run.

| Capture | Recorded outcome | Scope |
| --- | --- | --- |
| [permitted](permitted/result.json) | PASS, inferior exit 0 in 65.040215 seconds; valid receipt | Original selector under the debugger; no requested stack sample was reached before completion. |
| [repeat2](repeat2/result.json) | 180.035864-second host timeout; valid receipt | Same executable, unchanged original assertion and 180-second budget; samples at 55, 100 and 160 seconds. |

Both captures retain executable, fixture and provider-file hashes. Original
[start](repeat2/started.json), result, [raw MI stream](repeat2/gdb-mi.log.gz),
[100-second stack](repeat2/stack-100.json), [160-second stack](repeat2/stack-160.json)
and runners are copied byte-exact. Receipt `raw_mi_sha256` values hash the compressed
`.gz` file bytes. [Archive manifest](archive-manifest.json) records every original
path, byte count and SHA-256 without rewriting a historical receipt.

The repeated capture's CPU counters show the active test thread accumulating
approximately one CPU-second per wall-second. Its 100/160-second stacks include
`ssa::represented_contents_origins`; the later stack passes through joins of
phi-input origins. These observations locate CPU-bound SSA origin work during the
timeout. They do not establish a particular oscillating equation graph or a final
root cause. The passing first capture and timed-out repeat remain separate outcomes.
No assertion, input, five-profile loop or time budget was changed.

The [historical196 build receipt](../consumer196-preflight-20261014/result.json),
[compiled source hash map](../consumer196-preflight-20261014/source-before.json.gz)
and [ordinary201 receipt](../tcltest-production201-20261014/result.json) retain their
original hash identities and independent scopes. Main changed before these debugger
runs; the original receipt scope records concurrent Root203 build activity as a
limitation. Debugger pauses count within the
180-second host budget. These runs provide no current-source acceptance, comparable
ordinary-run performance result, provider completion or semantic closure for #2286.

The 236,889,048-byte ELF is deliberately excluded; [copy metadata](elf-copy.json)
retains its identity. The runners preserve their original absolute `/tmp` and
`/workspace` paths, debugger/library paths and environment protocol. They are
records of the original invocation, not portable entry points. Replay needs the
same historical executable or an independently verified rebuild, original source
and build closure, fixture, provider trees, debugger and runtime libraries. The
source hash map is an identity inventory, not a self-contained source archive.
Missing outside paths are portability limits, not evidence that a current rebuild
has the same behaviour. No original receipt or canonical proof digest is renewed.
