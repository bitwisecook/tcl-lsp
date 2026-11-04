# Read-local W210 attempt — incomplete

Frozen base `f4f99e4ed3612afeb42189842fd5c3a865f93203` plus the archived
unverified patch (SHA-256
`50752e8872979c4a70c5265c1f65a2e4da1b6598dcdac2ade7bc111814b8f2f3`).
The combined compiler unit/integration build passed in 4m19s. All 41,094 present
tracked files and the patch were unchanged through runtime.

The actual production analyser passes all 105 diagnostic checks and 70 original
source-value checks across Tcl 8.4–9.1. This includes uncalled-procedure safe
AND/negated-OR/double-negation, active unsafe guards, distinct scalar/element
controls and the later genuine read's exact span.

The owner suite passes 6/7 tests. Its new thirteen-case guard test stops at
`[info exists a(safe)] && $a(safe) > 0`: the expected same-element guard is
unavailable. The following other-element and read-trace owner controls have not
run. The product draft remains stashed and uncommitted pending diagnosis.
No failing CI test or unverified production change is installed by this receipt.
#2376 remains open; full release gates remain pending.
