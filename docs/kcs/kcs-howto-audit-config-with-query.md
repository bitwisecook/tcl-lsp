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
# Names of offending VSes:
$ f5 query --raw '.ltm.virtual[]
  | select(port(.destination) != 80)
  | select(port(.destination) != 443)
  | select(port(.destination) != null)
  | .name' bigip.conf

# Their full SCF stanzas (for review or piping into another verb):
$ f5 query --scf '.ltm.virtual[]
  | select(port(.destination) != 80)
  | select(port(.destination) != 443)
  | select(port(.destination) != null)' bigip.conf
```

The trailing `select(port(...) != null)` keeps VSes that lack an explicit port out of the report — those are usually wildcard VSes.

### Cross-partition leak detection

A pool in `/Tenant_A/` that's attached to a VS in `/Tenant_B/` is a leak — find them:

```
# Offending VS names:
$ f5 query --raw '.ltm.virtual[]
  | select(partition(.name) != partition(.pool))
  | .name' bigip.conf

# Or as JSON with both endpoints of the leak (the VS object + its `.pool` ref):
$ f5 query --json '.ltm.virtual[]
  | select(partition(.name) != partition(.pool))' bigip.conf
```

`--json` renders each matched VS as a structured record with every projected field, so the offending `name` and `pool` are both in the JSON payload — useful when you want to feed the report to another tool.

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

# Names of VSes whose pool has any member outside the chosen range:
$ f5 query --raw '.ltm.virtual[]
  | select(any(.pool.members[].address | in_cidr(., "192.168.0.0/16") | not))
  | .name' bigip.conf

# Same VSes as JSON — payload includes destination and member addresses:
$ f5 query --json '.ltm.virtual[]
  | select(any(.pool.members[].address | in_cidr(., "192.168.0.0/16") | not))' bigip.conf
```

The stream of member addresses is piped through `in_cidr(., "...")` and then `not` (each item becomes the boolean "outside the range"); `any` collapses the stream of booleans into a single decision.

### iRule reference hygiene

```
# iRules with no pool references (often a sign of a half-cleaned rule)
$ f5 query --paths-only '.ltm.rule[]
  | select(.refs.pools | count == 0)' bigip.conf
```

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

Each statement is evaluated against the evolving source.  In the current runner only the final statement's values are surfaced as output (earlier statements run for their side-effects — edits and the audit information they print to stderr).  For a categorised audit report, run each predicate as its own `f5 query` invocation and prefix the output yourself:

```
$ for q in \
    '.ltm.pool[]    | select(referenced_by(.) | count == 0) | .name' \
    '.ltm.virtual[] | select(.pool == "") | .name' \
    '.ltm.virtual[] | select(.pool != "" and partition(.name) != partition(.pool)) | .name' \
    '.ltm.rule[]    | select(referenced_by(.) | count == 0) | .name'
do
    echo "=== $q ==="
    f5 query --raw "$q" bigip.conf
done
```

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [F5 query DSL — builtin function reference](../references/f5_query/builtins.md)
- [How do I filter and transform streams with f5 query?](kcs-howto-compose-query-streams.md)
- [BIG-IP Related-Object Grep](features/kcs-feature-bigip-grep.md) — alternative for relationship-shaped questions.
