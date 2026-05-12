# KCS: How do I filter and transform streams of objects with `f5 query`?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I compose `select`, `map`, `any`, `all`, `sort`, and `unique` to filter and transform streams of BIG-IP objects?

## Before you start

- A `bigip.conf` / SCF file.
- A rough idea of which property you want to filter, project, or aggregate on.
- The full per-function reference at [`docs/design/f5-query-dsl-builtins.md`](../design/f5-query-dsl-builtins.md), or `f5 query --help-builtins NAME` from the terminal.

## Answer

`f5 query` pipelines are read left-to-right: each stage receives the previous stage's output as its input (`.`).  The stream builtins are the primitives that move data through those stages.

### `select` — drop values that don't pass a predicate

`select(body)` keeps the current value when *body* evaluates to a truthy result, drops it otherwise.  Stack `select(...)` calls to AND predicates together; use `or` inside one body to OR.

```
# VSes that have at least one attached iRule
$ f5 query '.ltm.virtual[] | select(.rules | count > 0) | .name' bigip.conf

# VSes whose destination is in 10.0.0.0/8 AND whose pool is non-empty
$ f5 query '.ltm.virtual[]
  | select(in_cidr(.destination, "10.0.0.0/8"))
  | select(.pool != "")
  | .name' bigip.conf
```

### `map` — apply an expression to every item

`map(body)` evaluates *body* against each item with `.` rebound to that item, and collects the results into a list.  Use it when you want to transform a list, not filter it.

```
# Strip the partition from every default pool reference
$ f5 query '.ltm.virtual[].pool | map(basename(.))' bigip.conf

# What partitions own at least one VS?
$ f5 query '.ltm.virtual[].name | map(partition(.)) | unique | sort' bigip.conf
```

### `any` / `all` — collapse a list to a boolean

`any(list)` and `all(list)` test whether at least one (or every) item is truthy.  Combined with `map` they give per-item predicates a clean spelling:

```
# VSes whose pool has a member in 10.0.0.0/8
$ f5 query '.ltm.virtual[]
  | select(any(.pool.members[].address | map(in_cidr(., "10.0.0.0/8"))))
  | .name' bigip.conf

# VSes whose every attached iRule lives in /Common/
$ f5 query '.ltm.virtual[]
  | select(.rules | count > 0)
  | select(all(.rules | map(startswith(., "/Common/"))))
  | .name' bigip.conf
```

### `sort` + `unique` — clean up stream output

`sort` returns a sorted list; `unique` deduplicates preserving first-seen order.  Both flatten a stream into a list, so they're usually the last stage in a reporting pipeline.

```
# Every distinct default pool, sorted
$ f5 query '.ltm.virtual[].pool | unique | sort' bigip.conf

# Count distinct partitions in use across all VSes
$ f5 query '.ltm.virtual[].name | map(partition(.)) | unique | count' bigip.conf
```

### `first` / `last` / `count` — pick or measure

```
# The alphabetical-first VS
$ f5 query '.ltm.virtual[].name | sort | first' bigip.conf

# How many VSes have a non-empty pool?
$ f5 query '.ltm.virtual[] | select(.pool != "") | count' bigip.conf
```

### Putting it together: orphan-pool audit

```
$ f5 query --paths-only '.ltm.pool[]
  | select(referenced_by(.) | count == 0)' bigip.conf
```

`referenced_by(.)` walks the reverse edges of the reference graph (the same one `f5 grep` uses).  Pools that nothing references are surfaced; `--paths-only` prints one full-path per line so the output is pipeable into `xargs`-style cleanup loops.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [F5 query DSL — builtin function reference](../design/f5-query-dsl-builtins.md)
- [How do I find BIG-IP objects with a query expression?](kcs-howto-find-objects-by-query.md)
- [How do I bulk-readdress virtual servers into a new subnet?](kcs-howto-readdress-virtuals-with-query.md)
