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
| Upstream suite, when used | Suite revision, file/test IDs, exact definitions and support files, harness version/dependencies, constraints and pass/fail/skip counts with reasons |
| Raw channels | Relative stdout/stderr/log paths, process status |
| Per case | Setup/acceptance status, completion code, byte-valued result and available error options |
| Interpretation | Question answered and release/context scope; disagreement/unsupported/unavailable stated explicitly |
| Conformance mapping | Vector/fixture and consumer tests using the observation, or “not promoted” |

`local-tcl8.6.16` is an exploratory observation from an installed system
interpreter. It is outside the pinned 8.6.18 reference and does not establish
a completed C release gate or Jim/F5 agreement.

[Compiler101](compiler-production101-20261014/README.md) preserves the unexecuted linker failure; [Compiler102](compiler-production102-20261014/interruption.json) preserves the interrupted original equality assertion output. The [retained92 phase probe](compiler-production92-phase-20261014/README.md) diagnoses #2286 without semantic acceptance.

[Compiler103](compiler-production103-20261014/README.md) retains the original source producer observations. [VM104](vm-production104-20261014/README.md) retains the preflight failure and distinct actual Jim version outputs.

[VM107](vm-production107-20261014/README.md) records31/35 current naming controls with zero captured drift. [Runtime110](runtime-production110-20261014/README.md) records102/104 with original registration failures repaired and two remaining controls.

[Compiler114](compiler-production114-20261014/README.md) records208/265 ordinary naming controls with source-subset drift0 and lossless large-log retention.

[Core117](core-production117-20261014/README.md) records71/101 ordinary LSP naming controls with all92 prior planned selectors retained and source-subset drift0.
