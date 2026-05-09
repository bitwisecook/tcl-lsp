# KCS: feature — BIG-IP Config Cleanup

> **Audience:** User
> **Type:** Functionality

## Summary

Generate a `tmsh` script that deletes BIG-IP objects unreferenced by any virtual server, ordered so each delete runs only after the objects that reference its target have been removed.

## Applies to

VS Code, tcl-lsp CLI, Claude skill

## Question

How do I find and remove BIG-IP objects that are no longer used by any virtual server?

## How to use

The cleanup feature parses one or more `bigip.conf` / SCF files, treats every `ltm virtual` and `gtm wideip *` as a graph root, walks the forward reference graph (object-property references plus iRule body references — `pool`, `persist`, `class match`, `snatpool`, `virtual`, `node`), and emits `tmsh delete` commands for everything unreachable.  Output order is reverse-topological: a referencing object is always deleted before the object it references, so `tmsh` accepts every line in turn.

### VS Code

1. Open a `bigip.conf` (or any file with the `BIG-IP Config` language id).
2. Run **Tcl: Generate BIG-IP Cleanup Script** from the command palette, or pick it from the editor context menu.
3. The extension opens two side-by-side documents:
   - the `tmsh` script with the delete commands,
   - a JSON metadata report with per-object `kind`, `range`, and `reason`.
4. Review every line, then paste the script into a `tmsh` shell on the BIG-IP.

### tcl-lsp CLI

```
python -m explorer.f5_cli cleanup samples/bigip/bigip.conf
python -m explorer.f5_cli cleanup --json bigip.conf
python -m explorer.f5_cli cleanup --keep /Common/important_pool bigip.conf
python -m explorer.f5_cli cleanup --no-keep-common bigip.conf
```

Once the zipapp ships an `f5` console-script the same calls are simply `f5 cleanup …`.

### Claude skill

Run `/bigip-cleanup` (the `bigip-cleanup` skill).  The skill calls `ai/claude/tcl_ai.py bigip-cleanup` under the hood and presents the candidates grouped by kind, plus the ready-to-run script.

## Options

- `--json` — emit the report as JSON instead of a `tmsh` script.  Useful for piping into editors / pipelines.
- `--keep PATH` — keep an object by full path (repeatable).  Paths ending in `/` are partition prefixes, e.g. `--keep /Common/`.
- `--no-keep-common` — disable the default `/Common/` partition guard.  Lets the script delete factory-shipped objects too.  Use with care.
- `-o FILE` — write the script (or JSON) to `FILE` instead of stdout.

## Example

### Input

```
ltm node /Common/n_orphan { address 10.0.0.99 }
ltm monitor http /Common/m_orphan { defaults-from /Common/http }
ltm pool /Common/p_orphan {
    members { /Common/n_orphan:80 { address 10.0.0.99 } }
    monitor /Common/m_orphan
}
ltm rule /Common/r_orphan {
when HTTP_REQUEST {
    pool /Common/p_orphan
    if { [class match [HTTP::host] equals /Common/dg_orphan] } { reject }
}
}
ltm data-group internal /Common/dg_orphan {
    type string
    records { x { data y } }
}
ltm virtual /Common/vs_kept {
    destination /Common/10.0.0.10:80
}
```

### Output (`f5 cleanup --no-keep-common bigip.conf`)

```
# tcl-lsp BIG-IP cleanup
# Sources: file:///bigip.conf
# Roots (kept): 1 virtual server(s) / wide-IP(s)
# Candidates: 5 unreferenced object(s)
#   ltm_data_group_internal: 1
#   ltm_monitor_http: 1
#   ltm_node: 1
#   ltm_pool: 1
#   ltm_rule: 1
#
# Review each delete before running.  Order is reverse-
# topological: referencing objects are deleted before the
# objects they reference, so tmsh accepts each command in turn.

delete ltm rule /Common/r_orphan
delete ltm data-group internal /Common/dg_orphan
delete ltm pool /Common/p_orphan
delete ltm monitor http /Common/m_orphan
delete ltm node /Common/n_orphan
```

The rule is deleted first to release its references to the data-group and pool; the pool is deleted before its monitor and node so they are no longer referenced when their `delete` lines run.

## Out of scope

- The tool never deletes anything itself — the BIG-IP operator runs the script after review.
- `ltm virtual` and `gtm wideip *` are roots by definition and are never proposed for deletion.
- `ltm virtual_address` is excluded because BIG-IP manages address records implicitly from virtual destinations.
- Modules outside `ltm` / `gtm` (cm, auth, sys, net) are never considered for cleanup.

## Related

- [iRule Extraction](kcs-feature-irule-extraction.md) — pulls iRule bodies out of a `bigip.conf` for editing.
