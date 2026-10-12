# Lifecycle argument offset

Does the declaration graph select the original setup, body and cleanup scripts
when its CapturedLifecycle grammar belongs to a registry subcommand with a
nonzero argument_offset?

The fixed Rust regression defines an authored offset-lifecycle/run schema using
the existing Tcltest lifecycle grammar, and analyses input.tcl with the trusted
stock provider fixture. The graph must schedule puts CLEANUP, puts BODY and
puts SETUP from the reordered options. This is a compiler-only registry fixture;
no native offset-lifecycle command or execution observation is claimed.

The fix applies the same checked phase argument_offset adjustment used by the
existing scoped lifecycle body accessor. Overflow retains unknown body coverage.
No physical read, entered frame, Normal completion or edit grant is added.

source-before.json and declaration-flow-before.rs retain the pre-edit owner;
source-frozen.json and declaration-flow-frozen.rs retain the tested owner and
three exact owner hashes. The other two unchanged owner files are also retained.
The receipts delimit these three owners, command/environment overlays and exact
streams; they do not claim a complete transitive build/source snapshot or a
native oracle capture. Compiler controls below remain distinct checks.

Observed compiler/consumer checks on the frozen owners:

| Check | Passed | Failed |
| --- | ---: | ---: |
| Nonzero-offset phase-script regression | 1 | 0 |
| Tcltest filter | 6 | 3 |
| Logical namespace filter (four namespace controls plus one storage-facet control) | 5 | 0 |
| Correctly qualified retained-original import test | 1 | 0 |
| Packaged imported Tcltest LSP consumer | 2 | 0 |

The three Tcltest failures remain the C8.4 missing-read controls: actual zero,
expected 1/1/2. They were not repaired in this question. An earlier incorrect
module-qualified filter selected zero tests and is retained as an empty
selection, not a validation pass. The corrected intended test ran once.
The nonzero-offset regression was not run on the pre-edit owner, so no baseline
execution result is invented.

The coordinator reports HEAD changed during these builds due only to
documentation/proof commits. All three scoped Rust owner byte hashes remained
identical to source-frozen.json before and after the checks; no identical-HEAD
claim, full gate result, new native oracle observation or complete compiler
parity is asserted. The issues-cache lease was released after the packaged run.
