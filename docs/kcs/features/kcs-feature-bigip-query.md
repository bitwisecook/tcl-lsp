# KCS: feature — BIG-IP Query DSL

> **Audience:** User
> **Type:** Functionality

## Summary

`f5` CLI tool with a `query` verb that runs a small jq-flavoured DSL over a `bigip.conf` / SCF, projecting fields, filtering objects, and rewriting matched values — including readdressing virtual servers, renaming objects everywhere they appear, and adjusting iRule references.

## Applies to

tcl-lsp CLI

## Question

How do I select or rewrite many BIG-IP objects at once with a single expression, instead of chaining `grep`, `sed`, and `rename`?

## How to use

`f5 query` parses one or more `bigip.conf` / SCF files into the same object model the rest of the `f5` CLI uses, then runs a jq-flavoured expression against each one.  The expression navigates the parsed tree (`.ltm.virtual["/Common/web_vs"].pool`), filters with `select(...)`, and — with `=` / `|=` / `+=` / `-=` — rewrites matched values.  Identity-field writes (assigning to `.name` or `."full-path"`) automatically route through the same engine `f5 rename` uses, so renaming a pool also moves every reference to it.

By default the verb is a dry-run: read-only queries print their projected values, mutating queries print a unified diff.  Pass `--write` to send the rewritten config to stdout, or `--in-place` to overwrite the input.

### tcl-lsp CLI

```
f5 query '.ltm.virtual[] | .name' bigip.conf
f5 query '.ltm.virtual["~/vs_prod_"] | .pool' bigip.conf
f5 query '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)' bigip.conf
f5 query '.ltm.pool["/Common/old"].name = "/Common/new"' --write bigip.conf > new.conf
```

The `q` alias is provided as a shorthand:

```
f5 q '.ltm.virtual[] | .name' bigip.conf
```

In dev, before the zipapp ships the bare `f5` script, invoke the same module directly: `python -m explorer.f5_cli query …`.

The DSL itself has three companion help screens, all served from the verb's own argparse so they work offline:

- `f5 query --help-dsl` — full grammar reference (operators, precedence, divergences from jq).
- `f5 query --help-builtins` — every function exposed to the DSL, with signature and summary.  Pass a name (`--help-builtins ip`) to drill down to the full per-function reference (signature, semantics, worked examples, return types, error cases).
- `f5 query --help-examples` — a cookbook of common one-liners covering filter, projection, mutation, rename, and iRule rewrites.

The same per-function reference is available as a rendered document at [`docs/design/f5-query-dsl-builtins.md`](../../design/f5-query-dsl-builtins.md) — the canonical builtin reference, generated from the registry so it can't drift from the runtime.  For grammar and architectural background see [`docs/design/f5-query-dsl.md`](../../design/f5-query-dsl.md).

Complex worked examples live in KCS How-Tos:

- [Composing query streams](../kcs-howto-compose-query-streams.md) — `select` / `map` / `any` / `all` / `sort` patterns.
- [Auditing a config](../kcs-howto-audit-config-with-query.md) — orphans, naming, ports, partition leaks.
- [Multi-step transformations](../kcs-howto-cross-config-transforms-with-query.md) — rename + readdress + policy edits in one query.
- [Bulk readdressing](../kcs-howto-readdress-virtuals-with-query.md), [partition migration](../kcs-howto-migrate-partition-with-query.md), [iRule reference rewriting](../kcs-howto-rewrite-pool-refs-in-irules.md), [finding objects by predicate](../kcs-howto-find-objects-by-query.md).

## Options

- `-f, --from-file FILE` — read the query expression from `FILE` instead of the positional argument.  Useful for multi-line queries that share comments and intermediate computations.
- `--scf` — render every selected value as an SCF stanza when possible.  Pairs well with `f5 query ... | f5 cleanup`.
- `--raw` — render scalar values one per line with no quoting; matches jq's `--raw-output`.
- `--paths-only` — print only the full-path of each object or path-ref produced.  Cheap and pipeable, useful in shell loops.
- `--json` — render the result as a JSON array; objects serialise as `{"kind", "full-path", "fields"}`.
- `--write` — when the query mutates, print the rewritten config to stdout (default: print a unified-diff preview).  Mutually exclusive with `--in-place`.
- `--in-place` — when the query mutates, overwrite each input file with the rewritten config.  Reads strictly UTF-8 (refuses undecodable bytes rather than substituting U+FFFD), and refuses `--format tmsh` (which would silently overwrite SCF source with a tmsh script).
- `--format scf|tmsh` — output format for the rewritten config.  `scf` (default) emits the source with edits applied in-place, preserving comments / whitespace / field order.  `tmsh` emits a `tmsh modify` script suitable for piping to a remote device or redirecting to a file.
- `--input-json NAME=PATH`, `--input-jsonl NAME=PATH`, `--input-csv NAME=PATH[:hdr1,hdr2]`, and `--input-f5log NAME=PATH` — bind structured side inputs to `$NAME` without making them primary BIG-IP configs.  Use these for inventories, NAT maps, event streams, and BIG-IP logs that enrich a config query.
- `--enable-probes` — allow network probe builtins such as `ping`, `portping`, `url_get`, `socket_get`, and `tls_handshake`.  Probes are disabled by default so ordinary queries stay offline-safe.
- `--ca-bundle PATH` — trust a specific CA bundle for TLS probes.  This is useful for internal endpoints and lab certificates.
- `--help-dsl` — print the DSL grammar reference and exit.
- `--help-builtins [NAME]` — print every builtin's signature and example, or one named entry.
- `--help-examples` — print the worked-example cookbook.

