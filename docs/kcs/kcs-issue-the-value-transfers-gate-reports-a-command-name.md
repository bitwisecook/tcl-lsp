# KCS: The value-transfers gate says my change recognises a command by name

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp CLI, make rust-check

## Question

`cargo xtask value-transfers --check` (or `make xtask-check`) fails on a
line I wrote, on a file's site count, or says the generated inventory is
stale. What is it checking, and how do I make it pass?

## Symptoms

One of five messages:

- `N site(s) recognise a command by name in a file the gate holds clean`,
  with a `path:line: snippet` list.
- `` `path` has N unwaived site(s) against a pin of M; the count may only
  fall ``, with the file's sites.
- `` `path` has N unwaived site(s) against a pin of M; lower the pin ``.
- `` `cmd` writes a variable and declares no semantics; classify it in
  KNOWN_GAPS ``.
- `docs/generated/value-transfers.md is stale — run cargo xtask
  value-transfers`.

## Cause

The gate keeps per-command knowledge on the value axis in the registry
(`docs/design/compiler/value-transfers.md`). It scans the compiler and the
analysis and tooling crates for a Tcl command or subcommand name used to
recognise an invocation — `command == "unset"`, `matches!(cmd.as_str(),
"foreach" | "lmap")`, a `match head { "expr" => … }` arm — and for a `match`
arm on a catalogued evaluator id (`NativeEvalId::…`) outside the registry.

Every scanned file is held to one of two rules. A file the migration has
already rewritten (`CLEAN_FILES` in `rust/xtask/src/value_transfers.rs`)
is *clean*: every site is waived or gone. Every other file is *ratcheted*:
its count of unwaived sites is pinned in `RATCHET`, the count may only
fall, and a file with no pin may hold no site. A pin is lowered beside the
review that removes or waives the file's sites — never raised, never
added — and the ledger in
`docs/design/compiler/value-transfers-migration.md` § *The ratchet over
unreviewed files* carries the same pins with the slice, or the axis
migration, that reviews each file; the gate fails when the two disagree.

It also enumerates every command and subcommand through the invocation
resolver and writes the result to `docs/generated/value-transfers.md`; a
command that writes a variable and declares no semantics is a *gap*, and a
gap must be classified with the migration slice that gives it semantics.

## Fix

1. If the fact belongs on a registry field, put it there: declare
   `semantics` on the spec, or add the argument role, trait, or option the
   fact really is, and delete the name check.
2. If the site is tracked debt or the sanctioned exception, add
   `// value-transfer-ok: <axis> — <reason>` on the line or in the comment
   block directly above it. The axis is the registry field the fact belongs
   to (`options`, `arg_roles`, `traits`, `case_list`, `native_lowering`,
   …), `dataflow` for a value-axis site awaiting its slice, or
   `irreducible` with why. A file whose sites all share one axis may carry
   one `// value-transfer-ok(file): <axis> — <reason>` in its header.
3. If the message says the count may only fall, the file is ratcheted and
   your change added a site: do step 1 or 2 for the new site. A pin is
   never raised.
4. If the message says to lower the pin, your change removed or waived a
   site in a ratcheted file: set the file's count in `RATCHET` and in its
   ledger row to the number the message gives, and drop both when it
   reaches zero.
5. For a new variable-writing command with no semantics, add it to
   `KNOWN_GAPS` in `rust/xtask/src/value_transfers.rs` with the slice that
   closes the gap, or give it a declaration. An entry no row matches any
   more is stale and must go.
6. Run `cargo xtask value-transfers` (no `--check`) from the repository
   root to rewrite the inventory, and commit it with the change.

## How to tell it worked

`cargo xtask value-transfers --check` prints `value-transfers: OK` with the
clean, waived, and pinned counts; the inventory's *Hand-written command
knowledge outside the registry* table lists your waiver under its axis, or
its *The ratchet* table shows the file's new count.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-qa-what-does-a-value-transfer-declaration-say.md](compiler/kcs-qa-what-does-a-value-transfer-declaration-say.md)
- [value-transfers-migration.md](../design/compiler/value-transfers-migration.md) § *The drift gate and the generated inventory* and § *The ratchet over unreviewed files*
