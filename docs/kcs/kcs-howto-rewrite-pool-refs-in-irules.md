# KCS: How do I rename a pool everywhere — including inside iRule bodies?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI

## Question

How do I rename a pool everywhere — including inside iRule bodies — without writing a shell pipeline of `sed` and `grep`?

## Before you start

- A `bigip.conf` / SCF that contains the pool and the iRules that reference it.
- The pool's current full-path and the target full-path (for example `/Common/old_pool` → `/Common/new_pool`).

## Answer

Assigning to an object's identity field (`.name` or `."full-path"`) inside `f5 query` automatically routes through the same engine `f5 rename` uses, which rewrites both the pool's header *and* every reference — including pool references buried in iRule command arguments (`pool $name`, `class match`, etc.).

1. Preview the rewrite with a dry-run:

   ```
   f5 query '.ltm.pool["/Common/old_pool"].name = "/Common/new_pool"' bigip.conf
   ```

   The verb prints a unified diff plus a stderr line of the form `renamed '/Common/old_pool' -> '/Common/new_pool' (N occurrence(s))` so you can confirm the blast radius before persisting.

2. When the count looks right, persist:

   ```
   f5 query --in-place '.ltm.pool["/Common/old_pool"].name = "/Common/new_pool"' bigip.conf
   ```

3. To rename several pools by pattern in one pass, chain statements with `;`:

   ```
   f5 query --in-place '
     .ltm.pool["/Common/old_web"].name  = "/Common/new_web";
     .ltm.pool["/Common/old_api"].name  = "/Common/new_api"
   ' bigip.conf
   ```

   Each statement collects its rename plan, and they're applied in order so an iRule mentioning both pools is rewritten consistently.

For finer-grained edits inside an iRule — say, fixing one specific pool reference but not another — read the pool refs and rewrite them in place:

```
f5 query '.ltm.rule[].refs.pools[]' bigip.conf
```

Lists every pool reference found in any iRule body.  Combine with `select(...)` to narrow further.

If you only want to rename the pool but not its references (rare — usually a sign the references should be deleted), edit the SCF stanza directly: `f5 query --scf` will surface the stanza, and `f5 rename` will not run.

## Related

- [BIG-IP Query DSL](features/kcs-feature-bigip-query.md)
- [BIG-IP rename](features/kcs-feature-rename.md)
- [BIG-IP Related-Object Grep](features/kcs-feature-bigip-grep.md)
