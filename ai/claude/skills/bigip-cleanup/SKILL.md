---
name: bigip-cleanup
description: "Generate a tmsh script that deletes F5 BIG-IP objects unreferenced by any virtual server. Walks the full reference graph (including iRule body references — pool, persist, class match — and config-property references like pool monitors and profile fallbacks) and emits delete commands in reverse-topological order so tmsh accepts every line. Use when cleaning up a bigip.conf, finding orphaned pools / nodes / monitors / iRules / data-groups / profiles / persistence / SNAT pools, or generating a safe delete script for a BIG-IP config."
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
3. The JSON has `candidates` (ordered `delete` commands with `fullPath`,
   `kind`, `module`, `objectType`, `range`, `reason`), `summary` (counts by
   kind), and `tmshScript` (what the CLI prints).
4. Report: headline counts by kind, the script verbatim in a fenced block,
   and a reminder that the order is reverse-topological (referencing objects
   go first) and every line needs review — especially anything referencing
   config outside the files given. An empty report: say every object is
   referenced and how many virtuals were roots.

## What the tool understands

- Roots are `ltm virtual` and `gtm wideip *`; everything reachable is kept.
- iRule bodies are parsed: `pool`, `persist`, `class match … DG`,
  `snatpool`, `virtual`, `node` references count.
- Config-property references come from the object registry: members →
  nodes, monitors, rules, profiles and `defaults-from` chains, persistence,
  SNAT pool members. Cycles break with a deterministic tie-break.
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
