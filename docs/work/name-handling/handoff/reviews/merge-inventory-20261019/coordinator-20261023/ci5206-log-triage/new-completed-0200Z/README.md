# CI 5206 newly completed logs at 02:00 UTC

This supplement preserves the exact connector-decoded log bytes. Each gzip was decompressed and compared byte-for-byte to its supplied file. Hashes are in `hash-manifest.json`. It leaves previous ledgers unchanged.

| Scope | Reached runtime outcomes | Exact baseline PASS joins | Current absent from baseline |
| --- | --- | --- | --- |
| Main1 job120175508573 | 238 PASS, 39 FAIL, 3 SIGTERM, 1 SIGKILL; 281/10552 reached | 158 PASS; 26 nonpass (details distinguish original signal status) | 80 PASS, 17 nonpass |
| Heavy job120175508112 | 312 PASS, 42 FAIL; all354 reached | 293 PASS, 18 FAIL | 19 PASS, 24 FAIL |
| Native partition3 job120178740920 | 536 PASS, 292 FAIL; all828 reached | 467 PASS, 272 FAIL | 69 PASS, 20 FAIL |
| Native partition2 job120178741001 | 560 PASS, 274 FAIL; all834 reached | 495 PASS, 257 FAIL | 65 PASS, 17 FAIL |

All four scopes executed tests; no build/list failure was observed. Repeated terminal display rows were deduplicated by exact binary identity and selector, with zero conflicting statuses. Partition2/3 share zero observed identities. Exact PASS/FAIL/signal selector lists and per-job outcome JSON retain the original status. Full comparisons are losslessly compressed `*-exact-baseline-join.json.gz`.

Main1 left10,271 planned tests unreached and2 unnamed tests skipped; those are unknown/skip, never failures. Signal causes remain unknown. Native partitions skipped1709/1703 out-of-partition tests respectively. The heavy baseline is the authenticated separate312-test raw log, not the main24312-selector baseline. Missing baseline identities remain separate from regressions; one heavy baseline selector is absent from current.

The `test-ext` aggregate job120186673586 ran no tests: channel=success, partitions=failure, multi=failure, exit1. It adds no independent test failure. No whole-CI, release-gate or shipping admission follows from this supplement.
