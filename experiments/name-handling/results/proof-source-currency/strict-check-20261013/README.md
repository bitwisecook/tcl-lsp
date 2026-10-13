# Strict naming proof check

The exact strict command and complete output are retained in `record.json` and
`check.log`. It exits1 with1739 errors over2606 questions/7069 candidates. The
errors comprise769 evidence bindings,952 retained scrub census hashes,2 missing
test catalogue bindings,4 missing document assertions and12 census path changes.
`changed-evidence.json` lists the actual question/path pairs requiring review.
No source/evidence hash or census classification was renewed.

The checker ran in the shared checkout while two coordinated test-only repairs
and new experiment files were added. This is a diagnostic snapshot, not an
immutable full-tree receipt or green gate. Review each current contract and its
actual acceptance before changing a proof binding. Scrubbed transport images
must remain distinct from historical execution seals; preserve the explicit
[current transport projection](../../../../../scripts/dev/bigip-probes/resolution-2286/evidence/TRANSPORT_PROJECTION.md)
and never restore removed sensitive input. Newly retained paths require actual
classification and provenance rather than automatic promotion.
