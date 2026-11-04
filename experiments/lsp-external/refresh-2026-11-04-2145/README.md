# External compiler regression refresh

All 11 existing regression tests pass against production source `ad4d43160`
plus the exact reviewed stage patch `e663f580…`, now published in `02a9c35c6`.
The generated harness includes the existing surface, taint and full linked
binding-report modules. It links read-only against the matching completed
compiler libraries; no Cargo or target writes occur. The runtime takes
273.15 seconds. All 145 dependency files, native archives, test inputs and the
compiled binary retain their before/after hashes. Commands and hashes are in
the link and runtime receipts; the complete diagnostic stream is retained.

The surface and taint groups use Tcl 8.6 and retain genuine unsafe controls.
Binding observations cover Tcl 8.4–9.1, with active W210 and W140 controls.
The report programs remain quiet for their asserted diagnostics. This does
not establish reached coverage of every reported body: unsupported version
rows retain their availability warnings, the original lassign example may
skip its loop, and the separate reached-read/Tcltest controls recorded in the
external proof comments remain gaps. No partial issue is promoted to fixed,
and no native LSP or Jim revalidation is claimed by this run.

The generated harness and raw commands retain the original absolute paths.
For a new checkout, run the three named existing integration targets against
the recorded product revision with an isolated Cargo target. The manifest
pins the exact source bytes and dependencies used by this capture.
