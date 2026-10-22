# Plain-Tcl rewrite report — F5 change comparison

This check retains the existing test
`current_diagnostics::tests::actual_direct_and_push_rewrites_retain_their_captured_document_source`.
It opens version 7 of `file:///rewrite-wire-captured-source.tcl`, sets
`optimiser.profile = full`, requests a direct diagnostic report, then runs
the push provider and compares captured rewrite source associations.

```tcl
proc p {y} {
    set x [llength $y]
    puts $x
}
```

The with-F5 build at `aeecf397` fails the existing assertion
`actual worker must retain an applicable rewrite` before reaching the push
report comparison (55.84 seconds). Receipt: `rewrite-with-f5-20261022.txt`.

For the comparison, `/tmp/bigip-without-f5.py` saves exact original bytes
of `rust/tcl-lsp-server/src/{current_diagnostics.rs,lib.rs}`, checks reverse
application of `c9dc3c289285f548626237a4caa41bd93e6b6c9a` for only these two
files, applies those reverse patches, then builds and runs the unchanged
assertion. Its finally block restores both originals and prints SHA-256s.
The generic test/assertions are outside that F5 patch and stay present.

Comparison result: **FAIL at the identical applicable-rewrite assertion**,
12.43 seconds, 1 selected test (839 filtered), with the generic assertions
unchanged. Receipt: `rewrite-without-f5-20261022.txt`. Both exact original
files were restored, their SHA-256s printed, both executables deleted, and
`git diff` on the two server paths is empty. The three editor-generator
comparison tests passed 3/3 in the same wrapping command.

The without-F5 build selects both `tcl-lsp-server` and `xtask`, which
unifies their dependency features; the earlier with-F5 receipt selected
server alone. The exact Cargo commands are retained in the receipts. This
records the requested failure observation without the F5 source change;
a controlled feature-identical bisection is a separate check.

The shared build also runs the three editor generated-region tests; it
builds, runs and deletes every test executable in one command. Environment:
CARGO_INCREMENTAL=0, CARGO_PROFILE_TEST_DEBUG=0,
CARGO_PROFILE_DEV_DEBUG=0, TCL_LSP_REQUIRE_TCLSH=9.0,8.6 and verified pinned
Jim 0.84, Tcl 9.0.4/8.6.18. Source: `aeecf397` plus owned lint-only changes;
no later coordinator compiler/provider changes are part of this receipt.

If the assertion fails identically without F5, route the existing minimal
fixture to generic diagnostic/compiler rewrite owners. Required contract:
when applicable rewrite advice is produced, direct and completed push
reports must carry the actual offered document's source association,
including its URI, version and original ranges. The present test also
requires at least one applicable rewrite for this full-optimiser fixture;
its owner must resolve that expectation against current compiler output,
without weakening assertions merely to make a gate green. This comparison records persistence of the failure without the F5 change;
it does not identify the missing rewrite-producer step.
