---
name: bigip-cleanup
description: "Report BIG-IP objects not reached by the reference edges recognised in the supplied configuration, and propose a tmsh cleanup script for review. Use when inspecting possible orphaned pools / nodes / monitors / iRules / data-groups / profiles / persistence / SNAT pools in a BIG-IP config."
allowed-tools: Bash, Read
---

# BIG-IP Config Cleanup

## Steps

1. Read the config; confirm it is BIG-IP (`ltm virtual …`, `ltm pool …`).
   Given a directory, ask which files.
2. Run `f5-query cleanup FILE...` (pass `bigip.conf` and `bigip_base.conf`
   together when both exist). Options: `--keep PATH` (repeatable; a trailing
   `/` keeps a partition), `--no-keep-common` (by default `/Common/` is kept
   so factory objects survive).
3. The JSON has `candidates` (proposed deletes with `fullPath`, `kind`,
   `module`, `objectType`, `range`, `reason`), `summary` (counts by kind), and
   `tmshScript` (what the CLI prints). This is bounded by the graph edges and
   source recognised by the provider; it is not proof of a complete BIG-IP
   reference census.
4. Report the proposed counts and script verbatim in a fenced block, and say
   the order follows the provider's reported reference graph. Where supplied
   sources or unsupported/dynamic references leave completeness unresolved, say
   so explicitly. Review every line, especially references to config outside
   the supplied files and target paths. An empty candidate list means only that this analysis found no
   candidates; it does not prove every object is referenced.

## What the tool understands

- The analysis uses the root and edge types supported by the current provider.
  Reachability in that conditional source graph is not a closed-world proof of
  all BIG-IP references.
- Recognized iRule body references include `pool`, `persist`, `class match`,
  `snatpool`, `virtual`, and `node`; unsupported or dynamic effects may remain
  unknown.
- Recognized config-property references include registry-modelled members,
  nodes, monitors, rules, profiles and `defaults-from` chains, persistence, and
  SNAT pool members. This list does not imply all config properties are
  modelled. Cycles use the provider's deterministic ordering.
- Never deleted: anything outside `ltm` / `gtm`, and `ltm virtual_address`
  (BIG-IP manages it from virtual destinations). The tool only emits the
  script; the user runs it after review.

## Output

```text
Found 12 unreferenced objects:
  ltm_pool: 3
  ltm_node: 5
  ...

Run this on the BIG-IP, top to bottom:

\`\`\`tmsh
delete ltm rule /Common/old_redirect
delete ltm pool /Common/legacy_pool
...
\`\`\`
```

$ARGUMENTS
