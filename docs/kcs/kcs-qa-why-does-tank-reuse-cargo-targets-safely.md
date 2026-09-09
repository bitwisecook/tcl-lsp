# KCS: Why does Tank reuse Cargo targets safely?

> **Audience:** Maintainer
> **Type:** Q&A

## Applies to

tcl-lsp-cli

## Question

Why can the Tank runner reuse a Cargo target without mixing worktrees?

## Answer

The CI helper gives each runner registration, repository, and checkout root a
different identity. It hashes those values into a private target directory.
An identity marker must match before Cargo can use an existing directory.

The helper rejects symlinks, non-canonical paths, wrong ownership, and unsafe
permissions. Hosted overflow does not use the retained target. Tank remains
serialised with `queue: max` and `cancel-in-progress: false`.

Before Cargo starts, a bounded janitor removes only old, marked targets. It
skips locked targets and checks a 20 GiB free-space floor. An always-run final
step reports the target size and free space, while compiler-cache statistics
remain independent.

For the implementation contract, see [persistent Cargo targets on
Tank](../design/contracts/tank-persistent-cargo-target.md).
