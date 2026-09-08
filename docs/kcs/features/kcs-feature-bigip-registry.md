# KCS: feature — BIG-IP Object Registry

> **Audience:** User
> **Type:** Functionality

## Summary

The catalogue that tells `f5` and the editor what every TMSH object
kind is — pools, virtuals, monitors, profiles, firewall policies, GTM
wide-IPs, and the rest.

## Applies to

tcl-lsp CLI, VS Code, Zed

## How to use

You do not invoke the registry directly.  It runs behind every BIG-IP verb
and every editor session on a `.conf` / `.scf` file.  What you see:

- **Reference-following projections**: a typed property that names another
  object is a reference you can navigate through, so
  `.ltm.virtual[].pool | .members[]` walks from a virtual to its pool's
  members.
- **Editor click-through**: clicking a TMSH path in a `.conf` file jumps to
  the referenced object's stanza header.
- **Reference / rename safety**: renaming an object updates every reference
  the registry knows about.
- **Unresolved-reference diagnostics**: a registry-declared reference that
  names no object in the configuration is flagged as `BIGIP6013`.

## Example

```
$ f5 query '.ltm.virtual[].pool | .members[]' --paths-only bigip.conf
/Common/n1:80
```

The query starts at a virtual server, follows its `pool` property into the
pool object, and lists that pool's members — the registry supplies the type
of each hop.

## Operational context

The contract behind the registry — value-spec protocol, source ranges, and
how compound value types work — lives in
[`docs/design/bigip-registry-architecture.md`](../../design/bigip-registry-architecture.md).
