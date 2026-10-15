# HTTP_REQUEST event introduction

Question: Which BIG-IP release does the official F5 event reference declare for
`HTTP_REQUEST`? This supplies event-catalogue availability metadata when the
requested target predates the shared BIG-IP baseline.

The [official reference](https://clouddocs.f5.com/api/irules/HTTP_REQUEST.html)
records `Introduced: BIGIP-9.0.0` in the introduction section. The complete response
is retained in [HTTP_REQUEST.html](HTTP_REQUEST.html), line 873, with SHA-256
`b4fb441da1d5c5b20c402b7ca20534f30b5a03851f9d304739f47159b864f387`.
The F5 documentation's copyright and community-content notice remain in the
original response.

[capture.json](capture.json) retains the exact URL, HTTP response headers, capture
source hash, actual Python executable/version and before/after executable hashes,
invocation, timestamps and successful exit. [stdout](stdout) and [stderr](stderr)
retain the process channels. [capture.py.txt](capture.py.txt) is the exact executed
capture source; it can be replayed with `python3 capture.py.txt NEW_OUTPUT_DIRECTORY`.
The original invocation pathname remains in the record. The initial sandbox
network refusal and preliminary successful fetch are retained separately in
[initial-network-refusal.json](initial-network-refusal.json) and
[initial-fetch.json](initial-fetch.json). [SHA256SUMS](SHA256SUMS) covers the retained
artifacts.

This is documentation evidence, not an appliance execution observation. It does
not establish TMM event activation, command availability within the event, or any
appliance tclsh, MCPD/CLI or iApps behavior. The explicit introduction belongs to
`HTTP_REQUEST`; events without an introduction retain the shared baseline and
independent retirement data remains authoritative.
