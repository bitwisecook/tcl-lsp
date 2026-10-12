# Do the three scoped renewals survive the F5 merge and make the source proof checker pass?

All three renewed implementation bindings survive in the merged 2,327-record
manifest and still match their two source owners. The maintained canonical
renderer checked only their three pages and changed 0; retained diffs show
those page changes. Manifest fields were preserved exactly.

The maintained Python source proof checker **fails**, with exit status
`1`. It ran once, after the three-page render check. It reports
2327 questions, 0 pending candidates,
0 unlinked tests and 315 errors.

Exact error classes:

- `evidence digest changed`: 315

[check.log](check.log) preserves the complete raw combined output;
[record.json](record.json) records the exact argument vector, status, summary
and log digest. [before.json](before.json) and [after.json](after.json) retain
the selected source, manifest, renderer and checker digests. These selected
inputs and the complete manifest bytes stayed unchanged during the checker.

This result concerns source evidence currency and coverage in the merged
working tree. It adds no oracle answer, native/Rust execution or conformance
claim. No source, proof manifest or other canonical page was edited, and no
additional renewal or checker-error repair was performed.
