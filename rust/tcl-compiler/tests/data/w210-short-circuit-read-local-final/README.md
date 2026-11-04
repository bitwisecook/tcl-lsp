# Read-local W210 verification

Frozen source is `ab8d0742efe1271e3c3f28d4af78be9bd526d2ea` plus patch
`f138ec24802376e44737181ecd7d7bbcc2b9f9a252831022381ba4be9ea838eb`.
All 41,817 file hashes and the patch are unchanged through build and runtime.
The combined unit/integration build passes in 5m14s.

The focused owner suite passes 7/7 tests, including fourteen original-read
guard cases. All 185 production integration rows pass: 115 diagnostic checks
across Tcl 8.4–9.1 and seventy original source-value/admission checks. Uncalled
procedure guards are quiet; unsafe OR/negated-AND and different-cell/element
controls remain active. A later genuine read retains its exact span. Original
query/read geometry and canonical cell identity are required, and intervening
effects expire inherited predicates. No Native/Normal producer is changed.

The additional existing query-owner suite is 6/8. Its two strict-query
must-None assertions also fail identically in the preserved pre-read-local
canonical-consumer unit binary. These are baseline failures requiring owner
review; no assertion is weakened or ignored. Their helper uses default
Standalone source-analysis options, independently of the actual analyser-input
guard regressions. Raw current and prior failures are retained.

Production Clippy is blocked by an unchanged nested cache type at
`command_binding/retained_source.rs:78`. That file is outside this patch.
Full `rust-check` and `prep-pr` remain pending under the user-authorised
checkpoint exception. #2376 remains open for coordinator review of these
baseline causes and this receipt.

Native array evidence and the corrected MayBound fixture boundary are in
[`w210-short-circuit-array-boundaries`](../w210-short-circuit-array-boundaries/README.md);
the five exact executed scripts also have standalone `evidence/tcl` records.
Earlier canonical result/whole-expression/totality guards remain covered.
