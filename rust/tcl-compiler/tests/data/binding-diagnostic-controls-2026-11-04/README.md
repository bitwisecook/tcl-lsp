# Binding diagnostic control audit, 4 November 2026

This is a bounded product audit, not a current-tip or release gate pass.
The standalone probe links the immutable production compiler library built at
`f5b2f7d05ea1a322ef8f3f018e1af04394b36477` plus patch
`da4e3730ceb54fb84cebe0801e4f7e9699041e881ce623050950d5831030b4b1`.
The library SHA-256 is
`5d1be13ab0851b1f85a83172acaa6d1ac0465a1bd51d8db594d670546c32759c`;
its complete archived dependency hashes matched before and after linking.
The link receipt retains the probe/binary/manifest hashes and exact arguments.
No Cargo target was shared or modified.

The probe supplies an explicit `ResolvedAnalysisInput`, then builds its
compilation unit with the same retained input and source settings. It runs
both analyser diagnostics and `run_all_checks` and inspects original SSA read
references. All 36 processes exited zero with a resolved input and no context
error. This audit executed no Tcl reference interpreter.

- #2296's exact input has no W220 in Tcl 8.4–9.1 or Jim profiles. The original
  nested `$k` read at bytes 34..36 retains an SSA read reference at version 1
  in all six profiles. Replacing the computed key with a literal removes that
  read, but still produces no W220. Namespace-store visibility makes that
  absence insufficient as a positive diagnostic control.
- #2297's exact input has one retained procedure and an original `$go` read,
  and no W220 in all six profiles. The independent local-unused control
  `proc p {} {set spare 1; puts ok}; p` produces no W211 in all five C Tcl
  profiles; Jim reports W211 at bytes 15..20. The quiet C result cannot certify
  full unused-store coverage.
- #2247's qualified and public forms have no W002 or W211 under Tcl 8.6, 9.0
  and 9.1. The qualified form correctly retains W143. An invented private
  subcommand reports W123 and W143 in all three profiles, so availability is
  being checked. A separate unused local in the qualified body also receives
  no W211; the W211 part remains limited by the diagnostic control above.

The three issues remain open. These observations narrow the remaining work;
they do not promote conditional source facts into Native execution or prove
all related diagnostics. `rows.json` preserves complete stdout/stderr, exact
source hashes, profiles and durations. The runner retains the fixed scripts;
it is an audit helper, not a passing CI regression.
