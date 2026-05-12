# KCS: When should I use f5 query versus f5 grep or f5 rename?

> **Audience:** User
> **Type:** Q&A

## Applies to

tcl-lsp CLI

## Question

When should I reach for `f5 query` versus `f5 grep` or `f5 rename` — they all seem to overlap?

## Answer

The three verbs cover three different shapes of question, and trying to pick by feature list will leave you unsure.  Pick by shape instead.

**`f5 grep` answers relationship questions.**  "Which objects touch X?", "What does this pool depend on?", "Does any iRule mention this data-group?"  It walks the reference graph in both directions from one or more seed objects.  Reach for it when you want a tree of related things, when the seed is a name or a CIDR, or when you need to surface IP literals buried inside iRule bodies.  Its output is a report, not a rewrite.

**`f5 query` answers property and bulk-rewrite questions.**  "Every VS whose destination is in 10.0.0.0/8", "Every pool whose load-balancing-mode is the default", "Set every VS's destination to ip(new_net, .)".  It runs a small jq-flavoured DSL over the parsed object tree.  Reach for it when the predicate is shaped like a filter on an object's own properties, or when you want to project / rewrite a specific field across many objects in one expression.  An assignment to `.name` auto-routes through `rename_object`, so renaming with `f5 query` is identical in effect to running `f5 rename`.

**`f5 rename` answers the precise rename question.**  "Swap full-path X for Y everywhere it appears."  It is the simplest verb for that case: two positional arguments, one dry-run unified diff by default, `--in-place` to persist.  Reach for it when you already know both names and you do not need a filter or a property edit.

A useful shorthand: if the question fits in two positional arguments, use `rename`.  If the answer is a list of objects, use `grep` for relationship-shaped questions and `query` for property-shaped ones.  If you want to update a field on many objects in one expression, use `query`.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [BIG-IP Related-Object Grep](features/kcs-feature-bigip-grep.md)
- [BIG-IP rename](features/kcs-feature-rename.md)
