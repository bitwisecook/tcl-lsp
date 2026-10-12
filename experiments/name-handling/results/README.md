# Oracle result records

Each run has its own directory containing input identity, provenance,
stdout/stderr, exit status and decoded case outcomes. Use names identifying
oracle, exact version/revision and context. Never reuse `latest` as a durable
identity or overwrite older runs.

A result document records:

| Field | Required content |
| --- | --- |
| Oracle | C Tcl/Jim/F5 and exact version/revision/build |
| Context | Global interpreter, proc/namespace frame, TMM event, CLI script, iApp, or other exact context |
| Input | Stable case IDs, script paths, SHA-256 and invocation |
| Raw channels | Relative stdout/stderr/log paths, process status |
| Per case | Setup/acceptance status, completion code, byte-valued result and available error options |
| Interpretation | Question answered and release/context scope; disagreement/unsupported/unavailable stated explicitly |
| Conformance mapping | Vector/fixture and consumer tests using the observation, or “not promoted” |

`local-tcl8.6.16` is an exploratory observation from an installed system
interpreter. It is outside the pinned 8.6.18 reference and does not establish
a completed C release gate or Jim/F5 agreement.
