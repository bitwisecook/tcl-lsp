# Original Jim deferred lambda namespace

`jim/stdout.tsv` was captured on the genuine full-extension Jim
0.84-9-g5bac7c9, revision `5bac7c9`, on 2026-10-14. `probe.c` invokes the
original `apply` command with a retained original integer namespace child and
samples its original `bytes` pointer before any subsequent namespace getter.
Comment-only and invalid bodies both leave that pointer absent. The other five
rows retain empty-body, entered-command and arity controls.

The SDK library SHA-256 was
`2e75f1e259dc7896fa6996ebe2ad0fb5de2864770a772a0d102b2b6306280c61`.
The [original capture manifest and provider source snapshots](../../../../../experiments/name-handling/results/original-jim-lambda-context-sdk-20261014/captures/deferred-context-followup/manifest.json)
are archived with exact-byte hashes and capture-time provenance. The source
and stdout here are exact copies of that capture. No implementation result is implied.

Reproduce against that original build with:

```sh
cc -std=c99 -I/path/to/jimtcl probe.c /path/to/jimtcl/libjim.a -lm -lssl -lcrypto -lz -ldl -o /tmp/jim-deferred-namespace-probe
/tmp/jim-deferred-namespace-probe
```
