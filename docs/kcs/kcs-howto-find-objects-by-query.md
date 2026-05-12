# KCS: How do I find BIG-IP objects with a query expression?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I find BIG-IP objects matching arbitrary conditions on their properties, using a query expression instead of chained `grep`s?

## Before you start

- A `bigip.conf` / SCF file.
- A clear idea of which property you want to filter on (destination, pool, attached iRules, partition, …).

## Answer

`f5 query` lets you filter objects with `select(...)` against any property, including computed ones — IP-in-CIDR membership, regex matches, list contents, partition names.  For lookups by full-path or simple pattern, `f5 grep` is still the right tool.  Reach for `f5 query` when the predicate is property-shaped.

1. List every VS whose pool is empty (no default pool set):

   ```
   f5 query '.ltm.virtual[] | select(.pool == "") | .name' bigip.conf
   ```

2. Find every VS whose destination falls inside a CIDR:

   ```
   f5 query '.ltm.virtual[] | select(in_cidr(.destination, "10.0.0.0/8")) | .name' bigip.conf
   ```

3. Find every VS attached to a specific iRule:

   ```
   f5 query '.ltm.virtual[] | select(contains(.rules, "/Common/log_rule")) | .name' bigip.conf
   ```

4. Find every iRule that references a removed pool:

   ```
   f5 query '.ltm.rule[] | select(contains(.refs.pools, "/Common/removed_pool")) | .name' bigip.conf
   ```

5. Emit full SCF stanzas for selected VSes (so the output is pipeable into other `f5` verbs):

   ```
   f5 query --scf '.ltm.virtual["~^vs_prod_"]' bigip.conf | f5 cleanup -
   ```

   `--scf` renders the matched objects as parseable SCF; the `cleanup` verb (or any other) reads it from stdin.

For relationship-shaped questions ("what depends on X?") use [`f5 grep`](features/kcs-feature-bigip-grep.md), which walks the reference graph.  `f5 query` is the right tool when the predicate is shaped like a filter on the object's own properties.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [BIG-IP Related-Object Grep](features/kcs-feature-bigip-grep.md)
- [When should I use f5 query versus f5 grep or f5 rename?](kcs-qa-query-vs-grep-vs-rename.md)