The exit code is `0` when the query produced at least one value or applied at least one edit, `1` when a read-only query produced nothing, and `2` for a parse / type / edit error.

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
$ f5 query --paths-only '.ltm.virtual[].pool' bigip.conf
/Common/web_pool
/Common/web_pool
```

### Filter VSes by destination CIDR

```
$ f5 query '.ltm.virtual[] | select(in_cidr(.destination, "10.10.0.0/24")) | .name' bigip.conf
web_vs
api_vs
```

### Readdress every VS, keeping host bits

```
$ f5 query '.ltm.virtual[] | .destination |= ip("192.168.9.0/24", .)' bigip.conf
--- bigip.conf
+++ bigip.conf (modified)
@@ -7,7 +7,7 @@
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

### Migrate every object in a partition

```
$ f5 query 'rename_partition("Tenant_A", "Tenant_B")' bigip.conf
--- bigip.conf
+++ bigip.conf (modified)
@@ -1,4 +1,4 @@
-auth partition Common { description default }
+auth partition Tenant_A { description default }
-ltm pool /Common/web_pool {
-    members { /Common/n1%5:80 { address 10.0.0.1%5 } }
+ltm pool /Tenant_A/web_pool {
+    members { /Tenant_A/n1%5:80 { address 10.0.0.1%5 } }
     monitor /Common/http
 }
-ltm virtual /Common/web_vs {
-    destination /Common/10.10.0.5%5:443
-    pool /Common/web_pool
+ltm virtual /Tenant_A/web_vs {
+    destination /Tenant_A/10.10.0.5%5:443
+    pool /Tenant_A/web_pool
 }
```

`rename_partition` cascades through every reference, including
partition prefixes embedded in compound values (destination
addresses, pool-member names, iRule body literals) and the
`auth partition` stanza itself.  Route domains and ports are
preserved through the move.

### Set a route domain on every destination

```
$ f5 query --write '.ltm.virtual[] | .destination |= with_route_domain(., 7)' bigip.conf
ltm virtual /Common/web_vs {
    destination /Common/10.10.0.5%7:443
    pool /Common/web_pool
}
```

`with_route_domain` sets, replaces, or strips (pass `""` or `null`)
the route domain on an address.  `ip(network, source)` preserves the
route domain when readdressing — `%5` survives the subnet rebase.

### Single-object rename (same engine as `f5 rename`)

```
$ f5 query 'rename("/Common/web_pool", "/Common/app_pool")' bigip.conf
```

`f5 rename old new file.conf` is a thin shell over this builtin, so reaching for `rename()` from the DSL is the right move when you want to combine a rename with other transforms in one statement — for example, rename and then mutate a property in a single dry-run preview:

```
$ f5 query '
  rename("/Common/web_pool", "/Common/app_pool") ;
  .ltm.pool["/Common/app_pool"].monitor = "/Common/tcp"
' bigip.conf
```

The two statements run in order against the evolving source.

### Rename a pool everywhere

```
$ f5 query '.ltm.pool["/Common/web_pool"].name = "/Common/app_pool"' --write bigip.conf
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

A `renamed /Common/web_pool -> /Common/app_pool (3 occurrence(s))` line is also printed to stderr so the multi-stanza rewrite is visible.

## Out of scope

- General command-argument rewriting inside iRule bodies is deferred to v2.  In v1 the only writable slots inside an iRule are the reference lists `.refs.pools[]`, `.refs.persists[]`, and `.refs.data-groups[]`, which are rewritten via the same token-bounded engine `f5 rename` uses.
- Compound property values are partially writable.  Per-member fields on a pool member (`.ltm.pool[].members[].address` and siblings) carry real byte-offset slots and accept field edits in place; other sub-block compound values (e.g. policy rule actions, persistence body) are still v2.  Add or remove pool members by editing the pool object directly, or pipe through `f5 cleanup` after a query that emits SCF stanzas.
- The DSL has lexical variable bindings — `expr as $name | body` (jq-flavoured) — and a top-level `$name` form addressing each loaded source by its filename stem.  It also supports jq-style stream comma (`a, b`) and conditionals.  There is no user-defined-function syntax; compose larger queries with `as` bindings, comma streams, pipes, and `;`-separated statements.

## Related

- [BIG-IP Related-Object Grep](kcs-feature-bigip-grep.md) — uses the same reference graph for "which objects touch X?" queries.
- [BIG-IP Config Cleanup](kcs-feature-bigip-cleanup.md) — pairs with `f5 query --scf` to emit a tidy projection.
- [F5 CLI](kcs-feature-f5-cli.md) — the umbrella verb catalogue.
- [F5 query DSL design](../../design/f5-query-dsl.md) — grammar, value model, and edit pipeline internals.
