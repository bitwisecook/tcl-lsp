# Analyser and optimiser at HEAD over the 14 Tcl-area fixtures

The audit agents measured the analyser through the cached MCP binary (built 19 November), which lags the source. This directory re-runs every fixture of the fourteen area directories on the source at `b0311765`:

- `analyser-optimiser-head.tsv`: `D` rows list each fixture's diagnostics (`code@line:message`) under `tcl8.6` and `tcl9.0`; `O` rows list the optimiser rewrites applied (non-hint, non-overlapping) under each profile.
- `rewrites-executed.tsv`: every rewritten fixture run before and after under the matching tclsh (8.6 or 9.0); `DIFF` rows whose only difference is the script name in an error trace are not real.
- `probe.rs`: the temporary test (append to a `tcl-compiler` test module importing `Analyser`; `PROBE_DIRS`, `PROBE_OUT`; opt-level 2).

Jim fixtures are not included: there is no `jim` dialect profile for the analyser (`DialectProfile::find("jim")` is `None`); the Jim page used the server's `jimtcl` dialect.
