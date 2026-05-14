# KCS: How do I migrate every object from one partition into another?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I migrate every object from one partition into another — including the references buried in destination addresses, pool-member names, and iRule bodies?

## Before you start

- A `bigip.conf` / SCF that contains the partition you want to move.
- The destination partition name (must match `[A-Za-z0-9_.-]+`).
- A clear idea of whether you want the **whole partition** to move (everything in `/Common/`) or only **specific kinds** (just the pools, just the virtuals).

## Answer

`f5 query` ships two complementary operations for partition work.  Pick by intent:

### Move every object in the partition

Use `rename_partition(old, new)` when the partition itself is moving:

```
f5 query 'rename_partition("Tenant_A", "Tenant_B")' bigip.conf
```

This applies a token-bounded prefix rewrite across the entire source: every `/Common/<name>` reference — including the structural prefix on destination addresses (`destination /Common/10.10.0.5:443`), pool-member identifiers (`/Common/n1:80`), and iRule body literals (`pool /Common/web_pool`) — moves to `/Tenant_A/<name>`.  The bare `auth partition Common` stanza header is renamed too.  Route domains and ports are preserved through the move.

The default is a dry-run unified diff; pass `--in-place` to overwrite the file or `--write` to print the rewritten config to stdout.

### Move one kind, leave the others

When you want to migrate only certain kinds — say, move every pool out of `/Common/` but keep the virtuals where they are — use a filter plus an identity-field update:

```
f5 query '.ltm.pool["~^/Common/"] | .name |= with_partition(., "Tenant_A")' bigip.conf
```

Each match routes through the same engine `f5 rename` uses, so the pool's header *and* every reference to it (in virtuals, in iRule bodies, in data-groups) is rewritten together — but compound values that merely share the `/Common/` prefix (like the virtual's `destination /Common/...`) are left untouched.

Chain multiple kinds with `;` to migrate a curated set in one statement:

```
f5 query '
  .ltm.pool["~^/Common/"]    | .name |= with_partition(., "Tenant_A");
  .ltm.monitor["~^/Common/"] | .name |= with_partition(., "Tenant_A")
' bigip.conf
```

### Route domains

Route domains attach to addresses with `%<n>` and are part of the routable identity.  They survive `ip(net, source)` rebases and `rename_partition` cascades unchanged.  To set, replace, or strip them explicitly:

```
f5 query '.ltm.virtual[] | .destination |= with_route_domain(., 7)' bigip.conf
f5 query '.ltm.virtual[] | .destination |= with_route_domain(., "")' bigip.conf
```

The first sets every VS's destination route domain to `7`; the second strips the route domain entirely.  Use `route_domain(.)` to project the current value as a string.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [BIG-IP rename](features/kcs-feature-rename.md)
- [How do I bulk-readdress virtual servers into a new subnet?](kcs-howto-readdress-virtuals-with-query.md)
- [F5 query DSL design](../design/f5-query-dsl.md)
