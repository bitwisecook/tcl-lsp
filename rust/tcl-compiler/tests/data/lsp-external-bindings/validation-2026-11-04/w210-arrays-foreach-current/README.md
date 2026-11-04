# Current W210 array and empty-foreach probe

`probe.rs.txt` is the standalone Rust driver linked read-only to the normal
`libtcl_compiler.rlib` named in `manifest.json`. It runs 17 retained inputs for
Tcl 8.4–9.1 and Jim, constructs `ResolvedAnalysisInput` from each resolved
profile, and prints W210 diagnostics. `probe.jsonl` is the complete 102-row
output. The driver itself does not rebuild the compiler.

The Tcl rows show the exact #2379 size/names/get examples quiet, the #2374
exact guarded distinct-element read warning, and the #2441 top-level `y`
warning while missing procedure-local `x` after the false existence guard.
Adjacent `array statistics`/`startsearch` error controls and the same-element
warning are included in the raw output. C Tcl errors for the two array
subcommands; Jim does not implement them, which is recorded separately. Jim-profile rows differ; those results are not generalized from the
Tcl profiles.

`jim-2376-probe.rs.txt` and `jim-2376-probe.jsonl` retain three focused Jim
short-circuit checks against the same rlib: safe `&&` is quiet, unsafe `||`
warns at `$x`, and the later genuine read warns at its exact span. This is not
a pass claim for the broader #2376 matrix.

The byte-exact native runtime observations are retained in the linked evidence
file. These are diagnostic baseline receipts, not issue closures.
