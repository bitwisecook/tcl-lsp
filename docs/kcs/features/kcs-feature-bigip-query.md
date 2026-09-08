# KCS: feature — BIG-IP Query DSL

> **Audience:** User
> **Type:** Functionality

## Summary

`f5-query` CLI verb that runs a small jq-flavoured DSL over a `bigip.conf` / SCF, projecting fields, filtering objects, and rewriting matched values — including readdressing virtual servers, renaming objects everywhere they appear, and adjusting iRule references.

## Applies to

tcl-lsp CLI

## Question

How do I select or rewrite many BIG-IP objects at once with a single expression, instead of chaining `grep`, `sed`, and `rename`?

## How to use

`f5-query query` parses one or more `bigip.conf` / SCF files into the same object model the rest of the CLI uses, then runs a jq-flavoured expression against each one.  The expression navigates the parsed tree (`.ltm.virtual["/Common/web_vs"].pool`), filters with `select(...)`, and — with `=` / `|=` / `+=` / `-=` — rewrites matched values.  Identity-field writes (assigning to `.name` or `."full-path"`) route through the same engine `f5-query rename` uses, so renaming a pool also moves every reference to it.

By default the verb is a dry-run: read-only queries print their projected values, mutating queries print a unified diff.  Pass `--write` to send the rewritten config to stdout, or `--in-place` to overwrite the input.

```
f5-query query '.ltm.virtual[] | .name' bigip.conf
f5-query query '.ltm.virtual["~/vs_prod_"] | .pool' bigip.conf
f5-query query '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)' bigip.conf
f5-query query '.ltm.pool["/Common/old"].name = "/Common/new"' --write bigip.conf > new.conf
```

`q` is an alias for the same verb.

The DSL carries its own offline help screens:

- `--help-dsl` — grammar reference (operators, precedence, divergences from jq).
- `--help-builtins [NAME]` — the builtin catalogue, or one function's category, arity, and flags.
- `--help-examples` — a cookbook of one-liners covering filter, projection, mutation, rename, and iRule rewrites.
- `--help-manual` — grammar, builtins, and cookbook in one document.
- `--help-renderers` / `--help-inputs` — the registered `--render` plugins and `--input` formats.

Per-function prose lives in [`docs/references/f5_query/builtins.md`](../../references/f5_query/builtins.md), generated from the registry so it cannot drift from the runtime.  For grammar and architectural background see [`docs/references/f5_query/dsl.md`](../../references/f5_query/dsl.md).

Complex worked examples live in KCS How-Tos:

- [Composing query streams](../kcs-howto-compose-query-streams.md) — `select` / `map` / `any` / `all` / `sort` patterns.
- [Auditing a config](../kcs-howto-audit-config-with-query.md) — orphans, naming, ports, partition leaks.
- [Multi-step transformations](../kcs-howto-cross-config-transforms-with-query.md) — rename + readdress + policy edits in one query.
- [Bulk readdressing](../kcs-howto-readdress-virtuals-with-query.md), [partition migration](../kcs-howto-migrate-partition-with-query.md), [iRule reference rewriting](../kcs-howto-rewrite-pool-refs-in-irules.md), [finding objects by predicate](../kcs-howto-find-objects-by-query.md).

## Options

- `-f, --from-file FILE` — read the query expression from `FILE` instead of the positional argument.  Useful for multi-line queries that share comments and intermediate computations.
- `--scf` — render every selected value as an SCF stanza when possible.
- `--raw` — render scalar values one per line with no quoting; matches jq's `--raw-output`.
- `--paths-only` — print only the full-path of each object or path-ref produced.  Cheap and pipeable, useful in shell loops.
- `--json` — render the result as a JSON array; objects serialise as `{"kind", "full-path", "fields"}`.
- `--table` / `--table-lineart` — render the result as an ASCII or box-drawing grid.
- `-R, --render NAME` with `--render-opt KEY=VALUE` — dispatch output through a renderer plugin; see [renderers](kcs-feature-f5-query-renderers.md).
- `--write` — when the query mutates, print the rewritten config to stdout (default: print a unified-diff preview).  Mutually exclusive with `--in-place`.
- `--in-place` — when the query mutates, overwrite each input file with the rewritten config.  Reads strictly UTF-8 (refuses undecodable bytes rather than substituting U+FFFD), and refuses `--format tmsh` (which would silently overwrite SCF source with a tmsh script).
- `--format scf|tmsh|tmsh-delta` — output format for the rewritten config.  `scf` (default) emits the source with edits applied in place, preserving comments, whitespace, and field order.  `tmsh` emits a `tmsh modify` script; `tmsh-delta` emits only the changed objects.  Add `--transaction` to wrap a tmsh script in a `cli transaction`.
- `--input-json NAME=PATH`, `--input-jsonl NAME=PATH`, `--input-csv NAME=PATH[:hdr1,hdr2]`, `--input-f5log NAME=PATH`, and `--input KIND NAME=PATH` — bind structured side inputs to `$NAME` without making them primary BIG-IP configs.  Use these for inventories, NAT maps, event streams, and BIG-IP logs that enrich a config query.
- `--name NAME=PATH` and `--merge` — bind a config to `$NAME`, or merge every config into one namespace.
- `--strict` — exit non-zero when a read-only query matched nothing.
- `--enable-probes` — allow network probe builtins such as `ping`, `portping`, `url_get`, `socket_get`, and `tls_handshake`.  Probes are disabled by default so ordinary queries stay offline-safe.
- `--ca-bundle PATH` — trust a specific CA bundle for TLS probes.  This is useful for internal endpoints and lab certificates.
The exit code is `0` on success, `2` for a parse, type, or edit error.  With `--strict`, a read-only query that produced nothing exits `1`.

## Example

### Input

