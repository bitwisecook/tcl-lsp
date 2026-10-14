# KCS: Does the language server follow `rename` and `interp alias`?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Answer

Yes, when the original source identifies the command and its arguments.
The server follows the binding at each call's source position.

```tcl
interp alias {} myfmt {} format
myfmt %s hello
rename format origfmt
origfmt %s world
```

The alias and moved command can retain `format` argument assistance.
A call before the rename keeps its earlier binding.
A replacement procedure does not borrow the builtin's grammar.

Imports, fixed alias prefixes, and nested source bodies retain their own context.
Known prefix arguments shift later argument positions together.
Captured prefix data has no editable position at the later call.
A global spelling such as `::myfmt` is distinct from a caller-relative lookup.

Features use these facts for highlighting, argument hints, source structure,
and navigation. Formatting and rename require their own source checks.
Knowing a handler's name alone does not permit a source edit.

Computed names, unavailable child interpreters, callbacks, and conflicting
bindings can leave the target unknown. Conditional declarations may supply
limited assistance without proving that their bodies execute.
The server preserves that uncertainty instead of choosing a matching name.

## Related

- [Command registry design](../design/compiler/command-registry.md#known-limitations)
- [Shared naming contract](../design/contracts/name-handling.md)
- [Command source and lookup owners](../design/compiler/namespace-resolution.md)
