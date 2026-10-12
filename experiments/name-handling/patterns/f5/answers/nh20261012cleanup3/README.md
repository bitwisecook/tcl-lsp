# Final owned staging and configuration checks

All eight exact staging paths were removed after verifying their retained capture seals. The actual remote path is anchored by a captured input-hash/load transcript; the standalone provenance files identify the run without a separate staging field. Each cleanup rejects symlinks, lists regular files recursively, removes only that exact owned path and checks its absence. Every final operation returned zero. Remote content hashes were not remeasured.

The initial local attempt stopped at a missing provenance field before any remote cleanup operation. Its exact helper and failure receipt are retained. The completed helper and actual outputs are retained independently.

Ten final configuration listings returned zero and contain only `listing-exit=0`, with no matching task prefix. The filter covers the probe and temporary-partition prefixes in rules, virtuals, pools, templates, application services, CLI scripts, iCall scripts/periodic handlers, folders and partitions. Full unfiltered listings and connection configuration are omitted. No configuration was saved.

These receipts establish exact staging absence and matching-prefix absence in those ten listing scopes at capture time. Runtime cell cleanup is supported separately by each run; neither earlier unowned interpreter state nor native catalogue retirement is inferred. [receipts.json](receipts.json) retains every command, seal/source hash and actual exit code.
