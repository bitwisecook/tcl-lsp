# E3 trace-lane oracle artifacts (preserved from an ephemeral session)

Working artifacts from the E3 traces lane (PR #1639, merged into rust at
a75ddc144). Preserved here because the lane's session container is ephemeral
and re-deriving these is expensive.

Contents:

- `mutants/` — the 26-mutant corpus with `.from`/`.to` anchor pairs and
  `manifest.tsv`, plus the anchor-validation guard approach: a mutation whose
  anchor no longer matches is reported as UNAPPLIED instead of being scored a
  false survivor. `gen_mutants.py` regenerates the corpus.
- `a*.tcl` and companion scripts — the differential probe corpus (about 12
  scripts) exercised across three engines (real tclsh, tcl-vm, runtime).
- Captured C-oracle rows for the three POST-RELEASE trace divergence items:
  issues #1569, #1574, #1575. These were pinned byte-for-byte against
  tclsh 8.6.16 (tmp/tcl8616-install/bin/tclsh8.6 — note the PATH tclsh8.6 is
  8.6.14 and must not be used) and tclsh 9.0.4.

Whoever picks up #1569/#1574/#1575 post-release should start from these
oracles rather than re-deriving them.
