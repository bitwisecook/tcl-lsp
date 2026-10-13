# Compiled array iterator storage and original var-13.2

The production Runtime completes the unchanged C Tcl 8.6.18 `var-13.2`
selection with the matching original library and Tcltest harness: total 110,
passed 1, skipped 109, failed 0. The C Tcl 8.6.18 shell returns the same counts.
This is one measured case; skipped tests establish no acceptance.

## Question and cause

Does removing an element invalidate the original array search without aborting
its containing compiled procedure?

Both the unchanged upstream wrapper and its separately extracted 161-byte body
initially abort with `realloc(): invalid pointer`. The diagnostic stack resolves
to `ArrayIteration::advance`, then `obj::change_type`, then
`obj::shrink_string_to_exact`. The abort occurs during the initial six-pair
`array set`, before `array startsearch` or `unset` executes.

C Tcl's `INST_FOREACH_START` and `INST_FOREACH_STEP` use two private empty
object headers for iterator counters and a borrowed auxiliary pointer
(`generic/tclExecute.c:7008–7154`). Runtime plain String objects instead store
buffer capacity in `internal_rep`. Putting the iteration counter there causes
the third pair to treat the previous count, 2, as allocation capacity and shrink
canonical static empty storage to one byte.

The Runtime adapter retains the two empty private headers and keeps iterator
metadata in Rust fields, as its compiled foreach adapter already does. Physical
search ownership and central naming policy are unchanged. This repair removes
a validation dependency; it supplies no evidence of consumer-wide name lookup
centralisation.

## Observations

| Channel | Runtime | C Tcl 8.6.18 |
| --- | --- | --- |
| Unchanged original harness, baseline | SIGABRT; no final counts | 1 passed, 109 skipped, 0 failed |
| Unchanged original harness, diagnostic preload | SIGABRT; bounded stack captured | 1 passed, 109 skipped, 0 failed |
| Unchanged original harness, repaired | Exit 0; 1 passed, 109 skipped, 0 failed | Exit 0; same counts |
| Extracted original body, baseline diagnostic | SIGABRT during compiled array set | Error: `couldn't find search "s-1-a"` |
| Deterministic regressions | 2 tests passed | Measured fixed pair-count outputs in C 8.6.18 and 9.0.4 |

The original body regression checks its error result and
`TCL LOOKUP ARRAYSEARCH s-1-a` error code. The pair-count fixture checks actual
array contents for fixed 0, 1, 2, 3, 6 and 8-pair compiled applications against
separately captured C 8.6.18 and 9.0.4 outputs. No other release, Jim or F5 result
is asserted. Required repository-wide gates have not run for this repair.

## Evidence and replay

- `capture.py baseline` and `capture.py repaired` execute the unchanged
  [original wrapper](../runtime-c86-real-suites/array-search-isolation/var-13.2/wrapper.tcl).
  Labels must name fresh directories. Each capture verifies all six original
  input hashes and the wrapper against the original receipt, snapshots actual
  runner dep-info inputs, records executable hashes and bounded process results,
  and verifies compiled and harness inputs remain unchanged during execution.
- `baseline/`, `traced-baseline/` and `repaired/` retain distinct raw stdout,
  stderr, compiled-input snapshots, process receipts and file seals. Neither
  baseline directory is rewritten by repaired execution.
- `original-body.tcl` preserves the exact upstream body bytes; `reduced-catch.tcl`
  is a separate result/error-code diagnostic. The extracted body is not the
  original-suite execution channel.
- `realloc-trace.c`, `diagnostic-record.json`, `baseline-symbols.txt` and
  `reduced-baseline.stderr.txt` retain bounded diagnostic instrumentation, build
  identity and the resolved stack. The preload records the abort stack; it does
  not issue a lookup witness or supply semantic oracle expectations.
- `conformance.tcl`, `conformance-record.json` and `c-*.stdout.txt` retain the
  fixed pair-count oracle inputs and outputs. C test expectation execution uses
  neither Tcltest nor a substituted harness.
- `regression.log` and `runner-build.log` retain the focused test and runner
  build output. Existing dead-code warnings remain explicit in those logs.

Regression selector:
`cargo test --manifest-path runtime/rust/Cargo.toml --test native_array_search_unset`.
Use the configured libtommath source and the matching Runtime build environment.
