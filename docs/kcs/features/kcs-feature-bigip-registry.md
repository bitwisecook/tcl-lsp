# KCS: feature — BIG-IP Object Registry

> **Audience:** User
> **Type:** Functionality
## Applies to

tcl-lsp CLI / VS Code / Zed (BIG-IP `.conf` / `.scf` files)

## Summary

The BIG-IP object registry is the catalogue `f5` uses to understand
every TMSH object kind — pools, virtuals, monitors, profiles,
firewall policies, GTM wide-IPs, etc.  It powers consistent
behaviour across `f5 grep`, `f5 query`, `f5 explain`, `f5 rename`,
and every BIG-IP editor feature (document links, go-to-definition,
references, rename, semantic-token highlighting).

## How to use

You don't invoke the registry directly — it runs in the background
of every BIG-IP command and every editor session.  The user-visible
behaviour:

- **Smart `f5 query` projections**: typed property fields surface
  structured children (e.g. `.ltm.virtual[].profiles[] | select(.context == "clientside")`,
  `.ltm.pool[].monitor.monitors[]`).
- **Editor click-through**: clicking a TMSH path in a `.conf` file
  jumps to the referenced object's stanza header.
- **Reference / rename safety**: renaming an object updates every
  reference the registry knows about.
- **Diagnostics for malformed values**: e.g. `min 0 of { ... }`
  on a monitor expression flags as invalid before you push the
  config.

## Operational context

The contract behind the registry — value-spec protocol, source
ranges, the pilot migration table, how compound value types work —
lives in [`docs/design/bigip-registry-architecture.md`](../../design/bigip-registry-architecture.md).
Refer to it when extending the registry, adding a new typed
property, or hooking a new editor feature into the dispatch.
