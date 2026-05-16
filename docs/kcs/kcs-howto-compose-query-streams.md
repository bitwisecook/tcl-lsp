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
- The full per-function reference at [`docs/references/f5_query/builtins.md`](../references/f5_query/builtins.md), or `f5 query --help-builtins NAME` from the terminal.

## Answer

`f5 query` pipelines are read left-to-right: each stage receives the previous stage's output as its input (`.`).  The semantics match jq's:

- `.X[]` is a **stream** — the pipe iterates it, applying the next stage once per item.
- Plain lists (the value of `.rules`, `.members`, etc.) pass through `|` **whole**.  To iterate a list, either pass it to a list-aware builtin like `map`, or write `.X[]` to convert it to a stream.
- To fold a stream back into a list for aggregators like `sort` / `unique`, wrap it in a **list literal** `[ ... ]`.  That's jq's standard idiom.

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

`.rules | count > 0` works because `.rules` is a list — the pipe passes it whole to `count`.

### `map` — apply an expression to every item of a list

`map(body)` evaluates *body* against each item of its input with `.` rebound to that item, and collects the results into a list.  Use it when the input is already a list.

```
# Strip the partition from every iRule reference attached to a VS
$ f5 query '.ltm.virtual[] | .rules | map(basename(.))' bigip.conf
```

When you want to project a field across a *stream* (not a list), pipe through the builtin directly — pipe-iteration already runs the builtin per item:

```
# Stream of basenames, one per matching VS's default pool
$ f5 query --raw '.ltm.virtual[].pool | basename(.)' bigip.conf

# Distinct partitions in use across all VSes — wrap the per-item
# projection in a list literal, then aggregate:
$ f5 query '[.ltm.virtual[].name | partition(.)] | unique | sort' bigip.conf
```

### `any` / `all` — collapse a list or stream to a boolean

`any(values)` and `all(values)` test whether at least one (or every) item is truthy.  They work both on plain lists and on streams.

```
# VSes whose pool has a member in 10.0.0.0/8
$ f5 query '.ltm.virtual[]
  | select(any(.pool.members[].address | in_cidr(., "10.0.0.0/8")))
  | .name' bigip.conf

# VSes whose every attached iRule lives in /Common/
$ f5 query '.ltm.virtual[]
  | select(.rules | count > 0)
  | select(all(.rules | map(startswith(., "/Common/"))))
  | .name' bigip.conf
```

The first form pipes the stream of addresses through `in_cidr` per item (yielding a stream of booleans), then `any` collapses.  The second uses `map` because `.rules` is already a list.

### `sort` + `unique` — aggregate a list

`sort` returns a sorted list; `unique` deduplicates preserving first-seen order.  They expect a single list/stream value as input — so wrap stream-producing path expressions with a list literal first.

```
# Every distinct default pool, sorted
$ f5 query '[.ltm.virtual[].pool] | unique | sort' bigip.conf

# Count distinct partitions in use across all VSes
$ f5 query '[.ltm.virtual[].name | partition(.)] | unique | count' bigip.conf
```

### `first` / `last` / `count` — pick or measure

```
# The alphabetical-first VS name
$ f5 query '[.ltm.virtual[].name] | sort | first' bigip.conf

# How many VSes have a non-empty pool?
$ f5 query '[.ltm.virtual[] | select(.pool != "")] | count' bigip.conf
```

The list literal `[ ... ]` is the jq idiom for "collect this stream into an array so the next stage sees one value".

### Putting it together: orphan-pool audit

```
$ f5 query --paths-only '.ltm.pool[]
  | select(referenced_by(.) | count == 0)' bigip.conf
```

`referenced_by(.)` walks the reverse edges of the reference graph (the same one `f5 grep` uses) and returns a list.  Inside `select`, the list passes through `|` whole to `count`, and the resulting integer compares with `> 0`.  Pools that nothing references are surfaced; `--paths-only` prints one full-path per line so the output is pipeable into `xargs`-style cleanup loops.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [F5 query DSL — builtin function reference](../references/f5_query/builtins.md)
- [How do I find BIG-IP objects with a query expression?](kcs-howto-find-objects-by-query.md)
- [How do I bulk-readdress virtual servers into a new subnet?](kcs-howto-readdress-virtuals-with-query.md)
