# Repository-wide remaining-consumer audit

The complete number of remaining naming consumers is **not yet established**. The earlier 27-function snapshot, 330-row audit and grouped work items are bounded records, not a repository-wide total.

The current audit is bound to commit `cc24c394004fc187378ba0eefb6c2c5b2615fb00`. Its complete Git tree has **110,709 tracked paths**, each assigned exactly once; independent reconciliation found no missing paths, overlaps or extras. These include retained original providers, fixtures and archives; the file count does not measure migration work.

| Review area | Tracked paths | Scope |
| --- | ---: | --- |
| Compiler | 4,312 | Compiler except target emitters; lexer, syntax, dialect, version, core types, bytecode, regex |
| Registry and database | 17,486 | Registry, DB, SpecTcl, hooks, Studio and spec metadata |
| LSP and editors | 1,203 | Core, server, browser/WASI transports, iRules and all editor integrations |
| Runtime and hosts | 10,074 | Runtime, shared command/runtime/engine interfaces, platform, hosts, sandbox, SslicTcl |
| VM and code generation | 1,537 | VM/CLI/WASM, compiler emitters, BPF, explorer and diagrams |
| CLI, debugger, MCP and AI | 344 | Tcl/F5 CLIs, packages, debugger, MCP, XC and AI templates |
| Cross-cutting audit | 75,753 | Remaining first-party crates, generators, tooling, grammars, documentation, tests and retained evidence |

Each first-party production surface is being reviewed for actual naming purposes and owner transport. An inventory or text search does not close review. Original provider trees, test data and archives receive explicit provenance-based dispositions; their harnesses and adapters still need source review. Generated surfaces trace their generators and inputs.

Each remaining site will record its file, symbol, source hash, purpose and context, existing shared API, missing integration, evidence, related controls and validation status. Implementation work, migrated code awaiting validation and verified integrations are separate. Unexamined areas remain explicit and prevent a complete total.

The working file roster, partition check, reviewer coverage and consumer rows are retained under `/workspace/name-session/full-consumer-audit/`; a portable reconciled ledger will replace this in-progress snapshot. The named baseline remains fixed while subsequent source changes receive separate invalidation/review records. Required repository gates remain unfinished.

The first consolidated review checkpoint contains583 provisional purpose rows, including shared owners and display-only sites. All recorded source SHA256 values match actual baseline Git blobs; coverage partitions reconcile and no same-key duplicates were found. This does not establish a remaining-consumer count: unexamined production, multi-symbol groups and independent purpose reconciliation remain unfinished. The portable checkpoint is indexed in [handoff/packets.json](handoff/packets.json).
