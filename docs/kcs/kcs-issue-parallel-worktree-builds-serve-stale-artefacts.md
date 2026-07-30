# KCS: Parallel worktree builds serve stale or cross-checkout artefacts

> **Audience:** Contributor
> **Type:** Issue

## Applies to

tcl-lsp-cli

## Question

Why does a build or test run in one git worktree fail (or pass) with
errors that name code from a *different* checkout of this repository?

## Symptoms

- A binary built in one worktree behaves like a sibling branch: for
  example, `cargo xtask diag-tables` emits a diagnostic that exists only
  on another branch, and the drift gate fails spuriously.
- Phantom compile errors (`E0603: … is private`, "cannot find …",
  "no variant named …") that name line numbers or symbols from another
  checkout, varying from run to run, while the source in front of you
  plainly contains the item.
- A test suite reports failures whose output matches code that was
  already fixed in the tree being tested.

## Answer

The worktrees are sharing one `CARGO_TARGET_DIR`.  Cargo's unit hashing
does not reliably tell apart workspace-member crates built from different
worktree paths of the same workspace, so concurrent builds overwrite each
other's `deps/` outputs, and a later link step can pick up the sibling
worktree's rlib.

1. Give every worktree its own `CARGO_TARGET_DIR`.  The easiest way is
   `source scripts/dev/agent-build-env.sh` in each shell — it pins the
   target dir to `<worktree-root>/target` and sets `CARGO_INCREMENTAL=0`
   and `CARGO_PROFILE_DEV_DEBUG=0` so each dir stays around 3-4 GB.
2. Recover the wedged worktree with `cargo clean -p <crate>` for the
   affected crate (or a full `cargo clean`), then rebuild.
3. Re-run whatever gate produced the suspect result.  Any green or red
   produced while the dir was shared — or after a disk-full (ENOSPC)
   event — is untrustworthy until reproduced on a clean, isolated build.

Sharing `CARGO_HOME` (the downloaded-dependency cache) across worktrees
is safe; only workspace-member artefacts collide.

The success signal: repeated builds in both worktrees stop flip-flopping,
and `scripts/dev/agent-build-env.sh --check` reports a per-worktree
target dir with no warning.

## Related

- [KCS index](README.md)
- The "Parallel worktrees and agent build isolation" section of
  [AGENTS.md](../../AGENTS.md) — the contributor-facing rules the helper
  script enforces.
