# KCS: The registry-axes gate says my change compares a keyword

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp CLI, make rust-check

## Question

`cargo xtask registry-axes --check` (or `make xtask-check`) fails on a line
I wrote, on a file's site count, on a waiver comment, or says the generated
ledger is stale. What is it checking, and how do I make it pass?

## Symptoms

One of five messages:

- `N site(s) compare a registry word in a file the gate holds clean`, with
  a `path:line: snippet` list.
- `` `path` has N unwaived site(s) against a pin of M; the count may only
  fall ``, with the file's sites.
- `` `path` has N unwaived site(s) against a pin of M; lower the pin ``.
- `path:line: waiver …` — the waiver names an unknown axis, has no
  expiry, expires `never` on an axis other than `irreducible`, or expires
  with a step or slice that has already landed.
- `docs/generated/registry-axes.md is stale — run cargo xtask
  registry-axes`.

## Cause

The command registry declares every word a consumer could recognise a
construct by: command and subcommand names, option spellings, member
keywords of a class-definition body (`method`, `constructor`, …), clause
keywords (`elseif`, `else`, `then`, `on`, `trap`, `finally`), and
special-variable names. The gate computes that vocabulary from the
registry and scans the compiler and the analysis and tooling crates for a
string literal holding one of those words as the operand of `==` or `!=`,
a `matches!` pattern, a `match` arm pattern, an `.eq(` argument, an
element of an inline array searched with `.contains(`, or an entry of a
`&[&str]` table the file reads again. Such a site teaches a consumer a fact
the registry already describes — `if word == "elseif"` is the clause
grammar of `if`, spelled by hand. Words inside comments, longer strings,
and `#[cfg(test)]` items are never sites; the registry crate and every
file under a `tests/` directory are not scanned.

Every scanned file is held to one of two rules. A file a build step has
already rewritten (`CLEAN_FILES` in `rust/xtask/src/registry_axes.rs`) is
*clean*: every site is waived or gone. Every other file is *ratcheted*:
its count of unwaived sites is pinned in `RATCHET`, the count may only
fall, and a file with no pin may hold no site.

## Fix

1. If the registry already answers the question, ask it: the clause plan,
   the member row, the option effects, the argument roles, or the special
   variable table, and delete the comparison.
2. If the site is tracked debt or the sanctioned exception, add
   `// registry-axis-ok: <axis> — <reason>; until <expiry>` on the line
   or in the comment block directly above it; for a `match` arm or a
   `matches!` pattern, the comment may sit above the enclosing `match` or
   `matches!` instead. The axis is `command`, `subcommands`,
   `clause_grammar`, `definition_body`, `options`, `special_vars`, or
   `irreducible`. The expiry is `step N` (the consumer-contracts build step
   that retires the site), `slice N` (the value-transfers slice that
   does), or `never`, which only `irreducible` may carry. The comment may
   wrap onto the standalone comment lines below it. A file whose sites all
   share one axis may carry one
   `// registry-axis-ok(file): <axis> — <reason>; until <expiry>` in its
   first 80 lines.
3. If the message says the count may only fall, the file is ratcheted and
   your change added a site: do step 1 or 2 for the new site. A pin is
   never raised.
4. If the message says to lower the pin, your change removed or waived a
   site in a ratcheted file: set the file's count in `RATCHET` to the
   number the message gives, and drop the row when it reaches zero.
5. If a waiver's expiry has landed, the step or slice that was to retire
   the site shipped without it: retire it now, or re-review it and name
   the change that will.
6. Run `cargo xtask registry-axes` (no `--check`) from the repository root
   to rewrite the ledger, and commit it with the change.

## How to tell it worked

`cargo xtask registry-axes --check` prints `registry-axes: OK` with the
vocabulary size and the clean, waived, and pinned counts; the ledger's
*The ledger* table lists your waiver under its axis with its expiry, or its
*The ratchet* table shows the file's new count.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-issue-the-value-transfers-gate-reports-a-command-name.md](kcs-issue-the-value-transfers-gate-reports-a-command-name.md)
- [registry-consumer-contracts.md](../design/compiler/registry-consumer-contracts.md) § *The per-axis lint and ledger*
