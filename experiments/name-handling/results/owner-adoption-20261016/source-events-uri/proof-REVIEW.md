# Additive SourceWhen proof proposal

Official record read from current manifest:
`naming.consumer.original-irules-source-context`.
`proposal-records.json` is existing updater-compatible schema_version/proofs input.
It contains the entire proposed replacement record, not a delta schema invented
for this task. `record.diff` is the bounded JSON review. `record-before.json`
preserves the exact current record read before proposal generation.

Six actual owner bindings, four exact new source-control selectors and four new
role=implementation whole-file hashes are appended. Evidence IDs follow the
existing current source-catalogue implementation style. No provider/observation
artifact or execution result is appended. Prior evidence prefix (empty), old
owners/test prefixes, every old linked/unlinked status, all seven provider
answers/status/evidence, source anchors, replay and record identity are exact.
Conclusion and scope retain their complete old text and append only the bounded
SourceWhen source-purpose contract. No historical hash was refreshed.

Implementation hashes were computed from the integrated, root-formatted main
files, not the earlier TMP candidate. `source-bindings.json` pins those exact
files and the before-record digest. It binds source context/card/inventory,
source API ingress, actual exports and explain consumer/source controls.

`contract-prose.patch` adds one paragraph to the existing iRule event-report
paragraph in docs/design/contracts/name-handling.md. It describes unavailable
versus owned/partial/complete-empty inventory, original source cards and retained
mutation obligations. It grants no installed handler, frame, TMM execution,
Native identity or broader executable-inventory permission. It includes no
transient development/build-status statement and adds no document/ID/count.

Read-only schema_errors validation succeeded against the actual current proof
schema; assertions verified all historical prefixes and preserved fields.
Contract patch dry-run passed. The official updater was not imported/invoked,
no proof/contract/main file was written, and no Cargo/build/test or Git action
was performed. Parent should verify this record still matches record-before
before replacing it via the official updater; unrelated 89MB-manifest records
must not be overwritten from this snapshot.
