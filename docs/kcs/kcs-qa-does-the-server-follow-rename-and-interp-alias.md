# KCS: Does the language server follow `rename` and `interp alias`?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli

## Answer

Yes, for the bindings a file states unconditionally at its top level.

Tcl resolves a command by its interpreter-level *binding*, not by the spelling
used to invoke it. After

```tcl
interp alias {} myfmt {} format
```

`myfmt` **is** `format`, and after

```tcl
rename format origfmt
```

`origfmt` is `format` while `format` no longer exists at all. The server reads
those statements and applies the real command's grammar to the name that now
carries it, and withholds it from the name that no longer does.

Four kinds of statement are followed:

| Statement | Effect |
|---|---|
| `namespace import ::tcltest::*` | `test` is `::tcltest::test` |
| `interp alias {} myfmt {} format` | `myfmt` is `format` |
| `rename format origfmt` | `origfmt` is `format`; `format` is gone |
| `proc format {args} {…}` | `format` is your procedure, not the built-in |

Chains compose: after `interp alias {} a {} format` and then `rename a b`, `b`
is `format`.

The explicitly global spelling behaves identically to the bare one, so
`::myfmt` and `myfmt` are the same command.

### What changes as a result

Everything that reads a command's grammar follows the binding: syntax
highlighting, format-specifier hints, code folding, **formatting**, minifying,
go-to-declaration, the call graph, parameter-usage inference, and — for
iRules — which arguments name BIG-IP objects.

Formatting is the most visible. Before this landed, a document that renamed a
body-bearing command still had its calls laid out under the grammar of the
command they no longer were:

```tcl
rename if maybe
maybe {$x} {puts a}
```

The `maybe` call is now left exactly as written, because `maybe`'s argument is
no longer the server's business — and a call to `if` through a proven alias is
expanded onto its own lines exactly as `if` would be.

### What the server will not guess

It abstains — leaves the name alone, or applies no grammar at all — rather
than guessing, whenever it cannot prove the binding:

- a **computed** binding (`rename $old new`, `interp alias {} $n {} eval`);
- an alias with **pre-bound arguments** (`interp alias {} pad {} format %08x`),
  which shifts every argument position;
- a binding in **another interpreter** (`interp alias slave …`), or a command
  hidden in a safe one;
- a binding that is **conditional or nested** — inside an `if`, a procedure, an
  `eval`, or a `namespace eval` block — because it is not an unconditional
  fact about the whole file;
- anything reached through `unknown`, a trace, or a computed command name.

It also abstains when a file binds the same name twice and the reader has no
byte position to choose between them, which is the case for the formatter and
the minifier's body rewriting.

Bindings are also read **in source order**, so a call written before a `rename`
still sees the original command.

## Related

- [Command registry design doc](../design/compiler/command-registry.md#known-limitations)
  — the descriptor, the positioned/unpositioned split, and the per-consumer table.
- [Name resolution](../design/name-resolution.md)
  — the import/alias/rename link graph and its follow-versus-rewrite policy.
