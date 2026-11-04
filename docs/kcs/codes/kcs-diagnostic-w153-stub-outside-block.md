# KCS: W153 — Stub directive outside a block

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, analyser, command-walk

## Profiles

default

## Question

Why does a `# tcl-lsp: stub` directive have no effect?

## Why

An inline stub declaration only takes effect between `stubs-begin` and
`stubs-end` comments. W153 marks a valid directive outside those markers so
the ignored declaration is visible at its source line. A call to the undeclared
command can also receive W123.

## Symptoms

- A warning on the stub comment says it is outside a block and has no effect.
- Calls to the intended command can still show `W123 Unknown command`.

## Example that triggers it

```tcl
# tcl-lsp: stub db_eval {sql script:body}
db_eval {select 1} {puts row}
```

The first line gets W153, and `db_eval` still gets W123.

## Fix

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub db_eval {sql script:body}
# tcl-lsp: stubs-end
db_eval {select 1} {puts row}
```

The markers activate the stub. The same rule covers valid `stub expr-func` and
`stub expr-op` declarations. A malformed stub has no W153 because it was not
a valid declaration to begin with.

## How to suppress

Use a top-of-file `# tcl-lsp: disable=W153` directive, or disable W153 in
editor diagnostic settings. Suppressing W153 does not activate the stub, so
W123 can still appear on the call.

## Related

- [KCS codes index](README.md)
- [How to annotate commands with stubs](../kcs-howto-annotate-commands-with-stubs.md)
- [W123 — unresolved command](kcs-diagnostic-w123-unresolved-command.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
