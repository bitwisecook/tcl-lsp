---
name: bigip-cleanup
description: "Generate a tmsh script that deletes F5 BIG-IP objects unreferenced by any virtual server. Walks the full reference graph (including iRule body references — pool, persist, class match — and config-property references like pool monitors and profile fallbacks) and emits delete commands in reverse-topological order so tmsh accepts every line. Use when cleaning up a bigip.conf, finding orphaned pools / nodes / monitors / iRules / data-groups / profiles / persistence / SNAT pools, or generating a safe delete script for a BIG-IP config."
allowed-tools: Bash, Read
---

# BIG-IP Config Cleanup

Identify BIG-IP objects no longer referenced by any virtual server (or
GTM wide-IP) and produce a `tmsh` deletion script in the correct order.

## Steps

1. Read the config file the user pointed at.  Confirm it parses as a
   BIG-IP config (`ltm virtual …`, `ltm pool …`, etc.).  If the user
   pointed at a directory, ask which file(s) to analyse.
2. Run the cleanup tool:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py bigip-cleanup $FILE
   ```
   To analyse multiple files together (e.g. `bigip.conf` plus
   `bigip_base.conf`), pass them all on the command line.
3. The tool returns a JSON document with three useful fields:
   - `candidates`: the ordered list of `delete` commands plus
     per-object metadata (`fullPath`, `kind`, `module`, `objectType`,
     `range`, `reason`).
   - `summary`: candidate counts grouped by kind.
   - `tmshScript`: the ready-to-run script (also what `f5 cleanup`
     prints).
4. Present the report to the user in this shape:
   - **Headline counts** by kind (from `summary`).
   - The **`tmsh` script** verbatim, in a fenced code block, so the
     user can copy-paste directly.
   - A short call-out reminding them that the order is
     reverse-topological — referencing objects are deleted first so
     tmsh accepts each line in turn.  They should review every
     `delete` before running, especially anything that references
     external config files not in scope.
5. If the user asks to keep specific objects, re-run with
   `--keep PATH` (repeatable).  Paths ending in `/` are treated as
   partition prefixes (e.g. `--keep /Common/`).  By default
   `/Common/` is preserved so factory-shipped objects are not
   deleted; pass `--no-keep-common` if they explicitly want to scrub
   the partition too.

## What the tool understands

- `ltm virtual` and `gtm wideip *` are roots; everything reachable
  from them is kept.
- iRule bodies are parsed: `pool /Common/foo`, `persist /Common/bar`,
  `class match … /Common/dg`, `snatpool …`, `virtual …`, `node …` all
  count as references.
- Config-property references are picked up via the BIG-IP object
  registry: pool members → nodes, pool monitors → monitor profiles,
  virtual rules → iRules, virtual profiles → profiles, profile
  `defaults-from` chains, persistence references, SNAT pool members.
- Cycles (rare; e.g. `ltm rule generate` self-references) are broken
  with a deterministic tie-break so the script remains reviewable.

## Out of scope

- The tool does **not** delete anything itself — it only emits the
  script.  The user runs it on the BIG-IP after review.
- Objects outside `ltm` / `gtm` (cm, auth, sys, net) are never
  considered for cleanup.
- `ltm virtual_address` is never a candidate (BIG-IP manages its
  lifecycle implicitly from virtual destinations).

## Output format

Headline summary, then the script.  Example:

```text
Found 12 unreferenced objects:
  ltm_pool: 3
  ltm_node: 5
  ltm_monitor_http: 2
  ltm_rule: 1
  ltm_data_group_internal: 1

Run this on the BIG-IP, top to bottom:

\`\`\`tmsh
delete ltm rule /Common/old_redirect
delete ltm pool /Common/legacy_pool
delete ltm node /Common/10.1.2.3
delete ltm monitor http /Common/legacy_http_mon
…
\`\`\`

Order is reverse-topological — each delete runs only after the
objects that reference its target have already been removed.  Review
every line before applying.
```

If the report is empty, confirm that every object is referenced and
note how many virtuals were treated as roots.
