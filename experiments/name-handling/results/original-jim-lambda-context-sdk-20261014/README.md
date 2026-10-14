# Original Jim lambda context SDK observations

These original SDK captures identify the genuine full-extension Jim build
`0.84-9-g5bac7c9` observed on 2026-10-14. The provider revision is fixed at
capture time; no claim is made about a later upstream revision. The original
static library SHA-256 is
`2e75f1e259dc7896fa6996ebe2ad0fb5de2864770a772a0d102b2b6306280c61`.

- [Initial deferred namespace capture](captures/lambda-deferred-namespace/manifest.json): actual empty, return and namespace-current bodies.
- [Preflight capture](captures/lambda-preflight/manifest.json): actual empty/nonempty bodies and missing required arguments.
- [Comment and invalid-body follow-up](captures/deferred-context-followup/manifest.json): original integer namespace stays without resident String bytes when command lookup is unreached.
- [Qualified-context capture](captures/qualified-context/manifest.json): original command/variable and qualified lookup controls, with original bytes sampled before the later explicit getter.

Every directory retains the original `probe.c`, stdout, stderr, compiler output,
manifest and identified provider source snapshots without byte changes.
[archive-receipt.json](archive-receipt.json) records each source and archived hash.
Original executable hashes remain in the captured manifests and were checked at
archival time; executable binaries and static libraries are not copied here.
`verify.py` checks every retained artifact without needing the original `/tmp`
paths. Those paths remain historical provenance inside the unchanged manifests.

Reproduce one capture against the same original full-extension Jim SDK build:

```sh
cc -std=c99 -I/path/to/jimtcl captures/deferred-context-followup/probe.c /path/to/jimtcl/libjim.a -lm -lssl -lcrypto -lz -ldl -o /tmp/jim-lambda-context-probe
/tmp/jim-lambda-context-probe
```

The probes observe original interpreter objects and getter timing. They do not
prove implementation success, Native compiler admission, C Tcl behaviour or
full Jim suite conformance.
