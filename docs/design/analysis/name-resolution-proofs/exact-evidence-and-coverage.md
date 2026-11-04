# Historical proof inventory and coverage gate

This page described a former development process for indexing source reviews,
fixtures, build artifacts and captured observations together. That process and
its checksum-based inventory, source review and synthetic tests are retired.
The history of this page and the former tools remains available in Git; their
results are not Tcl execution evidence and do not certify coverage.

The current [Tcl execution evidence standard](../../contracts/tcl-execution-evidence.md)
keeps each executed script, its question and assumptions, and the recorded
results for its actual environments in one Markdown file under
[`evidence/tcl`](../../../../evidence/tcl/). It does not require a digest.

Run `make check-tcl-evidence` to check the required document structure and
reversible visible-byte blocks. The former `make check-name-resolution-proofs`
target is a compatibility alias for that check. The check does not determine
whether a question is adequately scoped, whether an execution happened, or
whether every historical capture has been migrated; those require review of
the recorded run. A linked test or source inspection is not an execution result.