```
ltm pool /Common/web_pool {
    members { /Common/n1:80 { address 10.0.0.1 } }
    monitor /Common/http
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5:443
    pool /Common/web_pool
}
ltm virtual /Common/api_vs {
    destination /Common/10.10.0.6:80
    pool /Common/web_pool
}
```

### Project every VS's default pool

```
$ f5-query query --paths-only '.ltm.virtual[].pool' bigip.conf
/Common/web_pool
/Common/web_pool
```

### Filter VSes by destination CIDR

```
$ f5-query query '.ltm.virtual[] | select(in_cidr(.destination, "10.10.0.0/24")) | .name' bigip.conf
web_vs
api_vs
```

### Readdress every VS, keeping host bits

```
$ f5-query query '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)' bigip.conf
--- bigip.conf
+++ bigip.conf (modified)
@@ -3,10 +3,10 @@
     monitor /Common/http
 }
 ltm virtual /Common/web_vs {
-    destination /Common/10.10.0.5:443
+    destination /Common/192.168.9.5:443
     pool /Common/web_pool
 }
 ltm virtual /Common/api_vs {
-    destination /Common/10.10.0.6:80
+    destination /Common/192.168.9.6:80
     pool /Common/web_pool
 }
```

### Move every object to another partition

```
$ f5-query query 'rename_partition("Tenant_A", "Tenant_B")' tenant.conf
renamed 'partition /Tenant_A/' -> '/Tenant_B/' (5 occurrence(s))
renamed 'auth partition Tenant_A' -> 'auth partition Tenant_B' (1 occurrence(s))
--- tenant.conf
+++ tenant.conf (modified)
@@ -1,9 +1,9 @@
-auth partition Tenant_A { description tenant }
-ltm pool /Tenant_A/web_pool {
-    members { /Tenant_A/n1%5:80 { address 10.0.0.1%5 } }
+auth partition Tenant_B { description tenant }
+ltm pool /Tenant_B/web_pool {
+    members { /Tenant_B/n1%5:80 { address 10.0.0.1%5 } }
     monitor /Common/http
 }
-ltm virtual /Tenant_A/web_vs {
-    destination /Tenant_A/10.10.0.5%5:443
-    pool /Tenant_A/web_pool
+ltm virtual /Tenant_B/web_vs {
+    destination /Tenant_B/10.10.0.5%5:443
+    pool /Tenant_B/web_pool
 }
```

`rename_partition` cascades through every reference, including partition
prefixes embedded in compound values (destination addresses, pool-member
names, iRule body literals) and the `auth partition` stanza itself.  Route
domains and ports survive the move.  It refuses to rename `/Common`, because
tenant partitions reference `/Common` one-way and the renames would not
follow.

### Set a route domain on every destination

```
$ f5-query query --write '.ltm.virtual[] | .destination |= with_route_domain(., 7)' bigip.conf
ltm pool /Common/web_pool {
    members { /Common/n1:80 { address 10.0.0.1 } }
    monitor /Common/http
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5%7:443
    pool /Common/web_pool
}
ltm virtual /Common/api_vs {
    destination /Common/10.10.0.6%7:80
    pool /Common/web_pool
}
```

`--write` prints the whole rewritten config, not just the changed stanzas.
`with_route_domain` sets, replaces, or strips (pass `""` or `null`) the route
domain on an address.  `ip(network, source)` preserves the route domain when
readdressing — `%5` survives the subnet rebase.

### Rename, on its own or combined

`f5-query rename old new file.conf` is a thin shell over the `rename()`
builtin, so reach for the builtin when you want a rename plus other
transforms in one dry-run preview:

```
$ f5-query query '
  rename("/Common/web_pool", "/Common/app_pool") ;
  .ltm.pool["/Common/app_pool"].monitor = "/Common/tcp"
' bigip.conf
```

The two statements run in order against the evolving source.  Assigning to
`.name` does the same thing:

```
$ f5-query query '.ltm.pool["/Common/web_pool"].name = "/Common/app_pool"' --write bigip.conf
renamed '/Common/web_pool' -> '/Common/app_pool' (3 occurrence(s))
ltm pool /Common/app_pool {
    members { /Common/n1:80 { address 10.0.0.1 } }
    monitor /Common/http
}
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5:443
    pool /Common/app_pool
}
ltm virtual /Common/api_vs {
    destination /Common/10.10.0.6:80
    pool /Common/app_pool
}
```

The `renamed …` line goes to stderr, so the multi-stanza rewrite is visible
even when stdout is redirected.

## Out of scope

- Inside an iRule body, the only writable slots are the reference lists `.refs.pools[]`, `.refs.persists[]`, and `.refs.data-groups[]`, rewritten through the same token-bounded engine `f5-query rename` uses.  General command-argument rewriting is not supported.
- Compound property values are partially writable.  Per-member fields on a pool member (`.ltm.pool[].members[].address` and siblings) carry real byte-offset slots and accept field edits in place; other sub-block compound values (policy rule actions, persistence body) do not.  Add or remove pool members by editing the pool object directly.
- There is no user-defined-function syntax.  Compose larger queries with `as` bindings (`expr as $name | body`), comma streams, pipes, and `;`-separated statements.  A top-level `$name` addresses each loaded source by its filename stem.

## Related

- [BIG-IP Related-Object Grep](kcs-feature-bigip-grep.md) — uses the same reference graph for "which objects touch X?" queries.
- [BIG-IP Config Cleanup](kcs-feature-bigip-cleanup.md) — pairs with `f5-query query --scf` to emit a tidy projection.
- [F5 CLI](kcs-feature-f5-cli.md) — the umbrella verb catalogue.
- [F5 query DSL design](../../references/f5_query/dsl.md) — grammar, value model, and edit pipeline internals.
