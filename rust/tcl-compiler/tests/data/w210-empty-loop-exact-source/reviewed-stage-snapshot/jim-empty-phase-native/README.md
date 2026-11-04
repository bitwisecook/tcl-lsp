# Jim empty-loop source-phase native controls

These are actual native runs, not a compiler fix receipt. The two canonical
human-readable records are:

- [Source phases](../../../../../../../evidence/tcl/w210-empty-foreach-source-phase-controls-2026-11-04.md).
- [Observer boundary](../../../../../../../evidence/tcl/w210-empty-foreach-observer-boundaries-2026-11-04.md).

All six successful runs have exit 0 and empty stderr. Each JSON capture records
its exact script, interpreter path/hash, version observation and raw streams.
The first observer attempt is retained: it used the removed `trace variable`
form directly, so Tcl 9.0/9.1 emitted errors and supplied no variable-trace
observations. The final script uses the accepted modern registration form with
an old-engine fallback. Jim's missing `trace` interface and the C engines'
missing `xtrace` interface remain explicitly unavailable.

Read-only current-library traces show that generic foreach object preparation
withdraws Jim's authored source world before the empty loop exit. The existing
AuthoredJim literal-pool receipt deliberately permits only text effects, not
C-style object conversion or physical object facts. A proposed stock Jim source
phase must preserve that distinction. No production change or passing compiler
assertion is claimed here. The separate conditional Jim body-shape gap remains
open. Existing C5 stage verification is unchanged.

## Paused implementation handoff

The user requested stopping after the next completed issue and final branch
integration. The three-file private Jim draft is preserved only as
`unbuilt-jim-empty-phase.patch.txt`, based on `68df69c92`; its digest and
remaining obligations are in `unbuilt-jim-empty-phase.json`. The user then clarified that all current work should be pushed. The exact
archived draft has therefore been reapplied as an explicitly UNBUILT
development checkpoint. No build, test or lint has run on this draft, and no
claim is made that it compiles or resolves either Jim gap.

The draft adds a separate stock-Jim empty-iteration schema, plus a source-only
consumer after argument/observer handling and the existing implementation
check, before generic object preparation. It requires selected authored Jim
source and original literal iterator operands, refuses conditional/unknown
entries, observers and substituted iterator objects, and preserves only the
post-argument state without issuing a Normal, physical, binder/body or compiler
receipt. This boundary needs review and focused positive/refusal tests. The
exact original report also ends in a top-level failing read, so restoring an
empty-loop world alone may still leave its independent conditional Jim body
shape gap. Keep #2441 open until both exact reads and active controls pass.
