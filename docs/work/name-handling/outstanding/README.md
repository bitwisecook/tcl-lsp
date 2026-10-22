# Outstanding work by agent

One file per implementation agent, each maintained by that agent. Each lists what the agent considers it must still complete, what it is doing now, what is blocked and on whom, and what it finished since its last update with evidence. They are agent-reported, not validation: validated results live in [../current-state.md](../current-state.md).

| Agent | Area | File |
| --- | --- | --- |
| lsp | tcl-lsp-core/db/server, hosts | [lsp.md](lsp.md) |
| compiler | Lowering, C6, iRules realm, itcl/snit, OO | [compiler.md](compiler.md) |
| compiler-names | Name lookup in command_binding and the analyser | [compiler-names.md](compiler-names.md) |
| compiler-codegen | Compile service, codegen, pack native rows | [compiler-codegen.md](compiler-codegen.md) |
| vm-runtime | SpecTcl execution and acceptance | [vm-runtime.md](vm-runtime.md) |
| bigip-lsp | BIG-IP/F5 LSP units and editors — moved on 2026-10-22 to a separate session started by the user; pushes to `work` directly | [bigip-lsp.md](bigip-lsp.md) |
| perf | Analysis and pack-load cost — moved on 2026-10-22 to a separate session started by the user; pushes to `work` directly | [perf.md](perf.md) |
| review-triage | External review replay — moved on 2026-10-22 to a separate session started by the user; pushes to `work` directly | [review-triage.md](review-triage.md) |
| coordinator | Routing, snapshots, status, release gate | [coordinator.md](coordinator.md) |

Each file ends with a `## Resume` section (where to pick up, exact commands, where evidence is stored in the repo) so the work survives a restart.

Rules: an agent edits only its own file; it updates it whenever an item starts, finishes, changes or becomes blocked, and at least every 30 minutes; the coordinator's 30-minute WIP snapshot commits and pushes every file here, and checks that each active agent's "Last updated" is no older than an hour.

Agent notes that refer to `scratchpad/<dir>/...` paths: read the copies in [`../scratchpad-mirror/`](../scratchpad-mirror/README.md).
