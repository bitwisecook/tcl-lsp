# KCS: How do I compose multi-step transformations across a BIG-IP config?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I compose multi-step transformations — readdressing **and** renaming **and** policy edits — in one `f5 query` invocation?

## Before you start

- A `bigip.conf` / SCF file.
- A clear sequence of edits you want to land.  This is best done in stages — preview each one with the default unified diff before you persist with `--in-place`.

## Answer

`f5 query` runs `;`-separated statements **in order against the evolving source** — each statement sees the result of every previous statement.  This is what lets you compose a multi-step migration in one preview-able expression.

### Tenant migration: rename a partition, then add an iRule to every VS in it

```
$ f5 query --in-place '
  rename_partition("Tenant_A", "Tenant_B") ;
  .ltm.virtual["~^/Tenant_A/"]
    | select(not contains(.rules, "/Tenant_A/audit_rule"))
    | .rules += "/Tenant_A/audit_rule"
' bigip.conf
```

1. **Statement 1**: `rename_partition("Tenant_A", "Tenant_B")` cascades through every object header, every reference, every destination prefix and pool-member identifier, and the `auth partition` stanza.  After this statement the in-memory source has every `/Common/` replaced with `/Tenant_A/`.
2. **Statement 2** runs against the post-rewrite source, so the regex subscript already finds the renamed VSes under `/Tenant_A/` and the audit-rule path also lives there.  `+=` appends to the `rules` list field; the dedup `select(not contains(...))` keeps the operation idempotent.

### Readdress and rename in one pass

```
$ f5 query '
  rename("/Common/old_web_pool", "/Common/web_pool") ;
  .ltm.virtual[]
    | select(.pool == "/Common/web_pool")
    | .destination |= ip("10.20.0.0/24", .)
' bigip.conf
```

After the rename, every VS that points to the pool sees the new name.  The second statement readdresses each of those VSes into the new subnet, preserving host bits.

### Bulk rename with derived names

```
$ f5 query '.ltm.pool["~^/Common/old_"]
  | .name |= sub(., "/Common/old_", "/Common/new_")' bigip.conf
```

`|=` on an identity field auto-routes through the rename engine, so the substring transform applied by `sub` lands as a full identity rename per match — references update everywhere too.

### Stage-by-stage migration with a script file

For anything more than three steps, put the query in a file and use `-f`:

```
# tenant-migration.fq
# 1. Move every object from /Tenant_A/ into /Tenant_B/.
rename_partition("Tenant_A", "Tenant_B") ;

# 2. Standardise pool names.
.ltm.pool["~^/Tenant_A/old_"]
  | .name |= sub(., "/old_", "/")
;

# 3. Strip route domains from every destination
#    (consolidating onto the default RD post-migration).
.ltm.virtual[].destination |= with_route_domain(., "")
;

# 4. Bring every dev VS to maintenance (port 0).
.ltm.virtual[]
  | select(contains(.name, "_dev_"))
  | .destination |= sub(., ":[0-9]+$", ":0")
```

Run it as a dry-run first:

```
$ f5 query -f tenant-migration.fq bigip.conf
```

Read the unified diff, fix any predicate that's too broad, then persist:

```
$ f5 query -f tenant-migration.fq --in-place bigip.conf
```

### Statement-boundary rules

Two restrictions to know:

- **Prefix rewrites can't mix with field edits in the same statement.**  ``rename_partition()`` shifts byte offsets across the source; field-slot ranges captured at projection time would target the wrong span after.  Split them with ``;`` (as in every example above) and each statement sees the post-rewrite source.
- **Identity-field writes can't use ``+=`` / ``-=``.**  Arithmetic on a name is nonsensical; identity-field assignments support `=` and `|=` only.

### Composing with the per-verb tools

`f5 query` doesn't replace `f5 grep` or `f5 cleanup` — it composes with them.  A common pattern is to use `f5 query --paths-only` to surface a set of objects, then pipe the paths through `xargs` to a per-object verb:

```
# Move every orphan pool into a `/Trash/` partition for review before delete
$ f5 query --paths-only '
  .ltm.pool[] | select(referenced_by(.) | count == 0)
' bigip.conf | while read path; do
    f5 rename "$path" "$(echo "$path" | sed 's|^/[^/]*/|/Trash/|')" bigip.conf
  done
```

(The same effect can be done in pure DSL with `.ltm.pool[] | select(...) | .name |= with_partition(., "Trash")`, which is faster — but the shell-pipeline pattern is useful when you want to surface candidates for human review between detection and action.)

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [F5 query DSL — builtin function reference](../references/f5_query/builtins.md)
- [How do I migrate every object from one partition into another?](kcs-howto-migrate-partition-with-query.md)
- [How do I audit a BIG-IP config with f5 query?](kcs-howto-audit-config-with-query.md)
- [How do I rename a pool everywhere?](kcs-howto-rewrite-pool-refs-in-irules.md)
