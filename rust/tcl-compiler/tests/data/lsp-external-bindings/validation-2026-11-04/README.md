# External binding report validation, 4 November 2026

Product source: `b7b653dd2bf0ca65edfbc1a322bab4dcc768ba42`.
The fresh isolated build used `b71a4b31224b012b0d4bf0ed6d69fbede767dde1`,
whose only difference is `docs/LSP_RELEASE_BINDINGS.md`. No draft compiler
changes were present. The earlier W210 implementation work remains stashed.

The production API measured here is `Analyser::analyse`. This is a compiler
diagnostic check, not an editor transport run or the upstream packages' full
test suites. The complete linked source files and quoted reproductions are in
the parent directory. Their upstream Git blobs, SHA-256 digests and licence
notices are retained there.

## Results

| Issue | Exact reported input result | Controls and limits |
| --- | --- | --- |
| #2286 | Four quoted programs with their Tcltest import context, and the complete 124,908-byte linked file, produce no W210 in all five Tcl profiles. | The unchanged historical Tcltest program still emits 0 W210 instead of 2 in every profile. Direct and captured-alias genuine missing-read controls emit 0 instead of 1. Standalone W210 controls pass. This does not establish correct shared-frame diagnostics. |
| #2289 | Complete 13,125-byte linked file and the quoted `New` procedure followed by a call produce no W140 for `vectorCoreChild`, Tcl 8.4–9.1. | The never-created-interpreter control emits W140 in each profile. Real C Tcl runs the reduced creator/call and returns `VALUE`; the full RBC suite was not executed. |
| #2290 | Complete 34,272-byte current file produces no W210 for `data` or `type`, Tcl 8.4–9.1. | A procedure-local missing-`data`/`type` control emits both warnings in every profile. The current upstream file changed on 19 October; the September source was not identified. |
| #2501 | Exact 219-byte example produces no W210 for `start`; a reached-loop variant also has no W210 under Tcl 8.5–9.1. | Standalone missing-local controls warn. A reached, procedure-local variant with `lassign` removed still misses the genuine `start` warning. Tcl 8.4 has no `lassign`. |
| #2502 | Exact 286-byte example produces no W210 for `radius` or `bg` under supported Tcl 8.6–9.1. | Standalone missing-local and isolated missing-`bg` controls warn. A procedure-local variant with unpack removed still misses the genuine `radius` warning. Tcl 8.4 lacks `dict`, 8.5 lacks `lmap`, and Jim rejects the example's `min(...)` expression. |

There are 60 completed original-input/profile measurements in
`observations.json`, including ten positive diagnostic controls. The separate
genuine-read controls above are retained as limitations, not counted as passes.
None of these external issues is closed by this evidence.

## Commands and receipts

The following command built the compiler and test binary successfully in
2 minutes 3 seconds. Its test execution passed the standalone controls and the complete five-profile
#2286 test before interruption during #2289; it is not a complete six-test pass.

```sh
source /tmp/lsp-release-env.sh
source scripts/dev/agent-build-env.sh
python3 /tmp/lsp-heavy-build.py cargo test --offline -p tcl-compiler \
  --test lsp_external_binding_reports -- --nocapture --test-threads=1
```

The already-built test binary then completed the other five tests:

```sh
target/debug/deps/lsp_external_binding_reports-18065bf0a9ef6fd8 \
  --nocapture --test-threads=1 \
  --skip issue_2286_tcltest_setup_body_and_cleanup
```

Result: **5 passed, 0 failed, 1 filtered out**, in 109.34 seconds. The same
frozen compiler library independently repeated the Tcl 9.1 full-file/snippet
#2286 measurements through the small public-API driver retained as
`probe-driver.rs.txt`. Raw logs, original oracle outcomes, additional control
inputs and command/source/binary hashes are retained beside this README.

All twelve original frozen input and test-harness file digests match before
and after validation. `source-and-commands.json` records the build snapshot,
product-source equivalence, input hashes and binary digests. The complete
original sources were not edited to obtain the results. Licence metadata was
added separately without changing the source inputs or test harness.

The real C Tcl 8.4–9.1 Tcltest program prints `direct {1 0}`,
`captured-alias {1 0}`, `stopped-setup {0 1}`, `opaque-result 0`, and
`counts 4 3 1`. Its one deliberately failing setup case is expected;
cleanup runs and the unreachable body stays unentered. Jim reaches the direct
case but rejects this program's `interp alias` interface. Real C Tcl 8.6–9.1
and Jim also confirm missing `start` and `radius` errors in the procedure-local
controls with their binders removed.

`cargo fmt --all --check` passed; its empty output and exit status are recorded separately. The full `make rust-check`
and `make prep-pr` gates were not completed for this validation checkpoint;
the earlier gate failures and pending broader checks remain in the release
tracking document. No full-suite or editor-path pass is inferred.

Raw log whitespace is preserved exactly, including Cargo's unfinished progress
line at interruption and its final blank line.
