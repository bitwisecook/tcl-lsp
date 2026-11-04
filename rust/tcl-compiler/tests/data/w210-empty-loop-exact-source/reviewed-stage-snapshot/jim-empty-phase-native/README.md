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
