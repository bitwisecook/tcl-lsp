# Focused upstream name-suite observations

Exact original Tcl9.0.4 tests were run with `--match`, one selected ID per fresh
process, using [the retained capture runner](../../run_suites.py). C/runtime
receive the configure-and-source script over stdin; VM receives the original
absolute upstream file and `--match`. Entire source files are read, including
unconditional helper definitions and file-level setup; only matching test bodies
run. These are focused suite selections, not rewritten reproductions.

The first capture started at 2026-10-12T10:42:23Z. The last result files were written
at 2026-10-12T10:43:06+00:00 (UTC filesystem capture time). Individual provenance records
retain capture start, exact library/Tcltest/overlay/source/input/binary hashes,
command, and measured process duration. At most two case captures ran at once,
with three engine processes per case.

C controls and native runtime loaded real upstream init.tcl and Tcltest2.5.11
from `/workspace/tcl-lsp/tmp/tcl9.0.4/library`; their raw NH_HARNESS records
include the Tcltest source loader. VM uses its real-library initialisation in
`run_test`; its stdout does not emit NH_HARNESS, so bootstrap provenance comes
from the separate VM startup capture and the retained binary identity.

| Selected ID | Engine | Status | Passed/failed/skipped | Seconds | Observation |
| --- | --- | --- | --- | --- | --- |
| namespace-56.2 | c | pass | 1/0/313 | 0.027 | Selected test passes. |
| namespace-56.2 | runtime | process-error | —/—/— | 2.677 | Uncaught invalid command name set; no summary. |
| namespace-56.2 | vm | process-error | —/—/— | 11.167 | Uncaught invalid command name list; no summary. |
| rename-6.1 | c | pass | 1/0/18 | 0.021 | Selected test passes. |
| rename-6.1 | runtime | pass | 1/0/18 | 0.541 | Selected test passes. |
| rename-6.1 | vm | process-error | —/—/— | 8.851 | Uncaught empty-string boolean error; no summary. |
| trace-34.1 | c | pass | 1/0/289 | 0.023 | Selected test passes. |
| trace-34.1 | runtime | fail | 0/1/289 | 2.09 | Extra leavestep callback after remove/readd of execution traces. |
| trace-34.1 | vm | process-error | —/—/— | 11.688 | Uncaught empty-string boolean error; no summary. |
| oo-6.19 | c | pass | 1/0/387 | 0.028 | Selected test passes. |
| oo-6.19 | runtime | pass | 1/0/387 | 3.325 | Passes both focused selection and whole-file execution. |
| oo-6.19 | vm | process-error | —/—/— | 10.378 | Uncaught empty-string boolean error; no summary. |
| ooUtil-1.7 | c | pass | 1/0/32 | 0.019 | Selected test passes. |
| ooUtil-1.7 | runtime | process-error | —/—/— | 0.478 | Uncaught restore of puts collides with existing ::puts; no summary. |
| ooUtil-1.7 | vm | fail | 0/1/32 | 9.054 | Test reports return code1 rather than allowed0/2. |
| next-tailcall-simple-1 | c | pass | 1/0/61 | 0.02 | Selected test passes. |
| next-tailcall-simple-1 | runtime | process-error | —/—/— | 0.731 | Uncaught restore of puts collides with existing ::puts; no summary. |
| next-tailcall-simple-1 | vm | pass | 1/0/61 | 9.202 | Selected test passes. |
| interp-2.13 | c | pass | 1/0/354 | 0.027 | Selected test passes. |
| interp-2.13 | runtime | pass | 1/0/354 | 2.933 | Passes focused selection. |
| interp-2.13 | vm | process-error | —/—/— | 11.511 | File-level child cleanup errors: could not find interpreter "a x2"; no summary. |

All seven C selections passed exactly one test. The remaining upstream IDs
are counted as skipped because of the exact filter, not because their semantics
were measured or declared unsupported. Engine process errors have no final
summary and must not be counted as zero failed tests. Every directory retains
stdout, stderr, record.json, input.tcl, provenance.json and the capture runner.

The runtime has three passing selections, one assertion failure and three
process errors. The VM has one passing selection, one assertion failure and
five process errors. No selection timed out. A passing capture command only
means evidence was saved; statuses above describe conformance outcomes.

The namespace-56.2 destruction failure, runtime execution-trace mismatch,
runtime puts restoration collisions, and VM errors reproduce independently of
preceding selected test bodies. Runtime oo-6.19 and interp-2.13 pass both
in isolation and whole-file execution. Their containing files have other
failures or unfinished coverage. Preserve whole-file evidence; these passes do
not explain the separate whole-file interp timeout at interp-34.2.

No implementation, upstream init/Tcltest procedure, expected result or semantic
source was changed for these measurements. Raw results support future reduced
witnesses; distinguish interpreter behavior from any Tcltest bootstrap failure
before assigning an implementation cause.
