# KCS: How do I audit a BIG-IP config for orphans, anomalies, and policy violations with `f5 query`?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I audit a BIG-IP config — orphan objects, naming-convention violations, ports outside policy, partition-leak checks — using `f5 query`?

## Before you start

- A `bigip.conf` / SCF file.
- A clear list of the conditions you want to audit for.
- Familiarity with the stream-composition pattern in [`kcs-howto-compose-query-streams.md`](kcs-howto-compose-query-streams.md) helps but is not required.

## Answer

Auditing is just a series of `select(...) | .name` queries — each one expresses a policy as a predicate and emits the names that violate it.  Combine them into one script with `;` for a single-pass audit, or run them as separate commands for one finding per query.

### Orphan objects

Pools, monitors, profiles, and iRules that nothing references are usually safe to delete.  The same `referenced_by(...)` builtin that powers `f5 grep --direction reverse` answers this from the DSL:

```
# Orphan pools
$ f5 query --paths-only '.ltm.pool[] | select(referenced_by(.) | count == 0)' bigip.conf

# Orphan iRules (no VS attaches them)
$ f5 query --paths-only '.ltm.rule[] | select(referenced_by(.) | count == 0)' bigip.conf

# Orphan monitors
$ f5 query --paths-only '.ltm.monitor[] | select(referenced_by(.) | count == 0)' bigip.conf
```

Combine into one pass with `;`:

```
$ f5 query --paths-only '
  .ltm.pool[]    | select(referenced_by(.) | count == 0) ;
  .ltm.rule[]    | select(referenced_by(.) | count == 0) ;
  .ltm.monitor[] | select(referenced_by(.) | count == 0)
' bigip.conf
```

### Naming-convention enforcement

```
# VSes that don't follow the `<env>_<service>_vs` convention
$ f5 query '.ltm.virtual[]
  | select(not match(.name, "^[a-z]+_[a-z]+_vs$"))
  | .name' bigip.conf

# Pools whose name doesn't end in `_pool`
$ f5 query '.ltm.pool[]
  | select(not endswith(.name, "_pool"))
  | .name' bigip.conf
```

### Port policy

Some operators enforce that only `:80` / `:443` are allowed.  This catches anything else:

```
$ f5 query '.ltm.virtual[]
  | select(port(.destination) != 80)
  | select(port(.destination) != 443)
  | select(port(.destination) != null)
  | { name: .name, port: port(.destination) }' bigip.conf
```

The trailing `select(port(...) != null)` keeps VSes that lack an explicit port out of the report — those are usually wildcard VSes.

### Cross-partition leak detection

A pool in `/Tenant_A/` that's attached to a VS in `/Tenant_B/` is a leak — find them:

```
$ f5 query '.ltm.virtual[]
  | select(partition(.name) != partition(.pool))
  | { vs: .name, pool: .pool }' bigip.conf
```

### Pool-member sanity checks

```
# Pools with zero members
$ f5 query --paths-only '.ltm.pool[]
  | select(.members | count == 0)' bigip.conf

# Pools whose members all share the same address (single-member redundancy bug)
$ f5 query '.ltm.pool[]
  | select(.members | map(.address) | unique | count == 1)
  | select(.members | count > 1)
  | .name' bigip.conf

# VSes whose pool has a member outside the destination's network
$ f5 query '.ltm.virtual[]
  | select(any(.pool.members[].address | map(in_cidr(., "192.168.0.0/16") | not)))
  | { name: .name, dst: .destination, members: .pool.members[].address }' bigip.conf
```

### iRule reference hygiene

```
# iRules referencing a pool that no longer exists — surface as needs-fix
$ f5 query --paths-only '.ltm.rule[]
  | select(any(.refs.pools | map(. | not)))' bigip.conf
```

(The `.refs.pools` field returns `PathRef`s; an unresolved path stringifies to a placeholder that `not` catches.)

### Putting it together: a one-shot audit script

Save the queries you care about in a file and run with `-f`:

```
# audits.fq
.ltm.pool[]    | select(referenced_by(.) | count == 0) | .name ;
.ltm.virtual[] | select(.pool == "") | .name ;
.ltm.virtual[] | select(.pool != "" and partition(.name) != partition(.pool)) | .name ;
.ltm.rule[]    | select(referenced_by(.) | count == 0) | .name
```

```
$ f5 query -f audits.fq bigip.conf
```

Each statement runs against the same source; the runner emits each statement's output as it goes, so the audit doubles as a categorised report.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [F5 query DSL — builtin function reference](../design/f5-query-dsl-builtins.md)
- [How do I filter and transform streams with f5 query?](kcs-howto-compose-query-streams.md)
- [BIG-IP Related-Object Grep](features/kcs-feature-bigip-grep.md) — alternative for relationship-shaped questions.
