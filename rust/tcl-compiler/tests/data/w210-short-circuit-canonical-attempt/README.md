# #2376 canonical consumer review

This uncommitted candidate was tested at `fef11c63a` after a complete isolated
target clean. All 41,048 present tracked files plus the integration driver
matched their frozen hashes through compilation and runtime. The exact patch,
driver, input cases, executable digest and raw outputs are retained here.

All 60 strict diagnostic checks pass across Tcl 8.4–9.1. The verbatim report is
quiet, unsafe and distinct-cell reads still warn, and the later genuine read
moves from the skipped `$x` at 54..56 to the final `$x` at 79..81.

The second test ran all 70 cases but failed its blanket unavailable-result
expectation: six canonical cases gain final `set x` results in all five
profiles. All 30 values match their native final values. `SourceNormalResult`
is the value on all represented Normal alternatives; its owner explicitly
grants neither a physical object nor an absence-of-error proof. The separate
compiler-admission, source-purpose and totality gates still require comparison,
as do whole-expression results that must not borrow raw child bytes. This is a
review blocker, not a waived guard or a verified product fix.

The combined no-run build exited 101 because nine existing `include_str!` uses
referenced five authentic experiment fixtures omitted by the sparse checkout.
It emitted the integration executable, which ran directly outside the wrapper.
The five fixtures were restored only after the runtime freeze closed, using
their exact HEAD Git blobs; private model tests await the next allocated build.
Full `rust-check` and `prep-pr` remain pending. No candidate code or failing CI
test is installed by this evidence checkpoint.
