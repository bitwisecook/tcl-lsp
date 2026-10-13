# Actual shared Date-worker controls: first compile failure

The first actual `tcl-cmd-core` library-test build exited 101. No test binary,
native test list or exact test execution was reached. The registry build and all
three planned controls remain unexecuted. No source fix or test rerun occurred.
The exclusive `tmp/name-target-runtime` cache was released on failure; the
release timestamp is in [execution.json](execution.json).

The captured command was:

```text
cargo test --offline -j2 -p tcl-cmd-core --lib --no-run --message-format=json
```

The five errors in `rust/tcl-cmd-core/src/clock.rs` are two E0277 comparisons
between `Cow<[u8]>` and byte-array references at line 1589, E0658 `as_slice` at
1593, and E0308 `Cow`/`Vec` type mismatches at 1601 and 1606. The complete
[rendered errors](first-compiler-errors.txt) and raw
[Cargo JSON output](tcl-cmd-core-compile.stdout.gz) /
[stderr](tcl-cmd-core-compile.stderr.gz) preserve the first failure.

[summary.json](summary.json) states the result and limits.
[artifact-graph.json](artifact-graph.json) records the actual Cargo dependency
artifacts emitted before failure, including their `fresh` flags, and separately
identifies the failed library-test target. No MCP or iRule package was emitted
in that graph. The compressed conservative source-context manifests contain a
broader tracked source/build superset; they do not claim each file compiled.

The accepted five-source hashes match the supplied v2 readiness record before
and after the failed build. Their gzip source images are retained here. The
original [question evidence](../../get-date-fields-question/README.md), including
its fixture output, matches its existing seal before and after this attempt.
No fixture or existing source header was changed.

The planned names in the execution record are expected names, not observed
native `--list` results. The calendar control would compare 16 retained original
C86 successful rows through the pure numeric helper; its two overflow guards
would exercise operational refusal. The registry control would check C86-only
surface admission, Dict result, non-pure trait and null native compiler hook.
None ran. Previous standalone helper results and separately owned Runtime tests
are distinct evidence and are not substituted for these actual controls.

[executed-driver.py.gz](executed-driver.py.gz) preserves the exact driver used.
[run.py](run.py) differs only by its subsequently added AGPL licence header and
has not been rerun. The capture driver stopped at the first actual failed build.
